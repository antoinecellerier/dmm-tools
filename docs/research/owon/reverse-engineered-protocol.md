# OWON Bluetooth multimeters (B33, B35T+, B41T+, OW16B, OW18B, OW18E, CM2100B): Reverse-Engineered Protocol Specification

What OWON's Bluetooth LE multimeters send and accept. After a connection the
meter notifies 6-byte frames on characteristic FFF4: a 16-bit function/range
word (function, SI prefix, decimal-point position), a 16-bit status word and
a 16-bit sign-and-magnitude count, all little-endian, with no header, length
or checksum. A one-off read of FFF2 gives the model code and firmware version;
16-byte commands (record, read-back, rename and a challenge/response) go to
FFF1, and 2-byte key presses to FFF3. OWON's app also decodes a 15-byte frame
for Voltcraft-branded and other OWON meters (§10, app only). Not implemented,
and no OWON meter has been on our bench: every fact in §1-12 comes from OWON's
Android app, OWON's PC software source and OWON's manuals and product pages.
The community cross-reference has not been done (§14). The approach doc beside
it records the sources, the method and the clean-room boundary.

Based on:
- OWON's Android app iMeter 1.2.4 (`com.owon.imeter`), a Flutter build,
  decompiled with blutter
- OWON's PC software pcMultimeter 1.4.5 (Windows 7/8 package "multimeterBLE",
  plug-in 1.4.4), whose Java source OWON ships in the package as an Eclipse
  source bundle; it reaches the meter through OWON's TI CC2540 USB dongle
- The eight OWON manuals (33 series, 35 series & B41T, OW16, OW18, CM2100;
  user manuals and quick guides), read from the rendered pages
- OWON's product pages on owon.com.hk and the datasheets they link

Citation keys:
- **app `file:lines`** — the blutter listing under
  `references/owon/app/blutter/asm/`; every cited file name is unique under
  `owon_imeter/` and `imeter_base/`. Short forms:
  **BBM** = `owon_imeter/device_manager/device/built_in_ble_multimeter.dart`,
  **BM** = `imeter_base/device_basics/ble_multimeter.dart`,
  **PE** = `imeter_base/protocol_parser/protocol_element.dart`,
  **MD** = `imeter_base/device_basics/type_inf/multimeter_device.dart`,
  **R2W** = `imeter_base/protocol_parser/r2w_protocol_parser.dart`,
  **R10W** = `imeter_base/protocol_parser/r10w_protocol_parser.dart`,
  **OFF** = `imeter_base/protocol_parser/offline_data_parse_helper.dart`,
  **AUTH** = `imeter_base/utils/auth_tool.dart`
- **pc `path:lines`** — `references/owon/pc/win7/src/`, the source bundle
  `com.owon.pc.multimeter.source_1.4.4.jar` unpacked. Short forms:
  **MC** = `model/MultimeterClient.java`, **BC** = `kernal/BleClient.java`,
  **BSI** = `model/BleSeriesInfo.java`, **HC** = `kernal/HexCmds.java`,
  **ODM** = `model/offlineRec/OfflineDataModelVariant.java`
- **Manuals**, `<key> p.N/M` = PDF page N, printed page M, under
  `references/owon/manuals/`: **B33-UM** (`OWON_33_Series_…USER_MANUAL.pdf`,
  Oct. 2018 V1.2.2), **B35-UM** (`OWON_35_Series&B41T_USER_MANUAL.pdf`, Sep.
  2026 V1.4.0), **B35-QG** (its quick guide, 2026.09 V1.6.6), **OW16-UM**
  (July 2019 V1.0.3), **OW16-QG** (2018.10 V1.0.1), **OW18-UM** (Nov 2023
  V1.1.3), **OW18-QG** (2023.11 V1.1.1), **CM2100-UM** (Jan. 2026 V1.0.3)
- **page `<model>`** — the product page saved under `references/owon/web/`
  (B35 = `products_owon_3_5-6_…`, B41T = `products_owon_4_1-2_…_b41t.html`)

Byte and bit numbering: byte 0 is the first byte of a frame or value; bit 0
is the least significant bit of a little-endian word.

Confidence levels:
- **[KNOWN]** — stated in an OWON manual or on OWON's site, cited by page
- **[VENDOR, app]**, **[VENDOR, PC]**, **[VENDOR, both]** — read from OWON's
  app, OWON's PC source, or both independently, cited by file and lines
- **[INFERRED]** — deduction from the above, reason given
- **[UNVERIFIED]** — no source confirms it, or the sources disagree; needs a
  real meter ([verification.md](verification.md) lists the checks)
- **[HARDWARE]** — seen on a real meter: none yet
- **[COMMUNITY]** — from a community source, §14 only: none yet

---

## 1. Models and identification

The manuals' Bluetooth models [KNOWN]:

| Manual | Bluetooth models | Siblings without Bluetooth | Cite |
|---|---|---|---|
| 33 series | B33(T)(+) | D33(T) | B33-UM p.1, p.50/45 |
| 35 series & B41T | B35(T)(+), B41T(+) | D35(T) | B35-UM p.1, p.54/49 |
| OW16 | OW16B | OW16A | OW16-UM p.1, p.50/45 |
| OW18 | OW18B, OW18E | OW18A, OW18D | OW18-UM p.1, p.48/43 |
| CM2100 | CM2100B | CM2100 | CM2100-UM p.1, p.34/31 |

"T" is true RMS and "+" the offline record function, both optional (B35-UM
p.1; B33-UM p.1) [KNOWN].

**The model code.** Both vendor programs identify a meter after connecting,
by byte 0 of the FFF2 read (§4), never by its advertisement [VENDOR, both:
BBM:1949-2004; BC:213-219]. The codes and the names each source gives them:

| Code | App name (MD line) | PC name (BSI:6) | Frame |
|---|---|---|---|
| 18 | OW18B (`:72`) | `OW18_16` | 6-byte |
| 20 | OW18E (`:99`) | `OW18E` | 6-byte |
| 33 | B33 (`:131`) | `B33` | 6-byte |
| 35 | B35 (`:163`) | `B35` | 6-byte |
| 41 | B41 (`:190`) | `B41` | 6-byte |
| 21 | CM2100 (`:222`) | — | 6-byte |
| 55 | — | `OW55` | 6-byte, own sign rule (§6.5) |

Both [VENDOR] for their columns. The app's other codes are 15-byte meters
(§10.1). Notes:

- The app's names are the manuals' series names without the T and +
  suffixes [INFERRED by name]; which code a T or a "+" variant sends is
  [UNVERIFIED].
- **OW16B**: none found in the app's model table or the PC source,
  2026-10-01. The PC's name `OW18_16` for 18 is the only link [INFERRED
  from the name].
- **55** is in the PC source only; its product is not named anywhere in
  these sources [UNVERIFIED].
- **21** (CM2100) is in the app only; the PC keeps an unknown code as read
  (BC:214-216) and decodes it like a B-series meter, its series tests all
  false (BSI:56-69) [VENDOR, PC].
- **223** (0xDF) has counts (20000) and shares the CM2100's full-scale table
  in the app, but no model entry, so the app rejects a meter that sends it
  (MD:6739-6793; `multimeter_fs.dart` ~1069-1080) [VENDOR, app]; its product
  is [UNVERIFIED].

