# New Device Candidates

Research into multimeters worth supporting (April 2026; UNI-T's own catalogue
swept model by model 2026-09-21). Covers USB HID, Bluetooth LE, USB-serial, and
IR-optical connections. A meter leaves this file once supported: its research
is under `docs/research/<family>/`, its entry in
[supported devices](../supported-devices.md).

## Landscape Overview

Modern multimeters with PC connectivity use one of these transports:

| Transport | Examples | Sigrok coverage |
|-----------|----------|-----------------|
| **USB HID** (current dmm-tools) | UNI-T (CP2110/CH9329/CH9325), Brymen (BU-86X), Victor | Good on all platforms |
| **Bluetooth LE** (current dmm-tools) | 121GW, OWON B35T/B41T+, UNI-T UT-D07B, Aneng/BSIDE/ZOYI | **Linux only, experimental, flaky** |
| **USB serial (CDC)** | OWON XDM (CH340), Fluke IR (FTDI), APPA (CP2102) | Good on all platforms |
| **USB TMC/SCPI** | Rigol, Siglent bench instruments | Good, well-served by pyvisa/lxi-tools |

**Key gap:** sigrok has **no BLE support on Windows or macOS** — only a
Linux-only BlueZ backend that's described as "slow and occasionally
flaky." No cross-platform desktop tool provides reliable BLE multimeter
connectivity.

---

## USB HID Candidates

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

### UNI-T Native-BLE Meters and the UT-D07A

The UT-D07B adapter and the meters it carries are supported, as are the
UT60BT and UT202BT ([supported devices](../supported-devices.md),
[research/ut-d07b](ut-d07b/reverse-engineered-protocol.md),
[research/ut61-family](ut61-family/reverse-engineered-protocol.md)). What
follows is the rest of UNI-T's Bluetooth range. The native-BLE paragraphs
were rewritten 2026-09-25 from the iDMM2.0 app.

