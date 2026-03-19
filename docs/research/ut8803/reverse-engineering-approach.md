# Reverse Engineering Approach: UT8803/UT8803E Protocol

## Objective

Reconstruct the UT8803/UT8803E USB communication protocol using only
official, publicly available sources:

1. **UT8803E User Manual** (from UNI-T, `UT8803E_User_Manual.pdf`)
2. **UT8803E Programming Manual** (from UNI-T, `UT8803E_Programming_Manual.pdf`)
3. **UT8803E Data Sheet** (from UNI-T, `UT8803E_DataSheet.pdf`)
4. **UNI-T SDK V2.3** (from UNI-T, `UNI-T_SDK_V2.3.zip`)
5. **UT8803E Software V1.1** (from UNI-T, `UT8803E_Software_V1.1.rar`)
6. **CP2110 Datasheet** (from Silicon Labs)
7. **AN434: CP2110/4 Interface Specification** (from Silicon Labs)

No community implementations, forum posts, or third-party reverse
engineering work.

## What Each Source Provides

### UT8803E User Manual

The manual is a consumer-facing document for the UT8803 and UT8803E
bench DMMs. It does not describe the USB protocol, but it fully defines
the **application-layer semantics** that the protocol must encode:

- **Measurement modes**: DC voltage, AC voltage, DC current, AC current,
  Resistance, Capacitance, Inductance (with Q and R sub-measurements),
  Frequency, Duty ratio, Diode, Continuity, hFE, Thyristor (SCR),
  Temperature (C/F)
- **Display**: 6000 counts maximum (5999), refresh 2-3 times/sec
- **Bar graph**: Simulated bar graph on LCD
- **Display indicators**: L/C, AUTO, RANGE, MAX, MIN, HOLD, REL-delta,
  SER (series), PAL (parallel), USB, hFE, D/Q/R (sub-measurement),
  AC, DC, AC+DC, negative, units
- **Ranges**: DC voltage (600mV/6V/60V/600V/1000V), AC voltage
  (600mV/6V/60V/600V/750V), DC current (600uA/6mA/60mA/600mA/10A),
  AC current (600uA-6mA/60mA-600mA/10A), Resistance
  (600R/6k/60k/600k/6M/60M), Capacitance (6nF/60nF/600nF/6uF/60uF/
  600uF/6mF), Inductance (600uH/6mH/60mH/600mH/6H/60H/100H),
  Frequency (600Hz/6kHz/60kHz/600kHz/6MHz/20MHz), Temperature
  (-40 to 1000C / -40 to 1832F)
- **USB port**: Located on rear panel, mentioned as "USB connection"
  with Software DVD included in box

### UT8803E Programming Manual (V1.0/V1.1)

This is the **most critical source**. It is an official UNI-T document
that specifies the programming interface for the UT8800 series bench
DMMs. Key findings:

**Supported models and USB identification:**

| Model | Interface | VID&PID | Instrument Address |
|-------|-----------|---------|-------------------|
| UT632/UT632N | USB HID | 0x1A86 & 0xE008 | `[C:DM][D:T632][T:HID][PID:0xe008][VID:0x1a86]` |
| UT803/UT803N | USB HID | 0x1A86 & 0xE008 | `[C:DM][D:T803][T:HID][PID:0xe008][VID:0x1a86]` |
| UT804/UT804N | USB HID | 0x1A86 & 0xE008 | `[C:DM][D:T804][T:HID][PID:0xe008][VID:0x1a86]` |
| UT805A/UT805N | USB COM (serial) | N/A | `[C:DM][D:T805A][T:COM][PORT:8][BAUD:9600]...` |
| UT8802/UT8802N | USB HID | 0x10C4 & 0xEA80 | `[C:DM][D:T8802][T:HID][PID:0xea80][VID:0x10c4]` |
| UT8803/UT8803N | USB HID | 0x10C4 & 0xEA80 | `[C:DM][D:T8803][T:HID][PID:0xea80][VID:0x10c4]` |

**Communication architecture**: UCI (United Communication Interface)
SDK-based. The host does NOT send raw byte-level commands like the
UT61E+. Instead, the host uses the `uci.dll` library, which abstracts
the USB HID transport. The UCI library handles framing, transport, and
protocol details internally.

