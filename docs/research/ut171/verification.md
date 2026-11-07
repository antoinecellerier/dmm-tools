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

## Remote keys

None of these has been sent to a meter. Each decides whether its button stays.

- HOLD `07 5A`, AUTO `02 00`, MAX/MIN `05 01`/`05 00`, Hz% `03 5A` and STORE
  `0B 5A` ([§4.7](reverse-engineered-protocol.md#47-function-range-and-key-commands----vendor)):
  does the meter act as on its own key? Needs the `hold`, `auto`, `minmax`,
  `hz_key` steps, and `dmm-cli --device ut171 command save` for STORE. (#4)
- Hz% in VFC: the manual names it in V~, I~ and Hz, not in VFC, which we
  offer it in as a V~ function (§4.7) — decides VFC's Hz/% button. Needs
  `dmm-cli command hz` in VFC. (#4)
- RANGE as `02 <row>`: the current row from auto, the next one in manual, and
  whether the meter refuses it where the manual has no RANGE (continuity and
  temperature, which have a row in UNI-T's software) (§4.7) — decides the
  stepping. Needs `manual_range`, and `dmm-cli command range` in those. (#4)
- REL as `04 01` with the reading's own float: does the meter zero on it as on
  its key, and `04 00` leave (§4.7) — decides the REL button. Needs `rel`, `rel_off`. (#4)
- Peak as `06 01`/`06 00`, chosen by byte 5 bit 6, and which bit tells P-Max
  from P-Min (§4.7, §5.3) — decides the PEAK button and a Peak flag. Needs
  `peak`, `peak_off`. (#4)
- The reply: OK, ER or NO within 300 ms, and the connect frame's own reply, if
  any, which the first key's wait may take for its own
  ([§5.11](reverse-engineered-protocol.md#511-replies-types-0x01-and-0x72)) —
  decides the reply wait. Needs any key step. (#4)
- Function select `01 MM` within a dial position's functions, and range
  select `02 NN`, against the meter's own SELECT and RANGE (§4.7, §6) —
  decides the function and range dropdowns. Needs `capture`'s mode and range
  walks. (#4)
- MAX/MIN while REL or Peak is on: UNI-T's software sends nothing then, and we
  send it (§4.7) — decides whether to hold it back. Needs `dmm-cli command
  minmax` with REL on. (#4)
- The square wave's dial position: the manual's text reaches it "in uA gear",
  its Fig 13 shows a ⎍OUT stop, and UNI-T's software lights its button with
  the clamp's (§6) — decides the function list it sits in. Needs a UT171C. (#4)
- What MAX/MIN, REL and Hz% (`05 01`, `04 01`, `03 5A`) do in the square
  wave, where the manual gives those keys the frequency and duty (§4.7, §5.9)
  — decides those buttons' hovers there. Needs a UT171C on `square_wave`. (#4)
- On a UT171A, which lacks nS, temperature, the square wave and the clamp
  (§1.1), and % 4-20mA (the manual's SELECT list, B/C only): what the
  function select's `01 MM` does to one (§6) — decides whether the A's list
  drops them. Needs a UT171A and `dmm-cli set mode`. (#4)

## Detection and the adapter

- What detection's probes do to a UT171, and its frames against the UT181A's:
  [Device auto-detection](../../verification-backlog.md#device-auto-detection).
- A UT171 behind the UT-D07A or UT-D07B: in the
  [UT-D07B checks](../ut-d07b/verification.md#meters-behind-the-adapter).

## Vendor sources

- The UT171 series PC software UNI-T uploaded 2023-02-03 ([candidates](../new-device-candidates.md#uni-t-chinese-sites))
  — hash-compare it with the UT171C setup the approach doc traced; decides
  whether the spec covers the current app.
