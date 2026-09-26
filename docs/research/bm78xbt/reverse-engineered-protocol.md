# Brymen BM78xBT: Reverse-Engineered Protocol Specification

What the Brymen BM78xBT Bluetooth LE multimeters — the BM788BT, and the
BM787BT that EEVblog sells — send and accept. Brymen publishes the protocol.
After an application-level password command, the meter notifies a 152-byte
block: a 24-byte device information packet, one 32-byte reading packet and
three all-zero 32-byte blocks. The reading is numeric (a signed 24-bit count,
a decimal-point position, a power-of-ten prefix, a unit code and two function
IDs), not LCD segments. Commands are 32-byte packets written to a second
characteristic. Every framed packet runs `FF 01` or `FF 02` … `FF 03` and
carries a CRC-16/MODBUS. It is implemented, experimentally
(`crates/dmm-lib/src/protocol/bm78xbt/`), and no BM78xBT has been on our
bench: every fact in §1-11 comes from Brymen's protocol document (two
revisions), Brymen's app and the two user manuals, and §12 compares them with
community sources. The approach doc beside it records the sources, the method
and the clean-room boundary.

Based on:
- Brymen's "BM78xBT Wireless Data Communication Protocol" (the printed
  title; the PDF titles say "Wireless Communication Protocol"), r4 (PDF
  created 2025-10-31) and r2 (2025-04-07, PDF title starting "BM780 Wireless…"), read from the
  rendered pages; r2 compared with r4 by a pixel diff
- Brymen's "BM78xBT OTA programming" rev4, for its GATT and module facts
- Brymen's Android app "IoMBTC Wireless Data Comm" 1.0.73 (`com.IOMBTC.app`),
  a Flutter build, decompiled with blutter
- The BM780(BT) user's manual (BM788BT, BM789, BM785; ©MMXXVI) and the
  BM787BT manual (BM789, BM785, BM788BT, BM787BT; ©MMXXV, the older edition),
  read from the rendered pages

Citation keys:
- **r4 p.N** — `references/bm78xbt/protocol/BM78xBT-Wireless-Communication-Protocol-r4.pdf`,
  PDF page N, which is also the printed number (renders in
  `references/bm78xbt/renders/`)
- **r2 p.N** — `protocol/BM780-Wireless-Communication-Protocol-r2.pdf`; r2
  p.1-2 match r4 p.1-2; r2 p.3-5 hold r4 p.3-6 without 0x0040 and the
  firmware-version example; from p.6 on, r2 p.N is r4 p.N+1
- **OTA p.N** — `protocol/BM78xBT-OTA-programming-rev4.pdf`
- **BM788BT manual p.N (printed M)** — `manuals/BM780(BT)-Print2.pdf`;
  **BM787BT manual p.N (printed M)** — `manuals/BM787BT-Manual.pdf`. In both
  the printed number is the PDF page minus 1 from p.2 on
- **app `file:lines`** — the blutter listing of that name under
  `references/bm78xbt/app/blutter/asm/brymen/`; each cited name is unique
  there except `settings/controller.dart` (`app/modules/settings/controller.dart`).
  Library files carry their package, under `asm/`. Short forms: **BC** =
  `models/bluetooth_command.dart`, **CL** =
  `models/bluetoothDataTransfer/service/model/commandListint.dart`, **BP** =
  `models/bluetoothDataTransfer/service/bluetooth_package.dart`, **DS** =
  `services/bluetooth_service/device_connect/ble_discover_services.dart`
- **Community sources**, §12 only — paths under
  `references/bm78xbt/community/` at the commits of §12.1; a bare
  `transport.py`, `parsers.py`, `commands.py`, `constants.py` or `scanner.py`
  is in `brymenble/src/brymenble/`, and a bare `tools/…` or `docs/…` is in
  `brymenble/`

Byte and bit numbering: `[N]` is byte N of a packet as r4 indexes it, `[0]`
being the `FF` head byte; bit 0 is the least significant bit, as r4's
Bit0-Bit7 columns.

Confidence levels:
- **[KNOWN]** — stated in a Brymen document (protocol, OTA, a manual), cited
  by page
- **[VENDOR]** — read from Brymen's app, cited by file and lines
- **[INFERRED]** — deduction from the above, reason given
- **[UNVERIFIED]** — no source confirms it, or the sources disagree; needs a
  real meter (all in §11)
- **[HARDWARE]** — seen on a real meter: none yet
- **[COMMUNITY]** — stated in or implied by a community source, §12 only
  (opened 2026-09-26); not a vendor fact

---

## 1. Models and identification

