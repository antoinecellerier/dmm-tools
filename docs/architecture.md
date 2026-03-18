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
| `protocol.rs` | Message framing: find `AB CD` header, extract payload, validate checksum |
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

Thin CLI binary using `clap`. Subcommands: `list`, `info`, `read`, `command`, `debug`.
All protocol logic lives in the library crate.

### ut61eplus-gui

`eframe`/`egui` application. Runs a background `std::thread` for device I/O,
communicates with the UI via `mpsc` channels. Main graph via `egui_plot`,
minimap via custom painter. Features: responsive layout, dark/light themes,
PPK2-style minimap navigation, continuous timeline across reconnects,
UI zoom (Ctrl+/-), CSV recording/export, persistent settings.

## Key Design Decisions

1. **Sync, not async** — 9600 baud, single device, request/response. No benefit to async complexity.
2. **Direct hidapi, no cp211x_uart** — the cp211x_uart crate is unmaintained (2017). Our CP2110 layer is ~120 lines.
3. **hidraw backend** — required for HID feature reports on Linux (libusb backend doesn't support them).
4. **Transport trait** — enables `MockTransport` for testing without hardware.
5. **Device tables via trait** — adding a new meter model = adding one file implementing `DeviceTable`.
6. **No nom** — payload is a fixed 14-byte struct. Direct indexing is clearer.
