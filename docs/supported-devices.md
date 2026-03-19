# Supported & Compatible Devices

<!-- Keep this file updated when adding support for new models. -->

All devices listed here use the Silicon Labs CP2110 HID-to-UART bridge
(VID `0x10C4`, PID `0xEA80`) for USB communication.

## Supported (same protocol as UT61E+)

These meters share the same 0xABCD-framed request/response protocol and
differ only in their mode/range/unit tables. Adding support requires
implementing a `DeviceTable` with the correct tables — no protocol changes.

| Model | Brand | Counts | Status | Notes |
|-------|-------|--------|--------|-------|
| **UT61E+** | UNI-T | 22000 | Tested | Reference device |
| **UT61D+** | UNI-T | 6000 | Untested | Adds temperature, shares manual/USB module with UT61E+ |
| **UT61B+** | UNI-T | 6000 | Untested | Base model, shares manual/USB module with UT61E+ |
| **UT161E** | UNI-T | 22000 | Untested | Higher-end variant, same protocol confirmed by [ljakob](https://github.com/ljakob/unit_ut61eplus) |
| **UT161D** | UNI-T | 6000 | Untested | Higher-end variant, same protocol confirmed by [ljakob](https://github.com/ljakob/unit_ut61eplus) |
| **UT161B** | UNI-T | 6000 | Untested | Higher-end variant, same protocol confirmed by [ljakob](https://github.com/ljakob/unit_ut61eplus) |
| **UT60BT** | UNI-T | — | Untested | Bluetooth variant; same serial protocol over BT serial ([ljakob](https://github.com/ljakob/unit_ut61eplus)) |

If you have any of the untested models, please [submit a capture](../CONTRIBUTING.md#protocol-captures) so we can confirm support and add the correct device tables.

### Reference implementations

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) — Python, most complete; supports UT61B+/D+/E+, UT161B/D/E, UT60BT with per-model tables
- [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — Go, UT61E+ only

## Future candidates (related protocol)

These meters use CP2110 and the same 0xABCD magic header, but have a
different command format (7-byte commands, continuous streaming at ~3 Hz
instead of poll-based). Supporting them would require a new protocol
dialect, not just new device tables.

| Model | Brand | Type | Status | Notes |
|-------|-------|------|--------|-------|
| **UT8803 / UT8803E** | UNI-T | Bench DMM | Confirmed CP2110 | 7-byte commands, continuous streaming |
| **UT8802 / UT8802N** | UNI-T | Bench DMM | Suspected same protocol | Per [philpagel](https://github.com/philpagel/ut8803e) |
| **UT632 / UT632N** | UNI-T | Bench DMM | Suspected same protocol | Per [philpagel](https://github.com/philpagel/ut8803e) |
| **UT803 / UT803N** | UNI-T | Bench DMM | Suspected same protocol | Per [philpagel](https://github.com/philpagel/ut8803e) |

### Reference implementations

- [philpagel/ut8803e](https://github.com/philpagel/ut8803e) — Python, UT8803/UT8803E bench meters, detailed protocol docs
- [hskim7639/UNI-T](https://github.com/hskim7639/UNI-T) — Python (Windows), UT8803E

## Other CP2110 meters (different protocols)

These meters use CP2110 for USB transport but have fundamentally different
serial protocols. Listed here for reference — supporting them would be
separate implementation efforts.

| Model | Brand | Type | Protocol | Reference |
|-------|-------|------|----------|-----------|
| **UT171A/B/C** | UNI-T | Industrial DMM | 0xABCD header, LE floats, proprietary Unitrend format | [gulux/Uni-T-CP2110](https://github.com/gulux/Uni-T-CP2110), [smartypies.com](http://www.smartypies.com/projects/ut171a-data-reader-on-linux/) |
| **UT181A** | UNI-T | Logging DMM | Reversed 0xCDAB header, float32 values, complex command structure | [antage/ut181a](https://github.com/antage/ut181a) (Rust, with [protocol docs](https://github.com/antage/ut181a/blob/master/Protocol.md)), [loblab/ut181a](https://github.com/loblab/ut181a) (C++) |
| **UT612** | UNI-T | LCR meter | ES51919 chipset, TX-only | [sigrok wiki](https://sigrok.org/wiki/UNI-T_UT612) |
| **Voltcraft VC-890** | Voltcraft (UNI-T rebrand) | DMM | ES51997P chipset, own protocol | [sigrok wiki](https://sigrok.org/wiki/Voltcraft_VC-890) |

## USB cables

| Cable | Chip | VID:PID | Direction | Notes |
|-------|------|---------|-----------|-------|
| **UT-D09** | CP2110 | `10C4:EA80` | Bidirectional | Used by UT61x+, UT161x, UT171x |
| **UT-D04** | CH9325 / HE2325U | `1A86:E008` | RX only | Used by older UNI-T meters (UT61E original, etc.) |
| **UT-D02** | RS232 level converter | N/A | Bidirectional | Serial port, no USB |

## Useful libraries

- [antage/cp211x_uart](https://github.com/antage/cp211x_uart) — Rust crate for CP2110/CP2114 UART control
- [rginda/pycp2110](https://github.com/rginda/pycp2110) — Python CP2110 library
- [pyserial CP2110 handler](https://github.com/pyserial/pyserial/blob/master/serial/urlhandler/protocol_cp2110.py) — CP2110 support built into pyserial
