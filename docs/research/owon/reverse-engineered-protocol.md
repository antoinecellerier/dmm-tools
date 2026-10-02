# OWON Bluetooth multimeters (B33, B35T+, B41T+, OW16B, OW18B, OW18E, CM2100B, CMS101, CMS061, OW65B, OW67B, OW69B; Voltcraft VC871, VC891, VC915, VC925 PV): Reverse-Engineered Protocol Specification

What OWON's Bluetooth LE multimeters send and accept. After a connection the
meter notifies 6-byte frames on characteristic FFF4: a 16-bit function/range
word (function, SI prefix, decimal-point position), a 16-bit status word and
a 16-bit sign-and-magnitude count, all little-endian, with no header, length
or checksum. A one-off read of FFF2 gives the model code and firmware version;
16-byte commands (record, read-back, rename and a challenge/response) go to
FFF1, and 2-byte key presses to FFF3. The CMS101/061, OW65B/67B/69B and
Voltcraft VC871/891/915/925 PV send a 15-byte frame with a sub-display
instead (§10), decoded by OWON's app and by Voltcraft's, which is the same
code (§10.10). Both frames are implemented in `protocol/owon/`,
experimental. No OWON or Voltcraft meter has been on
our bench: every fact in §1-12 comes from OWON's and Voltcraft's Android
apps, OWON's PC software source, and OWON's and Voltcraft's manuals and
product pages. §14 compares them with community projects and their captures.
The approach doc beside it records the sources, the method and the
clean-room boundary.

Based on:
- OWON's Android app iMeter 1.2.4 (`com.owon.imeter`), a Flutter build,
  decompiled with blutter
- OWON's PC software pcMultimeter 1.4.5 (Windows 7/8 package "multimeterBLE",
  plug-in 1.4.4), whose Java source OWON ships in the package as an Eclipse
  source bundle; it reaches the meter through OWON's TI CC2540 USB dongle
- The eight OWON manuals (33 series, 35 series & B41T, OW16, OW18, CM2100;
  user manuals and quick guides), read from the rendered pages
- OWON's product pages on owon.com.hk and the datasheets they link
- For the 15-byte meters (§9.4, §10): Voltcraft's Android app "Voltcraft
  VC800 VC900 Series" 1.2.5 (`com.voltcraft.series800`), the same Flutter
  code as iMeter, decompiled with blutter; OWON's manuals for the CMS101,
  CMS061, OW65, OW67 and OW69; Conrad's manuals for the Voltcraft VC831,
  VC851, VC871, VC891, VC915 and VC925 PV, its datasheets for the first
  four, and its manuals for Voltcraft's app

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
  V1.1.3), **OW18-QG** (2023.11 V1.1.1), **CM2100-UM** (Jan. 2026 V1.0.3);
  for the 15-byte meters **CMS101-UM** (`CMS_User_Manual.pdf`, Sep. 2025
  V1.0.4), **CMS061-UM** (Sep. 2025 V1.0.4), **OW65-UM** (Feb. 2025
  V1.0.0), **OW67-UM** (Dec. 2025 V1.0.2), **OW69-UM** (Sep. 2025 V1.0.2)
- **V: `file:lines`** — the blutter listing of Voltcraft's app under
  `references/owon/voltcraft-app/blutter/asm/`, the same file names as the
  app's
- **Voltcraft manuals**, `<key> p.N` = PDF page, under
  `references/owon/voltcraft/` (Conrad item numbers; English sections):
  **VC871-UM** (`002576867ML00`, p.56-109, v12_0826), **VC891-UM**
  (`002576866ML00`, p.51-98, v9_0826), **VC915-UM** (`003072347ML00`,
  p.55-105, v4_00626), **VC925-UM** (`003072348ML00`, p.57-109, V2_0826),
  **VC831-UM** (`002576864ML00`, p.47-89), **VC851-UM** (`002576865ML00`,
  p.46-88); **VC-APP**, Voltcraft's app manual (`002576866ML04`,
  V1_1222, © 2022); **VAPP**, the later app manual bundled in Voltcraft's
  app 1.2.5 (`2576866_67_V4_0526_02_jh_m_App_EN`, © 2023)
- **page `<model>`** — the product page saved under `references/owon/web/`
  (B35 = `products_owon_3_5-6_…`, B41T = `products_owon_4_1-2_…_b41t.html`)

Byte and bit numbering: byte 0 is the first byte of a frame or value; bit 0
is the least significant bit of a little-endian word.

Confidence levels:
- **[KNOWN]** — stated in an OWON or Voltcraft manual or on OWON's site,
  cited by page
