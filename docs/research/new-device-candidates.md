# New Device Candidates

Research into multimeters worth supporting (April 2026; UNI-T's own catalogue
swept model by model 2026-09-21). Covers USB HID, Bluetooth LE, USB-serial, and
IR-optical connections.

## Landscape Overview

Modern multimeters with PC connectivity use one of these transports:

| Transport | Examples | Sigrok coverage |
|-----------|----------|-----------------|
| **USB HID** (current dmm-tools) | UNI-T (CP2110/CH9329/CH9325), Brymen (Cypress), Victor | Good on all platforms |
| **Bluetooth LE** (current dmm-tools) | 121GW, OWON B35T/B41T+, UNI-T UT-D07B, Aneng/BSIDE/ZOYI | **Linux only, experimental, flaky** |
| **USB serial (CDC)** | OWON XDM (CH340), Fluke IR (FTDI), APPA (CP2102) | Good on all platforms |
| **USB TMC/SCPI** | Rigol, Siglent bench instruments | Good, well-served by pyvisa/lxi-tools |

**Key gap:** sigrok has **no BLE support on Windows or macOS** — only a
Linux-only BlueZ backend that's described as "slow and occasionally
flaky." No cross-platform desktop tool provides reliable BLE multimeter
connectivity.

---

## USB HID Candidates

### Brymen BM52x / BM82x / BM86x — RECOMMENDED

**Strongest USB HID candidate. Clear software gap. Officially documented protocol.**

| Aspect | Details |
|--------|---------|
| Models | BM525s, BM527s, BM821s, BM829s, BM867s, BM869s |
| Also compatible | BM257s, BM250s (via BRUA-20X cable, same VID/PID) |
| Price range | ~$120 (BM257s) to ~$340 (BM869s) |
| Counts | 60000 (BM86x), 40000 (BM82x), 50000 (BM52x) |
| Connection | USB HID via **BU-86X** optical IR cable (~$40) |
| USB chip | Cypress CY7C63743 enCoRe in cable |
| VID:PID | `0820:0001` |
| Protocol | LCD segment bitmap, 72 bytes as 3x24-byte HID reports |
| Trigger | 4-byte command: `\x00\x00\x86\x66` |
| Direction | Read-only (no meter control commands) |
| Sigrok driver | `brymen-bm86x` (fully supported) |

#### Protocol details

The BU-86X cable contains a Cypress CY7C63743 enCoRe USB controller that
reads the meter's optical IR output and presents it as a USB HID device.

**Communication sequence:**
1. Host sends 4-byte HID report: `\x00\x00\x86\x66`
2. Meter responds with 3 HID interrupt reports of 24 bytes each (72 bytes total)
3. The 72 bytes encode the LCD segment bitmap — every segment of the LCD display is mapped to a specific bit
4. Software must decode 7-segment digit patterns into numeric values

**Protocol documentation:** Brymen provides official protocol PDFs
(e.g., `BM250-BM250s-6000-count-digital-multimeters-r1.pdf`) documenting
the segment-to-bit mapping.

#### Community popularity

**EEVBlog forums:**
- "If Brymen BM869s is cheaper and as good, why people would still buy Fluke?" — 17+ page thread
- Multiple dedicated review and comparison threads
- The BM869s is a go-to recommendation in the $200-350 range
- Users frequently complain about the official PC software

