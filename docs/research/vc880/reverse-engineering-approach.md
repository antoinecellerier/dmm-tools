# Reverse Engineering Approach: Voltcraft VC-880 / VC650BT

## Objective

Independently derive the USB communication protocol for the Voltcraft
VC-880 (handheld DMM) and VC650BT (bench DMM), both 40,000-count meters
using a Silicon Labs CP2110 HID-to-UART bridge.

## Sources

All findings derive from official vendor sources. Community
implementations (pylablib) were consulted only after independent
analysis was complete (Phase 3 cross-reference).

### 1. Voltsoft Installer (Conrad item 124609/124411)

Downloaded from Conrad's product page. Inno Setup installer containing
the Voltsoft application suite.

**Key finding**: The VC-880 (item 124609) and VC650BT (item 124411)
installers are **byte-identical** (MD5: `4b955a1e8a51e7c89338c0c852e1c469`),
confirming both devices use the same software and protocol.

**Extraction**:
```sh
innoextract -d vc880-extracted/app autorun.exe
```

**Binary identification**: All protocol-relevant binaries are .NET
assemblies (not native code), enabling clean C# decompilation via
ILSpy rather than Ghidra.

### 2. DMSShare.dll (ILSpy decompilation)

The protocol implementation lives in `DMSShare.dll` (26,600 lines of
decompiled C#). Key classes:

| Class | Lines | Purpose |
|-------|-------|---------|
| `VC880Obj` | 21013-21905 | Device connection, command sending, frame reading, message dispatch |
| `VC880Reading` | 15443-16032 | Measurement mapping dictionary (38 entries), flag fields, value parsing |
| `VC880Type` | 20265-21012 | Device settings/configuration model |
| `HidUartController` | 7605-8622 | P/Invoke wrapper for SLABHIDtoUART.dll (CP2110 API) |

**Decompilation command**:
```sh
ilspycmd DMSShare.dll > DMSShare_decompiled.cs
```

### 3. VC880 User Manual (Conrad item 124609, VERSION 03/14)

English section pages 35-65. Provides:
- Measurement ranges and accuracy specifications (pages 62-65)
- 40,000 counts, 2-3 measurements/second
- CAT III 1000V / CAT IV 600V
- Measurement modes: DCV, ACV, AC+DC V, DCA, ACA, resistance,
  capacitance, frequency, duty cycle, diode, continuity, temperature,
  low-pass filter ACV
- Input impedance: >10MΩ (V range)

### 4. VC890 Installer (Conrad item 124600)

Also downloaded. Different MD5 (`52891ed3c724f430c74fdb0a0ad50a8e`) —
the VC-890 uses different software. The decompiled `DMSShare.dll`
contains a separate `VC890Obj`/`VC890Reading` class hierarchy,
confirming VC-890 has a different protocol from VC-880/VC650BT.

## Analysis Techniques

### .NET Decompilation (ILSpy)

Unlike native binaries (which require Ghidra and produce C pseudocode
with unnamed variables), .NET assemblies decompile to near-original C#
with full type names, method names, string constants, and enum values.
This produces significantly higher-confidence results than Ghidra.

The decompiled source preserves:
- Named flag fields (`Max_flag`, `Min_flag`, `Rel_flag`, `Hold_flag`, etc.)
- Named constants (`cmd_autoRange = 71`, `cmd_hold = 74`, etc.)
- Named function strings (`"VC880_FUNCTION_DCV"`, `"VC880_FUNCTION_ACV"`, etc.)
- Complete measurement mapping with human-readable range strings

### Targeted Analysis

Functions analyzed:
- `VC880Obj.OpenDevice()` (line 21221): CP2110 initialization
- `VC880Obj.WriteCommand()` (line 21238): Frame construction with checksum
- `VC880Obj.GetOneMessage()` (line 21555): Frame extraction from buffer
- `VC880Obj.CheckSum()` (line 21756): Per-message-type checksum validation
- `VC880Obj.ProcessMessage()` (line 21829): Message type dispatch
- `VC880Reading.SetLiveData()` (line 16325): Live data parsing entry point
- `VC880Reading.SetDeviceMode_And_Unit_And_Range()` (line 16333): Function code → measurement mapping
- `VC880Reading.SetReadingValue()` (line 16726): Display value extraction (4 fields)
- `VC880Reading.SetStatus()` (line 16782): Flag byte extraction (7 bytes, 28 named flags)
- `VC880Reading._MeasurementMapping` (line 15610): Complete measurement dictionary (38 entries)

### Installer Binary Comparison

The VC-880 and VC650BT installers are byte-identical, confirming shared
protocol. The VC-890 installer differs, and its `DMSShare.dll` contains
a separate `VC890Reading` class with a different measurement mapping
(38 entries but different function codes and range tables), confirming
VC-890 requires independent implementation.

## What Each Source Provides

### From the User Manual ([KNOWN])

| Finding | Source |
|---------|--------|
| 40,000 counts | Manual page 62 |
| 2-3 measurements/second | Manual page 62 |
| CAT III 1000V / CAT IV 600V | Manual page 62 |
| DCV ranges: 400mV / 4V / 40V / 400V / 1000V | Manual page 62 |
| ACV ranges: 4V / 40V / 400V / 1000V | Manual page 62-63 |
| AC+DC V ranges: 4V / 40V / 400V / 1000V | Manual page 63 |
| Impedance ranges: 400Ω / 4kΩ / 40kΩ / 400kΩ / 4MΩ / 40MΩ | Manual page 64 |
| Capacitance ranges: 40nF / 400nF / 4µF / 40µF / 400µF / 4000µF / 40mF | Manual page 64 |
| Frequency range: 10Hz - 40MHz (+ 400MHz unspecified) | Manual page 64 |
| Duty cycle: 5Hz-2kHz, 10%-90% | Manual page 65 |
| Temperature: °C and °F | Manual page 65 |
| Diode test voltage: 2.73V | Manual page 65 |
| Continuity threshold: <10Ω | Manual page 65 |
| DC current ranges: 400µA / 4000µA / 40mA / 400mA / 10A | Manual page 63 |
| AC current ranges: 400µA / 4000µA / 40mA / 400mA / 10A | Manual page 63 |

### From DMSShare.dll Decompilation ([VENDOR])

| Finding | Confidence | Source |
|---------|------------|--------|
| CP2110 bridge at 9600 baud, parity=NONE, stop=SHORT | [VENDOR] | `VC880Obj.OpenDevice()` line 21228 |
| Frame header: 0xAB 0xCD | [VENDOR] | `_header = { 171, 205 }` line 21061 |
| Frame format: header + length + command + data + checksum_BE16 | [VENDOR] | `WriteCommand()` lines 21238-21263 |
| Length byte = data.length + 3 | [VENDOR] | `WriteCommand()` line 21269 |
| Checksum = BE16 sum of all preceding bytes | [VENDOR] | `WriteCommand()` line 21243 |
| Live data message type = 0x01 | [VENDOR] | `msgTypeLiveData = 1` line 21051 |
| Live data frame = 39 bytes total (37 data + 2 checksum) | [VENDOR] | `CheckSum()` line 21796: sum bytes 0..37 |
| Function code at msg[4] (19 functions: 0x00-0x12) | [VENDOR] | `SetDeviceMode_And_Unit_And_Range()` switch |
| Range byte at msg[5] (ASCII '0'-'7', i.e. 0x30-0x37) | [VENDOR] | `SetDeviceMode_And_Unit_And_Range()` |
| Main display: 7 ASCII bytes at msg[6..12] | [VENDOR] | `SetReadingValue()` line 16728 |
| Sub display 1: 7 ASCII bytes at msg[13..19] | [VENDOR] | `SetReadingValue()` line 16738 |
| Sub display 2: 7 ASCII bytes at msg[20..26] | [VENDOR] | `SetReadingValue()` line 16748 |
| Bar graph: 3 bytes at msg[27..29] | [VENDOR] | `SetReadingValue()` line 16758 |
| Status bytes: 7 bytes at msg[30..36], 28 named flags | [VENDOR] | `SetStatus()` lines 16782-16813 |
| Streaming model (continuous read, no trigger needed) | [VENDOR] | `ContinuouslyReadDataLoop()` line 21734 |
| 16 command codes (0x00, 0x41-0x57, 0x5A, 0xFF) | [VENDOR] | `VC880Obj` static fields lines 21063-21125 |
| 38 measurement mapping entries | [VENDOR] | `_MeasurementMapping` lines 15610-16030 |
| Display values are ASCII strings (Encoding.ASCII.GetString) | [VENDOR] | `GetValue()` line 17117 |
| Frame recovery: scan for 0xAB 0xCD, skip non-matching bytes | [VENDOR] | `GetOneMessage()` line 21557 |
| VC880 and VC650BT share identical software (byte-identical installer) | [VENDOR] | MD5 comparison |
| VC890 has a separate protocol (different VC890Reading class) | [VENDOR] | DMSShare.dll class hierarchy |

### Deduced ([INFERRED])

| Finding | Confidence | Reasoning |
|---------|------------|-----------|
| No trigger/activation command needed | [INFERRED] | `OpenDevice()` calls `ContinuouslyReadData()` immediately after UART config; no write before first read |
| "PC button" activation is meter-side only | [INFERRED] | Manual describes pressing PC button; software has no corresponding command |
| msg[5] range byte uses 0x30 offset (ASCII encoding) | [INFERRED] | All comparisons use literal values 48-55 (0x30-0x37 = ASCII '0'-'7') |

### Requires Device Verification ([UNVERIFIED])

| Finding | Question |
|---------|----------|
| All 19 function codes produce correct mode labels | Need real meter to confirm |
| Range byte values per function code | Need real meter to confirm each range index |
| Status flag bit positions | Named in vendor code but not yet validated |
| Sub-display field formats | Are they always numeric ASCII? |
| Bar graph byte interpretation | 3 bytes — exact encoding unknown |
| Overload representation in ASCII value fields | "OL"? "---"? Need real meter |
| Streaming rate matches manual (2-3 Hz) | Need real meter |
| UART config parity=3 means NONE in SiLabs enum | Cross-check with SLABHIDtoUART.dll API docs |

## Cross-Reference Against pylablib (Phase 3)

Performed **after** independent analysis above.

| Finding | Our RE (Voltsoft) | pylablib VC880 class | Agreement |
|---------|-------------------|---------------------|:---------:|
| Frame header 0xAB 0xCD | Yes (vendor code) | Yes | ✓ |
| BE16 checksum | Yes (vendor code) | Yes | ✓ |
| Length byte = payload + 3 | Yes (vendor code) | Yes | ✓ |
| Live data type 0x01 | Yes (vendor code) | Yes | ✓ |
| 19 function codes (0x00-0x12) | Yes (vendor switch) | Yes (19 functions) | ✓ |
| Range byte 0x30-based | Yes (vendor comparisons) | Yes (subtract 0x30) | ✓ |
| 7-byte ASCII display values | Yes (Encoding.ASCII.GetString) | Yes (7 bytes) | ✓ |
| Status byte 1: bit0=Rel, bit1=Avg, bit2=Min, bit3=Max | Yes (SetStatus) | bit0=Rel, bit1=Avg, bit2=Min, bit3=Max | ✓ |
| Streaming (no trigger) | Yes (continuous read loop) | Yes (no trigger) | ✓ |
| cmd 0x47 = autorange enable | Yes (cmd_autoRange = 71) | Yes (0x47) | ✓ |
| cmd 0x46 = manual range | Yes (cmd_manualRange = 70) | Yes (0x46) | ✓ |
| 33-byte payload (pylablib) vs 35 data bytes (our count) | 35 bytes msg[4..38] | 33 bytes (function+range+values+flags) | ~¹ |
| Status bytes: 6 of 7 undocumented in pylablib | All 7 bytes, 28 flags named | Only stat[1] decoded | Our RE is richer |

¹ pylablib counts 33 bytes of "payload" (after header+length+type), while our analysis counts from msg[4] through msg[36] = 33 bytes of measurement data + 2 checksum = 35. These are consistent when accounting for what each includes.

**Key finding from cross-reference**: Our vendor decompilation reveals significantly more than pylablib:
- All 7 status bytes fully mapped (pylablib only decodes 1 of 7)
- 28 named flag bits (pylablib has 4)
- 16 command codes (pylablib has 2)
- 38 measurement mapping entries with function names
- VC890 confirmed as a separate protocol

## File Inventory

| Source | File | What it provides |
|--------|------|------------------|
| Conrad | `references/vc880/vendor-software/download-124609-*.zip` | Voltsoft installer (VC880) |
| Conrad | `references/vc880/vendor-software/download-124411-*.zip` | Voltsoft installer (VC650BT, byte-identical) |
| Conrad | `references/vc880/vendor-software/download-124600-*.zip` | Voltsoft installer (VC890, different) |
| Analysis | `references/vc880/vendor-software/DMSShare_decompiled.cs` | ILSpy C# decompilation (26,600 lines) |
| Conrad | `references/vc880/manuals/manual-124609-*.pdf` | VC880 user manual (specs pages 62-65) |
| Conrad | `references/vc880/manuals/datasheet-124411-*.pdf` | VC650BT datasheet |