**Two query commands**:
- `data?;` -- Returns raw measurement value as an 8-byte `double`
- `disp?;` -- Returns a `DMFRM` structure with display strings and flags

**DMFRM structure** (from `disp?;` command):
```c
struct DMFRM {
    TCHAR MainDisp[20];     // Main display string (20 chars)
    TCHAR AuxDisp[20];      // Secondary display string (20 chars)
    double MainValue;       // Main display numeric value (8 bytes)
    double AuxValue;        // Secondary display numeric value (8 bytes)
    unsigned long long Flags; // Flag bits (8 bytes)
};
```

**Status bits (low 32 bits of Flags, common to all models):**

| Bits | Width | Meaning |
|------|-------|---------|
| D0-D3 | 4 | Functional coding (see table) |
| D4-D5 | 2 | AC&DC status (0=OFF, 1=AC, 2=DC, 3=AC+DC) |
| D6 | 1 | Auto Range (0=no, 1=yes) |
| D7 | 1 | Over load (0=no, 1=yes) |
| D8-D11 | 4 | Physical unit type (see table) |
| D12-D14 | 3 | Physical unit magnitude (see table) |
| D15 | 1 | Low battery (0=no, 1=yes) |
| D16 | 1 | USB communication (0=no, 1=yes) |
| D17 | 1 | Under status (0=no, 1=yes) |
| D18 | 1 | Over status (0=no, 1=yes) |
| D19 | 1 | Display minus symbol (0=no, 1=display) |
| D20-D23 | 4 | Position coding (model-specific) |
| D24-D27 | 4 | Scaling position (starts from 1) |
| D28 | 1 | MAX (UT8802/UT8803 only) |
| D29 | 1 | MIN (UT8802/UT8803 only) |
| D30 | 1 | REL (UT8802/UT8803 only) |
| D31 | 1 | HOLD (UT8802/UT8803 only) |

**Status bits (high 32 bits of Flags, UT8802/UT8803 only):**

| Bits | Width | Meaning |
|------|-------|---------|
| D0 | 1 | Error flag |
| D1 | 1 | Test mode (1=serial/SEL, 0=parallel/PAL) |
| D2 | 1 | Diode direction right-to-left (1=valid) |
| D3 | 1 | Diode direction left-to-right (1=valid) |
| D4 | 1 | Inductance quality element measurement |
| D5 | 1 | Equivalent resistance measurement |
| D6 | 1 | Capacitance loss element measurement |
| D7 | 1 | Capacitance equivalent resistance measurement |
| D8-D15 | 8 | Position (model-specific functional coding) |
| D16-D31 | 16 | Hold |

**Functional coding table (common):**

| Coding | Function |
|--------|----------|
| 0 | Voltage |
| 1 | Resistance (OHM) |
| 2 | Diode |
| 3 | Continuity |
| 4 | Capacitance |
| 5 | Frequency |
| 6 | Temperature Fahrenheit |
| 7 | Temperature Centigrade |
| 8 | hFE |
| 9 | Current |
| 10 | % (4-20mA) |
| 11 | Duty |
| 12 | Thyristor (SCR) |
| 13 | Inductance (including Q and R) |

**UT8803/UT8803N position coding** (from high 32 bits D8-D15):

| Position | Coding |
|----------|--------|
| ACV | 0 |
| DCV | 1 |
| ACuA | 2 |
| AC mA | 3 |
| AC A | 4 |
| DCuA | 5 |
| DC mA | 6 |
| DC A | 7 |
| OHM | 8 |
| Continuity | 9 |
| Diode | 10 |
| Inductance (L) | 11 |
| Inductance (Q) | 12 |
| Inductance (R) | 13 |
| Capacitance (C) | 14 |
| Capacitance (D) | 15 |
| Capacitance (R) | 16 |
| hFE | 17 |
| SCR | 18 |
| Temp Centigrade | 19 |
| Temp Fahrenheit | 20 |
| Frequency | 21 |
| Duty | 22 |

**Physical unit type coding:**

