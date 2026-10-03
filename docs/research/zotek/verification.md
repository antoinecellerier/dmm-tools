# ZOTEK verification

Open checks for the ZT-300AB / AN9002, ZT-5566SE / AN999S, ZT-5BQ / ST207
and ZT-5B / V05B, issues
[#28](https://github.com/antoinecellerier/dmm-tools/issues/28),
[#29](https://github.com/antoinecellerier/dmm-tools/issues/29),
[#30](https://github.com/antoinecellerier/dmm-tools/issues/30) and
[#31](https://github.com/antoinecellerier/dmm-tools/issues/31).
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). Community captures
(spec §11) answer many of these; each still wants a reporter's capture.

## First report

- The advertised name and a few seconds of raw FFF4 notifications, with the
  model, function and LCD reading noted — settles most items below at once.
  Needs any model and `dmm-cli capture`. (#28–#31)
- A first confirmed report per model — gates that model's spec tables, which
  its ZOTEK manual carries. (#28–#31)

## Models and layouts

- Each model's type byte, [INFERRED] in §1, four seen in §11.4 — decides the
  entry each alias sits on. Unseen: ZT-5BQ, AN999S, ZT-5566, ZT-5566S, BSIDE's
  ZT-5B, ZT-5BQ and ZT5566; ZT-6S: no evidence. Needs any capture. (#28–#31)
- Whether the ZT-5566 and ZT-5566S stream readings, their manuals giving
  Bluetooth to a speaker (§1; a ZT-5566SE streams, §11.4) — decides a ZT-5566
  alias and keeps the ZT-5566S's on `zt5566se`. Needs one of them. (#29)

## Advertising and GATT

- Which models, if any, advertise "ZY", and whether the name sits in the
  advertisement or the scan response (§2, §11.3 D9) — decides the entries'
  Bluetooth names. Needs a passive scan per model. (#28–#31)
- FFF4 writable and FFF3 absent on a type-1 and a type-4 meter (§2, §11.3 D8)
  — decides the FFF0 profile's write characteristic. Needs GATT discovery on
  a ZT-5BQ / ST207 and a ZT-5566SE. (#30, #29)
- Whether FFF4 also takes a write with response (§2) — settles §2's
  write-type row. Needs any meter. (#28–#31)

## Packets

- Exactly 10, 10, 11 and 19 bytes per type, nothing after (§5, §11.4) —
  decides where the extractor cuts (`frame::packet_len`). Needs any capture.
  (#28–#31)
- Notifications per second against the LCD's 3 updates (§3; about 2.6 on an
  AN9002, §11.4) — decides whether one update is one packet. Needs any
  capture. (#28–#31)

## Display

- Which digit positions each word uses, and whether type 4 shows words (§6.4;
  §11.3 D2, §11.4) — decides `glyph::read`'s patterns. Needs `ohm_ol` (types
  1-2: `diode_ol`), `auto_idle`, `ncv`; type 4: `ohm_ol`, `key_ncv`. (#28–#31)
- What the number of NCV dashes means (§6.4; they fill from the left, §11.4)
  — decides the NCV level reported. Needs `ncv` at several distances from the
  wire. (#28, #30, #31)
- Dashes with INRUSH lit (§11.3 D3) — decides the inrush-wait word. Needs
  `inrush` on a ZT-5BQ / ST207. (#30)
- What two DP bits in one packet mean, never seen (§6.2) — decides which point
  `glyph::read` keeps. Needs a capture that shows one. (#28–#31)
- Whether a blank digit ever carries a sign or DP (`10`, Implementation
  Notes) — decides how such a blank reads. Needs a negative reading with a
  blank leading digit. (#28–#31)
- Which prefixes a meter sets per unit (§7.5), and type 3's m and µ
  capacitance bits (§11.3 D4) — decides the layouts' prefix bits. Needs
  10-100 µF on a ZT-300AB / AN9002: byte 8 `A0`. (#28)

## Flags

- Type-4 bar graph: bar segments counted against byte 13 bit 4 and bytes
  14-18's unread bits near 4 % of range, and the colon in clock mode (§6.3,
  §11.3 D1) — decides a bar graph and the colon. Needs a ZT-5566/SE. (#29)
- Whether HOLD freezes a type-4 stream (§11.4 reports it does not) — decides
  whether a reading with HOLD lit is the held one. Needs `hold` on a
  ZT-5566SE. (#29)
- Type-4 AC and the secondary display, never in the community log (§11.4) —
  decides the AC bit and the secondary sub-value. Needs `acv` and `hz` on a
  ZT-5566SE. (#29)
- Type-4 byte 3 bit 0, byte 13 bits 5 and 0 and byte 18 bit 7 (§7.4, §11.4)
  — decides which bits stay silent. Needs ZT-5566SE captures across its
  modes. (#29)
- Whether type-4 PEAK, byte 4 bit 2, is ever set: never set in community
  captures, and the LCD has no PEAK icon (§7.4) — decides the "peak" mode.
  Needs ZT-5566SE captures across its modes. (#29)
- Type-3 byte 10 bits 7-4 (TRUE RMS?) and MANUAL, byte 10 bit 1, never set in
  community captures (§7.1, §11.4) — decides whether they stay silent. Needs
  `acv` and `manual_range` on a ZT-300AB / AN9002. (#28)
- What `power` (types 1, 2), `vfc` and `l1_power` (type 4) mean (§7.2-7.4,
  §11.4) — decides whether any becomes a flag. Needs captures across modes.
  (#29–#31)
- Whether type-1 byte 3 bit 2 stays set at high and at zero voltage (§11.3
  D6) — decides Bluetooth icon or HV mark. Needs `dcv`, `acv` and `auto_idle`
  on a ZT-5BQ / ST207. (#30)
- Whether type-2 byte 3 bit 2 (over-voltage) sets at high AC V and clears low
  (§7.3, §11.3 D5) — decides the HV warning flag. Needs a ZT-5B / V05B above
  180 V AC; no capture step: that would put the leads on mains. (#31)

## Keys and commands

- Which key codes each type honours (§8.2): a V05B's subset (§11.4), what
  a ZT-5B's `B0`, `B1`, `B3`, `B5` and `B8` do from Auto, and whether the
  ZT-300AB's dial acts on any — decides each entry's keys (`keys.rs`).
  Needs the `key_*` steps. (#28–#31)
- What each of `C8`-`CB` selects, the apps swapping the AC codes (§8.2) —
  decides the current key's code. Needs `key_current` on a ZT-5566SE in each
  current mode. (#29)
- Whether types 1, 3 and 4 answer a key press with cmd `FD`, as a ZT-5B
  does (§8.2) — decides whether the stream could confirm a key. Needs any
  `key_*` step. (#28–#30)
- Whether a type-4 meter needs or acts on the clock set, cmd `04` (§8.3; the
  app side only, §11.2) — decides whether the driver sends it; it never does.
  Needs a ZT-5566/SE; no capture step sends it. (#29)

## Detection

- What the UT61+, UT181A and UT171 probes do to a ZOTEK meter, and the ZOTEK
  detection row: in the [verification backlog](../../verification-backlog.md).
