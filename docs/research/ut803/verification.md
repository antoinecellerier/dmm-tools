# UT803 / UT804 verification

Open checks for the UT803 and UT804, issues
[#15](https://github.com/antoinecellerier/dmm-tools/issues/15) and
[#16](https://github.com/antoinecellerier/dmm-tools/issues/16).
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). The UT804 has run
every capture step; no UT803 has been read.

## First UT803 report

- `dmm-cli --device ut803 capture --unverified` — one packet confirms the
  UT803 layout ([§7.4][s74] item 4) and settles most items below. Needs a
  UT803 on the UT-D04 cable, RS232 on. (#15)
- The line, 19200 7O1 by the manual ([§1.2][s12]) — decides the UT803
  `init`'s 19200 report. Needs `RUST_LOG=dmm_lib=trace dmm-cli --device ut803
  debug`: its CH9325 line's baud and the bytes' bit 7. (#15)
- Its packet rate, where the UT804 sends one about every 656 ms
  ([§4.2][s42]). Needs `dmm-cli --device ut803 read --count 60
  --interval-ms 0`. (#15)

## UT803 payload

- The sign (position 8 bit 2), overload (8 bit 0), HOLD (9 bit 3) and
  AUTO, AC and DC (10) ([§7.4][s74] items 2 and 4) — decides `Ut803Fields`.
  Needs `dcv_negative`, `ohm_ol`, `hold`, `acv`. (#15)
- The mode codes and range tables ([§3.4][s34], [§7.4][s74] item 4) —
  decides `ut803_mode_info`, and whether it gets range labels as the
  UT804's do. Needs every mode step. (#15)
- Frequency range 0's decimal point, a guess in `ut803_mode_info`, and
  tachometer (RPM) packets — decide those rows. Needs `hz` and `rpm`. (#15)
- What nibble 9 bits 2-1 light (MAX MIN, REL?) — the parser leaves them
  silent ([§7.4][s74] item 2). Needs MAX MIN and REL pressed under a trace;
  no step asks for them. (#15)
- Both coupling bits (AC+DC), which the manual keeps off the wire
  ([§5][s5]) — decides the `ac/dc bits` report. Needs `acdcv`. (#15)
- Code E, read as `ADP` with no unit — decides that row. Needs `adp`. (#15)

## UT804 functions

- The °F packets, code D ([§3.4][s34]) — decides `ut804_mode_info`'s 0xD
  row. Needs the °C °F position with SELECT for °F; no step asks. (#16)
- AC mV: coupling 1 or 3 on code 3 ([§3.5][s35]), taken without a report, or
  code 0, the sheet's `AC_mV`, an unknown mode ([§3.4][s34]) — decides both.
  Needs the mV position with the AC/AC+DC button pressed; no step asks. (#16)

## UT804 status and digits

- Where low battery shows, if at all — status bit 3, the sheet's sign, was
  never set ([§3.6][s36]); decides a `low_battery` flag. Needs a UT804 with
  its battery symbol lit and a `debug` trace. The last item on #16. (#16)
- Digit nibbles `B` and `D`-`F` ([§3.2][s32]): the vendor zeroes a `B` in
  nibble 4; `F` is in the 4-20 mA "HI" pattern ([§8][s8]). Decides
  `parse_ut804_layout`. Needs `ma_percent` above 20 mA, the LCD noted. (#16)

## CH9325 bridge

- Which feature-report layout the bridge reads, the apps' or the SDK DLL's
  ([§1.2][s12]) — 2400 may be its default, so only another rate settles it;
  decides `baud_report`. Needs a UT803 streaming at 19200. (#15)
- Whether a meter needs the `0x5A` trigger start-up sends — the apps send
  nothing ([§4.2][s42]); decides whether `Ch9325::start_up` keeps it. Needs
  a dev build without it on a UT804. (#16)
- The first packet after a pause on Linux, stale or spliced — `Dmm` drops
  the queue after 250 ms unread (`Ut80xProtocol::discard_input`); confirms
  that covers it. Needs a UT804 on Linux, `capture` under a trace. (#16)
- A CH9325 on macOS, open for every bridge but the CP2110
  ([#2](https://github.com/antoinecellerier/dmm-tools/issues/2)): in the
  [verification backlog](../../verification-backlog.md#usb-bridges).

## Spec data

- UT803 hFE: "bo ≈10µA" is kept as printed; the manual does not define `bo`.
  Decides the note in `specs_ut803.rs`. Needs a later manual or UNI-T's word.

## Detection

- The UT803/UT804 detection row, and the CH9325's rate during detection (a
  UT803 at 19200 is not found):
  [Device auto-detection](../../verification-backlog.md#device-auto-detection).

[s12]: reverse-engineered-protocol.md#12-uart-parameters--vendor
[s32]: reverse-engineered-protocol.md#32-digit-encoding--vendor
[s34]: reverse-engineered-protocol.md#34-mode-codes-nibble-7--vendor
[s35]: reverse-engineered-protocol.md#35-acdc-indicator-nibble-8--vendor
[s36]: reverse-engineered-protocol.md#36-status-flags-nibble-9--vendor
[s42]: reverse-engineered-protocol.md#42-data-streaming
[s5]: reverse-engineered-protocol.md#5-differences-between-ut803-and-ut804
[s74]: reverse-engineered-protocol.md#74-sign-nibbles-12-14-and-the-two-model-split--vendor
[s8]: reverse-engineered-protocol.md#8-cross-reference-with-community-sources
