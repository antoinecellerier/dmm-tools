# EEVblog 121GW: Reverse-Engineered Protocol Specification

What the EEVblog 121GW multimeter sends and accepts over Bluetooth LE. One
packet: 19 binary bytes, starting `F2` and ending in an XOR checksum, that
carries the main display (mode, range, an 18-bit value), the secondary
display, the bar graph and the annunciators. Commands to the meter are short
ASCII-hex frames led by `F4` (key presses) or `F8` (clock set). It is
implemented, experimentally (`crates/dmm-lib/src/protocol/eevblog121gw/`),
and no 121GW has been on our bench: every fact in §1-14 comes from EEVblog's
packet-format documents, EEVblog's app, UEi's app and the user manual, and
§15 compares them with community sources. The approach doc beside it records
the sources, the method and the clean-room boundary.

Based on:
- EEVblog's "BLE Packet Format" documents, V1 (2018-03-22) and V2
  (2018-07-03), read from the rendered pages
- EEVblog's cross-platform Xamarin app (gitlab.com/Sepps/app-121gw), whose
  final protocol code is on the unmerged branch `PrivatePostRelease` at
  `aab403d` (2018-09-04); older commits for the earlier formats (§12)
- UEi's Android app `kr.co.finest.eevblog` 1.0.6, decompiled with jadx 1.5.6
- The 121GW user manual, last revised 3 March 2025, read from the rendered
  pages

Citation keys:
- **V1 p.N**, **V2 p.N** — `references/121gw/protocol/121GW-BLE-Packet-Format-V1.pdf`
  and `Revised-Packet-Format-Blob-V2.pdf`, PDF page N (renders in
  `protocol/render/`)
- **manual p.N** — `references/121gw/manuals/EEVblog-121GW-Manual.pdf`, PDF
  page N (the printed page number is the same)
- **PPR `path:line`** — EEVblog's app, `references/121gw/official-app/`, the
  file at commit `aab403d` (`git show aab403d:<path>`). Short forms:
  **Packet** = `121GW.Core/Controls/Multimeter/Packet121GW.cs`, **Screen** =
  `121GW.Core/Controls/Multimeter/MultimeterScreen.cs`, **Multimeter** =
  `121GW.Core/Controls/Multimeter/Multimeter.cs`
- **`git:<sha>:path:line`** — the same repository at an older commit;
  **master `path:line`** — its checked-out `master` working tree
- **UEi `File.java:line`** — `references/121gw/uei-app/jadx/sources/kr/co/finest/eevblog/`;
  **UEi strings.xml** = `uei-app/jadx/resources/res/values/strings.xml`
- **Community sources**, §15 only — paths under `references/121gw/community/`;
  **fw 1.02** = `121gw-re/database/EEVBlog-102.c`, tpwrules' IDA export of
  firmware 1.02

Byte and bit numbering: byte 0 is the `F2` start byte; bit 7 is the most
significant bit, as the documents' b7..b0 columns.

Confidence levels:
- **[KNOWN]** — stated in the packet-format documents or the manual, cited by
  page
- **[VENDOR]** — read from EEVblog's or UEi's app, cited by file and line
- **[INFERRED]** — logical inference from the above, reason given
- **[UNVERIFIED]** — no source confirms it, or the sources disagree; needs a
  real meter (all in §14)
- **[HARDWARE]** — seen on a real meter: none yet
- **[COMMUNITY]** — stated or captured by a community source, §15; not a
  vendor fact

---

## 1. Model and firmware

One model, the EEVblog 121GW. Its Bluetooth is a "BLE112-A Class 2 certified
Low Energy Bluetooth module", FCC ID QOQBLE112 (manual p.15, p.24); the
measurement chipset is a Hycon HY3131 with an ST STM32L152D (p.15) [KNOWN].

The manual's firmware list runs from 1.00 to 2.05, "released 22/4/2021"
(p.9-10), and two entries concern the Bluetooth packet [KNOWN]:

| Firmware | Manual entry (p.9) | Matching change |
|---|---|---|
| 1.21 | "Added support for °C and °F in the Bluetooth packet." | V2 adds °C/℉ bits to byte 6 and bytes 15/16 (V2 p.1); EEVblog's app reads the byte-6 bits from commit `58fe42f` (2018-06-15) |
| 1.22 | "Added 2 more bits of data for the main data to accommodate values over 65535." | V2 adds "Value (Bit 17, Bit 16)" to byte 5 (V2 p.1); EEVblog's app commits `e01726d` "Added support for extended main value in packet." (2018-06-19) and `91bd2a9` "Fixed error with mode with new packet format 1.22" (2018-06-20) |

So the V2 layout is the packet of firmware 1.22 and later [INFERRED from the
dates and wording]. Which firmware first sent the 19-byte packet at all:
none found in the documents, the two apps or the manual, 2026-09-26. The V1
document (created 2018-03-22) is phrased as a proposal
("we propose to make a shorter packet", V1 p.1), and EEVblog's app switched
to it the same day (commit `43b3e94`, "New Packet Format Implementated")
[UNVERIFIED which firmware]. Earlier firmware sent ASCII-encoded packets
(§12).

No field for a model, firmware version or packet version: none found in the
V2 p.1 table, 2026-09-26 [INFERRED from the V2 p.1 table].

## 2. Advertising and GATT

| Item | Value | Source | Tag |
|---|---|---|---|
| Module | BLE112-A (manual p.15, p.24) | manual | [KNOWN] |
| Service | `0bd51666-e7cb-469b-8e4d-2742f1ba77cc` (UEi `BLEService.java:39`; as the hex string `0bd51666e7cb469b8e4d2742f1ba77cc` at `:36`) | UEi | [VENDOR] |
| Characteristic | `e7add780-b042-4876-aae1-112855353cc1` (UEi `BLEService.java:40`), the only one UEi's app uses: fetched at `:92`, subscribed at `:93-96`, written at `:186-191` | UEi | [VENDOR] |
| Subscribe | `setCharacteristicNotification(true)`, then `ENABLE_INDICATION_VALUE` written to each descriptor of that characteristic, one per callback (UEi `BLEService.java:93-96`, `:103-107`, `:193-201`); the constant's value is `02 00` | UEi | [VENDOR]; the byte value [INFERRED from the Android `BluetoothGattDescriptor` API and the Bluetooth Core Specification's CCCD values, general platform references] |
| EEVblog's app | names no UUID in any revision. It subscribes to every characteristic of every service that can update (Android: `StartUpdatesAsync` when `CanUpdate`, PPR `App 112GW/App_112GW.Android/Peripherals/Bluetooth/Characteristic.cs:49-50`; UWP: only characteristics with the Indicate property, writing the CCCD for Indicate, PPR `App 112GW/App_112GW.UWP/Peripherals/Bluetooth/Characteristic.cs:25-35`), feeds all of them into one packet assembler (Multimeter:148) and writes key codes to every characteristic (Multimeter:205-210) | EEVblog | [VENDOR]; consistent with one characteristic. The UWP path working at all implies the data characteristic has Indicate [INFERRED] |
| Notify | whether the characteristic also offers notifications (community: see §15.4) | — | [UNVERIFIED] |
| Write type | neither app sets one: UEi calls `writeCharacteristic` with the characteristic's default type (`BLEService.java:186-191`); EEVblog calls `WriteAsync`/`WriteValueAsync` with no option (PPR Android `Characteristic.cs:22-33`, UWP `Characteristic.cs:17`) | apps | [VENDOR]; which types the characteristic takes [UNVERIFIED] (no community answer, §15.4) |
| MTU | neither app requests one | apps | [VENDOR] |
| Discovery, UEi | no name filter: the app walks the raw scan record and compares the first UUID of the first AD structure of type `0x07` (Complete List of 128-bit Service UUIDs), byte-reversed, with the service UUID (`BLEService.java:55-56`, `:213-239`) | UEi | [VENDOR]; so the meter advertises the service UUID in an AD type 0x07 [INFERRED] |
| Discovery, EEVblog | accepts any device whose name contains `121GW` or `Bluegiga` (PPR `121GW.Core/Controls/Multimeter/Settings.cs:19`, applied at PPR `121GW.Core/Peripherals/Bluetooth/Client.cs:68-69`); unnamed devices are dropped (the same line; Android `DeviceWatcher_Added`, PPR `App 112GW/App_112GW.Android/Peripherals/Bluetooth/Client.cs:24`) | EEVblog | [VENDOR] |
| Advertised name | the exact name: none found in the two apps or the manual (p.55 is the Bluetooth section), 2026-09-26; the EEVblog filter admits two spellings (community: see §15.4) | — | [UNVERIFIED] |
| Pairing | bonding or pairing calls: none found in the UEi jadx sources (no `createBond`) or in EEVblog's connect paths (PPR Android `Client.cs:43-63`, UWP `Client.cs:31-52`, which only connect), 2026-09-26; a PIN: none found in the manual (p.55), 2026-09-26 | apps, manual | [VENDOR], [KNOWN] |

## 3. Bring-up

1. **On the meter:** "hold the “1ms PEAK” button until BT is displayed on
   the LCD"; hold it again until BT is gone to switch Bluetooth off (manual
   p.55, p.33) [KNOWN]. The BT annunciator is byte 16 bit 6 (§9).
2. **Host:** connect, discover services, enable indications on
   `e7add780-…` (UEi `BLEService.java:82`, `:92-96`) [VENDOR].
3. The meter then pushes packets. Neither app writes a start command or a
   keep-alive: EEVblog's app writes only when a key button is pressed
   (Multimeter:155-163, :205-210); UEi's writes key codes and the clock set
   (§11) [VENDOR]. That the meter streams unprompted is [INFERRED] from both
   apps; the manual says "the display data will be transmitted via the
   Bluetooth connection" while the meter keeps working normally (p.55)
   [KNOWN].

UEi's app also sends the clock set (§11.2) once a second after connecting
until the meter acknowledges it (UEi `MainActivity.java:113-136`); EEVblog's
app never sends it [VENDOR].

**Rate.** The display updates "5 times per second nominal" (manual p.15)
[KNOWN]. The packet rate: none found in the documents, the two apps or the
manual, 2026-09-26 [UNVERIFIED]. Community: see §15.4.

**Auto power-off.** The meter powers off after 30 minutes unless APO is set
off; APO is disabled while logging to SD (manual p.61) [KNOWN]. How APO
treats a Bluetooth connection: not found in the manual, 2026-09-26
[UNVERIFIED]; community: see §15.4. The APO annunciator is byte 15 bit 1
(§9).

## 4. Framing

| Rule | Source | Tag |
|---|---|---|
| A packet is 19 bytes; byte 0 is `F2` ("Start COMMAND") | V2 p.1 table; V2 p.2 `u8 Bytes[19u]`; `new PacketProcessor(0xF2, 19)`, Multimeter:13 | [KNOWN], [VENDOR] |
| No length byte, no version byte | V2 p.1 | [KNOWN] |
| Byte 18 is the XOR of bytes 0 to 17, start byte included | V2 p.1 table ("XOR of bytes 0 ... 17"); Packet:295-303, with the length check `input.Length == 19u` at :303 | [KNOWN], [VENDOR] |
| 16-bit values are big-endian: the high byte comes first | V2 p.1 (Value_H at byte 7 before Value_L at byte 8; Serial B3 at byte 1); `(mData[7] << 8) \| mData[8]`, Packet:169; `MSB * 256 + LSB`, Packet:178-186 | [KNOWN], [VENDOR] |
| `F2` is not escaped; a binary field can hold `F2` | V2 p.1 ("The packet should be a binary blob, no encoding is necessary", p.1) | [INFERRED] |

