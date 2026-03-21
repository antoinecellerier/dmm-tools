# dmm-tools

[![CI](https://github.com/antoinecellerier/dmm-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/antoinecellerier/dmm-tools/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/antoinecellerier/dmm-tools)](https://github.com/antoinecellerier/dmm-tools/releases)
[![License: GPL-3.0-or-later](https://img.shields.io/github/license/antoinecellerier/dmm-tools)](LICENSE)

Rust tools for communicating with UNI-T digital multimeters over USB (CP2110 HID bridge). Supports the **UT61E+** family (verified) with experimental support for **UT8803**, **UT171**, and **UT181A**.

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
Sent hold
```

Connect to other device families with `--device`:

```
$ ut61eplus --device ut8803 capture
WARNING: UNI-T UT8803 support is EXPERIMENTAL (unverified against real hardware).
```

## GUI

- Real-time value display and time-series graph with minimap
- Statistics (min/max/avg), cursor measurements, reference lines with threshold triggers
- Recording with CSV export
- Remote control buttons (hold, rel, range, min/max, peak)
- Light and dark themes, responsive layout

## Supported devices

| Family | Models | Protocol | Status |
|--------|--------|----------|--------|
| UT61+/UT161 | UT61E+, UT61B+, UT61D+, UT161B/D/E | Polled, ASCII values | **Verified** (UT61E+) |
| UT8803 | UT8803, UT8803E | Streaming, 21-byte frames | Experimental |
| UT171 | UT171A/B/C | Streaming, float32 values | Experimental |
| UT181A | UT181A | Streaming, float32 + unit strings | Experimental |

**Experimental** means the protocol was reverse-engineered from vendor software but has not been tested against real hardware. If you have one of these meters, please run `ut61eplus --device <family> capture` and [share the report](https://github.com/antoinecellerier/dmm-tools/issues).

See [docs/supported-devices.md](docs/supported-devices.md) for the full compatibility list and reference implementations.

## Quick start

Pre-built binaries for Linux and Windows are available on the [Releases](https://github.com/antoinecellerier/dmm-tools/releases) page.

### Prerequisites

- **Linux:** `libudev-dev` (Debian/Ubuntu) or `systemd-devel` (Fedora) — only needed when building from source
- **Windows:** [CP2110 driver](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers) from Silicon Labs
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
  ut61eplus-lib/   — library: CP2110 transport, protocol, measurement parsing
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