Two models have the radio: the BM788BT ("BLE-Comm Wireless Data Communication
(Model 788BT only)", BM788BT manual p.19 (printed 18)) and the BM787BT
("Wireless Data Communication (Models 788BT & 787BT only)", BM787BT manual
p.19 (printed 18)) [KNOWN]. The BM789 and BM785 in the same manuals have none
(BM788BT manual p.23 (printed 22)) [KNOWN]. That the BM787BT uses the same
radio and protocol is [INFERRED]: its manual's reset text ("…to restore
original factory settings for wireless communication Connection Password &
Device Name…", BM787BT manual p.19 (printed 18)) is nearly verbatim r4's
footnote (r4 p.5-6), and the r2 document came from the same EEVblog store
page as the BM787BT manual (`SOURCE.txt`). The words "BLE" and "Bluetooth": none
found in the BM787BT manual, 2026-09-26.

Neither protocol revision names a model. The body says "BM78xBT" throughout,
and "BM78x" once ("ID for BM78x Model series", r4 p.4); a search of both texts
for "78[0-9]" found nothing, 2026-09-26 [KNOWN]. "BM780" appears only in r2's
PDF title and file name [KNOWN].

| Where | Value | Tells apart | Source |
|---|---|---|---|
| Advertised name | `BM78xBT` by default, user-changeable (§2) | nothing | r4 p.1, p.5 |
| Manufacturer data | `42 4D 0B 00`: "BM", Model Series ID `0B`, Status `00` | the series | r4 p.1 |
| Command 0x0116 reply, Arg0 | `0B`, "ID for BM78x Model series" | the series | r4 p.4 |
| Info packet [5] | Device Category, `02` multimeter, `03` clamp-on meter | the category | r4 p.8 |
| Reading packet [4-7], [17] | data set ID `000001` "for BM78xBT", PK ID `01`, Device Type `01` meter | nothing | r4 p.10 |

A field that tells a BM788BT from a BM787BT: none found in r2, r4 or the app,
2026-09-26. The app holds no "BM78" string (`pp.txt`) and compares model IDs
only in its battery logic (900, 0x0B, 0x0D; DS:245) [VENDOR]. The two models
differ in which functions they have (§9.5), so the function IDs a meter sends
are the only hint [INFERRED].

The module is a Cypress/Infineon CYBLE-212006-01 (OTA p.1; BM788BT manual
p.31 (printed 30); its certifications p.23 (printed 22)) [KNOWN]. A module
name or radio certification: none found in the BM787BT manual (p.22, p.31
read), 2026-09-26.

## 2. Advertising and GATT

The default advertising packet, 20 bytes (r4 p.1; identical on r2 p.1)
[KNOWN]:

```
02 01 06   08 09 42 4D 37 38 78 42 54   07 FF 31 01 42 4D 0B 00
flags      name "BM78xBT"               manufacturer data
```

| Item | Value | Source | Tag |
|---|---|---|---|
| Flags | `06`, "General discoverable mode"; remark "Don't care" | r4 p.1 | [KNOWN]; LE General Discoverable with BR/EDR not supported [INFERRED from the Bluetooth Core Specification's AD format] |
| Name | `BM78xBT`, lower-case `x` (0x78), AD type `09`. "Remote Changeable"; 1-12 ASCII characters, length byte `02`-`0D`; set by 0x0142 (§7.1), restored by the factory reset (§9.2) | r4 p.1, p.5-6 | [KNOWN] |
| Name, app | the app refuses a new name longer than 11 characters (`ble_set_new_name.dart:64-69`) | app | [VENDOR]; r4 allows 12 (§11) |
| Manufacturer data | length `07`, type `FF`, then `31 01` ("Manufacturer Specific Data-1/-2"), `42 4D` ('B' 'M'), `0B` ("Model Series ID"), `00` ("Status"); "Fixed", except the type byte [13], "Don't care" | r4 p.1 | [KNOWN]; `31 01` is company identifier 0x0131, low byte first [INFERRED from the AD format; the GATT UUIDs also end `0131`] |
| Status byte | `00` is the only value printed; its meaning is not stated in r4 | r4 p.1 | [KNOWN] |
| Scan response | not stated in r4 | — | — |
| App's filter | no service-UUID filter, low-power scan (`ble_scan.dart:363-375`); keeps a device whose manufacturer data, company ID included, has at least 4 bytes and starts `31 01 42 4D`, and whose RSSI is −90 dBm or better (`ble_refresh_found_device_list.dart:42-44`, `:175-211`). The series byte, the status byte and the name are not checked | app | [VENDOR] |
| Service | `0003CDD0-0000-1000-8000-00805f9b0131` ("Unknown Service", "Primary Service") | r4 p.7; OTA p.1 | [KNOWN] |
| Readings | `0003CDD5-0000-1000-8000-00805f9b0131`, NOTIFY, descriptor 0x2902 (CCCD). "Length : 185 Bytes" is printed under the descriptor. "Meter sends measurements using notification" (OTA p.1) | r4 p.7 | [KNOWN]; that 185 describes the value, not the 2-byte CCCD [INFERRED] |
| Commands | `0003CDD4-0000-1000-8000-00805f9b0131`, "(Command Packet)", READ/WRITE, 32 bytes; "Application commands are sent using the UUID 0003cdd4-…" | r4 p.7; OTA p.1 | [KNOWN] |
| Indicate | not stated in r4 | — | — |
| MTU | "Caution: Attribute MTU size must be set to 185 bytes" (r4 p.7; again p.15; red in r2 p.6 and p.14). The app requests 185 after connecting (`ble_connect.dart:2826`; the value at `flutter_reactive_ble/src/reactive_ble.dart:2950-2951`). Community: see §12.3 D1. | r4, app | [KNOWN], [VENDOR] |
| Write type | not stated in r4. Every app write is with response; none without (search of `asm/brymen`, 2026-09-26) | app | [VENDOR]; write without response [UNVERIFIED] |
| Pairing | pairing, bonding, encryption: not stated in r4; pairing steps not stated in either manual; none in the app (no bond or pair call in `asm/brymen`, no "bond" in `libapp.strings`, 2026-09-26). Access is gated by the 0x0151 password (§3). Community: see §12.2. | r4 p.3, p.15; app | [KNOWN], [VENDOR] |
| Connection parameters | r4 proposes interval 100 ms, slave latency 25, supervision timeout 6000 ms (r4 p.15). The app sets none of them; it asks for high-performance priority only when a live chart starts (`ChannelSelectController.dart:458-460`) | r4, app | [KNOWN], [VENDOR] |
| Bootloader | `00060001-f8ce-11e4-abf4-0002a5d5c51b`, OTA only; which service holds it is not stated | OTA p.1 | [KNOWN] |

The UUIDs use the base `-0000-1000-8000-00805f9b0131`, not the Bluetooth SIG
base ending `00805f9b34fb` [KNOWN as printed; INFERRED comparison]. r4 prints
the first group in upper case, OTA p.1 in lower case. Whether the meter
advertises while connected, or accepts more than one connection: not stated
in r4 [UNVERIFIED]. Community:
see §12.3 D4, §12.4.

## 3. Bring-up

### 3.1 r4's flowchart

"Proposal design flowchart" (r4 p.15; r2 p.14 is the same but for footnote 2
and its red MTU line) [KNOWN]:

1. The meter advertises.
2. The central sets ATT MTU 185 and the connection parameters, scans and
   connects.
3. The central sends 0x0151 with the password (default 0000).
4. Wrong password: the meter answers `0x8001 + ArgN0151F` and the flow
   returns to step 3. Right password: `0x0151 + ArgN0151`.
5. "Central opens UUID Notification Channels".
6. "Upon UUID channels are opened successfully by Central, BM78xBT will
   output display reading periodically¹⁾ ²⁾ with the format (totally 152
   bytes)". Footnote 1: "Subject to Central's connection interval setting
   and/or BM78xBT's display reading update rate." Footnote 2 (r4 only): "In
   case of disconnection, Central may need to resend Command 0x0151 with
   Password to get the reconnection."

A start, stop or poll command: none found in r4's command table (p.3), 2026-09-26. Whether the
meter streams without 0x0151, or when notifications were enabled before it:
not stated in r4 [UNVERIFIED]. Community: see §12.2.

### 3.2 What Brymen's app does

[VENDOR], in order:

1. Connect with a 12 s timeout and no service list (`ble_connect.dart:531-543`);
   400 ms after the link is up, request MTU 185 (`:2556`, `:2826`), then
   discover services (`:2926`).
2. In CDD0, walk the characteristics in discovered order (DS:5137-6119):
   CDD5, if notifiable, is subscribed (DS:5451-5708); CDD4, if writable with
   response, starts steps 3-5. So whether notifications are on before the
   password depends on the meter's GATT order [UNVERIFIED]; the app does not
   follow r4's order.
3. **0x0101 GetBLEAddress** with the MAC bytes zero
   (`ble_get_mac_address.dart:78-111`): write CDD4, wait 300 ms, read CDD4
   (`:137-158`). The reply's [5..10] become the device's MAC, but only if the
   reply's [3] is `02` (BC:5181-5245); otherwise login stops with "Failed to
   get MAC address, cannot send login packet" (`ble_send_login_cmd.dart:103-159`).
   0x0101 is not in r4, which says the address "can be got from any Response
   Packet" (r4 p.2).
4. **0x0151** with the stored password (default "0000"; encoding §7.2).
   Success is a reply whose [12] is not `80`; the echoed password is not
   checked. On failure the app shows an error and does not retry
   (`ble_send_login_cmd.dart:189-292`).
5. **0x0106 GetDeviceActiveState**; unless its reply's Arg0 is 3, **0x0021
   GetOffsetRelativeAdjustment** once per channel (`ble_init_info.dart:10-316`).
   Neither is in r4 (§7.3).
6. Listen on CDD5. Nothing is written periodically: the app's 100 s
   keep-alive task only refreshes its own scan status
   (`ble_keep_alive_task_handler.dart`; `ble_service.dart:1308-1372`).
7. At the start of each live-chart session, **0x0010** sets the meter's clock
   (§7.1).

On a disconnect the app waits a user-set delay, scans again and repeats steps
1-5, 0x0101 and 0x0151 included (`ble_listenner_disconnect.dart:179-1163`),
as r4's footnote 2 asks [VENDOR].

**Response channel.** Whether a response is read from CDD4 or notified on
CDD5: not stated in r4. Its flowchart has the 0x0151 response before the
notification channels are open (r4 p.15), which fits a read of CDD4
[INFERRED]. The app writes CDD4 and reads the response from CDD4, with a
3 s timeout (`ble_send_package_universal.dart:162-201`), and hands CDD5
notifications only to its reading dispatcher (DS:77-162) [VENDOR]. Whether the
meter also notifies responses is [UNVERIFIED]. Community: see §12.2.

**Rate.** No figure in r4 (footnote 1 above) [KNOWN]. The display updates at
most 5 times a second and the bar graph at most 50 (BM788BT manual p.22
(printed 21)); REC mode has its own rates (§9.4) [KNOWN]. The notification
rate is [UNVERIFIED]. Community: see §12.4.

## 4. Framing

| Rule | Source | Tag |
|---|---|---|
| A framed packet starts `FF` ("HEAD"), then `01` ("SOH") in the command, response and information packets, and `02` in the reading packet, whose remark still says "SOH"; r2 is the same | r4 p.2, p.8, p.10; r2 p.9 | [KNOWN]; the label against the value [UNVERIFIED]. Community: see §12.2. |
| [2] is the packet length, `20` (32) or `18` (24) | r4 p.2, p.8, p.10 | [KNOWN] |
| [3] is the packet type: `01` command, `02` response, `04` device information, `05` device reading | r4 p.2, p.8, p.10 | [KNOWN]; the app's table adds 3 Notification, 6 LoggerData, 7 IRImagePacket, 8 FilenameListpacket, 9 Stringpacket, 16 FFTpacket (BC:14340-14464) [VENDOR] |
| A framed packet ends `FF 03` ("HEAD", "ETX") | r4 p.2, p.8, p.10 | [KNOWN] |
| The two bytes before the end are a CRC of [2] up to the byte before them: [2..27] in a 32-byte packet, [2..19] in the information packet | r4 p.2, p.8, p.10 | [KNOWN] |
| The CRC routine is printed in C (r4 p.2-3, again p.8-9 and p.10-11): start 0xFFFF; XOR in each byte, then 8 times shift right, XOR-ing 0xA001 when bit 0 was set; no final XOR. "CRC-16 reverse algorithm based on the polynomial x16+x15+x2+1 (0x8005)" | r4 | [KNOWN]; that is CRC-16/MODBUS, check value 0x4B37 over "123456789" [INFERRED, computed] |
| CRC byte order: not stated in r4 or r2, and neither has a complete frame to test it. The app writes and compares the low byte at Checksum0, the high at Checksum1 (`bleBase.dart:58-218`; CL:5053-5097; `checkSum.dart:62-104`). Community: see §12.2. | app | [VENDOR] |
| Multi-byte fields are little-endian: the reading ("Device Reading [2] ~ [0]", [21] = Reading0) and the data set ID (`000001`, [4] = `01`) | r4 p.10 | [INFERRED from the index order and the examples]; the app reads the count so, and the information packet's [16..18] (BC:6654-6733, :5492-5557) [VENDOR] |
| No sequence counter; the RTC (§6.8) is the only field that changes with every sample | r4 p.10 | [INFERRED from the layout] |
| No escaping: `FF` can occur inside a packet (CRC, count, RTC) | — | [INFERRED from the fixed layouts] |

**The 152-byte output** (r4 p.8, p.15) [KNOWN]: one Device Information Packet
(24 bytes), one Device Reading Packet (32 bytes), then three Device Reading
Packets "(0x00 for all 32 bytes)" — no head, no CRC, no ETX. r4 counts the
zero blocks among the "Four Device Reading Packet (32 bytes x4)" (p.15).

At ATT MTU 185 a notification carries up to 182 bytes, so the output fits in
one [INFERRED from the 3-byte ATT notification header, Bluetooth Core
Specification]. How the meter sends it at a smaller MTU is [UNVERIFIED].

The app decodes each notification on its own, with no reassembly (DS:77;
BC:4921-5100): it reads a packet, advances by its [2], and stops at a block
whose [0] is not `FF` or when fewer than 3 bytes remain. It so gets the
information packet, the reading packet and one zero block. For the zero
block, [3] looks up as packet type "None" and parsing is skipped
(BC:5162-5245); its channel stays −1 (BC:9849-9851); the dispatcher ignores
it (DS:128-162) [VENDOR]. The decoder checks neither [1] nor the end
bytes. It computes a CRC mismatch flag per packet (BC:4953-4961)
that nothing reads (search of `asm/brymen`, 2026-09-26) [VENDOR]; that check
takes [2..27] and [28..29] of the remaining buffer even for the 24-byte
information packet, so there it spans into the reading packet [INFERRED from
the offsets]. Responses read from CDD4 get no CRC check (BP:470-1001)
[VENDOR].

## 5. Command and response packet

32 bytes, both directions (r4 p.2; identical on r2 p.2) [KNOWN]; the last
column is what the app writes (CL:4526-5280) [VENDOR].

| Byte | Name (r4) | r4 value, remark | App |
|---|---|---|---|
| 0 | HeadByte0 | `FF` HEAD | `FF` |
| 1 | HeadByte1 | `01` SOH | `01` |
| 2 | Packet Length | `20` | `20` |
| 3 | Packet Type | `01` Command, `02` Response | `01` |
| 4 | Protocol Version | `01` | `01` |
| 5-10 | MAC address0-5 | "BLE Device Address"; "BLE Device address can be got from any Response Packet" | zeros in 0x0101; afterwards a reply's [5..10], in received order (CL:4603-4665; BC:5252-5268) |
| 11 | Command0 | see §7 | low byte of the code |
| 12 | Command1 | | high byte |
| 13 | Password Identification | `01`; meaning not stated | `01` or `00` by command (below) |
| 14-27 | Arg0-Arg13 | per command (§7) | |
| 28-29 | Checksum0-1 | "CRC of [2]~[27]" | low byte, high byte |
| 30-31 | EndByte0-1 | `FF 03` HEAD, ETX | `FF 03` |

- **Code byte order.** r4 writes codes as `[Command1:Command0]` (r4 p.2-3),
  which puts the low byte in [11] [INFERRED from the notation]. The app does
  so: 0x0151 goes out as `51 01` (typeMap, BC:1008-4920), and it recognises
  0x8001 by [12] = `80`, [11] = `01` (BP:519-588) [VENDOR].
- **MAC order.** The app shows an address by reversing [5..10] (and the
  information packet's [6..11]) (BC:14469-14526, :5402-5430), so the wire
  carries the least significant octet first [INFERRED: the reversing helper is
  unnamed in the listing and taken to be `reversed` from its use]. r4 states
  neither the order, nor what goes in [5..10] before any response, nor whether
  the meter checks them. Community: see §12.2.
- **[13].** r4 prints `01` [KNOWN]. The app sends `01` with 0x0101, 0x0004,
  0x0005, 0x0010, 0x0116, 0x0140 and 0x0151 (e.g.
  `0x0151_VerifyAccountPassword.dart:66-70`, `ble_get_mac_address.dart:122-124`)
  and leaves `00` in 0x0021, 0x0040, 0x0106 and 0x0142 (no store found)
  [VENDOR]. Whether the meter checks it is [UNVERIFIED]. Community: see
  §12.2.
- **Responses** have this layout with [3] = `02` (r4 p.2) [KNOWN]. The app
  reads [11]-[12], the arguments, [5..10] and, on the 0x0101 path only, [3];
  not [0], [1] or the CRC (BP:470-1001, :1365-1534) [VENDOR].

---

## 6. Reading output

### 6.1 Device Information Packet, 24 bytes

r4 p.8 (identical on r2 p.7) [KNOWN]; the app's reading from BC:5337-5574
[VENDOR].

| Byte | Name (r4) | r4 value | r4 remark | App |
|---|---|---|---|---|
| 0-1 | HeadByte0-1 | `FF 01` | HEAD, SOH | not checked |
| 2 | Packet Length | `18` | | stride |
| 3 | Packet Type | `04` | 0x04 Device Information; 0x05 Device Reading | dispatch |
| 4 | Protocol Version | `01` | | |
| 5 | Device Category ID | `02` | 0x02 Multimeter; 0x03 Clamp-on meter | looked up in a table, 0 NULL, 1 Environmental instrument, 2 Digital MultiMeter, 3 Clamp meter, 4 Power Meter (BC:14265-14339); the result is unused and the MAC is always parsed (BC:5387-5430) |
| 6-11 | MAC address0-5 | | Bluetooth Device Address | reversed for display (BC:5402-5430) |
| 12 | Device Battery Status | `00` | "If 0x02, Device is at Low Battery status" | §6.9 |
| 13 | Power Source Flag | `00` | (none) | never read |
| 14-15 | Reserve1-2 | `00 00` | | |
| 16-18 | Reading Packet No.1-3 | `04 00 00` | "The number of "Device Reading Packet" will be transmitted out after "Device Information Packet"" | one 24-bit little-endian count (BC:5492-5557) |
| 19 | Device Reading PK No. | `01` | "0x01 for BM78xBT (Single Display Device)" | number of channels: entries 1 to [19] are created (DS:505-560) |
| 20-21 | Checksum0-1 | | "CRC calculation of Index [2]~[19] bytes" | |
| 22-23 | EndByte0-1 | `FF 03` | HEAD, ETX | |

4 is the four 32-byte blocks after the packet, zero blocks included: 24 + 4 ×
32 = 152 [INFERRED from r4 p.8 and p.15]. Whether [16..18] is one count or
three fields is not stated.

### 6.2 Device Reading Packet, 32 bytes

r4 p.10 (identical on r2 p.9) [KNOWN]; the app's reading from BC:5575-7417
[VENDOR].

| Byte | Name (r4) | r4 value, remark | App |
|---|---|---|---|
| 0-1 | HeadByte0-1 | `FF 02`, "HEAD", "SOH" | not checked |
| 2 | Packet Length | `20` | stride |
| 3 | Packet Type | `05` | dispatch |
| 4-6 | Logging Data set ID1-3 | `01 00 00`, "0x000001 for BM78xBT" | not used for readings (BC:5581-5610) |
| 7 | Device Reading PK ID | `01`, "0x01 only for BM78xBT (Single Display Device)" | channel [7] (BC:5620-5635) |
| 8-13 | Device RTC Time | §6.8 | |
| 14-16 | Device Status Flag0-2 | §6.6 | one 24-bit little-endian word (BC:6178-6289) |
| 17 | Device Type | `01`, "0: Sensor; 1: Meter" | selects the name tables (§6.5) |
| 18 | Main-Function ID | §6.5 | |
| 19 | Reserved | `00` | read, not used |
| 20 | Sub-Function ID | §6.5 | |
| 21-23 | Device Reading0-2 | "Device Reading [2] ~ [0]: Signed bytes e.g.: 0x008000 = 32768 0xFF8000 = -32768" | §6.3 |
| 24 | Reading Decimal Point | §6.3 | |
| 25 | Metrics Prefix | "-9="n"; -6="μ"; -3="m"; 0=" "; 3="k", 6="M", 9="G"" | §6.3 |
| 26 | Function Unit | §6.4 | |
| 27 | Display Digit Number | "3:XXX; 4:"XXXX"; 5:"XXXXX"; 6:"XXXXXX"" | |
| 28-29 | Checksum0-1 | "CRC calculation of Index [2]~[27] bytes" | |
| 30-31 | EndByte0-1 | `FF 03` | |

### 6.3 Value, decimal point, prefix, sign

- **Count.** [21] low to [23] high, 24-bit two's complement [INFERRED from
  "[2] ~ [0]" and the two examples, which on the wire are `00 80 00` and
  `00 80 FF`]; the app reads it so (BC:6654-6733) [VENDOR].
- **Decimal point [24]**, by [27] (r4 p.13) [KNOWN]:

  | [27] | [24] = 0 | 1 | 2 | 3 | 4 |
  |---|---|---|---|---|---|
  | 5 | XXXXX | X.XXXX | XX.XXX | XXX.XX | XXXX.X |
  | 4 | XXXX | X.XXX | XX.XX | XXX.X | — |

  So [24] counts the digits before the point, 0 meaning no point, and the
  digits after it are [27] − [24] when [24] is not 0, none when it is
  [INFERRED from the tables]. The app divides
  the count by 10^([27] − [24]) when [24] is not 0 (BC:7043-7255) [VENDOR].
  Tables for 3 and 6 digits: none found in r4 (p.13), 2026-09-26; the app applies the same rule
  to any [27] [VENDOR].
- **Prefix [25]** is a power of ten, −9 to 9 in steps of 3 (r4 p.10)
  [KNOWN]; its byte encoding is not stated in r4. The app reads a signed byte
  (so −3 is `FD`) and also knows 12 = "T" (BC:6580-6616, :11232-11332); it
  shows the prefix in the unit string and does not scale the number by it
  (BC:7273-7408) [VENDOR]. The value is then count × 10^−([27] − [24]) when
  [24] is not 0, the count itself when [24] is 0, in both cases × 10^[25] in
  unit [26] [INFERRED].
- **Digits.** The display is "4-5/6 digits 60000 counts" (BM788BT manual p.4
  (printed 3); BM787BT manual p.4 (printed 3)) [KNOWN]. Which [27], [24] and
  [25] a given range sends is not stated in r4 [UNVERIFIED].
- **Sign.** The count is signed (r4 p.10), and Flag1 bit 6 is "Sign", set
  when the "Display reading is Negative" (r4 p.12) [KNOWN]. r4 does not say
  whether both are always set together. The app shows the magnitude and takes
  the sign from the flag (BC:20-571, :15072-15313) [VENDOR]. Whether the meter
  sends a negative count, a magnitude with the flag, or both is [UNVERIFIED].
  Community: see §12.3 D2.
- **Overload.** Flag1 bit 5 "OL": "When Bit5=1, it means Display shows OL.
  Device Reading [2] ~ [0] can be ignored." (r4 p.12) [KNOWN]. The app shows
  "OL" or "-OL" by the sign flag, and for temperature (main 0x0C sub 0 or 1,
  or unit °C/°F) "-----" or "----" instead (BC:92-350) [VENDOR]. The manual
  draws over-range as `.0L` (BM788BT manual p.11 (printed 10)) [KNOWN].
  Community: see §12.4.

### 6.4 Function Unit [26]

r4 p.13 [KNOWN] against the app's `UnitIDTable` (BC:10235-11231) [VENDOR].

| Code | r4 | App |
|---|---|---|
| `02` | V, Volt | V |
| `03` | A, Amp | A |
| `04` | Ω, Ohm | Ω |
| `05` | S, Siemens (℧) | S |
| `06` | F, Farad | F |
| `08` | Hz, Hertz | Hz |
| `0A` | %, Duty | % |
| `14` | °C, Celsius | °C |
| `15` | °F, Fahrenheit | °F |
| `4F` | %4~20mA, Current loop | empty string: no entry |

The app has further codes r4 lacks: `07` H, `09` RPM, `0B`-`0F` W, VA, J,
kWh, mVAh, `11`-`13` dB, dBm, sec, `16`-`1A` °K, %RH, PSI, Pa, bar, `30`-`35`
dB, %, %, sec, min, Hr, `45` oz, `46` lbs, `4E` VAR; every other code is an
empty string [VENDOR]. r4's ten codes are the units the manuals' functions
need (§9.5) [INFERRED]. The app names main 0x06 sub 0x08 "%4-20mA" but has no
unit for `4F`; what the meter sends there is [UNVERIFIED].

### 6.5 Function IDs

r4 p.14 [KNOWN] against the app's meter tables, `FunctoinMainNameTable` and
`FunctoinSubNameTable` for Device Type 1 (BC:11333-14264) [VENDOR]. ~~Struck~~
rows are struck through in r4 p.14; in r2 p.13 the same four rows are red and
struck, and the 0x23 row is red and not struck [KNOWN].

| Main | r4 type | Sub | r4 display function | App |
|---|---|---|---|---|
| `02` | AutoCheck | `00` | LoZ-ACV | ACV-LoZ ("Auot Check") |
| | | `01` | LoZ-DCV | DCV-LoZ |
| | | `02` | — | OHM (BC:12139-12161) |
| | | `03` | AUTO | — (no sub 3) |
| `03` | Volt | `00`-`02` | ACV, DCV, DC+ACV | ACV, DCV, ACV+DCV |
| | | `03` | ~~Hz of Line Volt~~ | HZ |
| | | `04`-`08` | — | VFD, LPF, THD%, DF%, V_FFT |
| `17` | VFD | `00` | Hz of VFD-ACV | VFD Hz |
| | | `01` | VFD-ACV | VFD ACV |
| | | `02`-`04` | — | ACA, Peak Voltage, Peak Current |
| `04` | mV | `00`-`02` | ACmV, DCmV, DC+ACmV | ACmV, DCmV, ACmV+DCmV |
| | | `03` | — | HZ |
| `05` | μA | `00`-`02` | ACμA, DCμA, DC+ACμA | ACuA, DCuA, "ACuA+ DCuA" |
| | | `03` | ~~Hz of μA~~ | HZ |
| | | `04`-`06` | — | LPF, Inrush, I_FFT |
| `06` | mA | `00`-`02` | ACmA, DCmA, DC+ACmA | ACmA, DCmA, "ACmA+ DCmA" |
| | | `03` | ~~Hz of mA~~ | HZ |
| | | `04`-`07` | — | LPF, AC+DC Inrush, Inrush, I_FFT |
| | | `08` | %4~20mA | %4-20mA |
| `07` | A | `00`-`02` | ACA, DCA, DC+ACA | ACA, DCA, "ACA+ DCA" |
| | | `03` | ~~Hz of A~~ | HZ |
| | | `04`-`06` | — | LPF, Inrush, I_FFT |
| `0C` | Temperature | `00`-`02` | T1, T2, T1 - T2 | T1, T2, T1-T2 |
| `0D` | Resistance | `00` | Resistance | Resistance |
| `0E` | Capacitance | `00` | Capacitance | Capacitance |
| `0F` | Continuity | `00` | Continuity | Continuity |
| `10` | Diode | `00` | Diode | Diode |
| `11` | nS Conductance | `00` | nS Conductance | nS Conductance |
| `12` | Duty Cycle (%) | `00` | Duty Cycle (%) | Duty Cycle |
| `13` | Logic-Hz | `00` | Logic-Hz | Logic-Hz |
| `22` | EF | `00`, `01` | EF-Lo, EF-Hi | EF-L, EF-H (BC:13985-14012) |
| `23` | Hz of Line signal | `00` | Hz of Line Volt/Current | "Generic Hz", no sub table |

- **Line frequency.** The red marks in r2 read as one edit that struck the
  four "Hz of …" subs and added main 0x23 [INFERRED from the colour]. The app
  still names sub 3 "HZ" in those mains and adds it under mV [VENDOR]. Which
  the meter sends is [UNVERIFIED].
- **AutoCheck.** r4 has sub `03` AUTO and no `02`; the app has `02` OHM and
  no `03` [KNOWN], [VENDOR] — [UNVERIFIED].
- The app's tables cover Brymen's other products too: mains `00` None, `01`
  Other, `08`-`0B` Clamp, Tip Clamp, Flex, dBm, `14` Insulation Resistance,
  `16` Power, `18`-`21` and `24`, and a sensor table for Device Type 0
  [VENDOR]. Main IDs `00`, `01`, `08`-`0B`, `14`, `16`, `18`-`21` and `24`
  are not in r4.
- On a change of (main, sub) or of °C/°F the app clears that channel's chart,
  unless Flag0 bit 0 is set (DS:1534-1830, :2108-2320) [VENDOR].

### 6.6 Status flags [14]-[16]

r4 p.12 [KNOWN]. The app reads the three bytes as one word and tests bit
(enum index + 1) (`statusFlag`, BC:698-1007) [VENDOR]; its names line up as
below [INFERRED: the jump table that pairs enum and mask is not in the
listing].

| Byte | Bit | r4 name | r4: set means | App |
|---|---|---|---|---|
| Flag0 [14] | 7 | CREST | CREST mode activated | CREST |
| | 6 | RELative | REL mode activated | REL |
| | 5 | HOLD | reading in HOLD status | HOLD |
| | 4 | AUTO-Ranging | auto-ranging (clear: manual) | Auto_range |
| | 3 | AUTO-HOLD | AUTO-HOLD mode activated | Auto_HOLD |
| | 2 | ASCII reading | display is not a numerical reading (§6.7) | ASCIIReading |
| | 1 | x | Don't care | unnamed enum 0 |
| | 0 | x | Don't care | read alone: when set, the function-change handling is skipped (DS:2115-2118, :2238-2239) |
| Flag1 [15] | 7 | x | Don't care | **TestLead** |
| | 6 | Sign | reading is negative | Negative |
| | 5 | OL ¹⁾ | display shows OL | OL |
| | 4 | RECORD | Record mode activated | Record |
| | 3 | MAX ²⁾ | LCD "MAX" on | MAX |
| | 2 | MIN ²⁾ | LCD "MIN" on | MIN |
| | 1 | AVG ³⁾ | LCD "AVG" on | AVG |
| | 0 | x | Don't care | unnamed enum 7 |
| Flag2 [16] | 7-0 | x | "Don't care" for 0 and 1 | bits 0-3 unnamed enums 15-18; [16] shifted right 2 is read by the single-step-record page (BC:6758-6774) |

Footnotes (r4 p.12): ¹⁾ as quoted in §6.3; ²⁾ "Accompany with RECORD or CREST
mode"; ³⁾ "Accompany with RECORD mode". The manual's REC key steps MAX → MIN →
AVG → all three, and CREST shows "MAX" and toggles CMAX/CMIN (BM788BT manual
p.18 (printed 17)) [KNOWN]; that the flags follow those annunciators is
[INFERRED from the names]. Flag0 bit 0, Flag1 bit 7 and Flag2 in the meter's
packets are [UNVERIFIED].

A field for AC/DC (the sub-function carries it), the bar graph, the beeper,
the Bluetooth annunciator or low battery (§6.9): none found in r4 p.10-12, 2026-09-26
[INFERRED from the layout].

### 6.7 ASCII readings

When Flag0 bit 2 is set, the count selects a display text (r4 p.12-13)
[KNOWN]. The app looks values below 16 up in its own table and shows an empty
string above (BC:572-697) [VENDOR].

| Count | r4 "Meter is displaying" | App | Manual |
|---|---|---|---|
| `000000` | — | OL | |
| `000001` | "Auto" | Auto | AutoV armed, no input (BM788BT manual p.5 (printed 4)) |
| `000002` | "InEr" | InEr | wrong-jack warning (p.18 (printed 17)) |
| `000003`-`000007` | "-", "- -", "- - -", "- - - -", "- - - - -" | -, --, ---, ----, ----- | EF field strength, "a series of bar-graph segments", drawn as dashes (p.15-16 (printed 14-15)); "- - - - -" also AutoHold waiting (p.17 (printed 16)) |
| `000008` | — | diSC | |
| `000009` | — | CALi | |
| `00000A` | "EF-H" | EF-H | EF detection (p.15 (printed 14)) |
| `00000B` | "EF-L" | EF-L | EF detection (p.15 (printed 14)) |
| `00000C`-`00000F` | — | rS-3, SoC, SoH, bAd | |

The manual column matches the texts by wording [INFERRED].

### 6.8 RTC [8]-[13]

r4 p.12 [KNOWN]:

| Bytes | Bits | Field |
|---|---|---|
| [13]-[12] as one word, [13] high | 15-9 | year, "2000+1~127" |
| | 8-5 | month 1-12 |
| | 4-0 | date 1-31 |
| [11]-[8] as one word, [11] high | 31-27 | printed `0 0 0 0 0` |
| | 26-22 | hour, printed "1~23" |
| | 21-16 | minute 0-59 |
| | 15-10 | second 0-59 |
| | 9-0 | "Mini-Second" 0-999 |

The app reads the six bytes as one little-endian 48-bit value and extracts
the same fields (BC:5642-6164, :6777-6798) [VENDOR]. The hour range "1~23"
differs from command 0x0010's 0-23 (r4 p.4) and the year's 1-127 from its
0-99 [KNOWN]. What the RTC holds before the first 0x0010, and across
power-off, is not stated in r4 [UNVERIFIED]. Community: see §12.4.

### 6.9 Battery

Low battery is reported only in the information packet: [12] = `02` "Device
is at Low Battery status" (r4 p.8) [KNOWN]; other values are not stated. The
reading packet has no battery bit (r4 p.10-12) [KNOWN]. The app honours only
bit 1 of [12] once it knows the model is `0B` or `0D` from a 0x0116 reply;
before that (model 900 by default) it ignores the byte, and for other models
it reads a four-level scale from bits 4-6 (DS:241-440) [VENDOR]. The meter's
own threshold is "Below approx. 3.7V" (BM788BT manual p.23 (printed 22))
[KNOWN].

---

## 7. Commands

### 7.1 r4's command set

The command table (r4 p.3) and arguments (r4 p.3-5) [KNOWN]. A success
response echoes the code with the arguments below; every failure is 0x8001
(§8). Unlisted arguments are `00`.

| Code | Name (r4) | Command arguments | Success reply arguments |
|---|---|---|---|
| 0x0004 | Get BLE firmware version | — | Arg[2:0] "BLE Firmware version ID" |
| 0x0010 | RTC time calibration | Arg0 second 0-59, Arg1 minute 0-59, Arg2 hour 0-23, Arg3 date 1-31, Arg4 day 1-7, Arg5 month 1-12, Arg6 year 0-99 (2000-2099) | the same, echoed |
| 0x0040 | Set BLE to OTA standby mode (r4 only) | Arg0 = `01` | Arg0 "0x00 or 0x01" |
| 0x0116 | Get Model Series ID | — | Arg0 = `0B` "ID for BM78x Model series" |
| 0x0140 | Set Connection Password | Arg0-3 new password | Arg0-3 new password |
| 0x0141 | Get Connection Password | — | Arg0-3 password |
| 0x0142 | Set Device Name | Arg0-11 new name | Arg0-11 new name |
| 0x0143 | Get Device Name | — | Arg0-11 name |
| 0x0151 | Verify Connection Password | Arg0-3 password; "Default Connection Password: 0000" | Arg0-3 password |

- **0x0004.** "If Arg[2:0] = [0x00, 0x01, 0x11], BLE Firmware version is
  0.1.17" (r4 p.3; the example is not in r2) [KNOWN]. The app prints
  "Arg2.Arg1.Arg0" (`0x0004_GetBleFirmwareVersion.dart:10-102`), which reads
  the example's list as [Arg2, Arg1, Arg0] and agrees [VENDOR].
- **0x0010.** Binary or BCD is not stated in r4. The app sends binary
  values, weekday Monday = 1 to Sunday = 7, year − 2000
  (`set_rtc.dart:95-353`), at the start of each live-chart session
  (`ChannelSelectController.dart:485`) [VENDOR]. Community: see §12.2,
  §12.4.
- **0x0040** refers to the "BM78xBT OTA Programming file" (r4 p.4). There,
  after 0x0004 and 0x0116, 0x0040 makes the meter advertise again; the host
  reconnects without resending the password and runs Cypress's bootloader
  protocol on the bootloader characteristic (OTA p.1-5) [KNOWN]. r2 has no
  0x0040 [KNOWN].
- **0x0142.** The padding of a name shorter than 12 is not stated. The app
  sends the name's code points and refuses more than 11 characters
  (`ble_set_new_name.dart:64-69`) [VENDOR].
- **0x0141, 0x0143**: a call site in the app: none found in `asm/brymen`,
  2026-09-26.

### 7.2 Password encoding

The default is "0000" (r4 p.3, p.6; BM788BT manual p.20 (printed 19)) [KNOWN];
whether it goes as ASCII or binary is not stated in r4 or r2. The app sends
decimal digits as binary values 0-9, but in two orders [VENDOR]:

- **0x0151** reverses them: "1234" is Arg0-3 = `04 03 02 01`
  (`0x0151_VerifyAccountPassword.dart:42-133`);
- **0x0140** keeps them in order: `01 02 03 04`
  (`ble_set_new_password.dart:160-211`).

The default is `00 00 00 00` either way. The meter's encoding and order are
[UNVERIFIED]. Community: see §12.2, §12.3 D3.

### 7.3 Sent by the app, not in r4

[VENDOR]; how a BM78xBT answers each is [UNVERIFIED].

| Code | App name | When | Arguments |
|---|---|---|---|
| 0x0101 | GetBLEAddress | every connect, before 0x0151 | none, MAC zero (§3.2) |
| 0x0106 | GetDeviceActiveState | after 0x0151 | none; reply Arg0 kept as the device mode (`0x0106_GetDeviceActiveState.dart:64-79`) |
| 0x0021 | GetOffsetRelativeAdjustment | after 0x0106, per channel, unless the mode is 3 | Arg0 channel, Arg1-2 a high/low selector (`0x0021_GetOffsetRelativeAdjustment.dart:408-519`) |
| 0x0005 | GetMCUfirmwareVersion | Settings "check info", after 0x0004 and 0x0116 (`settings/controller.dart:246-287`) | none; reply read as Arg2.Arg1.Arg0 (`0x0005_GetMcuFirmwareVersion.dart:45-95`) |

The app's code table (BC:1008-4920) names further codes from 0x0000 to
0x0701 (button presses, data logging and power-meter modes among them); none
is in r4 [VENDOR].

## 8. Responses and error codes

A failure response carries code 0x8001, Arg[1:0] the failed code, Arg[3:2]
the error code, Arg4-13 `00` (r4 p.3-5) [KNOWN]. The app matches the failed
code as Arg0 low, Arg1 high, takes the error number from Arg2 and uses Arg3 ≠
0 to pick a second table (BP:666-990) [VENDOR]; that Arg3 is the high byte of
a 16-bit code is [INFERRED].

| Code | r4 p.6 | App, Arg3 = 0 | App, Arg3 ≠ 0 |
|---|---|---|---|
| 0 | Checksum error | Checksum error | File does not exist. |
| 1 | Invalid channel ID | Invalid channel ID. | Unable to delete file. |
| 2 | Out of setting range | Out of setting range. | Unable to open file. |
| 3 | invalid password | Invalid connection password. | Battery low, unable to start data logger. |
| 4 | invalid password | Invalid administrator password. | The number of files exceeds 1000 |
| 5 | invalid arguments | Invalid arguments. | Command timeout. |
| 6 | Insufficient permissions | Insufficient permissions. | The required parameters are currently unavailable. |
| 7-9 | — | Insufficient memory.; SD card is not inserted.; Device data logger recording now. | — |

App texts from BP:1592-1992 [VENDOR]; a merged table adds 17 "This feature is
not available." r2 ends the list with "…… to be continued" (r2 p.5); r4 drops
that line (r4 p.6) [KNOWN]. The app also knows 0x8000 "Invalid command"
([12] = `80`, [11] = `00`, BP:555-556), which r4 does not list [VENDOR].

---

## 9. Meter behaviour (manuals)

The manuals' figures show "representative model(s)" (BM788BT manual p.3
(printed 2); BM787BT manual p.3 (printed 2)) with the BM789's dBm labels
[KNOWN]. A BM788BT or BM787BT dial drawing: none found in either manual,
2026-09-26.

### 9.1 Bluetooth on and off

Hold the ((D)) (Δ) button for one second or more to enable it, and again to
disable it; the LCD's ((D)) annunciator shows it is on (BM788BT manual p.19
(printed 18); BM787BT manual p.19 (printed 18)) [KNOWN]. The BM788BT manual
says disabling will "extend battery power", the BM787BT manual "save battery
power, if not in need" [KNOWN]. Enabling it adds "2mA typical" to the supply current
(BM788BT manual p.23 (printed 22)) [KNOWN]. In AutoV "only HOLD, AutoHold, EF and Backlight" work
(p.5 (printed 4)), which read literally excludes the Δ long press there
[INFERRED]. Whether the Bluetooth state survives power-off is not stated in
either manual [UNVERIFIED].