Two passages of the documents contradict the table, and both apps side with
the table:

- **Checksum code.** The example sender on V1/V2 p.2 XORs all 19 bytes,
  including the checksum field it has not yet set, and the usage example on
  p.3 never initialises that field. That equals "XOR of bytes 0 ... 17" only
  when byte 18 starts at 0. A document error [INFERRED]; EEVblog's app checks
  bytes 0-17 against byte 18 (Packet:295-303). Equivalently, the XOR of all
  19 bytes of a good packet is 0 [INFERRED].
- **Byte order and packing.** The p.2 C union declares `u32 Serial` at byte 1
  and `u16` values with no packing or endianness note. Unpacked, that struct
  is not 19 bytes, and a little-endian host would put the low byte first. A
  document error [INFERRED]; the table and the app are big-endian.

**Chunking.** 19 bytes fit one indication under the default 23-byte ATT MTU
(20-byte payload) [INFERRED from the Bluetooth Core Specification's default
ATT MTU, a general platform reference]. Whether the meter always sends one packet per
indication is [UNVERIFIED]: EEVblog's app appends every indication to one
buffer, drops bytes before the first `F2` and cuts 19 bytes at a time
(PPR `121GW.Core/Packet/PacketProcessor.cs:30-55`); UEi's parser keeps its
state across indications (UEi `Protocol.java:111-321`) [VENDOR]. A 2017-12
commit message speaks of "packet cutoff which is caused by connection
interval" on Windows, in the ASCII era (`0cca1b3`) [VENDOR]. Community: see
§15.4.

**V1 and V2.** Every bit V2 adds was a fixed `0` in V1 (V1 p.1, V2 p.1)
[KNOWN], so a V1 packet reads under V2 as "value bits 17-16 = 0, no °C/℉"
[INFERRED]. There is no way to tell them apart in the packet.

## 5. Packet layout

V2's table (V2 p.1) with the field names of its C union (V2 p.2), and how the
apps read each field. UEi's app decodes the older 54-byte ASCII packet, not
this one (§12); its column gives the reading of the same field there, whose
bits match [INFERRED from §12].

| Byte | Bits | Field (V2) | Meaning | EEVblog (Packet) | UEi (`Protocol.java`) |
|---|---|---|---|---|---|
| 0 | 7-0 | Start | `F2` | Multimeter:13 | `:117`, `:133` |
| 1 | 7-0 | Serial B3 | "Year ( 4 … 0 )" (§10) | :137 | — |
| 2 | 7-4 | Serial B2 | "Month (1 … 0)" | :138 | — |
| 2 | 3-0 | | "Serial Number Digit 4" | :139-144 | — |
| 3 | 7-4, 3-0 | Serial B1 | digits 3, 2 | :139-144 | — |
| 4 | 7-4, 3-0 | Serial B0 | digits 1, 0 | :139-144 | — |
| 5 | 7-6 | MainMode | main value bits 17, 16 (V2 only) | :169 | — |
| 5 | 5 | | `0` | not read | — |
| 5 | 4-0 | | main mode, 0-24 (§6.1) | :151 (`& 0x1F`) | `:154-159` |
| 6 | 7 | MainRange | OFL | :160 | `:166` |
| 6 | 6 | | sign, set = negative | :161 | `:167` |
| 6 | 5 | | °C (V2 only) | :162 | — |
| 6 | 4 | | ℉ (V2 only) | :163 | — |
| 6 | 3-0 | | range, "RANGE ( 0 ~ 6 )" (§6.2) | :166 | `:165` |
| 7-8 | 16 bits | MainValue | main value bits 15-0, unsigned, high byte first | :169 | `:172-184` |
| 9 | 7-0 | SubMode | "100 ~ 199, 0 ~ 24" (§7.1) | :171 | `:185-190` |
| 10 | 7 | SubRange | OFL | :172 | `:195` |
| 10 | 6 | | sign, set = negative | :173 | `:196` |
| 10 | 5 | | "k" | :174 | `:197` |
| 10 | 4 | | "Hz" | :175 | `:198` |
| 10 | 3 | | `0` | read as part of the point (:176) | not read |
| 10 | 2-0 | | "Point( 0 ~ 4 )" (§7.2) | :176 | `:194` |
| 11-12 | 16 bits | SubValue | secondary value, unsigned, high byte first | :178-186 | `:202-221` |
| 13 | 7-5 | BarStatus | `0` | not read | — |
| 13 | 4-0 | | USE, 0~150, +/-, 1000 / 500 (§8) | :219-222 | skipped (`:222-226`) |
| 14 | 7-5 | BarValue | `0` | masked off | — |
| 14 | 4-0 | | "BAR GRAPH 0 ~ 25" (§8) | :223 | skipped |
| 15 | 7-0 | IconStatus1 | °C (V2 only), 1KHz, 1ms, DC + AC, AUTO, APO, BAT (§9) | :228-233 | `:227-235` |
| 16 | 7-0 | IconStatus2 | ℉ (V2 only), BT, ↙, REL, dBm, MIN/MAX (§9) | :235-239 | `:236-240` |
| 17 | 7-0 | IconStatus3 | `0`, TEST, MEM, A-HOLD, AC, DC (§9) | :241-245 | `:241-246` |
| 18 | 7-0 | Checksum | XOR of bytes 0-17 (§4) | :295-303 | — |

---

## 6. Main display

### 6.1 Mode codes

Byte 5 bits 4-0. The documents give only "0 ~ 24 (0x00 ~ 0x18)" (V2 p.1)
[KNOWN]; the names come from the two apps, which agree on every code
(EEVblog `eMode`, Packet:29-55; UEi `initModeInfoMap`, `Protocol.java:411-435`,
names from UEi strings.xml:91-133) [VENDOR]. The dial column places each code
on the meter from the manual [INFERRED: by name].

| Code | EEVblog | UEi | On the meter (manual) |
|---|---|---|---|
| 0 | `Low_Z` | Low-Z | Low Z dial position (p.38) |
| 1 | `DCV` | DCV | V position, DC (p.36) |
| 2 | `ACV` | ACV | V position, AC (p.36) |
| 3 | `DCmV` | DCmV | mV position, DC (p.36) |
| 4 | `ACmV` | ACmV | mV position, AC (p.36) |
| 5 | `Temp` | Temp. | mV position, third MODE step (p.36, p.39); K-type thermocouple (p.23) |
| 6 | `Hz` | Hz | Hz position, frequency (p.48) |
| 7 | `mS` | mS | Hz position, one MODE press: positive pulse width in ms (p.50) |
| 8 | `Duty` | Duty | Hz position, two presses: duty cycle (p.49) |
| 9 | `Resistor` | Resistor | Ω position (p.44) |
| 10 | `Continuity` | Continuity | Ω position, one press (p.45) |
| 11 | `Diode` | Diode | Ω position, diode (p.44, p.51) |
| 12 | `Capacitor` | Capacitor | Ω position, three presses (p.47) |
| 13 | `ACuVA` | ACuVA | µA position's µVA label (p.31); how it is selected: none found in the manual, 2026-09-26 (community: see §15.4) |
| 14 | `ACmVA` | ACmVA | mVA/VA position (p.53) |
| 15 | `ACVA` | ACVA | mVA/VA position (p.53) |
| 16 | `ACuA` | ACuA | µA position, AC (p.41) |
| 17 | `DCuA` | DCuA | µA position, DC (p.41) |
| 18 | `ACmA` | ACmA | A/mA position, AC, red lead in mA µA (p.40, p.43) |
| 19 | `DCmA` | DCmA | A/mA position, DC, mA µA jack |
| 20 | `ACA` | ACA | A/mA position, AC, A 500mA jack (p.40) |
| 21 | `DCA` | DCA | A/mA position, DC, A 500mA jack |
| 22 | `DCuVA` | DCuVA | as 13 |
| 23 | `DCmVA` | DCmVA | as 14 |
| 24 | `DCVA` | DCVA | as 15 |

- The manual's calibration table (p.71) lists the same functions in the same
  order and spelling (Low-Z, DCV, ACV, DCmV, ACmV, Temp., …, DCVA), but
  numbers them its own way: its row 6 "Hz/mS/%" merges three wire codes and
  it calls continuity "Beep", so its numbers from 7 on are not the wire codes;
  it also has an unnumbered "PeakHold" row between ACV and DCmV [KNOWN table;
  INFERRED comparison].
- EEVblog's `eMode` also has 25 `TempC` and 26 `TempF`; the app substitutes
  them for code 5 when byte 6 bit 5 or 4 is set (Packet:147-159). They are
  not wire codes [VENDOR].
- **DC+AC V has no code.** The V position's MODE cycles DC → AC → DC+AC
  (manual p.36), and AC+DC V has its own spec rows (p.17); a code for it: none
  found in the two apps, 2026-09-26. Byte 15 bits 4-3 can say "DC + AC" (§9); how the meter
  reports this function is [UNVERIFIED]; community: see §15.4.
- Byte 5 bit 5 is `0` in both documents [KNOWN] and read by neither app
  [VENDOR].

### 6.2 Ranges

Byte 6 bits 3-0 index a per-mode range table that the documents do not give
("RANGE ( 0 ~ 6 )", V2 p.1) [KNOWN]. Both apps carry one:

- **EEVblog** (Packet:93-122): per range, the number of digits before the
  decimal point on a five-digit readout, and a prefix character. The digits
  are `count / 10^(5 − digits)` (Packet:257); below, 1 → `d.dddd`,
  2 → `dd.ddd`, 3 → `ddd.dd`, 4 → `dddd.d`, 5 → `ddddd`. The app uses the
  table twice, and the two uses differ in some modes:
  - **its LCD** lights unit segments by mode (Screen:360-467, called from
    Screen:867) and adds a prefix segment from the prefix character
    (`m`, `M`, `k`, `u`, `n`; Screen:545-553). The column below gives what the
    LCD shows;
  - **its chart and log** multiply the reading by the prefix character alone
    (`m` ×10⁻³, `u` ×10⁻⁶, `n` ×10⁻⁹, `k` ×10³, `M` ×10⁶, blank ×1;
    Packet:271-291, used at Multimeter:122). Where that differs from the LCD,
    the row says so. Each range entry's unit string (`mUnits`) is assigned
    but never read (Packet:11, :19).
- **UEi** (`Protocol.java:411-435`): per range, a unit string and the weight
  of one count.

Both are [VENDOR]. The manual column is the calibration table's range cell
(p.71, "R0"…"R6") where it has one, else the spec table's range and
resolution (p.17-23), quoted as printed [KNOWN]. "Counts" is the manual's
full-scale reading divided by the count weight the LCDs of both apps agree
on [INFERRED arithmetic]; it rests on the manual row named.

