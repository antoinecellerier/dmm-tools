# UT171 verification

Open checks for the UT171A, UT171B and UT171C, issue
[#4](https://github.com/antoinecellerier/dmm-tools/issues/4).
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). No UT171 has been read
yet: every item waits on a first report, and everything the spec says of the
frames is what UNI-T's software reads.

## First report

- `dmm-cli --device ut171 capture --unverified`, with USB communication on in
  the meter's setup menu (`Usb`) — one measurement frame closes the framing
  ([§3.1](reverse-engineered-protocol.md#31-general-structure)) and settles
  most items below. Needs any UT171 and its USB cable. (#4)

## Frames

- Which modes set the aux and bar bits (byte 5 bits 0 and 3), and the aux
  unit code each sends ([§5.2](reverse-engineered-protocol.md#52-optional-blocks-aux-bar-and-sto-minutes))
  — decides the aux label per mode. Needs `vac`, `vacdc`, `hz` and Hz% presses. (#4)
- The aux block under REL: the reference the manual puts on the auxiliary
  display, whatever its unit code (§5.2) — decides the `Reference` label. Needs `rel`. (#4)
- NCV's live frame: its length and the bytes after the mode, at `EF` and at
  each dash level ([§5.8](reverse-engineered-protocol.md#58-ncv-mode-0x1a)) —
  decides the NCV reading. Needs `ncv`, away from and near a live wire. (#4)
- The square-wave frame: frequency, duty, width and its decimals against the
  meter's settings ([§5.9](reverse-engineered-protocol.md#59-square-wave-output-mode-0x1d))
  — decides the square-wave reading. Needs a UT171C on `square_wave`. (#4)

## Flags

- HOLD, REL, Peak, STO and AUTO: byte 5 bits 7, 4, 6, 1 and byte 6 bit 0
  ([§5.3](reverse-engineered-protocol.md#53-status-bits-bytes-5-and-6)) —
  decides the flags read. Needs HOLD, REL, `peak`, a timed store and `manual_range`. (#4)
- Byte 6 bits 5–6 against the meter's MAX, AVG and MIN, and byte 5 bit 5 when
  neither of them is set (§5.3) — decides the MAX/MIN flags. Needs `max_min`. (#4)
- Byte 5 bit 2 as low battery: no reader in the app, gulux alone
  ([§10](reverse-engineered-protocol.md#cross-reference-with-community-sources))
  — decides `low_battery`. Needs a meter showing its battery symbol. (#4)
- Byte 6 bit 1 against the lightning bolt, the manual's "Alarm for high
  voltage" (§5.3) — decides `hv_warning`. Needs V AC or V DC high enough to
  light it; the manual states no threshold. (#4)
- Byte 6 bit 2, on which the app skips logging, meaning unknown, and bits 3,
  4 and 7, which it never reads (§5.3) — decides whether they stay silent.
  Needs the whole capture. (#4)

## Modes

- The mode bytes of [§6](reverse-engineered-protocol.md#6-mode-byte-table----vendor),
  the app's map; 0x0B above all: capacitance in the app, diode in gulux
  (§10) — decides the mode table. Needs every mode step, `cap` and `diode` first. (#4)
- That 0x10 and 0x24 never appear, and that duty stays mode 0x0F (§6) —
  decides whether either returns to the table. Needs `duty` and every mode step. (#4)

## Ranges

- The range byte per row of [§5.4](reverse-engineered-protocol.md#54-range-byte-offset-8),
  1-based, AUTO in byte 6 bit 0 — decides the range labels. Needs
  `manual_range` and RANGE presses in Ω, capacitance, Hz and the current modes. (#4)
- A UT171A's range bytes: the app's rows and our labels are the 60,000-count
  B/C's (6/60/600), the A's manual column 4/40/400 (§5.4) — decides the A's
  labels. Needs a UT171A across ranges. (#4)
- The manual's 10 A and 10 MHz tops against the app's 20 A and 60 MHz rows
  (§5.4) — decides the last A and Hz labels. Needs `adc`, `aac` and `hz` on
  their top range. (#4)

## Values

- The unit code byte 14 carries per mode and range, which the app shows
  unscaled ([§5.6](reverse-engineered-protocol.md#56-unit-codes-and-scaling))
  — decides every reading's unit. Needs `ohm`, `cap`, `hz` and `ns` across ranges. (#4)
- Byte 13's states: `OL`, `-OL`, dashes, and `LEAD`, `DISC`, `Lo`, `Hi`, the
  app's words, none in the manual's LCD table ([§5.5](reverse-engineered-protocol.md#55-byte-13-decimals-and-display-state))
  — decides overload against a word. Needs `ohm` open, an overrange and a lead in the wrong jack. (#4)

## Commands

- Whether the meter streams without the connect frame once communication is
  on ([§4.3](reverse-engineered-protocol.md#43-streaming-commands----vendor))
  — decides whether `init` must send it. Needs a trace before `connect`. (#4)

## Detection and the adapter

- What detection's probes do to a UT171, and its frames against the UT181A's:
  [Device auto-detection](../../verification-backlog.md#device-auto-detection).
- A UT171 behind the UT-D07A or UT-D07B: in the
  [UT-D07B checks](../ut-d07b/verification.md#meters-behind-the-adapter).

## Vendor sources

- The UT171 series PC software UNI-T uploaded 2023-02-03 ([candidates](../new-device-candidates.md#uni-t-chinese-sites))
  — hash-compare it with the UT171C setup the approach doc traced; decides
  whether the spec covers the current app.
