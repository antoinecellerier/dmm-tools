# Reverse Engineering Approach: UT61+/UT161 Protocol Family

## Objective

Determine whether the UT61B+, UT61D+, UT161B, UT161D, and UT161E share
the same USB communication protocol as the UT61E+ (already
reverse-engineered). Use only official, publicly available sources.

## Key Finding

**All six models use the identical wire protocol.** The same vendor
software binary (`Software V2.02`) serves all models with zero
model-specific protocol logic. The only differences are at the
application layer: display count, available measurement modes, range
tables, and bar graph segment count.

## Sources Used

1. **UT61+ Series User Manual** (UNI-T, P/N: 110401109614X, no revision
   printed) — single manual covering UT61B+, UT61D+, and UT61E+. The spec
   data comes from its "IX. Specifications" (PDF pp. 14-18, printed
   25-34), cross-checked against the UT61+/UT161 series datasheet (one
   page for both series, no revision printed) and the UT61+ series product
   page on meters.uni-trend.com (read 2026-09-19)
2. **UT161 Series User Manual** (UNI-T, P/N: 110401109612X) — single
   manual covering UT161B, UT161D, and UT161E. Compared page by page with
   the UT61+ manual: its spec pages differ only in the fuses of the
   current ranges (PDF p. 17)
3. **UNI-T UT61E+ Software V2.02** — previously decompiled for the
   UT61E+ analysis (see `docs/research/ut61eplus/`)
4. **UT161E Software** — downloaded and binary-compared against V2.02
5. **CP2110 Datasheet** and **AN434** (Silicon Labs) — transport layer
6. **UNI-T protocol deck "UT61+系列通讯协议"** — a 9-slide `.pptx` titled
   "UT161系列 UT61+系列 UT202S 蓝牙通讯协定" (Bluetooth protocol for the
   UT161 series, UT61+ series and UT202S), listed on the UT61E+ product page
   of UNI-T's Chinese site (https://meters.uni-trend.com.cn/content/1301.html)
   and uploaded 2023-08-16. The page links it on `admin-meters.uni-trend.com.cn`,
   which fails; the same path on `meters.uni-trend.com.cn` works. Read
   2026-09-18 from the rendered slides. The only protocol description UNI-T
   publishes for the family: line settings, framing, command table, mode
   table, a range table per model and the status bits. Its body also covers
   the **UT216XD** clamp meter, which the title leaves out: slides 4 and 8 note
   that it has no bargraph, so `Msg[12]-Msg[13]` can be ignored, and give it no
   range table of its own. Tagged [VENDOR-DOC]
7. **UT61E+ PC software on the same page** (uploaded 2023-02-03) — its
   installer is Software V2.02 unchanged (below)
8. **iDMM2.0 Android app v1.2.356** (`references/idmm2/`), UNI-T's own
   Bluetooth client for the family, decompiled with jadx 1.5.6 to
   `references/idmm2/jadx-out/` and read 2026-09-22: the source for command
   **0x5D** (`AB CD 03 5D 01 D8`), which the app sends once to start reading
   and which is in neither the deck nor V2.02. Tagged [VENDOR]. Read again
   2026-09-25 for the models it drives natively (working note
   `references/idmm2/analysis/findings/protocol-groups.md`): the **UT60BT**
   and **UT202BT** go through the same parser (`TestDataModel.anylseData`),
   19-byte frame and 0x5F-then-0x5D handshake as the UT61+ and UT161 behind a
   UT-D07B, over the same ISSC service (`BleManager`), and differ only in
   their range tables, the APK assets `funOl1_UT60BT.json` and
   `funOl1_UT202BT.json`, which share the UT61+ assets' schema

