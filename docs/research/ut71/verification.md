# UT71 / Voltcraft VC9x0 verification

Open checks for the UT71A–E and the Voltcraft VC920, VC940 and VC960,
issues [#22](https://github.com/antoinecellerier/dmm-tools/issues/22) and
[#23](https://github.com/antoinecellerier/dmm-tools/issues/23).
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). No UT71 or VC9x0
packet has been seen, so every item waits on a first report.

## Line and packet

- The data bytes' high nibble and bit-7 parity (§1.2, §2): LF as `0A` would
  tell a UT71 from a UT804, which sends `8A`, only if the UT71 is 8N1. Needs
  any packet in a `RUST_LOG=dmm_lib=trace dmm-cli debug` trace. (#22, #23)
- Whether packets follow the display's 2-3 updates a second (§4.1, §5) —
  settles §5. Needs `dmm-cli read --count 60 --interval-ms 0`. (#22, #23)

## Cables

- The bridge in the UT71's cable and in Voltcraft's USB adapter 120317, CH9325
  (`1A86:E008`) or HE2325U (`04FA:2490`) (§1.1) — the HE2325U has no transport
  match and no udev rule. Needs `lsusb` with the cable. (#22, #23)
- A UT71 over the UT-D07A Bluetooth adapter: in the [UT-D07B verification
  list](../ut-d07b/verification.md#meters-behind-the-adapter).

## Functions

- Code E packets from a UT71E or VC940: their point, after digit 4 in the app
  (§3.2; community sources differ, [§7][xref]), VA and cos φ (§3.1), the blue
  key on W (§3.2) — decides the Power mode. Needs `power`. (#22, #23)
- That code 0 (AC mV) is never sent (§3.2) — decides whether it stays an
  unknown mode in `ut804_mode_info`. Needs every function swept. (#22, #23)
- The duty-cycle sign bit, and duty's point, after digit 3 in the app (§3.4;
  community sources differ, [§7][xref]) — decides the duty override. Needs
  `duty`. (#22, #23)
- Continuity's point, after digit 3 in the app (§3.5; community sources
  differ, [§7][xref]) — decides the continuity row. Needs `cont`. (#22, #23)

## Ranges

- AC V range 4, 1000 V on a UT71 and 750 V on a VC9x0 (§3.5) — decides
  `range_label`'s VC920 case. Needs `acv` with RANGE stepped to the top range,
  leads open, on each brand. (#22, #23)
- The UT71A/B's range codes, at half the C/D/E's full scales (§3.5) — decides
  `ut71ab_range_label`. Needs the function steps on a UT71A/B. (#22)
- The codes sent at 4000 counts, blue key or RANGE at power-on, and on a
  VC9x0's Ω (§3.1, §3.5) — decides their labels. Needs `fast_mode`, and `ohm`
  on a VC9x0. (#22, #23)
- The 10 A range code, 0 on the sheet, 1 from a UT804 (§3.5; community sources
  differ, [§7][xref]) — settles §3.5; the parser takes any code. Needs `dca`.
  (#22, #23)

## Status and coupling

- Coupling on DC readings, 0 or 2 (§3.3) — settles §3.3; both read as DC.
  Needs `dcv`, `dcmv`, `dcua`, `dcma`, `dca`. (#22, #23)
- Status bit 3, and the AUTO/Manual field's values (§3.4) — decides the
  status-bits report and `auto_range`. Needs `dcv_negative`, `manual_range`,
  `auto_range`, and a single-range mode such as `diode`. (#22, #23)

## Digits and overload

- Digit values `B` and `D`-`F`, and overload and LO packets beyond the two
  patterns in [§7][xref], against the LCD (§3.1) — decides the overload read.
  Needs `ohm_ol`, and `ma_percent` with no loop current (LO). (#22, #23)

## Buttons

- HOLD, REL, MAX MIN and PEAK HOLD on the wire (§3.4) — decides the flags and
  whether HOLD silences the meter, as on a UT804. Needs `hold`, `rel`,
  `max_min`; no step asks for PEAK HOLD. (#22, #23)
- What RECALL + ▶ sends, and how each reading's store time reaches the
  software (§4.1) — decides whether a recall needs a decoder. Needs stored
  readings on a UT71B-E or VC9x0 and a trace; no step asks for it. (#22, #23)

## Capture steps

- Whether a VC9x0 of the 2009 manual ranges manually on its RANGE/SETUP key,
  as the 2005-2006 manuals say (§4.2) — decides the VC920's `manual_range`,
  `max_min` and `auto_range` steps. Needs a VC9x0. (#23)

## Spec data

- Spec tables for `ut71ab`, `ut71cde` and `vc920`, from the UT71 and Voltcraft
  manuals — wait for a first hardware confirmation. (#22, #23)

## Detection

- The CH9325 detection row, which claims a UT71 or VC9x0 as a UT804: in the
  [verification backlog](../../verification-backlog.md).

[xref]: reverse-engineered-protocol.md#7-cross-reference-with-community-sources