- **[VENDOR, app]**, **[VENDOR, PC]**, **[VENDOR, both]** — read from OWON's
  app (or Voltcraft's, cited `V:`), OWON's PC source, or both independently,
  cited by file and lines
- **[INFERRED]** — deduction from the above, reason given
- **[UNVERIFIED]** — no source confirms it, or the sources disagree; needs a
  real meter ([verification.md](verification.md) lists the checks)
- **[HARDWARE]** — seen on a real meter: none yet
- **[COMMUNITY]** — from a community source, §14 only

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
| CMS | CMS101, CMS061 (a "BLE" label on the panel) | none named | CMS101-UM p.12/7, p.33/28; CMS061-UM p.12/7 |
| OW65, OW67, OW69 | OW65B, OW67B, OW69B: "Bluetooth √ (Only for B model)" | the models without B | OW65-UM p.48/43; OW67-UM p.33/28, p.49/44; OW69-UM p.30/25, p.51/46 |
| Voltcraft VC800 | VC871, VC891: "Bluetooth® LE 4.0" | VC831, VC851 (below) | VC871-UM p.68, p.100; VC891-UM p.63, p.90 |
| Voltcraft VC900 | VC915, VC925 PV: "Bluetooth LE 5.0" (on the VC925 in a §23.3 that follows the adaptor's §23.2, not assigned to either) | — | VC915-UM p.99; VC925-UM p.101 |
| Voltcraft VC831, VC851 | none: no Bluetooth, app or radio-module text in either manual or datasheet, and no BLE key or icon on the panel | VC831, VC851 | VC831-UM p.46-89, p.52; VC851-UM p.46-88, p.50 |

"T" is true RMS and "+" the offline record function, both optional (B35-UM
p.1; B33-UM p.1) [KNOWN]. The VC831/VC851 row is a search of those sources,
2026-10-02.

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
  (MD:6739-6793; `multimeter_fs.dart` 1095-1101) [VENDOR, app]; its product
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
| Advertised name | "BDM" by default: the app's device list shows it ("select BDM", "Click 'BDM' in the device list to pair") | B35-UM p.26/21; OW18-UM p.24/19; OW16-UM p.25/20; B33-UM p.24/19; CM2100-UM p.17/14-18/15 | [KNOWN] (cross-reference §14) |
| Name, 15-byte meters | "BDM" in the app's list in the CMS and OW6x manuals, and on every device card and controls screen of VAPP, whose controls screen shows the VC925's key list (§10.8). Voltcraft's meter manuals say to select "VC871", "VC891" or "VCxxx"; VC-APP's screenshots show "VC8xx_1" and "VC8xx_2" | CMS101-UM p.34/29; OW65-UM p.28/23-29/24; OW67-UM p.33/28-34/29; OW69-UM p.30/25-31/26; VAPP p.6-8, p.11, p.14-15, p.20, p.22-23; VC871-UM p.92; VC891-UM p.82; VC915-UM p.88; VC925-UM p.94; VC-APP p.12, p.15 | [KNOWN]; which name a Voltcraft meter advertises out of the box is [UNVERIFIED] (cross-reference §14) |
| Rename | the app can rename the meter; the name "will be memorized in the device"; "Only digits, letters and underscore can be entered". Voltcraft: "The new name is permanently stored on the multimeter" | B35-UM p.28/23; VAPP p.7 | [KNOWN]; the command is §7.2 |
| Renamed name advertised | the app shows only advertised names in its list, and the manual says the name is stored in the meter | — | [INFERRED]; [UNVERIFIED] on a meter (cross-reference §14) |
| Name filter | none in either program. The app shows any peer whose name, stripped to `[0-9a-zA-Z_]`, is not empty, de-duplicated by id (`device_add_view_model.dart:202-563`; `built_in_ble_device.dart:36-78`); the PC lists every advertiser by address, its name field left `""` with the comment `// "BDM"` (`kernal/Event.java:178-191`, `:188`) | app, PC | [VENDOR, both] |
| Other advertising data | read by neither: the app uses only the device id and name; the PC ignores the event type, address type and advertising data (`kernal/Event.java:159-203`) | app, PC | [VENDOR, both] |
| Service filter | the app's optional "Filter device" switch scans with service `0000fff0-0000-1000-8000-00805f9b34fb` (`ble_device_scanner.dart:30`, `:130-145`); it is off by default in OWON's build (`device_add_view_model.dart:63-83`, `:181-185`; `main.dart:13-21`) and on in Voltcraft's (V:`device_add_view_model.dart:63-80`, `:181-185`). The manuals: "Filter device" hides incompatible meters (B35-UM p.25/20-26/21); VAPP tells the user to switch it on and shows "BDM" found with it on (p.6-7) | app, manual | [VENDOR, app], [KNOWN]; that the meter advertises the FFF0 UUID [INFERRED from the filter and VAPP's screenshot]; [UNVERIFIED] (cross-reference §14, D1) |
| Address type | the PC connects with the peer address type hard-coded to public (`kernal/Command.java:160-176`) | PC | [VENDOR, PC]; that the meters use a public address [INFERRED] (cross-reference §14) |
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
`0000fff1…fff4-0000-1000-8000-00805f9b34fb` are [INFERRED] (cross-reference
§14).

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
different GATT tables [INFERRED] (per model: cross-reference §14); where
FFF4's CCCD sits is not shown [UNVERIFIED].

**Notify, CCCD, write types.**
- The PC handles ATT notifications (event 0x051B) and no indications
  (`kernal/Event.java:11-23`, `:77-109`), so FFF4 notifies [INFERRED]. The
  app subscribes with the plugin's plain call (`ble_adapter.dart:548`)
  [VENDOR, app].
- The PC never writes a CCCD (no descriptor write in the source) and parses
  every notification on the link whatever its handle (BC:90-96, :148-166)
  [VENDOR, PC]. Whether the meter notifies with its CCCD unset is
  [UNVERIFIED] (cross-reference §14).
- The app writes every characteristic with response (`ble_adapter.dart:392`);
  the PC writes FFF3 with a Write Request and FFF1 with a long write (Prepare
  + Execute, TI's GATT_WriteLongCharValue, `kernal/Command.java:25-78`)
  [VENDOR, both]. So FFF1 takes both forms for a 16-byte value [INFERRED].
  Write without response: [UNVERIFIED] (cross-reference §14).
- Neither program requests an MTU [VENDOR, both]; every command is at most 16
  bytes and fits the default ATT MTU of 23 [INFERRED].

Whether the meter advertises while connected, and how many centrals it
accepts: not stated in the manuals [UNVERIFIED] (cross-reference §14). The app
and the PC can each hold several meters (B35-UM p.25/20; PC "up to three",
B35-UM p.49/44) [KNOWN].

## 3. Bring-up

### 3.1 What the meter needs

No start, poll or keep-alive command: none found in the app or the PC source,
2026-10-01. The app subscribes to FFF4 and writes nothing periodically; the
PC writes nothing at all before readings arrive [VENDOR, both]. So a
connection, with FFF4 notifications enabled as a standard host does, is all
the stream needs [INFERRED] (cross-reference §14); the challenge is §3.4.
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
is answered is [UNVERIFIED] (cross-reference §14).

## 4. Device information, FFF2

| Byte | App (PE:7209-7424; BM:41-163) | PC (BC:206-275; BSI:85-106) |
|---|---|---|
| 0 | model code (§1) | series ID, read signed |
| 1 | read, discarded | battery, percent; 0-100 valid, anything else "not supported"; logged only |
| 2-4 | firmware version, shown "b2.b3.b4" in decimal | firmware a.b.c, v = a·100 + b·10 + c |
| 5 | `1` = recording in progress | `FF` offline record not supported; `00` supported; `01` supported and recording |

[VENDOR] per column. The PC tolerates a shorter value, reading each field only
if present (BC:214, 227, 240, 248). OWON's example is 16 bytes (§12.2)
(cross-reference §14).

**Firmware gates** [VENDOR, both]: the PC enables rename from v ≥ 11
("from 0.1.1") and the dated offline header from v ≥ 12 ("from 0.1.2")
(BSI:85-91); the app picks the old offline read-back when the version with
its dots removed, parsed as an integer, is below 12 (BM:176-226). Both read
"0.1.2" as 12 and "4.0.9" as 409 [INFERRED from the arithmetic]. Which
models run a version below 12 is [UNVERIFIED] (cross-reference §14).

## 5. Live frame: framing

| Rule | Source | Tag |
|---|---|---|
| A frame is 6 bytes: three 16-bit little-endian words | MC:212 (`COMMON_ONE_DATA_LENGTH = 6`), :1366-1388; BM:1155-1233; `bits_utils.dart:6-79` | [VENDOR, both] |
| No start byte, length, sequence number or checksum | MC:1366-1733; R2W:10-147 | [VENDOR, both]; alignment is positional, so a lost byte shifts every later frame [INFERRED] |
| Frames per notification: the PC joins notifications and carries a remainder over; the app cuts each notification into 6-byte chunks and drops a shorter remainder | MC:1374-1388, :1720-1730; BM:1155-1233 | [VENDOR, both]; whether a notification ever holds anything but whole frames is [UNVERIFIED] (cross-reference §14) |
| Rate: the manuals give 3 samples a second (B33-UM p.50/45; B35-UM p.54/49; OW16 and OW18 p.48/43; CM2100-UM "Numerical Value Conversion Rate", p.33/30) and 2 for the B41T ("Sample rate for digital data", B35-UM p.54/49); the B41T page gives "shift rate(BLE link) 2 times/s" and "Update rate 3 times/s" | manuals, page B41T | [KNOWN]; the notification rate is [UNVERIFIED] (cross-reference §14) |

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
every live frame carries it is [UNVERIFIED] (cross-reference §14).

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
  if anything, is open (cross-reference §14).
- **14-15**: the app's Power W and Power VA against the PC's nothing. No
  dial in these manuals has a power position; the B35 LCD carries an
  unexplained "RPM" segment and a lightning bolt, and its app screen the
  "POWER" code (B35-UM p.15/10, p.27/22) [KNOWN] (cross-reference §14).
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
functions is [UNVERIFIED]; 4 is [INFERRED] from the app forcing it
(cross-reference §14).

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
meaning is [UNVERIFIED] (cross-reference §14).

- **OL** is the meter's overload display: "the reading exceeds the display
  range" (B35-UM p.15/10), reversed diode leads (B35-UM p.20/15), a current
  range exceeded (OW18-UM p.21/16) [KNOWN]. **UL**: none found in the eight
  manuals, 2026-10-01; "underload" is [INFERRED from the pairing with OL].
- Under UL both programs keep the raw count. Under OL the PC keeps it, while
  the app replaces the value with 6000 (4-digit models) or 20000 (the
  others), testing the text for "OL" alone (MC:1594-1598; R2W:378-497)
  [VENDOR, both]. What the meter puts in the magnitude then is
  [UNVERIFIED] (cross-reference §14).
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
  [VENDOR, PC]. Whether a meter sends it is [UNVERIFIED] (cross-reference §14).
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
  in a 6-byte meter's frames are [UNVERIFIED] (cross-reference §14, D2).
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
display word (OW18-UM p.19/14-20/15) [KNOWN] (cross-reference §14).

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

[VENDOR] per column. Key 4 is the B-series △/ᛒ key and the CM2100's ZERO/ᛒ
key, key 5 the OW16/OW18 Hz/Duty△/ᛒ key (§9.1), and a long press of those keys
switches Bluetooth on (and on the B33 and CM2100 off) [INFERRED by name from
§9.1]. What a remote long press of them does is [UNVERIFIED] (cross-reference
§14).

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
  (app) or 4 (PC), and what it counts are [UNVERIFIED] (cross-reference §14).
  Manual screenshots show "Offline data bytes: 22 bytes" (B33-UM
  p.27/22-28/23) and "48 bytes" (CM2100-UM p.24/21) [KNOWN].
- **The challenge** (§3.4) is the other FFF1 write.

OWON's PC source also holds a comment with a long write of
`D2 70 40 14 0B 07 00…` (10 more `00`) to handle 0x0025
(`kernal/Command.java:55-59`) [VENDOR, PC]; what command it is, is
[UNVERIFIED].

### 7.3 Key sets per model

The keys each program offers [VENDOR] (cross-reference §14, D6):

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

So the dump is 20 bytes of `FF`, the payload, and 20 bytes of `FF` [INFERRED,
both programs agreeing in bytes] (cross-reference §14). Neither escapes `FF`
in the payload: in the app a run of 20 `FF` ends the dump, and a longer
leading run leaks its extra `FF`s into the header; in the PC any `FFFF` word
counts as a mark, contiguous or not [VENDOR, both]. The app gives up if no
FFF4 data comes within 5 s (BBM:661-700) [VENDOR, app].

### 8.3 Payload

| Offset | Size | Field | When |
|---|---|---|---|
| 0 | 1 | century, binary (the app reads 0 as 20) | firmware ≥ 12 (`*READ1?`) |
| 1-6 | 6 | YY MM DD hh mm ss of the start, binary | firmware ≥ 12 |
| 7 | 1 | unused, skipped by both | firmware ≥ 12 |
| 8-11 | 4 | interval, u32 LE, seconds | firmware ≥ 12 |
| 12-15 (0-3 before 12) | 4 | u32 LE: record count (app), "length", logged and unused (PC) (cross-reference §14, D7) | always |
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

The 15-byte meters' manuals [KNOWN]: "OL" on all; "EF" and one to four
dashes for NCV on the CMS (CMS101-UM p.25/20), "INRUSH", and "Trig",
"Ready", "Scan", "Stop" in its scope mode (p.30/25); "Loz", "REC", "COMP" and
"FAIL", "Lo" on the OW67/69, low-pass on the OW69 (OW67-UM p.14/9,
p.30/25-32/27; OW69-UM p.14/9-15/10); "Loz", "Lo", "COMP", "RECORD", "STOP", "SAVE" and
"LOG" on the VC915 (VC915-UM p.67-68); "Check inPut" for a
lead in the wrong socket on the VC871 (VC871-UM p.73); "HOT PANEL REMOVE
SENSOR", "INV" and "PV Direction Fail" in the VC925's PV mode (VC925-UM
p.70-71, p.90-91, p.93).

### 9.4 The 15-byte meters

OW67B = VC871 and OW69B = VC891 by their panels, keys, dials, menus and
spec tables [INFERRED from comparing the manuals]; the VC915 and VC925 PV
are a 20000-count IP65 design of their own.

| | CMS101, CMS061 | OW65B | OW67B, VC871 | OW69B, VC891 | VC915 | VC925 PV |
|---|---|---|---|---|---|---|
| Counts | not stated; ranges in 20000 steps | 6000 | 60000 | 60000 | 20000 | 20000 |
| Bluetooth on | hold Tab⇌ (BLE) | hold SETUP (BLE) | hold < (BLE), about 2 s on the VC871 | hold < (BLE), about 2 s on the VC891 | hold < (BLE) about 2 s | hold < (BLE) about 2 s |
| Bluetooth off | not stated | hold SETUP | not stated ("switchable" on the VC871) | not stated ("switchable" on the VC891) | not stated | not stated |
| Bluetooth idle-off | not stated | not stated | 5 min (OW67) | not stated | not stated | not stated |
| Auto power-off | about 10 min, can be cancelled; suspended while Bluetooth is on | 5/10/15/30 min or Always ON | as OW65 | — | 5/10/15/30 min or Always ON | 5/10/15/30 min or Always ON |
| Clock (Set Time, Set Date) | none found | none found | yes | yes | yes | yes |
| App | QR code only | QR code only | "Voltcraft VC800-Series" (VC871) | "Voltcraft VC800-Series" (VC891) | "Voltcraft VC800 VC900 series" | "Voltcraft VC800 VC900 series" |

"—" is not read for this spec. Cites: counts CMS101-UM p.35/30-36/31,
OW65-UM p.48/43, OW67-UM p.49/44, OW69-UM p.51/46, VC871-UM p.68, p.100,
VC891-UM p.63, p.90, VC915-UM p.66, p.98, VC925-UM p.100; Bluetooth on CMS101-UM p.33/28-34/29, OW65-UM
p.13/8, OW67-UM p.13/8, p.33/28, OW69-UM p.30/25, VC871-UM p.92, VC891-UM
p.82, VC915-UM p.88, VC925-UM p.94; off OW65-UM p.13/8, VC871-UM p.68,
VC891-UM p.63; idle-off OW67-UM p.33/28 ("after 5 minutes of inactivity";
left out of its quick guide); auto power-off CMS101-UM p.34/29, p.37/32,
OW65-UM p.17/12, OW67-UM p.17/12-18/13, VC871-UM p.89-90, p.100, VC915-UM
p.90, p.98, VC925-UM p.73, p.88, p.100; clocks OW67-UM p.11/6, OW69-UM
p.11/6, p.18/13, VC871-UM p.91, VC891-UM p.81, VC915-UM p.90, VC925-UM p.73; apps VC871-UM p.92, VC891-UM p.82,
VC915-UM p.88, VC925-UM p.94. Also [KNOWN]:

- **Bluetooth.** No manual gives a PIN or pairing step. OWON's say "More
  than one meters can be connected simultaneously" to the app (OW65-UM
  p.28/23); none says how many phones one meter takes. Voltcraft's app must
  be reconnected, with Bluetooth enabled again, after every power-on (VC-APP
  p.10). Over Bluetooth "The knob function cannot be controlled!" (VC871-UM
  p.92). OWON's manuals describe the offline record's read-back with the
  Android app only (OW65-UM p.35/30; OW67-UM p.40/35).
