# Brymen BM78xBT verification

Open checks for the BM788BT and BM787BT, issue
[#33](https://github.com/antoinecellerier/dmm-tools/issues/33).
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). The community SDK
(spec §12) narrows several of these; each still wants a reporter's trace.

## First report

- `RUST_LOG=dmm_lib=trace dmm-cli --device bm78xbt debug` for a few seconds
  in DC V, leads reversed on a battery, then in Ω, leads open — settles most
  items below at once. Needs either model. (#33)
- A first confirmed report per model — gates the spec tables both manuals
  carry. The BM787BT's AC V and AC+DC V accuracy is lower outside 50-60 Hz
  (BM787BT manual p.23-24), and nothing on the wire picks a table (§1). (#33)

## Login

- An accepted login with 0000 as binary digits, [13] = `01` and the address
  low byte first (§5, §7.2; the SDK agrees, §12.2) — lets those rows take
  [HARDWARE]. Needs a trace. (#33)
- Whether the meter streams with notifications on before 0x0151, or with no
  login (§3; r4's order works, §12.2) — decides the bring-up order in
  `transport/ble/`. Needs a dev build that subscribes first. (#33)
- What CDD4 holds when read at once after 0x0151 — the reply, or the command
  still in place (reported) — and whether replies also come on CDD5 (§3.2):
  decides a pause before `brymen::log_in`'s read. Needs a trace. (#33)
- How long the login takes, write to reply — decides `LOGIN_TIMEOUT`, 8 s
  against the app's 3 s response timeout (§3.2). Needs a trace. (#33)
- Whether the meter answers 0x0101, GetBLEAddress, outside r4 (§3.2, §7.3) —
  decides the address on macOS, where the platform gives none. Needs a trace
  on macOS. (#33)
- Whether the meter checks the address in [5..10] (§5; low byte first works,
  §12.2) — decides whether the login with zeros, sent when no address is
  known, works. Needs a dev build that sends zeros. (#33)
- The digit order of a non-zero password in 0x0151 (§7.2, §12.3 D3) —
  decides whether a password other than 0000 can be offered
  (`login::PASSWORD`). Needs a meter with a changed password. (#33)
- Error codes 3 and 4, both "invalid password" in r4, and any above 6 (§8)
  — decides the refused-login messages (`login::refused`). Needs a login
  with a wrong password. (#33)
- The reset gesture and its "Org" (§9.2) — decides the refused-login error
  and the activation text. Needs either model. (#33)
- The BM787BT's defaults 0000 and "BM78xBT", not printed in its manual
  (§9.2) — decides the same texts for it. Needs a BM787BT. (#33)
- Password Identification [13] = `00`, sent by four of the app's commands
  (§5; `01` works, §12.2) — settles §5's row. Needs a dev build. (#33)
- How the meter answers 0x0106, 0x0021 and 0x0005, and whether it sends
  0x8000 (§7.3, §8; none needed to stream, §12.2) — settles those rows; the
  driver sends none. Needs a dev build. (#33)

## Advertising and GATT

- The Status byte, the scan response, the OTA-mode advertisement, a service
  UUID, and advertising while connected (§2; §12.3 D4, §12.4) — decides
  whether the search can match more than the name. Needs a passive scan. (#33)
- The device name: 12 characters (r4) or 11 (the app), and a short name's
  padding (§2, §7.1) — settles those rows. Needs a meter renamed by
  Brymen's app, and a scan. (#33)
- Write without response on CDD4 (§2) — settles §2's write-type row; every
  write the driver sends is acknowledged. Needs GATT discovery. (#33)

## Framing

- The CRC low byte first, and the reading packet's `FF 02` head (§4; the
  SDK's commands are accepted so, §12.2) — either wrong loses every reading
  to a timeout, a CRC mismatch showing at DEBUG. Needs a trace. (#33)
- What the meter sends at an MTU under 155, where 152 bytes do not fit one
  notification (§4, §12.3 D1) — decides whether the open, which requests
  none and warns, must request 185. Needs a host with a small MTU. (#33)

## Readings

- The sign: a negative count, a magnitude with Flag1 bit 6, or both (§6.3,
  §12.3 D2) — decides whether `decode::number` keeps reporting a negative
  count without the flag. Needs `dcv_negative`. (#33)
- [27], [24] and [25] per range, the point with 3 or 6 digits, and the
  prefix byte (§6.3) — decides `decode::scaling`'s reports, and a range
  label, empty today. Needs `manual_range` and each function's steps. (#33)
- `4F` for %4~20mA, which the app lacks (§6.4) — decides the "%" unit in
  `tables::UNITS`. Needs `ma420` on a BM788BT. (#33)
- AutoCheck sub `03` AUTO (r4) or `02` OHM (the app) (§6.5) — decides which
  `tables::MAINS` keeps. Needs the `autov` steps. (#33)
- Line frequency as main `23` or the struck `03` subs (§6.5) — decides which
  `tables::MAINS` keeps. Needs `line_hz` and `line_hz_a`. (#33)
- ASCII codes outside r4: 0, 8, 9, `0C`-`0F` (§6.7) — decides whether
  `tables::shown` keeps reporting the app's words. Needs any capture. (#33)
- r4's "x" bits: Flag0 bits 0-1, Flag1 bits 0 and 7 (the app's TestLead),
  Flag2 (§6.6) — decides whether they stay silent. Needs captures across
  modes and `iner`. (#33)
- The information packet: battery values other than `02`, the Power Source
  Flag, [16..18] as one count, [19] (§6.1, §6.9) — decides which fields are
  reported. Needs captures, one at low battery. (#33)
- The RTC: hour 1-23 or 0-23, what it holds before the first 0x0010 and
  across power-off, binary or BCD in 0x0010 (§6.8, §7.1; §12.2, §12.4) —
  settles them; the driver ignores it and never sets it. Needs a trace. (#33)

## Rate and gaps

- Notifications per second against the display's 5 and REC's rates (§3.2,
  §9.4; ~5, §12.4) — a read waits 2 s, which DC+AC in REC at 1/s fits;
  decides whether capacitance does. Needs `cap` with a large capacitor. (#33)
- How long notifications stop on a function or range change (§12.4) —
  decides whether the pause ends a read in a timeout. Needs a trace across a
  SELECT press and `manual_range`. (#33)

## Meter behaviour

- Auto power-off after 30 or 15 minutes, what a connection does to it, and
  whether Bluetooth stays on across power-off (§9.1, §9.3) — decides the
  activation text. Needs a connected meter left idle. (#33)
- Whether the Δ long press works in AutoV (§9.1) — decides the activation
  text's "any function but Auto V/LoZ". Needs AutoV. (#33)
- Any field that differs between a BM788BT and a BM787BT (§1) — decides
  whether both keep one entry, `bm78xbt`. Needs a trace from each. (#33)

## Detection

- What the UT61+, UT181A and UT171 probes do to a BM78xBT, a cached BM78xBT
  against a known UT-D07B, and the BM78xBT detection row: in the
  [verification backlog](../../verification-backlog.md#device-auto-detection).
