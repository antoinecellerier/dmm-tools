# Reverse Engineering Approach: UCI Bench DMM Family

## Objective

Extend the UT8803 reverse engineering to cover the remaining UCI bench
DMM models: UT8802/UT8802N, UT632/UT632N, UT803/UT803N, UT804/UT804N,
and UT805A/UT805N. These models all share the UCI (United Communication
Interface) protocol layer but differ in transport (CP2110 vs QinHeng HID
vs serial), wire format (0xAC 8-byte vs 0xABCD 21-byte), and
measurement capabilities.

## Sources

All findings derive from sources already obtained during the UT8803 RE:

1. **UT8803E Programming Manual V1.0/V1.1** (UNI-T) -- official
   document specifying the UCI API, DMFRM struct, all coding tables, and
   per-model support information
2. **UNI-T SDK V2.3** -- `uci.dll` library (protocol implementation)
3. **uci.dll Ghidra decompilation** (451K lines at
   `references/ut8803/vendor-software/uci_dll_decompiled.txt`) -- the
   primary source for wire-level protocol details

No additional downloads or vendor software were required. The UT8803
RE already captured all the binary analysis needed for the full UCI
family.

## What Each Source Provides for New Models

### UT8803E Programming Manual

The programming manual explicitly covers all UCI bench models:

| Model | Transport | VID:PID | Instrument Address |
|-------|-----------|---------|-------------------|
| UT632/UT632N | USB HID | 0x1A86:0xE008 | `[C:DM][D:T632][T:HID]...` |
| UT803/UT803N | USB HID | 0x1A86:0xE008 | `[C:DM][D:T803][T:HID]...` |
| UT804/UT804N | USB HID | 0x1A86:0xE008 | `[C:DM][D:T804][T:HID]...` |
| UT805A/UT805N | USB COM | Serial | `[C:DM][D:T805A][T:COM][PORT:8][BAUD:9600][PARITY:N][STOP:1][DATA:7]` |
| UT8802/UT8802N | USB HID | 0x10C4:0xEA80 | `[C:DM][D:T8802][T:HID]...` |
| UT8803/UT8803N | USB HID | 0x10C4:0xEA80 | `[C:DM][D:T8803][T:HID]...` |

The manual provides:
- **UT8802 position coding table** (page 10): 30+ position codes
  (0x01-0x2D) encoding both function and range
- **UT805A/UT805N range coding table** (page 12): range codes 0-9 for
  DCV, ACV+DCV, DCI, ACI+DCI, OHM, CAP, FREQ
- **UT804/UT804N range coding table** (page 12): range codes 0-8 for
  ACV, DCV, OHM, CAP, degC, uA, mA, 10A, Diode, FREQ, degF
- **Common status bit list** (page 6): 32-bit flags applicable to all
  models
- **High 32-bit status list** (page 7): UT8802/UT8803-specific flags
- **Note**: "query currently only support the device based on HID
  communication, COM communication does not yet add in query system" --
  UT805A serial was not yet implemented in the UCI SDK at time of
  writing

### uci.dll Ghidra Decompilation (Targeted Analysis)

The existing 451K-line decompilation was analyzed for specific functions
relevant to the new models:

**UT8802 parser (FUN_1001e0a0, line 24660)**:
- Complete 8-byte frame layout with 0xAC header
- BCD digit encoding (5 nibbles from bytes 2-4)
- Decimal point, sign, and flag extraction from bytes 5-7
- No checksum -- only header and position code validation

**Position-to-function lookup (FUN_1001c7b0, line 23234)**:
- Switch statement mapping 30 position codes (0x01-0x2D) to 13
  functional codes
- Cross-referenced against programming manual table

**QinHeng HID init (FUN_1001d360, line 23979)**:
- Primary init: 10-byte feature report + 0x5A trigger byte
- Fallback init (FUN_1001d270): different feature report, no trigger
- Buffer size 512 bytes, 64 HID input buffers

**Connection dispatch (FUN_1001ef50, line 25322)**:
- CP2110 path: tries UT8802 (300ms timeout) then UT8803 (2000ms)
- QinHeng path: tries primary init then fallback
- Auto-detect: scans for 0xAC or 0xAB+0xCD headers in incoming data