### 9.2 Factory reset of the password and name

"Press-and-hold the Hz button and then turn Rotary Switch from OFF to
Capacitance function position within 0.6 second can restore original factory
settings for wireless communication Connection Password (Default: 0000) &
Device Name. Meter will display "Org" shortly to confirm." (r4 p.5-6)
[KNOWN]. The BM788BT manual gives the same gesture, "(Model 788BT only)", with
the defaults 0000 and BM78xBT (p.20 (printed 19)). The BM787BT manual gives
it with no model restriction and no default values (p.19 (printed 18))
[KNOWN]; the BM787BT's defaults are [UNVERIFIED].

### 9.3 Auto power-off

The operation text says "approximately 30 minutes" (BM788BT manual p.18
(printed 17); BM787BT manual p.18-19 (printed 17-18)); the specifications say
"APO Timing: Idle for 15 minutes" (BM788BT manual p.23 (printed 22); BM787BT
manual p.22 (printed 21)) [KNOWN]. APO is off in REC and CREST, or for a
session when SELECT is held at power-on ("dSAPO"); Δ held at power-on
shortens it to about 8 s; SELECT wakes the meter (BM788BT manual p.18-19
(printed 17-18)) [KNOWN]. How a Bluetooth connection affects APO, and what
the link does at APO: not stated in either manual [UNVERIFIED].

