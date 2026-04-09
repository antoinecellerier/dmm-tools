# Architecture

## Crate Layout

```
ut61eplus/
├── crates/
│   ├── ut61eplus-lib/     # Core library
│   ├── ut61eplus-cli/     # CLI binary
│   └── ut61eplus-gui/     # GUI binary
```

### ut61eplus-lib

The library crate handles all device communication and data parsing. It has no UI dependencies.

**Module responsibilities:**

| Module | Responsibility |
|--------|---------------|
| `cp2110.rs` | CP2110 HID transport: open device, init UART, read/write interrupt reports |
| `ch9329.rs` | CH9329 HID transport: open device, read/write 65-byte HID reports (experimental) |
| `qinheng.rs` | QinHeng CH9325 HID transport: 8-byte reports with 0xF0+len framing, dual baud rate probing (2400/19200) |
| `transport.rs` | `Transport` trait abstracting HID I/O; `Box<dyn Transport>` delegation for runtime transport selection; `MockTransport` for tests |
| `protocol/mod.rs` | `Protocol` trait (object-safe), `DeviceFamily` enum, `DeviceProfile`, `Stability` |
| `protocol/registry.rs` | Device registry: `SelectableDevice` entries, factory functions, `resolve_device()` lookup. CLI and GUI use the registry for device selection — no device-specific code in app crates. |
| `protocol/framing.rs` | Message framing: find `AB CD` or `0xAC` header, extract payload, validate checksum (or position code + BCD for UT8802) |
| `protocol/ut61eplus/` | UT61E+ family: `Ut61PlusProtocol`, `Mode` enum, `Command` enum, `tables/` (per-model `DeviceTable` impls with range info and spec data) |
| `protocol/ut8802/` | UT8802 family: `Ut8802Protocol` — streaming protocol with 0x5A trigger, 0xAC 8-byte BCD frames |
| `protocol/ut8803/` | UT8803 family: `Ut8803Protocol` — streaming protocol with 0x5A trigger |
| `protocol/ut171/` | UT171 family: `Ut171Protocol` — streaming protocol, float32 LE values |
| `protocol/ut181a/` | UT181A: `Ut181aProtocol` — streaming protocol, device-sent unit strings |
| `protocol/vc880/` | VC-880/VC650BT: `Vc880Protocol` — streaming, AB CD framing (reuses UT61E+ extractor), ASCII display values |
| `protocol/vc890/` | VC-890: `Vc890Protocol` — polled (0x5E request), AB CD framing, 60K counts, 66-byte frames |
| `measurement.rs` | `Measurement` struct: mode, value, unit, flags (protocol-agnostic) |
| `flags.rs` | `StatusFlags`: Hold, Rel, Auto, Min/Max, Low Battery |
| `error.rs` | `Error` enum via `thiserror` |
| `lib.rs` | `Dmm` struct: top-level API tying everything together |

**Data flow:**

```
CLI/GUI ──► registry::resolve_device()
                       │
                       └──► SelectableDevice.new_protocol()
                                           │
USB HID ──► Cp2110 or Ch9329 (Box<dyn Transport>) ──► Box<dyn Protocol> ──► Measurement { mode, value, unit, flags }
                                           │
                                           ├── Ut61PlusProtocol  (polled, AB CD framing, per-model DeviceTable)
                                           ├── Ut8802Protocol    (streaming, 0xAC 8-byte BCD, no checksum)
                                           ├── Ut8803Protocol    (streaming, AB CD 21-byte, BE checksum)
                                           ├── Ut171Protocol     (streaming, float32 LE)
                                           ├── Ut181aProtocol    (streaming, device-sent units)
                                           ├── Vc880Protocol     (streaming, AB CD framing, ASCII values)
                                           └── Vc890Protocol     (polled, AB CD framing, 60K counts)
```

`Dmm<T: Transport>` holds a `Box<dyn Protocol>`. The `Protocol` trait provides `init()`,
`request_measurement()`, `send_command()`, `get_name()`, `profile()`, and `capture_steps()`.
Each family implements its own framing, parsing, and command encoding internally, but all
produce the same `Measurement` struct.