**The UT-D07A** lists `UT513B, UT513C, UT513D, UT512E` on its
[accessory page](https://meters.uni-trend.com.cn/content/4374.html); the
adapter itself is still open ([its checks](ut-d07b/verification.md#the-ut-d07a)).

**Two clamp meters speak the UT61+ frame.** UNI-T's UT61+ protocol deck
([research/ut61-family](ut61-family/reverse-engineering-approach.md)) specifies
the **UT202S** with a full range table, and the **UT216XD** as speaking the
same frame as the UT61+ — read from the archived slides 2026-09-21, the only
stated difference being that the UT216XD has no bargraph, so `Msg[12]–Msg[13]`
can be ignored. The deck's function table already carries the clamp modes
(`0x16` clamp ACA, `0x17` clamp DCA, `0x1C` clamp LPF, `0x1D` clamp AC+DC).
`[UNVERIFIED]`: the deck gives range tables for the UT61B+/D+, UT61E+ and
UT202S but **none for the UT216XD**, and states no transport for either clamp.

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

- **The UT61+ parser: UT60BT and UT202BT.** Supported
  ([research/ut61-family](ut61-family/reverse-engineered-protocol.md)).
- **The UT171 and UT181A**, over the UT-D07A/B. Supported.
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

---

### OWON: the older B35/B35T, and the 15-byte meters

OWON's B33, B35T+, B41T+, OW16B, OW18B, OW18E and CM2100B are supported
(`owon`, [research/owon](owon/reverse-engineered-protocol.md)). What OWON's
sources describe beyond them:

- **Next: the older B35 and B35T, on the FS9922 14-byte ASCII frame.** OWON's
  PC source keeps a fully commented-out parser for 14-byte frames, and notes
  that the B35 without offline record "still uses its chip protocol"
  ([research/owon §11](owon/reverse-engineered-protocol.md#11-an-earlier-format-14-byte-ascii-pc-source-commented-out));
  the status-bit constants it uses are defined nowhere in that source.
  Community tools describe FS9922 B35T units sending those frames on FFF4
  in the same service (spec §14.4) `[COMMUNITY]`. When taken up, OWON's
  commented-out parser is the source, before any community cross-reference.
- **OWON's 15-byte frame.** OWON's iMeter app decodes a second, 15-byte frame
  with a sub-display for the Voltcraft VC831, VC851, VC871, VC891, VC915 and
  VC925, OWON's OW65, OW67 and OW69, and the CMS061/CMS101 clamp-scopes; it
  is specified from the app alone
  ([research/owon §10](owon/reverse-engineered-protocol.md#10-the-15-byte-frame-owons-app-only)).
  These Voltcraft meters are not the VC880/VC890 line already supported.
  OWON's OW65, OW67 and OW69 manuals give Bluetooth to their B models.
  **In progress (2026-10-02), every app code in scope.** Demand, seen
  2026-10-02, is about a tenth of the 6-byte group's: the CMS101 has five
  YouTube reviews of 6.9k-26.4k views, an EEVblog thread, about 108
  AliExpress sales and about 15 amazon.de ratings; the VC871 and VC891 have
  6 and 4 amazon.de ratings and Voltcraft's own app 1k+ downloads; the VC915
  and VC925 PV (2025) none yet; the OW65B, OW67B and OW69B are sold only by
  regional distributors. All are current (2022-2025). No Bluetooth was found
  for the VC831 or VC851: Voltcraft's app lists only the VC871, VC891, VC915
  and VC925 PV, and the VC851 manual never mentions it.

Out of scope: the CMS061/CMS101's scope features; the B35's "Bluetooth 2.0"
version, which OWON lists as Android-only and which is presumably classic
Bluetooth `[UNVERIFIED]`.

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
| Connection | USB serial via CH340, SCPI protocol, 115200 baud; the XDM2041 adds RS232, and a community tool reaches the XDM3041/XDM3051 over TCP too |
| Protocol | SCPI, documented in OWON's XDM1000 series programming manual |

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

- dmm-lib's open path is hidapi-only (`open_transport` in
  `crates/dmm-lib/src/lib.rs`, `KNOWN_TRANSPORTS` in
  `crates/dmm-lib/src/transport/open.rs`) and auto-detection probes over HID
  bridges. Any of these meters needs an address-based open (host or device
  path) and identification by `*IDN?`. The `Protocol` trait is poll-based
  (`request_measurement`) and fits SCPI query/response as it is; the
  `Transport` trait (`crates/dmm-lib/src/transport/mod.rs`) is a byte
  stream whose one link setting, `set_baud`, is optional, so a network
  transport fits it but for `link()`: `Link` has only `UsbCable` and
  `Bluetooth`, so it needs a network variant. The SCPI text replies
  need a new protocol family; no existing frame parser applies.
- Transports, cheapest first: (1) **VXI-11 over `std::net`** — portmapper
  plus core channel, a few hundred lines of XDR, std only, cross-platform,
  reaches every model, must tolerate the UT8805N/E's unpadded replies;
  (2) **raw socket 5025 over `std::net`** — trivial, UT8805A/UT8806 line
  only. Both LAN options need a host entry in the GUI and its settings (the
  N line announces nothing to discover) and a network `Link` variant
  through every site that matches on `Link`; (3) **Linux `/dev/usbtmcN`
  through `std::fs`** — the kernel adds the headers; Linux only, plus a
  udev rule. The driver opens a file with auto-abort off and a 5 s timeout,
  so after a timed-out read the late reply stays queued on the meter and
  the next read fails on its bTag, unless the transport sets the USBTMC
  ioctls (`unsafe`) or treats a timeout as a reconnect; (4) **USBTMC over
  libusb/nusb** on macOS and Windows — the USBTMC headers, bTag and abort
  are then ours to write, it is a new dmm-lib dependency against the
  "hidapi, thiserror, log only" rule in `.claude/rules/protocol.md`, and
  Windows needs WinUSB (Zadig) or a vendor VISA driver, which then owns the
  meter; (5) **RS-232** — the serial transport the UT805A and OWON XDM
  would also use, with its own dependency decision.
- No maintained VISA-free Rust library carries any of these links (crates.io
  and GitHub, 2026-09-30): `visa-rs` links an installed vendor VISA at
  build time; `rust-usbtmc` (on `rusb`, last release 2021) and `lxi` (2019)
  are unmaintained; `instrument-core` 0.1.0 defines a transport trait and
  implements none; Atmelfan's `lxi-rs` is the instrument side.
- Work every link needs: one reading takes three or four replies (`CONF?`,
  `RANG:AUTO?`, `UNIT:TEMP?`, `READ?`), so `raw_payload` has to carry them
  all for replay and golden files; readings need a unit prefix and
  `display_raw` per range; if queries lock the keys, capture's hand steps
  have to set the function with `FUNC` instead; returning the meter to local
  needs a session-end hook the `Protocol` trait lacks, and it would still
  miss detection-only opens and a failed init; and the open path,
  `check_cable`, the `NoTransportFound` text, the `LinkLost` wording and
  the connection help assume a HID cable or Bluetooth.
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
owns one; the open hardware questions are in its
[verification list](ut8805/verification.md).

**Costed 2026-09-30 and parked**: an implementation plan (Linux `usbtmc`,
`nusb` elsewhere) came out larger than the work above suggests, mostly in
the work every link needs. The smallest first cuts are read-only: Linux
`usbtmc` (every model, Linux only), or VXI-11 (every model and OS, with the
host entry).

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
| **72-7730 / 72-7732** | Tenma | Handheld DMM | USB HID (`1A86:E008`) | UNI-T UT71 rebrands, CH9325 / HE2325U (UT-D04), per sigrok only. The supported UT71 decoder covers them ([research/ut71](ut71/reverse-engineered-protocol.md)); a Tenma would be named as a UT71 |
| **UT804+** | UNI-T | Bench DMM (59999 counts per its Chinese product page) | USB (HID per UNI-T's download listing, unverified) | A newer model than the supported UT804 (40000 counts). A "UT804" [programming manual](https://instruments.uni-trend.com.cn/static/upload/file/20220920/UT804%E7%BC%96%E7%A8%8B%E6%89%8B%E5%86%8C%20REV.2.pdf) is the Chinese original of the UCI SDK manual (V1.1, 2019): it covers the UT804/UT804N and not the UT804+ ([research/uci-bench-family](uci-bench-family/reverse-engineered-protocol.md)). The [UT804+ page](https://instruments.uni-trend.com.cn/cate/143.html) lists software but no protocol document. The "UT804接口协议" on the [UT800 series page](https://instruments.uni-trend.com.cn/cate/140.html), read 2026-09-19, describes the UT804 alone (its first digit runs 0-4, a 40000-count display), so whether the UT804+ speaks a protocol we support is still open |
| **UT202S** | UNI-T | Clamp meter | Bluetooth, per UNI-T's protocol deck | Speaks the UT61+ protocol: the [deck](ut61-family/reverse-engineering-approach.md) gives its range table (V and A to 600, LPF, temperature) and says it sends a main and a secondary display in AC, LPF and temperature modes. Its [page](https://meters.uni-trend.com.cn/content/1340.html) offers only the UT202S/UT202BT manual. Its Ω ladder is not the supported UT202BT's ([research/ut61-family](ut61-family/reverse-engineered-protocol.md#ut202bt-9999-counts)). The Bluetooth transport carries it; it needs a registry entry and a capture |
| **UT117C, UT197 / UT197PV, UT219PV** | UNI-T | Not recorded | Bluetooth LE, built in (ISSC) | One polled `AB CD` frame with a 2-byte length and ASCII readings, a field layout per model (Bluetooth section above). The iDMM2.0 app is the only source |
| **UT805A / UT805N** | UNI-T | Bench DMM (220000 counts) | Serial | USB-to-serial (virtual COM port, not HID), ASCII text protocol (9600/8N1, bidirectional); see [research/ut8803](ut8803/reverse-engineering-approach.md). The shared bench programming manual's device table gives `[T:COM][PORT:8][BAUD:9600][PARITY:N][STOP:1][DATA:7]` and a CP210x driver |
| **UT216XD** | UNI-T | Clamp meter | Unstated — the deck that specifies it is a Bluetooth protocol | Speaks the UT61+ frame, bargraph bytes aside (Bluetooth section above). **No archived source carries its ranges**: the deck names it only in the two bargraph exceptions, the iDMM2.0 APK has no UT216 package or range asset (checked 2026-09-21), and it has no page in the Chinese catalogue. Its range table would have to come from hardware or from vendor software we do not have |
| **UT61B / UT61C / UT61D / UT61E** | UNI-T | Handheld DMM (classic, pre-`+`) | UT-D04 (CH9325) in practice | UNI-T's "protocol" downloads are the chipset datasheets: ["UT61E接口协议"](https://meters.uni-trend.com.cn/static/upload/file/20220908/1662605553430400.pdf) is the Cyrustek **ES51922** (19230 baud, 7-odd-1) and ["UT61B通信协议"](https://meters.uni-trend.com.cn/static/upload/file/20220110/UT61B%20protocol.pdf) is the Fortune **FS9922-DMM3**. Long discontinued; sigrok covers both chipsets |
| **UT81A+ / UT81B+ / UT81C+ / UT81D+** | UNI-T | Handheld scopemeter | Type-C, transport unstated (CDC, TMC or HID) | Their pages advertise "支持SCPI通信功能，可二次开发" (SCPI, open to development) but publish no command list. The only UT81 protocol document, ["UT81系列接口协议"](https://meters.uni-trend.com.cn/static/upload/file/20211102/ut81b%E9%80%9A%E8%AE%AF%E5%8D%8F%E8%AE%AE.rar) (read 2026-09-21, `references/ut81/`), covers the **older UT81A/B**: 9600 8N1, records framed by `0x5A` with a decimal-digit length and checksum, a 15-byte instrument-state block, two 10-byte ASCII readings and an optional 160-sample waveform block, with `0x5A` doubled where it occurs in the data. It names no cable or bridge chip |
| **UT620A / UT620B** | UNI-T | 直流/回路电阻测试仪 — DC and loop resistance tester | USB, "免安装驱动" (driver-free) and bidirectional — a HID hint, unconfirmed | Not a DMM, but the closest neighbour to our framing: ["UT620B通信接口参数"](https://meters.uni-trend.com.cn/static/upload/file/20211123/UT620B%E9%80%9A%E4%BF%A1%E6%8E%A5%E5%8F%A3%E5%8F%82%E6%95%B0.docx) gives 19200 and a 23-byte `"ABCD"`-headed frame with a 2-byte sum, plus host commands (0x30 keypress, 0x31 one-shot read, 0x32 memory dump). Cable UT-D18 |
| **BM2257, BM786, BM235, BM036** | Brymen (sold as EEVblog) | Handheld DMMs | Not recorded | Brymen meters under EEVblog's name on [EEVblog's store](https://eevblog.store/collections/multimeters), noted 2026-09-26; of its Brymen meters only the BM787BT (supported) was recorded with Bluetooth |
| **BM257** | Brymen | Handheld DMM | Serial (per a community logger) | Named only as the serial variant a community Python logger reads; not investigated |

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
| ANENG 681, 683 | None (USB-C charging only) | High-volume AliExpress sellers (3,000–10,000+ sold, 2026-10-01); no Bluetooth or data link found in retailer listings or manual pages, 2026-10-01 |
| FNIRSI DMC-100 | None (USB-C for firmware only) | FNIRSI's product page: the USB port is "only for firmware transmission"; no Bluetooth (2026-10-01) |
| FNIRSI 2C23T, 2C53T, DST-201/210 | USB mass storage | Scope/meter/generator combos; USB exports saved screenshots and waveforms and updates firmware, with no live readings; FNIRSI's software page lists nothing for them (2026-10-01) |

---

## Priority Summary

### Tier 1: Highest value

| Candidate | Transport | Why | Gap |
|-----------|-----------|-----|-----|
| **Fluke 287/289** | USB serial (IR) | Officially documented ASCII protocol, millions of units, $200 Windows-only software is terrible | Large |

### Tier 2: Worth considering

| Candidate | Transport | Why | Gap |
|-----------|-----------|-----|-----|
| **OWON B35/B35T (FS9922, before the B35T+)** | BLE (built in) | The supported OWON meters' service per community tools; OWON's PC source keeps a commented-out parser ([research/owon §11](owon/reverse-engineered-protocol.md#11-an-earlier-format-14-byte-ascii-pc-source-commented-out)) | Moderate: a 14-byte ASCII frame whose status bits OWON's source does not define |
| **Voltcraft VC831/851/871/891/915/925, OWON OW65/67/69, CMS061/101** | BLE (built in) | OWON's 15-byte frame, specified from OWON's iMeter app ([research/owon §10](owon/reverse-engineered-protocol.md#10-the-15-byte-frame-owons-app-only)); the same transport and family module as the B/OW meters | Moderate: a second frame layout with a sub-display, one vendor source |
| **Victor 70C/86C** | USB HID | Cheap, protocol documented, no good software | Moderate |
| **UNI-T UT632/UT632N** | USB HID (CH9325) | Bench DMM on a bridge we already drive; the UT803 app's UT632 configuration frames its stream on a high-nibble-E byte but decodes nothing, so the payload needs a capture and the `ut80x` parsing does not carry over | Unmeasured |
| **UNI-T UT117C, UT197/UT197PV, UT219PV** | BLE (built in) | Three models on one polled frame over the Bluetooth transport we have; vendor-sourced from the iDMM2.0 app | Moderate: a new protocol family with a field layout per model |
| **UNI-T UT8805/UT8806** | LAN (VXI-11, socket 5025); USB TMC; RS-232 | Specified ([research/ut8805](ut8805/reverse-engineered-protocol.md)); plain SCPI query/response that the poll-based `Protocol` trait already fits; a `std::net` VXI-11 transport reaches every model with no dependency change and opens a SCPI family for Rigol/Siglent maps; no cross-platform VISA-free GUI logger exists over LAN or USB (TestController covers RS-232) | Parked 2026-09-30 after costing (see its section): a new link (network or USBTMC), a SCPI protocol family, address-based open and `*IDN?` identification, and a session-end hook |

### Tier 3: Lower priority

| Candidate | Transport | Why excluded or deprioritized |
|-----------|-----------|-------------------------------|
| Mooshimeter | BLE | Discontinued, shrinking user base |
| OWON XDM series | USB serial SCPI | Already well-served by rusty_meter (100 stars, Rust/egui) |
| Pokit Pro | BLE | Already well-served by dokit (63 stars) |
| Rigol/Siglent bench | USB TMC/SCPI, LAN | Well-served by pyvisa/lxi-tools; separate SCPI dialects (Rigol DM3000 tree; Truevolt layout on DM858, Siglent SDM, Teledyne T3DMM). Would ride the UT8805/UT8806 SCPI family (Tier 2) as extra command maps over the same transport, not a project of its own |

### Strategic notes

- **BLE is where the open demand is.** The Bluetooth transport exists and
  reaches OWON's B, OW and CM2100B meters; OWON's older and 15-byte meters are
  the next step. sigrok's BLE is Linux-only and experimental; no
  competitor fills this space cross-platform.
- **rusty_meter** (100 stars, Rust/egui, OWON XDM) validates the exact
  tech stack dmm-tools uses. Proves community demand for native desktop
  multimeter apps.
- **Other brands' BLE meters need more than names and tables.** A meter
  whose GATT layout no profile we carry covers needs a profile of its own,
  and its protocol needs vendor sources: for OWON, its iMeter app and PC
  software (2026-10-01).
- **Bluetooth-DMM-For-Windows** (53 stars, last pushed 2026-05-01) proves
  demand for a multi-device BLE desktop app; it is Windows-only.
- **Adding serial transport** is smaller scope than BLE but the
  highest-value serial target (Fluke) overlaps more with existing tools.
- **No other UNI-T handheld family publishes a wire protocol**: besides the
  supported UT61+ and UT71, the 2026-09-21 sweep found none for the UT171,
  UT181A, UT161, UT139, UT89, UT19x, UT21x or the clamp line. UNI-T's newer bench multimeters
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
- [sigrok Victor protocol](https://sigrok.org/wiki/Victor_protocol)
- [sigrok Device cables](https://sigrok.org/wiki/Device_cables)
- [improwis.com Multimeter chips](http://improwis.com/projects/reveng_multimeters/)

### Bluetooth LE
- [DeanCording/owonb35](https://github.com/DeanCording/owonb35) — OWON B35 Linux client
- [sercona/Owon-Multimeters](https://github.com/sercona/Owon-Multimeters) — OWON multi-model support
- [Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows) — Windows BLE DMM app
- [pcolby/dokit](https://github.com/pcolby/dokit) — Pokit cross-platform tools
- [mooshim/Mooshimeter-PythonAPI](https://github.com/mooshim/Mooshimeter-PythonAPI)
- [sigrok Bluetooth support](https://sigrok.org/wiki/Bluetooth)

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
- "FlukeView Forms alternative" — active demand thread
- "OWON XDM1041 the unknown multimeter" — 8+ pages
