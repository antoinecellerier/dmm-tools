# Reverse Engineering Approach: UT171 Family Protocol

## Objective

Reconstruct the UT171A/B/C USB communication protocol using official,
publicly available sources:

1. **UT171A/B/C User Manual** (from UNI-T, `uni-t_ut171_manual.pdf`)
2. **UT171C PC Software** (from UNI-T, `UT171C setup.exe`)
3. **UT171C Software Installation Instruction** (from UNI-T)
4. **SLABHIDtoUART.dll** (from Silicon Labs, bundled with software)
5. **SLABHIDDevice.dll** (from Silicon Labs, bundled with software)

Community implementations are used only for cross-referencing after
the primary analysis is complete.

## What Each Source Provides

### UT171A/B/C User Manual

The manual defines all **application-layer semantics**:

- **Display counts**: UT171A: 40,000; UT171B/C: 60,000
- **Measurement modes**: DCV, ACV, DCA, ACA, AC+DC voltage/current,
  resistance, conductance (nS, UT171B/C only), capacitance, frequency,
  duty cycle, temperature (C/F), diode, continuity, LoZ ACV, VFC,
  % (4-20mA), NCV, square wave output (UT171C only), 600A clamp
  (UT171C only)
- **Ranges**: Complete per-mode range tables with accuracy specs
  (pages 28-34)
- **Bar graph**: 21 pointers (UT171A), 31 pointers (UT171B/C)
- **Data logging**: manual and timed storage, recall, delete; no record
  capacity found (storage section and page 4, renders, 2026-11-07)
- **Update rate**: 4-5 times per second
- **Keys and display words**: what each key does, the LCD symbol table,
  NCV's `EF` and dash segments, the square-wave output
- **Settings**: USB communication on/off, backlight, buzzer, auto power
  off, RTC date/time (UT171C)

PDF pages 16-19, the specification tables, have no text layer, and the
2026-11-07 key and display reading
(`references/ut171/manual-keys-2026-11-07.md`) was done from page renders
only.

### UT171C PC Software

The vendor software installer (`UT171C setup.exe`, InstallShield)
installs to `C:\Program Files\DMM\UT171C\` with these files:

| File | Size | Purpose |
|------|------|---------|
| UT171C.exe | 10.3 MB | Main application (Delphi, DevExpress GUI) |
| SLABHIDtoUART.dll | 80 KB | Silicon Labs CP2110 HID-to-UART library |
| SLABHIDDevice.dll | 108 KB | Silicon Labs HID device library |
| gdi32.dll | 305 KB | Custom GDI shim |
| sys.ini | 62 B | Default settings (interval, duration) |

### UT171C Software Installation Instruction

This PDF (4 pages) shows the software UI including:
- **Complete function button grid**: All measurement modes the software
  can command, organized by category
- **Toolbar**: Conn, DisConn, Start/Pause Receive Data, Hold, Hz%,
  Peak, Max Min, Parameter
- **Data management**: Save Current Data, Auto Save, Read Data by Index,
  Read All Data, Query Data Count, Delete Data by Index, Delete All Data

## Analysis Techniques Used

### 1. Download and catalog

Downloaded from `meters.uni-trend.com`:
- User manual PDF (11.2 MB)
- UT171B software ZIP (5.8 MB, contains UT171C setup.exe)

### 2. Installer extraction

The InstallShield setup.exe could not be extracted with standard tools
(7z, unshield). Wine was used to run the installer and extract the
application files:

```sh
WINEPREFIX=~/wine-ut171 wine "UT171C setup.exe" /v"/qn"
# Files installed to: drive_c/Program Files (x86)/DMM/UT171C/
```

### 3. String extraction

```sh
strings -a -el UT171C.exe | grep -iE 'mode|range|HidUart|connect'
```

Key findings:
- `HidUart_Open`, `HidUart_Read`, `HidUart_Write`, `HidUart_SetUartConfig`,
  `HidUart_Close` -- confirms Silicon Labs HidUart API usage
- Mode name strings: LoZV, VFC, VAC, VDC, mVDC, mVAC, OHM, BEEP, CAP,
  DIOD, nS, ADC, AAC, mADC, mAAC, uADC, uAAC, AAC+DC, mAAC+DC,
  uAAC+DC, mVAC+DC, VAC+DC, PLUSE_O, 600A
- Per-mode chart range strings: `ADC Range 1: -6 to 6`,
  `BEEP Range 1: 0 to 600`, `nS Range 1: 0 to 60`, etc.
- `"First Query Data Count,Please!"` -- data logging sequence
- VID/PID strings: `L"10C4"`, `L"EA80"`

`strings -el` drops characters outside ASCII, so the Ω and µ rows came out
as ` Range 1: 0 to 600 ` with no unit; the first reading took them for
unlabelled rows and missed the resistance table.

### 4. Ghidra headless decompilation

```sh
~/stuff/ghidra/ghidra_12.0.4_PUBLIC/support/analyzeHeadless \
    /tmp/ghidra_ut171c2 ut171c_proj \
    -import UT171C.exe \
    -postScript GhidraDecompile.java \
    -deleteProject -scriptPath /tmp \
    > ut171c_decompiled.txt 2>&1