**Device registry** (`protocol/registry.rs`) is the single source of truth for all selectable
devices. Each `SelectableDevice` entry contains an ID, display name, aliases, activation
instructions, and a factory function that creates the correct `Protocol` instance. The CLI
and GUI resolve user input via `resolve_device()` and use `open_device_by_id()` to connect —
they never match on `DeviceFamily` variants or instantiate protocol types directly.
`open_device_by_id_auto()` tries CP2110, then CH9329, then QinHeng CH9325, returning a `Box<dyn Transport>`.
Adding a new device requires only a registry entry and a `Protocol` implementation; zero app code changes.

### ut61eplus-cli

CLI binary using `clap`. Split into three modules:

| Module | Responsibility |
|--------|---------------|
| `main.rs` | CLI framework, command dispatch, `list`/`info`/`read`/`command`/`debug` subcommands |
| `capture.rs` | Guided protocol capture tool: types (`CaptureReport`, `StepResult`, `SampleData`), step definitions, interactive prompting, multi-part capture orchestration, YAML report I/O |
| `format.rs` | Measurement output formatting (text/csv/json) |

All protocol logic lives in the library crate. The `capture` subcommand provides a guided
interactive wizard for protocol verification, outputting YAML reports with raw bytes.
Uses `console` crate for colored output and single-key input, `serde_yaml` for report format.
Capture reports are written atomically (temp file + rename) for crash safety.

### ut61eplus-gui

`eframe`/`egui` application. Runs a background `std::thread` for device I/O,
communicates with the UI via `mpsc` channels. Main graph via `egui_plot`,
minimap via custom painter. Uses `clap` for CLI argument parsing (`--device`,
`--theme`, `--mock-mode`) — overrides are session-only and don't persist to
`settings.json`. Features: responsive layout with resizable panels,
dark/light themes with WCAG-compliant colors, PPK2-style minimap navigation,
continuous timeline across reconnects, pause/resume capture, graph overlays
(mean line, reference lines, measurement cursors, min/max envelope, trigger markers),
remote control buttons, UI zoom (Ctrl+/-), CSV recording/export with scrollable
sample log, persistent settings.

## Key Design Decisions

1. **Sync, not async** — 9600 baud, single device, request/response. No benefit to async complexity.
2. **Direct hidapi, no cp211x_uart** — the cp211x_uart crate is unmaintained (2017). Our CP2110 layer is ~120 lines.
3. **hidraw backend** — required for HID feature reports on Linux (libusb backend doesn't support them).
4. **Transport trait** — enables `MockTransport` for testing without hardware.
5. **Protocol trait** — each device family implements `Protocol` (object-safe, `Send`). `Dmm` dispatches through `Box<dyn Protocol>`, so callers don't need to know the family at compile time.
6. **Device tables via trait** — within the UT61E+ family, adding a new meter model = adding one file implementing `DeviceTable`.
7. **No nom** — payload is a fixed 14-byte struct. Direct indexing is clearer.
8. **Measurement fields use `&'static str`** — `unit` and `range_label` reference static table data, avoiding heap allocation per measurement.
9. **Graph segment caching** — segments and gap ranges are rebuilt only when history changes, not every render frame.
10. **Bounded buffers** — graph history (10K points), recording (500K samples), and the background channel prevent unbounded memory growth during sustained use.
11. **Settings schema evolution** — `#[serde(default)]` on `Settings` allows adding new fields without breaking existing config files.
12. **Device registry** — all device metadata (display names, aliases, activation instructions, protocol factories, manual URLs) lives in a single `DEVICES` slice in the library. CLI and GUI consume the registry without device-specific knowledge, so adding a new device family requires zero app code changes.
13. **Static spec data** — per-range specifications (resolution, accuracy bands) and per-mode metadata (input impedance, notes) are `&'static` arrays in `tables/specs_*.rs` files, transcribed from device manuals. The GUI caches spec lookups keyed on `(mode_raw, range_raw)` and re-looks up only on mode/range changes — zero per-frame allocations. Use `cargo run -p ut61eplus-lib --example dump_specs` to verify spec data against manuals.
