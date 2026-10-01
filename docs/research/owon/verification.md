# OWON verification

Open checks for OWON's Bluetooth meters — the B33(T)(+), B35(T)(+),
B41T(+), OW16B, OW18B, OW18E and CM2100B, issues
[#39](https://github.com/antoinecellerier/dmm-tools/issues/39) (OW18B/OW16B),
[#40](https://github.com/antoinecellerier/dmm-tools/issues/40) (OW18E),
[#41](https://github.com/antoinecellerier/dmm-tools/issues/41) (B33),
[#42](https://github.com/antoinecellerier/dmm-tools/issues/42) (B35T+),
[#43](https://github.com/antoinecellerier/dmm-tools/issues/43) (B41T+) and
[#44](https://github.com/antoinecellerier/dmm-tools/issues/44) (CM2100B).
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). Community captures
(spec §14) bear on many of these and are noted per item; they close none,
and each still wants a capture of our own.

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
  software expects ([§3.4](reverse-engineered-protocol.md#34-the-challenge);
  every community client does, on all six models, §14.4) — decides whether a
  decoder needs the MD5 step. Needs any model, subscribe only.
- The challenge reply: 16 raw MD5 bytes on FFF1 10 ms after the write (§3.4;
  OWON's 2018 app read 16 bytes back, lengths only, §14.4) — settles §3.4.
  Needs a trace of OWON's app, or a dev build that sends it.
- Whether notifications flow with FFF4's CCCD unset, the PC writing none
  ([§2](reverse-engineered-protocol.md#2-advertising-and-gatt); raw-HCI and
  gatttool clients get them, §14.4) — settles §2's row. Needs a host that
  skips the subscribe.

## Advertising and GATT

- The default name "BDM", the FFF0 UUID in the advertisement (the app's
  optional filter; absent on a B35T+ and a B41T+, §14.3 D1) and a public
  address (the PC's assumption; public on a B35T+) (§2) — decides what a
  search can match. Needs a passive scan, the CM2100B first.
- Whether a rename (`40` + name) changes the advertised name, and keeps it
  across a power cycle (§2, §7.2; changed on a B41T+, §14.4) — settles §2's
  row. Needs a meter renamed by OWON's app.
- Notify or indicate on FFF4, the full 128-bit UUIDs, and write without
  response on FFF1 and FFF3 (§2; FFF3 only on a B41T+, §14.4) — settles
  §2's rows, and whether OWON's profile is chosen over FFF0 (only where FFF4
  takes no write). Needs GATT discovery per model, OW-handle models first.
- The GATT handle layouts, BT (0x25…) and OW (0x15…), per model (§2; BT on the
  B35T, B35T+, B41T+, OW on the OW18B, OW18E, CM2100B, §14.4) — settles which
  models share a GATT table. Needs a discovery on a B33 and an OW16B.
- Advertising while connected, and a second central (§2; one client at a
  time on a B35T+, §14.4) — decides whether a connected meter shows in a
  search. Needs two hosts.
- What FFF5 holds, read-only behind authentication on a B41T+ (§14.4) —
  decides whether it matters to a decoder. Needs a bonded host.
- Whether any model asks for pairing or bonding before it streams or takes
  key presses (§2; FFF5 needs authentication on a B41T+, §14.4) — decides
  setup's "no pairing". Needs a first connection from a host never paired.
- Whether a host that has cached the GAP Device Name lists "LILLIPUT" rather
  than "BDM" (a B41T+'s name; Windows showed "Lilliput" for a B35T+, §14.4)
  — decides the entries' second Bluetooth name. Needs `dmm-cli list` per OS.

## Models and device information

- FFF2 byte 0 per model: 33, 35, 41, 18, 20 and 21 by the app's names; what
  the T and "+" variants and the OW16B send; which product sends 55
  ([§1](reverse-engineered-protocol.md#1-models-and-identification); 41 on a
  B41T+, §14.5) — decides the model table. Needs each model.
- FFF2 byte 1 as battery percent (PC) and byte 5's `FF` on a meter without
  offline record ([§4](reverse-engineered-protocol.md#4-device-information-fff2);
  byte 1 `FF` on a B41T+, §14.4) — settles §4. Needs a B35 without "+", or a
  low battery.
- Firmware versions per model, and whether any is below 12 (§4; 0.1.2 on a
  B41T+) — decides which offline header a meter sends. Needs the FFF2 value
  per model.
- Whether a CM2100B's frames follow the 6-byte layout byte for byte (§1;
  sercona's Ω run does, §14.5) — settles the CM2100B's place in the 6-byte
  group. Needs a CM2100B.
- Which B35 units send the 14-byte ASCII frames
  ([§11](reverse-engineered-protocol.md#11-an-earlier-format-14-byte-ascii-pc-source-commented-out);
  FS9922 B35T over BLE and the Bluetooth 2.0 B35T over SPP, §14.4) — decides
  whether that format needs a decoder. Needs an older B35 or B35T.

## Live frame

- Frames per notification, and notifications per second against the
  manuals' 3/s (B41T 2/s; its page "BLE link 2 times/s")
  ([§5](reverse-engineered-protocol.md#5-live-frame-framing); one per
  notification, about 1.7/s on a B35T+ and 2/s on a B41T+, §14.4) — settles
  §5. Needs the first-report trace.
- Function-word bits 10-15: always `111100` in live frames, as in OWON's
  example ([§6.1](reverse-engineered-protocol.md#61-functionrange-word); so
  in every community capture) — decides whether they can confirm alignment.
  Needs captures across modes.
- The prefix sent with duty, ℃, ℉, hFE and NCV
  ([§6.3](reverse-engineered-protocol.md#63-prefix); 4 in the B35T+, B41T+
  and OW18B data, §14.4) — settles §6.3. Needs those functions.
- Decimal code 5, and the magnitude and sign sent with UL and OL; what UL
  means ([§6.4](reverse-engineered-protocol.md#64-decimal-point-ul-ol); OL
  sends magnitude 0, sign clear, on a B35T+ and B41T+; UL and code 5 unseen,
  §14.4) — settles §6.4. Needs Ω with open leads, and a reversed diode.
- The 0x6FFF magnitude as a no-reading marker
  ([§6.5](reverse-engineered-protocol.md#65-reading-word); unseen in 840
  B35T+ frames, §14.4) — settles §6.5. Needs a function change and power-up
  traced.
- A 5-digit count above 9999 on the B41T, OW18E and CM2100B, with its
  decimal code (§6.5; seen with code 4 on all three, §14.5; none above 16383,
  §14.3 D4) — settles §6.5 for those models. Needs a reading near full scale.

## Functions and flags

- Status bits 6-15: the app's names (AVG, RMR, Loz, …) or the PC's (OL,
  RMR, PMIN, …) ([§6.6](reverse-engineered-protocol.md#66-status-word); none
  set in any 6-byte capture, bit 6 clear at OL, §14.3 D2) — decides which
  bits are named. Needs captures across modes and keys, OL among them.
- RMR, bit 7, "Current value (only B41 model)" in the manuals (§6.6) —
  settles its meaning. Needs a B41T.
- Function 13 on a B-series meter (NCV or "ADP"), and functions 14-15
  (Power W/VA or nothing) ([§6.2](reverse-engineered-protocol.md#62-function-codes);
  14-15 are W and VA on a 15-byte VC871, §14.5) — decides those codes' names.
  Needs a B35 or B41T in every position.
- NCV levels 0-4 and the decimal code sent with them
  ([§6.7](reverse-engineered-protocol.md#67-ncv); OW18B word 0xF360, values
  0-4, §14.2) — settles §6.7. Needs NCV near a live conductor on an OW16B,
  OW18B/E or CM2100B.
- Function 9 on an OW18E: whether the count is in ℉ or, as one project
  says, in ℃ (§6.2, §14.3 D10) — decides the ℉ decode. Needs an OW18E at ℉
  against its LCD.

## Commands and keys

- Short `[code, 01]` and long `[code, 00]` per key and model, and key 7
  ([§7.1](reverse-engineered-protocol.md#71-key-presses-fff3); community
  long presses: auto range, backlight, Bluetooth off, leave MIN/MAX, §14.4)
  — settles §7.1. Needs each key pressed remotely, the LCD noted.
- Whether a remote long press of key 4 (B-series △/ᛒ, CM2100 ZERO/ᛒ) or
  key 5 (OW16/OW18 Hz/Duty△/ᛒ) turns Bluetooth off (§7.1; `04 00` does on
  a B35T+ per DeanCording, §14.4) — decides whether those long presses may
  be offered. Needs each.
- Hz/Duty on an OW18B/E: key 5, as both OWON programs send, or 4, as
  MartMet sends ([§7.3](reverse-engineered-protocol.md#73-key-sets-per-model),
  §14.3 D6) — decides the OW key table. Needs `04 01` and `05 01` sent.
- `rel` on an OW16B, OW18B or OW18E in AC V, AC A or Hz: the same `05 01`
  as `hz_duty`, so it steps Hz/Duty instead (§7.3). Needs one sent `rel` there.
- Max/Min (key 6) on a B33, which has no such key (§7.3) — settles §7.3.
  Needs a B33.
- Whether the FFF2 read fails on any model or host (`dmm-cli info`: "device
  information: not read"), keys then going out unchecked — decides whether
  they stay on without a model code. Needs `dmm-cli info` per model and OS.
- The `*READlen?` reply: 2 bytes (app) or 4 (PC), and what it counts
  ([§7.2](reverse-engineered-protocol.md#72-fff1-commands); the payload's
  byte count on a B41T+, §14.4) — settles §7.2. Needs a "+" meter with a
  recording.
- Any reply to `*DATe`, `*RECOrd,`, `*STOP` and the rename, and whether
  `*STOP` ends a recording (§7.2; `*RECOrd,` disconnects, §14.2) — settles
  §7.2. Needs a "+" meter.

## Offline records

- The dump: exactly 20 `FF` on each side, and the u32 at bytes 12-15 (0-3
  before firmware 12) as the record count
  ([§8](reverse-engineered-protocol.md#8-offline-records); both sides and a
  byte count on a B35T+ and B41T+, §14.3 D7-D8) — settles §8.2 and §8.3.
  Needs a short recording read back.
- Whether a range word takes a time slot, the apps disagreeing (§8.3; a new
  range word comes at each change, §14.2) — decides the readings' times.
  Needs a recording across a range change.

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
- A dropped link (out of range, idle-off, power-off): whether the meter
  advertises again without the ᛒ key held (B35-UM p.33/28: reconnect after a
  restart) — decides the reconnect steps. Needs a meter walked out of range.

## Detection

- Two frames in one `LISTEN_ONLY_WINDOW` (1.5 s) with no model code read, at
  about 1.7 frames a second (B35T+, §14.4) — decides that window, or one
  frame on a "BDM" link. Needs a `RUST_LOG=dmm_lib=debug` detection, timed.
- What the UT61+, UT181A and UT171 probes do to an OWON meter and the OWON
  detection row: [Device auto-detection](../../verification-backlog.md#device-auto-detection);
  a chance OWON claim on a UT-D07B link: [Bluetooth](../../verification-backlog.md#bluetooth).

## 15-byte frame

- Any VC831/851/871/891/915/925, OW65/67/69 or CMS101/061 — a trace settles
  §10's G24 bit 11, V24 status 5 and bits 19 and 13-23, the `FF` skip rule
  and the VC871's sub-display under REL/MAX/MIN
  ([§10](reverse-engineered-protocol.md#10-the-15-byte-frame-owons-app-only);
  VC871 captures show bit 11 marking the sub-display word and G24 bits 16-23
  `F0`, §14.4). Out of scope until a reporter has one.