### 9.4 Update rates

Digits at most 5 a second nominal, the 31-segment bar graph at most 50
(BM788BT manual p.22 (printed 21); BM787BT manual p.22 (printed 21)) [KNOWN].
In REC (BM788BT manual p.30 (printed 29)): DC 10/s; AC 5/s, and 10/s "For
Models 788BT & 789 only" (the BM787BT manual reads "For Model 789 only", p.30
(printed 29)); VFD 5/s; DC+AC 1/s; nS 1/s; capacitance by value; Hz and
T1-T2 2/s; Ω, T1, T2 and others 5/s [KNOWN]. How these relate to the
notification rate is [UNVERIFIED]. Community: see §12.4.

### 9.5 Display and functions per model

One main display, "4-5/6 digits 60000 counts", and a 31-segment analog bar
graph (BM788BT manual p.4, p.22 (printed 3, 21)) [KNOWN]; no secondary
display [INFERRED from the LCD drawing, p.4 (printed 3)]. r4 has no
bar-graph field, so the bar graph (which in ~Hz shows the trigger level, p.7
figures) is not transmitted [INFERRED from r4 p.10-12].

| Function (manual) | Position | BM788BT | BM787BT | r4 Main/Sub [INFERRED by name] |
|---|---|---|---|---|
| AutoV LoZ, AC or DC chosen automatically | Auto V / LoZ | yes | yes | `02`/`00`, `02`/`01`; "Auto" with no input = ASCII 1 |
| ACV, VFD-ACV | V~ | yes | yes | `03`/`00`, `17`/`01` |
| DCV, DC+ACV | V⎓ | yes | yes | `03`/`01`, `03`/`02` |
| DCmV, ACmV, DC+ACmV | mV | yes | yes | `04`/`01`, `04`/`00`, `04`/`02` |
| logic Hz, duty % | mV | yes | yes | `13`/`00`, `12`/`00` |
| Ω, continuity, nS | Ω | yes | yes ¹⁾ | `0D`, `0F`, `11` /`00` |
| capacitance, diode | capacitance | yes | yes | `0E`, `10` /`00` |
| temperature T1 | T1/T2 | yes | yes | `0C`/`00` |
| temperature T2, T1-T2 | T1/T2 | yes | **no** | `0C`/`01`, `0C`/`02` |
| A, mA, µA: DC, AC, DC+AC | A/mA, µA | yes | yes | `07`, `06`, `05` / `01`, `00`, `02` |
| %4-20mA | mA | yes | **no** | `06`/`08`, unit `4F` |
| line Hz | ~Hz key in V, VFD, current | yes | yes | `23`/`00` or a struck `03` sub (§6.5); `17`/`00` in VFD |
| EF-H, EF-L | EF key | yes | yes | `22`/`01`, `22`/`00`; ASCII `0A`, `0B`, dashes |
| dBm | V~, mV | no (BM789 only) | no | none |