**Serial transport classes (line 48010+)**:
- CSerialPort wraps Win32 serial API (CreateFile, BuildCommDCB)
- Default 9600/8N1 configuration
- FS9721-style frame parsers found but likely for older UNI-T models,
  not UCI bench DMMs

**Helper functions**:
- FUN_1001ca30: ACDC coupling lookup per position code
- FUN_1001cd30: unit prefix/scale lookup per position code
- FUN_1001cf30: unit category lookup per position code
- FUN_1001b9b0: bitset construction (likely bargraph) from byte 6

## Analysis Techniques Used

### 1. Targeted Function Analysis

Rather than a full re-decompilation, we performed targeted reads of
specific function addresses already identified during the UT8803 RE:

```sh
# Read UT8802 parser (around line 24660)
# Read position-to-function-code lookup (around line 23234)
# Read QinHeng init functions (around line 23926-24050)
# Read connection dispatch (around line 25320-25450)
```

### 2. Pattern Search in Decompilation

Grep searches for specific patterns in the decompiled output:

```sh
grep -n "0xac\|0xAC\|-0x54" uci_dll_decompiled.txt    # UT8802 header
grep -n "QinHeng\|1A86\|E008" uci_dll_decompiled.txt    # QinHeng refs
grep -n "CSerialPort\|SyncCOM\|COM" uci_dll_decompiled.txt  # Serial
grep -n "0x5a\|0x5A" uci_dll_decompiled.txt              # Trigger byte
grep -n "local_54" uci_dll_decompiled.txt                 # Device type
```

### 3. Cross-Reference with Programming Manual

All decompilation findings were cross-referenced against the programming
manual tables:
- UT8802 position codes: decompiled switch statement matches page 10
  table exactly
- Functional coding: decompiled function codes 0-13 match page 7-8
  common table
- Status bit layout: decompiled format string matches page 6 bit list

## What Was Determined

### From the Programming Manual (highest confidence)

| Finding | Confidence | Source |
|---------|------------|--------|
| UT632/803/804 use QinHeng HID (1A86:E008) | **[KNOWN]** | Programming manual support table |
| UT805A uses serial port (9600 baud) | **[KNOWN]** | Programming manual support table |
| UT805A address string specifies DATA:7 | **[KNOWN]** | Programming manual address format |
| UT804 range coding table (page 12) | **[KNOWN]** | Programming manual |
| UT805A range coding table (page 12) | **[KNOWN]** | Programming manual |
| UT8802 position coding table (page 10) | **[KNOWN]** | Programming manual |
| Common functional coding (14 functions) | **[KNOWN]** | Programming manual |
| Common status bits (low 32) for all models | **[KNOWN]** | Programming manual |
| High 32-bit status for UT8802/UT8803 only | **[KNOWN]** | Programming manual |
| UCI serial query not yet implemented | **[KNOWN]** | Programming manual note |

### From uci.dll Decompilation

| Finding | Confidence | Source |
|---------|------------|--------|
| UT8802: 0xAC header, fixed 8-byte frame | **[VENDOR]** | Ghidra: FUN_1001e0a0 |
| UT8802: BCD nibble encoding (5 digits) | **[VENDOR]** | Ghidra: digit extraction code |
| UT8802: no checksum validation | **[VENDOR]** | Ghidra: no checksum code in parser |
| UT8802: position code → function via switch | **[VENDOR]** | Ghidra: FUN_1001c7b0 |
| UT8802: byte 7 carries sign + status flags | **[VENDOR]** | Ghidra: bit extraction code |
| UT8802: AUTO flag has inverted logic | **[VENDOR]** | Ghidra: `~(byte7 >> 2)` |
| QinHeng primary init: feature report + 0x5A | **[VENDOR]** | Ghidra: FUN_1001d360 |
| QinHeng fallback init: different report, no trigger | **[VENDOR]** | Ghidra: FUN_1001d270 |
| CP2110 init: enable + config + 0x5A trigger | **[VENDOR]** | Ghidra: FUN_1001d460 |
| CP2110 init: no purge report (unlike UT61E+) | **[VENDOR]** | Ghidra: absence of [0x43,0x02] |
| Auto-detect: scans for 0xAC or 0xABCD headers | **[VENDOR]** | Ghidra: FUN_1001eb30 |
| Frame dispatch: type 4=UT8802, type 5=UT8803 | **[VENDOR]** | Ghidra: connection dispatch |
| Serial default: 9600/8N1 (not 7 data bits) | **[VENDOR]** | Ghidra: CSerialPort defaults |