| Mode | Range | EEVblog LCD | UEi | Manual | Counts | Agree? |
|---|---|---|---|---|---|---|
| 0 Low_Z | 0 | `dddd.d` V | 0.1 V | 600.0V (p.71) | 6000 | yes |
| 1 DCV, 2 ACV | 0 | `d.dddd` V | 0.0001 V | 5.0000V (p.71); 5V, resolution printed "0.1" with no unit (p.17, DC V); 5V, 0.1 mV (p.17, AC V) | 50000 | yes |
| | 1 | `dd.ddd` V | 0.001 V | 50.000V (p.71) | 50000 | yes |
| | 2 | `ddd.dd` V | 0.01 V | cell cut off in the render, "500.00▸" (p.71); 500 V, 10 mV (p.17) | 50000 | yes |
| | 3 | `dddd.d` V | 0.1 V | 600.0V (p.71) | 6000 | yes |
| 3 DCmV, 4 ACmV | 0 | `dd.ddd` mV | 0.001 mV | 50.000mV (p.71); 1 µV (p.17) | 50000 | yes |
| | 1 | `ddd.dd` mV | 0.01 mV | 500.00mV | 50000 | yes |
| 5 Temp | 0 | `dddd.d` °C or °F (byte 6 bits 5/4) | 0.1 °C | −200.0 °C to 1350.0 °C (p.71); 0.1 °C (p.23) | 13500 at the top | yes |
| 6 Hz | 0 | `dd.ddd` Hz | 0.001 Hz | 99.999Hz (p.71, p.21) | 99999 | yes |
| | 1 | `ddd.dd` Hz | 0.01 Hz | 999.99Hz | 99999 | yes |
| | 2 | `d.dddd` kHz | 0.0001 kHz | 9.9999 kHz | 99999 | yes |
| | 3 | `dd.ddd` kHz | 0.001 kHz | 99.999 kHz | 99999 | yes |
| | 4 | `ddd.dd` kHz | 0.01 kHz | 999.99 kHz | 99999 | yes |
| 7 mS | 0 | `d.dddd` ms (Screen:394-396); chart ×1 under the label "Period (s)" | 0.0001 ms | pulse width "in milli-seconds" (p.50); no range or spec row | — | yes: ms on both LCDs and in the manual |
| | 1 | `dd.ddd` ms, as above | 0.001 ms | — | — | yes |
| | 2 | `ddd.dd` ms, as above | 0.01 ms | — | — | yes |
| 8 Duty | 0 | `dddd.d` % | 0.1 % | 99 %, 0.1 % (p.21) | — | yes |
| 9 Resistor | 0 | `dd.ddd` Ω | 0.001 Ω | 50.000Ω (p.71); 0.001 Ω (p.19) | 50000 | yes |
| | 1 | `ddd.dd` Ω | 0.01 Ω | 500.00Ω | 50000 | yes |
| | 2 | `d.dddd` kΩ | 0.0001 kΩ | 5.0000 kΩ | 50000 | yes |
| | 3 | `dd.ddd` kΩ | 0.001 kΩ | 50.000 kΩ | 50000 | yes |
| | 4 | `ddd.dd` kΩ | 0.01 kΩ | 500.00 kΩ | 50000 | yes |
| | 5 | `d.dddd` MΩ | 0.0001 MΩ | 5.0000 MΩ | 50000 | yes |
| | 6 | `dd.ddd` MΩ | 0.001 MΩ | 50.000 MΩ | 50000 | yes |
| 10 Continuity | 0 | `ddd.dd` Ω (Screen:403-406) | 0.01 Ω | 500Ω (p.71); 10 mΩ (p.23) | 50000 | yes |
| 11 Diode | 0 | `d.dddd` V | 0.0001 V | 3.0V (p.71); 3V, 0.001 V (p.20) | 30000 by the apps | **no**: 0.1 mV (apps) or 1 mV (p.20) |
| | 1 | `dd.ddd` V | 0.001 V | 15 V, 0.001 V (p.20) | 15000 | yes |
| 12 Capacitor | 0 | `ddd.dd` nF | 0.01 nF | 10.00nF (p.71); 0.01 nF (p.19) | 1000 | yes |
| | 1 | `dddd.d` nF | 0.1 nF | 100.0nF | 1000 | yes |
| | 2 | `dd.ddd` µF | 0.001 µF | 1.000 µF | 1000 | yes |
| | 3 | `ddd.dd` µF | 0.01 µF | 10.00 µF | 1000 | yes |
| | 4 | `dddd.d` µF | 0.1 µF | 100.0 µF | 1000 | yes |
| | 5 | `ddddd` µF | 1 µF | 10.00 mF (p.71); "9999 uF", 1 µF (p.19) | 10000 or 9999 | apps yes; the manual disagrees with itself |
| 13 ACuVA, 22 DCuVA | 0 | `ddd.dd` µVA (Screen:414-418, :450-454); chart ×1 | 0.01 µVA | 250.00 uVA (p.22) | 25000 | yes |
| | 1 | `dddd.d` µVA, as above | 0.1 µVA | 2500.0 uVA (p.22) | 25000 | yes |
| | 2 | `dddd.d` µVA, as above | 0.1 µVA | 2500.0 uVA (p.22) | 25000 | yes |
| | 3 | `ddddd` µVA, as above | 1 µVA | 25000 uVA (p.22) | 25000 | yes |
| 14 ACmVA, 23 DCmVA | 0 | `dd.ddd` mVA | 0.001 mVA | 25.000 mVA (p.22) | 25000 | yes |
| | 1 | `ddd.dd` mVA | 0.01 mVA | 250.00 mVA (p.22) | 25000 | yes |
| | 2 | `ddd.dd` mVA (Screen:419-423, :455-459); chart ×1, i.e. VA | 0.01 mVA | 250.00 mVA (p.22) | 25000 | yes |
| | 3 | `dddd.d` mVA, as above; chart ×1, i.e. VA | 0.1 mVA | 2500.0 mVA (p.22) | 25000 | yes |
| 15 ACVA, 24 DCVA | 0 | `dddd.d` mVA | 0.1 mVA | 2500.0 mVA (p.22) | 25000 | yes |
| | 1 | `ddddd` mVA | 1 mVA | 25000 mVA (p.22) | 25000 | yes |
| | 2 | `dd.ddd` VA | 0.001 VA | 50.000 VA (p.22) | 50000 | yes |
| | 3 | `ddd.dd` VA | 0.01 VA | 500.00 VA (p.22) | 50000 | yes |
| 16 ACuA, 17 DCuA | 0 | `dd.ddd` µA (Screen:428-435); chart ×1 | 0.001 µA | 50.000uA (p.71); 1 nA (p.18) | 50000 | yes |
| | 1 | `ddd.dd` µA, as above | 0.01 µA | 500.00uA | 50000 | yes |
| 18 ACmA, 19 DCmA | 0 | `d.dddd` mA | 0.0001 mA | 5.0000mA (p.71); 0.1 µA (p.18) | 50000 | yes |
| | 1 | `dd.ddd` mA | 0.001 mA | 50.000mA | 50000 | yes |
| 20 ACA, 21 DCA | 0 | `ddd.dd` mA | 0.01 mA | 500.00mA (p.71); 10 µA (p.18) | 50000 | yes |
| | 1 | `d.dddd` A | 0.0001 A | 5.0000A | 50000 | yes |
| | 2 | `dd.ddd` A | 0.001 A | 10.000A (p.71); 10 A, 1000 µA (p.18) | 10000 | yes |

Notes:

- **Where the sources disagree.** Only two rows remain open (§14): the diode
  3 V resolution, where both apps give 0.1 mV and the manual 1 mV, and the
  top capacitance range, where the manual gives 10.00 mF on p.71 and 9999 µF
  on p.19 [UNVERIFIED] (community: see §15.2, §15.3). In modes 7, 13/22, 14/23 and 16/17 the two apps' LCDs
  and the manual agree (ms, µVA, mVA, µA) [VENDOR]; the manual rows [KNOWN];
  the range mapping for modes 14/23 ranges 2-3 [INFERRED], §14 "VA range
  index". The difference there lies inside EEVblog's app, between its LCD and
  the multiplier its chart and log apply (×1 for modes 7, 13/22 and 16/17, VA
  for modes 14/23 ranges 2-3) [VENDOR]. That is an app inconsistency, not a
  question about the meter.
  EEVblog's table also changed between `master` and the branch for modes 13,
  14, 18 and 23 (master
  `App 112GW/App_112GW/Controls/Multimeter/Packet121GW.cs:89-113`; the VA rows
  in commit `2345065`, "Some packet corrections for milli display") [VENDOR].
- **VA ranges.** The four ranges of each VA mode fit the product of the two
  current ranges on its jack and two voltage ranges: 50 µA or 500 µA, 5 mA or
  50 mA, 500 mA or 10 A, each times 5 V or 50 V, gives 250 µVA, 2500 µVA,
  2500 µVA, 25000 µVA; 25, 250, 250, 2500 mVA; 2500 mVA, 25000 mVA, 50 VA,
  500 VA — the apps' ranges 0-3 in order and every row of the manual's
  nine-range VA table (p.22) [INFERRED]. The manual prints the 50 V factor
  ("500 VA, with 50 V and 10 A", p.22 footnote 11); a 5 V factor: none found
  in the manual, 2026-09-26. The calibration table's two VA columns (p.71: R0
  "(50uA*50V) 2500.0uVA", R1 "(500uA*50V) 25000uVA", and likewise for mVA
  and VA) are calibration points at 50 V, which are ranges 1 and 3 under this
  reading, not ranges 0 and 1 [INFERRED]. Under this reading both apps' LCDs
  show the manual's rows. EEVblog's chart multiplier for modes 14/23 ranges
  2-3 gives 250.00 and 2500.0 VA: 2500.0 VA would exceed the meter's 500 VA
  maximum, and 250.00 VA is not a row of the p.22 table [INFERRED].
  Community: see §15.2.
- **Counts.** The display is "4 ⅘ digits 55,000 counts" (p.15, p.13); p.32
  still says "50,000 count" [KNOWN]. How far above the range's full scale a
  reading goes before the meter changes range or shows OFL: none found in the
  manual, 2026-09-26 [UNVERIFIED]. Frequency has "an apparent 99,999 counts" (p.21) [KNOWN]: it
  is the only range above whose count exceeds 65535, which is what firmware
  1.22's two extra bits accommodate (p.9) [INFERRED].
- **Range index vs keys.** RANGE steps the range, including "3 V or 15 V
  diode mode" (p.33, p.51) [KNOWN]; that diode range 0 is 3 V and 1 is 15 V
  follows from the resolutions [INFERRED]. A range index past a mode's last
  row is in neither app's table (UEi clamps it, `Protocol.java:507-520`;
  EEVblog's lookup would throw) [VENDOR].

### 6.3 Value, sign, overload

- **Count:** `(byte5 >> 6) << 16 | byte7 << 8 | byte8`, 18 bits unsigned
  (Packet:169; V2 p.1) [VENDOR], [KNOWN]. UEi's app reads 16 bits (its ASCII
  format has no bits 17-16) [VENDOR].