```

The decompilation produced 881,179 lines (28,745 functions). It stops at
VA 0xBB2318 and skips almost all of the main form unit
(0xBB6D08–0xBC3400): the receive handler, the frame timer and every frame
handler are missing from it.

Functions the first reading identified, and what they turned out to be:
- `FUN_00755228` (line 397330) -- reader thread (polling HidUart_Read,
  SendMessage to main thread with received data). Holds.
- `FUN_0075425c` (line 396673) -- USB HID initialization (LoadLibrary
  SLABHIDtoUART.dll, VID/PID matching, UART configuration). Holds.
- `FUN_00755400` (line 397440) -- taken for the command frame builder. It
  is the published method `TCP2110.SendCommand`, with no caller in `.text`.
- `FUN_0065478e` (line 297968) -- taken for the frame header validator. It
  parses the app's Midas/ClientDataSet packets, as do `FUN_0065492d` and
  `FUN_00654bf5`.
- `FUN_00630e0b`, `FUN_00630d5c`, `FUN_00630dd4`, `FUN_00630c1e`,
  `FUN_0064081c`, `FUN_006405b1` -- taken for the mode transition table,
  the mode groups and the mode encoders. They are field-type code in the
  statically linked Midas/DBClient dataset engine.

### 5. Disassembly re-read (2026-11-06 and 2026-11-07)

Asked by the user while planning the UT171 rework: two blind reads of
UT171C.exe, each by a separate subagent that saw no other's notes, the
second also without the spec's earlier conclusions until its own reading
was done. Their findings agree except on the captions of the °C and °F
buttons (stale "OHM" in the first, the degree signs in the second), so the
spec cites those buttons' handler names. The second is more complete on
commands, replies and status bits.

- `objdump -d -M intel` of UT171C.exe, cited by VA, for the main form unit
  the decompile lacks.
- TMainFrm's RTTI tables: the published method table for method names and
  addresses, the extended field table for field names and offsets
  (`FMeasure_Code` at +0x670 and so on).
- The TMainFrm DFM resource for button names, captions and default label
  text.
- A direct-call search over all of `.text` for each function the spec
  cited, and a scan of all 35 loads of the status-array pointer for the
  readers of bytes 5 and 6.
- The manual's keys, dial, display words, NCV and square wave, read from
  page renders (2026-11-07).

The reports are in `references/review-2026-11-04/arch/vendor-audit-2026-11-06/`
(`ut171.md`, `second-read/ut171.md`) and
`references/ut171/manual-keys-2026-11-07.md`.

## What Misled the First Reading

- The decompile has none of the app's frame code (technique 4), so the
  first reading searched it for anything that looked like a parser and
  found the Midas/DBClient dataset engine: its packet magic `0xBDE01996`,
  its field-type switch and its type-size table. The header check, the
  flag bits, the frame type in byte 6, the 26-entry mode table, the mode
  groups, the transition commands and the square-wave pseudo-mode 0x1007
  all came from there. None of those functions has a caller outside the
  engine.
- `TCP2110.SendCommand` writes a 1-byte length and was taken for the
  command builder; the commands 0x01, 0x51, 0x52 and 0xFF came from it.
  Every send site writes constant 2-byte-length templates instead.
- `strings -el` hid the Ω and µ range rows (technique 3).
- gulux's parser was taken for capture-driven; the 2026-11-06 read found
  no capture in it, so the "USB confirmed" mode bytes and the 0x40/0x20
  "DC/AC" byte rested on its two test frames and its code.

How it was settled: the disassembly by VA, the RTTI field and method
tables and the DFM captions name what each address does, and the two blind
reads came to the same map. The spec's §3.3, §4, §5 and §6 now carry it.

## What Was Determined

### From the User Manual (highest confidence)

| Finding | Confidence | Source |
|---------|------------|--------|
| UT171A: 40,000 counts; UT171B/C: 60,000 counts | **[KNOWN]** | Manual page 3 |
| All measurement ranges and accuracy specs | **[KNOWN]** | Manual pages 28-34 |
| Data logging: STORE, timed storage (1–240 s interval), recall, delete | **[KNOWN]** | Manual page 9 |
| K-type thermocouple (UT171B/C only) | **[KNOWN]** | Manual page 20 |
| Square wave output (UT171C only) | **[KNOWN]** | Manual page 25 |
| 600A current clamp (UT171C only) | **[KNOWN]** | Manual page 23 |
| NCV: `EF`, then 1–5 dash segments | **[KNOWN]** | Manual page 24 |
| Lightning bolt: "Alarm for high voltage" | **[KNOWN]** | Manual page 6 |
| Update rate: 4-5 times/second | **[KNOWN]** | Manual page 4 |
| CAT III 1000V / CAT IV 600V | **[KNOWN]** | Manual page 2 |

### From Vendor Software Analysis

| Finding | Confidence | Source |
|---------|------------|--------|
| Uses SLABHIDtoUART.dll (HidUart API) | **[VENDOR]** | String extraction + decompilation |
| VID 0x10C4, PID 0xEA80 (CP2110) | **[VENDOR]** | Decompilation: L"10C4", L"EA80" |
| Baud rate index 6 = 9600 | **[VENDOR]** | Decompilation: FUN_00754560 |
| Reader thread: 70ms polling, SendMessage dispatch | **[VENDOR]** | Decompilation: FUN_00755228 |
| UART: 500ms timeout, 100ms read interval | **[VENDOR]** | Decompilation: FUN_00754560 |
| Frame check: header, LE16 length, LE16 checksum | **[VENDOR]** | Disassembly: `tmr1Timer`, VA 0xBB978C |
| Command table, all on the 2-byte framing | **[VENDOR]** | Disassembly: send sites, `.data` templates 0xBE1D9C |
| Measurement layout, byte 13, unit codes, no scaling | **[VENDOR]** | Disassembly: `HandleMeasurement`, VA 0xBBFBF0 |
| Status bits of bytes 5 and 6, with every reader | **[VENDOR]** | Disassembly: status-array scan |
| Range rows per mode | **[VENDOR]** | Disassembly: range builder, VA 0xBBD330 |
| Mode map 0x01–0x1F | **[VENDOR]** | Disassembly: function commands, DFM captions, ribbon groups |
| Stored record, replies 0x01 and 0x72 | **[VENDOR]** | Disassembly: VA 0xBC159C, 0xBBF9E0, 0xBC2800 |
| Complete CP2110 feature report map (20 reports) | **[VENDOR]** | Ghidra: SLABHIDtoUART.dll decompilation |
| UART config report 0x50 byte layout confirmed | **[VENDOR]** | Ghidra: SLABHIDtoUART.dll |
| UART status FIFO counts are big-endian | **[VENDOR]** | Ghidra: SLABHIDtoUART.dll report 0x42 |

### Retracted

| First reading | What it was | Source of the correction |
|---------------|-------------|--------------------------|
| Mode bytes from FUN_0064081c, FUN_006405b1, FUN_00630c1e | Dataset field-type codes | Second read, callers only inside the engine |
| Flag bits: AUTO bit 6 inverted, HOLD bit 7, LowBat bit 2, extended frame bit 1, MIN/MAX bit 3 gated by bit 0 (FUN_00980070, FUN_0065492d + "USB captures") | HOLD bit 7 holds; the rest are §5.3's readings; bit 2 has no reader, and the "captures" were gulux's code | Status-array scan, both reads |
| Frame type in byte 6 (FUN_00654bf5) | Status bits | Both reads |
| Header check in FUN_0065478e | Dataset packet parser; the real check is VA 0xBB978C | Both reads |
| Command builder FUN_00755400, lengths 0x03/0x04/0x0A/0x12, commands 0x01/0x51/0x52/0xFF | Uncalled `SendCommand` | Direct-call search |
| Mode transition table, square-wave 0x1007 with 0xE0/0xE1 (FUN_00630e0b) | Field-type conversion | Second read |
| NCV = 0x24 | NCV = 0x1A; 0x24 is not in the app | Both reads |

### What Still Requires Device Verification

The open checks are in [verification.md](verification.md).

## Sources Used and Avoided

| Source | Status | Date |
|--------|--------|------|
| UT171A/B/C manual, UT171C software and installation instruction, Silicon Labs DLLs | Used | From the start (2026-06); re-read 2026-11-06/07 |
| gulux/Uni-T-CP2110 (GPL-3.0): `README.md`, `UT171A.py`, `UTHID.py`, `testing/UTTest.py` | Used for cross-reference only, after the vendor reads | Fetched 2026-11-06, at the user's request |
| Other community UT171 code | Not consulted for this spec (the UT-D07B docs cite libreble for the adapter only) | — |

The two blind vendor reads opened no community code and read the spec's
cross-reference sections only as claims to test. gulux's frames stay out
of the repo by the user's ruling: the spec says they decode under its
layout without reproducing them.

## Cross-Reference with Community Sources

### gulux/Uni-T-CP2110 (Python, UT171A)

Compared in the spec's
[cross-reference table](reverse-engineered-protocol.md#cross-reference-with-community-sources),
2026-11-07. It agrees on the framing, the float, HOLD and twelve mode
bytes, and its two test frames decode under the app's layout; it reads
0x0B as diode against the app's capacitance, and byte 5 bit 2 as low
battery, which the app does not read.

## File Inventory

| Source | File | What it provides |
|--------|------|-----------------|
| UNI-T | `references/ut171/uni-t_ut171_manual.pdf` | All modes, ranges, accuracy, features |
| UNI-T | `references/ut171/vendor-software/UT171C.exe` | Main application binary (10.3 MB) |
| UNI-T | `references/ut171/vendor-software/SLABHIDtoUART.dll` | CP2110 UART library |
| UNI-T | `references/ut171/vendor-software/SLABHIDDevice.dll` | CP2110 HID library |
| UNI-T | `references/ut171/vendor-software/sys.ini` | Default config |
| Analysis | `references/ut171/vendor-software/ut171c_decompiled.txt` | Ghidra decompilation (881K lines), without the main form unit |
| Analysis | `references/review-2026-11-04/arch/vendor-audit-2026-11-06/ut171.md`, `second-read/ut171.md` | The two disassembly reads |
| Analysis | `references/ut171/manual-keys-2026-11-07.md` | The manual's keys and display words, from renders |
| UNI-T | `references/ut171/vendor-software/UT171B_Software.zip` | Original download |