**What differs per code** in the two programs [VENDOR]:

| Code | App digits | App counts | PC digits | PC GATT handles | PC function 13 |
|---|---|---|---|---|---|
| 18 | 4 | 6000 | 4 | OW | NCV |
| 20 | 5 | 20000 | 4 | OW | NCV |
| 33 | 4 | 4000 | 4 | BT | ADP |
| 35 | 4 | 6000 | 4 | BT | ADP |
| 41 | 5 | 22000 | 5 | BT | ADP |
| 21 | 5 (4 for DC A and AC A) | 20000 | 4, as unknown | BT, as unknown | ADP, as unknown |
| 55 | — | — | 5 | OW | NCV |

App: MD:21-744 (fields at :6417-6506), counts from `baseCounts`
(MD:6739-6793), the CM2100's 4-digit current from MD:6358-6401 and
BM:1110-1137. PC: digits MC:1626-1656, handles BSI:67-69 and
`kernal/Command.java:184-190`, function 13 MC:1514-1520. Digits and counts
only shape the text and bar graph each program draws; the value decode is the
same for every code but 55 [VENDOR, both]. The manuals' counts are in §9.1.

## 2. Advertising and GATT

| Item | Value | Source | Tag |
|---|---|---|---|
| Advertised name | "BDM" by default: the app's device list shows it ("select BDM", "Click 'BDM' in the device list to pair") | B35-UM p.26/21; OW18-UM p.24/19; OW16-UM p.25/20; B33-UM p.24/19; CM2100-UM p.17/14-18/15 | [KNOWN] |
| Rename | the app can rename the meter; the name "will be memorized in the device"; "Only digits, letters and underscore can be entered" | B35-UM p.28/23 | [KNOWN]; the command is §7.2 |
| Renamed name advertised | the app shows only advertised names in its list, and the manual says the name is stored in the meter | — | [INFERRED]; [UNVERIFIED] on a meter |
| Name filter | none in either program. The app shows any peer whose name, stripped to `[0-9a-zA-Z_]`, is not empty, de-duplicated by id (`device_add_view_model.dart:202-563`; `built_in_ble_device.dart:36-78`); the PC lists every advertiser by address, its name field left `""` with the comment `// "BDM"` (`kernal/Event.java:178-191`, `:188`) | app, PC | [VENDOR, both] |
| Other advertising data | read by neither: the app uses only the device id and name; the PC ignores the event type, address type and advertising data (`kernal/Event.java:159-203`) | app, PC | [VENDOR, both] |
| Service filter | the app's optional "Filter device" switch scans with service `0000fff0-0000-1000-8000-00805f9b34fb` (`ble_device_scanner.dart:30`, `:130-145`); it is off by default in OWON's build (`device_add_view_model.dart:63-83`, `:181-185`; `main.dart:13-21`). The manuals: "Filter device" hides incompatible meters (B35-UM p.25/20-26/21) | app, manual | [VENDOR, app], [KNOWN]; that the meter advertises the FFF0 UUID [INFERRED from the filter]; [UNVERIFIED] |
| Address type | the PC connects with the peer address type hard-coded to public (`kernal/Command.java:160-176`) | PC | [VENDOR, PC]; that the meters use a public address [INFERRED] |
| Service | FFF0, required by the app in its short or 128-bit form (`owon_imeter/device_manager/utils.dart:92-146`). No service UUID appears in the PC source; it discovers nothing (`kernal/BleAgent.java:161-163`) | app | [VENDOR, app] |
| FFF1 | commands (§7.2); read back for the challenge reply and the `*READlen?` reply. PC name "RW" | app, PC | [VENDOR, both] |
| FFF2 | device information, read once (§4). PC name "R" | app, PC | [VENDOR, both] |
| FFF3 | key presses (§7.1). PC name "W" | app, PC | [VENDOR, both] |
| FFF4 | every meter-to-host byte: live frames and the offline dump. PC name `CHAR4_NTF_HANDLE`, "NTF" | app, PC | [VENDOR, both] |

Cites for the four rows: app BBM:2351-2466 (each discovered UUID lower-cased
and matched against "fff1".."fff4"), FFF1 writes and reads BBM:44, 343, 556,
864, 1052, 1272, 1602, :1113, :1722, FFF2 :1929-1957, FFF3 :189-224, FFF4
:1402-1446; PC `kernal/Command.java:14-21`, `model/SeriesBT.java:6-9`,
`model/SeriesOW.java:6-9`. The app matches substrings, so the full
characteristic UUIDs are never written out; the 16-bit-base forms
`0000fff1…fff4-0000-1000-8000-00805f9b34fb` are [INFERRED].

**Two GATT layouts.** The PC writes by fixed handle, chosen by the model code
[VENDOR, PC]:

| Characteristic | "BT" handles (33, 35, 41, unknown) | "OW" handles (18, 20, 55) |
|---|---|---|
| FFF1 | 0x0025 | 0x0015 |
| FFF2 | 0x0028 | 0x0017 |
| FFF3 | 0x002B | 0x0019 |
| FFF4 | 0x002E | 0x001B |

Reads go by UUID (BC:122-146), so the FFF2 read works before the code is
known. The spacing, 3 per characteristic on BT and 2 on OW, suggests two
different GATT tables [INFERRED]; where FFF4's CCCD sits is not shown
[UNVERIFIED].

**Notify, CCCD, write types.**
- The PC handles ATT notifications (event 0x051B) and no indications
  (`kernal/Event.java:11-23`, `:77-109`), so FFF4 notifies [INFERRED]. The
  app subscribes with the plugin's plain call (`ble_adapter.dart:548`)
  [VENDOR, app].
- The PC never writes a CCCD (no descriptor write in the source) and parses
  every notification on the link whatever its handle (BC:90-96, :148-166)
  [VENDOR, PC]. Whether the meter notifies with its CCCD unset is
  [UNVERIFIED].
- The app writes every characteristic with response
  (`ble_adapter.dart:392`); the PC writes FFF3 with a Write Request and FFF1
  with a long write (Prepare + Execute, TI's GATT_WriteLongCharValue,
  `kernal/Command.java:25-78`) [VENDOR, both]. So FFF1 takes both forms for a
  16-byte value [INFERRED]. Write without response: [UNVERIFIED].
- Neither program requests an MTU [VENDOR, both]; every command is at most 16
  bytes and fits the default ATT MTU of 23 [INFERRED].

Whether the meter advertises while connected, and how many centrals it
accepts: not stated in the manuals [UNVERIFIED]. The app and the PC can each
hold several meters (B35-UM p.25/20; PC "up to three", B35-UM p.49/44)
[KNOWN].

## 3. Bring-up

### 3.1 What the meter needs

No start, poll or keep-alive command: none found in the app or the PC source,
2026-10-01. The app subscribes to FFF4 and writes nothing periodically; the
PC writes nothing at all before readings arrive [VENDOR, both]. So a
connection, with FFF4 notifications enabled as a standard host does, is all
the stream needs [INFERRED]; the challenge is §3.4.
"When the multimeter restarts or shuts down, it needs to be reconnected"
(B35-UM p.33/28) [KNOWN].

### 3.2 What OWON's app does

[VENDOR, app], in order (BBM:1138-1204):

1. Connect with no service list (`ble_adapter.dart:584-603`); discover,
   require FFF0 and the four characteristics, read FFF2 (§4).