### Deduced (logical inferences)

| Finding | Confidence | Source |
|---------|------------|--------|
| UT632/803/804 use same wire format as UT8802 or UT8803 | **[KNOWN]** | Auto-detected at runtime; DLL scans for 0xAC or 0xABCD headers |
| QinHeng feature reports: primary=2400, fallback=19200 baud | **[KNOWN]** | CH9325 baud = uint16 LE (sigrok wiki + Lukas Schwarz UT61B + HE2325U driver) |
| UT805A serial likely uses same measurement frame format | **[DEDUCED]** | Same UCI layer; serial parsers in DLL are for older FS9721 meters |
| Byte 6 in UT8802 frame is bargraph/progress indicator | **[DEDUCED]** | Passed to bitset construction function FUN_1001b9b0 |

### What Still Requires Device Verification

1. ~~**QinHeng feature report baud rate encoding**~~ — **RESOLVED**
   2026-04-09: CH9325 uses uint16 LE baud encoding. Primary report
   `00 60 09 03...` = 2400 baud, fallback `00 00 4B 03...` = 19200 baud.
   Confirmed via [sigrok CH9325 wiki](https://sigrok.org/wiki/WCH_CH9325),
   [Lukas Schwarz UT61B analysis](https://lukasschwarz.de/ut61b), and
   [HE2325U driver code](https://github.com/thomasf/uni-trend-ut61d).

2. ~~**Which wire format do UT632/803/804 use?**~~ — **RESOLVED**
   2026-04-09: The vendor DLL does not dispatch per model. All QinHeng
   models use runtime auto-detection: scan incoming data for 0xAC or
   0xABCD headers (Ghidra FUN_1001eb30). Implementation should auto-detect.

3. **UT805A serial frame format**: The programming manual mentions
   7 data bits, but the DLL defaults to 8. The actual serial framing
   for UCI bench DMMs is unclear -- the FS9721-style parsers in the
   DLL appear to be for older models. [UNVERIFIED]

4. **UT8802 byte 6 purpose**: Passed to bitset construction but exact
   meaning unknown. Likely bargraph or secondary status. [UNVERIFIED]

5. **UT8802 byte 7 complete bit mapping**: The flag construction from
   byte 7 involves complex Ghidra stack aliasing that makes exact bit
   positions uncertain. Status flags (HOLD, REL, MAX, MIN, AUTO) are
   present but exact bit assignments need device confirmation.
   [UNVERIFIED]

6. **Diode/SCR flag construction anomaly**: Ghidra shows comparisons
   against 0x10/0x11 on a 2-bit-wide variable -- likely decompiler
   artifacts. The actual flag bit meanings for diode direction need
   device verification. [UNVERIFIED]

## File Inventory

All source files were already present from the UT8803 RE:

| Source | File | What it provides for this analysis |
|--------|------|-----------------------------------|
| UNI-T | `references/ut8803/UT8803E_Programming_Manual.pdf` | UT8802 position codes, UT804/UT805A range tables, model support table |
| Analysis | `references/ut8803/vendor-software/uci_dll_decompiled.txt` | UT8802 parser, QinHeng init, serial transport, connection dispatch |
| UNI-T | `references/ut8803/UT8803E_User_Manual.pdf` | UT8803 measurement ranges (reference) |
| UNI-T | `references/ut8803/UNI-T_SDK_V2.3.zip` | UCI SDK headers, examples, uci.dll binary |
| Silicon Labs | (same as UT61E+/UT8803 analysis) | CP2110 datasheet, AN434 |

No new downloads were required. Per-model user manuals for UT8802,
UT632, UT803, UT804, and UT805A would provide measurement ranges and
specifications but are not needed for protocol understanding.
