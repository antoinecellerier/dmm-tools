# UT8802 verification

Open checks for the UT8802 and UT8802N, issue
[#12](https://github.com/antoinecellerier/dmm-tools/issues/12). No UT8802 has
been captured: every entry below rests on uci.dll and UNI-T's programming
manual. What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). The UT8803 has its own
spec, [`../ut8803/`](../ut8803/reverse-engineered-protocol.md).

## Stream

- Streaming with no 0x5A sent ([§7.1](reverse-engineered-protocol.md#71-multi-transport-support))
  — decides whether `Ut8802Protocol::init` must send a trigger. Needs the
  CP2110 cable and `RUST_LOG=dmm_lib=trace dmm-cli --device ut8802 debug`. (#12)
- Frames of exactly 8 bytes, back to back ([§3.1](reverse-engineered-protocol.md#31-frame-format))
  — decides the extractor and detection's two-adjacent-frames rule. Needs
  that trace. (#12)
- The frame rate, which no source gives — decides whether one LCD update is
  one frame. Needs that trace with timestamps. (#12)

## Positions and units

- Each dial position's code against the 35 in [§3.3](reverse-engineered-protocol.md#33-position-code-table----known--vendor)
  — decides `POSITION_TABLE` and the extractor's valid codes. Needs every
  capture step, and each manual range stepped through on V, A and Ω. (#12)
- Range-relative digits: a 200 mV reading arriving as "123.45" mV, 1.234 kΩ
  on the 2 kΩ range as "1.234" (§3.3) — decides the prefixes in
  `POSITION_TABLE`'s units. Needs `dcv` on 200 mV and a known resistor. (#12)

## Digits

- Digit order, MSD in byte 4's low nibble, and the point in byte 5's ([§3.2](reverse-engineered-protocol.md#32-display-encoding----vendor))
  — decides `parse_measurement`'s nibble order. Needs any reading with
  distinct digits, noting the LCD. (#12)
- Overload: byte 7 bit 6, 0x0C nibbles, both or neither, and when bit 5
  (over-range) sets (§3.2, [§3.5](reverse-engineered-protocol.md#35-byte-7-status-flags----vendor))
  — decides whether the 0x0C digit check stays. Needs `ohm` with leads open. (#12)
- Which nibble a blank leading digit sends, 0x0 or another (§3.2) — decides
  `bcd_to_char` and the extractor's valid nibbles. Needs a reading under
  10000 counts. (#12)
- Byte 7 bit 7 on a negative reading (§3.2) — confirms the sign and the `-`
  in the display text. Needs `dcv_negative`. (#12)

## Flags

- HOLD, REL, MAX and MIN at byte 7 bits 4, 3, 1 and 0, and AUTO on bit 2
  clear (§3.5) — decides the flags `parse_measurement` sets. Needs each key
  pressed with the trace running; no capture step covers them yet. (#12)

## Other bytes

- What byte 6 carries, a bar graph or other status ([§3.6](reverse-engineered-protocol.md#36-byte-6-purpose----unverified))
  — decides whether it becomes a reading field. Needs traces across modes,
  one with a reading swept towards full scale. (#12)
- Byte 5 bits 4-5: the diode and SCR direction values, and anything outside
  them ([§3.4](reverse-engineered-protocol.md#34-byte-5-flags-bits-4-5----vendor))
  — decides whether they stay unread elsewhere. Needs `diode`, `scr`. (#12)

## Detection

- The UT8802 detection row, and whether a streaming meter on Linux HID reads
  behind the LCD: in the verification backlog's [detection](../../verification-backlog.md#device-auto-detection)
  and [streaming](../../verification-backlog.md#streaming-meters-read-continuously) sections.

## Vendor sources

- Whether UNI-T's general-purpose PC software ([backlog](../../verification-backlog.md#vendor-sources-not-yet-read)),
  listed among the UT88 results, drives a UT8802 and how it decodes it —
  could settle several items before a capture. Needs the download unpacked;
  no meter. (#12)