Sources: BM788BT manual p.5-15 (printed 4-14) and p.23 (printed 22); BM787BT
manual p.12 (printed 11), "Temperature T1 (N/A for Model 785) & T2 (Models
789 & 788BT only)", and p.15 (printed 14), "%4-20mA (N/A for Models 787BT &
785)" [KNOWN]. ¹⁾ The BM787BT manual's nS heading excludes only the BM785
(p.10 (printed 9)), but its footnote 5 says "For Model 789 only" (p.26
(printed 25)) [KNOWN]. HOLD, AutoHold, Δ (REL), REC, CREST and RANGE map to
the Flag0/Flag1 bits of §6.6 [INFERRED by name]; SELECT toggles °C/°F when
both are enabled (p.12 (printed 11)), which fits units `14`/`15` [INFERRED].

---

## 10. Worked examples

Synthetic, built from the layouts above, not captured [INFERRED]. Each CRC was
computed with r4's routine and placed low byte first as the app does; each
was checked by recomputing the CRC over [2] through the CRC bytes, which gives
0.

**1. Reading: −1.2345 V DC, autorange, 2026-09-26 14:05:30.250**

```
FF 02 20 05 01 00 00 01 FA 78 85 03 3A 35 10 40 00 01 03 00 01 C7 CF FF 01 00 02 05 76 C3 FF 03
```