2. Byte 0 must be a code in the model table (§1), else the connect fails.
3. The challenge of §3.4 on FFF1; a wrong reply fails the connect.
4. Subscribe to FFF4.
5. For codes 87, 89, 91, 92, 67 and 69 only (15-byte meters), `#TIMEsync`
   (§10.8) (MD:6302-6350).

A failure's text "The device is not supported! code:0..3" (no FFF0, fewer
than four characteristics, unknown model code, challenge failed) goes only
to the console; the UI shows −1 (no FFF0 service), −2 (any of those four) or
−3 (wrong device class) (`ble_adapter.dart:428-533`;
`built_in_ble_device.dart:209-235`). No reconnect or retry
(`device_base.dart:14-59`).

### 3.3 What OWON's PC software does

[VENDOR, PC]. Through the CC2540 dongle (TI's HCI over a USB serial port,
115200 baud, `kernal/BleAgent.java:30`, `:114-115`; the manual's dialog shows
115200 greyed, OW18-UM p.39/34): connect, read FFF2 by UUID
(`model/BleSlaveManager.java:94-107`; BC:204-275), then parse every
notification. Nothing is written during bring-up: an identity check exists
only as a comment, `// sendCommand(ID_VERIFY, null);` (BC:272; constant
MC:49-50).

### 3.4 The challenge

OWON's app checks the meter's answer to a challenge on every connect
(AUTH; BBM:1526-1797) [VENDOR, app]:

1. Draw six values r0..r5, each 0-35 (`nextInt(36)`, AUTH:280-426).
2. Write 16 bytes to FFF1: r_i + [200, 100, 50, 20, 10, 5]_i for i = 0..5,
   then ten `00` (AUTH:224-279; BBM:1552-1636).
3. Wait 10 ms, read FFF1, and turn each byte into lower-case hex without zero
   padding (BBM:1646-1784).
4. Compare that with an MD5 the app computes: of a 6-character string whose
   first three characters are r0..r2 looked up in one 36-character table of
   the app and last three r3..r5 in another (tables at AUTH:766-973 and
   :974-1181), hex-encoded the same way (AUTH:460-765; BBM:1785-1797).

So the meter is expected to answer with the 16 raw bytes of that MD5
[INFERRED from the comparison]: the challenge authenticates the meter to the
app, not the app to the meter. OWON's PC software never sends it and still
parses readings (§3.3) [VENDOR, PC], so the meters it was written for stream
without it [INFERRED]. Whether a later firmware withholds readings until it
is answered is [UNVERIFIED].

## 4. Device information, FFF2

| Byte | App (PE:7209-7424; BM:41-163) | PC (BC:206-275; BSI:85-106) |
|---|---|---|
| 0 | model code (§1) | series ID, read signed |
| 1 | read, discarded | battery, percent; 0-100 valid, anything else "not supported"; logged only |
| 2-4 | firmware version, shown "b2.b3.b4" in decimal | firmware a.b.c, v = a·100 + b·10 + c |
| 5 | `1` = recording in progress | `FF` offline record not supported; `00` supported; `01` supported and recording |

[VENDOR] per column. The PC tolerates a shorter value, reading each field
only if present (BC:214, 227, 240, 248). OWON's example is 16 bytes (§12.2).

**Firmware gates** [VENDOR, both]: the PC enables rename from v ≥ 11
("from 0.1.1") and the dated offline header from v ≥ 12 ("from 0.1.2")
(BSI:85-91); the app picks the old offline read-back when the version with
its dots removed, parsed as an integer, is below 12 (BM:176-226). Both read
"0.1.2" as 12 and "4.0.9" as 409 [INFERRED from the arithmetic]. Which
models run a version below 12 is [UNVERIFIED].

## 5. Live frame: framing

| Rule | Source | Tag |
|---|---|---|
| A frame is 6 bytes: three 16-bit little-endian words | MC:212 (`COMMON_ONE_DATA_LENGTH = 6`), :1366-1388; BM:1155-1233; `bits_utils.dart:6-79` | [VENDOR, both] |
| No start byte, length, sequence number or checksum | MC:1366-1733; R2W:10-147 | [VENDOR, both]; alignment is positional, so a lost byte shifts every later frame [INFERRED] |
| Frames per notification: the PC joins notifications and carries a remainder over; the app cuts each notification into 6-byte chunks and drops a shorter remainder | MC:1374-1388, :1720-1730; BM:1155-1233 | [VENDOR, both]; whether a notification ever holds anything but whole frames is [UNVERIFIED] |
| Rate: the manuals give 3 samples a second (B33-UM p.50/45; B35-UM p.54/49; OW16 and OW18 p.48/43; CM2100-UM "Numerical Value Conversion Rate", p.33/30) and 2 for the B41T ("Sample rate for digital data", B35-UM p.54/49); the B41T page gives "shift rate(BLE link) 2 times/s" and "Update rate 3 times/s" | manuals, page B41T | [KNOWN]; the notification rate is [UNVERIFIED] |

Every frame is a reading: the PC decimates nothing (MC:1663-1668, :1709)
[VENDOR, PC]. The app drops frames whose magnitude is the 0x6FFF sentinel
(§6.5) [VENDOR, app].

## 6. Live frame: fields

| Bytes | Word | Bits |
|---|---|---|
| 0-1 | function/range | 0-2 decimal-point code (§6.4); 3-5 prefix (§6.3); 6-9 function (§6.2); 10-15 not read in live frames (series 55: bit 10 sign) |
| 2-3 | status | flags (§6.6) |
| 4-5 | reading | 0-14 magnitude; 15 sign (series 55: 0-15 magnitude) |

Cites: app R2W:580-685 (function word), :539-578 (status), :149-201
(reading); PC MC:1411-1449, :1527-1549, :1550-1570 [VENDOR, both].

### 6.1 Function/range word

`dp = w & 7`, `prefix = (w >> 3) & 7`, `function = (w >> 6) & 0xF`
[VENDOR, both]. Bits 10-15 are read by neither program in live frames, but
for series 55's sign (§6.5). OWON's own live example carries `111100` there
(gear `0xF019`, §12.1), the same pattern that marks a function/range word in
an offline dump (§8.3) [VENDOR, PC example; INFERRED comparison]. Whether
every live frame carries it is [UNVERIFIED].

### 6.2 Function codes

| Code | App (PE:122-869): label, unit | PC (MC:1449-1525; names `storeroom/util/FuncUnit.java:4-6`) |
|---|---|---|
| 0 | DC, V | DC, V |
| 1 | AC, V | AC, V |
| 2 | DC, A | DC, A |
| 3 | AC, A | AC, A |
| 4 | RES, Ω | RES, Ω |
| 5 | CAP, F | CAP, F |
| 6 | Hz, Hz | Hz, Hz |
| 7 | DUTY, % | DUTY, % |
| 8 | TEMP, ℃ | TEMP, ℃ |
| 9 | TEMP, ℉ | TEMP, ℉ |
| 10 | DIODE, V | DIODE, V |
| 11 | CONT, Ω | CONT, Ω |
| 12 | hFE, no unit | hFE, unit string "hFE" |
| 13 | NCV, no unit (every model) | "NCV" on series 18, 20, 55; "ADP" on every other (unit string the same, unit type empty) |
| 14 | Power, W | "Null" |
| 15 | Power, VA | "Null" |