| Coding | Type |
|--------|------|
| 0 | Voltage (V) |
| 1 | Current (A) |
| 2 | Resistance (ohm) |
| 3 | Frequency (Hz) |
| 4 | Centigrade (C) |
| 5 | Fahrenheit (F) |
| 6 | RPM(rpm)hold |
| 7 | Capacitance (F) |
| 8 | Triode hFE |
| 9 | Percentage (%) |
| 0xF | No display |

**Physical unit magnitude coding:**

| Coding | Prefix |
|--------|--------|
| 0 | n (nano) |
| 1 | u (micro) |
| 2 | m (milli) |
| 3 | standard (none) |
| 4 | K (kilo) |
| 5 | M (mega) |
| 6 | G (giga) |

**AC and DC status coding:**

| Coding | Status |
|--------|--------|
| 0 | OFF |
| 1 | AC |
| 2 | DC |
| 3 | AC+DC |

**GetUnit helper** (from programming manual example code):
```c
std::wstring unit[] = {
    "V", "A", "ohm", "Hz", "C", "F",
    " ", "F", "beta", "%", " "
};
std::wstring scale[] = {
    "n", "u", "m", " ", "k", "M", "G", " "
};
unsigned char unit_scale = Bits(dfrm.Flags, 8, 0xf);  // bits 8-11
unsigned char unit_type  = Bits(dfrm.Flags, 12, 0x7);  // bits 12-14
```

**Example output** (from programming manual):
```
UT8802N instrument displayed: 50.23 Hz
disp? : main = 50.23, aux = , mv = 50.230000 Hz, av = 0.000000, flags = 0x2013345
data? : value = 50.230000 Hz, flags = 0x2013345
```

### CP2110 Datasheet + AN434

Same transport layer as the UT61E+ -- see the UT61E+ research docs for
full details. The UT8803 uses the same CP2110 HID bridge with VID
0x10C4, PID 0xEA80.

### UNI-T SDK V2.3

