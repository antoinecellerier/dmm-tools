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
| **UT61D+** | UNI-T | 6000 | Untested | Adds temperature and LoZ ACV |
| **UT61B+** | UNI-T | 6000 | Untested | Base model, 10A max current |
| **UT161E** | UNI-T | 22000 | Untested | Same as UT61E+ |
| **UT161D** | UNI-T | 6000 | Untested | Same as UT61D+ |
| **UT161B** | UNI-T | 6000 | Untested | Same as UT61B+ |
| **UT60BT** | UNI-T | — | Untested | Bluetooth variant; same serial protocol over BT serial |

### Independent research findings

Our clean-room reverse engineering of the vendor software confirms that
all six UT61+/UT161 models use identical protocol code:

- **Vendor software binary comparison** ([approach](research/ut61-family/reverse-engineering-approach.md)):
  UT161E installer has 67/69 files byte-identical to UT61E+ Software V2.02.
  Only DMM.exe (model name string, 8 bytes) and options.xml (model tag)
  differ.
- **Zero model-specific protocol logic** in any of the four decompiled
  binaries (CustomDmm.dll, DMM.exe, CP2110.dll, DeviceSelector.dll). All
  framing, parsing, command generation, and mode/range table lookups are
  shared.
- **Per-model differences** are documented in
  [reverse-engineered-protocol.md](research/ut61-family/reverse-engineered-protocol.md):
  display count (6000 vs 22000), bar graph segments (31 vs 46), available
  modes, and range tables.

### Cross-correlation with community sources

