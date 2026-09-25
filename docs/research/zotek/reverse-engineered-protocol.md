# ZOTEK Bluetooth meters: Reverse-Engineered Protocol Specification

What the Bluetooth LE multimeters made by ZOTEK — sold as ZOYI, ZOTEK, BSIDE
and ANENG — send and accept. One protocol: every notification is XOR-scrambled
with a fixed 20-byte key, starts `5A A5` once descrambled, and carries a type
byte that selects one of four packet layouts, each a dump of the meter's LCD
segments and annunciators. Nothing here is implemented, and no meter has been
on our bench: every fact comes from ZOTEK's apps and manuals. The approach doc
beside it records the sources, the method and the clean-room boundary.

Based on:
- e-Bull V2 1.1.2 (`com.zoyi.bleapp`), a uni-app whose protocol code is the
  JavaScript in `app-service.js`, read and run in node
- e-Bull V1.0.12 (`com.yscoco.multimeter`) and its white-label twin Bluetooth
  DMM 1.0.13 (`com.yscoco.wyboem`), decompiled with jadx 1.5.6; their protocol
  classes are identical
- The ZOTEK manuals for the ZT-300AB, ZT-5B, ZT-5BQ, ZT-6S, ZT-5566 and
  ZT-5566S/SE, read from the rendered pages

Citation keys:
- **V2@N** — byte offset N into
  `references/zotek/e-bull-v2/unzipped/assets/apps/__UNI__59084C7/www/app-service.js`
