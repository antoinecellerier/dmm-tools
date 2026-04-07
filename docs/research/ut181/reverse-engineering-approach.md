# Reverse Engineering Approach: UT181A Protocol

## Objective

Document the UT181A communication protocol using publicly available
community reverse engineering work. The UT181A is one of the most
thoroughly reverse-engineered UNI-T meters, with three independent
implementations that agree on the protocol details.

## Sources

1. **antage/ut181a** (Rust library + Protocol.md) --
   https://github.com/antage/ut181a -- the primary protocol reference,
   MIT-licensed, includes complete [Protocol.md](https://github.com/antage/ut181a/blob/master/Protocol.md)
2. **loblab/ut181a** (C++ tool) -- https://github.com/loblab/ut181a --
   independent implementation with recording download support
3. **sigrok uni-t-ut181a driver** (C) --
   https://github.com/sigrokproject/libsigrok/tree/master/src/hardware/uni-t-ut181a --
   the most complete implementation, includes COMP mode and all 97
   measurement modes
4. **antage/cp211x_uart** (Rust crate) --
   https://github.com/antage/cp211x_uart -- CP2110/CP2114 UART
   control, used by antage/ut181a
5. **UNI-T UT181A user manual** -- from UNI-T website
6. **sigrok wiki** -- https://sigrok.org/wiki/UNI-T_UT181A -- hardware
   details, chipset identification

No vendor software decompilation was originally needed for the CP2110
protocol -- the community work is comprehensive. However, vendor
software analysis was later performed to investigate CH9329 (WCH)
cable support (see "Phase 2: CH9329 Transport Analysis" below).

## What Each Source Provides

### antage/ut181a (Rust)

The most important source. Provides:
- **Protocol.md**: Complete protocol specification document with frame
  format, all command codes, measurement packet layout, mode word table,
  recording protocol, and timestamp format
- **Rust library**: Working implementation covering monitoring, saved
  measurements, and recordings
- **cp211x_uart crate**: Reusable CP2110 UART control library

### loblab/ut181a (C++)

Independent implementation confirming:
- Frame format (header, length, checksum)
- Recording download protocol with 250-sample chunking
- CSV export of recorded data
- Command structure

### sigrok uni-t-ut181a driver

The most complete implementation:
- All 97 measurement modes parsed
- COMP (comparator) mode support
- Full recording protocol
- Bargraph data parsing
- All measurement variants (normal, relative, min/max, peak)

### sigrok wiki

Hardware teardown details:
- Cyrustek ES51997 analog frontend
- STM32F103 MCU
- 512 KiB flash, 1Mx16 SRAM, 24C256 EEPROM
- DS2086 RTC
- 7.4V 2200 mAh Li-ion battery + CR2032 backup

## Analysis Techniques

### 1. Community Implementation Cross-Reference

All three implementations were compared for agreement on:
- Frame header bytes (0xAB, 0xCD)
- Length field encoding (uint16 LE)
- Checksum algorithm (16-bit LE sum of length + payload bytes)
- Command codes (0x01-0x12)
- Measurement packet format (all variants)
- Mode word values (97 modes)
- Range byte values (0x00-0x08)
- Recording protocol (start, info, data download)
- Timestamp format (packed 32-bit)

All three agree on every detail. This gives [KNOWN] confidence.

### 2. Header Byte Clarification

A critical finding: the UT181A sends 0xAB then 0xCD on the wire --
the **same bytes** as UT61E+. The "reversed 0xCDAB header" description
(including in our `docs/supported-devices.md`) is misleading:

- antage/ut181a Protocol.md describes the magic as "0xCDAB" because it
  reads bytes as LE uint16: byte[0]=0xAB, byte[1]=0xCD → 0xCDAB
- UT61E+ docs describe the same bytes as "AB CD" (BE interpretation)
- Confirmed by all three codebases:
  - antage: `pkt.push(0xAB); pkt.push(0xCD);`
  - loblab: `START_BYTE1 = 0xAB; START_BYTE2 = 0xCD;`
  - sigrok: `FRAME_MAGIC 0xcdab` (LE uint16 constant)

The actual protocol differences from UT61E+ are in the length field
(2 bytes LE vs 1 byte), checksum (LE vs BE), and value encoding
(float32 vs ASCII).

### 3. Existing Project Context

Checked existing project files:
- `docs/supported-devices.md` lists UT181A with references to
  antage/ut181a and loblab/ut181a
- No existing `docs/research/ut181/` directory (created for this work)
- No UT181A-specific code in the Rust codebase

## What Was Determined

### Fully Confirmed ([KNOWN])

| Finding | Source |
|---------|--------|
| Frame structure (0xAB 0xCD header, uint16 LE length, uint16 LE checksum) | 3 implementations agree |
| Checksum = byte sum of length field + payload | 3 implementations agree |
| All 15 command codes (0x01-0x12) | antage + sigrok + loblab |
| All 97 mode words (0x1111-0xA231) | antage + sigrok |
| Range bytes 0x00-0x08 | antage + sigrok + loblab |
| Measurement packet format (all 4 variants) | antage + sigrok |
| COMP mode fields | sigrok driver |
| Unit strings (sent by device in packets) | antage + sigrok |
| Timestamp format (packed 32-bit) | antage + sigrok + loblab |
| Recording protocol (start/stop/info/data) | antage + sigrok + loblab |
| Record download chunking (250 samples max) | loblab implementation |
| 9600 baud 8N1 via CP2110 | all implementations |
| Communication must be manually enabled on meter | sigrok wiki + manual |
| Wire header is 0xAB 0xCD (not reversed) | all 3 codebases |
| 60,000 counts, dual display, TFT LCD | user manual |
| All measurement modes and ranges | user manual + implementations |

### Vendor Software Cross-Reference (April 2026)

Decompilation of UT181A.exe (V1.05, 13,980 functions) confirmed:
- Frame header 0xAB 0xCD written explicitly in transmit function
- Checksum algorithm identical (sum bytes[2..end], uint16 LE)
- Length field = payload + 2 (confirmed via `data_len + 3` pattern)
- Command codes 0x03, 0x05, 0x07-0x0A, 0x0C, 0x0E all confirmed
- Response dispatch on types 0x01, 0x02, 0x03, 0x04, 0x05, 0x72 confirmed
- **New: Command 0x0F** (DEL_RECORDING, uint16 LE index) found in vendor
  software but not in any community implementation. Confirmed from call
  site: prompted by "Are you sure that you want to delete this record?",
  followed by 300ms sleep and GET_REC_COUNT (0x0E) refresh.

The `0xBDE01996` constant found in the Delphi code is an ODBC/MDB
cursor header, not a protocol constant — the app uses database
functionality internally.

## Phase 2: CH9329 Transport Analysis

### Background

In April 2026, a user reported (dmm-tools#5) that their UT181A came
with a UT-D09 cable using a WCH CH9329 chip (VID `0x1A86`, PID
`0xE429`) instead of the expected CP2110 (VID `0x10C4`, PID `0xEA80`).
The [UT-D09 is listed on UNI-T's website](https://meters.uni-trend.com/product/ut-d-series-2/)
as suitable for UT171 series, UT243, and UT181A.

### Source: Vendor Software

Downloaded "UT181 updated software installation file.zip" from UNI-T's
meters site. The installer is an InstallShield 16 wrapper (`Setup.exe`,
5.1 MB, PE32) containing an MSI (`UT181A V1.05.msi`).

**Extraction steps:**
```sh
# Extract MSI from InstallShield wrapper via Wine
WINEPREFIX=/tmp/ut181-wine wine Setup.exe /b"Z:\tmp\ut181-extract"
# Results: UT181A V1.05.msi
# The installer also installs files directly to:
#   C:\Program Files (x86)\DMM\UT181A\
```

### Key Finding: Dual Transport Support [VENDOR]

The installed application ships with **both** transport DLLs:

| File | Size | Purpose |
|------|------|---------|
| `UT181A.exe` | 7.6 MB | Main application (Delphi, PE32) |
| `SLABHIDtoUART.dll` | 80 KB | Silicon Labs CP2110 HID-to-UART API |
| `SLABHIDDevice.dll` | 108 KB | Silicon Labs HID device enumeration |
| `CH9329DLL.dll` | 15 KB | WCH CH9329 HID API (built 2022-02-18) |
| `config.ini` | 151 B | Recording/display settings |
| `User Manual.pdf` | 543 KB | Software user manual (9 pages) |

This confirms that UNI-T's "updated" software version (V1.05, April 2022)
officially supports both the CP2110 and CH9329 cables.

### String Analysis Results

**Delphi class structure** (from ASCII/wide string extraction):

The application is a Delphi GUI with these transport-related classes:
- `TDeviceSelector` / `TFormDeviceSelect` -- device selection UI
- `DMCH9329` -- data module for CH9329 communication
- `TCH9329` -- Delphi wrapper for CH9329DLL, with `CH9329VID`/`CH9329PID` properties
- `CP21101Connect`/`CP21101Disconnect`/`CP21101ReceiveData` -- CP2110 event handlers
- `DeviceSelector1ReadyRead` -- common data-ready event for either transport
- `Baudrate`/`FBaudrate` -- UART baud rate field

**CP2110 API calls** (from `SLABHIDtoUART.dll`):
`HidUart_Open`, `HidUart_Read`, `HidUart_Write`, `HidUart_SetUartConfig`,
`HidUart_SetTimeouts`, `HidUart_GetNumDevices`, `HidUart_GetString`,
`HidUart_GetUartConfig`, `HidUart_Close`, `HidUart_CancelIo`

**CH9329DLL API** (exported functions):
`CH9329DllInt`, `CH9329OpenDevice`, `CH9329OpenDevicePath`,
`CH9329CloseDevice`, `CH9329ReadData`, `CH9329WriteData`,
`CH9329GetCFG`, `CH9329SetCFG`, `CH9329SetDEF`, `CH9329SetTimeOut`,
`CH9329Reset`, `CH9329GetAttributes`, `CH9329GetDevicePath`,
`CH9329GetHidGuid`, `CH9329GetBufferLen`, `CH9329InitThreadData`,
`CH9329ReadThreadData`, `CH9329GetThreadDataLen`,
`CH9329ClearThreadData`, `CH9329StopThread`

**CH9329DLL.dll internals** (from string and import analysis):
- Built with MSVC 9.0 (VS 2008)
- PDB path: `F:\workspace2008\CH9329DLL1\Release\CH9329DLL.pdb`
- Uses Windows HID API directly: `HidD_SetOutputReport`,
  `HidD_GetInputReport`, `HidD_GetAttributes`, `HidD_GetPreparsedData`,
  `HidP_GetCaps`
- Uses Windows SetupDi API for device enumeration
- Has a background read thread (`CreateThread`, `WaitForSingleObject`)
- Uses `CreateFileA` for HID device access

**Protocol-level functions** in UT181A.exe:
`CalCheckSum`, `FrameHeader`, `Cmd_Send`, `SendData`, `SendLen`,
`ReSend`/`ReSendNum`/`ReSendCount` (retry logic)

**VID/PID binary values** found in UT181A.exe:
- WCH VID `0x1A86`: 4 occurrences
- CH9329 PID `0xE429`: 8 occurrences
- SLAB VID `0x10C4`: 483 occurrences
- CP2110 PID `0xEA80`: 199 occurrences

### CH9329DLL Architecture [VENDOR]

The CH9329DLL.dll is a thin wrapper around Windows HID APIs. It does
**not** use serial port APIs (no `CreateFile("COM...")`, no
`SetCommState`). Instead it:

1. Enumerates HID devices via `SetupDiGetClassDevsA` + HID GUID
2. Matches devices by VID/PID using `HidD_GetAttributes`
3. Opens the device with `CreateFileA`
4. Reads via `HidD_GetInputReport` (not `ReadFile` on interrupt EP)
5. Writes via `HidD_SetOutputReport`
6. Runs a background thread for asynchronous reads

This means the CH9329 is configured in **Mode 0 or Mode 3** (custom HID
interface), not as a serial port. The DLL abstracts the HID report
framing so the application just calls `CH9329ReadData`/`CH9329WriteData`
with raw UART payload bytes.

### Implication for Our Implementation [INFERRED]

The CH9329 transport would work similarly to our existing CP2110
transport:
1. Open HID device by VID `0x1A86` / PID `0xE429`
2. Send data via HID output reports
3. Receive data via HID input reports
4. The UART-level protocol (frame format, commands, measurement parsing)
   is identical -- only the USB transport layer differs

The `DeviceSelector1ReadyRead` event handler in the Delphi app confirms
that both transports feed into the same data processing pipeline. The
application does not have separate protocol handling for each cable type.

### Ghidra Decompilation

Headless decompilation of both binaries:

```sh
# CH9329DLL.dll (15 KB, ~20 functions)
~/stuff/ghidra/ghidra_12.0.4_PUBLIC/support/analyzeHeadless \
    /tmp/ghidra_ch9329 ch9329_proj \
    -import CH9329DLL.dll \
    -postScript GhidraDecompile.java \
    -deleteProject -scriptPath /tmp \
    > references/ut181/vendor-software/CH9329DLL_decompiled.txt 2>&1

# UT181A.exe (7.6 MB Delphi app)
~/stuff/ghidra/ghidra_12.0.4_PUBLIC/support/analyzeHeadless \
    /tmp/ghidra_ut181a ut181a_proj \
    -import UT181A.exe \
    -postScript GhidraDecompile.java \
    -deleteProject -scriptPath /tmp \
    > references/ut181/vendor-software/UT181A_decompiled.txt 2>&1
```

**CH9329DLL.dll:** Decompiled successfully (44 functions, 1,936 lines).
Initially appeared to hang, but the root cause was a Wine symlink loop
in `/tmp` (`/tmp/ut181-wine/dosdevices/z: -> /`) causing Ghidra's
`GhidraSourceBundle.findPackageDirs()` to recurse infinitely when
scanning `-scriptPath /tmp`. Removing the Wine prefix fixed all
decompilation hangs.

**UT181A.exe:** Decompiled successfully (13,980 functions, 518K lines).
Delphi application with DevExpress GUI framework. Protocol code is
embedded in compiled virtual method tables, similar to UT171C.exe.

Key findings from UT181A.exe decompilation:
- Frame header validator at `FUN_006ceec9`: checks for `0xABCD`
  (measurement frames) and `0xBDE01996` (internal command framing,
  not wire format — community implementations confirm `0xAB 0xCD`
  is used in both directions on the wire)
- `CH9329OpenDevicePath` and `CH9329CloseDevice` calls confirm CH9329
  DLL integration

### Cross-Reference: UT61E+ DeviceSelector DLL [VENDOR]

The UT61E+ vendor software's `DeviceSelector.dll` (Qt/C++, already
decompiled in `references/ut61eplus/vendor-software/DeviceSelector_decompiled.txt`)
contains a complete CH9329 transport implementation with the same DLL.
This provides detailed HID report framing without needing to decompile
the UT181A binary:

**HID report framing** (from DeviceSelector decompilation,
functions FUN_10001650 and FUN_10001730):

```
Report layout (65 bytes = 0x41):
  Byte 0:      Report ID (always 0x00)
  Byte 1:      Data length (number of UART bytes in this report)
  Bytes 2-64:  UART data payload (up to 63 bytes)
```

Read path (`ReadFile`-based, overlapped I/O with configurable timeout):
1. `ReadFile(handle, buffer, report_size, ...)` — read one HID report
2. Byte 1 of buffer = actual UART data length
3. `memcpy(output, buffer+2, buffer[1])` — extract UART payload

Write path:
1. Zero 65-byte buffer
2. Set `buffer[1] = data_length` (guard: must be < 0x41 = 65)
3. `memcpy(buffer+2, data, data_length)` — fill UART payload
4. `WriteFile(handle, buffer, report_size, ...)` — send report

Report sizes are obtained dynamically from `HidP_GetCaps` after
calling `HidD_GetPreparsedData`. Input report size stored in
`DAT_100157d8`, output report size in `DAT_100157f8`.

**Initialization sequence** (from FUN_100013d0 and FUN_100055e0):
1. `CreateFileA(path, GENERIC_READ|GENERIC_WRITE, ...)` — open HID
2. `HidD_GetPreparsedData` + `HidP_GetCaps` — get report sizes
3. Set read/write timeouts to 1000ms each
4. Baud rate stored as `0x2580` (9600 decimal) at object offset 0x85c
5. Start background read thread (`HIDDevice::doRead` at FUN_10005b70)

**Config protocol** (used during init via `HidD_Set/GetOutputReport`,
may be optional for data streaming):
```
Config read (4 chunks × 32 bytes = 128 bytes total):
  Write: 00 A0 00 20   (0x2000a000 LE) → Sleep 100ms → Read
  Write: 00 A0 20 20   (0x2020a000 LE) → Sleep 100ms → Read
  Write: 00 A0 40 20   (0x2040a000 LE) → Sleep 100ms → Read
  Write: 00 A0 60 20   (0x2060a000 LE) → Sleep 100ms → Read

Config write (4 chunks):
  Write: 00 A1 00 20   (0x2000a100 LE)
  Write: 00 A1 20 20   (0x2020a100 LE) → Sleep 100ms
  Write: 00 A1 40 20   (0x2040a100 LE) → Sleep 100ms
  Write: 00 A1 60 20   (0x2060a100 LE)
```
These use `HidD_SetOutputReport`/`HidD_GetInputReport` (feature report
path), not the `ReadFile`/`WriteFile` data path. They correspond to the
CH9329's GET_PARA_CFG (0x08) and SET_PARA_CFG (0x09) serial commands.

**Device selector UI:**
Combo box with two options: "CP2110" and "CH9329".
Both feed into the same `readyRead(uchar *data, int len)` signal,
confirming a shared data processing pipeline.

**Architecture:**
- `CH9329Controller` class manages the transport
- `CH9329` (HIDDevice subclass, 0x86C bytes) handles HID I/O in a thread
- Constructor takes VID and PID as parameters
- Background read thread polls via `ReadFile` with 1000ms timeout
- Received UART data emitted via `readyRead` signal

### Resolved from Decompilation

| Item | Finding | Confidence |
|------|---------|------------|
| HID report structure | Report ID 0x00, byte 1 = length, bytes 2+ = UART data, 65 bytes total | [VENDOR] |
| UART baud rate | 9600 (0x2580 stored at object offset 0x85c) | [VENDOR] |
| Read/write mechanism | `ReadFile`/`WriteFile` for data, `HidD_Set/GetOutputReport` for config | [VENDOR] |
| Bidirectional | Yes — `WriteFile` used with same report layout | [VENDOR] |

### What Remains Unknown [UNVERIFIED]

| Item | What we need | Impact on implementation |
|------|-------------|------------------------|
| CH9329 operating mode | Mode 0 (composite KB+mouse+custom HID) or Mode 3 (custom HID only)? `lsusb -v` from device owner needed | If Mode 0, need to select the correct HID interface (not keyboard/mouse). `hidapi` filtering by usage page should handle this |
| Config sequence necessity | Is the 4-chunk config read/write required before data flows, or does the CH9329 come pre-configured? | Can implement without it initially — if data doesn't flow, add config init |
| Cable availability | Is UT-D09 now standard with new UT181A purchases, or a regional/production variant? | No impact on implementation |

## File Inventory

Community sources (online):

| Source | URL | What it provides |
|--------|-----|-----------------|
| antage/ut181a | github.com/antage/ut181a | Protocol.md + Rust library |
| loblab/ut181a | github.com/loblab/ut181a | C++ implementation + recording |
| sigrok driver | github.com/sigrokproject/libsigrok | Complete C driver |
| sigrok wiki | sigrok.org/wiki/UNI-T_UT181A | Hardware teardown |
| UNI-T manual | meters.uni-trend.com | User manual (specs, modes, ranges) |

Reference files (in `references/ut181/`):

| File | What it is |
|------|-----------|
| `vendor-software/UT181 updated.../Setup.exe` | InstallShield installer (V1.05) |
| `vendor-software/extracted/UT181A/UT181A.exe` | Main Delphi application |
| `vendor-software/extracted/UT181A/CH9329DLL.dll` | WCH CH9329 HID bridge DLL |
| `vendor-software/extracted/UT181A/SLABHIDtoUART.dll` | Silicon Labs CP2110 DLL |
| `vendor-software/extracted/UT181A/SLABHIDDevice.dll` | Silicon Labs HID device DLL |
| `vendor-software/extracted/UT181A/config.ini` | Default settings |
| `vendor-software/extracted/UT181A/User Manual.pdf` | Software manual (9 pages) |
| `vendor-software/CH9329DLL_decompiled.txt` | Ghidra decompilation output |
| `vendor-software/UT181A_decompiled.txt` | Ghidra decompilation output |