| Bytes | Decode |
|---|---|
| 0-7 `FF 02 20 05 01 00 00 01` | head, length 32, type 5, data set ID 1, PK ID 1 |
| 8-11 `FA 78 85 03` | `038578FA`: hour 14, minute 5, second 30, ms 250 |
| 12-13 `3A 35` | `353A`: year 26 (2026), month 9, date 26 |
| 14-16 `10 40 00` | AUTO-Ranging; Sign |
| 17-20 `01 03 00 01` | meter; Volt; reserved; DCV |
| 21-23 `C7 CF FF` | `FFCFC7` = −12345 |
| 24-27 `01 00 02 05` | point 1 of 5 digits (`X.XXXX`), no prefix, V → −1.2345 V |
| 28-29 `76 C3` | CRC 0xC376 over [2..27] |

The count is negative and the sign flag is set, both conventions at once;
which the meter uses is open (§6.3). [27] = 5 for this range is a guess.

**2. Information packet**, MAC 11:22:33:44:55:66 (invented), least
significant octet first (§5):

```
FF 01 18 04 01 02 66 55 44 33 22 11 00 00 00 00 04 00 00 01 FC 94 FF 03
```

CRC 0x94FC over [2..19]. A whole notification is this packet, example 1 and
96 bytes of `00`.

