# dmm-tools

USB and Bluetooth multimeter logger and remote control for UNI-T, Voltcraft, ZOTEK (ZOYI, BSIDE, ANENG) and EEVblog digital multimeters, on Linux, macOS and Windows.

[![CI](https://github.com/antoinecellerier/dmm-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/antoinecellerier/dmm-tools/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/antoinecellerier/dmm-tools)](https://github.com/antoinecellerier/dmm-tools/releases/latest)
[![License: GPL-3.0-or-later](https://img.shields.io/github/license/antoinecellerier/dmm-tools)](LICENSE)

Read, record and remote-control a digital multimeter over its USB cable, or wirelessly over Bluetooth. Supports the UNI-T UT61E+, UT61B+, UT61D+, UT161, UT60BT, UT202BT, UT71, UT171, UT181A, UT803, UT804, UT8802 and UT8803, the Voltcraft VC-880, VC650BT, VC-890, VC920, VC940 and VC960, the ZOTEK ZT-300AB, ZT-5566SE, ZT-5BQ and ZT-5B (ANENG AN9002, AN999S, ST207, V05B), and the EEVblog 121GW — see [supported devices](#supported-devices).

Includes a CLI with text, CSV and JSON output and a GUI with real-time graphing.

![dmm-gui on a DC mA session: the live reading, specifications and statistics beside the graph, where the minimap picks out a sensor's boot sequence and two cursors read its duration and charge](assets/gui-wide-layout.png)

## [GUI](docs/gui-reference.md)

- Live reading with the meter's flags and sub-values, and buttons to switch mode, range, HOLD, REL and MIN/MAX from the screen
- Time-series graph with minimap, cursors, mean and min/max overlays, and reference lines with threshold triggers
- Recording for hours at a time, exported as CSV, JSON or a replay file
- Software scale, offset and unit relabel for clamps, shunts and sensors
- Live specifications (resolution, accuracy) for the current range
- Big meter mode for bench-mount use

## [CLI](docs/cli-reference.md)

- Stream readings as text, CSV or JSON, at any interval
- Switch mode, range, HOLD, REL, MIN/MAX and Peak by name, or press the meter's buttons
- Coulomb counting / energy integration (`--integrate`)
- Software scaling (`--scale`, `--offset`, `--unit`) for clamps, shunts and sensors
- Guided protocol capture wizard for bug reports and verifying new meters

<!-- snippet via=dcma-boot-refresh.replay
dmm-cli read --interval-ms 2500 --count 5
-->
```
$ dmm-cli read --interval-ms 2500 --count 5
0.00 mA
32.54 mA
100.96 mA
107.95 mA
4.01 mA

--- 5 samples | Min: 0.0000 mA | Max: 107.9500 mA | Avg: 49.0920 mA
```
<!-- /snippet -->

Output as JSON for scripting:

<!-- snippet via=ohm.replay
dmm-cli read --format json --count 1
-->
```
$ dmm-cli read --format json --count 1
{"_metadata":{"device":"UNI-T UT61E+"}}
{"timestamp":"2026-09-17T10:47:27.597+00:00","mode":"Ω","value":4.649,"unit":"kΩ","range":"22kΩ","display_raw":"  4.649","progress":9,"experimental":false,"flags":{"hold":false,"rel":false,"auto_range":true,"min":false,"max":false,"avg":false,"low_battery":false,"hv_warning":false,"peak_max":false,"peak_min":false,"lead_error":false,"comp":false,"record":false,"loz":false,"void":false,"dc":false}}

--- 1 samples | Min: 4.6490 kΩ | Max: 4.6490 kΩ | Avg: 4.6490 kΩ
```
<!-- /snippet -->

Send remote commands, or switch the meter's mode, range, HOLD, REL, MIN/MAX and
Peak without touching it:

<!-- snippet via=mock:acv
dmm-cli command hold
dmm-cli set mode "AC V Hz"
dmm-cli set range 220V
-->
```
$ dmm-cli command hold
Sent hold

$ dmm-cli set mode "AC V Hz"
Meter now in AC V Hz

$ dmm-cli set range 220V
Meter now in 220V (manual range)
```
<!-- /snippet -->

The connected meter is detected automatically; `--device` pins a model:

```
$ dmm-cli --device ut8803 capture
WARNING: UNI-T UT8803 support is experimental (unverified against real hardware).
```

## Supported devices

<!-- devices:start -->
| Meter | USB | Bluetooth | Issue |
|---|---|---|---|
| UNI-T UT61E+ | ✅ Verified | ✅ Verified (adapter) | — |
| UNI-T UT61B+ | ✅ Verified | 🧪 Experimental (adapter) | [#19](https://github.com/antoinecellerier/dmm-tools/issues/19) |
| UNI-T UT61D+, UT161B/D/E | 🧪 Experimental | 🧪 Experimental (adapter) | [#7](https://github.com/antoinecellerier/dmm-tools/issues/7) |
| UNI-T UT60BT | — | 🧪 Experimental (built in) | [#26](https://github.com/antoinecellerier/dmm-tools/issues/26) |
| UNI-T UT202BT | — | 🧪 Experimental (built in) | [#27](https://github.com/antoinecellerier/dmm-tools/issues/27) |
| UNI-T UT171A/B/C | 🧪 Experimental | 🧪 Experimental (adapter) | [#4](https://github.com/antoinecellerier/dmm-tools/issues/4) |
| UNI-T UT181A | 🟡 Partly verified | 🧪 Experimental (adapter) | [#5](https://github.com/antoinecellerier/dmm-tools/issues/5) |
| UNI-T UT71A/B, UT71C/D/E | 🧪 Experimental | 🧪 Experimental (adapter) | [#22](https://github.com/antoinecellerier/dmm-tools/issues/22) |
| UNI-T UT803 | 🧪 Experimental | — | [#15](https://github.com/antoinecellerier/dmm-tools/issues/15) |
| UNI-T UT804 | ✅ Verified | — | [#16](https://github.com/antoinecellerier/dmm-tools/issues/16) |
| UNI-T UT8802, UT8802N | 🧪 Experimental | — | [#12](https://github.com/antoinecellerier/dmm-tools/issues/12) |
| UNI-T UT8803, UT8803E | 🧪 Experimental | — | [#3](https://github.com/antoinecellerier/dmm-tools/issues/3) |
| Voltcraft VC-880, VC650BT | 🧪 Experimental | — | [#13](https://github.com/antoinecellerier/dmm-tools/issues/13) |
| Voltcraft VC-890 | 🧪 Experimental | — | [#14](https://github.com/antoinecellerier/dmm-tools/issues/14) |
| Voltcraft VC920/VC940/VC960 | 🧪 Experimental | — | [#23](https://github.com/antoinecellerier/dmm-tools/issues/23) |
| ZOTEK ZT-300AB (ANENG AN9002) | — | 🧪 Experimental (built in) | [#28](https://github.com/antoinecellerier/dmm-tools/issues/28) |
| ZOTEK ZT-5566SE (ANENG AN999S) | — | 🧪 Experimental (built in) | [#29](https://github.com/antoinecellerier/dmm-tools/issues/29) |
| ZOTEK ZT-5BQ (ANENG ST207) | — | 🧪 Experimental (built in) | [#30](https://github.com/antoinecellerier/dmm-tools/issues/30) |
| ZOTEK ZT-5B (ANENG V05B) | — | 🧪 Experimental (built in) | [#31](https://github.com/antoinecellerier/dmm-tools/issues/31) |
| EEVblog 121GW | — | 🧪 Experimental (built in) | [#32](https://github.com/antoinecellerier/dmm-tools/issues/32) |
<!-- devices:end -->

✅ = confirmed on real hardware. 🟡 = connection and the main modes confirmed, the rest still to verify. 🧪 = reverse-engineered from vendor software, not yet tested on real hardware over that link. — = not supported over that link.

Bluetooth "adapter" is UNI-T's UT-D07A/B add-on; "built in" needs no adapter. Each issue collects hardware reports for its row; reports on the adapter itself go to [#25](https://github.com/antoinecellerier/dmm-tools/issues/25).

Cables, Bluetooth adapters, what to switch on and what each model has confirmed are in [supported devices](docs/supported-devices.md). Your model is not listed? [Open an issue](https://github.com/antoinecellerier/dmm-tools/issues) with its details.

## Quick start

1. Download the [latest release](https://github.com/antoinecellerier/dmm-tools/releases/latest) for your platform and extract it.
2. Give the tool access to the cable, or turn Bluetooth on:
   - Linux: install the udev rule, then re-plug the cable.
     ```sh
     sudo cp udev/70-dmm-tools.rules /etc/udev/rules.d/ && sudo udevadm control --reload-rules
     ```
   - Windows: the CP2110 cable may need the [Silicon Labs driver](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers); other cables need nothing.
   - macOS: nothing to install.
3. Switch the meter's data transmission on and run:
   ```sh
   dmm-cli read            # stream measurements
   dmm-gui                 # launch the GUI
   ```

[Setup & troubleshooting](docs/setup.md) covers drivers, udev details, headless machines, building from source and dev builds.

## Documentation

For users:

- [CLI reference](docs/cli-reference.md)
- [GUI reference](docs/gui-reference.md)
- [Setup & troubleshooting](docs/setup.md)
- [Supported devices](docs/supported-devices.md)
- [Changelog](CHANGELOG.md)

For contributors: [CONTRIBUTING.md](CONTRIBUTING.md) for bug reports, protocol captures and code changes; the [development guide](docs/development.md), [architecture](docs/architecture.md), [protocol details](docs/protocol.md), [UX design](docs/ux-design.md) and [adding a device](docs/adding-devices.md).

## License

GPL-3.0-or-later. See [LICENSE](LICENSE) for details.

## Acknowledgements

Community projects whose work informed or cross-checked ours, with thanks:

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) and [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — UT61E+ protocol, in Python and Go
- [gulux/Uni-T-CP2110](https://github.com/gulux/Uni-T-CP2110) — UT171 captures and parser
- [antage/ut181a](https://github.com/antage/ut181a) and [libsigrok `uni-t-ut181a`](https://github.com/sigrokproject/libsigrok/tree/master/src/hardware/uni-t-ut181a) — UT181A protocol
- [pylablib](https://github.com/AlexShkarin/pyLabLib) — VC-880 tables and flags
- [Silicon Labs AN434](https://www.silabs.com/documents/public/application-notes/an434-cp2110-4-interface-specification.pdf) — CP2110 HID-to-UART specification; [UNI-T](https://meters.uni-trend.com/download/ut61b-d-e-user-manual/) — the UT61+ user manual
