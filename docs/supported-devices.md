# Supported & Compatible Devices

<!-- Keep this file updated when adding support for new models. -->

All devices listed here use a HID-to-UART bridge for USB communication.
Two bridge chips are supported: Silicon Labs CP2110 (VID `0x10C4`, PID `0xEA80`)
and WCH CH9329 (VID `0x1A86`, PID `0xE429`). The tool auto-detects which
bridge is present. CH9329 support is experimental.

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

If you have any of the untested models, please [submit a capture](../CONTRIBUTING.md#protocol-captures) so we can confirm support and add the correct device tables. See [issue #7](https://github.com/antoinecellerier/dmm-tools/issues/7) for UT61D+/UT61B+ specific modes that need verification.

## Experimental: UT8802 and UT8803 (UCI protocol family)

These bench DMMs use the same CP2110 USB bridge but different
streaming protocols. Use `--device ut8802` or `--device ut8803`.

| Model | Brand | Type | VID:PID | Status | Notes |
|-------|-------|------|---------|--------|-------|
| **UT8802 / UT8802N** | UNI-T | Bench DMM | `10C4:EA80` | **Experimental** ([help verify](https://github.com/antoinecellerier/dmm-tools/issues/12)) | 0xAC header, 8-byte BCD frames, no checksum, streaming after 0x5A trigger |
| **UT8803 / UT8803E** | UNI-T | Bench DMM | `10C4:EA80` | **Experimental** ([help verify](https://github.com/antoinecellerier/dmm-tools/issues/3)) | 21-byte AB CD frames, streaming after 0x5A trigger |

## Future candidates (UCI protocol family)

| Model | Brand | Type | VID:PID | Status | Notes |
|-------|-------|------|---------|--------|-------|
| **UT803 / UT803N** | UNI-T | Bench DMM (6000 counts) | `1A86:E008` | Documented | QinHeng HID, FS9721 14-byte LCD segment protocol (not 0xAC/0xABCD) |
| **UT804 / UT804N** | UNI-T | Bench DMM (40000 counts) | `1A86:E008` | Documented | QinHeng HID, FS9721 14-byte LCD segment protocol (not 0xAC/0xABCD) |
| **UT805A / UT805N** | UNI-T | Bench DMM (220000 counts) | Serial | Documented | USB-to-serial (virtual COM port, not HID), ASCII text protocol (9600/8N1, bidirectional) |

**QinHeng HID (VID `0x1A86`, PID `0xE008`)** transport (WCH CH9325) is
implemented in `qinheng.rs`. The UT803/UT804 protocol (FS9721 14-byte LCD
segments) is not yet implemented — Ghidra decompilation of the standalone
UT803.exe/UT804.exe apps confirmed they use FS9721, not the 0xAC/0xABCD
UCI format that the uci.dll SDK auto-detects. The UT805A uses a serial
COM port (not HID) and needs a separate serial transport layer.

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
- **UT803/UT804**: Ghidra decompilation of standalone UT803.exe/UT804.exe
  (2026-04-10) revealed these meters use the **FS9721/FS9922 14-byte LCD
  segment protocol**, not the 0xAC/0xABCD UCI format. Evidence: CMP EBX,14
  frame loops, "123456789ABCDE" byte index validation, 7-segment decode
  tables. The UCI SDK (uci.dll) auto-detects 0xAC/0xABCD for QinHeng
  VID:PID but this is for a different operating mode.
- **UT805A**: Manual documents a bidirectional ASCII text protocol over
  USB-to-serial (9600/8N1, 10-byte frames + CR/LF, single-letter commands).
- **Extended UCI family analysis** ([approach](research/uci-bench-family/reverse-engineering-approach.md)):
  complete UT8802 wire protocol (0xAC 8-byte BCD frames), QinHeng HID
  init sequences and transport implementation, and per-model range tables
  from the programming manual.

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

## Experimental: UT171 and UT181A

Use `--device ut171` or `--device ut181a`. Requires manual "Communication
ON" in the meter's SETUP menu.

| Model | Brand | Type | VID:PID | Status | Notes |
|-------|-------|------|---------|--------|-------|
| **UT171A/B/C** | UNI-T | Industrial DMM | `10C4:EA80` | **Experimental** ([help verify](https://github.com/antoinecellerier/dmm-tools/issues/4)) | 1-byte length, LE float32, 26 modes |
| **UT181A** | UNI-T | Logging DMM | `10C4:EA80` | **Experimental** ([help verify](https://github.com/antoinecellerier/dmm-tools/issues/5)) | 2-byte LE length, float32 + unit strings, 97 modes |

## Experimental: Voltcraft VC-880 / VC650BT

Use `--device vc880` or `--device vc650bt`. Requires pressing the PC
button on the meter to enable USB communication.

| Model | Brand | Type | VID:PID | Status | Notes |
|-------|-------|------|---------|--------|-------|
| **VC-880** | Voltcraft | Handheld DMM (40000 counts) | `10C4:EA80` | **Experimental** ([help verify](https://github.com/antoinecellerier/dmm-tools/issues/13)) | AB CD framing (same as UT61E+), streaming, 19 modes, 7 status bytes |
| **VC650BT** | Voltcraft | Bench DMM (40000 counts) | `10C4:EA80` | **Experimental** ([help verify](https://github.com/antoinecellerier/dmm-tools/issues/13)) | Same protocol as VC-880 (byte-identical Voltsoft installer) |

### Independent research findings

Clean-room reverse engineering from Voltsoft `DMSShare.dll` (ILSpy
decompilation of .NET assembly, 26,600 lines). The VC-880 and VC650BT
installers are byte-identical (MD5: `4b955a1e8a51e7c89338c0c852e1c469`),
confirming shared protocol. Cross-referenced against pylablib (MIT).
See [docs/research/vc880/](research/vc880/).

### Cross-correlation with community sources

| Finding | Our RE (Voltsoft) | [pylablib](https://github.com/AlexShkarin/pyLabLib) | Agreement |
|---------|-------------------|---------------------|:---------:|
| AB CD header + BE16 checksum | Yes | Yes | ✓ |
| 19 function codes (0x00-0x12) | Yes (vendor switch) | Yes | ✓ |
| Range byte 0x30-based | Yes | Yes | ✓ |
| Status byte 1 flags (Rel/Avg/Min/Max) | Yes (all 28 flags named) | Yes (4 flags) | ✓ |
| Streaming (no trigger) | Yes | Yes | ✓ |
| Commands (auto/manual range) | Yes (28 commands total) | Yes (2 commands) | Our RE richer |

## Experimental: Voltcraft VC-890

Use `--device vc890`. 60,000-count OLED handheld DMM with ES51997P + EFM32
chipset. Polled protocol (request/response, like UT61E+) with 66-byte frames.
Confirmed as a separate protocol from VC-880 (different `VC890Reading` class
in Voltsoft, different installer binary, remapped function codes).

| Model | Brand | Type | VID:PID | Status | Notes |
|-------|-------|------|---------|--------|-------|
| **VC-890** | Voltcraft | Handheld DMM (60000 counts, OLED) | `10C4:EA80` | **Experimental** ([help verify](https://github.com/antoinecellerier/dmm-tools/issues/14)) | Polled, 66-byte frames, remapped function codes from VC-880 |

See [docs/research/vc890/](research/vc890/).

## Other CP2110 meters (not yet implemented)

### UNI-T

| Model | Brand | Type | Protocol | Reference |
|-------|-------|------|----------|-----------|
| **UT612** | UNI-T | LCR meter | ES51919 chipset, TX-only | [sigrok wiki](https://sigrok.org/wiki/UNI-T_UT612) |

### Non-UNI-T (Voltcraft / Conrad Electronics)

No unimplemented Voltcraft CP2110 meters remain — VC-880, VC650BT,
and VC-890 are all now experimentally supported.

The only shared component is the CP2110 transport — our existing
`Cp2110Transport` code works unmodified for the USB layer.

### Devices investigated but NOT using CP2110

These brands were evaluated during research and found to use different
USB bridge chips:

| Model | Brand | Bridge Chip | Notes |
|-------|-------|-------------|-------|
| **VC-870** | Voltcraft | CH9325 (UT-D04 cable) | ES51966A chipset, 40k counts, RS232/USB |
| **72-7730 / 72-7732** | Tenma | CH9325 / HE2325U (UT-D04) | UNI-T UT71 rebrands, HID but not CP2110 |
| **BM859 / BM869** | Brymen | IR-to-USB (BC-86X cable) | Infrared link, proprietary segment protocol |
| **70C / 86C** | Victor | Unmarked SO-20 | FS9922 14-byte HID report protocol |

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
| **UT-D09** (CP2110) | CP2110 | `10C4:EA80` | Bidirectional | Used by UT61x+, UT161x, UT171x, UT880x |
| **UT-D09** (CH9329) | CH9329 | `1A86:E429` | Bidirectional | Sold for UT181A, UT171 series, UT243; experimental support |
| **UT-D04** | CH9325 / HE2325U | `1A86:E008` | RX only | Used by older UNI-T meters (UT61E original, etc.) |
| **UT-D02** | RS232 level converter | N/A | Bidirectional | Serial port, no USB |

## Useful libraries

- [antage/cp211x_uart](https://github.com/antage/cp211x_uart) — Rust crate for CP2110/CP2114 UART control
- [rginda/pycp2110](https://github.com/rginda/pycp2110) — Python CP2110 library
- [pyserial CP2110 handler](https://github.com/pyserial/pyserial/blob/master/serial/urlhandler/protocol_cp2110.py) — CP2110 support built into pyserial