**3. Commands**, CRC over [2..27]:

| Command | Bytes |
|---|---|
| 0x0101 as the app sends it: MAC zero, [13] = `01` | `FF 01 20 01 01 00 00 00 00 00 00 01 01 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 11 35 FF 03` |
| 0x0151, password 0000, the MAC of example 2 | `FF 01 20 01 01 66 55 44 33 22 11 51 01 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 45 43 FF 03` |

---

## Implementation Notes

What the wire requires of any decoder:

- Readings arrive only as CDD5 notifications, which r4 starts once 0x0151 is
  accepted and notifications are enabled; at MTU 185 one notification holds
  the whole 152-byte output.
- A framed packet is `FF`, `01` or `02`, a length at [2], a type at [3], a
  CRC-16/MODBUS of [2] up to the CRC, and `FF 03`. The three reading blocks
  after the first are all zeros, with no framing.
- Multi-byte fields are little-endian; the count is 24-bit two's complement.
- [24] is the number of integer digits, not of decimals; the prefix [25] is
  a further power of ten on top of it.
- r4 defines the sign both in the count and in Flag1 bit 6; which the meter
  uses is open (§11). Under OL (Flag1 bit 5) the count carries nothing;
  under Flag0 bit 2 it is a text code.
- AC, DC and AC+DC are sub-function IDs, not flags.
- Low battery is only in the information packet.
- A field in the advertisement or the packets that tells a BM788BT from a
  BM787BT: none found in r2, r4 or the app, 2026-09-26; the advertised name
  is user-set.

## 11. Open questions — [UNVERIFIED]

Each item says whether the community sources (§12) narrow it or leave it
open. Data a meter sent: none found in their files, 2026-09-26; they rest on
one author's code and statements, so none of them closes an item.

1. **CRC byte order** on the meter: low byte first in the app; r4 and r2
   silent, with no complete frame (§4). **Narrowed**: a community SDK's
   commands, CRC low byte first, are accepted (if the meter checks the CRC,
   §12.2).
2. **Reading HeadByte1**: `02`, labelled "SOH" in both revisions (§4).
   **Narrowed**: a community SDK takes reading packets only with `02`
   (§12.2).
3. **Response channel**: the app reads CDD4; r4 silent; whether the meter
   also notifies responses on CDD5 (§3.2). **Narrowed**: reading CDD4 right
   after the write returns the reply (§12.2); on notified replies, none
   found in the repositories' files, 2026-09-26.
4. **Order and necessity of the bring-up**: r4 enables notifications after
   0x0151, the app in GATT order; whether the meter streams with
   notifications enabled first, or without 0x0151 (§3). **Narrowed**: r4's
   order streams with no command outside r4 and no MTU request (§12.2); a
   result with another order: none found in the repositories' files,
   2026-09-26.
5. **Password encoding and digit order**: ASCII or binary; the app reverses
   the digits in 0x0151 and keeps them in order in 0x0140 (§7.2).
   **Narrowed**: binary "0000" works (§12.2). The digit order stays open:
   a community SDK sends the digits in order in 0x0151; a result with a
   non-zero password: none found in the repositories' files, 2026-09-26
   (§12.3 D3).
6. **MAC bytes in commands**: their order, what the meter expects before any
   response, whether it checks them (§5). **Narrowed**: low byte first, in a
   0x0151 sent before any response (§12.2); whether the meter checks them
   stays open.
7. **Password Identification [13]**: `01` in r4, `00` in four of the app's
   commands (§5). **Narrowed**: `01` works (§12.2); a result with `00`:
   none found in the repositories' files, 2026-09-26.
8. **Commands outside r4**: whether and how the meter answers 0x0101, 0x0106,
   0x0021 and 0x0005, and 0x8000 (§7.3, §8). **Narrowed**: none is needed
   for streaming (§12.2).
9. **Sign**: negative count, magnitude plus flag, or both (§6.3). **Open**
   (§12.3 D2).
10. **Scaling fields per range**: [27], [24] and [25] for each range;
    decimal points for 3 and 6 digits; the prefix byte encoding (§6.3).
    **Open**.
11. **%4~20mA unit**: `4F` in r4, absent in the app (§6.4). **Open**.
12. **AutoCheck sub**: `03` AUTO in r4, `02` OHM in the app (§6.5). **Open**.
13. **Line frequency**: main `23` or the struck `03` subs (§6.5). **Open**.
14. **Bits r4 marks "x"**: Flag0 bits 0-1, Flag1 bits 0 and 7 (the app's
    TestLead), Flag2 (§6.6). **Open**.
15. **ASCII codes outside r4**: 0, 8, 9, `0C`-`0F` (§6.7). **Open**.
16. **Information packet fields**: battery values other than `02`, the Power
    Source Flag, [16..18] as one count, [19] (§6.1, §6.9). **Open**.
17. **Advertising**: the Status byte, the scan response, the OTA-mode
    advertisement, advertising while connected (§2). **Narrowed**: one
    connection at a time is reported (§12.4); a service UUID in the
    advertisement is claimed without evidence (§12.3 D4).
18. **RTC**: hour 1-23 or 0-23; its content before the first 0x0010 and
    across power-off; binary or BCD in 0x0010 (§6.8, §7.1). **Narrowed**:
    the author's SDK sends 0x0010 in binary (§12.2); the clock is reported to reset on
    power-off (§12.4); the hour range stays open.
19. **Device name**: 12 characters (r4) or 11 (app); padding (§2, §7.1).
    **Open**.
20. **Rate**: notifications per second against the display rates (§3.2,
    §9.4). **Narrowed**: about 5 a second is reported; no measurement
    recorded (§12.4).
21. **Write without response** on CDD4 (§2). **Open**.
22. **APO**: 30 or 15 minutes; the effect of a connection; whether Bluetooth
    stays on after power-off; the Δ long press in AutoV (§9.1, §9.3).
    **Open**.
23. **BM787BT defaults**: 0000 and BM78xBT are not printed in its manual
    (§9.2). **Open**.
24. **Error codes**: 3 and 4 both "invalid password" in r4, connection and
    administrator in the app; codes above 6 (§8). **Open**.
25. **Model identity**: nothing on the wire tells the BM788BT from the
    BM787BT (§1). **Open**.
26. **MTU below 185**: r4 requires 185 and the app requests it; a community
    SDK streams without requesting any (§12.3 D1). What the meter does when
    the negotiated MTU cannot carry 152 bytes in one notification (§2, §4).
    **Open**.