Codes 0-12 agree [VENDOR, both]. The manuals' app screens list the
function codes DC, AC, RES, CONT, DIODE, CAP, Hz, DUTY, TEMP and **POWER**
("Power measurement") (B35-UM p.27/22-28/23; the same screens in OW18-UM
p.25/20-26/21) [KNOWN]. Disagreements, all [UNVERIFIED]:

- **13**: NCV in the app on every model, "ADP" in the PC on the B-series. No
  NCV and no ADP position is on the B33, B35 or B41T dials (§9.2), while the
  OW16, OW18 and CM2100 have NCV [KNOWN]. What a B-series meter sends as 13,
  if anything, is open.
- **14-15**: the app's Power W and Power VA against the PC's nothing. No
  dial in these manuals has a power position; the B35 LCD carries an
  unexplained "RPM" segment and a lightning bolt, and its app screen the
  "POWER" code (B35-UM p.15/10, p.27/22) [KNOWN].
- Every 4-bit code is in the app's table; it has no entry above 29
  (PE:4425-4481), which only the 15-byte frame can reach (§10.5) [VENDOR,
  app].

### 6.3 Prefix

| Code | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| Prefix | p | n | µ | m | none | k | M | G |

App PE:1008-1200 (multipliers 1e-12 … 1e9); PC MC:1417-1446 ("K" for k)
[VENDOR, both]. For duty, ℃, ℉, hFE and NCV (7, 8, 9, 12, 13) the app forces
the prefix to 4 and leaves it out of the value and unit (PE:871-947,
:5798-5873); the PC leaves it out for duty, hFE and 13 but prefixes ℃ and ℉
(MC:1489-1498) [VENDOR, both]. What the meter sends in bits 3-5 for those
functions is [UNVERIFIED]; 4 is [INFERRED] from the app forcing it.

### 6.4 Decimal point, UL, OL

| Code | App (PE:4174-4366) | PC (MC:1572-1606) |
|---|---|---|
| 0 | count × 1 | count × 1 |
| 1 | ÷ 10 | ÷ 10 |
| 2 | ÷ 100 | ÷ 100 |
| 3 | ÷ 1000 | ÷ 1000 |
| 4 | ÷ 10⁴ | ÷ 10⁴ |
| 5 | ÷ 10⁵ | "err point": the count is not scaled |
| 6 | text "UL" | text "UL" |
| 7 | text "OL" | text "OL" |

[VENDOR, both]. The scaled count is in the unit of §6.2 with the prefix of
§6.3: count ÷ 10^dp × prefix [VENDOR, both: the app's SI value at
PE:5798-5873; the PC keeps display units, MC:1682]. A display of at most
five digits (§9.1) needs at most four decimals [INFERRED], so code 5's
meaning is [UNVERIFIED].

- **OL** is the meter's overload display: "the reading exceeds the display
  range" (B35-UM p.15/10), reversed diode leads (B35-UM p.20/15), a current
  range exceeded (OW18-UM p.21/16) [KNOWN]. **UL**: none found in the eight
  manuals, 2026-10-01; "underload" is [INFERRED from the pairing with OL].
- Under UL both programs keep the raw count. Under OL the PC keeps it, while
  the app replaces the value with 6000 (4-digit models) or 20000 (the
  others), testing the text for "OL" alone (MC:1594-1598; R2W:378-497)
  [VENDOR, both]. What the meter puts in the magnitude then is
  [UNVERIFIED].
- Both programs prepend the sign to any text when bit 15 is set, "-UL" and
  "-OL" included (MC:1658-1661; R2W:498-507) [VENDOR, both]; whether a meter
  sends a negative UL or OL is [UNVERIFIED].

### 6.5 Reading word

