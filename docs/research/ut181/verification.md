# UT181A verification

Open checks for the UT181A, issue [#5](https://github.com/antoinecellerier/dmm-tools/issues/5).
What real meters have confirmed is tagged `[HARDWARE]` in the [spec](reverse-engineered-protocol.md);
checks that span families are in the [verification backlog](../../verification-backlog.md).

## Mode words

- REL on the 21 words [§6](reverse-engineered-protocol.md#6-mode-word-table----known)
  lists as never sent — confirms `mode::rel_word` and §6.1's REL column there.
  Needs REL swept on V AC, mV, Ω, Cap, Hz, current and temperature. (#5)
- `0x2141` (mV AC+DC) sent from the host — only the meter's own keys have
  entered it; decides whether `mode_choices` keeps offering it. Needs `set mode`
  on the mV AC dial. (#5)
- A SET_MODE word the vendor app never sends — another dial family's, or
  n0 = 3 ([§6.1](reverse-engineered-protocol.md#secondary-nibble)) — ER or
  taken; decides whether `select`'s family guard is needed. Needs a hidapi
  script or dev build. (#5)

## Ranges

- A manual rung in a Peak variant ([§7](reverse-engineered-protocol.md#7-range-byte----known))
  — decides whether `range_choices` keeps rungs in Peak. Needs `set range`
  re-run in µA DC Peak, reading the LCD. (#5)
- The V AC, mV AC, mV DC, Ω, mA AC and plain V DC ladders — never driven;
  decides §7.1's ladders and `range_choices` there. Needs `capture --steps
  vac,mvac,mvdc,maac,ohm_ranges,vdc_ranges,manual_range,auto`. (#5)
- Duty and Pulse width rung names ([§7.1](reverse-engineered-protocol.md#71-vendor-range-ladders----vendor))
  — decides `lookup_range_label` and `range_choices`. On the Hz dial switched
  to Duty, then Pulse: `command range` ×4, `read --count 1` after each, noting
  the LCD's range; then `command auto`. (#5)

## Display and flags

- What the LCD shows during a [blank value](reverse-engineered-protocol.md#52-value-encoding)
  — dashes, nothing, the last reading; decides parse.rs `BLANK`. Needs a look
  at the meter right after a range change. (#5)
- misc2 bit 3 ([§5.1](reverse-engineered-protocol.md#51-common-header-5-bytes))
  — which LCD warning, if any, goes with it; decides its `LEAD ERR` label.
  Needs the A DC dial and the LCD. (#5)
- misc2 bit 5 (record) — never seen set; decides the `record` flag. Needs a
  recording started on the meter while it streams. (#5)
- COMP OUTER, BELOW and ABOVE, and a PASS result ([§5.4](reverse-engineered-protocol.md#54-comp-mode-extension))
  — only INNER/FAIL has come from a meter; decides parse.rs's COMP decoding.
  Needs COMP set on the meter in each. (#5)

## Commands

- HOLD as bare `[0x12]` ([§4.2](reverse-engineered-protocol.md#42-commands-host---device))
  — only `12 5A` has been sent; settles the HOLD row. Needs a hidapi script or
  dev build. (#5)

## Cables

- The CP2110 cable on a UT181A — both reports used the CH9329 (UT-D09); one
  older CP2110 unit was never detected on macOS by other software. Decides the
  PartlyVerified label. Needs a CP2110 UT181A and `dmm-cli read`. (#5)
- The UT-D09's CH9329 mode, 0 or 3 — decides whether the transport must pick
  the custom HID interface. Needs `lsusb -v -d 1a86:e429` from an owner
  ([test plan](ch9329-test-plan.md) step 1). (#5)

## Spec data

- Continuity remarks — read as covering both the short (`0x5211`) and the open
  alarm (`0x5212`); the manual does not say so. Decides `specs.rs` sharing one
  table. Needs a later manual or UNI-T's word.
- Current: "20A: 30s on, then 10min off; not specified above 10A" leaves open
  what 10–20 A readings are worth. Decides the current tables' notes. Needs a
  later manual or UNI-T's word.

## Vendor sources

- The UT181A PC software UNI-T uploaded 2023-02-03 (content/1261,
  [candidates](../new-device-candidates.md#uni-t-chinese-sites)) — hash-compare it with the V1.05
  the approach doc traced; decides whether Phase 3 covers the current app.