- **Reading:** count × the count weight of (mode, range) in §6.2, negated when
  byte 6 bit 6 is set (Packet:161, :257-258; UEi `Protocol.java:167`, `:327`)
  [VENDOR]. The documents label the bit only "+/-" (V2 p.1); both apps take
  set as negative [VENDOR].
- **Overload:** byte 6 bit 7, "OFL" (V2 p.1). Both apps then show "OFL"
  instead of the digits (Screen:498-500; UEi `Protocol.java:358-360`)
  [VENDOR]; the manual calls the display OFL (p.9, firmware 1.15) [KNOWN].
  What the value bytes hold during OFL is [UNVERIFIED]; community: see §15.5.

### 6.4 Temperature

Mode 5 is a thermocouple reading with one decimal (§6.2). V2 adds a °C bit
(byte 6 bit 5) and a ℉ bit (byte 6 bit 4) (V2 p.1), which EEVblog's app uses
to pick the unit (Packet:152-163) [VENDOR]. The manual's "c"/"F" setup item
changes "the displayed temperature" of the internal sensor, shown on the
secondary display (p.60) [KNOWN]; that it also sets the unit of the mode-5
thermocouple reading is [INFERRED] from p.60 and the V2 bits. V2 also puts °C at byte 15 bit 7 and ℉ at byte
16 bit 7 (V2 p.1), which no app reads; whether those are the same state or
the annunciators is [UNVERIFIED].

---

## 7. Secondary display

The secondary display "is only used when a function that requires it is
selected" (manual p.32) [KNOWN].

### 7.1 Sub mode, byte 9

A full byte: "100 ~ 199, 0 ~ 24" (V2 p.1) [KNOWN]. Codes 0-24 are read with
the main mode table by both apps (EEVblog casts to `eMode`, Packet:171; UEi
looks up the same map, `Protocol.java:502-511`) [VENDOR]; that the meter
uses them with the main mode's meaning is [INFERRED]. UEi's app reads a sub
mode in range 0 of its mode: it sets the sub range to 0 (`Protocol.java:193`)
and takes that range's unit string (`:502-511`), so V for sub modes 1 and 2,
µA for 16 and 17 and mA for 18-21, whatever the main range [VENDOR].
EEVblog's screen lights
unit segments on the secondary display by sub mode, e.g. V for sub modes 0-4
and 11, A for 16-21, V and A for 15 and 22-24 (Screen:565-654). In the VA main
modes, `Subm` adds an "m" when the main range's prefix is m, n or u, or when
the sub mode itself is a mA or µA mode (Packet:188-216, conditions at
:203-210) [VENDOR]; so in the VA modes the secondary display shows one of the
two operands [INFERRED].

Codes of 100 and over are special displays. EEVblog's `eMode` names twelve
(Packet:60-71); UEi's table has 25, formats included (`Protocol.java:436-460`,
names from UEi strings.xml) [VENDOR]. The manual column is the display the
manual shows for that setting [KNOWN].

| Code | EEVblog | UEi name, format | Manual |
|---|---|---|---|
| 100 | `_TempC` | Temp., ℃, `%.1f` | internal temperature, "c" (p.60; "24.3c" in DC V, p.65 figure) |
| 101 | — | Temp., ℃, `%.1f` | |
| 105 | `_TempF` | Temp., ℉, `%.1f` | "F" (p.60) |
| 106 | — | Temp., ℉, `%.1f` | |
| 110 | `_Battery` | Battery, V, `bAt%.1f` | "bAt" (p.59-60) |
| 120 | `_APO_On` | APO, `APO.oN` | "APO.oN" (p.61) |
| 121 | — | APO, `APO.oN` | |
| 125 | `_APO_Off` | APO, `APO.oF` | "APO.oF" (p.61) |
| 126 | — | APO, `APO.oF` | |
| 130, 131 | `_YEAR` (130) | YEAR: sub value's low byte + 2000 (`:329-333`) | "YYYY" (p.62) |
| 135, 136, 137 | `_DATE` (135) | DATE: `%2d-%2d` of bytes 11 and 12 (`:334-341`) | "MM-DD" (p.62) |
| 140, 141, 142 | `_TIME` (140) | TIME: `%2d-%2d` of bytes 11 and 12 | "HH-mm" (p.62) |
| 150 | `_BURDEN_VOLTAGE` | "Burden Vpltage" (sic), mV, `b%.1f` | "bd.XXX" / "bd.OFF" setting (p.42-43); burden voltage on the secondary display (p.18 footnote 4) |
| 160 | `_LCD` | LCD, `Lcd-%.0f` | "LCD X" (p.61) |
| 170, 172 | — | Continuity, `DN %.0f` | "dN 30", "dN 300" (p.45-46) |
| 171, 173 | — | Continuity, `UP %.0f` | "UP 30", "UP 300" (p.45-46) |
| 180 | `_dBm` | dBm | dBm in AC V, R_REF 600 Ω (p.37) |
| 190 | `_Interval` | Interval, `In %.0f` | "In x", seconds (p.63) |

- What tells the paired and tripled codes apart (101 from 100, 136/137 from
  135, …): none found in the two apps or the manual, 2026-09-26; that they are the same setting in another state
  (for example while it is being edited) is a guess [UNVERIFIED]. Community:
  see §15.4 (continuity codes 170-173).
- Units the apps disagree on: burden voltage is mV in UEi's table but lights
  "V" in EEVblog's (Screen:659-661); the logging interval lights "m" "S" in
  EEVblog's (Screen:677-680), while the manual sets it in seconds (p.63)
  [VENDOR], [KNOWN] — [UNVERIFIED].
- The buzzer setting ("b-ON"/"b-OFF", p.61) and the Multimeter ID ("XXXXX",
  p.63): a code for either, none found in the two apps, 2026-09-26 [VENDOR].
- What bytes 9-12 hold while the secondary display is blank is
  [UNVERIFIED]; community: see §15.4.

### 7.2 Sub range, byte 10

| Bit | V2 p.1 | Meaning | Source |
|---|---|---|---|
| 7 | OFL | overload; both apps show "OFL" | Packet:172, Screen:700; UEi `:195`, `:358-360` [VENDOR] |
| 6 | +/- | set = negative | Packet:173; UEi `:196` [VENDOR] |
| 5 | k | kilo | EEVblog lights a "k" icon (Screen:686); UEi replaces the secondary unit string with "K" (`:305-311`) [VENDOR] |
| 4 | Hz | hertz | UEi replaces the secondary unit string with "Hz", or "KHz" with bit 5 (`:305-311`); EEVblog reads it, never uses it (Packet:175) [VENDOR] |
| 3 | `0` | — | [KNOWN] |
| 2-0 | Point( 0 ~ 4 ) | digits after the decimal point | EEVblog `SubIntValue / 10^SubPoint` (Packet:265; it reads bits 3-0, :176); UEi 10^−point (`:524-536`) [VENDOR] |

When the secondary display shows a frequency (the k/Hz bits): none found in
the manual, 2026-09-26; firmware 1.51's "AC mode frequency persistence issue resolved"
(p.9) is the only hint [UNVERIFIED]. Community: see §15.4.

### 7.3 Sub value, bytes 11-12

16 bits unsigned, high byte first; sign and overload in byte 10 (V2 p.1;
Packet:178-186; UEi `:202-221`) [KNOWN], [VENDOR]. V2 gives the secondary
value no extra bits (V2 p.1) [KNOWN]. For the date and time codes UEi reads
the two bytes as two numbers (§7.1) [VENDOR].

---

## 8. Bar graph, bytes 13-14

| Byte | Bit | V2 p.1 | EEVblog's reading | Tag |
|---|---|---|---|---|
| 13 | 7-5 | `0` | not read | [KNOWN] |
| 13 | 4 | USE | bar shown when the bit is **clear** (`BarOn`, Packet:219); community: see §15.2 | label [KNOWN]; polarity [VENDOR] |
| 13 | 3 | 0~150 | picks one of two tick-label sets (Packet:220; Screen:737-753); community: see §15.4 | [VENDOR]; meaning [UNVERIFIED] |
| 13 | 2 | +/- | bar sign; EEVblog's app draws a minus when the bit is set — below | [VENDOR]; the meter's use [INFERRED] |
| 13 | 1-0 | 1000 / 500 | bar scale 0 = 5, 1 = 50, 2 = 500, 3 = 1000 (Packet:85-91, :222) | [VENDOR] |
| 14 | 7-5 | `0` | masked off (Packet:223); community: bit 5 seen set, see §15.3 | [KNOWN] |
| 14 | 4-0 | BAR GRAPH 0 ~ 25 | segment count; the screen lights value + 1 segments (Packet:223; Screen:783) | [KNOWN], [VENDOR] |

The LCD drawing shows the bar with a ± sign, tick labels 0, 12, 24, 36, 48,
510 and the end labels 1000 and 500 (manual p.31) [KNOWN]; the manual says it
is "A fast updating bargraph" (p.32) that keeps updating under HOLD (p.56)
[KNOWN]. UEi's
app skips both bytes (`Protocol.java:222-226`) [VENDOR].

**Sign polarity.** The documents give no polarity (V1/V2 p.1). EEVblog's
code reads bit 2 **clear** as negative (Packet:221) since commit `c111f7e`
(2018-04-04, "Inverted bargraph sign (packet has error)"), but its screen
swaps the two sign layers: `BarTick_Minus` loads the layer "Bar+" and
`BarTick_Plus` the layer "BarTick-" (Screen:318-319), a swap older than
`c111f7e` (`git:c111f7e^:App 112GW/App_112GW/Controls/Multimeter/MultimeterScreen.cs:309-310`).
`Bar+.svg` draws a plus sign and `BarTick-.svg` a minus bar (PPR
`121GW.Core/Image/Layer Resources/Bar+.svg`, `BarTick-.svg`). So at `aab403d`
a set bit decodes as positive, selects `BarTick_Plus` and draws the minus bar
(Screen:778-781): as drawn, EEVblog's app shows a minus when the bit is set,
and `c111f7e` cancelled the layer swap rather than a packet error [VENDOR].
That the meter sets the bit for a negative bar is [INFERRED] from that
drawing; community: see §15.3.

## 9. Annunciators, bytes 15-17

Labels exactly as V2 p.1; meanings from the apps and the manual.

