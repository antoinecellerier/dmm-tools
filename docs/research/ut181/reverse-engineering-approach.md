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
   the most complete implementation, includes COMP mode and all 79
   measurement modes
4. **antage/cp211x_uart** (Rust crate) --
   https://github.com/antage/cp211x_uart -- CP2110/CP2114 UART
   control, used by antage/ut181a
5. **UNI-T UT181A user manual** -- from UNI-T website
6. **sigrok wiki** -- https://sigrok.org/wiki/UNI-T_UT181A -- hardware
   details, chipset identification

No vendor software decompilation was originally needed for the CP2110
protocol -- the community work is comprehensive. However, vendor
software analysis was later performed twice: to investigate CH9329 (WCH)
cable support (see "Phase 2: CH9329 Transport Analysis" below), and to
put the mode- and range-setting commands on a second, independent
footing (see "Phase 3" below).

The community sources do document SET_MODE (0x01), SET_RANGE (0x02) and
the 79 mode words -- that material is [KNOWN] in
`reverse-engineered-protocol.md` §4.2, §6 and §7 and predates Phase 3.
What they do not give is the *semantics* around them: which mode words
are reachable from which dial position, what each nibble-1 variant is
called on the meter, which variants offer REL, the per-family manual
range ladders, and which families have no manual range at all. Phase 3
adds those, corrects one payload width the community specs get wrong
(SET_MIN_MAX), and independently re-sources the opcodes and payload
layouts that were previously agreement-between-implementations only.

Phase 3 used **only** the vendor binary that was already approved and
extracted for Phase 2 (`references/ut181/vendor-software/extracted/UT181A/UT181A.exe`).
No new community source was read for it, and no reference implementation
was consulted while tracing; the comparison against antage and sigrok in
"What this settles" happens after the fact, against the tables already in
`reverse-engineered-protocol.md`.

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
- All 79 measurement modes parsed
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
- Mode word values (79 modes; the figure was 97 in the first draft of
  this document and was corrected in the 2026-06 review — both sigrok
  and antage define 79)
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
| All 79 mode words (0x1111-0xA231) | antage + sigrok |
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

## Phase 3: Mode and range commands (TfrmSetting trace)

September 2026. Source: the vendor binary only
(`references/ut181/vendor-software/extracted/UT181A/UT181A.exe`, V1.05,
already approved and extracted in Phase 2). Result:
`reverse-engineered-protocol.md` §4.2, §6.1 and §7.1.

### Why the first vendor pass missed SET_MODE and SET_RANGE

The April 2026 pass read the Ghidra headless decompilation
(`UT181A_decompiled.txt`, 13,980 functions, 518K lines) and found
opcodes 0x03, 0x05, 0x07-0x0A, 0x0C, 0x0E and 0x0F -- but not SET_MODE
(0x01), SET_RANGE (0x02), SET_MIN_MAX (0x04), SAVE_MEAS (0x06),
GET_REC_SAMPLES (0x0D) or HOLD (0x12).

That is a gap in the decompilation, not in the binary. Ghidra's
auto-analysis creates functions from call targets; a Delphi event
handler is never called directly -- it is reached through the form's
RTTI method table -- so nothing marks it as code. Of the 69 published
`TfrmSetting` handlers, **67 are absent** from the decompile. (The two
it did get, `edtRelValueKeyPress` at `0x86d2ac` and `miRecordViewClick`
at `0x86f230`, happen to be call targets elsewhere.) The output jumps
straight from `FUN_00868128` to `FUN_0086cfc0`, and again from
`FUN_0086d600` to `FUN_0086eea0`.

The six send wrappers those handlers use -- `0x8702b8`, `0x8702d8`,
`0x8703a4`, `0x8703e8`, `0x8703fc`, `0x870588` -- are missing for the
same reason, one step removed: their *only* call sites are inside
handlers Ghidra never turned into code, so nothing referenced them
either. Their neighbours in the same 0x8702xx-0x8705xx run
(`0x870298`, `0x8702f8`, `0x87030c`, `0x870344`, `0x870364`,
`0x870378`, `0x870384`, `0x870390`, `0x870408`, `0x87059c`,
`0x8705b0`) are all present -- those are reached from ordinary
methods. That asymmetry is what made the first pass look like a
complete command list when it was not.