The SDK provides:
- `uci.dll` (C library, x86 and x64, ASCII and Unicode variants)
- `ucics.dll` (C# wrapper)
- `ucivb.dll` (Visual Basic wrapper)
- Header files: `uci.h`, `ucidef.h`, `uci_cpp.h`, `unit.h`
- Example code in C, C++, C#, CVI, LabView, VB
- Driver packages for libusb and USB serial port

The SDK is primarily designed for oscilloscopes and signal generators,
but includes DMM support through the `data?;` and `disp?;` commands.

**Key finding from string extraction on `uci.dll`**:
- Separate UT8802 and UT8803 parsers: `[UT880X] Parse is UT8802` / `[UT880X] Parse is UT8803`
- Checksum validation: `[UT8803] Data check sum error!`
- Frame parsing: `[Frame]%d cur frame = %dBytes, nRead = %d, left = %d`
- Protocol commands: `data?;`, `disp?;`, `FRMDATA?`, `mode`
- SCPI-like display command: `:DISPlay:DATA?`
- Baud rate configuration: `baud=%d parity=%c data=%d stop=%d`, `[BAUD:`
- HID transport: `HidD_SetFeature`, `HidD_GetAttributes`, `HidD_GetHidGuid`
- Class hierarchy: `HIDCOM`, `UTXHID`, `SyncCOM`, `CSerialPort`

### UT8803E Software V1.1

The vendor software is packaged as an InstallShield installer
(`Setup.exe`, 20 MB) containing `UT8803.msi`. The installer could not
be extracted with available tools (InstallShield format). Only the
outer `Setup.exe` and an embedded `User Manual.pdf` were recovered.

String extraction from Setup.exe confirmed it references `UT8803.msi`
but contained no protocol-level information.

## Analysis Techniques Used

### 1. Download and catalog

Downloaded from `instruments.uni-trend.com`:
- User manual PDF (12.2 MB, 30 pages)
- Programming manual PDF (1.2 MB, 13 pages)
- Data sheet PDF (586 KB)
- Vendor software RAR (19 MB)
- UNI-T SDK ZIP (158 MB)

```sh
curl -L -o references/ut8803/UT8803E_User_Manual.pdf "<url>"
curl -L -o references/ut8803/UT8803E_Programming_Manual.pdf "<url>"
curl -L -o references/ut8803/UT8803E_DataSheet.pdf "<url>"
curl -L -o references/ut8803/UT8803E_Software_V1.1.rar "<url>"
curl -L -o references/ut8803/UNI-T_SDK_V2.3.zip "<url>"
```

### 2. Installer extraction

```sh
# SDK (ZIP) -- extracted successfully
7z x -o"references/ut8803/sdk" references/ut8803/UNI-T_SDK_V2.3.zip

# Vendor software (RAR v5 with newer compression)
unrar x references/ut8803/UT8803E_Software_V1.1.rar references/ut8803/vendor-software/extracted/
# Yields: Setup.exe (InstallShield), User Manual.pdf

# Setup.exe is InstallShield -- cannot extract with 7z
file references/ut8803/vendor-software/extracted/Setup.exe
# → PE32 executable ... InstallShield
```

### 3. String extraction

```sh
# From uci.dll (the protocol library)
strings -a "references/ut8803/sdk/.../lib/C/Unicode/uci.dll" | grep -i -E 'UT880|parse|check|frame|data\?|disp\?'
strings -el "references/ut8803/sdk/.../lib/C/Unicode/uci.dll" | grep -i -E 'UT880|data\?|disp\?|baud|HID|mode'
```

Key findings:
- `[UT8803] Data check sum error!` -- confirms checksummed protocol
- `data?;` and `disp?;` -- the two query commands
- `[UT880X] Parse is UT8802` / `Parse is UT8803` -- separate parsers
- `baud=%d parity=%c data=%d stop=%d` -- UART configuration
- `HidD_SetFeature`, `HidD_GetAttributes` -- direct HID API usage
- Class names: `HIDCOM`, `UTXHID`, `SyncCOM`, `frm_ParseChunk`

### 4. Ghidra headless decompilation

```sh
GHIDRA=~/stuff/ghidra/ghidra_12.0.4_PUBLIC
mkdir -p /tmp/ghidra_ut8803_uci

$GHIDRA/support/analyzeHeadless \
    /tmp/ghidra_ut8803_uci ut8803_uci \
    -import "references/ut8803/sdk/.../lib/C/Unicode/uci.dll" \
    -postScript GhidraDecompile.java \
    -deleteProject \
    -scriptPath /tmp \
    > references/ut8803/vendor-software/uci_dll_decompiled.txt 2>&1
```

The uci.dll is a large (~4.5 MB) MFC-based DLL with thousands of
functions covering oscilloscopes, signal generators, and DMMs. The
decompilation produced 451,736 lines (13 MB) of C pseudocode.

Key functions identified:
- `FUN_1001e5f0` — UT8803 measurement response parser (21-byte frame,
  AB CD header, alternating-byte checksum, mode/range/flags extraction)
- `FUN_1001e0a0` — UT8802 measurement response parser
- Frame discriminator at `FUN_1001eb30`: `0xAC` → UT8802, `0xAB 0xCD` → UT8803
- `FUN_1001ca90`, `FUN_1001c880`, `FUN_1001cdc0`, `FUN_1001cff0` — mode/range/unit lookup functions
- `FUN_1001d170` — unit type name lookup
- `FUN_1001cec0` — unit magnitude prefix lookup
- `FUN_1001d460` — CP2110 HID initialization (UART enable, config, trigger)
- `FUN_1001fce0` — buffer append function (display byte accumulation)
- `FUN_1001f170` — frame read loop (HID read + frame reassembly + parser dispatch)
- `FUN_1002a380` — HID read with timeout accumulation
- `FUN_1002a500` — HID write (WriteFile wrapper)
- `FUN_1002a4d0` — single-byte UART write (used for 0x5A trigger)

### 5. SDK source code analysis

The SDK includes complete C/C++/C# example code showing the UCI API
usage pattern:

```c
// Open device
u_status r = uci_OpenX(
    "[C:DM][D:T8803][T:HID][PID:0xea80][VID:0x10c4]", 2000);

// Read display information
DMFRM dfrm;
r = uci_ReadX(session, "disp?;", 2000,
    (unsigned char*)&dfrm, sizeof(dfrm));

// Parse flags
unsigned char unit_scale = Bits(dfrm.Flags, 8, 0xf);
unsigned char unit_type  = Bits(dfrm.Flags, 12, 0x7);

// Read raw measurement value
double dv;
r = uci_ReadX(session, "data?;", 2000,
    (unsigned char*)&dv, sizeof(dv));
```

### 6. Targeted Ghidra decompilation analysis (Phase 2)

Following the initial parser analysis, a second pass targeted the
remaining protocol gaps by searching for specific patterns in the
451,736-line decompilation output:

**Baud rate confirmation**: Searched for `0x2580` (9600 in hex) and
`0x50` (feature report ID). Found `FUN_1001d460` which constructs
the CP2110 feature report 0x50 with `local_20 = 0x25000050` and
`local_1c = 0x3000080`. This decodes as report ID 0x50, baud rate
0x00002580 = 9600, parity=none, flow=none, data=8, stop=1. Also
found that `local_24[0] = 0x141` is the UART enable feature report
(0x41, value 0x01).

**Command encoding discovery**: Found `FUN_1002a4d0(param_2, 0x5a, 1000)`
called immediately after UART configuration. Traced `FUN_1002a4d0` to
`FUN_1002a500` which calls `WriteFile` — this is a HID write sending
byte 0x5A over UART. The frame read loop `FUN_1001f170` only calls
read functions (`FUN_1002a380`), never write functions, confirming
the meter streams continuously after the 0x5A trigger.

**Display byte encoding**: Analyzed `FUN_1001fce0` (10 call sites in
decompilation). The function signature is
`void __thiscall FUN_1001fce0(int *param_1, undefined1 *param_2)`.
It checks buffer capacity (`param_1[1]` vs `param_1[2]`), calls
`FUN_1001a5a0(1)` to grow the buffer if needed, then copies the byte
directly: `*(undefined1 *)param_1[1] = *param_2`. This is a standard
vector/buffer append — no byte transformation. The display bytes are
raw values.

**Unknown frame bytes**: Careful analysis of the parser's byte access
pattern (using `param_2` as `short *`):
- Byte 2: Part of `param_2[1]` word (bytes 2-3). Only byte 3 is
  checked (`!= '\x02'`). Byte 2 is not independently consumed.
- Byte 6: No parser code accesses byte offset 6. It is included in
  the alternating-byte checksum but otherwise ignored.
- Bytes 12-13: Accessed as `param_2[6]` (a 16-bit word). Bits are
  extracted for inductance test frequency and other measurement flags.
  Byte 13 specifically is accessed at `*(byte*)((int)param_2 + 0x11)`
  and combined with byte 16 to form a 9-bit field.

**LoZ mode bytes**: Searched for 0x15 and 0x16 near mode comparisons.
The only relevant hit was at line 29130 in `FUN_10023530`, which
checks `param_2 != 0x14 && param_2 != 0x15 && param_2 != 0x16 &&
param_2 != 0x17`. However, this function handles DSO (oscilloscope)
channel configuration (DSOCOMFit class), not DMM mode bytes. The
UT8803 mode bytes 0x15 and 0x16 are not referenced in the DMM parser
code — they are simply allowed by the `0x16 < bVar1` range check.
No disambiguation logic exists in uci.dll for these two mode values.

## What Was Determined

### From the Programming Manual (highest confidence)

The programming manual is an **official UNI-T document** that explicitly
defines the programming interface:

| Finding | Confidence | Source |
|---------|------------|--------|
| VID 0x10C4, PID 0xEA80 | **[KNOWN]** | Programming manual support model table |
| USB HID interface (not serial) | **[KNOWN]** | Programming manual support model table |
| UCI SDK-based communication | **[KNOWN]** | Programming manual API reference |
| `data?;` returns double (8 bytes) | **[KNOWN]** | Programming manual command reference |
| `disp?;` returns DMFRM struct | **[KNOWN]** | Programming manual command reference |
| DMFRM struct layout | **[KNOWN]** | Programming manual data structure section |
| 64-bit Flags word bit definitions | **[KNOWN]** | Programming manual status bit list |
| Functional coding table | **[KNOWN]** | Programming manual coding table |
| UT8803 position coding table | **[KNOWN]** | Programming manual UT8803 section |
| Physical unit type/magnitude tables | **[KNOWN]** | Programming manual coding tables |
| AC/DC status coding | **[KNOWN]** | Programming manual coding table |
| Models sharing protocol: UT632, UT803, UT804, UT8802, UT8803 | **[KNOWN]** | Programming manual support table |
| UT805A uses serial port, not HID | **[KNOWN]** | Programming manual support table |

### From SDK and string analysis

| Finding | Confidence | Source |
|---------|------------|--------|
| uci.dll implements HID transport internally | **[VENDOR]** | String extraction: HidD_* API calls |
| Separate UT8802 and UT8803 parsers | **[VENDOR]** | String: `[UT880X] Parse is UT8802/UT8803` |
| Checksummed protocol under UCI layer | **[VENDOR]** | String: `[UT8803] Data check sum error!` |
| Frame-based transport under UCI layer | **[VENDOR]** | String: `[Frame]%d cur frame = %dBytes` |
| `:DISPlay:DATA?` string (for oscilloscopes, not DMMs) | **[VENDOR]** | Ghidra: only in PLAIN-TEXT handler |
| libusb driver used on Windows | **[VENDOR]** | SDK driver package: DriverPack_Libusb |

### From Ghidra decompilation of uci.dll

| Finding | Confidence | Source |
|---------|------------|--------|
| Frame header: AB CD (same as UT61E+) | **[VENDOR]** | Parser: `*param_2 != -0x3255` (0xCDAB LE) |
| Byte 3 = 0x02 (measurement response type) | **[VENDOR]** | Parser: `byte[3] != '\x02'` |
| Frame size: 21 bytes minimum | **[VENDOR]** | Parser: `param_3 < 0x15` |
| Checksum: alternating-byte sum, BE at bytes 19-20 | **[VENDOR]** | Parser checksum loop |
| Mode byte at offset 4, raw (0x00-0x16) | **[VENDOR]** | Parser: `bVar1 = (byte)param_2[2]`, `0x16 < bVar1` |
| Range byte at offset 5, 0x30 prefix | **[VENDOR]** | Parser: `bVar5 = bVar5 - 0x30, 6 < bVar5` |
| Display bytes at offsets 7-11 | **[VENDOR]** | Parser: character processing loop |
| Flag bytes at offsets 14-18 | **[VENDOR]** | Parser: bit extraction operations |
| UT8802 header: 0xAC (single byte) | **[VENDOR]** | Discriminator: `*_Memory == -0x54` |
| UT8803 header: 0xAB 0xCD (two bytes) | **[VENDOR]** | Discriminator: `*_Memory == -0x55 && _Memory[1] == -0x33` |
| Inductance test freq field: 0=100Hz, 1=1kHz | **[VENDOR]** | Parser: mode 0x0B-0x10 branch |
| Status word matches programming manual layout | **[VENDOR]** | Format string: `ACDC,dotpos,fun,isauto,ismax,ismin,ishold,isrel,isOL` |
| Baud rate: 9600 (from feature report 0x50) | **[VENDOR]** | `FUN_1001d460`: `local_20 = 0x25000050` → 0x00002580 = 9600 |
| CP2110 init: 0x41 enable + 0x50 config + 0x5A trigger | **[VENDOR]** | `FUN_1001d460`: three-step initialization |
| Trigger command: single byte 0x5A | **[VENDOR]** | `FUN_1002a4d0(param_2, 0x5a, 1000)` |
| Streaming model: continuous after 0x5A trigger | **[VENDOR]** | `FUN_1001f170`: read loop with no write ops |
| Display bytes: raw passthrough (no 0x30 mask) | **[VENDOR]** | `FUN_1001fce0`: buffer append function, not transform |
| Byte 6: not accessed by parser (reserved) | **[VENDOR]** | Parser: no reference to byte offset 6 |
| Bytes 12-13: flag source for inductance/cap flags | **[VENDOR]** | Parser: `param_2[6]` bit extraction |
| High-byte flags: D4-D7 from mode equality checks | **[VENDOR]** | Parser: `bVar1==0x0C/0x0D/0x0F/0x10` |
| `:DISPlay:DATA?` is oscilloscope path, not DMM | **[VENDOR]** | Ghidra: only in `PLAIN-TEXT` handler |

### What still requires device verification

1. **Byte 2 in frame**: This byte is part of the `param_2[1]` short
   word (bytes 2-3), where byte 3 is verified as 0x02. Byte 2 itself
   is not independently consumed by the parser but is included in the
   checksum. Its actual value on the wire is unknown — could be a
   length byte, sequence number, or always zero. [UNVERIFIED]

2. **Maximum sampling rate**: The user manual says 2-3 Hz refresh.
   Whether the streaming rate is exactly 2 Hz, 3 Hz, or variable has
   not been measured. [UNVERIFIED]

3. **0x5A trigger semantics**: The byte 0x5A is sent once after UART
   init. Whether it is a "start streaming" command, a device wake
   signal, or serves another purpose is not clear from the decompilation
   alone. Whether the meter can be stopped/restarted with different
   commands is unknown. [UNVERIFIED]

**Previously unverified, now resolved:**

4. ~~**Baud rate**~~: **Confirmed 9600** from feature report 0x50
   construction in `FUN_1001d460`. [VENDOR]

5. ~~**Command format**~~: **Confirmed** as single 0x5A trigger byte,
   followed by continuous streaming. No SCPI text commands. [VENDOR]

6. ~~**Streaming vs polling**~~: **Confirmed streaming** — the read
   loop in `FUN_1001f170` only reads, never writes. [VENDOR]

7. ~~**Display byte encoding**~~: **Confirmed raw passthrough** —
   `FUN_1001fce0` is a buffer append function, not a byte
   transformation. [VENDOR]

8. ~~**Flag bit positions**~~: **Confirmed** matching the programming
   manual layout via the format string and bit shift analysis. [VENDOR]

9. ~~**Byte 6**~~: **Confirmed unused** — the parser does not access
   byte offset 6. It is included in the checksum but otherwise
   reserved/padding. [VENDOR]

10. ~~**Bytes 12-13**~~: **Confirmed as flag source** — `param_2[6]`
    (bytes 12-13) provides bits for inductance test frequency and
    other measurement-specific flags. [VENDOR]

## Fundamental Architectural Difference from UT61E+

The UT8803 uses a fundamentally different communication architecture
than the UT61E+:

| Aspect | UT61E+ | UT8803 |
|--------|--------|--------|
| Protocol layer | Raw byte protocol | UCI SDK abstraction over binary protocol |
| Command format | Binary: `AB CD 03 cmd chk_hi chk_lo` | Single 0x5A trigger byte |
| Response format | Binary: 19 bytes with mode/range/flags | Binary: 21 bytes with AB CD framing |
| Communication model | Polled (1 request per measurement) | Streaming (continuous after trigger) |
| USB bridge | CP2110 (VID 0x10C4, PID 0xEA80) | CP2110 (same VID/PID) |
| VID/PID | Silicon Labs defaults | Silicon Labs defaults |
| Software | Custom Qt app with CustomDmm.dll plugin | UCI SDK-based application |
| Flag encoding | 3 separate flag bytes in response | 5 flag bytes → 32-bit + 8-bit status words |
| Display value | 7 ASCII bytes in response | 5 raw bytes in response |

The raw wire protocol has been fully reverse-engineered from the
uci.dll decompilation, making a cross-platform implementation feasible
without the UCI SDK.

## File Inventory

| Source | File | What it provides |
|--------|------|-----------------|
| UNI-T | `references/ut8803/UT8803E_User_Manual.pdf` | Measurement modes, ranges, display specs |
| UNI-T | `references/ut8803/UT8803E_Programming_Manual.pdf` | UCI API, DMFRM struct, flags, coding tables |
| UNI-T | `references/ut8803/UT8803E_DataSheet.pdf` | Specifications summary |
| UNI-T | `references/ut8803/UT8803E_Software_V1.1.rar` | Vendor software installer |
| UNI-T | `references/ut8803/UNI-T_SDK_V2.3.zip` | UCI SDK with headers, libs, examples |
| Silicon Labs | (same as UT61E+ analysis) | CP2110 datasheet, AN434 |
| Analysis | `references/ut8803/vendor-software/uci_dll_decompiled.txt` | Ghidra decompilation of uci.dll |
| Extracted | `references/ut8803/sdk/UNI-T SDK V2.3/` | SDK contents (headers, examples, libraries) |
| Extracted | `references/ut8803/vendor-software/extracted/` | Setup.exe, User Manual.pdf |
