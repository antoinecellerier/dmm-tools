# UT61+ family verification

Open checks for the UT61B+, UT61D+, UT161B/D/E, UT60BT and UT202BT, and for
the commands, dials and range tables every model runs, our UT61E+ included;
issues [#6](https://github.com/antoinecellerier/dmm-tools/issues/6),
[#7](https://github.com/antoinecellerier/dmm-tools/issues/7),
[#19](https://github.com/antoinecellerier/dmm-tools/issues/19),
[#20](https://github.com/antoinecellerier/dmm-tools/issues/20),
[#26](https://github.com/antoinecellerier/dmm-tools/issues/26) and
[#27](https://github.com/antoinecellerier/dmm-tools/issues/27).
What real meters have confirmed is tagged `[VERIFIED]` in the
[spec](reverse-engineered-protocol.md). The UT61E+'s frames, flags and CP2110
cable are in [ut61eplus](../ut61eplus/verification.md); checks that span
families are in the [verification backlog](../../verification-backlog.md).

## Dial rings

- The UT61B+ rings no driver has pressed ([§3.1][s31]): `set mode duty` from
  AC mV, AC µA, AC mA and AC A, SELECT between DC mV and AC mV, and SELECT on
  V~, which should do nothing — decides `ut61b_plus.rs`'s dial table. (#7)
- The UT61D+ and UT161 dial tables, from the manual alone ([§3.1][s31]): on
  each position `get mode` lists what SELECT and Hz/% reach and each `set
  mode` lands — AC V/DC V on the V≂ SELECT, °C/°F on temperature. (#7)
- Hz → Duty % → AC V on a UT61B+, which stopped at Duty % with a timeout
  ([§3.1][s31], [§6.1][s61]) — checks the ack wait and the re-read since.
  Needs a re-run of the `hz-walk.yaml` plan from #20. (#20)
- A mode walk under HOLD on the UT61B+, which drops Hz/% there ([§6.3][s63]):
  `set hold on`, then `set mode Hz` from AC V, should press HOLD off and land;
  RANGE and AUTO under HOLD are unasked there. Needs a UT61B+. (#20)

## Flag settings

- Peak in AC µA, AC mA, AC A, AC+DC V and LPF V on our UT61E+ — only AC V
  and AC mV have entered it, DC V ignores it ([§6.1][s61], [UT61E+ spec
  §2.7][e27]); decides `AC_PEAK_MODES`. Needs `set peak p-max` in each.
- 0x4D on a UT61B+, which has no Peak: ignored or answered with an error
  ([§6][s6]) — confirms `choices` offering none there. Needs `dmm-cli
  --device ut61b+ command peak` under `RUST_LOG=dmm_lib=debug`. (#7)
- The UT61D+'s Peak modes, offered as the E+'s, and its refusals, [DEDUCED]
  from [§6.2][s62] — decide `AC_PEAK_MODES` and the dead lists for it. Needs
  a UT61D+ `capture --unverified`. (#7)
- A rung other than 1000V in LPF V, where AUTO does nothing ([§3.1][s31]) —
  decides what `set range` offers there. Needs LPF V with a signal applied
  and RANGE pressed.

## Range commands

- RANGE on the 6,000-count mV ladders, where the B+'s other two-rung ladders
  step ([§5.5][s55]) — decides `range_is_fixed`. A plain `capture` on a UT61B+
  or UT61D+ files `dcmv/range:600mV` or "RANGE did nothing". (#7)
- A UT61B+ Ω walk: four presses from 600kΩ auto read 3, 4, 5, 0, then 1
  unpressed, though its rungs hold ([§6.1][s61]) — a press doubled by the
  CH9329 or the meter. Needs RANGE walks on a UT61+ over a CH9329 (a
  reporter's UT61B+). (#19)

## Range tables

- UT61B+ AC V rungs 1 (60V) and 3 (1000V), pinned between rungs 0 and 2
  ([§5.1][s51]) — only `range_label` rides on them, the unit being V. Needs
  `capture --steps acv`, leads open. (#7)
- UT61B+ capacitance rungs 1-4 and 6, pinned between rungs 0 and 5
  ([§5.4][s54]) — the unit changes along the ladder, so a wrong rung is a 1000x
  error. Needs a capacitor per decade: RANGE does nothing there. (#7)
- The Hz rungs above 0 ([§5.9][s59]), the deck's labels, none seen on a
  meter — the MHz ones most (E+ bytes 5-7, 6,000-count byte 5). Needs a
  signal generator reaching each. (#6, #7)
- Frequency from the UT61B+'s V~ position, `0.0` at index 0 ([§5.9][s59]) —
  whether index 0 holds above 99.99 Hz, labelling a kHz reading Hz. Needs
  Hz/% on V~ with a ~1 kHz signal. (#7)
- UT61D+ A rungs, 6A at byte 0 and 20A at byte 1 where the deck gives 10A
  ([§5.5][s55]) — decides `ut61d_plus.rs`'s current tables. Needs a D+ frame
  in each A range. (#7)
- UT61E+ A range byte 0, never sent ([§5.5][s55]) — decides the placeholder
  `20A` at index 0 of `ut61e_plus.rs`'s current tables. Needs any A frame
  with byte 0. (#6)
- The UT61D+ and UT161 tables, [DEDUCED] from the shapes in [§9][s9] — a first
  `capture --unverified` confirms them. Needs a UT61D+ or a UT161. (#7)

## Readings with a signal

- DC mV, AC µA, AC mA and AC A on our UT61E+ — their mode bytes are seen
  with open leads only ([UT61E+ spec §2.5][e25]); a value confirms each
  decode. Needs a small DC source and AC current up to the A range. (#6)
- Duty % — only the mode byte seen. Needs a PWM signal. (#6)
- The UT61B+ bar graph against a moving input ([§4][s4]) — confirms the
  0-30 count between the fixed readings seen. Needs a slowly swept DC V. (#7)
- The UT61B+ NCV display with no field near ([§3][s3]) — the E+ idles at
  `EF`; decides `ncv_level` there. Needs step `ncv` away from any cable. (#7)

## The UT61D+

- LoZ V, 0x15 by the deck ([§3][s3]) — decides `Mode::LozV`'s byte. Needs step
  `loz`. (#7)
- Temperature: how °C and °F fill the display field, and the deck's range
  bytes 0 and 1 ([§5.6][s56]) — decides `temp_c`/`temp_f`. Needs a K-type
  thermocouple, steps `temp` and `tempf`. (#7)

## UT60BT and UT202BT

- A first reading from each, named and under `auto` — the open sends 0x5F,
  waits, then 0x5D ([§6.4][s64]); decides that handshake. Needs the meter and
  Bluetooth. (#26, #27)
- The range tables, the app asset's rungs under the manuals' labels, and
  where the two disagree ([§9][s9] notes) — decides each label. Needs
  `capture --unverified`. (#26, #27)
- UT202BT AC A as 0x11 or 0x16 ([§9][s9]) — decides which code the table
  keeps; the `aca` step asserts neither. (#27)
- UT202BT LPF V and LPF A off range byte 2, °C off byte 1 ([§9][s9]) — a frame
  there reports an unrecognised range. Needs steps `lpfv`, `lpfa`, `temp`. (#27)
- The UT202BT secondary display ([§2.3][s23]): which modes send it, before or
  after its main frame, how often, under HOLD, bytes 12-16, "CUT" — decides
  how `take_reading` attaches it. Needs a first UT202BT capture. (#27)
- Peak on the UT202BT: which of 0x37 (the app's long press) and 0x4D (the
  family's) enters it, and its flag bits ([§6.5][s65]) — decides offering
  Peak. Needs steps `peak` and `peak_off`, then each byte sent. (#27)
- 0x41 MIN/MAX on the UT60BT, which the app disables and a community client
  sends ([§10][s10]) — offered once it sets the MIN or MAX flag. (#26)
- 0x47 AUTO on the UT202BT, which the app never sends — decides whether it
  gets a range ladder. Needs `command auto` after a RANGE press. (#27)
- The UT60BT's SELECT ring and the UT202BT's 0x31/0x33/0x35 buttons
  ([§6.5][s65]) — decide a dial table, so remote mode selection. Needs each
  pressed through its functions. (#26, #27)

## Cables

- A reading a CH9329 holds across opens: `read` asks no name, so each polled
  reading may come one behind. Kill a `read` mid-poll, start it again and
  compare with the LCD. Needs a UT61+ on a CH9329. (#19)

## Spec data

Notes in `ut61eplus/specs/` kept close to unclear manual wording; each needs
a later manual or UNI-T's word.

- LPF V shares AC V's notes, "(1kHz–10kHz: 10%–100%)" included, which
  cannot apply with the filter on — decides notes of its own.
- UT61E+ crest factor "≤2.0 at 10000 counts, ≤1 at 22000 counts", while the
  add-ons that follow run to crest factor 3.
- "Add 4% / 5% / 7%" for a non-sine wave, which does not say of what.
- UT61E+ AC current "minimum 30µA at µA ranges", inside the 1kHz~10kHz
  clause, so it may bound that band only.
- UT61E+ AC+DC "For AC voltage, … ≤200 digits" against the AC V table's ≤10
  — which reading it covers.
- UT61E+ capacitance "add 10 digits when the accuracy is ≤3%".
- Duty "Frequency ≤10kHz, duty ratio 10.0%~90.0%" — the accuracy or the
  span; the row gives 0.1%~99.9%.
- UT61D+ temperature "should be less than 230°C/446°F" against a table to
  1000°C/1832°F; the manual does not tie it to the probe.
- The UT60BT and UT202BT have none (`SpecModel::Untranscribed`) until a meter
  confirms their range tables; the manuals' spec pages are the source then.
  (#26, #27)

## Vendor sources

- The UT61B+ and UT61D+ PC software UNI-T uploaded 2023-02-03 — hash-compare
  it with Software V2.02, as the UT61E+ one was ([approach
  doc](reverse-engineering-approach.md)); decides whether V2.02 covers them.
- iDMM2.0's UT202BT table against the deck's UT202S one — only the Ω ladder
  is compared, and differs ([§9 UT202BT](reverse-engineered-protocol.md#ut202bt-9999-counts));
  decides how far the deck backs `ut202bt.rs`.

## Detection

- The UT61D+ and UT161 reported names and the UT60BT and UT202BT rows:
  [Device auto-detection](../../verification-backlog.md#device-auto-detection).

[e25]: ../ut61eplus/reverse-engineered-protocol.md#25-mode-byte-values--vendor
[e27]: ../ut61eplus/reverse-engineered-protocol.md#27-flag-bytes--vendor
[s10]: reverse-engineered-protocol.md#10-cross-reference-with-community-sources-community
[s23]: reverse-engineered-protocol.md#23-secondary-display-frame--vendor--vendor-doc--manual
[s3]: reverse-engineered-protocol.md#3-available-modes-per-model--manual--vendor--vendor-doc
[s31]: reverse-engineered-protocol.md#31-function-dial-positions-and-cycle-rings--manual
[s4]: reverse-engineered-protocol.md#4-flag-byte-differences--manual--vendor
[s51]: reverse-engineered-protocol.md#51-dc-voltage
[s54]: reverse-engineered-protocol.md#54-capacitance
[s55]: reverse-engineered-protocol.md#55-current
[s56]: reverse-engineered-protocol.md#56-temperature-ut61d--ut161d-only--manual
[s59]: reverse-engineered-protocol.md#59-frequency--vendor-doc
[s6]: reverse-engineered-protocol.md#6-commands--vendor
[s61]: reverse-engineered-protocol.md#61-range-and-auto-semantics--verified-ut61e-2026-09-07
[s62]: reverse-engineered-protocol.md#62-which-modes-take-which-command--verified-ut61e-and-ut61b
[s63]: reverse-engineered-protocol.md#63-commands-under-hold--verified-ut61e-2026-09-14
[s64]: reverse-engineered-protocol.md#64-start-order-over-bluetooth--vendor
[s65]: reverse-engineered-protocol.md#65-ut60bt-and-ut202bt-buttons--vendor
[s9]: reverse-engineered-protocol.md#9-per-range-full-scale-limits