The three steps below close that gap without re-running Ghidra.

### Step 1 -- recover handler addresses from the Delphi method table

A Delphi class's published method table is a run of
`[u16 entry_len][u32 VA][shortstring name]` records with
`entry_len == len(name) + 7`. Scanning the raw PE bytes for that
invariant recovers every handler address with its source-level name:

```python
import re, struct
d = open('references/ut181/vendor-software/extracted/UT181A/UT181A.exe',
         'rb').read()
for m in re.finditer(rb'[\x03-\x40][A-Za-z_][A-Za-z0-9_]{2,63}', d):
    off = m.start(); n = d[off]; name = d[off+1:off+1+n]
    if len(name) != n or not re.fullmatch(rb'[A-Za-z_][A-Za-z0-9_]*', name):
        continue
    if off < 6:
        continue
    elen = struct.unpack_from('<H', d, off-6)[0]
    addr = struct.unpack_from('<I', d, off-4)[0]
    if elen == n + 7 and 0x401000 <= addr < 0x8d0000 \
            and 0x46a400 <= off <= 0x46b200:
        print(f"file 0x{off:x}  addr 0x{addr:08x}  {name.decode()}")
```

The `0x46a400..0x46b200` window is `TfrmSetting`'s table; find it by
searching for the class-name shortstring `b'\x0bTfrmSetting'` (file
offset `0x46acf2`) and bracketing around it, or drop the window
entirely to dump every class in the binary. 69 entries come out,
including:

```
addr 0x0086d31c  FormCreate
addr 0x0086ceac  btnUpdate1Click
addr 0x0086cf0c  cbBoxRangeChange
addr 0x0086dffc  rbtnFXClick
addr 0x0086e9d8  rbtnVAC_M1Click     ... one per primary radio ...
addr 0x0086cd0c  actHoldExecute
addr 0x0086cd18  actMaxMinExecute
addr 0x0086ce80  btnMaxMinExitClick
addr 0x0086ce90  btnMaxMinSaveClick
addr 0x0086ce9c  btnRestartClick
addr 0x0086d28c  edtRelValueChange
addr 0x0086f2c4  tmrRelTimer
```

The same trick reads the published *field* table (`vmtFieldTable`,
entries `[u32 offset][u16 class index][shortstring name]`), which turns
the `[ebx+0x438]` operands in the handlers into component names --
`rbtnVAC_F1`, `rbtnVAC_F2`, `rbtnVAC_F3`, `PageControl1` at `+0x408`,
and so on. 287 entries.

### Step 2 -- disassemble those addresses directly

`objdump` maps PE sections itself, so the virtual addresses from step 1
work as-is:

```sh
objdump -d -M intel --start-address=0x86ceac --stop-address=0x86cfc0 \
    references/ut181/vendor-software/extracted/UT181A/UT181A.exe
```

Delphi's `register` convention (`eax`, `edx`, `ecx`, then stack) makes
the send wrappers trivial to read. `0x8702b8`, for instance, is nine
instructions: split `dx` into two stack bytes, `push 2`, point `ecx` at
them, `mov dl,0x1`, `call 0x870408`. Two helper addresses recur and are
worth naming once: `0x4aeb9c` is `TControl.SetEnabled` (it writes the
bool to `[self+0x61]` and sends `CM_ENABLEDCHANGED`, `0xB00B`) and
`0x4aecdc` is `SetCaption`. `0x42a588` is the `Sleep` import thunk.

Delphi `UnicodeString` literals are addressed at their first character,
with `length` as a `u32` at `addr-4`, so a caption operand like
`mov edx,0x86ea4c` resolves with:

```python
o = 0x400 + 0x86ea4c - 0x401000        # .text: VA 0x401000 -> file 0x400
n = struct.unpack_from('<I', d, o-4)[0]
print(d[o:o+2*n].decode('utf-16-le'))  # -> 'VAC'
```