| Finding | Our RE | [ljakob](https://github.com/ljakob/unit_ut61eplus) | [mwuertinger](https://github.com/mwuertinger/ut61ep) | Agreement |
|---------|--------|--------|------------|:---------:|
| Same protocol for B+/D+/E+ | Yes (vendor code) | Yes (per-model tables, same framing) | N/A (E+ only) | ✓ |
| Same protocol for UT161 series | Yes (binary-identical software) | Yes (explicit UT161 support) | N/A | ✓ |
| UT60BT over Bluetooth | Not investigated | Yes (BT serial support) | N/A | — |
| 6000-count range tables | From manual | Per-model tables in code | N/A | To verify |
| Mode byte values 0x00-0x14 | Vendor software table | Same values | Same values | ✓ |
| LoZ mode 0x15/0x16 | Vendor has both; code treats differently | Uses 0x15 only | N/A | Partial |

**Key discrepancy**: ljakob's implementation uses only mode 0x15 for LoZ,
while the vendor software has entries for both 0x15 and 0x16 with
different display value handling. Requires UT61D+ device testing.

If you have any of the untested models, please [submit a capture](../CONTRIBUTING.md#protocol-captures) so we can confirm support and add the correct device tables.

## Future candidates (UCI protocol family)

These bench DMMs use the same CP2110 USB bridge but a fundamentally
different protocol: UNI-T's UCI (United Communication Interface) SDK
with streaming measurement frames and a 64-bit flags word. Supporting
them requires a separate protocol implementation.

| Model | Brand | Type | VID:PID | Status | Notes |
|-------|-------|------|---------|--------|-------|
| **UT8803 / UT8803E** | UNI-T | Bench DMM | `10C4:EA80` | RE complete | 21-byte AB CD frames, streaming after 0x5A trigger |
| **UT8802 / UT8802N** | UNI-T | Bench DMM | `10C4:EA80` | RE complete | 0xAC header, 8-byte BCD frames, no checksum |
| **UT632 / UT632N** | UNI-T | Bench DMM | `1A86:E008` | Documented | QinHeng HID with auto-detect (0xAC or 0xABCD) |
| **UT803 / UT803N** | UNI-T | Bench DMM | `1A86:E008` | Documented | QinHeng HID with auto-detect (0xAC or 0xABCD) |
| **UT804 / UT804N** | UNI-T | Bench DMM | `1A86:E008` | Documented | QinHeng HID with auto-detect, range table in programming manual |
| **UT805A / UT805N** | UNI-T | Bench DMM | Serial | Documented | Serial (9600, DATA:7 per manual), range table in programming manual |

### Independent research findings

Our clean-room RE of the UCI bench family is documented in
[docs/research/ut8803/](research/ut8803/) and
[docs/research/uci-bench-family/](research/uci-bench-family/):

- **Programming manual** (official UNI-T document): fully specifies the
  UCI API including DMFRM struct, 64-bit flags word, functional/position
  coding tables, and all supported models.
- **Ghidra decompilation of uci.dll** (451K lines): revealed the raw wire
  protocol under the UCI abstraction — 21-byte AB CD frames with
  alternating-byte checksum, confirmed 9600 baud, and discovered the
  streaming model (single 0x5A trigger byte starts continuous data).
- **UT8802 vs UT8803**: different wire formats. UT8802 uses 0xAC single-byte
  header with 8-byte frames; UT8803 uses standard AB CD header with
  21-byte frames.
- **UT632/UT803/UT804**: share the UCI protocol but use QinHeng HID
  (VID 0x1A86, PID 0xE008) instead of CP2110.
- **Extended UCI family analysis** ([approach](research/uci-bench-family/reverse-engineering-approach.md)):
  complete UT8802 wire protocol (0xAC 8-byte BCD frames), QinHeng HID
  init sequences, frame auto-detection, serial transport analysis, and
  per-model range tables from the programming manual.

### Cross-correlation with community sources

| Finding | Our RE | [philpagel](https://github.com/philpagel/ut8803e) | [hskim7639](https://github.com/hskim7639/UNI-T) | Agreement |
|---------|--------|-----------|-----------|:---------:|
| CP2110 bridge (0x10C4:0xEA80) | Programming manual + Ghidra | Same | Same | ✓ |
| 9600 baud | Ghidra (feature report 0x50) | 9600 in code | N/A | ✓ |
| AB CD frame header | Ghidra parser | Same | Same | ✓ |
| 21-byte frames | Ghidra (min frame size 0x15) | "19 byte data frame" | N/A | ~¹ |
| Streaming model | Ghidra (0x5A trigger, read-only loop) | Continuous read | N/A | ✓ |
| Alternating-byte checksum | Ghidra | "Weighted checksum" | N/A | To verify² |
| Mode/range tables | Programming manual + Ghidra | Empirical tables | N/A | To verify |
| UT8802 different format | Ghidra (0xAC header, 8-byte) | N/A | N/A | — |
| UT632/803/804 same protocol | Programming manual | Listed as compatible | N/A | ✓ |

¹ philpagel counts 19 data bytes (excluding header); our 21 includes the 2-byte AB CD header.
  These are consistent.

² philpagel describes a "weighted checksum" which may refer to the
  alternating-byte sum we found. Needs detailed comparison.

### Reference implementations

- [philpagel/ut8803e](https://github.com/philpagel/ut8803e) — Python, UT8803/UT8803E, detailed protocol docs
- [hskim7639/UNI-T](https://github.com/hskim7639/UNI-T) — Python (Windows), UT8803E

## Other CP2110 meters (different protocols)

These meters use CP2110 for USB transport but have fundamentally different
serial protocols. Listed here for reference — supporting them would be
separate implementation efforts.

| Model | Brand | Type | Protocol | Reference |
|-------|-------|------|----------|-----------|
| **UT171A/B/C** | UNI-T | Industrial DMM | RE complete: 0xABCD header, 1-byte length, LE float32, 26 modes, data logging | [gulux/Uni-T-CP2110](https://github.com/gulux/Uni-T-CP2110), [smartypies.com](http://www.smartypies.com/projects/ut171a-data-reader-on-linux/) |
| **UT181A** | UNI-T | Logging DMM | 0xABCD header¹, 2-byte LE length, float32 values, 97 modes, recording | [antage/ut181a](https://github.com/antage/ut181a) (Rust, with [protocol docs](https://github.com/antage/ut181a/blob/master/Protocol.md)), [loblab/ut181a](https://github.com/loblab/ut181a) (C++), [sigrok](https://sigrok.org/wiki/UNI-T_UT181A) |
| **UT612** | UNI-T | LCR meter | ES51919 chipset, TX-only | [sigrok wiki](https://sigrok.org/wiki/UNI-T_UT612) |
| **Voltcraft VC-890** | Voltcraft (UNI-T rebrand) | DMM | ES51997P chipset, own protocol | [sigrok wiki](https://sigrok.org/wiki/Voltcraft_VC-890) |

¹ The UT181A wire bytes are 0xAB, 0xCD -- **same as UT61E+**. Some sources
describe the header as "0xCDAB" because the UT181A protocol reads these bytes
as a little-endian uint16 (0xCDAB), while UT61E+ interprets them big-endian
(0xABCD). The actual difference is in the length field (2 bytes LE vs 1 byte),
checksum (LE vs BE), and value encoding (float32 vs ASCII).

### Independent research findings

Our research into the UT171 and UT181A protocols is documented in
[docs/research/ut171/](research/ut171/) and
[docs/research/ut181/](research/ut181/):

- **UT171**: Clean-room RE from official vendor software
  ([approach](research/ut171/reverse-engineering-approach.md)). Ghidra
  decompilation of UT171C.exe (881K lines) and SLABHIDtoUART.dll (13K
  lines) reveals: 26 mode bytes mapped, complete flag byte decoding
  (AUTO inverted bit 6, HOLD bit 7), data logging commands
  (0x01/0x51/0x52/0xFF), range byte is raw 1-based index, mode
  transition command table, and full CP2110 feature report map (20
  report IDs). Cross-referenced against
  [gulux/Uni-T-CP2110](https://github.com/gulux/Uni-T-CP2110).
- **UT181A**: Fully documented by the community (3 independent
  implementations agree on every detail). 97 measurement modes, complete
  recording/data logging protocol, COMP mode. The
  [antage/ut181a Protocol.md](https://github.com/antage/ut181a/blob/master/Protocol.md)
  is the definitive reference.

## USB cables

| Cable | Chip | VID:PID | Direction | Notes |
|-------|------|---------|-----------|-------|
| **UT-D09** | CP2110 | `10C4:EA80` | Bidirectional | Used by UT61x+, UT161x, UT171x, UT880x |
| **UT-D04** | CH9325 / HE2325U | `1A86:E008` | RX only | Used by older UNI-T meters (UT61E original, etc.) |
| **UT-D02** | RS232 level converter | N/A | Bidirectional | Serial port, no USB |

## Useful libraries

- [antage/cp211x_uart](https://github.com/antage/cp211x_uart) — Rust crate for CP2110/CP2114 UART control
- [rginda/pycp2110](https://github.com/rginda/pycp2110) — Python CP2110 library
- [pyserial CP2110 handler](https://github.com/pyserial/pyserial/blob/master/serial/urlhandler/protocol_cp2110.py) — CP2110 support built into pyserial