| Byte | Bit | V2 p.1 | Meaning | Source | Tag |
|---|---|---|---|---|---|
| 15 | 7 | °C | V2 only; not read by the apps | V2 p.1 | [KNOWN]; see §6.4 |
| 15 | 6 | 1KHz | 1 kHz low-pass filter, "Holding down [REL] in AC mode" (manual p.33) | Packet:228; UEi `:228` | [VENDOR]; meaning [INFERRED] |
| 15 | 5 | 1ms | 1 ms peak capture, "VAC only" (manual p.33, p.36); the LCD's "1ms" (p.31) | Packet:229, Screen:795-799; UEi `:229` | [VENDOR]; meaning [INFERRED] |
| 15 | 4-3 | DC + AC | 0 none, 1 DC, 2 AC, 3 DC+AC | Packet:78-84, :230; UEi `:230`, `:463-474` | [VENDOR] |
| 15 | 2 | AUTO | autorange | Packet:231; UEi `:231` | [VENDOR] |
| 15 | 1 | APO | auto power-off armed ("Apo" on the LCD, p.31) | Packet:232; UEi `:232` | [VENDOR]; meaning [INFERRED] |
| 15 | 0 | BAT | low battery, below about 4.2 V (manual p.16) | Packet:233; UEi `:233` | [VENDOR], [KNOWN] |
| 16 | 7 | ℉ | V2 only; not read by the apps | V2 p.1 | [KNOWN]; see §6.4 |
| 16 | 6 | BT | Bluetooth on (manual p.55) | Packet:235, Screen:881 | [VENDOR] |
| 16 | 5 | ↙ | meaning: none found in V1/V2 or the manual, 2026-09-26; EEVblog names it `StatusArrow` and never uses it; community: see §15.4 | Packet:236 | [UNVERIFIED] |
| 16 | 4 | REL | relative mode (manual p.57) | Packet:237; UEi `:237` | [VENDOR] |
| 16 | 3 | dBm | dBm | Packet:238, Screen:822 | [VENDOR] |
| 16 | 2-0 | MIN/MAX | 1 MAX, 2 MIN, 3 AVG, 4 MAX+MIN+AVG | Screen:840-858 (its comment: "UNKONWN MIN/MAX bits config", :824); UEi `:238`, `:476-489` | [VENDOR]; 5-7 [UNVERIFIED] (community: see §15.4) |
| 17 | 7 | `0` | — | V2 p.1 | [KNOWN] |
| 17 | 6 | TEST | EEVblog lights a TEST icon; its meaning: none found in the manual, 2026-09-26; community: see §15.4 | Packet:241, Screen:825 | [VENDOR]; meaning [UNVERIFIED] |
| 17 | 5-4 | MEM | non-zero lights MEM | Packet:242, Screen:826; UEi `:244` | [VENDOR]; the values [UNVERIFIED] (community: see §15.4) |
| 17 | 3-2 | A-HOLD | 1 = A-HOLD, 2 = HOLD | Screen:828-839; UEi `:242`, `:491-500` | [VENDOR]; the manual's "A-" and "HOLD" (p.56) [KNOWN] |
| 17 | 1 | AC | UEi: bits 1-0 are the secondary display's AC/DC, coded as bits 4-3 of byte 15 | UEi `:243`, `:300` | [VENDOR]; EEVblog reads, never uses (Packet:244-245); community: see §15.4 |
| 17 | 0 | DC | as above | | |

The manual names MIN, MAX, AVG and all three together (p.33, p.58) but gives
no cycle order [KNOWN]; the codes are the apps' [VENDOR].

## 10. Identity, bytes 1-4

"Each digit 0 – 9 of the serial number is stored separately" (V2 p.1)
[KNOWN]: five decimal digits, one per nibble, Digit 4 in byte 2's low nibble
down to Digit 0 in byte 4's low nibble. EEVblog's app reads them as a
five-digit number, Digit 4 the most significant (Packet:139-144), and shows
it as the tab title "121GW " + serial (assigned at Multimeter:121; title set
at PPR `121GW.Core/Controls/Multimeter/MultimeterPage.cs:14-25`) [VENDOR]. It reads byte 1 as a binary
year after 2000 (Packet:137) and byte 2's high nibble as a month
(Packet:138) [VENDOR]. Both community captures contradict the binary year:
they carry `0x17`, which reads 2023 that way, later than either capture
(§15.3, §15.5). How the documents' "Year ( 4 … 0 )" and
"Month (1 … 0)" are encoded: none found in V1/V2, 2026-09-26.

The manual's Multimeter ID is a user-set five-digit number ("XXXXX") that
"appears in the App" to tell several 121GWs apart (p.63) [KNOWN]. That the
serial digits carry it is [INFERRED] from the app showing them; what the year
and month record is [UNVERIFIED]. Community: see §15.4.

---

## 11. Commands

Both command formats are ASCII hex behind a binary header byte; neither app
writes anything else [VENDOR]. They go to the characteristic of §2.

### 11.1 Key press, `F4`

`F4`, the key code as two ASCII hex digits, then two more ASCII hex digits
that repeat the code. Both apps hold the sixteen frames as constants
(EEVblog Packet:314-329; UEi `MainActivity.java:54-69`) [VENDOR]; no code
computes them. That the second pair is a checksum, the XOR of the one-byte
payload, is [INFERRED] from UEi's checksum rule for the clock set
(`MainActivity.java:307-313`) and its parser's (`Protocol.java:315-319`).
A long press sets bit 7 of the code.

| Code | Button (manual p.33) | Short press | Long press (code \| 80) | What the button does held (manual) |
|---|---|---|---|---|
| 01 | RANGE | `F4 30 31 30 31` | `F4 38 31 38 31` | none found in the manual, 2026-09-26 |
| 02 | HOLD | `F4 30 32 30 32` | `F4 38 32 38 32` | none found in the manual, 2026-09-26 |
| 03 | REL | `F4 30 33 30 33` | `F4 38 33 38 33` | 1 kHz low-pass filter in AC (p.33) |
| 04 | 1ms PEAK | `F4 30 34 30 34` | `F4 38 34 38 34` | Bluetooth on/off (p.33, p.55) |
| 05 | MODE | `F4 30 35 30 35` | `F4 38 35 38 35` | backlight (p.33, p.55) |
| 06 | MIN/MAX | `F4 30 36 30 36` | `F4 38 36 38 36` | exit MIN/MAX (p.58) |
| 07 | MEM | `F4 30 37 30 37` | `F4 38 37 38 37` | start/stop SD logging (p.33, p.54) |
| 08 | SETUP | `F4 30 38 30 38` | `F4 38 38 38 38` | edit/save the setup item (p.33, p.60) |
| 09 | buzzer (UEi only) | `F4 30 39 30 39` | — | — |

- Both apps carry all sixteen codes 01-08 and 81-88 byte for byte [VENDOR].
  The button for each code is the constant's name in both (`KEYCODE_RANGE`,
  …, `KEYCODE_SETUP`) and UEi's button labels, "Range", "A-Hold", "Relative",
  "1msPEAK", "MAX/MIN", "SD MEM", "SET UP" (UEi strings.xml:60-79;
  `MainActivity.java:147-178`, `:197-224`, `:239-241`) [VENDOR]; the manual
  button names are [INFERRED] from them.
- EEVblog's app offers only HOLD, REL, MODE and RANGE, short presses
  (Multimeter:160-163); its `Keycode` enum has no long MODE, so
  `KEYCODE_LONG_MODE` is never sent (Packet:326, :331-348) [VENDOR]. UEi's
  sends all sixteen [VENDOR].
- `09` exists only in UEi's app, sent by the device list's "Identify" button:
  the click starts the connection, the frame goes one second later and the
  app disconnects a second after that (UEi `DeviceListAdapter.java:57-76`,
  `BLEConnection.java:21`, `:93-103`) [VENDOR]; that it sounds the buzzer is [INFERRED] from the name
  `KEYCODE_BUZZER`.
- **Whether current firmware accepts these frames is [UNVERIFIED].** Both
  apps' key frames date from the ASCII era (identical on EEVblog's `master`,
  `App 112GW/App_112GW/Controls/Multimeter/Packet121GW.cs:281-296`, and at
  `git:0f10ac7^`). EEVblog's final branch (2018-09-04), six months after the
  switch to the binary packet, still sends them unchanged (Packet:314-329):
  evidence that the command format did not change with the packet [INFERRED].
  Community (firmware 1.02 only): see §15.4.
- **Replies.** UEi's parser accepts an `F4`-led packet: `F4`, nine raw bytes,
  the key code and its checksum, as ASCII hex (`Protocol.java:117-162`)
  [VENDOR code; the frame shape INFERRED]. Whether the meter replies to a key,
  and in which format under the binary packet, is [UNVERIFIED]; community
  (firmware 1.02 only): see §15.4.

### 11.2 Clock set, `F8` (UEi's app only)

```
raw:   F8 YY MM DD hh mm ss CK      YY = year − 2000, MM 1-12, hh 24-hour
       CK = YY ^ MM ^ DD ^ hh ^ mm ^ ss
wire:  F8, then the seven raw bytes after F8 as 14 upper-case ASCII hex digits
```

(UEi `MainActivity.java:113-136`, `:293-313`) [VENDOR]. The app sends it once
a second while connected until it parses an `F8`-led reply, nine raw bytes
and six time bytes whose XOR matches the seventh (`Protocol.java:202-205`),
and never again after that [VENDOR]. EEVblog's app never sends it [VENDOR].

The manual sets the date and time by hand in SETUP (p.62) and uses them for
logged files' metadata (firmware 1.51, p.9) [KNOWN]; setting them over
Bluetooth: none found in the manual, 2026-09-26. Whether the meter acts on `F8` or replies is
[UNVERIFIED]; community (firmware 1.02 only): see §15.4.

---

## 12. Earlier firmware formats

Before the binary packet the meter sent ASCII: `F2`, then the same fields as
ASCII characters, except the serial number. The V1 document says so: "The
previous packet was 54 bytes", "The actual data format is identical, except
for the way serial number is formatted", and "Apart from encoding all the
fields are identical to the implementation prior to this change" (V1 p.1;
V2 p.1 repeats all three) [KNOWN]. The same page adds "The inactive sections
(See page 4) of the packet should also be dropped"; neither document has a
page 4 (both are three pages) [KNOWN]. That the dropped sections include the
two extra mode/range/value groups UEi's app reads in the 54-byte packet
(below) is [INFERRED]. Two traits
separate every ASCII packet from the binary one: every byte after `F2` is an
ASCII digit or letter, and the packet is longer than 19 bytes [INFERRED from
the apps below]. Their full decode is out of scope here; this is how the
apps recognised them.

| Reader | After `F2` | Content | Source |
|---|---|---|---|
| EEVblog's app, up to 2017-10 | 26 characters | 13 bytes as hex pairs, in the order of binary bytes 5-17; no serial | `git:0f10ac7^:App 112GW/App_112GW/Controls/Multimeter/Multimeter.cs:43`; `git:0f10ac7^:App 112GW/App_112GW/Packet/Packet121GW.cs:438-444` |
| EEVblog's app, 2017-10 to 2017-12 (still `master`) | 52 characters taken, all alphanumeric | 9 decimal digits (year 2, month 2, serial 5), then hex pairs for binary bytes 5-17 in characters 9-34; characters 35-51 not read | master `App 112GW/App_112GW/Controls/Multimeter/Multimeter.cs:44`; master `…/Packet121GW.cs:146-193`, `:255-266` |
| EEVblog's app, 2017-12 to 2018-03 | 37-40 characters | 9 decimal digits, hex pairs, and a final hex pair that is the XOR of the preceding hex bytes | `git:7490aca:…/Multimeter.cs:44`; `git:2761b11:…/Packet121GW.cs:272-285`, `:352-390` |
| UEi's app 1.0.6 | 53 characters, 54 bytes with `F2` | 9 raw bytes (the serial, never used), then 22 hex pairs: binary bytes 5-17's fields (the bar bytes skipped), two more mode/range/value groups the app calls "SUB1"/"SUB2", and an XOR checksum of the 21 bytes before it | UEi `Protocol.java:111-321` |