### Step 3 -- parse the form resource (binary DFM)

The click handlers only manipulate radio buttons; the `Tag` values that
actually compose the mode word, and the range combo contents, live in
the form resource. `TfrmSetting`'s binary DFM starts at file offset
`0x620334` (`TPF0` signature). The format is: object = class
shortstring, name shortstring, then `(name, value-type byte, value)`
property triples until a `0` byte, then child objects until a `0` byte.

```python
import struct
d = open('references/ut181/vendor-software/extracted/UT181A/UT181A.exe',
         'rb').read()
p = 0x620334 + 4                       # skip the 'TPF0' signature

def sstr():
    global p
    n = d[p]; p += 1; s = d[p:p+n]; p += n
    return s.decode('latin1')

def value():
    global p
    t = d[p]; p += 1                   # Delphi TValueType
    if t == 0: return None                                     # vaNull
    if t == 1:                                                 # vaList
        out = []
        while d[p] != 0: out.append(value())
        p += 1; return out
    if t == 2: v = struct.unpack_from('<b', d, p)[0]; p += 1; return v
    if t == 3: v = struct.unpack_from('<h', d, p)[0]; p += 2; return v
    if t == 4: v = struct.unpack_from('<i', d, p)[0]; p += 4; return v
    if t == 5: p += 10; return 'extended'
    if t in (6, 7): return sstr()                  # vaString / vaIdent
    if t == 8: return False
    if t == 9: return True
    if t == 10:                                                # vaBinary
        n = struct.unpack_from('<I', d, p)[0]; p += 4 + n; return f'bin{n}'
    if t == 11:                                                # vaSet
        out = []
        while True:
            e = sstr()
            if not e: break
            out.append(e)
        return set(out)
    if t == 12:                                                # vaLString
        n = struct.unpack_from('<I', d, p)[0]; p += 4
        s = d[p:p+n].decode('latin1'); p += n; return s
    if t == 13: return None                                    # vaNil
    if t == 14:                                                # vaCollection
        out = []
        while d[p] != 0:
            if d[p] in (2, 3, 4): value()
            item = {}
            while d[p] != 0:
                k = sstr(); item[k] = value()
            p += 1; out.append(item)
        p += 1; return out
    if t == 15: v = struct.unpack_from('<f', d, p)[0]; p += 4; return v
    if t in (16, 17, 21): p += 8; return 'num8'
    if t == 18:                                                # vaWString
        n = struct.unpack_from('<I', d, p)[0]; p += 4
        s = d[p:p+2*n].decode('utf-16-le'); p += 2*n; return s
    if t == 19: v = struct.unpack_from('<q', d, p)[0]; p += 8; return v
    if t == 20:                                                # vaUTF8String
        n = struct.unpack_from('<I', d, p)[0]; p += 4
        s = d[p:p+n].decode('utf-8', 'replace'); p += n; return s
    raise ValueError(f'unknown value type {t} at 0x{p-1:x}')

def obj(depth):
    global p
    if d[p] & 0xF0 == 0xF0:            # ffInherited / ffChildPos / ffInline
        f = d[p]; p += 1
        if f & 0x02: p += 2
    cls = sstr(); name = sstr(); props = {}
    while d[p] != 0:
        k = sstr(); props[k] = value()
    p += 1
    kids = []
    while d[p] != 0:
        kids.append(obj(depth + 1))
    p += 1
    return {'class': cls, 'name': name, 'props': props,
            'children': kids, 'depth': depth}

KEEP = ('Tag', 'Caption', 'OnClick', 'Visible', 'ItemIndex',
        'Items.Strings')

def walk(o):
    keep = {k: v for k, v in o['props'].items() if k in KEEP}
    print('  ' * o['depth'] + f"{o['class']} {o['name']} {keep}")
    for c in o['children']: walk(c)

walk(obj(0))
```

Output (abridged -- 21 tab sheets, 287 components, matching the field
table count from step 1):

