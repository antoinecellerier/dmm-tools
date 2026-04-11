# New Device Candidates

Research into multimeters worth supporting (April 2026). Covers USB HID,
Bluetooth LE, USB-serial, and IR-optical connections.

## Landscape Overview

Modern multimeters with PC connectivity use one of these transports:

| Transport | Examples | Sigrok coverage |
|-----------|----------|-----------------|
| **USB HID** (current dmm-tools) | UNI-T (CP2110/CH9329/CH9325), Brymen (Cypress), Victor | Good on all platforms |
| **Bluetooth LE** | 121GW, OWON B35T/B41T+, UNI-T UT-D07B, Aneng/BSIDE/ZOYI | **Linux only, experimental, flaky** |
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

### UNI-T Meters via UT-D07B BLE Adapter — HIGHEST STRATEGIC VALUE

| Aspect | Details |
|--------|---------|
| Adapter | UT-D07A / UT-D07B (~$30) |
| Connection | BLE 5.0 via ISSC BL79 BLETR chip |
| Compatible meters | UT61E+, UT61B+, UT61D+, UT161 series, UT171 series, UT181A |
| Protocol | **Transparent BLE-to-UART bridge** — same protocol as wired USB |

**Why this is the highest strategic value:** dmm-tools already parses
UT61E+/UT171/UT181A protocols. The UT-D07B is a transparent UART bridge
over BLE — adding BLE transport would unlock wireless operation with
**zero protocol changes**. All existing device tables and protocol
parsers work unmodified.

The UT60BT was the **#1 recommendation** in the 2024 EEVBlog "General
Purpose Multimeter Recommendations" thread for logging multimeters. The
official UNI-T "Smart Measure" and "iDMM2.0" apps work but are
phone-only — no desktop logging tool exists for BLE mode.

---

### EEVBlog 121GW

| Aspect | Details |
|--------|---------|
| Price | ~$221 (currently sold out on official store) |
| Connection | BLE 4.0 via BLE122 module (UART bridge) |
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

### Aneng / BSIDE / ZOYI BLE Meters (AN9002, ZT-300AB, ZT-5B, etc.)

| Aspect | Details |
|--------|---------|
| Price | $15-40 (very cheap) |
| Connection | BLE |
| Protocol | 10/11-byte packets, 7-segment LCD encoding, well reverse-engineered |
| BLE UUID | `0000fff4-0000-1000-8000-00805f9b34fb` |

All rebrands from the same manufacturer (ZOTEK/Zoyi), shared protocol.

**GitHub:** [ludwich66/Bluetooth-DMM](https://github.com/ludwich66/Bluetooth-DMM)
(43 stars) documents protocol variants.
[Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows)
(47 stars) is the main GUI tool but Windows-only and inactive.

**Gap: moderate.** No cross-platform desktop tool. But these are
extremely cheap meters — users may not invest in tooling.

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

### Rigol / Siglent Bench DMMs (USB TMC/SCPI)

Standard SCPI instruments, well-served by pyvisa, lxi-tools, sigrok, and
vendor software. **Not a priority target.**

---

## Meters Investigated and Ruled Out

| Brand/Model | Connection | Why excluded |
|-------------|-----------|-------------|
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
| **Brymen BM86x** | USB HID (Cypress) | Official protocol docs, strong community, no cross-platform GUI exists | Large |
| **UNI-T via UT-D07B** | BLE | Reuses existing protocol parsers, #1 recommended logging meter on EEVBlog 2024, no desktop BLE tool | Large |
| **Fluke 287/289** | USB serial (IR) | Officially documented ASCII protocol, millions of units, $200 Windows-only software is terrible | Large |

### Tier 2: Worth considering

| Candidate | Transport | Why | Gap |
|-----------|-----------|-----|-----|
| **EEVBlog 121GW** | BLE | Largest enthusiast community (292-page thread), fragmented software | Moderate |
| **OWON B35T+/B41T+** | BLE | Popular budget BLE meters, no cross-platform GUI, proprietary dongle required for PC | High |
| **Victor 70C/86C** | USB HID | Cheap, protocol documented, no good software | Moderate |

### Tier 3: Lower priority

| Candidate | Transport | Why excluded or deprioritized |
|-----------|-----------|-------------------------------|
| Aneng/BSIDE/ZOYI BLE | BLE | Very cheap meters, users may not invest in tooling |
| Mooshimeter | BLE | Discontinued, shrinking user base |
| OWON XDM series | USB serial SCPI | Already well-served by rusty_meter (100 stars, Rust/egui) |
| Pokit Pro | BLE | Already well-served by dokit (63 stars) |
| Rigol/Siglent bench | USB TMC/SCPI | Well-served by pyvisa/lxi-tools |

### Strategic notes

- **BLE transport is the biggest unlock.** It enables UNI-T UT-D07B
  (reuses existing parsers), 121GW, and OWON B35T/B41T+ — three of the
  most-demanded meters. sigrok's BLE is Linux-only and experimental; no
  competitor fills this space cross-platform.
- **rusty_meter** (100 stars, Rust/egui, OWON XDM) validates the exact
  tech stack dmm-tools uses. Proves community demand for native desktop
  multimeter apps.
- **Bluetooth-DMM-For-Windows** (47 stars, now abandoned) proves demand
  for a multi-device BLE desktop app. Its Windows-only nature and
  inactivity leave the gap wide open.
- **Adding serial transport** is smaller scope than BLE but the
  highest-value serial target (Fluke) overlaps more with existing tools.

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
- [ludwich66/Bluetooth-DMM](https://github.com/ludwich66/Bluetooth-DMM) — Aneng/BSIDE protocol docs
- [Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows) — Windows BLE DMM app
- [pcolby/dokit](https://github.com/pcolby/dokit) — Pokit cross-platform tools
- [mooshim/Mooshimeter-PythonAPI](https://github.com/mooshim/Mooshimeter-PythonAPI)
- [sigrok Bluetooth support](https://sigrok.org/wiki/Bluetooth)
- [AN9002 BLE protocol analysis](https://justanotherelectronicsblog.com/?p=930)

### USB serial / IR
- [markusdd/rusty_meter](https://github.com/markusdd/rusty_meter) — Rust/egui OWON XDM tool (100 stars)
- [TheHWcave/OWON-XDM1041](https://github.com/TheHWcave/OWON-XDM1041)
- [Fluke Remote Interface Specification](https://www.pewa.de/DATENBLATT/DBL_FL_FL187-9-89IV_BEFEHLSSATZ_ENGLISCH.PDF)
- [N0ury/dmm_util](https://github.com/N0ury/dmm_util) — Fluke 287/289 Python utility
- [FlukeView Forms alternative — EEVBlog thread](https://www.eevblog.com/forum/testgear/flukeview-forms-alternative/)
- [HKJ's Test Controller](https://lygte-info.dk/project/TestControllerIntro%20UK.html)

### EEVBlog forum threads
- "If Brymen BM869s is cheaper and as good, why people would still buy Fluke?" — 17+ pages
- "General Purpose Multimeter Recommendations 2024 (with logging)" — UT60BT #1 recommendation
- "EEVBlog 121GW Issues" — 292 pages
- "FlukeView Forms alternative" — active demand thread
- "OWON XDM1041 the unknown multimeter" — 8+ pages