- **Sign and magnitude**, not two's complement: bit 15 set is negative,
  bits 0-14 the magnitude (`& 0x7FFF`) (MC:1565-1569; R2W:149-201) [VENDOR,
  both]. The app takes the sign byte with a listing selector read as `last`
  [INFERRED; the PC's `high >> 7` settles it]. The largest count in these
  manuals, 22000 (B41T), fits in 15 bits [INFERRED].
- **Series 55** (PC only): the magnitude is all 16 bits, unsigned, and the
  sign is function-word bit 10 (MC:1561-1564) [VENDOR, PC].
- **0x6FFF**: the app treats a magnitude of 0x6FFF (28671) as "no reading"
  and drops the frame (R2W:202-223) [VENDOR, app]; the PC has no such check
  [VENDOR, PC]. Whether a meter sends it is [UNVERIFIED].
- **Digits.** Both programs show the count zero-padded to the model's digits
  with the point inserted (MC:1626-1656; R2W:400-472) [VENDOR, both]; the
  digits differ by program for code 20 (§1). Neither program changes the
  number itself.

### 6.6 Status word

App: the bit is set when `(flags & mask) == mask` (PE:3447-3532, names
PE:3534-3917) [VENDOR, app]. PC: an enum `VBAR(13) … HOLD(0)` matched
against the word's binary string (MC:1531-1548, :1743-1745); the names are
[VENDOR, PC] and their bit positions [INFERRED from that arithmetic, which
agrees with the enum's codes, for words below 0x4000].

| Bit | Mask | App | PC |
|---|---|---|---|
| 0 | 0x0001 | HOLD | HOLD |
| 1 | 0x0002 | REL | REL |
| 2 | 0x0004 | AUTO | AUTO |
| 3 | 0x0008 | Bat | Bat |
| 4 | 0x0010 | MIN | MIN |
| 5 | 0x0020 | MAX | MAX |
| 6 | 0x0040 | AVG | OL |
| 7 | 0x0080 | RMR | RMR |
| 8 | 0x0100 | Loz | PMIN |
| 9 | 0x0200 | LPF | PMAX |
| 10 | 0x0400 | Peak | UL |
| 11 | 0x0800 | — | LPF0 |
| 12 | 0x1000 | CosPhi | LPF1 |
| 13 | 0x2000 | AC | VBAR |
| 14 | 0x4000 | DC | — |
| 15 | 0x8000 | USB | — |

- **Bits 0-5**: the names agree [VENDOR, both]; the PC's positions are
  [INFERRED] as above. The manuals' app screens list the
  status codes HOLD, REL, AUTO, Bat, MAX, MIN and RMR, "Current value (only
  B41 model)" (B35-UM p.27/22-28/23) [KNOWN]; "Bat" as low battery is
  [INFERRED from the LCD's battery-low icon, B35-UM p.15/10].
- **Bits 6-15**: only bit 7, RMR, agrees. The app's table also serves the
  15-byte meters (§10), whose words run to bit 18. The meanings of bits 6-15
  in a 6-byte meter's frames are [UNVERIFIED].
- With bit 14 or 15 set, the PC's string matching shifts or flags every
  status [INFERRED from MC:1531-1548], so the PC assumes them clear.

### 6.7 NCV

| Value | Text |
|---|---|
| 0 | EF |
| 1 | - |
| 2 | - - |
| 3 | - - - |
| 4 | - - - - |

App PE:4033-4115 (other values throw, so no reading); PC MC:1608-1625 on
series 18, 20 and 55, switched on the scaled value truncated to an integer
(MC:1393, :1609), so the count-to-text table holds as given only for decimal
code 0 [VENDOR, both]. The CM2100 shows "EF" below
90 V and one to four dashes above, by field strength (CM2100-UM p.13/10)
[KNOWN]. The OW16 and OW18 signal NCV with an LED and a beep and name no
display word (OW18-UM p.19/14-20/15) [KNOWN].

## 7. Commands

### 7.1 Key presses, FFF3

Two bytes, `[key code, press]`: press `01` short, `00` long (PC
`control/MainControl.java:89-107`; app BBM:166-238, the UI passing short from
`onPressed` and long from `onLongPress`,
`device_control_normal_widget_provider.dart:1700-1721`,
`device_control_voltcraft_widget_provider.dart:236-277`, ~:605-640,
`device_control_view_model.dart:387`) [VENDOR, both]. No reply is read
[VENDOR, both]. The PC's long press needs the button held more than 2 s
(`frame/MainFrame.java:359-380`) [VENDOR, PC]. The manuals: the soft keys
"can be short or long pressed … just as press the corresponding keys"
(OW18-UM p.42/37; CM2100-UM p.19/16) [KNOWN].

| Code | App name | PC name | PC button, BT handles (`frame/MainFrame.java:302-328`) | PC button, OW handles (`:330-348`) |
|---|---|---|---|---|
| 1 | Select | selectPress | Select | Select |
| 2 | Range | rangePress | Range | Range |
| 3 | Hold / Hold/Light | holdPress | Light/H | Hold |
| 4 | Rel / Zero (CM2100) | relPress | Rel/BT | — |
| 5 | Hz/Duty / △/Hz/Duty | hzPress | Duty | Rel/Hz/Duty |
| 6 | Max/Min | maxPress | Max/Min | — |
| 7 | — | allPress, defined, no button | — | — |

[VENDOR] per column. Key 4 is the B-series △/ᛒ key and the CM2100's
ZERO/ᛒ key, key 5 the OW16/OW18 Hz/Duty△/ᛒ key (§9.1), and a long press of
those keys switches Bluetooth on (and on the B33 and CM2100 off) [INFERRED by
name from §9.1]. What a remote long press of them does is [UNVERIFIED].

### 7.2 FFF1 commands

16 bytes each, ASCII-led [VENDOR, both]. The app writes them with response;
the PC with a long write (§2).

| Command | Bytes | App (PE; BBM) | PC (HC; callers) |
|---|---|---|---|
| Read length | `*READlen?` `2A 52 45 41 44 6C 65 6E 3F` + 7×`00` | sent on the read page; 500 ms later reads FFF1 and takes bytes 0-1 as a u16 LE, shown "N bytes" (PE:2854-2937; BBM:1028-1130; BM:233-273) | sent when the read dialog opens; on the write response reads FFF1 and takes bytes 0-3 as a u32 LE, labelled "bytes" (HC:13-17; BC:178-202) |
| Read records, old | `*READ?` `2A 52 45 41 44 3F` + 10×`00` | firmware < 12 (PE:1734-1901; BM:176-226) | firmware < 12 (HC:20-24; BC:279-295) |
| Read records | `*READ1?` `2A 52 45 41 44 31 3F` + 9×`00` | firmware ≥ 12 | firmware ≥ 12 (HC:27-32) |
| Set date | `*DATe` `2A 44 41 54 65` + CC YY MM DD hh mm ss + 4×`00` | before every `*RECOrd,` (PE:2305-2853; BBM:884-901) | built, call commented out (HC:34-44; `frame/RecordSetDialog.java:196-203`) |
| Start record | `*RECOrd,` `2A 52 45 43 4F 72 64 2C` + interval u32 LE + count u32 LE | PE:1902-2303; BBM:832-1001 | HC:47-60; then disconnects 1 s later (`frame/RecordSetDialog.java:215-225`) |
| Stop record | `*STOP` `2A 53 54 4F 50` + 11×`00` | offered at connect when FFF2 byte 5 is `01` (PE:1202-1285; BBM:16-80) | defined, never called (HC:62-66) |
| Rename | `40` + name, zero-padded to 16 | name ≤ 15 characters (BBM:239-393), of `[A-Za-z0-9_]` (`change_name_page.dart:557-571`) | `@` + up to 15 bytes, defined, never called; gated on firmware ≥ 11 (HC:68-80; BSI:89) |

- **Strings.** The byte strings agree in both sources [VENDOR, both]. In the
  app `*DATe` and `*RECOrd,` are built from code points (PE:2347-2363,
  ~:1925-1950); the others are literal strings.
- **Date fields** are binary, not BCD, in local time: CC is the century
  (20), then YY MM DD hh mm ss [VENDOR, both: the app from `DateTime`, the PC
  parsing `yyyyMMddkkmmss` two digits at a time,
  `storeroom/util/DateUtil.java:11, 21-24`]. The app's hour is 0-23, the
  PC's `kk` 1-24 [VENDOR, both].
- **`*RECOrd,`**: interval first, then count — the app's validation names
  argument 1 间隔 (interval) and argument 2 总条数 (total count)
  (PE:1980-2010), and its `OfflineRecordConfig` keys are `interval` and
  `recordLen` (PE:5518-5762)
  [VENDOR, app]; the PC's builder orders them so (HC:47-60) [VENDOR, PC].
  The interval is in seconds: the PC labels it "Sec" [VENDOR, PC], and the
  manual says to "enter the recording interval time in seconds" (B35-UM
  p.40/35) [KNOWN].
  The count is at most 10,000 (B35-UM p.30/25-32/27; OW18-UM p.28/23-30/25)
  [KNOWN]. The meter "disconnects within about 2 s" and logs to its own
  memory; one recording is kept, a new one overwrites it (B35-UM
  p.30/25-32/27) [KNOWN].
- **Rename**: the app ignores the write's result and stores the name
  locally; it refuses only a name longer than 15 characters (BBM:295,
  :365-381) [VENDOR, app].
- **Replies.** Neither program reads a reply to any FFF1 command but the
  challenge and `*READlen?` [VENDOR, both]. The `*READlen?` width, 2 bytes
  (app) or 4 (PC), and what it counts are [UNVERIFIED]. Manual screenshots
  show "Offline data bytes: 22 bytes" (B33-UM p.27/22-28/23) and "48 bytes"
  (CM2100-UM p.24/21) [KNOWN].
- **The challenge** (§3.4) is the other FFF1 write.

OWON's PC source also holds a comment with a long write of
`D2 70 40 14 0B 07 00…` (10 more `00`) to handle 0x0025
(`kernal/Command.java:55-59`) [VENDOR, PC]; what command it is, is
[UNVERIFIED].

### 7.3 Key sets per model

The keys each program offers [VENDOR]:

| Code | App key list (MD line) | App keys (code) | PC panel |
|---|---|---|---|
| 18, 20 | `_owSeriesKeys` (5989) | Select 1, Range 2, Hold/Light 3, △/Hz/Duty 5 | OW: Select, Range, Rel/Hz/Duty, Hold |
| 33 | `_b33Keys` (5606) | Select 1, Range 2, Hold 3, Rel 4, Hz/Duty 5 | BT: all six of §7.1 |
| 35, 41 | `_b35b41Keys` (5153) | Select 1, Range 2, Hold/Light 3, Rel 4, Hz/Duty 5, Max/Min 6 | BT: all six |
| 21 | `_cm2100Keys` (4910) | Select 1, Hold/Light 3, Zero 4 (no Range) | — |
| 55 | — | — | OW |

The meters' own keys (§9.1) [KNOWN]: B33 Select, Range, Hz/Duty, Hold, ☀ and
△/ᛒ, with no Max/Min key — yet the PC's BT panel and a B33-UM screenshot
(p.25/20) offer Max/Min; B35 and B41T Select, Range, Hz/Duty, Max/Min, ☀/H
and △/ᛒ; OW16 and OW18 Select, Range, ☀/H and Hz/Duty△/ᛒ; CM2100 ZERO/ᛒ,
HOLD/☀ and SELECT. The CM2100's SELECT held 2 s toggles VFC mode on AC
(CM2100-UM p.9/6-10/7) [KNOWN].

## 8. Offline records

"+" models (B33, B35, B41T) and the OW16, OW18 and CM2100 log readings in
the meter while disconnected and are read back over Bluetooth (B35-UM p.1,
p.30/25-32/27; OW16-UM p.28/23-36/31; CM2100-UM p.16/13-24/21) [KNOWN].

### 8.1 Start, stop, read

Start: `*DATe` (app only), then `*RECOrd,` (§7.2). On reconnect while
recording the app offers "Stop recording" (`*STOP`) or "Continue and
disconnect" (B35-UM p.30/25-32/27) [KNOWN]; FFF2 byte 5 is `01` while
recording (§4). After a B33 recording ends the meter's Bluetooth turns off
and must be switched on again to read back (B33-UM p.28/23) [KNOWN]. To read:
optionally `*READlen?`, then `*READ1?` or `*READ?`; the dump arrives as FFF4
notifications (BBM:394-720; BC:155-161, :279-295) [VENDOR, both].

### 8.2 Dump framing

| Item | App (OFF:543-958) | PC (ODM:63-148) |
|---|---|---|
| Start | 20 consecutive `FF` bytes, counted across notifications | 10 words `FF FF`, counted up to exactly 10 |
| End | the next run of 20 `FF`, trimmed | 10 more `FF FF` words, counted down to 0 |
| Payload | every byte between | every non-`FFFF` 16-bit LE word between |

So the dump is 20 bytes of `FF`, the payload, and 20 bytes of `FF`
[INFERRED, both programs agreeing in bytes]. Neither escapes `FF` in the
payload: in the app a run of 20 `FF` ends the dump, and a longer leading run
leaks its extra `FF`s into the header; in the PC any `FFFF` word counts as a
mark, contiguous or not [VENDOR, both]. The app gives up if no FFF4 data
comes within 5 s (BBM:661-700) [VENDOR, app].

### 8.3 Payload

| Offset | Size | Field | When |
|---|---|---|---|
| 0 | 1 | century, binary (the app reads 0 as 20) | firmware ≥ 12 (`*READ1?`) |
| 1-6 | 6 | YY MM DD hh mm ss of the start, binary | firmware ≥ 12 |
| 7 | 1 | unused, skipped by both | firmware ≥ 12 |
| 8-11 | 4 | interval, u32 LE, seconds | firmware ≥ 12 |
| 12-15 (0-3 before 12) | 4 | u32 LE: record count (app), "length", logged and unused (PC) | always |
| then | 2 each | words | always |

App PE:4482-5107 (firmware ≥ 12), PE:5139-5286 (older); PC ODM:167-209,
:470-500 [VENDOR, both]. Before firmware 12 the meter sends no start time or
interval: the app uses the configuration it saved at `*RECOrd,` (model 33
only) or "now" and 1 s (BBM:924-1001) [VENDOR, app].

**Words** [VENDOR, both: R2W:687-1017, OFF:302-351; ODM:211-394]:
- `(byte1 & 0xFC) == 0xF0`, i.e. bits 10-15 = `111100`: a function/range
  word (§6.1 layout), which sets the range for the words after it.
- Any other word: a reading (§6.5 layout), under the last range.
- Series 55 (PC): the mask is `0xF8`, bits 11-15 = `11110`, bit 10 being its
  sign; its offline reading is read as a signed 16-bit value (ODM:318,
  :324-326), unlike its live reading [VENDOR, PC].
- No status word is stored [VENDOR, PC: the mode is the literal "FLM",
  ODM:58, :378].

The marker leaves function bits 8-9 free, and a reading word matches it only
with the sign set and a magnitude of 0x7000-0x73FF (28672-29695), above
every model's count (series 55: 0xF000-0xF7FF; its counts are unknown)
[INFERRED].

**Timing.** Readings are `interval` seconds apart (B35-UM p.40/35) [KNOWN],
as both programs date them [VENDOR, both]. Whether a range word takes a time
slot is [UNVERIFIED]: the app counts only reading words, invalid ones
included, and skips readings before the first range word (R2W:687-1017;
R10W:1514-1571); the PC counts every word after the length field
(ODM:200-205, :383-385) but writes no time to its file (ODM:431-433)
[VENDOR, both].

## 9. Meter behaviour (manuals)

### 9.1 Per model

| | B33(T)(+) | B35(T)(+) | B41T(+) | OW16B | OW18B | OW18E | CM2100B |
|---|---|---|---|---|---|---|---|
| Counts | 3999 | 6000 | 22000 | 5999 | 5999 | 19999 | 19999 |
| Bluetooth on | hold △/ᛒ | hold △/ᛒ | hold △/ᛒ | hold Hz/Duty△/ᛒ | hold Hz/Duty△/ᛒ | hold Hz/Duty△/ᛒ | hold ZERO/ᛒ about 2 s |
| Bluetooth off | hold △/ᛒ; after an offline record ends | not stated | not stated | not stated | not stated | not stated | hold ZERO/ᛒ about 2 s |
| Bluetooth idle-off | 10 min, two beeps | 10 min, two beeps | 10 min, two beeps | 10 min, two beeps | 10 min, two beeps | 10 min, two beeps | 5 min |
| Auto power-off | 15 min | 15 min | 30 min | 30 min | 30 min | 30 min | about 15 min |
| Bluetooth suspends auto power-off | yes | yes | yes | yes | yes | yes | not stated |
| Disable auto power-off | hold Select at power-on | not stated | hold Range, Hz/Duty, Max/Min or △/ᛒ at power-on | not stated | not stated | not stated | hold SELECT at power-on, until the next power cycle |

All [KNOWN]: counts B33-UM p.50/45, B35-UM p.54/49, OW16-UM p.50/45, OW18-UM
p.48/43, CM2100-UM p.33/30 (the panels print "6000 Counts" for the OW18B,
OW18-UM p.12/7, and "20000 Counts" for the CM2100, CM2100-UM p.9/6);
Bluetooth on B33-UM p.23/18, B35-UM p.25/20, OW16-UM p.24/19, OW18-UM
p.23/18, CM2100-UM p.16/13; off B33-UM p.28/23, p.35/30,
p.47/42, CM2100-UM p.9/6; idle-off B33-UM p.21/16-22/17, B35-UM
p.24/19-25/20, OW16-UM p.22/17, OW18-UM p.22/17-23/18, CM2100-UM p.16/13;
auto power-off B33-UM p.11/6, B35-UM p.11/6, OW16-UM p.11/6, OW18-UM p.11/6,
CM2100-UM p.14/11-15/12. That a long press of △/ᛒ toggles Bluetooth on the
B33 is [INFERRED]: the same press brings the ᛒ icon back and makes it
disappear (B33-UM p.28/23, p.35/30). The OW18 has a flashlight on the ☀/H
long press (OW18-UM p.11/6) [KNOWN].

### 9.2 Functions per model

From the dials and SELECT cycles [KNOWN]; the function codes are §6.2's,
matched by name [INFERRED].

| Function (code) | B33 | B35 | B41T | OW16B | OW18B, OW18E | CM2100B |
|---|---|---|---|---|---|---|
| V DC, AC (0, 1) | yes | yes, and a mV position | yes, and a mV position | yes | yes; a mV position on units without hFE | yes |
| A DC, AC (2, 3) | yes | yes | yes | yes | yes | yes (2A, 20A, 100A positions) |
| Ω, diode, continuity (4, 10, 11) | yes | yes | yes | yes | yes | yes |
| Capacitance (5) | yes | yes | yes | yes | yes | yes |
| Hz, duty (6, 7) | yes | yes | yes | yes | yes | yes |
| ℃ (8) | yes | yes | yes | yes | yes | no |
| ℉ (9) | no | yes | yes | yes | yes | no |
| hFE (12) | no | yes | no | on units with it, in place of µA | on units with it | no |
| NCV (13) | no | no | no | yes | yes | yes |

Cites: B33-UM p.12/7-13/8; B35-UM p.13/8-18/13; OW16-UM p.13/8; OW18-UM
p.12/7-14/9; CM2100-UM p.9/6. Also [KNOWN]: on AC V and AC A the Hz/Duty key
cycles frequency, duty and back (B35-UM p.21/16; OW18-UM p.22/17); the
CM2100 has a VFC mode on AC (CM2100-UM p.9/6-10/7) and ZERO on DC A
(p.9/6); a ℃/℉ annunciator but no temperature function (CM2100-UM p.10/7).
Which OW16 and OW18 units carry hFE is not stated per model letter (OW16-UM
p.13/8; OW18-UM p.14/9).

### 9.3 Display words

"OL" on every model (B33-UM p.15/10; B35-UM p.15/10; OW16-UM p.15/10;
OW18-UM p.15/10; CM2100-UM p.10/7); "EF" and one to four dashes for NCV on
the CM2100 (§6.7) [KNOWN]. Other display words (LEAd, Err, ----): none found
in the eight manuals, 2026-10-01. Below 2.2 V the CM2100 shows only the
battery symbol and "cannot work" (CM2100-UM p.15/12) [KNOWN]. With no
reading OWON's app shows "------" (`device_control_page.dart:1320-1323`)
[VENDOR, app].

## 10. The 15-byte frame (OWON's app only)

OWON's app decodes a second frame for the meters below; nothing in the PC
source, the manuals or the product pages covers it, so every fact in this
section is [VENDOR, app] from one source, unless tagged.

### 10.1 Models

The app picks the parser by the model's protocol type, 0 for the 6-byte frame
and 1 for this one (BM:274-300, :1347-1409); MD:21-744 [VENDOR, app].

| Code | App name (MD line) | Digits | Counts | Key list (MD line) | `#TIMEsync` |
|---|---|---|---|---|---|
| 101 | CMS101 (`:338`) | 5 (4 for DC A, AC A, duty, capacitance) | 20000 | `_cms101Keys` (4385) | no |
| 61 | CMS061 (`:448`) | as CMS101 | 20000 | `_cms101Keys` | no |
| 91 | VC915 (`:479`) | 5 | 20000 | `_c91Keys` (3502) | yes |
| 92 | VC925 (`:511`) | 5 | 20000 | `_c92Keys` (2909) | yes |
| 83 | VC831 (`:543`) | 4 | 6000 | `_c831And851Keys` (2454) | no |
| 85 | VC851 (`:570`) | 4 | 6000 | `_c831And851Keys` | no |
| 65 | OW65 (`:602`) | 4 | 6000 | `_Ow65Keys` (2069) | no |
| 87 | VC871 (`:634`) | 5 | 60000 | `_c871AndOw67Keys` (1474) | yes |
| 67 | OW67 (`:661`) | 5 | 60000 | `_c871AndOw67Keys` | yes |
| 89 | VC891 (`:693`) | 5 | 60000 | `_c891AndOw69Keys` (745) | yes |
| 69 | OW69 (`:720`) | 5 | 60000 | `_c891AndOw69Keys` | yes |

Counts from `baseCounts` (MD:6739-6793). Brand does not gate which codes
connect: one model table serves every build of the app [VENDOR, app].

### 10.2 Framing

Each notification is cut into 15-byte chunks and a shorter remainder dropped
(BM:367-399). A chunk whose byte 14 is `FF` is skipped (BM:399-405); that
the byte tested is the last is [INFERRED from the listing selector read as
`last`]. Whether real frames end in `FF` filler is [UNVERIFIED].

| Bytes | Field |
|---|---|
| 0-2 | main function/range, u24 LE (G24, §10.3) |
| 3-5 | main reading, u24 LE (V24, §10.4) |
| 6-8 | sub-display function/range, G24; read only if main G24 bit 12 is set |
| 9-11 | sub-display reading, V24; same condition |
| 12-14 | status, u24 LE: §6.6's bits 0-15 and §10.6 |

R10W:12-228, :96-160, :175-188, :229-268. No header, length or checksum.

### 10.3 Function/range word, G24

| Bits | Meaning |
|---|---|
| 0-2 | decimal-point code (§6.4) |
| 3-5 | prefix (§6.3) |
| 6-10 | function (§6.2 and §10.5) |
| 11 | stored, no consumer found; meaning [UNVERIFIED] |
| 12 | sub-display present |
| 13-23 | not read |

R10W:948-1071.

### 10.4 Reading word, V24

| Bits | Meaning |
|---|---|
| 0-18 | magnitude (`& 0x7FFFF`) |
| 19 | not read |
| 20-22 | status code, below |
| 23 | sign (the byte taken with the selector read as `last`, [INFERRED]) |

| Status | App |
|---|---|
| 0 | a number |
| 1 | "OL", value = full-scale counts |
| 2 | "UL", value 0 |
| 3 | "HI", value = full-scale counts |
| 4 | "LO", value 0 |
| 5 | a number, with a flag whose meaning is not found [UNVERIFIED] |
| 6, 7 | no reading |

R10W:270-947. Further rules (R10W:532-940): a count longer than the model's
digits is no reading; NCV (13) uses §6.7's texts; Time (19) is seconds shown
"H:MM:SS"; Motor (24) uses the texts 0 "- - -", 1 "- - -", 2 "1-2-3",
3 "3-2-1" (PE:1318-1393; other values throw); decimal codes 6 and 7 give
"UL" and "OL".

### 10.5 Function codes 16-31

| Code | Label, unit |
|---|---|
| 16 | Power Factor, none |
| 17 | 4~20mA, % |
| 18 | Power, Ah |
| 19 | Time, none |
| 20 | Power, Wh |
| 21 | Power, V |
| 22 | Power, A |
| 23 | AC+DC, V |
| 24 | Motor, none |
| 25 | Solar, W/m² |
| 26 | Angle, ° |
| 27 | Compass, ° |
| 28 | DC_HV, V |
| 29 | AC_HV, V |
| 30, 31 | none: no reading |

PE:122-869. Codes 16, 17, 19 and 24 also take no prefix (PE:871-947). With
main function 23, a DC V or AC V sub-display is labelled "DC" or "AC"
(PE:5952-6085).

### 10.6 Status bits 16-18

| Bit | Mask | App |
|---|---|---|
| 16 | 0x10000 | Err_port ("Err") |
| 17 | 0x20000 | Inrush |
| 18 | 0x40000 | OSC |

PE:3534-3917.

### 10.7 VC871 (code 87)

The only model branch in the parser (R10W:1077-1404): digits are always 5
and counts 60000, and when the status holds REL, MAX or MIN the sub-display
takes the main display's function, keeping its own prefix, decimal point and
flags. OW67, which shares the VC871's key list, gets the ordinary parser.

### 10.8 Commands

- `#TIMEsync` (9 bytes, raw string) + CC YY MM DD hh mm ss, 16 bytes, on
  connect for codes 87, 89, 91, 92, 67 and 69 (PE:2938-3369; BBM:1211-1301).
- Key presses as §7.1, with codes up to `11` (VC915: Compare `0F`, AC/DC
  `10`, Motor `11`; 4~20mA `0A`, Display `0C`, LPF 7, Peak 8, Inrush `0D` on
  the models whose lists carry them), and Hold/Light's long press sent as
  `09 01` on the CMS101/061, VC831/851, OW65, VC871/OW67 and VC891/OW69
  (MD:745-4910 lists).

### 10.9 Offline words

3-byte words: byte 2 = `F0` marks a G24 function/range word, any other word
is a V24 reading (R10W:1406-1739). G24 never uses bits 16-23, so the marker
fits [INFERRED].

## 11. An earlier format: 14-byte ASCII (PC source, commented out)

The PC source holds a fully commented-out parser for 14-byte frames
(`ONE_DATA_LENGTH = 14`, MC:211; parser MC:264-678): a sign byte `+` or `-`,
four digit bytes, a space, a decimal-point byte (`1`, `2` or `4` tested),
four status bytes sb1-sb4, a bar-graph byte, and CR LF (MC:423, :432)
[VENDOR, PC]. The status bit constants it uses are defined nowhere in the
source, so the flags cannot be recovered from it. A second commented-out
parser reads B33 LCD segments (MC:706-1361) [VENDOR, PC].

`isUsingB35ChipProtocol()` returns true for series 35 without offline record,
with the comment "目前B35仍用其芯片协议,除此之外B33和其他带离线记录功能的都用OS通用协议"
(B35 still uses its chip protocol; apart from it, B33 and the others with
offline record use the "OS" common protocol) (BSI:134-137); nothing calls it [VENDOR, PC]. The B35
product page lists a "Bluetooth 2.0 version - supports mobile device with
Android 4.0 or above OS" beside the "Bluetooth 4.0 version" (page B35)
[KNOWN]. So an older B35 without "+" may send this format [INFERRED];
whether any meter does is [UNVERIFIED].

## 12. Worked examples

### 12.1 OWON's live frame

From a PC source comment: a notification on handle 0x002E, the BT layout's
FFF4 (`kernal/Event.java:84-85`) [VENDOR, PC, the bytes]:

```
19 F0 04 00 BD 09
```

| Bytes | Word | Decode [INFERRED, from §6] |
|---|---|---|
| `19 F0` | 0xF019 | dp 1, prefix 3 (m), function 0 (DC V); bits 10-15 `111100` |
| `04 00` | 0x0004 | AUTO |
| `BD 09` | 0x09BD | positive, 2493 |

2493 ÷ 10 = 249.3 mV DC, autoranging; a 4-digit display shows "249.3". The
handle puts it on a B33, B35, B41 or any other code outside 18, 20 and 55
[INFERRED from §2's handle sets].

### 12.2 OWON's device information

From a PC source comment: a read-by-type response on handle 0x0017, the OW
layout's FFF2, length 0x12 (`kernal/Event.java:369-375`) [VENDOR, PC, the
bytes]:

```
12 63 04 00 09 00 00 00 00 00 00 00 00 00 00 00
```

Decode [INFERRED, from §4]: code 0x12 = 18 (app OW18B, PC `OW18_16`),
battery 0x63 = 99 % (PC), firmware 4.0.9 (≥ 12, so `*READ1?` and the dated
header), byte 5 `00`: offline record supported, not recording. Bytes 6-15
are `00`, read by neither program.

### 12.3 OWON's key press

From a PC source comment: a write of `01 01` to handle 0x002B, the BT
layout's FFF3 (`kernal/Command.java:27-32`) [VENDOR, PC, the bytes]. Decode
[INFERRED, from §7.1]: Select, short press, on a B-series meter.

### 12.4 Constructed

Built from the tables, not captured or taken from a vendor source
[INFERRED]. Bits 10-15 of the function word copy §12.1.

A negative DC V reading, `22 F0 04 00 D2 84`:

| Bytes | Decode |
|---|---|
| `22 F0` | 0xF022: dp 2, prefix 4 (none), function 0 (DC V) |
| `04 00` | AUTO |
| `D2 84` | 0x84D2: sign set, magnitude 0x04D2 = 1234 |

−12.34 V DC.

An offline dump on firmware ≥ 12: three readings at 1 s from 2026-10-01
14:05:30, in the range of §12.1, spaced for reading:

```
FF ×20
14 1A 0A 01 0E 05 1E 00   01 00 00 00   03 00 00 00
19 F0   BD 09   C2 09   B8 09
FF ×20
```

Century 20, 26-10-01 14:05:30, byte 7 unused, interval 1 s, count 3; a
range word (`F0 & FC = F0`), then 249.3, 249.8 and 248.8 mV DC.

---

## Implementation Notes

What the wire requires of any decoder, from the vendor sources:

- The model is FFF2 byte 0, read after connecting; the advertised name is
  "BDM" or one a user chose.
- Live frames arrive as FFF4 notifications with no request; the only write
  either vendor program makes before them is the app's challenge (§3.4).
- A frame is 6 bytes, three little-endian 16-bit words, with no framing;
  frame boundaries are only positional.
- The reading is sign-and-magnitude: bit 15 sign, bits 0-14 a binary count
  (series 55: gear bit 10 and 16 bits).
- The value is count ÷ 10^dp × prefix in the function's unit; dp codes 6 and
  7 are UL and OL, and the count then carries no agreed meaning.
- Status bits 0-5 are HOLD, REL, AUTO, Bat, MIN, MAX; above that the two
  vendor sources disagree.
- Function 13 is NCV on the OW16, OW18 and CM2100, whose NCV levels 0-4 are
  "EF" and one to four dashes; on B-series meters it is open.
- An offline dump sits between two runs of 20 `FF` bytes; inside it a word
  whose high byte masked with `FC` is `F0` sets the range for the readings
  after it.

## 13. Open questions — [UNVERIFIED]

The open checks are in [verification.md](verification.md).

---

## 14. Cross-reference with community sources [COMMUNITY]

Not done. As of 2026-10-01 the clean-room boundary stands: no community
source has been opened for this family (`reverse-engineering-approach.md`).
This section will hold the comparison once the boundary is opened.