- **CMS scope mode.** The power key toggles DMM and oscilloscope; the scope
  is one channel, 5 MSa/s (CMS101-UM p.30/25-33/28, p.37/32). Whether
  Bluetooth streams in scope mode is not stated; the ᛒ icon is part of the
  scope's status bar (p.31/26).
- **VC925 PV adaptor.** The VC925 PV searches for and connects to its
  LX-925 irradiance adaptor over Bluetooth, "Default adaptor name: "PV adaptor VC92"", from
  the PV position's Find page (VC925-UM p.89-90, p.92). In VAPP, F2 (APPS) or
  F3 (PV92) chooses which link the meter takes (VAPP p.20-21). Whether
  the phone and adaptor links can run together is not stated.

**Functions** from the dials, SELECT cycles and menus; the codes are §6.2's
and §10.5's, matched by name [INFERRED]:

| Function (code) | CMS101/061 | OW65B | OW67B, VC871 | OW69B, VC891 | VC915 | VC925 PV |
|---|---|---|---|---|---|---|
| V DC, AC (0, 1) | yes | yes | yes | yes | yes | yes |
| A DC, AC (2, 3) | clamp | yes | yes | yes | yes | µA and mA only |
| Ω, diode, continuity (4, 10, 11) | yes | yes | yes | yes | yes | yes |
| Capacitance, Hz, duty (5, 6, 7) | yes | yes | yes | yes | yes | yes |
| ℃, ℉ (8, 9) | no | yes | yes | yes | yes | yes |
| NCV (13) | yes | no | no | no | no | no |
| Power W, VA, PF (14, 15, 16) | no | no | AC and DC power | no | no | no |
| 4~20 mA (17) | no | yes | yes | yes | yes | yes |
| USB power (18-22) | no | no | yes (mAh, Wh, V, A, a timer; cross-reference §14, D11) | no | no | no |
| AC+DC V (23) | no | no | no | yes | yes | no |
| Motor (24) | no | no | no | no | yes | no |
| Solar, angle, compass (25-27) | no | no | no | no | no | with the LX-925 |
| HV DC, AC (28, 29) | no | no | no | no | no | 2 kV DC, 1.5 kV AC |
| LoZ, low-pass (Lo) | no | LoZ | both | both | both | LoZ |
| Peak, Inrush | Inrush | "Peak (1ms) √" in the specification only | Peak | Peak | no | no |

Cites: CMS101-UM p.17/12-25/20; OW65-UM p.13/8-14/9, p.19/14-25/20, p.48/43;
OW67-UM p.13/8-14/9, p.26/21-28/23, p.32/27; OW69-UM p.13/8-14/9,
p.28/23-29/24; VC871-UM p.62, p.71, p.75-90; VC891-UM p.56, p.65, p.70;
VC915-UM p.60, p.72-79; VC925-UM p.68-72, p.75-93. The OW67's "Lo" softkey
is not explained in its manual; the VC871's is a 1 kHz low-pass (VC871-UM
p.71).

**Sub-display.** MAX/MIN shows the live value and REL the reference on
the OW65, OW67 and VC871, while the CMS's MaxMin row shows Min, Max and Avg
(CMS101-UM p.27/22); the frequency or duty beside AC V and Hz; W with VA, V, A, PF
or Hz on the power functions; VAC or VDC beside AC+DC (VC915-UM p.73); the
voltage in Motor (p.76); irradiance or temperature in PV mode (VC925-UM
p.69). Cites: OW65-UM p.26/21-27/22; OW67-UM p.14/9, p.27/22-32/27;
VC871-UM p.78, p.85-88.

**Keys on the meter.** The CMS has a power key (DMM/OSC), HOLD (held: "DCA
to zero"), Tab⇌ (menu pages), F1-F4, ▲▼, ◄► and A (CMS101-UM p.11/6,
p.13/8, p.26/21-28/23). The OW65 has F1 RANGE, F2 MAX/MIN, F3 REL, F4 HOLD,
SELECT, SETUP, a torch key and LoZ (OW65-UM p.13/8-14/9). The OW67/69 and
the Voltcraft meters have F1-F4 under on-screen labels (page 1 RANGE,
MAX/MIN, REL, HOLD), SELECT (SETUP above it), < (BLE), > (torch) and LoZ
(OW67-UM p.13/8, p.29/24-32/27; VC871-UM p.62; VC915-UM p.60); on the
Voltcraft meters RANGE, MAX/MIN and REL held (about 1 s on the VC871 and
VC915) return to auto or leave the function (VC871-UM p.88-91; VC915-UM
p.86-90; VC925-UM p.86-87).

## 10. The 15-byte frame

OWON's app decodes a second frame for the meters below, and Voltcraft's app
is the same code (§10.10); nothing in the PC source or the manuals covers
the wire, so every wire fact in this section is [VENDOR, app], one code base,
unless tagged. The meters' own behaviour is §9.4. VC871 captures:
cross-reference §14.

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
connect: one model table serves every build of the app, Voltcraft's
included (§10.10) [VENDOR, app]. The VC831 and VC851 have no Bluetooth
(§1), so codes 83 and 85 name no product found with a radio, 2026-10-02.

### 10.2 Framing