**GitHub projects (6+):**
- [TheHWcave/BM869S-remote-access](https://github.com/TheHWcave/BM869S-remote-access) — Python, Linux/Win CLI
- [freedaun/Brymen-BM869s](https://github.com/freedaun/Brymen-BM869s) — Python, Windows logger
- [kittennbfive/869log](https://github.com/kittennbfive/869log) — C + AVR firmware, DIY cable
- [DawOp/Brymen869s-XmlLib](https://github.com/DawOp/Brymen869s-XmlLib) — C++, Windows library
- [sadol/brylog](https://github.com/sadol/brylog) — Python/matplotlib, BM257 (serial variant)
- [MartinD-CZ/brymen-867-interface-cable](https://github.com/MartinD-CZ/brymen-867-interface-cable) — DIY STM32 cable (sold on Tindie)

#### Software gap analysis

| Software | Platform | Type | Status |
|----------|----------|------|--------|
| **Brymen official** (Bs86x Data Logging Express v6.0.0.3s) | Windows only | GUI | Buggy, dated (2012), Java-dependent, broken on Win11, "truly horrible UI" per EEVBlog |
| **sigrok-cli** | Cross-platform | CLI | Fragile — version-sensitive, "broken for several years" on some Linux distros |
| **sigrok GUIs** (sigrok-meter, SmuView) | Cross-platform | GUI | Both marked "development state" / "not suitable for everyday use" |
| **TheHWcave/BM869S-remote-access** | Linux/Win | CLI (Python) | 3 stars, hangs if meter turned off |
| **freedaun/Brymen-BM869s** | Windows | CLI (Python) | 2 stars, minimal |
| **kittennbfive/869log** | Linux | CLI (C) | Requires DIY hardware (ATtiny25), not BU-86X compatible |
| **HKJ's Test Controller** | Java (cross-platform) | GUI | Most capable option but general-purpose lab tool, not modern |

**No native cross-platform application with a modern GUI exists.**

#### Implementation considerations

- **New transport:** The Cypress CY7C63743 is not a UART bridge — it uses raw HID reports. New `Transport` impl needed, but arguably simpler than CP2110 (no baud rate config, no UART framing).
- **LCD segment decoder:** New parsing paradigm. 72-byte bitmap → 7-segment digit decode → numeric values + mode + flags. Well-documented in Brymen PDFs.
- **Device tables:** Per-model segment position mappings. Similar models share layout.
- **Read-only:** No bidirectional control — simpler than polled protocols.

---

### UNI-T UT71A–E — SUPPORTED (EXPERIMENTAL) SINCE 2026-09-21

**Implemented as `ut71ab`, `ut71cde` and `vc920`** — see
[supported devices](../supported-devices.md) and the spec in
[research/ut71](ut71/reverse-engineered-protocol.md); the Tenma rebrands are
sigrok's claim and have no registry entry. The analysis below is as it stood
before.

**Nothing new below the parser: the cable, the bridge and the packet shape are
all ones we already handle, and UNI-T publishes the protocol.**

| Aspect | Details |
|--------|---------|
| Models | UT71A, UT71B, UT71C, UT71D, UT71E |
| Rebrands | Tenma 72-7730 / 72-7732; Voltcraft VC920 / VC940 / VC960 — UNI-T ships the same protocol file under both brands (below) |
| Connection | UT-D04 optical cable. UNI-T's [accessory page](https://meters.uni-trend.com.cn/content/4381.html) lists exactly `UT71A、UT71B、UT71C、UT71D、UT71E` for it |
| USB chip | CH9325 (`1A86:E008`), HE2325U (`04FA:2490`) on older cables — per sigrok; UNI-T names no bridge chip anywhere on its site |
| Protocol | Vendor document, read 2026-09-21 (below) |
| Direction | Meter to PC — the document describes no command |
| Sigrok driver | libsigrok has a `ut71x` parser, already cited by our UT803/UT804 cross-reference |

#### Protocol details

["UT71系列接口协议"](https://meters.uni-trend.com.cn/static/upload/file/20211030/ut71%20%E7%B3%BB%E5%88%97%E5%8D%8F%E8%AE%AE.rar),
a `.rar` holding `UT71通信协议.xls`, from the handheld download centre;
archived with its provenance in `references/ut71/`. One sheet, read 2026-09-21:

- 2400 baud, and the sheet says 8 data bits, no parity, 1 stop bit. sigrok and
  the `UT804.LOG` capture in our UT803/UT804 cross-reference both give 2400
  **7O1** for this format, while our own UT804 work found the vendor app
  opening its port at 8N1 and reading only the low nibbles — the same bytes
  either way, with bit 7 carrying parity rather than data.
- An 11-byte packet: bytes 0-4 the five LCD digits, byte 5 the range, byte 6 the
  function, byte 7 the coupling (`0000` off, `0001` AC, `0010` DC, `0011`
  AC+DC), byte 8 the sign and range mode (`x0xx` +, `x1xx` −, `xx01` auto,
  `xx10` manual), bytes 9-10 CR LF.
- A function table 0-15 with a range list each: 0 AC mV, 1 DCV, 2 ACV, 3 DC mV,
  4 Ω, 5 capacitance, 6 °C, 7 µA, 8 mA, 9 10 A, 10 `Fm`, 11 diode, 12 Hz,
  13 °F, 14 blank, 15 % (4-20 mA).
- The sheet leaves every byte's high nibble as `xxxx` and describes no checksum.

This is the layout `protocol/ut80x` already decodes for the UT803 and UT804,
and the function numbering matches the UT71x numbering our UT803/UT804
cross-reference took from sigrok (1 = V DC, 2 = V AC, 15 = loop current) — so
a vendor source now confirms what was a community-only reading.
`[UNVERIFIED]`: the counts of each model, whether
the high nibbles carry a position index as they do in UT804 packets, and
whether UT71D/E add modes the sheet does not list.

The archive's second file, `VC920_940_960 Protocol.xls`, differs as bytes but
renders identically — one document filed under two brands, so a UT71 decoder is
also a Voltcraft VC920/VC940/VC960 decoder.

#### Software gap analysis

| Software | Platform | Type | Status |
|----------|----------|------|--------|
| **UNI-T UT71 interface software** | Windows only | GUI | Shipped as a CD image, re-uploaded 2026-05-15, marked "适用于Win7系统" (for Windows 7) |
| **sigrok-cli** | Cross-platform | CLI | Works (`ut71x`) |

#### Implementation considerations

- **No transport work:** `transport/ch9325.rs` is used unchanged.
- **Framing already exists:** the 11-byte CR-LF reader in `protocol/ut80x`
  covers it; the work is a UT71 range and function table beside the UT803 and
  UT804 ones.
- **Read-only**, like the UT803/UT804 — no command surface to design.
- **Three brands for one implementation** (UT71, Tenma, Voltcraft VC9x0).
- Needs hardware to verify, like every family we have not held.

---

### UNI-T UT632 / UT632N

**Same bridge as the UT803, a different framing — and the vendor code we hold
decodes nothing.**

| Aspect | Details |
|--------|---------|
| Models | UT632, UT632N (bench DMM) |
| Connection | USB HID, driverless |
| VID:PID | `1A86:E008` — the CH9325 bridge we already drive |
| Protocol | Not the UT803's. The UT803 app's UT632 configuration ends each frame at a byte whose high nibble is E and decodes nothing; what the frames carry needs a capture. Spec: [research/ut632](ut632/reverse-engineered-protocol.md) |
| Vendor document | UNI-T's shared bench programming manual V1.1 lists it beside the UT803/UT804/UT8802/UT8803 with the device address `[C:DM][D:T632][T:HID][PID:0xe008][VID:0x1a86]`. No UT632 protocol document exists on either Chinese site |

The UCI layer these bench meters share is already specified in
[research/uci-bench-family](uci-bench-family/reverse-engineered-protocol.md),
which names the UT632/UT632N throughout.

**The UT803 vendor software we already hold carries a UT632 configuration that
the shipped form does not select.** Its `FormCreate` brands itself "UT632
Interface Program _Ver: 2.00" behind a `UT632` checkbox that the shipped form
leaves unchecked and out of view; the checked `IFUT803` box wins, so the exe
runs as the UT803 app. The `UT632` box installs **`H60BRData`** on the serial
port — not the `H70BRData` of the UT803 path — and no HID handler at all. The
same app also carries UT60D/UT60E, UT70B, UT61A-E, PR3315 and 5198x boxes, and
a `UT71D` box whose field nothing in the decompile reads.

`H60BRData` ([research/ut632](ut632/reverse-engineered-protocol.md)) reads
the port at 2400 baud, ends each frame at a byte whose high nibble is E and
decodes nothing — UT803.exe holds no decoder for it. The same routine in UT804.exe requires 14 bytes whose
high nibbles spell `123456789ABCDE` and 7-segment decodes them
([research/ut803 §2.4](ut803/reverse-engineered-protocol.md)), so the UT632
most likely sends 14-byte frames with index high nibbles and the data in the
low nibbles `[DEDUCED]`. It is not the UT803/UT804's 11-byte CR LF packets,
none of whose bytes has high nibble E. What the low nibbles encode — LCD
segments as on the UT60A/B/C, or something else — is `[UNVERIFIED]` and needs
a capture, as do the frame length, the line format and whether the meter needs
a button press to send.

Should the UT632 use the 14-byte index-nibble framing, the extractor for it is
recoverable from git: `extract_frame_fs9721`, removed in 1693093 when the UT804
turned out to send 11-byte packets instead. Only the framing is reusable — the
parser above it read those nibbles as the UT803/UT804 structured layout, not as
LCD segments.

---

### Victor 70C / 86C

**Secondary USB HID candidate. Cheap, protocol documented, smaller community.**

| Aspect | Details |
|--------|---------|
| Models | Victor 70C (~$44), Victor 86C (~$55) |
| Connection | USB HID built into meter (70C) or in cable (86C) |
| USB chip | Unknown unmarked SO-20 chip |
| VID:PID | Unknown (not documented in surveyed sources) |
| Protocol | 14-byte FS9922-DMM4, obfuscated |
| Direction | Read-only |
| Sigrok driver | `victor-dmm` (supported) |

#### Protocol details

Data is obfuscated before reaching the USB host:
1. Subtract ASCII values of `"jodenxunickxia"` from each of the 14 bytes
2. Reshuffle byte positions
3. Reverse bits in each byte
4. Result is standard FS9922-DMM4 LCD segment data

Documented on the [sigrok Victor protocol page](https://sigrok.org/wiki/Victor_protocol).

#### Software gap analysis

| Software | Platform | Type | Status |
|----------|----------|------|--------|
| **Victor official** | Windows only | GUI | Hard to find, outdated, "many issues" |
| **sigrok-cli** | Cross-platform | CLI | Works |
| **mvneves/victor70c** | Linux | CLI (C) | 17 stars, actively maintained |
| **bborncr/Victor70C-Tools** | Cross-platform | GUI (Python/matplotlib) | Self-described as "barely working", uses serial not HID |

Gap exists but is less acute. Smaller user base, budget meters.

#### Implementation considerations

- Unknown USB chip (need physical device or sigrok source for VID/PID)
- Obfuscation is trivial once understood
- FS9922 segment data is reusable across other FS-chipset meters

---

## Bluetooth LE Candidates

BLE is where the largest unmet demand lives. sigrok's BLE is Linux-only
and flaky. No cross-platform desktop BLE multimeter tool exists with a
modern GUI.

### UNI-T Meters via UT-D07B BLE Adapter — SUPPORTED SINCE 2026-09-22

**Implemented as the Bluetooth transport** — auto-detected, or pinned with
`--adapter <address>`; verified on our UT61E+. What the adapter does on the
wire is in [research/ut-d07b](ut-d07b/reverse-engineered-protocol.md); the
UT-D07A and the UT202S entry are still open ([backlog](../verification-backlog.md)).
The analysis below is as it stood before, except the native-BLE paragraphs,
rewritten 2026-09-25 from the iDMM2.0 app.

| Aspect | Details |
|--------|---------|
| Adapter | UT-D07A / UT-D07B (~$30) |
| Connection | BLE 5.0 via ISSC BL79 BLETR chip |
| Compatible meters | UT-D07B: the UT61+ series, per UNI-T's [accessory page](https://meters.uni-trend.com.cn/content/4375.html). UT-D07A: `UT513B, UT513C, UT513D, UT512E` ([page](https://meters.uni-trend.com.cn/content/4374.html)). UT171 and UT181A carry Bluetooth on their own spec sheets |
| Protocol | **Transparent BLE-to-UART bridge** — same protocol as wired USB. UNI-T's UT61+ protocol deck is titled the Bluetooth protocol for the UT161, UT61+ and UT202S ([research/ut61-family](ut61-family/reverse-engineering-approach.md)) |

**Why this is the highest strategic value:** dmm-tools already parses
UT61E+/UT171/UT181A protocols. The UT-D07B is a transparent UART bridge
over BLE — adding BLE transport would unlock wireless operation with
**zero protocol changes**. All existing device tables and protocol
parsers work unmodified.

**It also reaches two meters we cannot reach over USB at all.** The deck
specifies the **UT202S** clamp meter with a full range table, and the
**UT216XD** clamp meter as speaking the same frame as the UT61+ — read from the
archived slides 2026-09-21, the only stated difference being that the UT216XD
has no bargraph, so `Msg[12]–Msg[13]` can be ignored. The deck's function table
already carries the clamp modes (`0x16` clamp ACA, `0x17` clamp DCA, `0x1C`
clamp LPF, `0x1D` clamp AC+DC). `[UNVERIFIED]`: the deck gives range tables for
the UT61B+/D+, UT61E+ and UT202S but **none for the UT216XD**, and states no
transport for either clamp.

**Native-BLE UNI-T meters** (no adapter), from their Chinese product pages,
surveyed 2026-09-21: UT60BT, UT202BT, UT117C, UT197 and UT197 PV, UT219P,
UT219PV, UT281F, UT217A/B, UT505A BT, UT513C, UT513E/UT515C,
UT513G/UT515A+/UT516E, UT251A+/UT252C/UT253C, UT253A/B, UT267C, UT275+,
UT677A+/UT677C, UT620E, UT343E.

UNI-T's phone client is **iDMM2.0** (优利德智测). Its manual carries no model
list at all, but the archived APK (`references/idmm2/`) does. Read 2026-09-25
(working note `references/idmm2/analysis/findings/protocol-groups.md`), it
picks a decoder by advertised name alone, and every model but one talks over
the same ISSC transparent-UART service as the UT-D07B. The
`0000ff01`/`ff02`/`ff12` set it also carries is the UT513C's "Old" firmware
and nothing else's (`UT513Manager`). All frames start `AB CD` and end in a
16-bit byte sum, but they fall into four groups [VENDOR]:

- **The UT61+ parser: UT60BT and UT202BT.** Same 19-byte frame, same parser
  (`TestDataModel.anylseData`) and same 0x5F then 0x5D handshake as the UT61+
  and UT161 behind a UT-D07B; the meters advertise names starting `UT60BT` and
  `UT202BT` (one UT60BT advertises `UT60BTk`, per community sources). Only
  the range tables differ, shipped as `funOl1_UT60BT.json` (9999 counts,
  999.9 mV to 999.9 V, nF to mF, NCV) and `funOl1_UT202BT.json` (clamp: 600 V
  and A, INRUSH, LPF, peak) in the UT61+ tables' schema. A frame with bit 7 of
  byte 3 set carries a secondary display, for any model. **Implemented
  2026-09-25**, experimental: the transport accepts their names and the
  registry has `ut60bt` and `ut202bt` with these tables.
- **The UT171 and UT181A**, over the UT-D07A/B: 16-bit little-endian length,
  binary floats. Already supported.
- **A 2-byte big-endian length: UT117C, UT197/UT197PV, UT219PV.** One polled
  frame, `AB CD 00 04 05 00 01 81` every 300-600 ms, replying with ASCII value
  and unit strings; UT61+'s checksum rule. The field layout differs per model
  (`UT117cManager`, `UT197Manager`, `UT219pvManager`), so this would be a new
  family with a layout per model.
- **Formats of their own**, one model each, all polled unless noted:
  - UT219P: big-endian length, little-endian sum from byte 2; paged reads
    (cmd 05 + page 0-8), name cmd 0x17.
  - UT513C: big-endian length, big-endian sum over command and payload only;
    `AB CD 00 03 05 00 05`; ISSC or the `ff` set, whichever the meter has.
  - UT505A: the UT181A's framing; `AB CD 03 00 05 08 00` every 500 ms.
  - UT501E: little-endian length; the app sends no poll, so the meter
    presumably pushes.
  - UT503PV: polls like the UT505A, answers with a big-endian length and sum.
  - UT251C+ and UT275A: `AB CD 04 00 05 <type> cs` every 300 ms; big-endian
    length excluding the sum, which is little-endian on the UT251C+ and
    big-endian on the UT275A.

The app has no code, asset or resource naming the UT343E, UT620E, UT281F,
UT217A/B, UT677A+/C, UT253A/B/C, UT252C, UT251A+, UT267C, UT513E/G,
UT515A+/C or UT516E: no source for their protocols is known.

The UT60BT was the **#1 recommendation** in the 2024 EEVBlog "General
Purpose Multimeter Recommendations" thread for logging multimeters. The
official UNI-T "Smart Measure" and "iDMM2.0" apps work but are
phone-only — no desktop logging tool exists for BLE mode.

---

### EEVblog 121GW

**Specified 2026-09-26 from EEVblog's packet-format documents and app, UEi's
app and the manual: [research/121gw](121gw/reverse-engineered-protocol.md).
Implemented 2026-09-26, experimental.**

| Aspect | Details |
|--------|---------|
| Price | ~$221 (currently sold out on official store) |
| Connection | BLE 4.0 via BLE112 module (UART bridge) |
| Protocol | 19-byte binary packets, partially documented |
| Sigrok driver | Supported (Linux only, "slow ~2 samples/sec, occasionally flaky") |

#### Community popularity

**Extremely high.** The main EEVBlog 121GW Issues thread spans **292 pages**.
Dave Jones' personal meter, sold thousands of units. One of the most
discussed handheld DMMs in the hobbyist space.

**Reverse engineering:** [tpwrules/121gw-re](https://github.com/tpwrules/121gw-re)
(60 stars) — comprehensive RE including firmware disassembly. Unofficial
firmware: [121gw-88mph](https://github.com/tpwrules/121gw-88mph) (23 stars).

#### Software gap analysis

| Software | Platform | Type | Status |
|----------|----------|------|--------|
| **Official 121GW app** | iOS/Android/Windows | GUI (Xamarin) | Open source (33 stars), development sporadic, limited data logging |
| **Meteor for 121GW** | iOS | GUI | 3 ratings, beta since 2020 |
| **sigrok** | Linux only (BLE) | CLI | "Slow and occasionally flaky" |
| **zonque/121gw-qt5** | Cross-platform | GUI (Qt5) | 11 stars, last updated Jan 2024 |
| **chlordk/121gwcli** | Linux | CLI | 6 stars, updated Jan 2026 |

**Gap: moderate.** An official app exists (even cross-platform via
Xamarin) but has limited logging. No polished native desktop app with
real-time graphing. The community is large but software solutions are
fragmented.

**Brymen rebrands on the same store (a future candidate, not researched).**
[EEVblog's store](https://eevblog.store/collections/multimeters) sells the
BM787BT, BM2257, BM786, BM235 and BM036, which are Brymen meters under
EEVblog's name; the
[BM787BT](https://eevblog.store/products/eevblog-bm787bt-bluetooth-multimeter)
has Bluetooth, and a BLE protocol document for it is published (noted
2026-09-26).

---

### OWON B35T+ / B41T+

| Aspect | Details |
|--------|---------|
| Price | B35T+ ~$80-100, B41T+ ~$109 |
| Connection | BLE 4.0 (no USB data — Bluetooth only to phone, or proprietary OWON USB BLE dongle for PC) |
| Protocol | 14-byte BLE GATT packets, well reverse-engineered |
| Chip variants | Fortune FS9922 (pre-2017), Semic CS7729CN-001 (post-2017) |
| Sigrok | Not supported via BLE |

#### Community popularity

Moderate-high. Popular budget BLE logging meters, commonly recommended
on forums.

**GitHub projects:**
- [DeanCording/owonb35](https://github.com/DeanCording/owonb35) (34 stars) — Linux C client, CSV/JSON output, interactive control
- [sercona/Owon-Multimeters](https://github.com/sercona/Owon-Multimeters) (34 stars) — Linux, B35T+/B41T+/CM2100B/OW18E
- [inflex/owon-b35](https://github.com/inflex/owon-b35) (12 stars) — older C tool for FS9922 chip models

#### Software gap analysis

| Software | Platform | Type | Status |
|----------|----------|------|--------|
| **OWON official** (OWON Share) | Windows only | GUI | **Requires proprietary OWON USB BLE dongle** — does not work with standard BLE adapters |
| **OWON Multimeter BLE4.0** | iOS/Android | Mobile app | Basic functionality |
| **owonb35 / sercona** | Linux | CLI (C) | Gattlib-based (notoriously finicky), no GUI |
| **Bluetooth-DMM-For-Windows** | Windows | GUI (.NET) | 47 stars, development inactive ("probably there will not any Update") |

**Gap: high.** Official PC software requires a proprietary dongle. No
cross-platform GUI desktop app exists. Linux tools use fragile Gattlib.
The 47-star Windows-only app is abandoned.

---

### ZOTEK BLE Meters: ZOYI / ZOTEK / BSIDE / ANENG (ZT-300AB = AN9002, ZT-5B, ZT-5BQ, ZT-5566)

**One OEM, one protocol, four packet layouts. Specified 2026-09-25 from
ZOTEK's apps and manuals: [research/zotek](zotek/reverse-engineered-protocol.md).
Implemented 2026-09-26, experimental: all four layouts.**

| Aspect | Details |
|--------|---------|
| Models | ZT-300AB, ZT-5B, ZT-5BQ (clamp), ZT-5566/5566S/5566SE (19999-count desktop meter with a Bluetooth speaker), ZT-6S — made by Shenzhen ZOTEK, sold as ZOYI/ZOTEK and BSIDE. ANENG rebrands, by matching specs and keys: AN9002 = ZT-300AB, V05B = ZT-5B, ST207 = ZT-5BQ, AN999S ≈ ZT-5566S/SE |
| Connection | BLE, service `FFF0`; ZOTEK's older apps notify and write on characteristic `FFF4`. The apps and manuals list the meter as "Bluetooth DMM" |
| Protocol | Streamed unprompted. Each notification is XORed with a fixed 20-byte key; descrambled it starts `5A A5 <type>`, and the type byte (1-4) selects one of four layouts of LCD segments and annunciator bits. No checksum; the apps read 10, 10, 11 or 19 bytes by type, and the actual length is unverified. Remote key presses and a clock set go back as `AB CD` frames with a 16-bit sum |
| Apps | e-Bull V2 1.1.2 (`com.zoyi.bleapp`, uni-app, ZOTEK's current app); e-Bull V1.0.12 (`com.yscoco.multimeter`) and its white-label twin Bluetooth DMM 1.0.13 (`com.yscoco.wyboem`), which the AN9002 manual points to |

Which model sends which type byte is inferred from the layout names alone
(`AB_300`/`300ab` for the ZT-300AB, type 3, the priority); none of the apps
ties a type to a model. Community captures fit it for four models (AN9002 /
ZT-300AB 3, V05B / ZT-5B 2, ST207 1, ZT-5566SE 4). The ZT-5566 manuals
document the Bluetooth speaker, and the SE manual's app section names only other models. Every open question is in the
[backlog](../verification-backlog.md#zotek-zoyi--aneng--bside-experimental-awaiting-a-hardware-report).

**Gap: moderate.** No cross-platform desktop tool; Bluetooth-DMM-For-Windows
is Windows-only and inactive. The ZT-300AB/AN9002 pair ranks first of the
family in our listing survey.

---

### Mooshimeter (Discontinued)

| Aspect | Details |
|--------|---------|
| Price | ~$150 (was), no longer manufactured |
| Connection | BLE via TI CC2540 SoC |
| Protocol | Tree-based config system over BLE GATT, well documented |
| Sigrok | Supported (Linux BLE only) |

**Orphaned product.** Thousands in circulation, official app unmaintained
since 2018, breaks on newer OS versions. Multiple community rescue
projects keep appearing (3 repos updated 2025-2026).

**GitHub:** [mooshim/Mooshimeter-PythonAPI](https://github.com/mooshim/Mooshimeter-PythonAPI) (37 stars,
requires BLED112 dongle), [ghtyrant/libsooshi](https://github.com/ghtyrant/libsooshi) (21 stars).

**Gap: high but shrinking.** Real orphaned users but no new customers.

---

### Pokit Pro / Pokit Meter

| Aspect | Details |
|--------|---------|
| Price | ~$100-130 |
| Connection | BLE, multimeter + oscilloscope + data logger |
| Protocol | Partially documented, reverse-engineered by dokit project |

**Already well-served** by [pcolby/dokit](https://github.com/pcolby/dokit) (63 stars,
C++/Qt, cross-platform CLI, actively maintained through April 2026).
**Not a priority target.**

---

## USB Serial Candidates

### Fluke 287/289/189/187 (IR-optical to serial)

| Aspect | Details |
|--------|---------|
| Price | Fluke 289 ~$600+, IR189USB cable ~$87 |
| Connection | IR-optical → FTDI FT232RL USB-serial (115200 baud for 287/289, 9600 for 87-IV/89-IV) |
| Protocol | **Officially documented** ASCII text. 2-letter commands (QM, ID, RI, SF, DS). QM returns e.g. `9.323E0,VDC,NORMAL,NONE` |
| Sigrok driver | `fluke-dmm` (fully supported) |

#### Community popularity

**High.** Potentially millions of Fluke 287/289 meters in the field.
FlukeView Forms has **2.4/5 stars on Fluke's own website.** The EEVBlog
"FlukeView Forms alternative" thread shows clear demand.

#### Software gap analysis

| Software | Platform | Type | Status |
|----------|----------|------|--------|
| **FlukeView Forms** | Windows only | GUI | **$200**, terrible reviews (2.4/5 on Fluke.com) |
| **Fluke Connect** | Mobile | App | Crashes, notification spam, $1000/year subscription for data save |
| **sigrok** | Cross-platform | CLI | Works |
| **dmm_util** | Cross-platform | CLI (Python) | 18 stars, downloads recordings |
| **Various Python scripts** | Varies | CLI | Small, fragmented |

**Gap: large.** The official software is expensive ($200), Windows-only,
and terrible. Fluke Connect is subscription-based and unreliable. The
ASCII protocol is trivial to implement. However, requires serial transport
(not HID) and the $87 IR cable.

**Note:** Fluke 115/175/177/179 do NOT have IR ports or computer
connectivity.

---

### OWON XDM1041 / XDM1241 / XDM2041 (USB serial SCPI)

| Aspect | Details |
|--------|---------|
| Price | XDM1041 ~$80-90, XDM2041 ~$130-150 |
| Connection | USB serial via CH340, SCPI protocol, 115200 baud |
| Protocol | SCPI (standard, documented) |

**Already well-served** by [markusdd/rusty_meter](https://github.com/markusdd/rusty_meter)
(**100 stars** — same tech stack: Rust/egui). Also:
[TheHWcave/OWON-XDM1041](https://github.com/TheHWcave/OWON-XDM1041) (63 stars).

rusty_meter validates the Rust/egui approach for multimeter desktop apps.
**Not a priority target** since it's already well-served.

---

### UNI-T UT8805 / UT8805A / UT8806 / UT8806A (USB TMC/SCPI)

**UNI-T's other bench multimeter generation — same brand as the UT8802/UT8803
we support, an entirely different interface. Specified 2026-09-22 from
UNI-T's manuals, firmware and tools:
[research/ut8805](ut8805/reverse-engineered-protocol.md).**

| Aspect | Details |
|--------|---------|
| Models | UT8805N, UT8805A, UT8805E (5½ digits); UT8806, UT8806A, UT8806E (6½ digits). The UT805A+ listing serves the UT8805N files. No rebrand found, and no sign of an OEM link in either direction |
| Connection | Rear USB device port, LAN (RJ45), RS-232 (DB9); GPIB option |
| USB | Genuine **USBTMC**: interface FE/03/01, bulk endpoints only, no interrupt IN, no USB488 features (no REN/GTL/LLO, no status byte). Two ID lines: UT8805N/E `0483:7540`; UT8805A and UT8806/A/E `0483:5740`. Both IDs are shared with generic ST products (a thermal printer, ST's virtual COM port), so neither identifies the meter on its own; the interface class and `*IDN?` do |
| LAN | VXI-11 (`inst0`) on every line. A raw SCPI socket on **5025** on the UT8805A/UT8806 line only; that line serves HTTP too, and whether the UT8805N/E line does is disputed (spec §3.1). A UT8805E on V1.87.014 returned VXI-11 replies unpadded to 4 bytes (a strict XDR client fails; NI-VISA tolerates it), reportedly fixed in V1.87.017 |
| RS-232 | SCPI over the DB9 reported working on a UT8805E (the manual's default line is 9600 8N1); terminator and handshake unrecorded |
| Protocol | Plain query/response SCPI: `CONF?` gives function and range, then `READ?` or `DATA:LAST?` (the latter usable at any time, with a unit suffix). Overload is ±9.9E37. Replies end `\r\n` on the UT8805N/E, `\n` on the others. Queries can put the meter in remote (which ones varies by line: any query on the UT8805N/E, `READ?`/`FETCh?`/`MEAS?` on the UT8805A, every command but `*IDN?` on the UT8806); whether the keys lock is unverified; `*UNREMOTE` (UT8805) or `SYST:LOC` (UT8806) return it to local |
| Vendor tools | UNI-T's PC apps, IVI-C and LabVIEW drivers all go through NI-VISA; the UNI-T SDK (`uci.dll`) does not cover these meters |

**UNI-T's bench line splits in two.** The UT632, UT803, UT804, UT805A, UT8802
and UT8803 speak the UCI protocols we implement, over HID or a COM port. The
UT8805/UT8806 generation is a standard SCPI instrument, and its command tree
follows the same Keysight Truevolt run as Rigol's DM858 and Siglent's SDM:
the UT8805 manuals open with `ABORt`, `FETCh?`, `INITiate`,
`OUTPut:TRIGger:SLOPe`, `READ?`, `SAMPle:COUNt`, `UNIT:TEMPerature`, and
every image registers those (except `OUTPut:TRIGger:SLOPe` on the
UT8806) plus `DATA:LAST?`/`POINts?`/`REMove?` (the
quoted `FUNC?` reply and the `DATA:LAST?` unit suffix are Truevolt's own
behaviour, not departures; `R?` is left out — the manuals name it in prose
only and the UT8805A/UT8806 images give it no handler). UNI-T's real
departures: an unquoted `CONF?` reply with no resolution field, the long
`VOLT:DC` token, `*UNREMOTE`/`SYST:RWL`, `\r\n` replies on the UT8805N/E,
and NPLC as `Slow|Medium|Fast` on the UT8805. The UT8806 manual does not open with the run
and the UT8806 images do not register `OUTPut:TRIGger:SLOPe`. So it is a
third Truevolt-style dialect, and supporting it is the same decision as the
Rigol/Siglent one below, not the same hardware — the earlier "same call as
Rigol/Siglent" wording meant that.

**What it would take**, costed by work content:

- dmm-lib's open path is hidapi-only (`open_transport`, `KNOWN_TRANSPORTS`
  in `crates/dmm-lib/src/lib.rs`) and auto-detection probes over HID
  bridges. Any of these meters needs an address-based open (host or device
  path) and identification by `*IDN?`. The `Protocol` trait is poll-based
  (`request_measurement`) and fits SCPI query/response as it is; the
  `Transport` trait (`crates/dmm-lib/src/transport/mod.rs`) is written
  around HID reports and requires the HID-only `send_feature_report`, so a
  network transport has to fit or change that trait. The SCPI text replies
  need a new protocol family; no existing frame parser applies.
- Transports, cheapest first: (1) **VXI-11 over `std::net`** — portmapper
  plus core channel, a few hundred lines of XDR, std only, cross-platform,
  reaches every model, must tolerate the UT8805N/E's unpadded replies;
  (2) **raw socket 5025 over `std::net`** — trivial, UT8805A/UT8806 line
  only; (3) **Linux `/dev/usbtmcN` through `std::fs`** — the kernel adds the
  headers; Linux only, plus a udev rule; (4) **USBTMC over libusb/nusb** on
  macOS and Windows — a new dmm-lib dependency against the "hidapi,
  thiserror, log only" rule in `.claude/rules/protocol.md`, and Windows
  needs WinUSB (Zadig) or a vendor VISA driver; (5) **RS-232** — the serial
  transport the UT805A and OWON XDM would also use, with its own dependency
  decision.
- Value: pyvisa, NI-VISA and UNI-T's own tools serve them; HKJ's
  TestController reaches the UT8805E over RS-232 only, on Linux and Mac as
  well as Windows. What dmm-tools adds is a cross-platform GUI logger over
  LAN or USB with no VISA install. A SCPI family with
  per-dialect command maps would reach Rigol DM858/DM3068, Siglent SDM and
  Teledyne T3DMM over the same transport.
- UX cost to name: queries can put the meter in remote; whether the keys
  lock varies by line and is unverified, and a session has to return the
  meter to local when it closes.

**Recommendation: Tier 2, go LAN-first when the project takes on a network
transport** — VXI-11 plus the 5025 socket over `std::net`, no
dependency-rule change, one SCPI family with a UNI-T command map first and
Rigol/Siglent maps after. Defer USB TMC until the dependency decision is
made, with Linux `usbtmc` as the cheap interim. Not ahead of the Tier 1
items. Experimental plus a verification issue, since nobody on the project
owns one; the open hardware questions are in the
[backlog](../verification-backlog.md#ut8805ut8806-open-questions-before-a-scpi-implementation).

---

### Rigol / Siglent Bench DMMs (USB TMC/SCPI)

Standard SCPI instruments, well-served by pyvisa, lxi-tools, sigrok and
vendor software. Surveyed from their official documents 2026-09-21
(`references/rigol-siglent/`, gitignored): **not rebrands of one protocol,
but separate SCPI dialects over a common USBTMC and LAN transport** (VXI-11 on
Siglent and Teledyne, the Rigols state LXI only; a 5025 socket where a port is documented at all — the DM858
and SDM3000 give one, the DM3058 and SDM4000A none), in two lineages:

- **Rigol DM3058/DM3068:** Rigol's own `:FUNCtion:…`/`:MEASure …` tree, with
  `CMDSET` switching to a 34401A- or Fluke 45-compatible set (RIGOL is the
  power-on default).
- **Keysight Truevolt layout (34460A/34461A):** Rigol DM858, Siglent
  SDM3000/SDM4000A, and Teledyne LeCroy T3DMM — a rebadged SDM3000 (a
  leftover "SDM3055" sentence in Teledyne's manual, matching firmware
  version strings, and a Siglent distributor's confirmation).

| | Rigol DM3058/DM3068 | Rigol DM858, Siglent SDM, T3DMM | UNI-T UT8805/UT8806 |
|---|---|---|---|
| Select DC V | `:FUNCtion:VOLTage:DC` | `CONF:VOLT:DC`, `FUNC "VOLT"` / `"VOLT:DC"` | `CONF:VOLT:DC`, `FUNC "VOLT:DC"` |
| One reading | `:MEASure:VOLTage:DC?` (no arguments) | `READ?`, `R?`, `DATA:LAST?` (DM858; Siglent SDM EN02A and SDM4000A §4.1) | `READ?`, `DATA:LAST?` with a unit suffix |
| Overload | not stated remotely | `9.9E37` | `±9.90000000E+37` |

UNI-T's UT8805/UT8806 tree follows the same Truevolt run, so it is a third
dialect of that lineage; no sign of an OEM link between Rigol, Siglent and
UNI-T was found. **Not a priority target on its own**, but a SCPI family built for the
UT8805/UT8806 (above) would reach these with per-dialect command maps over
the same transport.

---

## Not Yet Investigated

| Model | Brand | Type | Transport | Notes |
|-------|-------|------|-----------|-------|
| **UT612** | UNI-T | LCR meter | USB HID (`10C4:EA80`) | ES51919 chipset, TX-only, CP2110 transport. [sigrok wiki](https://sigrok.org/wiki/UNI-T_UT612) |
| **VC-870** | Voltcraft | Handheld DMM (40000 counts) | USB HID (`1A86:E008`) | CH9325 (UT-D04 cable), ES51966A chipset |
| **72-7730 / 72-7732** | Tenma | Handheld DMM | USB HID (`1A86:E008`) | UNI-T UT71 rebrands, CH9325 / HE2325U (UT-D04), per sigrok only. The UT71A–E entry above is implemented; a Tenma would be named as a UT71 |
| **UT804+** | UNI-T | Bench DMM (59999 counts per its Chinese product page) | USB (HID per UNI-T's download listing, unverified) | A newer model than the supported UT804 (40000 counts). A "UT804" [programming manual](https://instruments.uni-trend.com.cn/static/upload/file/20220920/UT804%E7%BC%96%E7%A8%8B%E6%89%8B%E5%86%8C%20REV.2.pdf) is the Chinese original of the UCI SDK manual (V1.1, 2019): it covers the UT804/UT804N and not the UT804+ ([research/uci-bench-family](uci-bench-family/reverse-engineered-protocol.md)). The [UT804+ page](https://instruments.uni-trend.com.cn/cate/143.html) lists software but no protocol document. The "UT804接口协议" on the [UT800 series page](https://instruments.uni-trend.com.cn/cate/140.html), read 2026-09-19, describes the UT804 alone (its first digit runs 0-4, a 40000-count display), so whether the UT804+ speaks a protocol we support is still open |
| **UT202S** | UNI-T | Clamp meter | Bluetooth, per UNI-T's protocol deck | Speaks the UT61+ protocol: the [deck](ut61-family/reverse-engineering-approach.md) gives its range table (V and A to 600, LPF, temperature) and says it sends a main and a secondary display in AC, LPF and temperature modes. Its [page](https://meters.uni-trend.com.cn/content/1340.html) offers only the UT202S/UT202BT manual. The Bluetooth transport now carries it; it needs a registry entry and a capture |
| **UT60BT** | UNI-T | Handheld DMM (9999 counts, per its manual) | Bluetooth LE, built in | The UT61+ protocol over the UT-D07B's ISSC service (Bluetooth section above). Implemented 2026-09-25 (`ut60bt`, experimental) from the iDMM2.0 range table and the manual; awaits a hardware report |
| **UT202BT** | UNI-T | Clamp meter (9999 counts, per its manual) | Bluetooth LE, built in | As the UT60BT, with a clamp table (INRUSH, LPF, peak) and a secondary display. Implemented 2026-09-25 (`ut202bt`, experimental), the secondary display as a sub-value; its Ω ladder differs from the deck's UT202S one, the rest is unchecked |
| **UT117C, UT197 / UT197PV, UT219PV** | UNI-T | Not recorded | Bluetooth LE, built in (ISSC) | One polled `AB CD` frame with a 2-byte length and ASCII readings, a field layout per model (Bluetooth section above). The iDMM2.0 app is the only source |
| **UT805A / UT805N** | UNI-T | Bench DMM (220000 counts) | Serial | USB-to-serial (virtual COM port, not HID), ASCII text protocol (9600/8N1, bidirectional); see [research/ut8803](ut8803/reverse-engineering-approach.md). The shared bench programming manual's device table gives `[T:COM][PORT:8][BAUD:9600][PARITY:N][STOP:1][DATA:7]` and a CP210x driver |
| **UT216XD** | UNI-T | Clamp meter | Unstated — the deck that specifies it is a Bluetooth protocol | Speaks the UT61+ frame, bargraph bytes aside (Bluetooth section above). **No archived source carries its ranges**: the deck names it only in the two bargraph exceptions, the iDMM2.0 APK has no UT216 package or range asset, and it has no page in the Chinese catalogue. Its range table would have to come from hardware or from vendor software we do not have |
| **UT61B / UT61C / UT61D / UT61E** | UNI-T | Handheld DMM (classic, pre-`+`) | UT-D04 (CH9325) in practice | UNI-T's "protocol" downloads are the chipset datasheets: ["UT61E接口协议"](https://meters.uni-trend.com.cn/static/upload/file/20220908/1662605553430400.pdf) is the Cyrustek **ES51922** (19230 baud, 7-odd-1) and ["UT61B通信协议"](https://meters.uni-trend.com.cn/static/upload/file/20220110/UT61B%20protocol.pdf) is the Fortune **FS9922-DMM3**. Long discontinued; sigrok covers both chipsets |
| **UT81A+ / UT81B+ / UT81C+ / UT81D+** | UNI-T | Handheld scopemeter | Type-C, transport unstated (CDC, TMC or HID) | Their pages advertise "支持SCPI通信功能，可二次开发" (SCPI, open to development) but publish no command list. The only UT81 protocol document, ["UT81系列接口协议"](https://meters.uni-trend.com.cn/static/upload/file/20211102/ut81b%E9%80%9A%E8%AE%AF%E5%8D%8F%E8%AE%AE.rar) (read 2026-09-21, `references/ut81/`), covers the **older UT81A/B**: 9600 8N1, records framed by `0x5A` with a decimal-digit length and checksum, a 15-byte instrument-state block, two 10-byte ASCII readings and an optional 160-sample waveform block, with `0x5A` doubled where it occurs in the data. It names no cable or bridge chip |
| **UT620A / UT620B** | UNI-T | 直流/回路电阻测试仪 — DC and loop resistance tester | USB, "免安装驱动" (driver-free) and bidirectional — a HID hint, unconfirmed | Not a DMM, but the closest neighbour to our framing: ["UT620B通信接口参数"](https://meters.uni-trend.com.cn/static/upload/file/20211123/UT620B%E9%80%9A%E4%BF%A1%E6%8E%A5%E5%8F%A3%E5%8F%82%E6%95%B0.docx) gives 19200 and a 23-byte `"ABCD"`-headed frame with a 2-byte sum, plus host commands (0x30 keypress, 0x31 one-shot read, 0x32 memory dump). Cable UT-D18 |

## Meters Investigated and Ruled Out

| Brand/Model | Connection | Why excluded |
|-------------|-----------|-------------|
| UNI-T clamp meters (UT200+ … UT281E+, ~35 models) | None | Their Chinese comparison tables have no 数据传输 (data transfer) column at all. Only the Bluetooth models listed above connect to anything (surveyed 2026-09-21) |
| UNI-T handhelds UT139, UT89x, UT19x, UT15/17/18B MAX, UT33x, UT58x | None | No interface on any Chinese product page (surveyed 2026-09-21) |
| Parkside PDM-300 | Internal UART (requires soldering) | Hardware mod required, tiny user base |
| CEM DT-9989 | USB CDC | Undocumented protocol, niche product |
| APPA 100/300/500/700 | USB serial + BLE | Niche (European professional), sigrok driver not merged |
| Gossen Metrawatt | IR-optical, proprietary binary | Niche, complex proprietary protocol |
| Keysight U1272A | IR to serial | Proprietary protocol |

---

## Priority Summary

### Tier 1: Highest value

| Candidate | Transport | Why | Gap |
|-----------|-----------|-----|-----|
| **UNI-T UT71A–E** — implemented 2026-09-21 | USB HID (CH9325) | Lowest cost of any candidate: the cable, the bridge and the 11-byte packet shape are already implemented, UNI-T publishes the protocol, and the Tenma and Voltcraft VC9x0 rebrands come with it | Done, experimental: `ut71ab`, `ut71cde`, `vc920` await a hardware report ([supported devices](../supported-devices.md)) |
| **Brymen BM86x** | USB HID (Cypress) | Official protocol docs, strong community, no cross-platform GUI exists | Large |
| **UNI-T via UT-D07B** — implemented 2026-09-22 | BLE | Reuses existing protocol parsers, #1 recommended logging meter on EEVBlog 2024, no desktop BLE tool | Done, verified on a UT61E+: the UT171 and UT181 series UNI-T lists on the adapter await a hardware report ([supported devices](../supported-devices.md)) |
| **UNI-T UT60BT / UT202BT** — implemented 2026-09-25 | BLE (built in) | The UT61+ protocol over the Bluetooth transport we have; the UT60BT is the #1 logging pick on EEVBlog 2024 | Done, experimental: `ut60bt`, `ut202bt` await a hardware report |
| **Fluke 287/289** | USB serial (IR) | Officially documented ASCII protocol, millions of units, $200 Windows-only software is terrible | Large |

### Tier 2: Worth considering

| Candidate | Transport | Why | Gap |
|-----------|-----------|-----|-----|
| **EEVblog 121GW** — implemented 2026-09-26 | BLE (built in) | Largest enthusiast community (292-page thread), fragmented software; specified from EEVblog's and UEi's apps and EEVblog's packet-format documents ([research/121gw](121gw/reverse-engineered-protocol.md)) | Done, experimental: `121gw` awaits a hardware report |
| **OWON B35T+/B41T+** | BLE | Popular budget BLE meters, no cross-platform GUI, proprietary dongle required for PC | High |
| **Victor 70C/86C** | USB HID | Cheap, protocol documented, no good software | Moderate |
| **UNI-T UT632/UT632N** | USB HID (CH9325) | Bench DMM on a bridge we already drive; the UT803 app's UT632 configuration frames its stream on a high-nibble-E byte but decodes nothing, so the payload needs a capture and the `ut80x` parsing does not carry over | Unmeasured |
| **UNI-T UT117C, UT197/UT197PV, UT219PV** | BLE (built in) | Three models on one polled frame over the Bluetooth transport we have; vendor-sourced from the iDMM2.0 app | Moderate: a new protocol family with a field layout per model |
| **ZOTEK BLE (ZOYI/BSIDE/ANENG)** — implemented 2026-09-26 | BLE (built in) | Specified from ZOTEK's own apps ([research/zotek](zotek/reverse-engineered-protocol.md)): one streamed protocol in ZOTEK's apps, which serve ZOYI/ZOTEK meters and their BSIDE and ANENG rebrands, led by the ZT-300AB/AN9002; no cross-platform desktop tool | Done, experimental: `zt300ab`, `zt5566se`, `zt5bq` and `zt5b` await a hardware report |
| **UNI-T UT8805/UT8806** | LAN (VXI-11, socket 5025); USB TMC; RS-232 | Specified ([research/ut8805](ut8805/reverse-engineered-protocol.md)); plain SCPI query/response that the poll-based `Protocol` trait already fits; a `std::net` VXI-11 transport reaches every model with no dependency change and opens a SCPI family for Rigol/Siglent maps; no cross-platform VISA-free GUI logger exists over LAN or USB (TestController covers RS-232) | Moderate: a network transport (the HID-shaped `Transport` trait must fit or change), a SCPI protocol family, address-based open and `*IDN?` identification; USB TMC deferred behind the dependency decision |

### Tier 3: Lower priority

| Candidate | Transport | Why excluded or deprioritized |
|-----------|-----------|-------------------------------|
| Mooshimeter | BLE | Discontinued, shrinking user base |
| OWON XDM series | USB serial SCPI | Already well-served by rusty_meter (100 stars, Rust/egui) |
| Pokit Pro | BLE | Already well-served by dokit (63 stars) |
| Rigol/Siglent bench | USB TMC/SCPI, LAN | Well-served by pyvisa/lxi-tools; separate SCPI dialects (Rigol DM3000 tree; Truevolt layout on DM858, Siglent SDM, Teledyne T3DMM). Would ride the UT8805/UT8806 SCPI family (Tier 2) as extra command maps over the same transport, not a project of its own |

### Strategic notes

- **BLE transport is the biggest unlock.** It enables UNI-T UT-D07B
  (reuses existing parsers), 121GW, and OWON B35T/B41T+ — three of the
  most-demanded meters; the first two are implemented. sigrok's BLE is
  Linux-only and experimental; no competitor fills this space
  cross-platform.
- **rusty_meter** (100 stars, Rust/egui, OWON XDM) validates the exact
  tech stack dmm-tools uses. Proves community demand for native desktop
  multimeter apps.
- **Other brands' BLE meters need more than names and tables.** The 121GW,
  OWON B35T+/B41T+ and ZOTEK meters (ZOYI/BSIDE/ANENG) are not known to use
  the ISSC service (the ZOTEK apps use service `FFF0`, characteristic `FFF4`;
  the 121GW has a service of its own), so each needs its own GATT path. The
  OWON protocol is documented only in community code, so it needs a
  clean-room source decision before work starts; the ZOTEK and 121GW
  protocols are specified from their vendors' own apps and documents
  ([research/zotek](zotek/reverse-engineered-protocol.md),
  [research/121gw](121gw/reverse-engineered-protocol.md)).
- **Bluetooth-DMM-For-Windows** (47 stars, now abandoned) proves demand
  for a multi-device BLE desktop app. Its Windows-only nature and
  inactivity leave the gap wide open.
- **Adding serial transport** is smaller scope than BLE but the
  highest-value serial target (Fluke) overlaps more with existing tools.
- **UT71 is the only UNI-T handheld family besides the UT61+ with a published
  wire protocol**, and is implemented (experimental) since 2026-09-21. The
  2026-09-21 sweep found none for the UT171, UT181A, UT161, UT139, UT89,
  UT19x, UT21x or the clamp line. UNI-T's newer bench multimeters
  (UT8805/UT8806) publish theirs as SCPI over USB TMC, LAN and RS-232, now
  specified ([research/ut8805](ut8805/reverse-engineered-protocol.md)): a
  Truevolt-style dialect that the poll-based `Protocol` trait fits, needing
  a network transport and a SCPI protocol family rather than another frame
  parser. So growth inside UNI-T's catalogue means UT632 over USB, the BLE
  models (native or over an adapter), or the SCPI bench meters over LAN — the last of which also opens Rigol and
  Siglent.

---

## Sources

### USB HID
- [sigrok Supported Hardware](https://sigrok.org/wiki/Supported_hardware)
- [sigrok Brymen BM869 wiki](https://sigrok.org/wiki/Brymen_BM869)
- [sigrok Brymen BU-86X/Info](https://sigrok.org/wiki/Brymen_BU-86X/Info)
- [sigrok Victor protocol](https://sigrok.org/wiki/Victor_protocol)
- [sigrok Device cables](https://sigrok.org/wiki/Device_cables)
- [improwis.com Multimeter chips](http://improwis.com/projects/reveng_multimeters/)

### Bluetooth LE
- [EEVBlog 121GW app (GitHub)](https://github.com/EEVblog/EEVblog-121GW)
- [tpwrules/121gw-re](https://github.com/tpwrules/121gw-re) — 121GW reverse engineering
- [DeanCording/owonb35](https://github.com/DeanCording/owonb35) — OWON B35 Linux client
- [sercona/Owon-Multimeters](https://github.com/sercona/Owon-Multimeters) — OWON multi-model support
- [ludwich66/Bluetooth-DMM](https://github.com/ludwich66/Bluetooth-DMM) — Aneng/BSIDE protocol docs (cross-referenced after the vendor-only ZOTEK spec, 2026-09-25: its §11)
- [Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows) — Windows BLE DMM app
- [pcolby/dokit](https://github.com/pcolby/dokit) — Pokit cross-platform tools
- [mooshim/Mooshimeter-PythonAPI](https://github.com/mooshim/Mooshimeter-PythonAPI)
- [sigrok Bluetooth support](https://sigrok.org/wiki/Bluetooth)
- [AN9002 BLE protocol analysis](https://justanotherelectronicsblog.com/?p=930) (cross-referenced after the vendor-only ZOTEK spec, 2026-09-25: its §11)

### USB serial / IR
- [markusdd/rusty_meter](https://github.com/markusdd/rusty_meter) — Rust/egui OWON XDM tool (100 stars)
- [TheHWcave/OWON-XDM1041](https://github.com/TheHWcave/OWON-XDM1041)
- [Fluke Remote Interface Specification](https://www.pewa.de/DATENBLATT/DBL_FL_FL187-9-89IV_BEFEHLSSATZ_ENGLISCH.PDF)
- [N0ury/dmm_util](https://github.com/N0ury/dmm_util) — Fluke 287/289 Python utility
- [FlukeView Forms alternative — EEVBlog thread](https://www.eevblog.com/forum/testgear/flukeview-forms-alternative/)
- [HKJ's Test Controller](https://lygte-info.dk/project/TestControllerIntro%20UK.html)

### UNI-T Chinese sites

Surveyed 2026-09-19, swept model by model 2026-09-21. Model pages are
`meters.uni-trend.com.cn/content/<id>.html` (handhelds) and
`instruments.uni-trend.com.cn/cate/<id>.html` (bench). File links on
`admin-meters…` fail; the same path on `meters…` works. The instruments site's
model pages hide their download links behind scripts; its download centre,
`instruments…/download?keyword=<model>`, lists them.

Both sites' searches match titles only, so a keyword sweep alone misses models.
Three things make a sweep complete:

- The handheld category pages are AJAX. The full model list comes from
  `POST meters…/Content/getModelList.html` with `mid=<menu>&cid=<subcategory>`:
  `mid=171` digital multimeters, `172` clamp meters, `170` electrical testers,
  `179` accessories.
- Each handheld product page embeds a comparison table with a 数据传输 (data
  transfer) column. That column, not the download centre, is what says whether
  a model has an interface at all.
- Handheld search results carry a trailing space inside the `href`; strip it or
  the fetch 404s.

The bench site uses 编程手册 (programming manual) in titles where the handheld
site uses 协议 — searching 通讯 or 指令 there returns nothing.

| Model | Page | Protocol document | PC software |
|-------|------|-------------------|-------------|
| UT61E+ / D+ / B+ | content/1301, 1300, 1299 | "UT61+系列通讯协议" deck, E+ page only (read) | one per model, 2023-02-03 (E+ = V2.02) |
| UT171A/B/C | content/1258-1260 | — | one shared, 2023-02-03 |
| UT181A | content/1261 | — | 2023-02-03 |
| UT202S / UT202BT | content/1340, 1341 | the UT61+ deck carries its range table | — (iDMM2.0 app) |
| UT216XD | not listed | named in the UT61+ deck; no range table published | — |
| UT60BT | content/1298 | — | — (iDMM2.0 app) |
| UT612 | content/1232 | — | 2021-11-23 |
| UT71A–E | content/4534 | "UT71系列接口协议" (download centre), a `.rar` holding the UT71 and the Voltcraft VC920/940/960 sheets — **read 2026-09-21** | 2026-05-15 |
| UT81A/B (classic), UT81A+–D+ | content/1276-1279 (the `+` models) | "UT81系列接口协议" (download centre), a `.rar` holding `ut81b通讯协议.doc`, the classic UT81A/B — **read 2026-09-21** | 2024-06-13 |
| UT61B/C/D/E (classic) | no live page | "UT61E接口协议" is the Cyrustek ES51922 datasheet; "UT61B通信协议" is the Fortune FS9922-DMM3 datasheet | 2021-11-02 |
| UT620A / UT620B | content/1133, 1134 | "UT620B通信接口参数" (.docx) | 2023-08-14 |
| UT632 / UT632N | not listed | none of its own; the shared bench manual below names it as USB HID `1a86:e008` | — |
| UT800 series (UT804) | cate/140 | "UT804接口协议" V1.0, 2023-11-15 (read) | UT804 V2.0 |
| UT803 | cate/138 | "UT803编程手册" REV.2: the UCI manual (below) | REV.2 |
| UT804+ | cate/143 | — | REV.2, 2021-01-28 |
| UT8802N | cate/145 | "UT8802N编程手册" REV.2: the UCI manual (below) | V2.0 |
| UT8803N | cate/146 | "UT8803N编程手册" REV.2: the UCI manual (below) | V2.0 |
| UT8805 / UT8805A / UT8806 / UT8806A | bench download centre | one SCPI programming manual each; UT8806's read 2026-09-21 | per model |

The UT61+ and UT171/UT181A pages also carry the iDMM2.0 Android app. Not found
on either site: UT161 (a calibration certificate only — its product pages live
on the international site, which is too slow to browse and was deliberately not
fetched), UT216XD, UT632.

**What the 2026-09-21 sweep proved absent.** No protocol or programming
document exists on either Chinese site for the UT171 series, UT181A, the UT161
series, UT139, UT89x, UT19x, UT21x or the UT2xx clamp meters — those have PC
software or a phone app only. The UT8802 and UT8803 we support have no document
of their own either: they appear only in the shared bench manual below. UNI-T
names **no USB bridge chip anywhere on either site**: every chip attribution in
this document comes from sigrok and is marked as such.

Outside the DMM and clamp scope, UNI-T does publish wire protocols for other
instrument classes. Categories are UNI-T's own, from each model's Chinese page:

| Model | Category (UNI-T's own) | Protocol |
|-------|------------------------|----------|
| [UT620A / UT620B](https://meters.uni-trend.com.cn/static/upload/file/20211123/UT620B%E9%80%9A%E4%BF%A1%E6%8E%A5%E5%8F%A3%E5%8F%82%E6%95%B0.docx) | 直流/回路电阻测试仪 — DC and loop resistance tester | 19200, 23-byte `"ABCD"` frame with a 2-byte sum (its "Not Yet Investigated" row above) |
| [UT272+ / UT273+ / UT275+](https://meters.uni-trend.com.cn/static/upload/file/20220905/1662367458670606.pdf) | 接地电阻测试仪 — earth resistance tester | `0xFF 0xAA`, address, length, command, CRC16 |
| [UT278R](https://meters.uni-trend.com.cn/static/upload/file/20240626/1719386046435392.doc) | 接地电阻在线检测仪 — online earth resistance monitor | Modbus-RTU over RS485 |
| [UT315A](https://meters.uni-trend.com.cn/static/upload/file/20221208/1670485039527450.pdf) | 测振仪 — vibration meter | `AB CD` header, 16-bit LE sum, over a CH340N USB-serial bridge |
| [UT362](https://meters.uni-trend.com.cn/static/upload/file/20211115/UT362%E6%8E%A5%E5%8F%A3%E5%8D%8F%E8%AE%AE.pdf) | 风速计 — anemometer | 9600 8N1, LCD-segment dump |
| [UT372](https://meters.uni-trend.com.cn/static/upload/file/20211115/UT372%E6%8E%A5%E5%8F%A3%E5%8D%8F%E8%AE%AE.pdf) | 转速计 — tachometer | 2400 8N1, LCD-segment dump |
| [UT382](https://meters.uni-trend.com.cn/static/upload/file/20211116/UT382%E6%8E%A5%E5%8F%A3%E5%8D%8F%E8%AE%AE.pdf) | 照度计 — light meter | 19200 8N1, LCD-segment dump |
| [UT714](https://meters.uni-trend.com.cn/static/upload/file/20221117/1668653073693503.pdf) / [UT715](https://meters.uni-trend.com.cn/static/upload/file/20221117/1668653010631549.pdf) | 多功能温度校准仪 — multifunction temperature calibrator | SCPI-style command set |
| [UT725](https://meters.uni-trend.com.cn/static/upload/file/20221117/1668653117352124.pdf) | 多功能过程校准仪 — multifunction process calibrator | SCPI-style command set |
| [UT3550](https://meters.uni-trend.com.cn/static/upload/file/20220928/1664350861547605.pdf) | 电池分析仪 — battery analyser | SCPI |
| UT35xx bench line | DC-resistance, multi-channel temperature and data-logging instruments | SCPI, one manual per series in the bench download centre |
| UT53xx bench line | Insulation, withstand-voltage and earth-impedance safety testers | SCPI, same |
| UT8630N / UT8635 | 交流毫伏表 — AC millivoltmeter | SCPI, same |

The UT8805 and UT8806 bench **multimeters** are in scope and have a section of
their own below.

The three segment-dump protocols are the same shape as each other — a function
number followed by nibble-encoded LCD segments — and differ only in baud rate.

The "programming manuals" on the UT803, UT803+, UT804, UT8802N and UT8803N
pages are one file, byte-identical to the Chinese UCI SDK manual V1.1 that
the UCI bench spec was built from (checked 2026-09-19). The same file is also
filed under UT802+, UT805A and UT8804N, and its device table is what names the
UT632 and the UT805A's serial settings. The bench download centre also has a
general-purpose "优利德上位机软件" (UNI-T PC software), 2025-05-26 and
2026-09-09, and UNI-T SDK V2.3 (152 MB) — neither opened.

Two bench listings are mislabelled: "UT805A+编程手册 REV.3" serves the UT8805
SCPI manual, and a 2026-04-25 "UT3560+编程手册" listing serves a UT3300+
datasheet.

### UNI-T US site

Surveyed 2026-09-19. uni-trendus.com lists its catalog at `/products.json`.
For the meters we support it carries the UT8802E/UT8803E programming manual
and UNI-T SDK V2.3, both byte-identical to the copies the UCI bench spec
used, and guides to the UT171 and UT181A PC apps; nothing for the UT61+ or
UT161. The global uni-trend.com is slow and has had nothing the Chinese or
US sites lack.

### Conrad (Voltcraft)

Surveyed 2026-09-19. Conrad's shop pages refuse scripts (HTTP 403), but its
file server serves each item's downloads at
`https://asset.conrad.com/media10/add/160267/c1/-/gl/000<item><CODE>`, where
the code is ML (manual), DS (datasheet), DL (software) or IN (information)
and a two-digit slot. The IN files hold protocol documents:

| Model | Item | Protocol document |
|-------|------|-------------------|
| VC-880 | 124609 | IN01: "VC880 Protocol Rev 2.4", 9 pages |
| VC650BT | 124411 | IN01: the same VC880 Protocol Rev 2.4 |
| VC-890 | 124600 | IN01: "VC890 Protocol Rev 1.3", 13 pages |
| VC-870 | 124603 | IN01: a VC870 protocol (not fetched) |

Voltsoft's own downloads and voltcraft.com carry no protocol document.

### EEVBlog forum threads
- "If Brymen BM869s is cheaper and as good, why people would still buy Fluke?" — 17+ pages
- "General Purpose Multimeter Recommendations 2024 (with logging)" — UT60BT #1 recommendation
- "EEVBlog 121GW Issues" — 292 pages
- "FlukeView Forms alternative" — active demand thread
- "OWON XDM1041 the unknown multimeter" — 8+ pages
