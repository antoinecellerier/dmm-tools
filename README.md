# dmm-tools

[![CI](https://github.com/antoinecellerier/dmm-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/antoinecellerier/dmm-tools/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/antoinecellerier/dmm-tools)](https://github.com/antoinecellerier/dmm-tools/releases)
[![License: GPL-3.0-or-later](https://img.shields.io/github/license/antoinecellerier/dmm-tools)](LICENSE)

Rust tools for communicating with UNI-T digital multimeters over USB (CP2110 and CH9329 HID bridges). Supports the **UT61E+** family (verified) with experimental support for **UT8803**, **UT171**, and **UT181A**.

Includes a CLI for reading, recording, and remote-controlling the meter, and a GUI with real-time graphing.

![GUI screenshot — live DC mA measurement with graph, statistics, reference line triggers, and recording](assets/gui-screenshot.png)

## [CLI](docs/cli-reference.md)

- Live measurement streaming with text, CSV, and JSON output
- Coulomb counting / energy integration (`--integrate`)
- Remote control — send button presses over USB
- Guided protocol capture wizard for bug reports

```
$ ut61eplus read --count 5
9.090 MΩ [AUTO]
8.902 MΩ [AUTO]
10.182 MΩ [AUTO]
9.399 MΩ [AUTO]
9.176 MΩ [AUTO]

--- 5 samples | Min: 8.9020 | Max: 10.1820 | Avg: 9.3498
```

Output as JSON for scripting:

```
$ ut61eplus read --format json --count 1
{"display_raw":"  3.369","flags":{"auto_range":true,"dc":false,"hold":false,...},"mode":"DC V","range":"22V","unit":"V","value":3.369}
```

Send remote commands:

```
$ ut61eplus command hold
Sent hold
```

Connect to other device families with `--device`:

```
$ ut61eplus --device ut8803 capture
WARNING: UNI-T UT8803 support is EXPERIMENTAL (unverified against real hardware).
```

## [GUI](docs/gui-reference.md)

- Real-time value display and time-series graph with minimap
- Statistics, cursor measurements, reference lines with threshold triggers
- Live specifications (resolution, accuracy) for the current range
- Recording with CSV export and remote control buttons
- Big meter mode for bench-mount use

## Supported devices

| Family | Models | Protocol | Status |
|--------|--------|----------|--------|
| UT61+/UT161 | UT61E+, UT61B+, UT61D+, UT161B/D/E | Polled, ASCII values | **Verified** (UT61E+) |
| UT8803 | UT8803, UT8803E | Streaming, 21-byte frames | Experimental |
| UT171 | UT171A/B/C | Streaming, float32 values | Experimental |
| UT181A | UT181A | Streaming, float32 + unit strings | Experimental |

**Experimental** means the protocol was reverse-engineered from vendor software but has not been tested against real hardware. If you have one of these meters, we'd love your help verifying: [UT8803](https://github.com/antoinecellerier/dmm-tools/issues/3), [UT171](https://github.com/antoinecellerier/dmm-tools/issues/4), [UT181A](https://github.com/antoinecellerier/dmm-tools/issues/5). For UT61B+/UT61D+ owners: [help verify model-specific modes](https://github.com/antoinecellerier/dmm-tools/issues/7).

See [docs/supported-devices.md](docs/supported-devices.md) for the full compatibility list and reference implementations.

## Quick start

Pre-built binaries for Linux, Windows, and macOS are available on the [Releases](https://github.com/antoinecellerier/dmm-tools/releases) page.

> **macOS Intel users:** Pre-built binaries are provided but haven't been tested against real hardware yet. If you have an Intel Mac and a supported meter, please [let us know how it goes](https://github.com/antoinecellerier/dmm-tools/issues/2) — even "it works" helps.

### Prerequisites

- **Linux:** `libudev-dev` (Debian/Ubuntu) or `systemd-devel` (Fedora) — only needed when building from source
- **Windows:** Open Device Manager with the USB cable plugged in. If you see "CP2110 USB to UART Bridge" under HID devices, no action needed. If you see a yellow warning icon under "Other devices", install the [Silicon Labs driver](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers). Some UT-D09 cables use a different chip and appear as "USB Input Device" instead — these work without a driver.
- **macOS:** No driver needed — both cable types are recognized as standard HID devices
- A [supported UNI-T multimeter](docs/supported-devices.md) with USB adapter plugged in

### Install from source

Requires the Rust toolchain (stable, 2024 edition). On Linux, also requires `libudev-dev` (Debian/Ubuntu) or `systemd-devel` (Fedora).

```sh
cargo install --git https://github.com/antoinecellerier/dmm-tools.git ut61eplus-cli
cargo install --git https://github.com/antoinecellerier/dmm-tools.git ut61eplus-gui
```

Or clone and build the whole workspace:

```sh
cargo build --workspace
```

### udev rule (Linux)

Grant non-root access to the USB adapter (covers both CP2110 and CH9329):

```sh
sudo cp udev/99-dmm-tools.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Your user must be in the `plugdev` group (`sudo usermod -aG plugdev $USER`, then log out/in).

### Run

```sh
# List connected devices
ut61eplus list

# Stream measurements (UT61E+ default)
ut61eplus read

# Use a different device family
ut61eplus --device ut8803 read

# Launch the GUI
ut61eplus-gui
```

If the meter doesn't respond, make sure USB transmission is active: insert the USB module, turn the meter on, and long-press the **USB/Hz** button until the **S** icon appears on the LCD. For UT171/UT181A, enable "Communication ON" in the meter's SETUP menu.

## Project structure

```
crates/
  ut61eplus-lib/   — library: USB transport (CP2110, CH9329), protocol, measurement parsing
  ut61eplus-cli/   — CLI binary
  ut61eplus-gui/   — GUI binary (eframe/egui)
udev/              — udev rules for Linux
docs/              — architecture, protocol, setup, UX design docs
```

## Documentation

- [CLI reference](docs/cli-reference.md)
- [GUI reference](docs/gui-reference.md)
- [Setup & troubleshooting](docs/setup.md)
- [Architecture](docs/architecture.md)
- [Protocol details](docs/protocol.md)
- [UX design](docs/ux-design.md)
- [Development guide](docs/development.md)
- [Changelog](CHANGELOG.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to submit bug reports, protocol captures, and code changes.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE) for details.


## References

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) — Protocol reverse engineering and Python implementation
- [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — Protocol reverse engineering and Go implementation
- [Silicon Labs AN434](https://www.silabs.com/documents/public/application-notes/an434-cp2110-4-interface-specification.pdf) — CP2110/4 HID-to-UART interface specification
- [UT61B+/D+/E+ | User Manual](https://meters.uni-trend.com/download/ut61b-d-e-user-manual/) - UNI-T user manual
