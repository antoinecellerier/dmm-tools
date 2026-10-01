# OWON verification

Open checks for OWON's Bluetooth meters — the B33(T)(+), B35(T)(+),
B41T(+), OW16B, OW18B, OW18E and CM2100B; no issue is open yet.
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). No community
cross-reference has been done (spec §14).

## First report

- The advertised name, a GATT discovery, the FFF2 value and a few seconds of
  raw FFF4 notifications in DC V with the LCD noted — settles most items
  below at once. Needs any model.
- A first confirmed report per model — gates the spec tables (range,
  resolution, accuracy), which wait for a first hardware capture; the
  manuals and product pages disagree in places (approach doc, "Source
  disagreements"). Needs each model.

## Bring-up

- Whether the meter streams on FFF4 with no challenge written, as OWON's PC
  software expects ([§3.4](reverse-engineered-protocol.md#34-the-challenge))
  — decides whether a decoder needs the MD5 step. Needs any model, subscribe
  only.
- The challenge reply: 16 raw MD5 bytes on FFF1 10 ms after the write (§3.4)
  — settles §3.4. Needs a trace of OWON's app, or a dev build that sends it.
- Whether notifications flow with FFF4's CCCD unset, the PC writing none
  ([§2](reverse-engineered-protocol.md#2-advertising-and-gatt)) — settles
  §2's row. Needs a host that skips the subscribe.

## Advertising and GATT

- The default name "BDM", the FFF0 UUID in the advertisement (the app's
  optional filter) and a public address (the PC's assumption) (§2) —
  decides what a search can match. Needs a passive scan.
- Whether a rename (`40` + name) changes the advertised name, and keeps it
  across a power cycle (§2, §7.2) — settles §2's row. Needs a meter renamed
  by OWON's app.
- Notify or indicate on FFF4, the full 128-bit UUIDs, and write without
  response on FFF1 and FFF3 (§2) — settles §2's rows. Needs GATT discovery.
- The GATT handle layouts, BT (0x25…) and OW (0x15…), per model (§2) —
  settles which models share a GATT table. Needs a discovery per model.
- Advertising while connected, and a second central (§2) — decides whether
  a connected meter shows in a search. Needs two hosts.

## Models and device information

- FFF2 byte 0 per model: 33, 35, 41, 18, 20 and 21 by the app's names; what
  the T and "+" variants and the OW16B send; which product sends 55
  ([§1](reverse-engineered-protocol.md#1-models-and-identification)) —
  decides the model table. Needs each model.
- FFF2 byte 1 as battery percent (PC) and byte 5's `FF` on a meter without
  offline record ([§4](reverse-engineered-protocol.md#4-device-information-fff2))
  — settles §4. Needs a B35 without "+", or a low battery.
- Firmware versions per model, and whether any is below 12 (§4) — decides
  which offline header a meter sends. Needs the FFF2 value per model.
- Whether a CM2100B's frames follow the 6-byte layout byte for byte (§1) —
  settles the CM2100B's place in the 6-byte group. Needs a CM2100B.
- Whether any B35 without "+" sends the 14-byte ASCII frames
  ([§11](reverse-engineered-protocol.md#11-an-earlier-format-14-byte-ascii-pc-source-commented-out))
  — decides whether that format needs a decoder. Needs an older B35 or B35T.

## Live frame

- Frames per notification, and notifications per second against the
  manuals' 3/s (B41T 2/s; its page "BLE link 2 times/s")
  ([§5](reverse-engineered-protocol.md#5-live-frame-framing)) — settles §5.
  Needs the first-report trace.
- Function-word bits 10-15: always `111100` in live frames, as in OWON's
  example ([§6.1](reverse-engineered-protocol.md#61-functionrange-word)) —
  decides whether they can confirm alignment. Needs captures across modes.
- The prefix sent with duty, ℃, ℉, hFE and NCV
  ([§6.3](reverse-engineered-protocol.md#63-prefix)) — settles §6.3. Needs
  those functions.
- Decimal code 5, and the magnitude and sign sent with UL and OL; what UL
  means ([§6.4](reverse-engineered-protocol.md#64-decimal-point-ul-ol)) —
  settles §6.4. Needs Ω with open leads, and a reversed diode.
- The 0x6FFF magnitude as a no-reading marker
  ([§6.5](reverse-engineered-protocol.md#65-reading-word)) — settles §6.5.
  Needs a function change and power-up traced.
- A 5-digit count above 9999 on the B41T, OW18E and CM2100B, with its
  decimal code (§6.5) — settles §6.5 for those models. Needs a reading near
  full scale.

## Functions and flags

- Status bits 6-15: the app's names (AVG, RMR, Loz, …) or the PC's (OL,
  RMR, PMIN, …) ([§6.6](reverse-engineered-protocol.md#66-status-word)) —
  decides which bits are named. Needs captures across modes and keys, OL
  among them.
- RMR, bit 7, "Current value (only B41 model)" in the manuals (§6.6) —
  settles its meaning. Needs a B41T.
- Function 13 on a B-series meter (NCV or "ADP"), and functions 14-15
  (Power W/VA or nothing) ([§6.2](reverse-engineered-protocol.md#62-function-codes))
  — decides those codes' names. Needs a B35 or B41T in every position.
- NCV levels 0-4 and the decimal code sent with them
  ([§6.7](reverse-engineered-protocol.md#67-ncv)) — settles §6.7. Needs NCV
  near a live conductor on an OW16B, OW18B/E or CM2100B.

## Commands and keys

- Short `[code, 01]` and long `[code, 00]` per key and model, and key 7
  ([§7.1](reverse-engineered-protocol.md#71-key-presses-fff3)) — settles
  §7.1. Needs each key pressed remotely, the LCD noted.
- Whether a remote long press of key 4 (B-series △/ᛒ, CM2100 ZERO/ᛒ) or
  key 5 (OW16/OW18 Hz/Duty△/ᛒ) turns Bluetooth off (§7.1) — decides whether
  those long presses may be offered. Needs each.
- Max/Min (key 6) on a B33, which has no such key
  ([§7.3](reverse-engineered-protocol.md#73-key-sets-per-model)) — settles
  §7.3. Needs a B33.
- The `*READlen?` reply: 2 bytes (app) or 4 (PC), and what it counts
  ([§7.2](reverse-engineered-protocol.md#72-fff1-commands)) — settles §7.2.
  Needs a "+" meter with a recording.
- Any reply to `*DATe`, `*RECOrd,`, `*STOP` and the rename, and whether
  `*STOP` ends a recording (§7.2) — settles §7.2. Needs a "+" meter.

## Offline records

- The dump: exactly 20 `FF` on each side, and the u32 at bytes 12-15 (0-3
  before firmware 12) as the record count
  ([§8](reverse-engineered-protocol.md#8-offline-records)) — settles §8.2
  and §8.3. Needs a short recording read back.
- Whether a range word takes a time slot, the apps disagreeing (§8.3) —
  decides the readings' times. Needs a recording across a range change.

## Meter behaviour

- Bluetooth idle-off (10 min; 5 on the CM2100B) while connected and
  streaming ([§9.1](reverse-engineered-protocol.md#91-per-model)) — decides
  whether a long session ends by itself. Needs a connected meter left idle.
- Whether Bluetooth suspends auto power-off on the CM2100B, which its manual
  does not state (§9.1) — same. Needs a CM2100B.
- Whether a long press of the Bluetooth key turns Bluetooth off on the B35,
  B41T, OW16B and OW18B/E, which their manuals do not describe (§9.1) —
  settles §9.1. Needs each.
- B41T(+) capacitance: its own ⊣⊢ position or the Ω position, the manual
  saying both ([§9.2](reverse-engineered-protocol.md#92-functions-per-model))
  — settles §9.2. Needs a B41T.
- Which OW16B and OW18B/E units carry hFE, and the OW16's µA top range, 600
  (dial table) or 6000 µA (spec table) (§9.2) — settles §9.2 and the spec
  tables. Needs units of each variant.

## 15-byte frame

- Any VC831/851/871/891/915/925, OW65/67/69 or CMS101/061 — a trace settles
  §10's G24 bit 11, V24 status 5 and bits 19 and 13-23, the `FF` skip rule
  and the VC871's sub-display under REL/MAX/MIN
  ([§10](reverse-engineered-protocol.md#10-the-15-byte-frame-owons-app-only)).
  Out of scope until a reporter has one.