Each notification is cut into 15-byte chunks and a shorter remainder dropped
(BM:367-399); nothing is kept between notifications, so a frame split across
two is lost to the app (BM:357-405) [VENDOR, app]. Whether a meter ever
splits one, or sends several per notification, is [UNVERIFIED]
(cross-reference §14). A chunk whose byte 14 is `FF` is skipped (BM:399-405); that
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
| 11 | stored, no consumer found; meaning [UNVERIFIED] (cross-reference §14) |
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
| 5 | a number, flagged as large error data (below) |
| 6, 7 | no reading |

R10W:270-947. Status 5 sets a flag in the value (R10W:484-515, :530-533)
whose only reader found is the app's log: with the setting "Filter out large error
data" on, such a reading is not logged (`realtime_data_logger.dart:1653-1712`;
label `owon_imeter/generated/intl/messages_en.dart:963`). OWON's build leaves the setting off by
default; Voltcraft's turns it on at every start
(V:`realtime_data_logger.dart:784-791`). The display is unchanged, and the
6-byte parser never sets the flag (R2W:206-219, :252-269, :302-317,
:346-361, :514-529) [VENDOR, app]. That status 5 means a
large measurement error is [INFERRED from the setting's name]. Further rules
(R10W:532-940): a count longer than the model's
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
| 30, 31 | no entry: the lookup throws, dropping the rest of that notification [INFERRED] |

PE:122-869, :4425-4481. Codes 16, 17, 19 and 24 also take no prefix (PE:871-947). With
main function 23, a DC V or AC V sub-display is labelled "DC" or "AC"
(PE:5952-6085).

### 10.6 Status bits 16-18

| Bit | Mask | App |
|---|---|---|
| 16 | 0x10000 | Err_port ("Err") |
| 17 | 0x20000 | Inrush |
| 18 | 0x40000 | OSC |

PE:3534-3917. Each set bit's name is shown as a chip on the control page
(`device_control_page.dart:2798-3100`); no bit is gated by model, on either
frame (PE:3447-3533). With OSC set, the app neither charts the reading nor
keeps it in record lists (PE:6334-6393, :6505-6554;
`data_logger_client.dart:44-113`) [VENDOR, app].

### 10.7 VC871 (code 87)

The only model branch in the parser (R10W:1077-1404): digits are always 5
and counts 60000, and when the status holds REL, MAX or MIN the sub-display
takes the main display's function, keeping its own prefix, decimal point and
flags. OW67, which shares the VC871's key list, gets the ordinary parser.

### 10.8 Commands

- **`#TIMEsync`**: `23 54 49 4D 45 73 79 6E 63` ("#TIMEsync") + CC YY MM
  DD hh mm ss, 16 bytes, binary, local time, written to FFF1 with response
  after the FFF4 subscribe, for codes 87, 89, 91, 92, 67 and 69 only
  (`modelWithRTCList`, MD:6302-6357; PE:2938-3369; BBM:1138-1301). No reply
  is read; a failed write disconnects (BBM:81-165). The field order maps the
  runtime's `DateTime` parts [INFERRED]. These are the models with a clock
  (§9.4).
- **Key presses** as §7.1: `[code, 01]` short, `[code, 00]` long, except
  Hold/Light, whose long press is `09 01` (MD:745-4910; long press from
  `onLongPress`, settled through the Flutter code in the listing:
  `material_button.dart:140-143`, `ink_well.dart:777-936`) [VENDOR, app]:

| List (MD line) | Codes | Keys (code) |
|---|---|---|
| `_cms101Keys` (4385) | 101, 61 | Select 1, Range 2, Hold/Light 3, Rel 4, Hz/Duty 5, Max/Min 6, Inrush `0D` |
| `_c91Keys` (3502) | 91 | Select 1, Range 2, Hold 3, Rel 4, Max/Min 6, LPF 7, Compare `0F`, AC/DC `10`, Motor `11`, 4~20mA `0A`, Display `0C` |
| `_c92Keys` (2909) | 92 | Select 1, Range 2, Hold 3, Rel 4, Max/Min 6, Compare `0F`, 4~20mA `0A`, Display `0C` |
| `_c831And851Keys` (2454) | 83, 85 | Select 1, Range 2, Hold/Light 3, Rel 4, Hz/Duty 5, Max/Min 6 |
| `_Ow65Keys` (2069) | 65 | Select 1, Range 2, Hold/Light 3, Rel 4, Max/Min 6 |
| `_c871AndOw67Keys` (1474) | 87, 67 | Select 1, Range 2, Hold/Light 3, Rel 4, Max/Min 6, Peak 8, 4~20mA `0A`, Display `0C` |
| `_c891AndOw69Keys` (745) | 89, 69 | Select 1, Range 2, Hold/Light 3, Rel 4, Hz/Duty 5, Max/Min 6, LPF 7, Peak 8, 4~20mA `0A` |

The VC915/925 "Hold" long press is `03 00`. That code 9 is the light is
[INFERRED from the label "Hold/Light"]; the CMS manual gives its HOLD key's
long press as "DCA to zero" (CMS101-UM p.13/8) and names no light key.
Every list key is offered both ways; none is labelled Bluetooth
[VENDOR, app]. Which code, if any, the meters' BLE keys (§9.4) map to is
[UNVERIFIED]. The control page
adds "Read record" and "Start to record", which open app pages and send
nothing (`device_control_page.dart:3844-4199`). The manuals' screenshots of
the app show other sets: Select, Range, H/light, Rel, Hz/Duty, Max/Min on
the OW65, plus Peak on the OW67 and LPF and Peak on the OW69 (OW65-UM
p.31/26; OW67-UM p.36/31; OW69-UM p.33/28); VC-APP, for the VC871/891,
the `_c871AndOw67Keys` set (p.17). Whether `05 01` does anything on an OW65 or OW67 is [UNVERIFIED].

### 10.9 Offline words

3-byte words: byte 2 = `F0` marks a G24 function/range word, any other word
is a V24 reading (R10W:1406-1739). G24 never uses bits 16-23, so the marker
fits [INFERRED].

### 10.10 Voltcraft's app

Voltcraft's "VC800 VC900 Series" app 1.2.5 is OWON's iMeter code base
(`imeter_base`, `owon_imeter`). With addresses, branch targets and pool
offsets masked and each pool object compared by contents, the listings of
the model table, both frame parsers, the offline parser, the FFF2 reader, the
challenge, the BLE adapter, the scanner and the device classes are the same
as iMeter 1.2.4's; `protocol_element.dart` adds one read-only list used for
speech [VENDOR, app: V: against app, `findings/voltcraft-app.md`]. So §1-10
hold for both apps. In the code that connects and decodes, the build
differs in defaults only (the UI files were not compared): `OEM_CODE` 1
against 0 (`V:main.dart:15-21`), which selects Voltcraft's pages, turns the
"Filter device" switch on (§2) and turns "Filter out large error data" on at
every start (§10.4); and its CSV export starts with a UTF-8 BOM
(`V:imeter_base/utils/extension.dart:66-70`). Other changes are app-only:
speech, and the auto-save count's meaning. No comparison of an advertised
name was found in either listing, 2026-10-02; "VC" is tested only against the model name,
for the CSV header (`V:extension.dart:785-818`) [VENDOR, app].

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
whether any meter does is [UNVERIFIED] (cross-reference §14).

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
  "BDM" or one a user chose (Voltcraft's manuals name "VC871", "VC891"
  and "VCxxx").
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
- Codes 101, 61, 91, 92, 65, 87, 67, 89 and 69 send 15-byte frames: five
  little-endian 24-bit words, main and sub-display function and reading,
  then status; the sub-display words count only when main G24 bit 12 is
  set. Reading magnitudes are 19 bits, with a 3-bit status code and a sign
  bit.
- An offline dump sits between two runs of 20 `FF` bytes; inside it a word
  whose high byte masked with `FC` is `F0` sets the range for the readings
  after it.

## 13. Open questions — [UNVERIFIED]

The open checks are in [verification.md](verification.md).

---

## 14. Cross-reference with community sources [COMMUNITY]

Read 2026-10-01, after §1-13 were written from OWON's sources and committed
(`41ac399d`); nothing here was merged into §1-12, which only point here.
Every point a community source disputes was re-read in OWON's app and PC
source alone: the reading of OWON's code in §1-12 holds in every case, and
no body statement changed. One [INFERRED] statement, that the meters
advertise FFF0, is contradicted by captures (D1); it keeps its tag and
points here. A second read on 2026-10-02, after the 15-byte meters' vendor
text was committed (`ca23ba9d`), covered those meters under the same rule;
only the VC871 has hardware data, and one [INFERRED] mapping of §9.4 is
contradicted (D11). Sources and the boundary:
`reverse-engineering-approach.md`.

DeanCording's B35T+ work (2018) is the root of most later 6-byte clients
(jtcash, likeablob, PBrunot, palmerr23, pjpa365 cite it), and the status
names OL, RMR, PMIN … that jtcash, MartMet, webspiderteam and libreble carry
are the PC's enum (§6.6), not observations. Where such sources match §1-12
they confirm our reading of OWON's code, not the meters; their value is
their captures (§14.5).

### 14.1 Sources

| Source | Models tested | Covers | Hardware captures | Licence |
|---|---|---|---|---|
| [DeanCording/owonb35](https://github.com/DeanCording/owonb35) `dbbc4e1` (2018), and the thread that led to it, [inflex/owon-b35 issue #1](https://github.com/inflex/owon-b35/issues/1) | B35T+ (Semic CS7729CN-001, board 2017.03.24 v1.6) | 6-byte frame, status bits 0-5, keys short and long, offline record and read-back (README "Protocol", "Interactive Commands", "Offline Recording") | Yes: README dump, `*DATe` and `*RECOrd,` bytes; in issue #1 five live frames, an hcidump of the advertisement and the FFF4 declaration, and `packets.txt`, a Wireshark listing (lengths only) of OWON's 2018 Android app | MIT |
| [inflex/owon-b35](https://github.com/inflex/owon-b35) `73c6baf` (2017) | B35T with the FS9922 | 14-byte frames on handle 0x002E (`owoncli.c:161-326`) | Implied | BSD-3-Clause |
| [cransom/b35t-reader](https://github.com/cransom/b35t-reader) `2912481` (2017); [akemnade/owon-tools](https://github.com/akemnade/owon-tools) `2787c4c` (2017) | B35T | 14-byte frames; akemnade also key writes `[code, 1]` on FFF3 (`owon_device.vala:17-31`) | Implied | none; GPL-3.0 |
| [reaper7/M5Stack_BLE_client_Owon_B35T](https://github.com/reaper7/M5Stack_BLE_client_Owon_B35T) `540e576` | B35T and B35T+ | Both frames, told apart by length and byte 1 ≥ `F0` (`.ino:894-902`); keys `01` short, `00` long (`:83-86`) | One example of each in comments (`:100-110`, `:441`) | none |
| [ondras12345/B35T](https://github.com/ondras12345/B35T) `8842d9b` | B35T, "Bluetooth 2.0" version only | 14-byte frames over SPP (RFCOMM) | Yes: 2019 fixtures per function (`tests/unit/fixtures/decoder/`) | MIT |
| [sercona/Owon-Multimeters](https://github.com/sercona/Owon-Multimeters) `1718fda` (2023-24; [sercona/owon-cm2100b-clamp-meter](https://github.com/sercona/owon-cm2100b-clamp-meter) `bc683b2` moved here) | B35T+, B41T+, OW18E, CM2100B | 6-byte frame through `gatttool --char-read --listen`, handles per model (`code/owon_multi_cli.c:53-61`, `:403`); ESP32 client | Yes: Ω runs on all four (`test_txt/*-ref-data-ohms.txt`), README and code-comment frames | BSD-3-Clause |
| [JayTee42/ow18b](https://github.com/JayTee42/ow18b) `ce4e131` (2019); [kwasmich/ow18e](https://github.com/kwasmich/ow18e) `6dfe32e` (2020, its fork) | OW18B; OW18E | Raw HCI: an LE connection and ACL reads of notifications on handle 0x001B, no ATT PDU sent (`src/ow18b.c:540-700`); function words per mode (`include/ow18b.h:98-120`; `ow18e.txt`) | Per-mode constants, one annotated OW18E frame | MIT |
| [rbelnienk/OWON-OW18B-BLE-Connector](https://github.com/rbelnienk/OWON-OW18B-BLE-Connector) `2de51d2` | OW18B | V and mV through bleak | One README frame | MIT |
| [MartMet/OW18B](https://github.com/MartMet/OW18B) `c1c52e8` ([JAQUBA/OWON_OW18B](https://github.com/JAQUBA/OWON_OW18B) `c3277c7` ports it) | OW18B | Web Bluetooth; the PC's enums; keys (`Pages/Ble.razor.cs:122-190`) | No | MIT |
| [jtcash/OwonB41T](https://github.com/jtcash/OwonB41T) `9d880c2` (2021; art-ya/OwonB41T `1f23cba` forks it) | B41T+ | WinRT client: keys, `*DATe`, `*RECOrd,`, `*READlen?`, `*READ1?`, rename (`B41T.cpp:133-262`; `packet_handler.hpp:9-30`) | README read-back run | Unlicense |
| [likeablob/owon-bdm-webui](https://github.com/likeablob/owon-bdm-webui) `00a61b8`; [PBrunot/owonb41t](https://github.com/PBrunot/owonb41t) `db9dbbe`; [palmerr23/Owon_B41T](https://github.com/palmerr23/Owon_B41T) `dec8a5b` | B41T+ | Web Bluetooth clients; an ESP32 bridge that writes the CCCD (`BLEfuncs.h:71`) | No | MIT; MIT; none |
| [pjpa365/owon-suite](https://github.com/pjpa365/owon-suite) `693b534` (2026) | B41T+ | GATT enumeration, FFF2, write rules, offline read-back (`docs/protocol-spec.md` §2.1, §4-6) | Yes, 2026-07: `poc/tests/test_protocol.py:104-157` | none stated |
| [webspiderteam/Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows) `2b83d9e`, discussions [#49](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows/discussions/49) and [#66](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows/discussions/66) | B35T+ (a reporter, #49); Voltcraft VC871 (two reporters, #66) | Decoders for the 14-, 6- and 15-byte frames (`Decoders/DecoderOwon.cs`, whose comment also quotes a decompiled older OWON Android app) | Yes: 840 6-byte frames in `Utilities.cs` (`dev_type == 6`, `:1342`), added in `fb9ff02` on 2024-04-15, the day the #49 B35T+ owner posted OL logs [INFERRED provenance]; VC871 frames with the owner's readings and FireBird3314's bit notes (`VC871 BLE GATT.txt`, #66); read again 2026-10-02: #66's attachments `uuids.txt` (11 GATT dumps, two meters), RefuCire's log (411 frames) and FireBird3314's `TestLogs.zip` (369 raw notifications, 220 frames with the app's decode, 2026-01-23) | none stated (the repository's LICENSE is Microsoft's MIT text for "Bluetooth LE Explorer") |
| [libreble/multimeter](https://github.com/libreble/multimeter) `d26ba48` | none ("not yet live on a physical meter") | Ports of webspiderteam (`docs/protocols/owon-plus.md`, `owon-old.md`); a Voltcraft driver (`docs/protocols/voltcraft.md`, `drivers/voltcraft.ts`, read 2026-10-02) | No | MIT |
| [ble-multimeter/fakemeter](https://github.com/ble-multimeter/fakemeter) `4cca1d5` (2026-06) | none: a BlueZ emulator posing as code 91 (VC915) to Voltcraft's app 1.2.3 | The app's writes and checks against an emulated meter (`docs/PROGRESS.md`, `docs/owon-voltcraft-handshake.md`) | No meter; the vendor app's writes, captured | MIT |
| [53845714nF/OWON_B35T](https://github.com/53845714nF/OWON_B35T) `cbf1051`; [luissantos/multimeter_gui](https://github.com/luissantos/multimeter_gui) `44d6c4b`; [VYD3N/Mult-AI-Meter](https://github.com/VYD3N/Mult-AI-Meter) `89620da` | B35T+; B41T+; unstated | Derivative clients | No | MIT; none stated; none |
| [sigrok-devel, 2017-02-27](https://sourceforge.net/p/sigrok/mailman/message/35691836/) | B35T | "based on the Fortune Semiconductor FS9922-DMM4", its serial data sent over BLE | No | — |

Nothing on these meters in libsigrok (`0bc2487`: OWON appears only in
`scpi-dmm` for the XDM bench meters) or the sigrok wiki (search, no hits).

### 14.2 Agree

| Spec § | What the community sources show | Evidence |
|---|---|---|
| §2 name, address | Every scan shows "BDM"; a B35T+ advertises ADV_IND from a public address with a Texas Instruments OUI (hcidump, inflex #1) | captures |
| §2 UUIDs, roles | Every client finds FFF0 and FFF1/FFF3/FFF4 by the 16-bit-base 128-bit UUIDs; FFF1 takes the commands, FFF3 the keys, FFF4 notifies frames and the dump | working clients, all six models |
| §2 handle sets | Notifications on 0x002E from the B35T, B35T+ and B41T+, on 0x001B from the OW18B, OW18E and CM2100B; OWON's 2018 Android app reads 0x0028 and writes 0x0025 on a B35T+ | captures |
| §3.1 no start command | Clients subscribe, or not even that (§14.4 CCCD), and frames arrive | captures, all six models |
| §4 FFF2 | B41T+ `29 FF 00 01 02 00`: code 41, firmware 0.1.2 (12), so `*READ1?` and a dated header — which that meter then sent (pjpa365) | capture |
| §5 framing | One 6-byte frame per notification, no header or checksum (hcidump and app sniff, inflex #1; JayTee42 drops any other ACL length and still receives) | captures |
| §6.1-6.3 word layout | Every frame in §14.5 decodes to its display; bits 10-15 are `111100` in every live frame and offline range word seen, on all five 6-byte models (DeanCording: "possibly a start marker") | captures |
| §6.4 OL | Decimal code 7 at OL: B35T+ Ω, diode and continuity; B41T+ Ω, live and offline | captures |
| §6.5 sign | 285 of the 840 B35T+ frames are negative, sign-magnitude; webspiderteam's two's-complement decode showed −0.606 V as −33.374 V until fixed (#49) | captures |
| §6.6 bits 0-5 | HOLD, REL, AUTO, low battery, MIN, MAX (DeanCording, JayTee42); the B35T+ set shows AUTO, and REL without AUTO | captures in part |
| §6.7 NCV | OW18B NCV word 0xF360 (function 13, prefix 4), values 0-4 (`ow18b.h:120`); OW18E `f3 6x` "NCV" (`ow18e.txt`) | per-mode constants |
| §7.1 keys | `[code, 01]` short, `[code, 00]` long, codes 1-6 as named (DeanCording, akemnade, reaper7, jtcash); HOLD acted on a B41T+ (pjpa365) | writes; HOLD seen |
| §7.2 commands | `*DATe` binary CC YY MM DD hh mm ss: a B41T+ header returned the time sent; `*RECOrd,` interval in s, then count; the meter disconnects after it (DeanCording, jtcash, pjpa365) | captures |
| §8.2 dump framing | 20 `FF`, payload, 20 `FF`, in 20-byte notifications (DeanCording's B35T+ example; pjpa365's B41T+ streams) | captures |
| §8.3 payload | Century, YY…ss, byte 7 `00`, interval u32 in s, a u32, then a range word (`(b1 & FC) == F0`) and readings; a new range word at each range or OL change (pjpa365) | captures |
| §9.1 | Bluetooth suspends auto power-off: "The meter will not sleep while BT is enabled" (cransom README) | user report |
| §2 GATT, VC871 | FFF0 holds FFF1 (Read, Write Without Response, Write), FFF2 (Read), FFF3 (Write), FFF4 (Notify only) and FFF5 (Read); identical in 11 dumps from two meters. The declaration handles put the values at 0x0015, 0x0017, 0x0019 and 0x001B, the PC's "OW" handles [INFERRED from the declaration-handle arithmetic] | `uuids.txt` (#66), WinRT dumps |
| §3.1, §3.4 on the VC871 | It streams after a CCCD write alone: no FFF2 read, challenge or `#TIMEsync` (webspiderteam's client, `GattMonitor.cs:331-335`; both owners got readings) | working client |
| §10.2 framing | One 15-byte frame per notification (369 raw notifications, all 15 bytes); byte 14 never `FF` | capture (VC871) |
| §10.3-10.5 | G24 bits 13-23 are `0x780` (byte 2 and byte 8 `F0`) in every main and sub word; magnitudes above 16 bits (540.74 mV as `3A D3 00`); OL as V24 status 1 with decimal code 7; Time (19) in seconds (sub 62 against the LCD's "00:01:02"); prefix 4 with duty, ℃, ℉, PF, Time, diode and continuity | capture (VC871), LCD photo |
| §10.8 keys, `#TIMEsync` | Voltcraft's app 1.2.3, driving an emulated code 91, wrote exactly `_c91Keys`' short presses to FFF3 and `#TIMEsync` to FFF1 after the subscribe, needing no reply; it accepted the raw 16-byte MD5 and showed "code:3" on a wrong one, "code:-2" for an FFF2 shorter than 6 bytes | fakemeter `docs/PROGRESS.md:549-605`; the vendor app's behaviour, no meter |
| §10 15-byte frame | The VC871 frames in §14.5 decode per §10.2-10.5 to the readings reported, where one is; FireBird3314's notes match G24 bits 0-2, 3-5, 6-10 and 12, V24 bits 0-18 (">65535 counts" at bit 16), OL at bit 20 (status 1), sign at bit 23, status bits 0-5 and functions 14-22 | captures (VC871) |
| §11 14-byte layout | Sign, four ASCII digits, space, a DP byte `1`/`2`/`4`, four status bytes, a bar-graph byte, CR LF (inflex, akemnade, cransom, reaper7, ondras12345) | captures |

### 14.3 Disagree

"Vendor re-check" is what OWON's code does on a neutral re-read. "Kind"
says whether the conflict is between OWON's code and a meter, or a community
error.

| # | Spec § | Community | Vendor re-check | Kind: evidence favours | Settled by |
|---|---|---|---|---|---|
| D1 | §2: the meter advertises FFF0 [INFERRED from the app's filter] | B35T+: ADV_IND with Flags only; the scan response carries the name, AD type 0x12 and TX power, no UUID (inflex #1). B41T+: no service UUID in repeated Windows scans (pjpa365 §1). sercona's ESP32 client filters on an advertised FFF0, aimed at the CM2100B, with no result reported | The app's filter exists and is off by default (§2); neither program shows an advertisement | OWON's code vs meter: the captures, for the B35T+ and B41T+ | A passive scan per model, the CM2100B first |
| D2 | §6.6 bit 6: AVG (app), OL (PC) | jtcash, pjpa365, MartMet and webspiderteam name it OL, from the PC's enum. At OL the B35T+ and B41T+ send decimal code 7 with bit 6 clear (pjpa365 §3: "bit 6 was not observed to be set"); no 6-byte capture sets any of bits 6-15 | The PC's OL text comes from the decimal code (MC:1594-1598); its enum is names only | community naming copied from the PC; OL is carried by the decimal code; bits 6-15 stay open | Captures with MAX/MIN and REL held, and a B41T for RMR |
| D3 | §6.4 code 6 = UL | likeablob: 6 = overload (`utils/owon-b41t-plus.ts:90`) | Both: 6 UL, 7 OL | community error: the constant dates from when its "decimal" read bits 3-5 (prefix 6, M, at Ω OL), fixed in `ff9e0e5` without the constant | settled |
| D4 | §6.5 magnitude bits 0-14 | JayTee42, kwasmich: bits 0-13, bit 14 "unused" | Both `& 0x7FFF` | community error: seen on a 5999-count OW18B; the OW18E's 19999 needs bit 14 | A reading above 16383 on an OW18E or B41T |
| D5 | §6.3 prefix 0 = p | jtcash, pjpa365: 0 = "%" (×0.01) | Both p (1e-12 in the app) | community guess: no capture sends prefix 0 | Unseen |
| D6 | §7.1, §7.3 key 5 = Hz/Duty on OW meters | MartMet (and JAQUBA) send `04 01` for Hz/Duty and `04 00` for Bluetooth on an OW18B, no result stated | App `_owSeriesKeys` (MD:5989): code 5; PC OW panel: `hzPress` (5), `frame/MainFrame.java:330-348` | unsettled; both vendor programs say 5 | `04 01` and `05 01` on an OW18B/E, the LCD noted |
| D7 | §8.3 bytes 12-15: record count (app) | The payload's byte count, 2 × (1 + readings): 42 for a range word and 20 readings (DeanCording), 12, 22, 42 and 8 for 5, 10, 20 and 3 readings (pjpa365 §6.3-6.4); jtcash sizes the read-back as value / 2 − 1 and gets the same value from `*READlen?` | The app stores it in `OfflineRecordConfig.recordLen` (built at the end of PE:4482-5107; key at PE:5518-5762) and never bounds the parse with it (R2W:687-1017 walks the payload); the PC logs it as "length" | OWON's label vs meter: the captures. "Record count" is the app's label; the meter sends bytes, which also fits the manuals' "22 bytes" (1 + 10 words) and "48 bytes" (§7.2) | Settled for the B35T+ and B41T+; a range change mid-recording stays in §8.3's open item |
| D8 | §8.2 the closing 20 `FF` | pjpa365 §6.2: "there isn't one — no `0xFFFF` terminator" | Both expect it | community error: its own streams end in 20 `FF` (`_OPEN_CIRCUIT_FULL_STREAM`, `_VOLTAGE_FULL_STREAM`), and its 86- and 116-byte totals include them | settled |
| D9 | §7.2 `*RECOrd,` interval in s | DeanCording's README example: `40 16 40 00` (4 200 000) with count `10 27 00 00` (10 000) | Both seconds; PC "Sec" | unexplained example: DeanCording's code, jtcash and pjpa365 send seconds, and pjpa365's headers echo 1, 2 and 3 | settled |
| D10 | §6.2 function 9 = ℉ | kwasmich: at ℉ the OW18E sends 9 "(value is in °C)" (`ow18e.txt`) | Both label 9 ℉ and show the count as sent | unsettled: a note, no capture | An OW18E at ℉ against its LCD |
| D11 | §9.4's functions table: 18-22 as the OW67B/VC871's USB power [INFERRED by name] | VC871: 21 (V) and 22 (A) also come in AC power (status bit 13: 21 with sub 3, A AC) and DC power (bit 14: 14 with sub 22, 21 with sub 22); 18 (mAh) and 20 (Wh) with sub 19 under bit 15 (USB) | The app labels 21 "Power, V" and 22 "Power, A", with no USB tie (PE:122-869) | our inference vs meter: the captures | Settled for the VC871 |
| D12 | §10.1, §10.3 | libreble: "VC800 / R2W meters use a SEPARATE 6-byte protocol" (`docs/protocols/voltcraft.md:27`); "the real meter sends [the sub block] zero with bit 12 cleared" (`:65`); functions above 13 rejected (`:41-47`); "redundant AC/DC state bits 13/14" (`:88`). fakemeter: big-endian words (`docs/voltcraft-measurement-protocol.md:17`, `:36`), prefixes only p-m (`:228-231`) | Codes 87 and 89 are 15-byte (§10.1); bits 13/14 are the app's AC and DC chips (§6.6) | community errors, none from hardware: the VC871 sends 15 bytes, functions 14-22, sub words filled with bit 12 clear (§14.4), bits 13/14 only in power modes, and prefixes k and M, little-endian | settled |

### 14.4 New

Facts §1-12 lack or mark [UNVERIFIED], from captures unless marked:

| Topic (§) | Finding | Source |
|---|---|---|
| Hardware (§1, §11) | The B35T+ runs a Semic CS7729CN-001 (board 2017.03.24 v1.6, label "B35T+ … BLE4.0"); earlier B35/B35T a Fortune FS9922-DMM4, which sends the 14-byte frame | DeanCording (inflex #1); sigrok-devel |
| 14-byte meters (§11) | The B35T sends it over Bluetooth 2.0 SPP and, on FS9922 BLE units, as FFF4 notifications on 0x002E in the same service; the 6-byte B35T+ replaced it from mid-2017. Status bytes: byte 7 bit 5 AUTO, 4 DC, 3 AC, 2 REL, 1 HOLD; byte 8 bit 5 MAX, 4 MIN, 1 n, low battery at bit 2 (inflex) or 3 (webspiderteam); byte 9 bit 7 µ, 6 m, 5 k, 4 M, 3 continuity, 2 diode, 1 duty; byte 10 bit 7 V, 6 A, 5 Ω, 4 hFE, 3 Hz, 2 F, 1 ℃, 0 ℉; byte 11 the bar graph; OL as digits `?0:?` | ondras12345; inflex, cransom, akemnade, reaper7 |
| GATT layout per model (§1, §2) | BT handles: B35T, B35T+, B41T+; OW handles: OW18B, OW18E and the CM2100B, which the PC, holding no code 21, would drive with BT handles (§1) | sercona, JayTee42, kwasmich, inflex #1 |
| FFF4 declaration (§2) | `10 2E 00 F4 FF` at 0x002D on a B35T+: notify only, value handle 0x002E | inflex #1 |
| Other attributes (§2) | B41T+: GAP Device Name "LILLIPUT" (Windows showed "Lilliput" for a B35T+, #49); preferred connection 100-200 ms, latency 0, timeout 10 s; Device Information with SDK placeholder strings; FFF1 read and write only, reading "ABCDEFGHIJKLMN" + `00 00` when no command is pending; FFF5 read-only, refused with ATT error 0x05 (insufficient authentication). An older OWON Android app names FFF1 "Secure", FFF2 "Info", FFF3 "Write", FFF4 "Read", FFF5 "Not use", beside the SPP UUID `00001101-…` (decompiled, not a capture) | pjpa365 §2.1; `DecoderOwon.cs:156-179` |
| Advertisement (§2) | B35T+ scan response: the name in a 15-byte field ("BDM" and 12 non-printing bytes), AD 0x12 (connection interval range) and TX power 0 | inflex #1 |
| Write types (§2) | FFF1 refuses any length but 16 (ATT error 0x0D) and takes a Write Request; FFF3 takes Write Without Response (HOLD acted) | pjpa365 §2, §4-5, B41T+ |
| CCCD (§2, §3.1) | Notifications arrive with no CCCD write: JayTee42 and kwasmich send no ATT PDU at all (OW18B, OW18E); `gatttool --char-read --listen` never writes it (B35T, B35T+, B41T+, OW18E, CM2100B); OWON's 2018 app shows no such write. That BlueZ wrote none behind gatttool is [INFERRED] | captures |
| Challenge (§3.4) | No community client sends it and all get readings, up to a B41T+ on firmware 0.1.2 in 2026. OWON's 2018 Android app on a B35T+, by frame lengths: read FFF2 (6 bytes), a 16-byte Write Request to FFF1, a read of FFF1 (16 bytes) 4.5 ms after the write response, then notifications — §3.4's sequence before iMeter; it re-read FFF2 about every 6 s | pjpa365; inflex #1 `packets.txt` |
| Centrals (§2) | "only one client can connect to the multimeter at a time" | DeanCording README (B35T+) |
| Rename (§2, §7.2) | A B41T+ renamed with `@` + name is then found by the new advertised name; jtcash caps names at 14 characters itself | jtcash README |
| FFF2 (§4) | 6 bytes on a B41T+ and on a B35T+ (2018 sniff, length only); byte 1 `FF` on that B41T+ | pjpa365; inflex #1 |
| Rate (§5) | B35T+ about one frame per 0.6 s (DeanCording; 0.5-0.65 s in the 2018 sniff); B41T+ about 2/s | DeanCording, pjpa365 |
| Prefix (§6.3) | 4 with duty, ℃, hFE, diode and continuity on the B35T+, ℃ on the B41T+, ℃, ℉ and NCV on the OW18B | B35T+ set, pjpa365, `ow18b.h` |
| Status (§6.6) | `0000` (no AUTO) with duty, ℃, diode, continuity and hFE; AUTO stays set at Ω OL | B35T+ set |
| OL (§6.4) | Magnitude 0, sign clear, prefix kept (M at Ω OL), bit 6 clear | B35T+ set, pjpa365 |
| Negative zero (§6.5) | `00 80` is sent (B35T+) | B35T+ set |
| Not seen (§6.2-6.6) | In every 6-byte capture: UL, decimal code 5, the 0x6FFF magnitude, function 13 on a B-series meter, 14-15, status bits 6-15 | all |
| Long presses (§7.1) | `02 00` back to auto range, `03 00` backlight, `04 00` Bluetooth off, `06 00` leave MIN/MAX (B35T+, DeanCording's interactive keys; reused for the B41T+ by likeablob and pjpa365, only HOLD confirmed); `03 00` light on an OW18B (MartMet) | DeanCording README; MartMet README |
| `*READlen?` (§7.2) | Read as a u32: the payload's byte count, 62 on a B41T+ | jtcash README, `B41T.cpp:223-249` |
| Read-back (§8) | 2-3 live frames precede the opening `FF`s and live frames resume after the closing ones; the date reads 0 (century 0) until `*DATe` has been sent; `interval` 0 is accepted (pjpa365 timed about 500 samples/s, once, by stopwatch) | pjpa365 §6, DeanCording README |
| G24 (§10.3) | Bit 11 is 0 in the main word and 1 in the sub-display word; bits 16-23 are `F0` in both, like §10.9's offline marker; the sub-display bytes are filled even with bit 12 clear: a Hz word `A1 09 F0` carrying the main reading's magnitude early in a session, then `A2 09 F0 00 00 00` once a function with a sub-display has run, and a ℃ word (268.6) at µA DC | VC871 frames and notes (#66) |
| 15-byte status (§10.6) | VC871: bit 8 LoZ, 12 power factor, 13 AC, 14 DC and 15 USB power measurement — the app's Loz, CosPhi, AC, DC and USB (FireBird3314's notes); bit 16 set in 4 notifications right after the dial moved from mA AC to A DC (`… 04 00 01`), the app's "Err_port" [INFERRED: a wrong-jack warning]; AUTO clear at mV AC, continuity, diode, ℃, ℉ and in power modes; bits 6-7, 9-11 and 17-23 never seen set | FireBird3314 (#66), notes and captures |
| GATT extras (§2) | VC871: GAP Device Name Read + Notify; Device Information 2A29, 2A24, 2A26; a vendor service `00010203-0405-0607-0809-0a0b0c0d1912` (`…1911` in later dumps) with one Read + Write Without Response characteristic | `uuids.txt` (#66) |
| Name (§2) | Windows listed FireBird3314's VC871 as "VC871"; advertised name or GAP name cannot be told apart there | #66, 2026-01-25 |
| OL (§10.4) | The magnitude at OL is not full scale or 0: 10971 (kΩ), 911 (MΩ), 32258 (diode), `0x7FFFF` (nF) | capture (VC871) |
| Repeats (§10.2) | Unchanged frames are notified again; the rate is not logged | capture (VC871) |
| Sub-display pairs (§10, §9.4) | V AC and A AC with Hz; Hz with duty and duty with Hz; ℃ with ℉ and back; W with VA; PF with Hz; the power modes as D11 | capture (VC871) |
| Bluetooth (§9.4) | Bluetooth needs the BLE key after each power-on; an auto-reconnecting client still sees a ~1 s connection as the meter starts | FireBird3314 (#66), user report |
| Not seen (§10) | On the VC871: functions 17 and 23-29, V24 statuses 2-7, V24 bit 19, REL/MAX/MIN frames (§10.7), FFF2's bytes, the advertisement, any reply to keys or `#TIMEsync`. No capture of any other 15-byte model, 2026-10-02 | all |
| 15-byte models (§10.1) | The VC871 streams 15-byte frames on FFF4, up to 60000 counts | #66 |

### 14.5 Captured vectors

Decoded with §5-6 (§10 for the VC871, §11 for the 14-byte frames).
"Fits" means the decode matches the reading its source reports.

| Source, model | Bytes | Decode | Fit |
|---|---|---|---|
| inflex #1, B35T+ | `19 F0 04 00 E9 0D` | 0xF019 DC V, m, dp 1; AUTO; 3561 → 356.1 mV | fits (display "355.6" noted at another moment) |
| inflex #1, B35T+ | `20 F2 00 00 1D 00` | ℃, prefix 4, dp 0; no flags; 29 | fits ("0029") |
| inflex #1, B35T+ | `63 F0 04 00 10 00` | AC V, prefix 4, dp 3; AUTO; 0.016 V | fits |
| inflex #1, B35T+ | `21 F1 04 00 07 00` | Ω, prefix 4, dp 1; AUTO; 0.7 Ω | no fit with the reported "0.003 ohms"; likely a slip in the report |
| inflex #1, B35T+ | `E7 F2 00 00 00 00` | continuity, dp 7 (OL); no flags; 0 | fits ("0L") |
| webspiderteam set, B35T+ | `37 F1 04 00 00 00`, `A7 F2 00 00 00 00` | Ω, M, OL, AUTO; diode, OL | fits ("0.L" MΩ, ".0L" V, #49) |
| webspiderteam set, B35T+ | `19 F0 04 00 2B 89`, `19 F0 04 00 00 80` | −234.7 mV DC; −0.0 | fits |
| reaper7, B35T+ | `19 F0 04 00 49 04` | 109.7 mV DC, AUTO | example, no reading stated |
| sercona, B41T+ | `34 F1 04 00 81 2B` | Ω, M, dp 4; 11137 → 1.1137 MΩ | fits |
| sercona, CM2100B | `24 F0 04 00 C6 3A`; `22 F1 04 00 9F 11` | DC V, dp 4: 1.5046 V; Ω, dp 2: 45.11 Ω | fit (code comment; test datum "45.11 Ohms") |
| sercona, OW18E | `2B F1 04 00 56 09` | Ω, k, dp 3; 2.390 kΩ | fits |
| sercona, OW18E | `2C F1 04 00 0D 7F` | Ω, k, dp 4; 32525 | no fit as a display: above 19999 counts, mid-range-change in that run |
| rbelnienk, OW18B | `22 F0 04 00 00 00` | DC V, dp 2; 0.00 V | fits |
| kwasmich, OW18E | `24 F0 05 00 1F 00` | DC V, dp 4; HOLD, AUTO; 0.0031 V | fits its annotation |
| pjpa365, B41T+ | `24 F0 04 00 03 00` | DC V, dp 4; 0.0003 V | fits |
| pjpa365, B41T+ (FFF2) | `29 FF 00 01 02 00` | code 41, byte 1 `FF`, firmware 0.1.2, not recording | fits §4 |
| pjpa365, B41T+ (dump) | 3 × `37 F1 04 00 00 00`, 20 × `FF`, `00`×8 `02 00 00 00` `0C 00 00 00`, `37 F1` + 5 × `00 00`, 20 × `FF` | live OL frames; header: clock unset, interval 2 s, 12 bytes; Ω OL range word, five readings | fits §8 (D7, D8) |
| pjpa365, B41T+ (dump) | `14 1A 07 17 0D 10 1C 00` `01 00 00 00` `08 00 00 00` `37 F1` + 3 × `00 00` | 2026-07-23 13:16:28, 1 s, 8 bytes; Ω OL, three readings | fits the `*DATe` sent |
| DeanCording, B35T+ (dump) | `14 12 04 0E 0E 17 18 00` `02 00 00 00` `2A 00 00 00` `19 F0` `09 0E` … | 2018-04-14 14:23:24, 2 s, 42 bytes; mV DC range word; 359.3 mV … | fits §8.3 (D7) |
| reaper7, B35T (14-byte) | `2B 33 36 32 33 20 34 31 00 40 80 24 0D 0A` | +3623, DP `4` → 362.3; AUTO, DC; m; V; bar 36 | fits §11's layout |
| ondras12345, B35T BT2.0 | `2B 3F 30 3A 3F 20 31 21 20 10 20 3D 0D 0A` | `?0:?`: OL; M; Ω | fits (fixture `Ohm`) |
| #66, VC871 | `76 01 F0 F6 00 00 A1 09 F0 F6 00 00 04 00 00` | CAP, prefix 6 (M), code 6 (UL); 246; AUTO; sub word bit 12 clear | unclear: webspiderteam's first VC871 decoder printed "0.0246 nF"; no LCD reading given |
| #66, VC871 | `24 00 F0 46 27 80 A1 09 F0 46 27 80 05 00 00` | DC V, dp 4; sign, 10054 → −1.0054 V; HOLD, AUTO | fits |
| #66, VC871 | `1F 00 F0 19 11 11 A2 09 F0 00 00 00 04 00 00` | DC V, m, dp 7 (OL); V24 status 1 (OL) | fits ("OL mV") |
| #66, VC871 | `21 12 F0 F9 00 00 61 1A F0 00 03 00 00 00 00` | ℃, dp 1, sub present: 24.9; sub ℉: 76.8 | fits |
| #66, VC871 | `A1 13 F0 00 00 00 E1 1B F0 00 00 00 00 20 00` | function 14 (W), sub 15 (VA); status bit 13 | fits ("Power Measurement", "VA") |
| #66, VC871 | `20 15 F0 00 00 00 E0 1C F0 00 00 00 00 80 00` | function 20 (Wh), sub 19 (Time); status bit 15 | fits ("Wh", time on the sub-display) |
| #66 `TestLogs.zip`, VC871 | `24 00 F0 21 15 00 A1 09 F0 21 15 00 04 00 00` | 0.5409 V DC; sub Hz word with the main magnitude, bit 12 clear | fits |
| same | `1A 00 F0 3A D3 00 A2 09 F0 00 00 00 04 00 00` | 540.74 mV DC (a 19-bit magnitude) | fits |
| same | `59 10 F0 3B 02 00 A2 19 F0 00 00 00 00 00 00` | 57.1 mV AC, sub 0.00 Hz; no AUTO | fits |
| same | `29 01 F0 71 0E 00 A2 09 F0 00 00 00 04 00 00` | 369.7 kΩ | fits |
| same | `A4 02 F0 83 15 00 A2 09 F0 00 00 00 00 00 00` | 0.5507 V diode | fits |
| same | `4C 01 F0 B6 02 00 A2 09 F0 00 00 00 04 00 00` | 0.0694 nF | fits |
| same | `A3 11 F0 53 C3 00 E2 19 F0 77 11 00 04 00 00` | 50.003 Hz, sub 44.71 % | fits |
| same | `E2 11 F0 7A 14 00 A3 19 F0 96 C6 00 04 00 00` | 52.42 %, sub 50.838 Hz | fits |
| same | `21 12 F0 E2 00 00 61 1A F0 D8 02 00 00 00 00` | 22.6 ℃, sub 72.8 ℉ | fits |
| same | `61 12 F0 0F 03 00 21 1A F0 01 01 00 00 00 00` | 78.3 ℉, sub 25.7 ℃ | fits |
| same | `91 00 F0 36 3E 80 21 0A F0 7E 0A 00 04 00 00` | −1592.6 µA DC; sub ℃ word, bit 12 clear | fits |
| same | `D1 10 F0 88 06 00 A2 19 F0 00 00 00 04 00 00` | 167.2 µA AC | fits |
| same | `9B 00 F0 90 06 80 A2 09 F0 00 00 00 04 00 00` | −1.680 mA DC | fits |
| same | `DA 10 F0 62 00 00 A2 19 F0 00 00 00 04 00 00` | 0.98 mA AC | fits |
| same | `A4 00 F0 4D 0B 00 A2 09 F0 00 00 00 04 00 00` | 0.2893 A DC | fits |
| same | `E3 10 F0 4E 00 00 A2 19 F0 00 00 00 04 00 00` | 0.078 A AC | fits |
| same | `A1 13 F0 82 12 00 E1 1B F0 00 00 00 00 20 00` | 473.8 W, sub 0.0 VA; bit 13 | fits |
| same | `23 14 F0 01 00 80 A1 19 F0 00 00 00 00 30 00` | −0.001 PF, sub 0.0 Hz; bits 12-13 | fits |
| same | `62 15 F0 00 00 00 A2 1D F0 1D 00 00 00 80 00` | function 21 0.00 V, sub 22 0.29 A; bit 15 (USB) | fits |
| same | `98 14 F0 00 00 00 E0 1C F0 00 00 00 00 80 00` | 0 mAh (18, prefix m), sub Time 0; bit 15 | fits the LCD photo's mode |
| same | `61 15 F0 04 00 00 E1 18 F0 00 00 00 00 20 00` | function 21 0.4 V, sub A AC 0.0; bit 13 (AC power) | no reading stated (D11) |
| same | `A3 00 F0 00 00 00 A2 09 F0 00 00 00 04 00 01` | 0.000 A DC; AUTO and bit 16 | no reading stated |
| same | `2F 01 F0 DB 2A 10 …`, `4F 01 F0 FF FF 17 …` | kΩ and nF, decimal code 7 and V24 status 1: OL | fits ("OL") |

The `TestLogs.zip` readings are webspiderteam's decoder output, which the
owner said matched the meter ("all values and units are displayed
correctly", #66).
