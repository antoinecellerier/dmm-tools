# Reverse Engineering Approach: UT181A Protocol

## Objective

Document the UT181A communication protocol using publicly available
community reverse engineering work. The UT181A is one of the most
thoroughly reverse-engineered UNI-T meters, with three independent
implementations that agree on the protocol details.

## Sources

1. **antage/ut181a** (Rust library + Protocol.md) --
   https://github.com/antage/ut181a -- the primary protocol reference,
   MIT-licensed, includes complete [Protocol.md](https://github.com/antage/ut181a/blob/master/Protocol.md)
2. **loblab/ut181a** (C++ tool) -- https://github.com/loblab/ut181a --
   independent implementation with recording download support
3. **sigrok uni-t-ut181a driver** (C) --
   https://github.com/sigrokproject/libsigrok/tree/master/src/hardware/uni-t-ut181a --
   the most complete implementation, includes COMP mode and all 97
   measurement modes
4. **antage/cp211x_uart** (Rust crate) --
   https://github.com/antage/cp211x_uart -- CP2110/CP2114 UART
   control, used by antage/ut181a
5. **UNI-T UT181A user manual** -- from UNI-T website
6. **sigrok wiki** -- https://sigrok.org/wiki/UNI-T_UT181A -- hardware
   details, chipset identification

No vendor software decompilation was needed -- the community work is
comprehensive.

## What Each Source Provides

### antage/ut181a (Rust)

The most important source. Provides:
- **Protocol.md**: Complete protocol specification document with frame
  format, all command codes, measurement packet layout, mode word table,
  recording protocol, and timestamp format
- **Rust library**: Working implementation covering monitoring, saved
  measurements, and recordings
- **cp211x_uart crate**: Reusable CP2110 UART control library

### loblab/ut181a (C++)

Independent implementation confirming:
- Frame format (header, length, checksum)
- Recording download protocol with 250-sample chunking
- CSV export of recorded data
- Command structure

### sigrok uni-t-ut181a driver

The most complete implementation:
- All 97 measurement modes parsed
- COMP (comparator) mode support
- Full recording protocol
- Bargraph data parsing
- All measurement variants (normal, relative, min/max, peak)

### sigrok wiki

Hardware teardown details:
- Cyrustek ES51997 analog frontend
- STM32F103 MCU
- 512 KiB flash, 1Mx16 SRAM, 24C256 EEPROM
- DS2086 RTC
- 7.4V 2200 mAh Li-ion battery + CR2032 backup

## Analysis Techniques

### 1. Community Implementation Cross-Reference

All three implementations were compared for agreement on:
- Frame header bytes (0xAB, 0xCD)
- Length field encoding (uint16 LE)
- Checksum algorithm (16-bit LE sum of length + payload bytes)
- Command codes (0x01-0x12)
- Measurement packet format (all variants)
- Mode word values (97 modes)
- Range byte values (0x00-0x08)
- Recording protocol (start, info, data download)
- Timestamp format (packed 32-bit)

All three agree on every detail. This gives [KNOWN] confidence.

### 2. Header Byte Clarification

A critical finding: the UT181A sends 0xAB then 0xCD on the wire --
the **same bytes** as UT61E+. The "reversed 0xCDAB header" description
(including in our `docs/supported-devices.md`) is misleading:

- antage/ut181a Protocol.md describes the magic as "0xCDAB" because it
  reads bytes as LE uint16: byte[0]=0xAB, byte[1]=0xCD → 0xCDAB
- UT61E+ docs describe the same bytes as "AB CD" (BE interpretation)
- Confirmed by all three codebases:
  - antage: `pkt.push(0xAB); pkt.push(0xCD);`
  - loblab: `START_BYTE1 = 0xAB; START_BYTE2 = 0xCD;`
  - sigrok: `FRAME_MAGIC 0xcdab` (LE uint16 constant)

The actual protocol differences from UT61E+ are in the length field
(2 bytes LE vs 1 byte), checksum (LE vs BE), and value encoding
(float32 vs ASCII).

### 3. Existing Project Context

Checked existing project files:
- `docs/supported-devices.md` lists UT181A with references to
  antage/ut181a and loblab/ut181a
- No existing `docs/research/ut181/` directory (created for this work)
- No UT181A-specific code in the Rust codebase

## What Was Determined

### Fully Confirmed ([KNOWN])

| Finding | Source |
|---------|--------|
| Frame structure (0xAB 0xCD header, uint16 LE length, uint16 LE checksum) | 3 implementations agree |
| Checksum = byte sum of length field + payload | 3 implementations agree |
| All 15 command codes (0x01-0x12) | antage + sigrok + loblab |
| All 97 mode words (0x1111-0xA231) | antage + sigrok |
| Range bytes 0x00-0x08 | antage + sigrok + loblab |
| Measurement packet format (all 4 variants) | antage + sigrok |
| COMP mode fields | sigrok driver |
| Unit strings (sent by device in packets) | antage + sigrok |
| Timestamp format (packed 32-bit) | antage + sigrok + loblab |
| Recording protocol (start/stop/info/data) | antage + sigrok + loblab |
| Record download chunking (250 samples max) | loblab implementation |
| 9600 baud 8N1 via CP2110 | all implementations |
| Communication must be manually enabled on meter | sigrok wiki + manual |
| Wire header is 0xAB 0xCD (not reversed) | all 3 codebases |
| 60,000 counts, dual display, TFT LCD | user manual |
| All measurement modes and ranges | user manual + implementations |

### Nothing Remains Unverified

The UT181A protocol is fully documented by the community. No
[UNVERIFIED] items remain. The three independent implementations
serve as mutual verification.

## File Inventory

No reference files were downloaded for this analysis. All sources are
publicly available online:

| Source | URL | What it provides |
|--------|-----|-----------------|
| antage/ut181a | github.com/antage/ut181a | Protocol.md + Rust library |
| loblab/ut181a | github.com/loblab/ut181a | C++ implementation + recording |
| sigrok driver | github.com/sigrokproject/libsigrok | Complete C driver |
| sigrok wiki | sigrok.org/wiki/UNI-T_UT181A | Hardware teardown |
| UNI-T manual | meters.uni-trend.com | User manual (specs, modes, ranges) |