- **V1** — `references/zotek/e-bull-v1/jadx-out/sources/com/yscoco/multimeter/`;
  **BCU** = `ble/Util/BleComputeUtil.java`, **BMA** =
  `base/BaseMainActivity.java`, **DataParsing** = `ble/DataParsing.java`; **blue/** =
  `…/sources/com/yscoco/blue/`, the BLE library V1 and Bluetooth DMM share.
  Bluetooth DMM (**BD**) keeps its copies under `com/yscoco/wyboem/`, at the
  same lines except HomeActivity (§8.3), BMA (4 lines earlier around the
  cited lines), `MultimeterApp` (+1) and `LargeSeekActivity` (+2)
- **findings/** — our working reports, `references/zotek/analysis/findings/`
- **V1 zN** — index N of the flag array V1 builds from the packet
  (`getAllResult`, BCU:449-457; type 3 reordered by `getRightOrderTable300`,
  BCU:459-487); what each index shows is at BCU:548-813 and BMA:407-619
- **Manuals** — `references/zotek/manuals/`. `300AB p.N` = `ZT-300AB-EN.pdf`
  PDF page N (printed page N−4); `5B`, `5BQ`, `6S p.N/-k-` = leaflet PDF page
  N, printed panel k; `5566 p.N` = `ZT-5566.pdf`, `5566SE p.N` =
  `ZT-5566SE.pdf` (byte-identical to `ZT-5566S.pdf`), PDF pages

Byte and bit numbering: bytes are counted in the descrambled packet with `5A`
as byte 0; bit 0 is the least significant bit.

Confidence levels:
- **[KNOWN]** — stated in a ZOTEK manual, cited by file and page
- **[VENDOR]** — read from a ZOTEK app, with `file:line` or `V2@offset`
- **[INFERRED]** — logical inference from the above, reason given
- **[UNVERIFIED]** — no source confirms it; needs a real meter (all in §10)
- **[HARDWARE]** — seen on a real meter: none yet for this family

---

## 1. Models and packet types

Byte 2 of every packet is a type byte, 1 to 4 [VENDOR]. Nothing in any app
ties a type to a model: no code selects by model name (model names appear
only in identifiers), and no app picks a parser by advertised name
(`findings/protocol-groups.md`).
The only link is each layout's identifier, and the flag set each layout
carries against each manual's LCD legend. So **every model ↔ type row below
is [INFERRED]**.

| Type | V1 enum · V2 parser | Named for | Why it fits |
|---|---|---|---|
| 1 | `QB_5G` · `usartQb5gResolve` (V2@253396) | ZT-5BQ | its peak and inrush bits match the clamp's PEAK HOLD and INRUSH (5BQ p.1/-4-, p.2/-6-), though the 5BQ leaflet documents no REL or duty cycle, which type 1 carries |
| 2 | `S_5G` · `usartS5gResolve` (V2@263571) | ZT-5B | no REL, MAX, MIN or AUTO bit, as the auto-only ZT-5B documents none (the whole 5B leaflet) |
| 3 | `AB_300` · `usart300abResolve` (V2@1829045) | ZT-300AB | its bits are the ZT-300AB legend one for one, TRUE RMS aside (300AB p.7-8; §7.1) |
| 4 | `P_66` · `usart5566Resolve` (V2@854364) | ZT-5566 family | a leading "1" (19999 counts), a colon, a secondary display, MAX/MIN/REL — the ZT-5566 LCD (5566SE p.8) |

The enum is `myenum/DeviceType.java`; V1 maps the byte at BCU:815-826.

| Model | Also sold as [INFERRED] | Counts [KNOWN] | Type [INFERRED] | Bluetooth [KNOWN] |
|---|---|---|---|---|
| ZT-300AB | ZOYI/ZOTEK, BSIDE; ANENG AN9002 | 6000 (300AB p.5) | 3 | hold Hz% 2 s (300AB p.10, p.28) |
| ZT-5B | ZOYI/ZOTEK, BSIDE; ANENG V05B | 6000 (5B p.1/-2-) | 2 | short press of the power button (5B p.1/-3-) |
| ZT-5BQ | ZOYI/ZOTEK, BSIDE; ANENG ST207 | 6000 (5BQ p.1/-3-) | 1 | Power and Hz together (5BQ p.2/-7-) |
| ZT-5566, ZT-5566S, ZT-5566SE | ZOYI/ZOTEK, BSIDE ZT5566; ANENG AN999S (≈ S/SE) | 19999 (5566SE p.6) | 4 | the ZT-5566 manual documents a Bluetooth **speaker** only (5566 p.20); the SE manual has the speaker (5566SE p.22) and an app section naming other models (p.32-35) |
| ZT-6S | ZOYI/ZOTEK | 6000 (6S p.2/-8-) | unknown | "Bluetooth √", nothing more (6S p.2/-8-) |

The "also sold as" column comes from product and reseller pages that match
model numbers, ranges and keys, not from any protocol source
(`findings/discovery.md`).

**The ZT-5566 is the weak row.** Neither ZT-5566 manual says the meter sends
readings: Bluetooth is a speaker, and the "Bluetooth DMM" app pages the SE
manual carries (5566SE p.32-35) name only the ZT-300AB, ZT-5BQ and ZT-5B. Type
4 rests on the V2 parser's name, V1's clock set for it (§8.3) and ZOTEK's
download page listing e-Bull V2 for the ZT-5566 and ZT-5566SE.

---

## 2. Advertising and GATT

| Item | Value | Source | Tag |
|---|---|---|---|
| Name | **"Bluetooth DMM"**: V1/BD accept that exact name or exact **"ZY"** (V1 `ui/main/NewEquipmentActivity.java:161`); V2 accepts any name containing "Bluetooth DMM" (V2@1698973). Devices with no name are dropped (blue/a/c.java:57-58, 228-230) | apps | [VENDOR] |
| Name, manuals | the pairing list entry to tap is "Bluetooth DMM" (300AB p.29; 5BQ p.2/-7-, p.3/-1-; 5566SE p.32-35) | manuals | [KNOWN] |
| Name, AD type | whether the name is in the advertisement or the scan response, and whether any model sends "ZY" | — | [UNVERIFIED] |
| Scan filter | none: no service UUID filter (blue/BleConfig.java:26; V2@1698817), the raw scan record is stored but never parsed (blue/a/c.java:226, 198) | apps | [VENDOR] |
| Service | `0000FFF0-0000-1000-8000-00805F9B34FB` (V1 `MultimeterApp.java:131`; V2 takes the service whose UUID starts `0000FFF0`, V2@259092) | apps | [VENDOR] |
| Notify characteristic | `0000FFF4-…` (V1 `MultimeterApp.java:132`); V2 takes the last characteristic in FFF0 with `notify` (V2@259435) | apps | [VENDOR] |
| Write characteristic | V1/BD write to the **same** `0000FFF4-…` (V1 `MultimeterApp.java:133`); V2 takes the last characteristic in FFF0 with `write` (V2@259501). V1 also declares FFF3 notify / FFF2 write constants that nothing uses (V1 `constant/Constans.java:7-8`) | apps | [VENDOR]; that FFF4 is the meter's write characteristic [UNVERIFIED] |
| Subscribe | V1/BD: CCCD `0x2902` on FFF4 written with ENABLE_NOTIFICATION (blue/a.java:401-406). V2: on the characteristic it picked by property (V2@259726), with ENABLE_INDICATION instead if that characteristic can indicate (DCloud's `BluetoothBaseAdapter.notifyBLECharacteristicValueChange` in `classes2.dex`) | apps | [VENDOR] |
| Write type | V1/BD: write without response (blue/a.java:436). V2 asks for it on Android and a plain write elsewhere (V2@260897, @260953), but its Android runtime ignores `writeType` and writes with the characteristic's default type (`BluetoothBaseAdapter.writeBLECharacteristicValue`) | apps | [VENDOR]; which types FFF4 takes [UNVERIFIED] |
| MTU | never requested by any app (no `requestMtu`, no `setBLEMTU`) | apps | [VENDOR] |

V2's home page routes names containing "ZT-DY", "ZOYI" or "POWER" to its
power-supply protocol (V2@882866), a different device (the ZT-DY bench supply)
with `A5 5A` framing and a CRC16 (V2@19648, @19961), out of scope here
[VENDOR].

## 3. Bring-up

The meter streams unprompted [INFERRED: both app lines connect, discover,
subscribe and then only listen, so nothing is written before data arrives]
(V1 blue/a.java:51-58,99,151-173 → `ui/home/HomeActivity.java:183-185`; V2
`mult_main` onLoad, V2@906547). No app polls [VENDOR].

The one automatic write is V1/BD's clock set to type-4 meters, sent after the
first type-4 packet from a device and then every half hour (§8.3). V2 never
sends it.

The display updates 3 times a second on the ZT-300AB (300AB p.22) and the
ZT-5566 (5566SE p.26) [KNOWN]. Whether each update is one notification is
[UNVERIFIED].

## 4. Descramble

Every notification is XORed with a fixed 20-byte key, starting over at key
byte 0 for each notification [VENDOR] (V1 `util/EncryptionUtil.java:7,15-21`;
V2@23333 key, V2@19579 `usartEncCode`, applied per notification at V2@260478):

```
plain[i] = raw[i] XOR key[i % 20]
key = 41 21 73 55 A2 C1 32 71 66 AA 3B D0 E2 A8 33 14 20 1A AA BB
```

Both apps hold the key as signed bytes
(`{65,33,115,85,-94,-63,50,113,102,-86,59,-48,-30,-88,51,20,32,26,-86,-69}`).
Commands to the meter are scrambled with the same key from byte 0 (§8). XOR is
its own inverse, so one function does both [VENDOR].

On air a packet therefore starts `1B 84` and the type byte arrives XORed with
`73`: type 1 as `72`, 2 as `71`, 3 as `70`, 4 as `77` [INFERRED from the key].

## 5. Framing

After descrambling [VENDOR]:

| Rule | Source |
|---|---|
| Bytes 0-1 are `5A A5`; anything else is ignored | DataParsing:32; V2@250980 |
| Byte 2 is the type (§1) | DataParsing:34-47; V2@250980 |
| Bytes read: types 1 and 2 → 3-9; type 3 → 3-10; type 4 → 3-18. So a packet is at least 10, 10, 11 and 19 bytes | DataParsing:25,38-43,49-57; V2@250980 (`slice(3,10)`, `(3,11)`, `(3,19)`) |
| No length byte, no checksum, no trailer is read | DataParsing:24-58; V2@250980 |
| One notification is one packet; neither app reassembles | DataParsing:16-21; V2@260425 |
| A notification of 10 bytes or more whose byte 0 is `AB` is logged as "接收回应数据数据" (received reply data) and dropped by V1; V2 drops it at the header check | DataParsing:25-30 |
| Only types 1-4 are defined. V1 parses any other type byte with the type-1 layout, labelled `S_5G` (BCU:815-826); V2 ignores it | DataParsing:45-47; V2@250980 |

The apps give only minimum lengths. Whether the meter sends anything past the
last byte read is [UNVERIFIED]. No app requests a larger MTU [VENDOR]; every
layout fits in a 20-byte ATT payload (the default ATT_MTU of 23 minus the
3-byte header), which the 20-byte key covers exactly [INFERRED]. The meter
could still negotiate a larger MTU, so the notification length stays
[UNVERIFIED].

---

## 6. Digits

### 6.1 Seven-segment glyphs

Each glyph is one byte in segment order `a f e DP b g c d`, bit 7 to bit 0:
a = `0x80`, f = `0x40`, e = `0x20`, **DP = `0x10`**, b = `0x08`, g = `0x04`,
c = `0x02`, d = `0x01`. The segment names are [INFERRED] from the digit codes;
both apps compare a glyph with bit 4 cleared (`& 0xEF`) [VENDOR].

| Code | Glyph | Source | Tag |
|---|---|---|---|
| `EB` `0A` `AD` `8F` `4E` `C7` `E7` `8A` `EF` `CF` | 0 1 2 3 4 5 6 7 8 9 | BCU:15-108, 354-447; V2@255476 (table indexed by digit value) | [VENDOR] |
| `EE` | A | BCU:55-108 (`getEveryStringNum`) | [VENDOR] |
| `E5` | E | same | [VENDOR] |
| `E4` | F | same | [VENDOR] |
| `61` | L | same | [VENDOR] |
| `27` | o | same (V1 writes "O") | [VENDOR] |
| `23` | u | same (V1 writes "U") | [VENDOR] |
| `65` | t | same (V1 writes "T") | [VENDOR] |
| `04` | - | same | [VENDOR] |
| `67` `E1` `2F` | b C d | V2 table only, unused by the code (V2@255476) | [INFERRED] from segment geometry |
| `00` | blank, counted as 0 | V2@255661 | [VENDOR] |

### 6.2 Types 1-3: four digits split across bytes 3-7

Each digit takes its high nibble (segments a, f, e and a DP-or-sign bit) from
one byte and its low nibble (b, g, c, d) from the next [VENDOR] (BCU:124-135;
V2@253623, @263798, @1829670):

| Digit | High nibble | Low nibble | Bit 4 of the high-nibble byte |
|---|---|---|---|
| 1 (leftmost) | byte 3 bits 7-4 | byte 4 bits 3-0 | byte 3 bit 4: **minus sign** |
| 2 | byte 4 bits 7-4 | byte 5 bits 3-0 | byte 4 bit 4: DP before digit 2 (`d.ddd`) |
| 3 | byte 5 bits 7-4 | byte 6 bits 3-0 | byte 5 bit 4: DP before digit 3 (`dd.dd`) |
| 4 (rightmost) | byte 6 bits 7-4 | byte 7 bits 3-0 | byte 6 bit 4: DP before digit 4 (`ddd.d`) |

Byte 3's low nibble and byte 7's high nibble are flags (§7). One DP bit is
expected per packet; with two set, V1 honours the leftmost and V2 the
rightmost, so what the meter means by it is [UNVERIFIED].

### 6.3 Type 4: one byte per digit

Type 4 carries whole glyph bytes, least significant digit first [VENDOR]:

| Display | Bytes | DP (bit 4) | Sign | Source |
|---|---|---|---|---|
| Main | 9 (rightmost) to 12 | on byte 9, 10, 11, 12 → 1, 2, 3, 4 decimals | byte 13 bit 7 | BCU:247-261, 306-347; V2@854969, @855004 |
| Main, leading digit | byte 13 bits 3 **and** 2 both set = a leading "1", adding 10000 | — | — | BCU:357; V2@856978 |
| Main, colon | byte 13 bit 4, between the second and third digits from the right | — | — | BCU:261,320-321 |
| Secondary | 5 (rightmost) to 8 | on byte 5, 6, 7 → 1, 2, 3 decimals | byte 8 bit 4 | V2@854785, @854849, @854913 |

- The leading "1" uses bits b and g of byte 13, not a digit code, and adds
  10000 [VENDOR]: a half digit, which with four full digits makes the
  ZT-5566's 19999 counts (5566SE p.6) [KNOWN]. That it is drawn as "1" is
  [INFERRED] from both apps and the manual's `1.8.8:8.8` readout (5566SE
  p.8).
- The colon is [INFERRED]: only V1 renders byte 13 bit 4, as `:` between the
  digits in bytes 11 and 10 (V2 ignores the bit), and the ZT-5566 manual draws
  its main readout `1.8.8:8.8` (5566SE p.8). Its use (a clock in standby,
  5566SE p.23) is [UNVERIFIED].
- The secondary display is decoded only by V2, which then never shows it; V1
  does not read bytes 5-8 (DataParsing:49-57). The ZT-5566 has one ("Vice
  Display", legend #12, 5566SE p.9): in V, "the main display shows the
  voltage and the secondary display shows the frequency" (p.11 ②), and in
  frequency mode it shows the duty cycle (p.21) [KNOWN].

### 6.4 Special displays

The meters show words as glyphs, not codes, and the two apps recognise them
by different rules [VENDOR]:

- **V2** (`c()`, V2@1831448) scans from the rightmost digit, blanks counting
  as 0, to the first other glyph: `o`, `t`, `u`, `A` leftwards from there is
  AUTO; `F` then `E` is EF; a dash gives dashes from that position leftwards
  (four when it is the rightmost digit, V2@253820); anything else is "OL".
- **V1** (BCU:143-166, rendered at `ui/main/LargeSeekActivity.java:129-158`)
  checks fixed positions, digits numbered from the left as in §6.2: OL when
  digit 3 is `L`; AUTO when digit 1 is `A`; EF when digit 2 is `E` (F not
  checked); `-` when digit 1 is a dash with no sign bit, then `--`, `---`,
  `----` as digits 2, 3 and 4 are dashes too (BCU:155-166; jadx inverts the
  digit-2 test there, the smali does not).

The rules disagree on §9's EF example (`E`, `F` in digits 3-4): V2 reads it
as EF, V1 shows the glyphs but not as EF, since it checks only digit 2.
Which positions the meters use is [UNVERIFIED].

| Display | On the LCD | When | Tag |
|---|---|---|---|
| AUTO | `Auto` across the digits (5B p.1/-1-; 5BQ p.1/-1-, -4-) | auto mode before a reading: the meter shows one "only when the voltage is higher than 0.8V" (5B p.1/-3-; 5BQ p.1/-4-; 6S p.1/-3-); the ZT-300AB's AUTO dial position has the same rule (300AB p.12, p.14) but its own AUTO annunciator (legend #14) | look [KNOWN]; the "when" [INFERRED] |
| EF | `EF` (the apps' patterns) | NCV, as both apps treat it (V2@253820; V1 BMA:504, 533) | [INFERRED] |
| dashes | one to four `-` (the apps' patterns) | NCV, as for EF; what the count means is not stated | meaning [UNVERIFIED] |
| OL | `0L` | overload, open resistance, reversed diode (300AB p.6, p.16, p.17; 5566SE p.7, drawn `0L`) | [KNOWN] |

In type 4, V2 shows "OL" for any non-digit glyph and matches no word
(V2@856869); V1 renders the same glyph table (BCU:354-447).

---

## 7. Flags

Every bit not listed as a digit or a flag is read by neither app. "V2 name" is
the field name in V2's parser; "V1" is the flag index (see Citation keys) or
"—" where V1 does not read the bit. A meaning taken only from a V2 field name
that drives no UI is [INFERRED].

### 7.1 Type 3 (ZT-300AB) — priority

V2 parser V2@1829045; V1 DataParsing:38-43. Cross-checked against the
ZT-300AB legend (300AB p.7-8), whose items the bits match one for one; the
legend's TRUE RMS has no bit.

| Byte | Bit | Meaning | V2 name @offset | V1 | Tag |
|---|---|---|---|---|---|
| 3 | 7-5 | digit 1 segments a, f, e | @1829670 | BCU:124 | [VENDOR] |
| 3 | 4 | minus | `pol` @1829653 | BCU:128 | [VENDOR] |
| 3 | 3 | continuity | `beep` @1829190 | z0, BCU:779-782 | [VENDOR] |
| 3 | 2 | Bluetooth icon | `ble` @1829207 | — | [VENDOR]; meaning [INFERRED], legend #21 |
| 3 | 1 | Δ relative | `trigon` @1829223 | z1, BMA:433-434 | [VENDOR]; legend #20 [KNOWN] |
| 3 | 0 | low battery | `bat` @1829242 | z3 (no display) | [VENDOR]; meaning [INFERRED], legend #16 |
| 4-6 | all | digits (§6.2) | | | [VENDOR] |
| 7 | 7 | diode | `diode` @1829258 | z14, BCU:779-786 | [VENDOR] |
| 7 | 6 | °C | `temp_c` @1829278 | z20, BCU:575 | [VENDOR] |
| 7 | 5 | °F | `temp_f` @1829298 | z21, BCU:573 | [VENDOR] |
| 7 | 4 | HOLD | `hold` @1829318 | z2, read but not shown for this type (BMA:473-478) | [VENDOR] |
| 7 | 3-0 | digit 4 segments b, g, c, d | | | [VENDOR] |
| 8 | 7 | n, capacitance | `n1` @1829403 | z11, BCU:553 | [VENDOR] |
| 8 | 6 | m, capacitance | `m1` @1829420 | z17, BCU:559 | [VENDOR] |
| 8 | 5 | µ, capacitance | `u1` @1829436 | z12, BCU:555 | [VENDOR] |
| 8 | 4 | F | `f` @1829452 | z15, BCU:568 | [VENDOR] |
| 8 | 3 | AC | `ac` @1829336 | z8, BCU:777 | [VENDOR] |
| 8 | 2 | % (duty) | `percent` @1829351 | z23, BMA:435-436 | [VENDOR]; duty from legend #6 [KNOWN] |
| 8 | 1 | MIN | `min` @1829371 | — | [VENDOR]; legend #3 |
| 8 | 0 | MAX | `max` @1829387 | — | [VENDOR]; legend #2 |
| 9 | 7 | A | `A` @1829524 | z13, BCU:566 | [VENDOR] |
| 9 | 6 | DC | `dc` @1829540 | z9, BCU:778 | [VENDOR] |
| 9 | 5 | m, millivolts | `m2` @1829556 | z17, BCU:479 | [VENDOR] |
| 9 | 4 | V | `V` @1829572 | z10, BCU:564 | [VENDOR] |
| 9 | 3 | M (Ω, Hz) | `M` @1829467 | z16, BCU:557 | [VENDOR] |
| 9 | 2 | k (Ω, Hz) | `k` @1829481 | z18, BCU:561 | [VENDOR] |
| 9 | 1 | Ω | `R` @1829495 | z19, BCU:570 | [VENDOR] |
| 9 | 0 | Hz | `hz` @1829509 | z22, BCU:572 | [VENDOR] |
| 10 | 7-4 | not read; TRUE RMS may sit here | | | [UNVERIFIED] |
| 10 | 3 | m, milliamps | `m3` @1829587 | z17, BCU:479 | [VENDOR] |
| 10 | 2 | µ, microamps | `u2` @1829602 | z12, BCU:474 | [VENDOR] |
| 10 | 1 | MANUAL range | `manual` @1829617 | — | [VENDOR]; meaning [INFERRED], legend #13 |
| 10 | 0 | AUTO range | `auto` @1829636 | BCU:491, BMA:430 | [VENDOR]; legend #14 |

### 7.2 Type 1 (ZT-5BQ)

V2 parser V2@253396; V1 DataParsing:45-47. Bytes 4-6 and byte 7 bits 3-0 are
digits (§6.2).

| Byte | Bit | Meaning | V2 name @offset | V1 | Tag |
|---|---|---|---|---|---|
| 3 | 7-5, 4 | digit 1 a, f, e; minus | @253623; `pol` @253606 | BCU:124,128 | [VENDOR] |
| 3 | 3 | continuity | `beep` @253540 | z0 | [VENDOR] |
| 3 | 2 | Bluetooth icon | `ble` @253557 | z1, unused | [VENDOR]; meaning [INFERRED] |
| 3 | 1 | HOLD | `hold` @253573 | z2, BMA:516 | [VENDOR] |
| 3 | 0 | low battery | `bat` @253590 | z3 | [VENDOR]; meaning [INFERRED] |
| 7 | 7 | AC | `ac` @254136 | z8 | [VENDOR] |
| 7 | 6 | DC | `dc` @254153 | z9 | [VENDOR] |
| 7 | 5 | V | `V` @254169 | z10 | [VENDOR] |
| 7 | 4 | n | `n` @254184 | z11 | [VENDOR] |
| 8 | 7 | M | `M` @254259 | z16 | [VENDOR] |
| 8 | 6 | m | `m` @254275 | z17 | [VENDOR] |
| 8 | 5 | k | `k` @254290 | z18 | [VENDOR] |
| 8 | 4 | Ω | `R` @254305 | z19 | [VENDOR] |
| 8 | 3 | µ | `u` @254199 | z12 | [VENDOR] |
| 8 | 2 | A | `A` @254213 | z13 | [VENDOR] |
| 8 | 1 | diode | `diode` @254227 | z14 | [VENDOR] |
| 8 | 0 | F | `F` @254245 | z15 | [VENDOR] |
| 9 | 7 | unknown ("power") | `power` @254389 | z4, unused | [VENDOR]; meaning [UNVERIFIED] |
| 9 | 6 | PEAK | `peak` @254409 | z5; shown from `zArr2[0]`, BMA:429 | [VENDOR] |
| 9 | 5 | % | `percent` @254427 | z6, unused | [VENDOR] |
| 9 | 4 | INRUSH | `inrush` @254448 | z7; shown from `zArr2[1]`, BMA:428 | [VENDOR] |
| 9 | 3 | °C | `temp_c` @254320 | z20 | [VENDOR] |
| 9 | 2 | °F | `temp_f` @254339 | z21 | [VENDOR] |
| 9 | 1 | Hz | `hz` @254358 | z22 | [VENDOR] |
| 9 | 0 | REL | `rel` @254373 | z23, BMA:431 | [VENDOR] |

No AUTO, MAX or MIN bit.

### 7.3 Type 2 (ZT-5B)

V2 parser V2@263571; V1 DataParsing:34-36. Bytes 4-6 and byte 7 bits 3-0 are
digits (§6.2); byte 7 bits 5-4 are not read by V2 (V1: z6, z7, unused).

| Byte | Bit | Meaning | V2 name @offset | V1 | Tag |
|---|---|---|---|---|---|
| 3 | 7-5, 4 | digit 1 a, f, e; minus | @263798; `pol` @263781 | BCU:124,128 | [VENDOR] |
| 3 | 3 | continuity | `beep` @263710 | z0 | [VENDOR] |
| 3 | 2 | over-voltage | `over_vol` @263727 | z1, unused | [VENDOR]; meaning [INFERRED] |
| 3 | 1 | HOLD | `hold` @263748 | z2 | [VENDOR] |
| 3 | 0 | low battery | `bat` @263765 | z3 | [VENDOR]; meaning [INFERRED] |
| 7 | 7 | Bluetooth icon | `ble` @264302 | z4, unused | [VENDOR]; meaning [INFERRED] |
| 7 | 6 | unknown ("power") | `power` @264320 | z5, unused | [VENDOR]; meaning [UNVERIFIED] |
| 8 | 7 | µ | `u` @264397 | z12 | [VENDOR] |
| 8 | 6 | A | `A` @264413 | z13 | [VENDOR] |
| 8 | 5 | diode | `diode` @264428 | z14 | [VENDOR] |
| 8 | 4 | F | `F` @264447 | z15 | [VENDOR] |
| 8 | 3 | AC | `ac` @264339 | z8 | [VENDOR] |
| 8 | 2 | DC | `dc` @264354 | z9 | [VENDOR] |
| 8 | 1 | V | `V` @264369 | z10 | [VENDOR] |
| 8 | 0 | n | `n` @264383 | z11 | [VENDOR] |
| 9 | 7 | °C | `temp_c` @264518 | z20 | [VENDOR] |
| 9 | 6 | °F | `temp_f` @264539 | z21 | [VENDOR] |
| 9 | 5 | Hz | `hz` @264559 | z22 | [VENDOR] |
| 9 | 4 | % | `percent` @264575 | z23, unused | [VENDOR] |
| 9 | 3 | M | `M` @264462 | z16 | [VENDOR] |
| 9 | 2 | m | `m` @264476 | z17 | [VENDOR] |
| 9 | 1 | k | `k` @264490 | z18 | [VENDOR] |
| 9 | 0 | Ω | `R` @264504 | z19 | [VENDOR] |

No AUTO, REL, MAX or MIN bit.

### 7.4 Type 4 (ZT-5566 family)

V2 parser V2@854364; V1 DataParsing:49-57, which reads bytes 3, 4, 13, 16, 17 and 18,
plus the digits in 9-13.

| Byte | Bit | Meaning | V2 name @offset | V1 | Tag |
|---|---|---|---|---|---|
| 3 | 7 | unknown ("vfc") | `vfc` @854613 | z4, unused | [VENDOR]; meaning [UNVERIFIED] |
| 3 | 6 | diode | `diode` @854594 | z5, BCU:799-806 | [VENDOR] |
| 3 | 5 | continuity | `beep` @854576 | z6, BCU:799-808 | [VENDOR] |
| 3 | 4 | REL | `rel` @854559 | z7, BMA:444 | [VENDOR] |
| 3 | 3 | unknown ("l1_power") | `l1_power` @854538 | z0, unused | [VENDOR]; meaning [UNVERIFIED] |
| 3 | 2 | AUTO range | `auto` @854521 | z1, BMA:443 | [VENDOR] |
| 3 | 1 | MANU range | `manu` @854504 | z2, unused | [VENDOR]; meaning [INFERRED], legend #4 (5566SE p.8) |
| 3 | 0 | not read by V2 | — | z3, unused | — |
| 4 | 7 | % on the secondary display | `percent_small` @854757 | z12, unused | [VENDOR]; meaning [INFERRED] |
| 4 | 6 | Hz on the secondary display | `hz1_small` @854734 | z13, unused | [VENDOR]; meaning [INFERRED] |
| 4 | 5 | k on the secondary display | `k1_small` @854712 | z14, unused | [VENDOR]; meaning [INFERRED] |
| 4 | 4 | V | `v` @854697 | z15, BCU:594 | [VENDOR] |
| 4 | 3 | HOLD | `hold` @854680 | z8, BMA:553 | [VENDOR] |
| 4 | 2 | PEAK | `peak` @854663 | z9, BMA:442 | [VENDOR] |
| 4 | 1 | MAX | `max` @854647 | z10, BMA:446 | [VENDOR] |
| 4 | 0 | MIN | `min` @854631 | z11, BMA:447 | [VENDOR] |
| 5-8 | all | secondary digits; byte 8 bit 4 = secondary minus (§6.3) | @854785; `pol2_small` @854810 | — | [VENDOR]; meaning [INFERRED] |
| 9-12 | all | main digits (§6.3) | @854969 | BCU:247-261 | [VENDOR] |
| 13 | 7 | main minus | `pol1_big` @855004 | BCU:252 | [VENDOR] |
| 13 | 6 | AC | `ac` @855482 | z21, BCU:797 | [VENDOR] |
| 13 | 4 | colon | — | BCU:261,320-321 | [INFERRED] (§6.3) |
| 13 | 3, 2 | leading "1", both set | @856978 | BCU:357 | [VENDOR] |
| 13 | 1 | DC | `dc` @855466 | z18, BCU:798 | [VENDOR] |
| 13 | 5, 0 | not read by V2 | | z22, z19, unused | — |
| 14, 15 | all | not read; the bar graph may sit here | | | [UNVERIFIED] |
| 16 | 7 | Hz | `hz` @855499 | z24, BCU:602 | [VENDOR] |
| 16 | 6 | Ω | `l4` @855517 | z25, BCU:600 | [VENDOR] |
| 16 | 5 | k | `k2` @855534 | z26, BCU:591 | [VENDOR] |
| 16 | 4 | M | `m1` @855551 | z27, BCU:587 | [VENDOR] |
| 17 | 3 | n | `n` @855568 | z28, BCU:585 | [VENDOR] |
| 17 | 2 | m | `m2` @855583 | z29, BCU:589 | [VENDOR] |
| 17 | 1 | µ | `u` @855599 | z30, unused | [VENDOR] |
| 17 | 0 | F | `f` @855614 | z31, BCU:598 | [VENDOR] |
| 18 | 4 | A | `a` @855629 | z35, BCU:596 | [VENDOR] |

The ZT-5566 LCD has an analog bar graph (5566SE p.8-9), a T-RMS icon
(legend #15) and an auto-standby icon (legend #2) that no bit carries. It has no peak or temperature-measurement
icon; °C/°F appear only on the knob LCD for ambient temperature (5566SE
p.10) [KNOWN].

### 7.5 Units and prefixes

How V2 turns the bits into a unit string [VENDOR]; which prefixes a meter
actually sets per unit is [UNVERIFIED].

| Type | Prefix bits | Units they attach to | Source |
|---|---|---|---|
| 1, 2 | one shared set: n, µ, m, k, M | V (n, µ, m); A (n, µ, m); F (n, µ, m); Ω and Hz (k, M) | V2@254851, @264979 |
| 3 | three groups: capacitance n/m/µ (byte 8 bits 7-5); millivolts m (byte 9 bit 5); current m/µ (byte 10 bits 3-2); k/M (byte 9 bits 2-3) for Ω and Hz | as grouped | V2@1830571 |
| 4 | n, µ, m (byte 17 bits 3, 1, 2) for V, A, F; k, M (byte 16 bits 5, 4) for Ω, Hz | as grouped | V2@855988 |

The type-3 groups are the ZT-300AB legend's: "M k Ω", "n µ/m F", "mV",
"µ/m A" (300AB p.7-8) [KNOWN]. V1 merges them into one n/µ/m/k/M set
(BCU:459-487, 548-607).

---

## 8. Commands

### 8.1 Frame

Ten bytes, then scrambled with the §4 key from byte 0 [VENDOR] (V1
`ble/IssuedUtil.java:10-25`; V2@250571):

```
AB CD <cmd> <p0> <p1> <p2> <p3> <p4> <sumHi> <sumLo>
```

`sum` is the 16-bit sum of bytes 0-7, big-endian. V2 writes it in chunks of
up to 20 bytes (`sendData2`, V2@261945), so as one write. Neither app waits
for a reply; what the meter sends back is §5's `AB`-led frame, if anything
[UNVERIFIED].

### 8.2 Key press, cmd `03`

`AB CD 03 <key> 00 00 00 00 <sum>`, sent when the user taps a remote button
[VENDOR] (V1 `ble/IssuedData.java:10-15`,
`ui/main/MainActivity.java:199-268, 309-313`; V2 `toClickBt` V2@900222, labels
from the English tips V2@236005-236399).
The sum is `0x017B + key`; on air the frame reads
`EA EC 70 <key XOR 55> A2 C1 32 71 <sumHi XOR 66> <sumLo XOR AA>`.

| Key | Button (V2 English tip) | Sent when | Tag |
|---|---|---|---|
| `B8` | AUTO, "Click to switch to auto test functions" | always | [VENDOR] |
| `B6` / `B7` | °C/°F, "Test Celsius,Fahrenheit" | `B7` while the display shows °C, else `B6` | [VENDOR] |
| `B0` | "Test Capacitance" | always | [VENDOR] |
| `B1` | "Test Diode,Buzzer,Continuity" | always | [VENDOR] |
| `B2` | "Test NCV" | always | [VENDOR] |
| `B3` | "Test Frequency" | always | [VENDOR] |
| `B4` | HOLD, "Maintain the displayed data" | always | [VENDOR] |
| `B5` | ZERO, "Clear Key" | only in capacitance | [VENDOR] |
| `C4` | "Test the voltage of v" | always | [VENDOR] |
| `C6` | "Test the voltage of mv" (V1's icon: mV~/Hz) | always | [VENDOR] |
| `BE` | "Test Resistance" | always | [VENDOR] |
| `D1` | MAX/MIN, "Maximum/minimum value of the data" | always | [VENDOR] |
| `C8`-`CB` | "Test Current" | V1 by the current mode shown on a type-4 meter: `CB` AC A, `C8` DC A, `C9` AC mA, `CA` DC mA; `C9` on types 1-3. V2: `CB` in AC, `CA` in DC, else `C9`; its dead branch for A (it tests a field never set) would send `C9` in AC and `C8` in DC | codes [VENDOR]; the per-code meanings [UNVERIFIED]: V1 and V2's branch swap the AC codes |

Both apps offer every button to every type (V1
`jadx-out/resources/res/layout/activity_main.xml:219-292`; V2's static
`menuArr`, V2@891528); V1 greys out capacitance, NCV, Hz and HOLD for type 3
(BMA:473-478).
The manuals show the same remote-button screen (300AB p.29; 5BQ p.3/-2-)
[KNOWN] and never say the meter acts on it. The set fits the auto-ranging
pocket meters more than the ZT-300AB's rotary dial [INFERRED]: the ZT-5B's
H/ZERO "can clean the reading" in capacitance, as `B5` is sent only there,
and its SEL/NCV cycles the modes (5B p.1/-3-). Which keys each model honours
is [UNVERIFIED].

### 8.3 Clock set, cmd `04`

`AB CD 04 <hour> <minute> <second> 00 00 <sum>`, binary, 24-hour, the phone's
time [VENDOR] (V1 `ble/IssuedData.java:17-20`). **V1 and Bluetooth DMM only**:
they send it to a type-4 meter after its first packet on a connection, and
then every half hour, on the hour and the half hour (V1
`ui/home/HomeActivity.java:250-253, 130-141`, `receiver/TimeChangeReceiver.java:25-35`;
BD `ui/home/HomeActivity.java:218-220, 102-104`). V2 never builds it (`getSendByte` is called only from
`toClickBt`).

The ZT-5566 has a clock and alarm, set by hand with MODE and △/▽ (5566SE
p.21-22) [KNOWN]; neither manual says an app sets it. Whether the meter needs
or acts on this command is [UNVERIFIED].

---

## 9. Worked examples

Plain bytes are after descrambling. The four packets were decoded by V2's own
modules run in node (decode B). Decode A built its own type-1 packet,
differing only in byte 3 bit 2, and got the same reading from V1's methods
ported to Python.

**Type 3**, −12.34 V DC, AUTO:

```
raw    1B 84 70 41 08 5C 7D 7F 66 FA 3A
plain  5A A5 03 14 AA 9D 4F 0E 00 50 01
```

Digits: `0A`+sign (byte 3 high `1`, byte 4 low `A`) = 1 with minus; `AD` = 2;
`9F` = 3 with DP (byte 5 bit 4, two decimals); `4E` = 4. Byte 3 bit 2
Bluetooth; byte 9 = `50`: DC, V; byte 10 = `01`: AUTO.

**Type 3 special displays**, bytes 3-7 (node-checked against V2; V1 reads
the EF one differently, §6.4):

| Display | Bytes 3-7 |
|---|---|
| AUTO | `E0 2E 63 25 07` |
| EF | `00 00 E0 E5 04` |
| OL | `00 E0 6B 01 00` |
| `----` | `00 04 04 04 04` |

**Type 1**, 230.5 V AC, HOLD:

```
raw    1B 84 72 F3 2F 2E E9 D6 66 AA
plain  5A A5 01 A6 8D EF DB A7 00 00
```

Byte 3 = `A6`: HOLD (bit 1), Bluetooth (bit 2); byte 6 bit 4: one decimal;
byte 7 = `A7`: AC (bit 7), V (bit 5).

**Type 2**, 4.700 kΩ:

```
raw    1B 84 71 15 3C 2B D9 FA 66 A9
plain  5A A5 02 40 9E EA EB 8B 00 03
```

Byte 4 bit 4: three decimals; byte 9 = `03`: k, Ω. (Byte 7 bit 7, `ble`, is
also set.)

**Type 4**, main 1.2345 V DC, AUTO; secondary 50.00 Hz:

```
raw    1B 84 77 51 F2 2A C9 9A A1 6D 75 5F 5F A6 33 14 20 1A AA
plain  5A A5 04 04 50 EB FB EB C7 C7 4E 8F BD 0E 00 00 00 00 00
```

Byte 3 = `04`: AUTO. Byte 4 = `50`: secondary Hz (bit 6), V (bit 4). Secondary
bytes 5-8 = `EB FB EB C7`: 0, 0+DP (two decimals), 0, 5 → 50.00. Main bytes
9-12 = `C7 4E 8F BD`: 5, 4, 3, 2+DP (four decimals); byte 13 = `0E`: leading
"1" (bits 3, 2) and DC (bit 1) → 1.2345.

**Commands** (both decodes build the AUTO frame; the clock set is decode A's,
and both checksums were recomputed for this document):

| Command | Plain | Raw |
|---|---|---|
| AUTO key | `AB CD 03 B8 00 00 00 00 02 33` | `EA EC 70 ED A2 C1 32 71 64 99` |
| Clock set 13:45:30 | `AB CD 04 0D 2D 1E 00 00 01 D4` | `EA EC 77 58 8F DF 32 71 67 7E` |

---

## Implementation Notes

What the wire requires of any decoder:

- Each notification carries one whole packet, and descrambling restarts at
  key byte 0 in every notification; both apps rely on it.
- There is no checksum. Only `5A A5`, a type byte of 1-4 and the length for
  the type (§5; of the apps, only V1 checks it) mark a packet; digit glyphs
  outside §6.1 are the only other sign of corruption.
- The reading is an LCD image: digits are glyphs, units are annunciator bits,
  and words (AUTO, EF, OL, dashes) are glyph patterns, not codes.
- The same bit means different things in different types (byte 3 bit 2 is
  Bluetooth in types 1 and 3, over-voltage in type 2, AUTO in type 4), so the
  type byte is read before any flag.
- A blank digit is `00`, not a separate code.
- Commands carry their own `AB CD` header and a checksum; received packets
  carry neither.

## 10. Open questions — [UNVERIFIED]

1. **Model ↔ type byte.** Every row of §1: ZT-5BQ → 1, ZT-5B → 2, ZT-300AB →
   3, ZT-5566 family → 4, and each ANENG and BSIDE rebrand. The ZT-6S has no
   evidence at all.
2. **Does the ZT-5566 stream readings?** Its manuals document a Bluetooth
   speaker (5566 p.20, 5566SE p.22); the SE manual's app section names only
   other models (5566SE p.32-35).
3. **Advertised name** per model: "Bluetooth DMM", "ZY" or other, and whether
   it is in the advertisement or the scan response.
4. **Notification length** per type, and anything past the last byte the
   apps read (§5).
5. **Write characteristic.** Whether FFF4 accepts writes, or V2's by-property
   pick lands elsewhere, and whether it takes write without response, write
   with response or both (§2).
6. **Replies.** Whether the meter answers a command with an `AB`-led frame,
   and its format (§5, §8.1).
7. **Keys per model.** Which of §8.2's codes each type honours, and what
   each of `C8`-`CB` selects: V1 and V2 disagree on the AC codes.
8. **Clock set.** Whether a type-4 meter needs or acts on cmd `04` (§8.3).
9. **Type 3 byte 10 bits 7-4.** Unread by both apps; TRUE RMS is the one
   ZT-300AB legend item with no bit.
10. **Type 4 bytes 14-15** (bar graph?), byte 3 bit 0, byte 13 bits 5 and 0, byte 16 bits
    3-0, byte 17 bits 7-4, byte 18 bits other than 4.
11. **Unnamed bits:** `power` (types 1, 2), `vfc`, `l1_power` (type 4; the
    auto-standby icon, §7.4?).
12. **Special displays:** which digit positions each word uses (the apps'
    rules differ, §6.4); what the number of dashes
    means in NCV; whether type 4 shows words at all.
13. **Two DP bits** in one packet (§6.2), and the type-4 colon's use (§6.3).
14. **Update rate on the air** against the LCD's 3 per second (§3).