27. **Pause on a function or range change**: notifications are reported to
    stop while the function or range is switched, the link staying up
    (§12.4); how long the pause lasts (§9.4). **Open**.

---

## 12. Cross-reference with community sources [COMMUNITY]

Read 2026-09-26, after §1-11 were written from Brymen's sources,
grounding-checked and committed; nothing here was merged into §1-11 beyond
pointers. The boundary covered code repositories only
(`reverse-engineering-approach.md`). Community paths
below are relative to `references/bm78xbt/community/`, at the commits of
§12.1.

How the sources were made matters more than how many agree:

- **One author, one SDK.** All three repositories are Martin Chan's: a
  Python SDK on bleak (`brymenble`) and two programs built on it. Their
  tables (function IDs, units, ASCII codes, error texts,
  `brymenble/src/brymenble/constants.py`) follow Brymen's document, and
  their command set lacks r4's 0x0040 (`constants.py:47-61`), as r2 does.
  The author names the document `BM780-Wireless-Communication-Protocol-r2.*`
  (`brymenble/.gitignore:18`), our r2's file name, so their source is r2
  (author). Where they match §1-11 they restate the document; they are not independent
  evidence.
- **No captured bytes.** A packet a meter sent: none found in the three
  repositories' files at the §12.1 commits, 2026-09-26; their tests build
  frames synthetically. What rests on a real meter is the
  author's statements in code comments, documentation and the changelog, and
  the SDK working at all: the author runs it against a meter
  (`brymenble/tools/probe.py:1-7`, "against real hardware";
  `brymenble/README.md:28`, Linux and Windows). Below, "author" marks a
  statement with no recorded data; "SDK" marks a fact implied by the SDK
  working.
- Model names: none found in the three repositories' files (BM785-BM789),
  2026-09-26.

### 12.1 Sources

| Source | Commit | Covers | Licence |
|---|---|---|---|
| [milksplash/brymenble](https://github.com/milksplash/brymenble) | `02e28b6` (2026-08-31) | Python SDK on bleak: parsers, commands, transport, scanner; capture and probe tools | MIT © 2026 Martin Chan |
| [milksplash/brymenble-tc-bridge](https://github.com/milksplash/brymenble-tc-bridge) | `6355bbd` (2026-08-31) | forwards SDK readings to TestController | MIT, same holder |
| [milksplash/brymenble-overlay](https://github.com/milksplash/brymenble-overlay) | `af59fec` (2026-08-31) | video overlay that redraws the LCD from SDK readings | MIT, same holder |

### 12.2 Agree

| Spec § | What the community sources show | Evidence |
|---|---|---|
| §2 GATT | CDD0, CDD4 and CDD5 with Brymen's base (`brymenble/src/brymenble/transport.py:17-18`, `scanner.py:20`) | SDK |
| §2 pairing | the SDK connects with no pairing call (none found in `src/`, 2026-09-26); "Linux and Windows are supported" (`brymenble/README.md:28`) | SDK; author (platforms) |
| §3.1 bring-up order | connect; 0x0151 written to CDD4 with response and its reply read from CDD4; optionally 0x0010; 0.5 s; CDD5 notifications on (`brymenble/src/brymenble/transport.py:222-251`). The capture tool writes 0x0151, waits 0.5 s and subscribes (`brymenble/tools/capture.py:79-85`). Neither sends 0x0101, 0x0106 or 0x0021, or requests an MTU | SDK |
| §3.2 response channel | the reply is read from CDD4 right after the write (`transport.py:328-332`); the connect fails unless it starts `FF 01 20 02 01` (`parsers.py:550-561`, `transport.py:357-361`). The probe tool exists partly for "confirming the read-after-write response delivery" (`tools/probe.py:6-7`) | SDK; author |
| §4 CRC | r4's routine; commands carry it low byte first (`commands.py:51`), and received packets are checked low byte first (`parsers.py:383-385`) | SDK (commands), below |
| §4 reading head | a reading packet is taken only with [1] = `02` (`parsers.py:375`) | SDK |
| §5 MAC order | commands carry the address reversed (`commands.py:46`), 0x0151 first with no earlier response; "The meter sends the MAC byte-reversed on the wire" in the information packet (`parsers.py:318-319`) | SDK; author |
| §5 [13] | `01` in every command (`constants.py:40`) | SDK |
| §7.1 0x0010 | sent in binary: second, minute, hour, date, weekday Monday = 1 to Sunday = 7, month, year − 2000 (`commands.py:73-83`); the probe tool checks the time read back after setting it (`tools/probe.py:144-155`) | author |
| §7.2 encoding | the password goes as one binary byte per digit (`commands.py:56-62`); the connect with "0000" works | SDK, for 0000 only |

For §4: a meter that checks the command CRC, as r4's error 0 "Checksum
error" suggests (r4 p.6), takes it low byte first, or the SDK's connect would
fail [INFERRED]. The SDK does not drop a packet whose CRC fails; it flags it
(`parsers.py:385`), so its receive side is no evidence of the order.

### 12.3 Disagree

| # | Spec § | Community | Brymen source | Verdict |
|---|---|---|---|---|
| D1 | §2 MTU | the SDK requests no MTU and accepts a stream frame only as one whole 152-byte notification (`parsers.py:495`; `tools/capture.py:93`) | r4 p.7, p.15: "must be set to 185"; the app requests 185 | if the SDK works as reported, a 185 request was not needed on the author's hosts, and the MTU the OS negotiated let 152 bytes through in one notification (at least 155 [INFERRED from the 3-byte ATT header]); no value recorded; below that, open |
| D2 | §6.3 sign | the count is read signed, then `abs()`; the sign comes from Flag1 bit 6 only, "the protocol encodes it in exactly one place" (`parsers.py:410-414`; `docs/SDK_DATA_REFERENCE.md:40-42`) | r4 p.10: the count is signed, with a negative example | a design choice, not an observation; open (§11.9) |
| D3 | §7.2 digit order | 0x0151 and 0x0140 both send the digits in order (`commands.py:56-62`) | the app reverses them in 0x0151 only | a result with a non-zero password: none found in the repositories' files, 2026-09-26; open (§11.5) |
| D4 | §2 advertisement | the scanner accepts the service UUID in the advertisement, "advertised by some meters" (`scanner.py:19`, `:64-65`), or manufacturer data `42 4D 0B` after company 0x0131 (`:66-72`) | r4 p.1's advertisement carries no service UUID | no evidence for the UUID; open (§11.17). The manufacturer-data test also checks the series byte, which the app does not |

### 12.4 New

| Topic (§) | Finding | Evidence |
|---|---|---|
| Rate (§3.2, §9.4) | "The meter streams at ~5 Hz (~200 ms)" (`brymenble-tc-bridge/bridge/bridge.py:50-51`) | author |
| Function change (§9.4) | "while switching, the meter simply stops emitting reading packets (the BLE link stays up) and resumes with the new function's first frame", and the LCD reading blanks (`brymenble/docs/SDK_DATA_REFERENCE.md:236-245`); the bridge and overlay say the same of a "function/range switch" (`brymenble-tc-bridge/bridge/bridge.py:47-49`; `brymenble-overlay/main.py:43-45`). How long the gap lasts is not given | author |
| Temperature overload (§6.3) | the LCD shows "----" for a temperature overload; the author calls it "a display accommodation, not protocol behavior" (`brymenble-overlay/overlay/state.py:173-176`; `brymenble-tc-bridge/testcontroller/BrymenBM78xBT.txt:92`) | author |
| RTC (§6.8) | "the meter has no RTC battery, so its clock resets on power-off" (`brymenble/CHANGELOG.md:279-280`; `commands.py:89`) | author |
| 0x0010 timing (§7.1) | "The meter echoes the calibration command immediately, but its RTC needs a moment to register" — about 0.5 s — and "doesn't preserve sub-second precision" (`brymenble/tools/probe.py:144-148`, `:176-179`) | author |
| Connections (§2) | "a BM78xBT accepts a single connection" (`brymenble/README.md:106-110`). "The BM78xBT only advertises while NOT connected (protocol design flowchart)" (`scanner.py:22-30`) is a reading of r4 p.15, whose flowchart states no such rule [KNOWN, r4 p.15] | author |
| Function selection | "The meter's function **cannot be switched over BLE/TestController.**" (`brymenble-tc-bridge/README.md:121`); r4's command table has no such command | author |

Not answered by the community sources: §11.9-16, §11.19, §11.21-25, and
the rate and gap figures above as measurements. The SDK's function, unit and
ASCII tables restate the document, so they settle none of §11.11-13 or
§11.15.