- UEi's reading is the only coherent one of `putData` as jadx renders it: one
  switch step per decoded byte, which the per-character stepping in the jadx
  output contradicts [INFERRED; jadx output, not checked against smali].
- UEi's 54 bytes match the V1 document's "54 bytes"; EEVblog's `master`
  takes the first 52 of the same 53 characters after `F2` [INFERRED].
- The 26- and 37-40-character readers may reflect what those app versions
  accepted rather than separate firmware formats [UNVERIFIED].
- Which firmware versions sent which format: none found in the documents,
  the apps or the manual's changelog (p.9-10), 2026-09-26 [UNVERIFIED]. The
  binary packet dates from 2018-03-22 by the V1 document and EEVblog's commit
  `43b3e94`; firmware 1.21 and 1.22 (§1) came after [INFERRED]. Community:
  the firmware 1.02 frame, see §15.4.
- EEVblog's `master` accepts `F2` plus 52 alphanumeric characters only
  (`Packet121GW.cs:255-266`), so it cannot read the binary packet; UEi's 1.0.6
  cannot either, since it hex-decodes every byte after the ninth [INFERRED].

---

## 13. Worked examples

Constructed from the tables above, not captured; the checksums were computed
for this document and checked by XOR-ing all 19 bytes to zero. Bytes 1-4 in
all four are an invented identity: year byte `12`, month 6, serial 12345.

Zero bytes are not neutral. The bar bytes are `00 00` in all four, which
EEVblog's app renders as bar shown (USE clear), sign negative (its final
polarity), scale 5 and one segment (Packet:219-223, Screen:783); which bar
state the meter means by it is open (§8). In examples 2-4 the secondary bytes
are also `00`: sub mode 0 is `Low_Z`, point 0, value 0, so EEVblog's app
would show a secondary reading of 0 V (Packet:171, Screen:565-567); code 0 is
Low-Z in UEi's map too (`Protocol.java:411`), in its own format. What the
meter sends for a blank secondary display is open (§7.1).

**1. 1.2345 V DC, AUTO, BT; secondary display 24.3 (internal temperature, as
the manual's p.65 figure)**

```
F2 12 61 23 45 01 00 30 39 64 01 00 F3 00 00 0C 40 00 35
```

Byte 5 = `01`: DCV. Byte 6 = `00`: range 0 (`d.dddd` V), positive. Bytes 7-8
= `3039` = 12345 → 1.2345 V. Byte 9 = `64` = 100, `_TempC`; byte 10 = `01`:
one decimal; bytes 11-12 = `00F3` = 243 → 24.3. Byte 15 = `0C`: DC (bits 4-3 =
1) and AUTO (bit 2). Byte 16 = `40`: BT. Checksum `35`.

**2. 87.654 Hz — an 18-bit value**

```
F2 12 61 23 45 46 00 56 66 00 00 00 00 00 00 04 40 00 D5
```

87654 = `0x15666`. Byte 5 = `46`: bits 7-6 = `01` (value bit 16), mode 6 (Hz).
Byte 6 = `00`: range 0 (`dd.ddd` Hz). Bytes 7-8 = `5666`. Count = 1 × 65536 +
`0x5666` (22118) = 87654 → 87.654 Hz. Secondary bytes `00` (see above;
what a blank secondary display sends is open). Checksum `D5`.

**3. Overload, resistance, 50 MΩ range**

```
F2 12 61 23 45 09 86 00 00 00 00 00 00 00 00 04 40 00 2C
```

Byte 5 = `09`: Resistor. Byte 6 = `86`: OFL (bit 7), range 6 (50.000 MΩ).
Value bytes zero (their content under OFL is open, §6.3). Checksum `2C`.
A community capture of the same reading is in §15.5.

**4. −12.345 mV DC, AUTO**

```
F2 12 61 23 45 03 40 30 39 00 00 00 00 00 00 0C 40 00 E1
```

Byte 5 = `03`: DCmV. Byte 6 = `40`: sign (bit 6), range 0 (`dd.ddd` mV).
12345 → −12.345 mV. Checksum `E1`.

**Commands** (from the apps' byte tables; the clock set computed by the
rule of §11.2 for 2026-09-26 14:05:30):

| Command | Bytes |
|---|---|
| RANGE, short | `F4 30 31 30 31` |
| HOLD, long | `F4 38 32 38 32` |
| Clock set | raw `F8 1A 09 1A 0E 05 1E 1C` → wire `F8 31 41 30 39 31 41 30 45 30 35 31 45 31 43` |

---

## Implementation Notes

What the wire requires of any decoder:

- A packet is 19 bytes from `F2`, and the XOR of bytes 0-17 equals byte 18.
  `F2` is not escaped, so a value byte can equal it: the start byte alone
  does not mark a packet boundary, the checksum does.
- Multi-byte values are big-endian. The main value is 18 bits, its top two
  bits in byte 5 bits 7-6, so the mode is byte 5 bits 4-0 only.
- Values are unsigned magnitudes with a separate sign bit, and a separate OFL
  bit for each display.
- The main display's decimal point and unit prefix are not in the packet:
  they follow from the mode and range codes (§6.2). The secondary display
  carries its decimal point (byte 10 bits 2-0) and k/Hz flags, and its unit
  follows from the sub mode.
- Byte 9 is a full byte: codes 100 and over are setup items and auxiliary
  readings, such as internal temperature, battery voltage, burden voltage and
  dBm (§7.1).
- A V1 packet is a V2 packet with the V2-only bits at zero.
- Commands are ASCII hex behind a binary `F4`/`F8`; the packets the meter
  sends are binary.

## 14. Open questions — [UNVERIFIED]

Each item says whether the community sources (§15) answer it, narrow it or
leave it open. An answer from firmware 1.02 is the ASCII-era firmware, carried
over by V1 p.1's "identical" statement; none replaces a check on a current
meter.

1. **Advertised name** — what the meter advertises; EEVblog's filter admits
   "121GW" and "Bluegiga" (§2). **Answered** [COMMUNITY]: "121GW", seen on a
   meter; see §15.4.
2. **Notify or indicate, write type** — whether `e7add780-…` also notifies,
   and whether it takes write with or without response (§2). **Narrowed**:
   indications seen and a notify-only client works; the write type stays open;
   see §15.4.
3. **Packet rate and chunking** — packets per second against the display's 5
   updates a second, and whether one indication is always one packet (§3, §4).
   **Narrowed**: about 2 packets a second, and 18-byte values without `F2`,
   seen on a 2022 meter; see §15.4.
4. **Firmware ↔ format** — which firmware first sent the binary packet and
   which sent each ASCII format (§1, §12). **Narrowed**: firmware 1.02 sends
   the ASCII frame; the first binary firmware stays open; see §15.4.
5. **Commands on current firmware** — whether the ASCII `F4` key frames, code
   `09` and the `F8` clock set are honoured, and whether the meter replies
   (§11). **Narrowed** for firmware 1.02 only (accepted and echoed); current
   firmware open; see §15.4.
6. **Diode 3 V resolution** — 0.1 mV (both apps) or 1 mV (manual p.20) (§6.2).
   **Narrowed**: firmware 1.02 gives 0.1 mV; see §15.2.
7. **Top capacitance range** — 9999 µF (p.19) or 10.00 mF (p.71) (§6.2).
   **Narrowed**: firmware 1.02 gives 1 µF per count (9999 µF); see §15.3.
8. **VA range index** — whether ranges 0-3 are the current × voltage products
   of §6.2's note; both apps' LCDs and the manual's VA table (p.22) fit it,
   including mVA for modes 14/23 ranges 2-3 (§6.2). **Narrowed**: firmware
   1.02 composes the ranges this way; see §15.4.
9. **DC+AC V** — which mode code and flags the meter sends in the V position's
   third MODE step (§6.1). **Narrowed**: firmware 1.02 keeps mode 2 with AC/DC
   code 3; see §15.4.
10. **µVA on the µA position** — how codes 13/22 are selected on the meter
    (§6.1). **Narrowed**: MODE on the µA position, per firmware 1.02; see
    §15.4.
11. **Over-range counts and OFL value** — how far past full scale a range
    reads before OFL or a range change, and what the value bytes carry during
    OFL (§6.2, §6.3). **Narrowed**: one capture has value `00 00` under OFL;
    the thresholds stay open; see §15.5.
12. **V2 temperature bits** — whether byte 15/16 bit 7 duplicate byte 6 bits
    5/4 (§6.4). **Open**: no community answer.
13. **Special sub codes** — what separates the paired and tripled codes, which
    continuity code is which threshold, the burden-voltage and interval units
    (§7.1). **Narrowed**: the continuity thresholds are settled; the paired
    codes and the units stay open; see §15.4.
14. **Blank secondary display** — what bytes 9-12 hold when the secondary
    display is off (§7.1). **Narrowed**: all zeros in firmware 1.02; see
    §15.4.
15. **Sub k/Hz flags** — when the secondary display shows a frequency (§7.2).
    **Narrowed**: frequency with Hz (and k from range 2) in AC V and mV, seen
    on a meter; see §15.4.
16. **Bar graph** — the USE polarity, the 0~150 bit, the 0-25 scale against
    the range, and the sign polarity (§8). **Narrowed**: USE and 0~150 by
    firmware 1.02, the sign by EEVblog's drawing (§8) and firmware 1.02; the
    scale stays open; see §15.2-15.4.
17. **Unnamed and multi-state annunciators** — ↙ (byte 16 bit 5), TEST (byte
    17 bit 6), the MEM values, MIN/MAX values 5-7, byte 17 bits 1-0 as the
    secondary display's AC/DC (§9). **Narrowed** by firmware 1.02 (↙ = danger
    icon, and the values it uses); see §15.4.
18. **Identity bytes** — what the year and month record, and whether the
    serial digits are the Multimeter ID (§10). **Narrowed**: year BCD in both
    captures; calibration year-month and Meter ID per firmware 1.02; see
    §15.3, §15.4.
19. **Auto power-off and Bluetooth** — whether a connection holds off APO
    (§3). **Narrowed**: firmware 1.02 does not hold off APO for Bluetooth; see
    §15.4.

---

## 15. Cross-reference with community sources [COMMUNITY]

Read 2026-09-26, after §1-14 were written from the vendor sources and
grounding-checked; nothing here was merged into §1-14 beyond pointers. The
boundary covered
code repositories and sigrok only, no forum posts
(`reverse-engineering-approach.md`). Every point a community source disputes
was re-read in the vendor sources alone; where that re-read changed a verdict
(the bar sign, §8; the year byte, §10), the body says so and points here.
Working notes: `references/121gw/analysis/findings/cross-reference.md`.

How the sources were made matters more than how many agree:

