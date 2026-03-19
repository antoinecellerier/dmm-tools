# dmm-tools

Rust tools for communicating with digital multimeters over USB. Currently supports the **UNI-T UT61E+** via its CP2110 HID bridge.

Includes a CLI for reading, recording, and remote-controlling the meter, and a GUI with real-time graphing.

![GUI screenshot — live AC mV measurement with graph, statistics, reference line triggers, and recording](assets/gui-screenshot.png)

## CLI

- Live measurement streaming with text, CSV, and JSON output
- Remote control — send button presses (hold, rel, range, min/max, peak, light)
- Guided protocol capture wizard for bug reports
- Raw hex dump mode for protocol development

```
$ ut61eplus read --count 5
DC V  3.3042 V  22V AUTO
DC V  3.3041 V  22V AUTO
DC V  3.3044 V  22V AUTO
DC V  3.3042 V  22V AUTO
DC V  3.3043 V  22V AUTO

--- 5 samples | Min: 3.3041 | Max: 3.3044 | Avg: 3.3042
```

Output as JSON for scripting:

```
$ ut61eplus read --format json --count 1
{"display_raw":" 3.3042","flags":{"auto_range":true,"dc":true,"hold":false,...},"mode":"DC V","range":"22V","unit":"V","value":3.3042}
```

Send remote commands:

```
$ ut61eplus command hold
Sent Hold
```

## GUI

- Real-time value display and time-series graph with minimap
- Statistics (min/max/avg), cursor measurements, reference lines with threshold triggers
- Recording with CSV export
- Remote control buttons (hold, rel, range, min/max, peak)
- Light and dark themes, responsive layout

## Supported devices

| Model | Status | Notes |
|-------|--------|-------|
| **UT61E+** | Tested | 22000-count, full feature support |
| UT61D+ | Probably works | 6000-count, adds temperature, shares manual/USB module |
| UT61B+ | Probably works | 6000-count, base model, shares manual/USB module |

All three models use the same CP2110 USB adapter and communication protocol. If you have a UT61B+ or UT61D+, please [submit a capture](CONTRIBUTING.md#protocol-captures) so we can confirm support.

## Quick start

### Prerequisites

- Rust toolchain (stable, 2024 edition)
- **Linux:** `libudev-dev` (Debian/Ubuntu) or `systemd-devel` (Fedora)
- **Windows:** [CP2110 driver](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers) from Silicon Labs
- UNI-T UT61E+ with USB adapter plugged in

### Build

```sh
cargo build --workspace
```

### udev rule (Linux)

Grant non-root access to the CP2110 USB adapter:

```sh
sudo cp udev/99-cp2110-unit.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Your user must be in the `plugdev` group (`sudo usermod -aG plugdev $USER`, then log out/in).

### Run

```sh
# List connected devices
ut61eplus list

# Stream measurements
ut61eplus read

# Launch the GUI
ut61eplus-gui
```

If the meter doesn't respond, make sure USB transmission is active: insert the USB module, turn the meter on, and long-press the **USB/Hz** button until the **S** icon appears on the LCD.

## Project structure

```
crates/
  ut61eplus-lib/   — library: CP2110 transport, protocol, measurement parsing
  ut61eplus-cli/   — CLI binary
  ut61eplus-gui/   — GUI binary (eframe/egui)
udev/              — udev rules for Linux
docs/              — architecture, protocol, setup, UX design docs
```

## Documentation

- [Setup & troubleshooting](docs/setup.md)
- [Architecture](docs/architecture.md)
- [Protocol details](docs/protocol.md)
- [UX design](docs/ux-design.md)
- [Development guide](docs/development.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to submit bug reports, protocol captures, and code changes.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE) for details.


## References

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) — Protocol reverse engineering and Python implementation
- [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — Protocol reverse engineering and Go implementation
- [Silicon Labs AN433](https://www.silabs.com/documents/public/application-notes/AN433-CP2110-4-Interface-Specification.pdf) — CP2110/4 HID-to-UART interface specification
- [UT61B+/D+/E+ | User Manual](https://meters.uni-trend.com/download/ut61b-d-e-user-manual/) - UNIT-T user manual