```
TTabSheet SheetVAC {'Tag': 4352, 'Caption': '   VAC    '}
  TGroupBox GroupBoxRange11 {'Tag': 3, 'Caption': ' Range '}
    TComboBox cbBoxVACRange {'Tag': 4, 'ItemIndex': 0,
      'Items.Strings': ['Auto', '0 - 6', '0 - 60', '0 - 600', '0 - 1000']}
  TGroupBox GroupBoxMenu11 {'Tag': 1, 'Caption': ' Primary Mode'}
    TRadioButton rbtnVAC_M1 {'Tag': 1, 'Caption': 'VAC',
      'OnClick': 'rbtnVAC_M1Click'}
    ... M2 'VAC,HZ', M3 'Peak', M4 'LowPass', M5 'dBV', M6 'dBm' ...
  TGroupBox GroupBoxFX11 {'Tag': 2, 'Caption': ' Secondary Mode '}
    TRadioButton rbtnVAC_F1 {'Tag': 1, ...}
    TRadioButton rbtnVAC_F2 {'Tag': 2, 'Caption': 'REL', ...}
    TRadioButton rbtnVAC_F3 {'Tag': 3, 'Caption': 'Peak', ...}
```

Group-box `Tag` values are the key: 1 = Primary Mode, 2 = Secondary
Mode, 3 = Range, 4 = Rel entry. Both the mode-word builder and the
receive-side dialog refresh look controls up by those tags.

### Function map

| Address | Role |
|---------|------|
| `0x870408` | Generic sender `(conn, opcode, payload, len)` -- header, length, opcode, payload, checksum |
| `0x870298` | Checksum helper (byte sum, u16) |
| `0x8702b8` | **SET_MODE 0x01** -- u16 LE mode word |
| `0x8702d8` | **SET_RANGE 0x02** -- `is_auto ? 0 : index`, one byte |
| `0x8702f8` | DEL_SAVED_MEAS 0x09 -- u16 |
| `0x87030c` | DEL_RECORDING 0x0F -- u16 |
| `0x870344` | SET_MONITOR 0x05 -- one byte. The wrapper sends `arg == 0`, so its own parameter is inverted relative to the wire byte; the wire byte itself is 1 = stream, matching the hardware-verified `AB CD 04 00 05 01 0A 00` |
| `0x870364` | GET_SAVED_MEAS 0x07 -- u16 |
| `0x870378` | GET_SAVED_COUNT 0x08 -- no payload |
| `0x870384` | GET_REC_COUNT 0x0E -- no payload |
| `0x870390` | GET_REC_INFO 0x0C -- u16 |
| `0x8703a4` | **GET_REC_SAMPLES 0x0D** -- u16 index + u32 offset (6 bytes) |
| `0x8703e8` | **HOLD 0x12** -- single payload byte `0x5A` |
| `0x8703fc` | **SAVE_MEAS 0x06** -- no payload |
| `0x870588` | **SET_MIN_MAX 0x04** -- **one** byte |
| `0x87059c` | SET_REFERENCE 0x03 -- float32 (4 bytes) |
| `0x8705b0` | START_RECORDING 0x0A -- 17-byte payload: name, NUL at +9, u16 interval at +11, u32 duration at +13 |
| `0x86d31c` | `FormCreate` -- assigns the 20 tab-sheet base words |
| `0x86d700` | Mode-word builder: `ActivePage.Tag + (primary.Tag << 4) + secondary.Tag` |
| `0x86d72c` | Primary-radio click tail: caches the new primary nibble, re-selects the secondary radio |
| `0x86ceac` | `btnUpdate1Click` -- composes, compares with the live word, sends SET_MODE, `Sleep(100)` |
| `0x86cf0c` | `cbBoxRangeChange` -- sends SET_RANGE from the combo's `ItemIndex`, `Sleep(100)` |
| `0x86cfc0` | Receive-side dialog refresh: decomposes the meter's word into tab / primary / secondary |
| `0x86d1c4` | Receive-side range refresh: writes the meter's range byte into the combo's `ItemIndex` |
| `0x86d524` | Finds the active tab's range combo (group box `Tag == 3`) |
| `0x86cd0c` / `0x86cd18` / `0x86ce80` / `0x86ce90` / `0x86ce9c` | Hold, Max/Min toggle, Max/Min exit, Save, Restart actions |
| `0x86f2c4` | `tmrRelTimer` -- parses the REL edit box and sends SET_REFERENCE |
| `FUN_0085e69c` | Receive-side label decoder: switches on the mode word's high byte and `low & 0xF0` to build the record grid's "Pri" / "Sec" strings |