- **evotronix** is AI-generated from EEVblog's app: each README says "made
  with Grok AI" or "generated with Grok AI" (e.g.
  `evotronix/121GW-Android-port/README.md:1`), and its Swift decoder says it
  "matches official Packet121GW.cs"
  (`evotronix/121GW-port-for-iphone-and-macos/121GW_swift/PacketV2.swift:3`).
  It holds no captures, so where it matches §1-14 it restates EEVblog's app;
  it is not independent evidence.
- **121gwcli**'s `src/` is copied from 121gw-qt5 ("This file is originally
  from the project: https://github.com/zonque/121gw-qt5",
  `121gwcli/src/packetparser.cpp:1-4`), but its own scripts,
  `121gwcli.sh` and `parse121gw.pl`, were run on a real meter: the README
  shows their timestamped output (`121gwcli/README.md:46-48`), `o.log` holds a
  run, and "Tested on Debian 11 bullseye" (`README.md:17`).
- **121gw-re** (tpwrules) is a disassembly of **firmware 1.02** (added
  2018-01-22), which sends the older ASCII frame (§12). Its facts carry over to
  the binary packet only through V1 p.1's "The actual data format is identical,
  except for the way serial number is formatted" [INFERRED], less what later
  firmware changed (the 1.21/1.22 bits, §1). Symbol names are tpwrules'
  guesses. Every fw 1.02 fact below is scoped that way; none is a statement
  about current firmware.
- **sigrok** and **121gw-qt5** are independent decoders of the binary packet,
  written against the V2 document's field layout; sigrok quotes one packet a
  meter sent.

### 15.1 Sources