No community implementations, forum posts, or third-party reverse
engineering work during the primary RE. Two community implementations were
compared afterwards, for validation (table first committed 2026-03-19;
approval not recorded) — see
[Cross-Reference with Community Sources](#cross-reference-with-community-sources).

## Evidence: Single Shared Protocol

### Vendor software analysis — [VENDOR]

All four decompiled binaries from Software V2.02 were searched for
model-specific protocol logic:

- `CustomDmm_decompiled.txt` (13,115 lines) — protocol plugin
- `DMM_decompiled.txt` (48,396 lines) — main application
- `CP2110_decompiled.txt` (3,179 lines) — transport plugin
- `DeviceSelector_decompiled.txt` (9,850 lines) — device discovery

**Findings:**

1. **No model conditionals in protocol code.** Frame builder
   (`FUN_10002460`), frame parser (`FUN_10002540`), response parser
   (`FUN_10007d50`), mode/range lookup (`FUN_100023f0`), and command
   construction all have zero model parameters or model-name checks.

2. **Single flat mode/range table.** The table builder (`FUN_100027e0`
   in CustomDmm.dll, `FUN_00413f30` in DMM.exe) constructs one table
   containing ALL mode/range entries for ALL models. No model
   filtering — the meter firmware determines which modes are sent.

3. **Model name is cosmetic.** The string `"UT61B+"` is hardcoded as
   a default in the constructor. `options.xml` overrides it (e.g.,
   `<Model>UT61D+</Model>`). The value affects only the window title
   and export headers.

4. **One UI-only model check** (DMM.exe lines 3625-3639): two menu
   actions are hidden when the model name contains `"B"` (for
   UT61B+). This is purely cosmetic — likely hides Peak buttons.
   No protocol effect.

### UT161 binary comparison — [VENDOR]

The UT161E installer (43,129,397 bytes) was downloaded and extracted:

- **67 of 69 files are byte-for-byte identical** to the UT61E+ V2.02
  installer, including all protocol-critical binaries:
  `CustomDmm.dll`, `CP2110.dll`, `DeviceSelector.dll`,
  `SLABHIDtoUART.dll`
- **`DMM.exe`**: 8 bytes differ — only the model name string
  (`"UT161E"` vs `"UT61E+"`)
- **`options.xml`**: Only the `<Model>` tag differs
- **`uninst.exe`**: NSIS build nonces only

All three UT161 variants (B, D, E) serve the same zip from
meters.uni-trend.com. The model customization is purely cosmetic.

Note: The UT61E+ installer's `options.xml` says `<Model>UT61D+</Model>`
(not UT61E+), confirming UNI-T treats the model string as a UI label
with no protocol significance.

### UT61E+ 2023-02-03 package — [VENDOR]

The `.rar` on the UT61E+ CN page holds `Setup.exe` plus the software
manual as loose PDFs. Its installer unpacks to the same 69 files as the
V2.02 one, every one byte-identical (sha256, 2026-09-19): a repackaging,
not a new build.

### LoZ mode disambiguation — [VENDOR]

The vendor software mode table has two entries both labeled "LozV":
mode 0x15 and mode 0x16. Code analysis found they are treated
differently:

- **Mode 0x16**: SI prefix multiplication applied (like Ohm/Cap/Hz
  modes). In CustomDmm.dll line 1519: `cVar1 == '\x16'` is in the
  multiplier group.
- **Mode 0x15**: No SI prefix multiplication (like voltage modes).
  Not in the multiplier group.
- Both show numeric bar graph (neither in the "bar graph = dash"
  group).
- Both share the same "LozV" display name string.

Which byte the UT61D+ sends for its single LoZ dial position requires
device testing.

**Settled by the protocol deck** — [VENDOR-DOC]: 0x15 is LoZ (low-impedance
AC voltage) and 0x16 is a clamp meter's AC A, which fits the SI prefix
the software applies to 0x16 alone.

### Mode 0x17 (LPF) behavior — [VENDOR]

Mode 0x17 appears in the "bar graph = dash" group (line 1600:
`cVar1 == '\x17'`) but not in the SI multiplier group. This means LPF
mode displays "-" for bar graph and uses raw display values — consistent
with a voltage measurement mode. Mode 0x18 has no special handling
anywhere in the code.

The protocol deck makes 0x17 a clamp meter's DC A and puts LPF at 0x18,
the byte the UT61E+ sends for LPF V — see the UT61E+ spec §2.5.

## Commands Reference

All analysis commands are documented in
`docs/research/ut61eplus/reverse-engineering-approach.md`. No new
decompilation was needed for the other models.

The UT161 binary comparison used:

```sh
# Extract and compare
7z x -o"references/ut161/extracted-nsis" \
    "references/ut161/UT161E-Software.zip"
7z x -o"references/ut161/extracted-nsis" \
    "references/ut161/extracted-nsis/Setup.exe"

# MD5 comparison of all files
find references/ut61eplus/vendor-software/extracted/ -type f \
    -exec md5sum {} \; | sort > /tmp/ut61e_hashes.txt
find references/ut161/extracted-nsis/ -type f \
    -exec md5sum {} \; | sort > /tmp/ut161e_hashes.txt
```

## File Inventory

| Source | File | What it provides |
|--------|------|-----------------|
| UNI-T | `references/ut61eplus/ut61e_manual.pdf` | UT61+ Series manual (all 3 models); the spec data's source |
| UNI-T | `references/ut61b-plus/ut61b_manual.pdf`, `references/ut61d-plus/ut61d_manual.pdf` | The same file |
| UNI-T | `references/ut61eplus/ut61plus-datasheet.pdf` | UT61+/UT161 series datasheet |
| UNI-T | `references/ut161/UT161E-Software.zip` | UT161E installer (confirmed identical) |
| UNI-T | `references/ut161/UT161-UserManual.pdf` | UT161 Series manual |
| UNI-T | `references/ut61eplus/vendor-software/extracted/` | Software V2.02 (shared) |
| UNI-T | `references/ut61eplus/ut61plus-protocol-1692155466800163.pptx` | Protocol deck (UT161/UT61+/UT202S) |
| UNI-T | `references/ut61eplus/pc-software-2023-02-03/` | 2023-02-03 package (V2.02 repackaged) |
| Analysis | `references/ut61eplus/vendor-software/CustomDmm_decompiled.txt` | Protocol plugin |
| Analysis | `references/ut61eplus/vendor-software/DMM_decompiled.txt` | Main application |

## Cross-Reference with Community Sources

Consulted after the vendor analysis above, for validation only.

| Finding | Our RE | [ljakob](https://github.com/ljakob/unit_ut61eplus) | [mwuertinger](https://github.com/mwuertinger/ut61ep) | Agreement |
|---------|--------|--------|------------|:---------:|
| Same protocol for B+/D+/E+ | Yes (vendor code) | Yes (per-model tables, same framing) | N/A (E+ only) | ✓ |
| Same protocol for UT161 series | Yes (binary-identical software) | Yes (explicit UT161 support) | N/A | ✓ |
| UT60BT over Bluetooth | Not investigated | Yes (BT serial support) | N/A | — |
| 6000-count range tables | From manual | Per-model tables in code | N/A | To verify |
| Mode byte values 0x00-0x14 | Vendor software table | Same values | Same values | ✓ |
| LoZ mode 0x15/0x16 | 0x15 (protocol deck; 0x16 is a clamp's AC A) | Uses 0x15 only | N/A | ✓ |

**Former discrepancy**: ljakob's implementation uses only mode 0x15 for
LoZ, while the vendor software has entries for both 0x15 and 0x16 with
different display value handling. The protocol deck sides with 0x15.

Reference implementations:

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) — Python, UT61E+ and UT61B+/D+ tables, UT161 and UT60BT support
- [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — Go, UT61E+

### 2026-09-22: Bluetooth read rate and command 0x5D

The boundary was opened again on **2026-09-22**, for one question: how fast
the family can be read over the UT-D07B adapter, and whether command **0x5D**
(absent from the deck and from Software V2.02) starts a continuous send. The
findings are in `../ut61eplus/reverse-engineered-protocol.md` §7 and, for the
adapter itself, `../ut-d07b/reverse-engineered-protocol.md` §7; the working
note is `references/ut-d07b/analysis/findings/read-rate.md` §8.

Read that day, all marked [COMMUNITY]:

- [webspiderteam/Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows),
  commit `2b83d9e` — C# BLE client; names `AB CD 03 5D 01 D8` "Get Data" and
  sends it once per connection
- [libreble/multimeter](https://github.com/libreble/multimeter), commit
  `e887b0f` — Web-Bluetooth PWA; names 0x5D "start streaming measurements"
  and reports the stream live-confirmed on a UT60BT, plus the GET-NAME →
  name-frame → GET-DATA ordering the meter requires
- [olegv142/ut61xpy](https://github.com/olegv142/ut61xpy), commit `ad7b324` —
  Python logger for UT61X+ over the UT-D09A **and the UT-D07B**; polls 0x5E
  over both and publishes ~180 ms (USB) / ~800 ms (Bluetooth) minimum read
  intervals
- [mbraune/ut161b](https://github.com/mbraune/ut161b) — UT161B over the D09A
  cable; polls 0x5E, documents the same framing
- [libsigrok](https://github.com/sigrokproject/libsigrok) master `0bc2487778`
  — for the BLE serial layer only; it has **no** driver for this family
- EEVblog ["New Uni-T UT61 series (UT61e+)"](https://www.eevblog.com/forum/testgear/new-uni-t-ut61-series-(ut61e)/),
  all 7 pages — nothing about the protocol

Deliberately not read: `ljakob/unit_ut61eplus`'s `from_vendor/` directory
(vendor files re-hosted under a licence its README disclaims; we have our own
jadx tree of the same APK), community decompilations of the Android app, and
APK mirrors.

**Correction to the table above (2026-09-22).** The "UT60BT over Bluetooth"
row credits ljakob with BT serial support. It has none: `ut61eplus.py` opens
`hid.device()` on `0x10C4:0xEA80` and speaks CP2110 HID only. The UT60BT
appears in its README as a model whose unit tables would need adjusting, not
as a transport.

### 2026-09-25: UT60BT and UT202BT

Opened again after the iDMM2.0 read of the two native-BLE models (source 8),
to check it before tables are written. Working note:
`references/idmm2/analysis/findings/community-ut60bt-ut202bt.md`. All
[COMMUNITY]:

- [libreble/multimeter](https://github.com/libreble/multimeter) `d26ba48`
  (protocol files unchanged since `e887b0f`) — the only source with UT202BT
  code, marked ported-unverified, with no captures
- [webspiderteam/Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows) `2b83d9e`
- [QtDMM](https://github.com/qtdmm/QtDMM) `ad2f785` — new; UT60BT support
  from 2026-09-24 with a hardware dial walk
- [olegv142/ut61xpy](https://github.com/olegv142/ut61xpy) `3384a9f` — its
  UT60BT Bluetooth adapter

| Finding | iDMM2.0 | Community | Agreement |
|---------|---------|-----------|:---------:|
| Service and characteristics | ISSC, notify `1e4d`, write `8841` | Same in all four; a live UT60BT has no `ff`/`fff0` service | ✓ |
| Advertised name | `UT60BT`, `UT202BT` | One UT60BT advertises `UT60BTk` and answers Get Name with `UT60BT` | new: match a prefix |
| Handshake | 0x5F, then 0x5D, stream | Same bytes; the meter ignores 0x5D until it has answered 0x5F | ✓, ordering new |
| 0x5E poll | never sent by the app | QtDMM and ut61xpy poll it on a UT60BT, one frame per request | the meter answers it |
| 19-byte layout and flags | as the UT61+ | Same; checksums valid on real frames | ✓ |
| Secondary display (byte 3 bit 7) | any model | Masked by all; always 0 on a UT60BT | not covered |
| UT60BT range table | `funOl1_UT60BT.json` | QtDMM matches it, and its dial walk confirms V r0 = mV and Ω r4 = MΩ; libreble's generic table disagrees on V r0, Hz and capacitance r2/r5, mA r1 | ✓ where measured |
| UT202BT range table | `funOl1_UT202BT.json` | No source has one; libreble's generic codes give INRUSH and LPFA in V where the asset says A | not covered |
| Buttons | UT60BT 0x46-0x48, 0x4A, 0x4C; UT202BT 0x31-0x37 | Agree; libreble adds 0x41, 0x49, 0x4B for the UT60BT on a blanket claim | partly |

**Boundary note.** The subagent that did this read also opened
`ljakob/unit_ut61eplus`'s `from_vendor/funOl_UT60BT.json`, which the
2026-09-22 note above excludes. It was compared only; nothing was taken from
it. It is an older copy of the same vendor asset — same units, different
bounds. QtDMM's tables derive from that file.