### What this settles

**Corrections.** One item where the vendor binary contradicts the
community specs outright:

- **SET_MIN_MAX takes one byte, not four.** `0x870588` pushes a payload
  length of 1, making the whole frame 8 bytes. antage and sigrok both
  describe a uint32.

**New -- not in any community source.** The opcodes themselves were
already documented; what was missing is how a host is meant to drive
them:

- **The composition rule.** `word = ActivePage.Tag + (primary.Tag << 4)
  + secondary.Tag`, with the receive side decomposing the same way. The
  nibble *encoding* was already in §6; what is new is that the vendor
  app treats the high byte as the dial position and therefore **never
  emits a word from a different family** -- so a host cannot change the
  measurement function over USB, only variants within the dial's own
  family.
- **Per-family primary variants, their on-meter captions, and which of
  them offer REL** (`reverse-engineered-protocol.md` §6.1). Community
  tables list mode words; they do not say which are siblings of which,
  nor that REL is withheld on every Hz and Peak variant.
- **Per-family manual range ladders**, that the SET_RANGE index is
  1-based into them, and that A DC, A AC, Celsius, Fahrenheit, Beeper,
  ns and Diode have **no** manual range at all (§7.1). The Duty and
  ms-Pulse ladders have no counterpart in the community range table.

**Independently re-sourced.** Previously [KNOWN] only as
agreement-between-implementations, now also read out of the vendor
binary:

- SET_MODE = `0x01` + u16 LE mode word (`0x8702b8`); SET_RANGE = `0x02`
  + one byte, `0` = auto (`0x8702d8`); SAVE_MEAS = `0x06` with no
  payload; GET_REC_SAMPLES = `0x0D` with u16 index + u32 offset; and the
  frame builder itself (`0x870408`).
- **HOLD's payload byte is `0x5A`**, hard-coded in `0x8703e8` -- the
  `[0x12, 0x5A]` form antage uses, previously the only implementation
  that transmitted 0x12 at all.
- Several mode rows the 2026-06 review had flagged: `0x3121` = V DC
  AC+DC (not Hz), `0x4121` = mV DC Peak (not sigrok's alternative
  `0x4131`), DC-current `n1 = 2` = AC+DC, and `0x5212` / `0x6112` as
  Beeper open-circuit and Diode alarm rather than REL variants.

### What remains open

- **Everything above is hardware-unverified.** The vendor app's
  behaviour is evidence about what UNI-T's own software sends, not
  proof the meter accepts it. `docs/verification-backlog.md` carries
  the asks.
- **The reply frame was not traced.** Type `0x01` with `"OK"` / `"ER"`
  stays community-sourced; how (or whether) the vendor app checks it
  after a SET_MODE was not followed through.
- **mV AC+DC (`0x2141`).** The vendor UI emits it, but its own label
  decoder has no case for family `0x21` with `n1 = 4`. Flagged
  [UNVERIFIED] in §6.1.
- **`n0 = 3`.** A third "Peak" secondary radio exists on every tab but
  is hidden or disabled everywhere, so the vendor app never emits a
  word ending in 3. Whether the meter would accept one is unknown.
  (One handler, `rbtnVAC_M6Click`, does not touch that radio at all --
  it relies on whichever handler ran before it having disabled it.)
- **No trace of COMP.** The `actComp` action is `Visible = False` in the
  form resource, so the vendor app ships COMP mode switched off in the
  UI and there is no call site to read.

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