| Source | Covers | Hardware captures | Licence |
|---|---|---|---|
| [tpwrules/121gw-re](https://github.com/tpwrules/121gw-re) `38f558d` (2018-08-14) | IDA export of firmware 1.02: the ASCII packet builder, the `F4`/`F8` receiver, key mapping, packet timer, APO, annunciator logic | none; disassembly | none in the repository (it also redistributes the vendor firmware and PDFs) |
| [zonque/121gw-qt5](https://github.com/zonque/121gw-qt5) `02f0d15` (2019-03-10; first commit 2018-08-14) | Qt client: scan filter on the service UUID, CCCD write, an 18-byte packet struct, mode enum, icons, bar | none in the repository; a working client implies live data | GPL-2.0 (`LICENSE.GPL2`) |
| [chlordk/121gwcli](https://github.com/chlordk/121gwcli) `d543790` (2022-12-30) | gatttool + Perl logger: name, ATT handles, indications, 18-byte values, rate; `src/` copied from qt5 | yes: one gatttool line (`parse121gw.pl:159`), timestamped output and a JSON record (`README.md:46-48`, `:166-179`), `o.log` | GPL-3.0 |
| [evotronix](https://github.com/evotronix) Android, Chrome, iOS/macOS and Windows ports (2026-08/09; the store page's "Open source ports") | full decoder, key frames, both UUIDs | none | MIT © 2026 evotronix |
| libsigrok `src/dmm/eev121gw.c` (© 2018 Gerhard Sittig; last change `d66940a`, 2022-08-21), `src/serial_bt.c`, `src/hardware/serial-dmm/api.c` | full decoder, range tables from the manual's calibration table ("page 69 in the 2018-09-24 manual", `eev121gw.c:317-320`), BLE handles, scan name | one packet (`eev121gw.c:86`) and remarks on what was "seen" | `eev121gw.c` GPL-2.0-or-later; `serial_bt.c`, `api.c` GPL-3.0-or-later |

Community paths below are relative to `references/121gw/community/`.

### 15.2 Agree

| Spec § | What the community sources show | Evidence |
|---|---|---|
| §2 service, characteristic | Service `0bd51666-…` (qt5 `multimeter.cpp:24`, evotronix Chrome `js/ble.js:2`); characteristic `e7add780-…` in evotronix only (`121GW_swift/BLEManager.swift:8`, `js/ble.js:1`) | code; evotronix not independent |
| §2 advertised service UUID | qt5 lists only devices advertising the service UUID (`mainwindow.cpp:22`) | a working client |
| §3 no start command, no keep-alive | sigrok never writes (write handle 0, `serial_bt.c:265-273`; "no further request is needed", `eev121gw.c:69-76`); 121gwcli writes only the CCCD (`121gwcli.sh:10`) | a meter (121gwcli) |
| §4 framing | 19 bytes from `F2`; XOR of bytes 0-17 = byte 18; big-endian values (sigrok `eev121gw.c:146-230`; qt5 `packetparser.cpp:26-47`); both 2018/2022 packets of §15.5 pass the check | captures |
| §5 layout | Every field position of bytes 5-17 (sigrok `eev121gw.c:159-227`; qt5 `packetparser.h:56-103`); fw 1.02 builds the same fields in the same order (`EEVBlog-102.c:7259-7451`) | code, disassembly |
| §6.1, §7.1 mode codes | 0-24 and the sub codes 100-190 (sigrok `eev121gw.c:235-279`; qt5 `packetparser.h:16-54`; fw 1.02 enum, `121gw-re/database/EEVBlog-102 - enumerations.txt:26-66`, which also has an internal 25 `MM_BURDEN`) | code, disassembly |
| §6.2 ranges per mode | fw 1.02 `ranges_in_mode = {1,4,4,2,2,1,5,3,1,7,1,1,6,4,4,4,2,2,2,2,3,3,4,4,4}` (`EEVBlog-102.c:1729`) matches §6.2's row counts; diode 15 V goes out as range 1 (`:7276`) | disassembly |
| §6.2 count weights | fw 1.02's decimal places per mode and range (`EEVBlog-102.c:8284-8480`) match for capacitance, diode (4 decimals at 3 V, 3 at 15 V, `:8341-8346`, answering §14.6 for fw 1.02), µVA/mVA/VA and A; sigrok's tables match except §15.3 D2-D3 | disassembly, code |
| §7.2 point and k | point = decimals, k = ×1000 (sigrok `eev121gw.c:521-582`, `:1084-1087`) | code |
| §8 USE, scale | Bar shown when USE is clear, scale codes 0-3 = 5/50/500/1000 (sigrok `eev121gw.c:1178`, `:292-297`; qt5 `multimeter.cpp:471-484`); fw 1.02 sets USE in Hz, pulse width, duty, capacitance and temperature (`EEVBlog-102.c:7334-7337`), and the 2018 duty packet (§15.5) has it set | disassembly, a capture |
| §9 byte 15 bits 4-3 | 0/1/2/3 = none/DC/AC/DC+AC (sigrok `eev121gw.c:300-305`; fw 1.02 `:7346-7372`) | code, disassembly |
| §9 hold, MIN/MAX | fw 1.02 `hold_get_status` returns 2 for manual hold, 1 for auto hold (`EEVBlog-102.c:4792-4811`), shifted into bits 3-2 (`:7405`); the MIN/MAX mode is added into bits 2-0 (`:7399-7402`) | disassembly |
| §11 command frames | fw 1.02 accepts `F4` plus two equal hex pairs (`EEVBlog-102.c:7176-7186`) and `F8` plus seven hex pairs whose XOR checks (`:7137-7174`); its key codes 1-8 and 0x81-0x88 map to RANGE, HOLD, REL, PEAK, MODE, MIN/MAX, MEM, SETUP as §11.1 (`:15090-15190`); evotronix builds the same `F4` frames (`BLEManager.swift:65-79`) | disassembly |
| §12 54-byte ASCII packet | fw 1.02 writes `F2`, 9 identity digits, 22 hex pairs and CR LF, 56 bytes (`EEVBlog-102.c:7259-7451`): V1's "54 bytes" plus CR LF, and UEi's reader | disassembly |
| §7.1 internal temperature | Sub code 100/105 with one decimal (fw 1.02 `EEVBlog-102.c:7472-7500`); both packets of §15.5 carry it | captures |

### 15.3 Disagree

| # | Spec § | Community | Vendor re-check | Verdict |
|---|---|---|---|---|
| D1 | §8 bar sign, byte 13 bit 2 | set = negative: sigrok `eev121gw.c:1185`, qt5 `multimeter.cpp:487` and `display.cpp:396-400`, evotronix `PacketV2.swift:381`; fw 1.02 adds the bit when the bar reading is negative (`EEVBlog-102.c:7338-7339`) | EEVblog's app reads clear as negative but swaps its sign layers, so as drawn it shows a minus when the bit is set (§8) | the vendor drawing and fw 1.02 agree on set = negative; no negative-bar capture exists |
| D2 | §6.2 capacitance range 5 | sigrok: 10 µF per count, "10.00m" (`eev121gw.c:428`), from the manual's calibration table (its comment, `:317-320`) | both apps: 1 µF per count; the manual disagrees with itself (p.19 9999 µF, p.71 10.00 mF) | fw 1.02 gives range 5 zero decimals (`EEVBlog-102.c:8319-8323`) and logs capacitance ranges 2-5 in "uF" (`:13391-13399`): the apps and p.19 are supported; sigrok's value is manual-derived |
| D3 | §6.2 VA range 2 | sigrok labels it "25.000VA" (`eev121gw.c:437`), with the same 0.001 VA weight | 50.000 VA (p.22) | a label slip in sigrok; the weight agrees |
| D4 | §5, §8 byte 14 bits 7-5 = `0` (V2 p.1) | bit 5 is set on real meters: sigrok "Bit 5 of "bar value" was seen with value 1" (`eev121gw.c:85-87`), and both packets of §15.5 (`37`, `21`) | EEVblog masks bits 7-5 (Packet:223) | the document is wrong for real meters; the bit's meaning is unknown |
| D5 | §10 year byte | both captures carry `0x17` in byte 1 | EEVblog reads binary + 2000 (Packet:137): 2023 | binary gives a year later than either capture (sigrok's file last changed 2022-08, 121gwcli 2022-12); as BCD `17` reads 2017, and fw 1.02 sends these as decimal digits (`EEVBlog-102.c:7235-7250`); EEVblog's reading is contradicted, the V2 document is silent |
| D6 | §9 MIN/MAX | qt5 reads bits 0, 1, 2 as independent AVG, MAX, MIN flags (`packetparser.h:74-76`) | both apps enumerate 1-4 | fw 1.02 adds the mode value (`EEVBlog-102.c:7399-7402`): the spec's enumeration holds; qt5 is wrong |
| D7 | §9 hold | qt5: bit 2 HOLD, bit 3 "A-" (`packetparser.h:85-86`); evotronix: 1 = HOLD, 2 = A-HOLD (`PacketV2.swift:615-619`) | both apps: 1 = A-HOLD, 2 = HOLD | fw 1.02 `hold_get_status` (`EEVBlog-102.c:4792-4811`): the spec holds |
| D8 | §7.1 sub codes 101, 170-175 | evotronix: 101 = buzzer, 170/175 = "Calibration" (`PacketV2.swift:305-306`, `:328-329`) | UEi: 101 = Temp. ℃, 170-173 = continuity | 170-173 = continuity thresholds is supported (§15.4); 101 undecided: fw 1.02 never sends it and evotronix gives no evidence |

### 15.4 New

Facts §1-14 lack or mark [UNVERIFIED]. "Seen" is a meter observation; fw
1.02 is the ASCII-era disassembly, scoped as above.

| Topic (§) | Finding | Source |
|---|---|---|
| Name (§2, §14.1) | "121GW": `bt-device --list` shows a meter under that name (`121gwcli/README.md:221`, which also prints its address), and 121gwcli selects the first device matching it (`121gwcli.sh:5`); sigrok's scan table matches the name exactly (`sigrok/serial_bt.c:82`, `strcmp` at `:103`). "Bluegiga" was not seen | seen |
| ATT handles, CCCD (§2, §14.2) | Value handle `0x0008`, CCCD handle `0x0009`; writing `03 00` to the CCCD starts the stream (`121gwcli.sh:10`; sigrok `serial_bt.c:265-273`, CCCD value `0x0003`) | seen |
| Indications (§2, §14.2) | gatttool prints "Indication handle = 0x0008" (`parse121gw.pl:159-160`); qt5 writes only `01 00` (notify) to every CCCD and decodes packets (`multimeter.cpp:36-44`), which implies notifications work too | seen; implied |
| Write type (§2, §14.2) | no community answer: sigrok, 121gwcli and qt5 never write commands (qt5 `TODO.md`: "Implement logic to send button codes to the device") | — |
| Chunking (§4, §14.3) | GATT values are 18 bytes, bytes 1-18 of the packet without `F2`: 121gwcli matches exactly 18 bytes and checks that their XOR is `F2` (`parse121gw.pl:160-162`); qt5 requires `sizeof(PacketV2)` = 18 and seeds its checksum with `0xF2` (`packetparser.cpp:5-41`). So a packet spans more than one indication; no source shows the indication carrying `F2` | seen |
| Rate (§3, §14.3) | About 2 packets a second, 495 ms apart, on a 2022 meter (`121gwcli/README.md:46-48`, `o.log:1-4`). fw 1.02 sent one every 250 ms (a 25 × 10 ms counter, `EEVBlog-102.c:26580-26585`, sent at `:26868-26873`) | seen; fw 1.02 |
| Module profile (§2) | evotronix calls the service Bluegiga's "BLE112 Cable Replacement profile" (`121GW-Chrome-extension/README.md:6`), without evidence | evotronix only |
| fw 1.02 frame (§12, §14.4) | `F2`; 4 digits of the last calibration year-month and 5 digits of the Meter ID (`EEVBlog-102.c:7235-7250`); hex pairs for mode, range, value, sub mode/range/value, bar status/value, icons 1-3; a SUB1 group (frequency in AC modes, or the VA volts operand) and a SUB2 group (the VA amps operand) (`:7410-7445`); an XOR of the field bytes, excluding `F2` and the identity digits (`:7446`); CR LF (`:7447-7448`). Binary packets were decoded by qt5 by 2018-08; which firmware first sent them: no community answer | fw 1.02 |
| fw 1.02 receive side (§11, §14.5) | The receiver syncs on the first byte: `F4` → 5 bytes, `F8` → 15 bytes, anything else dropped (`EEVBlog-102.c:26688-26722`). A valid frame is echoed back as received (`bt_echo_rx_msg`, `:7101-7107`, called at `:7158`, `:7182`). Code 9 maps to no key (`:15090-15190`). `F8` sets the clock once per Bluetooth power-on (`bt_set_the_time`, `:7159-7168`, cleared at `:16371`). The echo is shorter than the reply UEi's parser expects (§11.2) [INFERRED]. Current firmware: no community observation | fw 1.02 |
| VA ranges (§6.2, §14.8) | `calc_power_ranges`: range 0 = low volts × low amps, 1 = high volts × low amps, 2 = low volts × high amps, 3 = high volts × high amps, the high amps range being 10 A in ACVA/DCVA (`EEVBlog-102.c:14944-14986`); the danger check treats VA ranges 1 and 3 as the volts range where 30000 counts = 30 V (`:8997-9004`), which fits 50 V there and 5 V in ranges 0 and 2 [INFERRED] | fw 1.02 |
| DC+AC V (§6.1, §14.9) | MODE from ACV sets a DC+AC flag and leaves the mode at 2 (`meter_enable_acv_dcv_mode`, `EEVBlog-102.c:14863-14890`); byte 15 bits 4-3 = 3 (`:7352-7354`); no frequency in SUB1 then (`:7413`) | fw 1.02 |
| µVA (§6.1, §14.10) | MODE cycles DCµA → ACµA → DCµVA → ACµVA on the µA position (tpwrules' comment, `EEVBlog-102.c:14679-14690`) | fw 1.02 |
| Special sub codes (§7.1, §14.13) | 170-173 are the continuity settings: 170 = 30 Ω beep below, 171 = 30 Ω above (break), 172 = 300 Ω below, 173 = 300 Ω above; sigrok: "only seen during setup" (`eev121gw.c:1107-1139`); fw 1.02 writes the setting as the code (`EEVBlog-102.c:7605`). In diode, sub code 11 carries the test voltage, 3 or 15 (`:7612-7616`; sigrok `:1140-1143`). Code 155 (burden display enabled) or 156 (not enabled), point 3, value 0, is sent in the burden setup (`:7730-7744`, called at `:7634` and `:7708` with `burden_enabled_for_current_ranges` / `burden_enabled_for_power_ranges`); neither app names them. fw 1.02 never sends 101, 106, 121, 126, 131, 136, 137, 141, 142; its enum names 190 `MM_SUB_mS` (`enumerations.txt:64`). The paired codes and the burden and interval units stay open | seen (setup); fw 1.02 |
| VA secondary (§7.1) | The secondary display alternates between volts (sub mode 1 or 2) and amps (16-21) in the VA modes (`EEVBlog-102.c:26859`, `:7784-7870`) | fw 1.02 |
| Blank secondary (§7.1, §14.14) | All zeros: mode, range and value `00` (`bt_write_no_sub`, `EEVBlog-102.c:7898-7907`), which decodes as Low-Z 0 (§13). No capture shows one: both packets of §15.5 carry internal temperature | fw 1.02 |
| Secondary frequency (§7.2, §14.15) | In AC mV a meter sends sub mode 6 with sub range `0x12` (Hz, point 2): 62.97 Hz beside 1.638 mV AC (`121gwcli/README.md:166-179`). fw 1.02 sets Hz with any frequency and k from frequency range 2 (`EEVBlog-102.c:7749-7781`) | seen; fw 1.02 |
| Continuity (§6.2) | Mode 10 carries the resistance, not the beeper state (sigrok `eev121gw.c:872-886`) | code comment |
| Bar 0~150 (§8, §14.16) | Set only when the bar scale is 1000 (`EEVBlog-102.c:7340-7341`), the scale for which EEVblog draws the other tick set | fw 1.02 |
| ↙ (§9, §14.17) | fw 1.02 sets byte 16 bit 5 from `meter_danger_icon` (`EEVBlog-102.c:7386-7387`), which drives the LCD's `S0H_ICON_DANGER` segment (`:4941-4947`), e.g. above 300 counts in Low Z (`:8995`); the manual's LCD has a hazardous-voltage bolt (p.31) [INFERRED match]. Sigrok ("20mA loop current?", `eev121gw.c:213`), qt5 ("Down") and evotronix ("Peak / continuity arrow", `PacketV2.swift:597`) only guess | fw 1.02 |
| Other annunciators (§9, §14.17) | fw 1.02 never sets TEST, byte 17 bits 1-0, or byte 15/16 bit 7; MEM is 1 while logging or in playback (`:7406-7407`); MIN/MAX is 1-4 only, 1 ms PEAK sending 1 or 2 (`:7392-7402`); dBm (byte 16 bit 3) is set in ACV while the secondary display shows dBm (`:7390-7391`). Both packets of §15.5 have byte 17 = `00` | fw 1.02; captures |
| Identity (§10, §14.18) | The year-month is the last calibration date, and the five digits are the user's Meter ID (fw 1.02 `last_cal_year_month`, `curr_meter_serial`, `EEVBlog-102.c:7234-7244`); sigrok: "It certainly is not the current date", "a user adjustable device identification number" (`eev121gw.c:739-757`). Two meters show year `17`, month 8, IDs 42121 and 54321 (§15.5); how months 10-12 are coded stays open | fw 1.02; captures |
| APO (§3, §14.19) | The countdown stops only when APO is off or logging is on, with no Bluetooth check (`EEVBlog-102.c:26606-26613`, the check at `:26609`); a beep resets it (`:9282-9287`); Bluetooth key frames take the same key path (`:16539`). APO is 60 min instead of 30 in MIN/MAX (`:9219-9225`) | fw 1.02 |

No community source answers §14.12 (the V2 temperature bits).

### 15.5 Captured packets

Two complete packets exist in the community sources, both passing the §4
checksum; `F2` is prepended to the 121gwcli line, whose indication carried
bytes 1-18 (§15.4). Community-captured, not ours.

**The packet quoted in sigrok**, `sigrok/eev121gw.c:86` (the driver is
© 2018):

```
F2 17 84 21 21 08 00 00 00 64 01 01 17 12 37 02 40 00 7D
```

| Bytes | Decode (§5-10) |
|---|---|
| 1-4 `17 84 21 21` | year `17`, month 8, ID 42121 |
| 5-8 `08 00 00 00` | mode 8 (Duty), range 0, 0.0 % |
| 9-12 `64 01 01 17` | sub 100 (internal °C), point 1, 0x0117 = 279 → 27.9 |
| 13-14 `12 37` | bar: USE set (off), scale 500; value `37`: bit 5 set (D4), 23 |
| 15-17 `02 40 00` | APO; BT |
| 18 `7D` | checksum, valid |

The comment says the bit-5 case was "seen … in FREQ and OHM", but this
packet's mode is 8, Duty.

**121gwcli's capture**, a gatttool line quoted in a comment,
`121gwcli/parse121gw.pl:159` (2022), `F2` prepended:

```
F2 17 85 43 21 09 86 00 00 64 01 01 02 01 21 06 40 00 8D
```

| Bytes | Decode (§5-10) |
|---|---|
| 1-4 `17 85 43 21` | year `17`, month 8, ID 54321 |
| 5-8 `09 86 00 00` | mode 9 (Resistor), OFL, range 6 (50 MΩ), value 0 |
| 9-12 `64 01 01 02` | sub 100, point 1, 0x0102 = 258 → 25.8 |
| 13-14 `01 21` | bar shown, positive, scale 50; value `21`: bit 5 set (D4), 1 |
| 15-17 `06 40 00` | AUTO, APO; BT |
| 18 `8D` | checksum, valid |

This is the reading of §13 example 3 with a real identity, secondary display
and bar. The same README also gives one reading as decoded fields, without
raw bytes or checksum: mode 4, range 0, value 1638 (1.638 mV AC); sub mode 6,
sub range `0x12`, 6297 (62.97 Hz); bar status `01`, value `00`; icons
`14 40 00`, AC and AUTO, BT — its printed "0x0e2800" is those hex strings
passed through `%02x` as decimals (`121gwcli/README.md:166-179`,
`parse121gw.pl:176-178`, `:184`).
