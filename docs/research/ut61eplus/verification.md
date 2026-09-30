# UT61E+ verification

Open checks for the UT61E+'s frames and flags and its CP2110 cable, run on
our UT61E+; no issue tracks them. What real meters have confirmed is tagged
`[VERIFIED]` in the [spec](reverse-engineered-protocol.md). The family's
commands, dials, range tables and other models are in
[ut61-family](../ut61-family/verification.md); checks that span families are
in the [verification backlog](../../verification-backlog.md).

## Auto power-off

- Why the meter stays on when polled ([§2.7][s27]): USB turns APO off, each
  request restarts its timer, or APO was off — settles §2.7's [UNVERIFIED].
  Needs the APO symbol at power-on, in `debug`, 15 min after it stops, cable in.
- Flag byte 15 bit 3 in a frame taken with the APO symbol lit — confirms the
  deck's `APO_flag` and decides whether `parse_flags` carries it. Needs the
  first `dmm-cli debug` frame from a meter showing the symbol.

## AC+DC V

- DC and AC components on different rungs ([§2.7][s27]) — decides whether
  the AC sub-value needs its own range; the display keeps the DC frame's
  (`held_reading.rs`). Needs AC+DC V on an AC signal riding a DC offset.

## Display field

- Whether the E+ draws NCV levels of two or more as the B+ does, one `-` per
  level ([§2.4][s24]) — confirms `ncv_level` on the E+. Needs step `ncv` held
  nearer a live cable than level 1 takes.
- hFE with a transistor fitted ([§2.5][s25]) — only the mode byte has come
  from the meter; decides whether `parse_measurement` reads the display as a
  plain number. Needs step `hfe`, a transistor in the socket.

## Power cycle

- The meter switched off and on at another dial position mid-session — the
  checksum error the [backlog's defect](../../verification-backlog.md#connection)
  records should be gone since `Dmm` drops input after a timeout; a rerun under
  `dmm-gui` on the CP2110 cable decides whether it closes.

[s24]: reverse-engineered-protocol.md#24-measurement-response-format--vendor
[s25]: reverse-engineered-protocol.md#25-mode-byte-values--vendor
[s27]: reverse-engineered-protocol.md#27-flag-bytes--vendor
