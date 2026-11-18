# UT71 / Voltcraft VC9x0 verification

Open checks for the UT71A–E and the Voltcraft VC920, VC940 and VC960,
issues [#22](https://github.com/antoinecellerier/dmm-tools/issues/22) and
[#23](https://github.com/antoinecellerier/dmm-tools/issues/23).
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). A UT71D has run
every function it has (#22); no UT71A/B/C/E or VC9x0 has been read.

## Cables

- The bridge in Voltcraft's USB adapter 120317, CH9325 (`1A86:E008`) as in
  a UT71D's cable, or HE2325U (`04FA:2490`) (§1.1) — the HE2325U has no
  transport match and no udev rule. Needs `lsusb` with the adapter. (#23)
- A UT71 over the UT-D07A Bluetooth adapter: in the [UT-D07B verification
  list](../ut-d07b/verification.md#meters-behind-the-adapter).

## Functions

- Code E packets from a UT71E or VC940: their point, after digit 4 in the app
  (§3.2; community sources differ, [§7][xref]), VA and cos φ (§3.1), the blue
  key on W (§3.2) — decides the Power mode. Needs `power`. (#22, #23)
- That a VC9x0 never sends code 0 (AC mV), as a UT71D does not (§3.2) —
  decides whether it stays an unknown mode in `ut804_mode_info`. Needs every
  function swept on a VC9x0. (#23)
- How a VC920/960 sends duty cycle: the 2005 VC app has no duty case (§4.3),
  a UT71D sends Hz with the sign bit (§3.4) — decides the override per
  model. Needs `duty` on a VC920/960. (#23)
- Continuity's point, after digit 3 in the app (§3.5; community sources
  differ, [§7][xref]) — decides the continuity row. Needs `cont` with the
  probe tips touching; open leads only show the overload. (#22, #23)

## Ranges

- AC V range 4, 1000 V on a UT71 and 750 V on a VC9x0 (§3.5) — decides
  `range_label`'s VC920 case. Needs `acv` with RANGE stepped to the top range,
  leads open, on each brand. (#22, #23)
- The UT71A/B's range codes, at half the C/D/E's full scales (§3.5) — decides
  `ut71ab_range_label`. Needs the function steps on a UT71A/B. (#22)
- The codes sent at 4000 counts with RANGE held at power-on, and on a VC9x0's
  Ω (§3.1, §3.5; the blue key's are seen) — decides their labels. Needs `ohm`
  after RANGE at power-on, and `ohm` on a VC9x0. (#22, #23)

## Digits and overload

- Digits `B`, `D`-`F`, overload patterns other than `A A 0 C A`, and HI,
  against the LCD (§3.1; the 2005 VC app has no LO, §4.3) — decides the
  overload read. Needs `ma_percent` above 20 mA. (#22, #23)

## Buttons

- That RANGE on a ranged function sends Manual, `10` in bits 1:0, as MAX MIN
  and REL do (§3.4) — settles §3.4 for RANGE. Needs `manual_range` and
  `max_min` on DC V; #22's ran on A, which has one range. (#22, #23)
- HOLD and PEAK HOLD on the wire (§3.4) — decides the flags and whether HOLD
  silences the meter, as on a UT804. Needs `hold` with REL off; no step asks
  for PEAK HOLD. (#22, #23)
- What RECALL + ▶ sends, and how each reading's store time reaches the
  software (§4.1) — decides whether a recall needs a decoder. Needs stored
  readings on a UT71B-E or VC9x0 and a trace; no step asks for it. (#22, #23)

## Capture steps

- Whether a VC9x0 of the 2009 manual ranges manually on RANGE/SETUP, as the
  2005-2006 manuals say; the 2007 calibration calls it RANGE (§4.2) — decides
  the VC920's `manual_range`, `max_min`, `auto_range` steps. Needs a VC9x0. (#23)

## Detection

- The CH9325 detection row, which claims a UT71 or VC9x0 as a UT804: in the
  [verification backlog](../../verification-backlog.md).

[xref]: reverse-engineered-protocol.md#7-cross-reference-with-community-sources
