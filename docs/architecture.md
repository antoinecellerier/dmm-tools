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
| `transport.rs` | `Transport` trait abstracting HID I/O; `MockTransport` for tests |
| `protocol.rs` | Message framing: find `AB CD` header, extract payload, validate checksum. `pub(crate)` — internal to library. |
| `measurement.rs` | `Measurement` struct + `parse()`: decode 14-byte payload into typed reading |
| `mode.rs` | `Mode` enum: 31 measurement modes (DCV, ACV, Ohm, etc.) |
| `flags.rs` | `StatusFlags`: Hold, Rel, Auto, Min/Max, Low Battery |
| `command.rs` | `Command` enum: remote button presses with wire encoding |
| `tables/` | `DeviceTable` trait + `Ut61ePlusTable`: mode/range → unit/label lookup |
| `error.rs` | `Error` enum via `thiserror` |
| `lib.rs` | `Dmm` struct: top-level API tying everything together |

**Data flow:**

```
USB HID ──► Cp2110 (Transport) ──► protocol::extract_frame() ──► Measurement::parse()
                                                                      │
                                                              DeviceTable lookup
                                                                      │
                                                                      ▼
                                                               Measurement {
                                                                 mode, value,
                                                                 unit, flags, ...
                                                               }
```

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
minimap via custom painter. Features: responsive layout with resizable panels,
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
5. **Device tables via trait** — adding a new meter model = adding one file implementing `DeviceTable`.
6. **No nom** — payload is a fixed 14-byte struct. Direct indexing is clearer.
7. **Measurement fields use `&'static str`** — `unit` and `range_label` reference static table data, avoiding heap allocation per measurement.
8. **Graph segment caching** — segments and gap ranges are rebuilt only when history changes, not every render frame.
9. **Bounded buffers** — graph history (10K points), recording (500K samples), and the background channel prevent unbounded memory growth during sustained use.
10. **Settings schema evolution** — `#[serde(default)]` on `Settings` allows adding new fields without breaking existing config files.
