# UT8803/UT8803E Protocol: Reverse-Engineered Specification

Based on:
- UT8803E Programming Manual V1.1 (UNI-T)
- UT8803E User Manual (UNI-T)
- UNI-T SDK V2.3 (headers, examples, uci.dll)
- CP2110 Datasheet (Silicon Labs)
- AN434: CP2110/4 Interface Specification (Silicon Labs)

Confidence levels:
- **[KNOWN]** -- documented in official UNI-T programming manual
- **[VENDOR]** -- confirmed by analyzing UNI-T's official SDK/software
- **[DEDUCED]** -- logical inferences from available evidence
- **[UNVERIFIED]** -- requires real device testing to confirm

---

## 1. Transport Layer: CP2110 HID Bridge

### 1.1 Device Identification -- [KNOWN]

From the programming manual support model table:

| Parameter | Value | Source |
|-----------|-------|--------|
| USB VID | **0x10C4** | Programming manual |
| USB PID | **0xEA80** | Programming manual |
| USB Class | HID | Programming manual: `[T:HID]` |
| Bridge Chip | CP2110 | Same VID/PID as UT61E+ CP2110 |

UNI-T kept the Silicon Labs default VID/PID. The same VID/PID is used
by both the UT8802/UT8803 and the UT61E+. Device discrimination must
be done at the application layer.

### 1.2 HID Report Structure -- [KNOWN]

From AN434 (same as UT61E+):

**UART Data Transfer (Interrupt Transfers):**
- Report IDs 0x01 through 0x3F carry UART data
- Report ID = byte count (1-63 data bytes)
- Byte 0 = Report ID, bytes 1-63 = UART data

**Device Configuration (Feature Reports):**
- 0x41: Get/Set UART Enable
- 0x43: Set Purge FIFOs
- 0x50: Get/Set UART Config (baud rate, data bits, parity, stop bits)

### 1.3 UART Configuration -- [VENDOR]

The Ghidra decompilation of uci.dll reveals the exact UART configuration
sent to the CP2110 via feature report 0x50. In `FUN_1001d460` (the
CP2110 HID initialization function):

```c
// FUN_1001d460 — CP2110 UART init for UT8802/UT8803
local_24[0] = 0x141;                    // Feature report 0x41: enable UART (0x01)
HidD_SetFeature(param_2, local_24, 2);  // Send 2-byte enable report
local_20 = 0x25000050;                  // Bytes: 0x50, 0x00, 0x00, 0x25
local_1c = 0x3000080;                   // Bytes: 0x80, 0x00, 0x03, 0x00
local_18 = 0;                           // Byte:  0x00
HidD_SetFeature(param_2, &local_20, 9); // Send 9-byte UART config report
```

The feature report 0x50 bytes decode per AN434 as:

| Byte | Value | Meaning |
|------|-------|---------|
| 0 | 0x50 | Report ID: Set UART Config |
| 1-4 | 0x00 0x00 0x25 0x80 | Baud rate: 0x00002580 = **9600** (big-endian) |
| 5 | 0x00 | Parity: **None** |
| 6 | 0x03 | Flow control: No flow control (0x03) |
| 7 | 0x00 | Data bits: **8** (0x00 = 8 bits per AN434) |
| 8 | 0x00 | Stop bits: **1** (short stop bit) |

This is identical to the UT61E+ configuration. Additionally, after
configuring UART, the init function sends byte `0x5A` over the UART
via `FUN_1002a4d0(param_2, 0x5a, 1000)`. This appears to be a
**wake/trigger command** sent to the meter immediately after CP2110
initialization.

| Parameter | Value | Confidence |
|-----------|-------|------------|
| Baud rate | **9600 bps** | [VENDOR] |
| Data bits | 8 | [VENDOR] |
| Parity | None | [VENDOR] |
| Stop bits | 1 (short) | [VENDOR] |
| Flow control | None | [VENDOR] |

### 1.4 Initialization Sequence -- [VENDOR]

From `FUN_1001d460` in uci.dll (CP2110 HID path for UT8802/UT8803):

1. Open HID device matching VID 0x10C4, PID 0xEA80
2. Send feature report 0x41: Enable UART (`[0x41, 0x01]`)
3. Send feature report 0x50: Configure 9600/8N1 (`[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x03, 0x00, 0x00]`)
4. Send byte 0x5A over UART (wake/trigger command, 1000ms timeout)
5. Set HID input buffer count to 64 (`HidD_SetNumInputBuffers(0x40)`)

Note: There is a separate function `FUN_1001d2b0` that sends different
feature report values (`0x34b0000` for the report payload) — this
appears to be for the QinHeng HID models (VID 0x1A86), not the CP2110
models.

### 1.5 Related Models -- [KNOWN]

The programming manual groups these models by transport:

**CP2110 HID (VID 0x10C4, PID 0xEA80):**
- UT8802 / UT8802N
- UT8803 / UT8803N

**QinHeng HID (VID 0x1A86, PID 0xE008):**
- UT632 / UT632N
- UT803 / UT803N
- UT804 / UT804N

**Serial port (9600 8N1):**
- UT805A / UT805N

All share the UCI protocol layer and `data?;`/`disp?;` commands.

---

## 2. Application Protocol

### 2.1 UCI Abstraction Layer -- [KNOWN]

The UT8803 communicates through UNI-T's UCI (United Communication
Interface) SDK. Unlike the UT61E+ which has a bare binary protocol,
the UT8803's wire protocol is abstracted by the `uci.dll` library.

**API usage pattern** (from SDK examples and programming manual):

```c
// 1. Open connection
u_session session;
u_status r = uci_Open(
    "[C:DM][D:T8803][T:HID][PID:0xea80][VID:0x10c4]",
    &session, 2000);

// 2. Read display data (DMFRM struct)
DMFRM dfrm;
r = uci_ReadX(session, "disp?;", 2000,
    (unsigned char*)&dfrm, sizeof(dfrm));

// 3. Read raw measurement value (double)
double value;
r = uci_ReadX(session, "data?;", 2000,
    (unsigned char*)&value, sizeof(value));

// 4. Close
uci_Close(session);
```

### 2.2 Query Commands -- [KNOWN]

| Command | Response Type | Size | Description |
|---------|--------------|------|-------------|
| `data?;` | `double` | 8 bytes | Raw measurement value |
| `disp?;` | `struct DMFRM` | 56+ bytes | Display strings, values, and flags |

**`data?;`** returns only the numeric measurement value as an IEEE 754
double-precision float. The return value from `uci_ReadX` encodes the
status (>0 = status code with mode/range information).

**`disp?;`** returns the full display state including text
representation, numeric values, and a 64-bit flags word encoding mode,
range, unit, and status information.

### 2.3 Raw Wire Protocol -- [VENDOR]

The UCI library internally uses a framed binary protocol to communicate
with the meter over the CP2110 UART. Ghidra decompilation of uci.dll
revealed the UT8803 parser function (at address `0x1001e5f0`), which
fully specifies the wire protocol:

#### Frame Format

```
+------+------+----------+------+------------------+----------+----------+
| 0xAB | 0xCD | mode_hi  | 0x02 | payload (17 B)   | chk_hi   | chk_lo   |
+------+------+----------+------+------------------+----------+----------+
  byte 0 byte 1  byte 2   byte 3  bytes 4-20        byte 19    byte 20
```

- **Header**: `0xAB 0xCD` -- same as UT61E+ [VENDOR]
- **Byte 2**: High byte of the mode word. The parser reads bytes 2-3 as
  a 16-bit `short` value (`param_2[1]` where `param_2` is `short *`),
  but only byte 3 is checked (`== 0x02`). Byte 2 is part of the same
  word; its low byte contributes to `(byte)param_2[2]` which is byte
  offset 4 (the mode byte). The actual role of byte 2 itself is not
  directly consumed in parsing -- it is included in the checksum but
  its value is not independently validated. [VENDOR]
- **Byte 3**: Response type `0x02` (measurement data) [VENDOR]
- **Minimum frame size**: 0x15 = 21 bytes [VENDOR]
- **Checksum**: At bytes 19-20 (offsets 0x13-0x14) [VENDOR]

#### Model Detection

The frame parser discriminates UT8802 vs UT8803 by the first bytes of
the payload:

| First Byte(s) | Model | Evidence |
|---------------|-------|----------|
| `0xAC` | UT8802 | `*_Memory == -0x54` (line 25275) |
| `0xAB 0xCD` | UT8803 | `*_Memory == -0x55 && _Memory[1] == -0x33` (line 25290) |

The UT8803 response thus begins with `AB CD` as a header, while the
UT8802 uses a single-byte `0xAC` header. [VENDOR]

#### Checksum Algorithm

The UT8803 checksum is an **alternating-byte sum** -- different from
the UT61E+'s simple sequential sum:

```c
// Decompiled checksum (simplified):
ushort sum_even = 0;  // sum of even-offset bytes
ushort sum_odd = 0;   // sum of odd-offset bytes
for (int i = 0; i < 0x12; i += 2) {
    sum_even += data[i];      // bytes 0, 2, 4, 6, 8, 10, 12, 14, 16
    sum_odd  += data[i + 1];  // bytes 1, 3, 5, 7, 9, 11, 13, 15, 17
}
ushort extra = 0;
if (i < 0x13) extra = data[i];  // byte 18 if present

ushort computed = extra + sum_even + sum_odd;
ushort received = (data[0x13] << 8) | data[0x14];  // bytes 19-20 big-endian
if (computed != received) → checksum error
```

The checksum covers bytes 0 through 18 (19 bytes), stored big-endian
at bytes 19-20. [VENDOR]

#### Payload Layout

From the parser at `0x1001e5f0` (decompiled UT8803 parser):

| Offset | Field | Notes |
|--------|-------|-------|
| 0-1 | Header | `0xAB 0xCD` |
| 2 | (frame byte) | Part of `param_2[1]` word; not independently consumed |
| 3 | Type | `0x02` for measurement response |
| 4 | Mode | `(byte)param_2[2]` → mode byte (raw, max 0x16) |
| 5 | Range | `*(byte*)((int)param_2 + 5)` → has 0x30 prefix (mask: `- 0x30`, max 6) |
| 6 | (included in checksum) | Not directly accessed by the parser. Included in the alternating-byte checksum sum but no field extraction. Likely reserved or padding. |
| 7-11 | Display | 5 raw bytes, appended to string buffer via `FUN_1001fce0` (buffer append, no transformation) |
| 12-13 | Flags0 | Part of `param_2[6]` (bytes 12-13 as a 16-bit word). Byte 12 low bits: unknown purpose. Byte 13 (`*(byte*)((int)param_2 + 0x11)`): combined with byte 16 to form a 9-bit field for inductance test frequency and other flags |
| 14-15 | Flags1 | `param_2[7]` (bytes 14-15): bit 0 = flag_27, bit 1 = unused, bit 2 = flag_1e, bit 3 = flag_1d; high byte provides additional flag bits |
| 16-17 | Flags2 | `param_2[8]` (bytes 16-17): bit 2+ = flag_23 (2-bit field), bit 1 = flag_29, bit 0 combined with byte 13 for 9-bit inductance field |
| 18 | Flags3 | `(char)param_2[9]`: bit 0 = flag_28, bit 1 = flag_15; high part provides flag_25 |
| 19-20 | Checksum | 16-bit BE, alternating-byte sum |

**Mode byte values**: `0x00` through `0x16` (max 22 values), checked
at line 25014: `if (0x16 < bVar1)` → reject. This corresponds to the
23-entry UT8803 position coding table. [VENDOR]

**Range byte**: Has a `0x30` prefix, masked with `- 0x30`, max value
6 after masking. Same encoding as UT61E+. [VENDOR]

**Display value parsing**: The function `FUN_1001fce0` is a **buffer
append** function, not a byte transformation. It appends raw bytes to
a dynamically-growing buffer (checking capacity, reallocating via
`FUN_1001a5a0` if needed). The display bytes at offsets 7-11 are
copied verbatim into the string buffer. After accumulation,
`FUN_1017f410` converts the string to a float value. The display bytes
are therefore **raw values** (not ASCII with 0x30 prefix like the
range byte). [VENDOR]

**Flag bit construction**: The parser builds a 32-bit status word
(at `param_4 + 0x14`) by combining extracted bits from the flag bytes.
The format string at line 25091 confirms the bit layout:
```
"ACDC:%s,dotpos:%d,fun:%d,isauto:%d,ismax:%d,ismin:%d,ishold:%d,isrel:%d,isOL:%d"
```

The debug format string extracts these fields from the status word:
- `fun` = bits 0-3 (`uVar9 & 0xf`) — functional coding
- `ACDC` = bits 4-5 (`uVar9 >> 4 & 3`) — AC/DC status
- `isauto` = bit 6 (`uVar9 >> 6 & 1`) — auto range
- `isOL` = bit 7 (`uVar9 >> 7 & 1`) — overload
- `dotpos` = bits 24-27 (`uVar9 >> 0x18 & 0xf`) — decimal point position
- `ismax` = bit 28 (`uVar9 >> 0x1c & 1`) — MAX mode
- `ismin` = bit 29 (`uVar9 >> 0x1d & 1`) — MIN mode
- `isrel` = bit 30 (`uVar9 >> 0x1e & 1`) — REL mode
- `ishold` = bit 31 (`uVar9 >> 0x1f`) — HOLD mode

This exactly matches the Flags word layout documented in the
programming manual (section 3.2). [VENDOR]

**High-byte flags (param_4 + 3)**: The parser also constructs a
separate byte at `param_4 + 0x18` (offset 24 from param_4 base):
```c
*(uint *)(param_4 + 3) =
    (((((((bVar1==0x10)*2 | (bVar1==0xf))*2 | (bVar1==0xd))*2 |
       (bVar1==0xc))*2 | local_15)*2 | local_25)*2 | local_20)*2 |
    local_26 ...
```
This encodes mode-specific flags by checking if the mode byte equals
specific inductance/capacitance sub-measurement modes (0x0C=IndQ,
0x0D=IndR, 0x0F=CapD, 0x10=CapR), corresponding to the high 32-bit
flags D4-D7 (IndQ, IndR, CapD, CapR) documented in the programming
manual. [VENDOR]

#### Inductance Test Frequency

The parser checks for inductance modes (0x0B through 0x10) and
extracts a 2-bit field determining the test frequency:
- `0` → "100Hz"
- `1` → "1kHz"

This corresponds to the UT8803's inductance test at 100Hz and 1kHz
frequencies. [VENDOR]

### 2.4 Command Encoding (Host → Meter) -- [VENDOR]

The Ghidra decompilation reveals that the UT8803 does **not** use
text-based SCPI commands on the wire. The UCI library translates the
text API commands (`data?;`, `disp?;`) into a binary polling
mechanism:

**Initialization trigger**: After CP2110 UART setup, the library sends
a single byte `0x5A` to the meter via `FUN_1002a4d0(handle, 0x5a, 1000)`.
This call uses `FUN_1002a500`, which calls `WriteFile` to write the byte
to the HID device. The 1000ms timeout suggests this is a blocking
handshake command.

**Read loop**: The `FUN_1001f170` function implements the frame read
loop. It calls `FUN_1002a380` which calls `FUN_1002a260` to perform
HID reads (with 0x32 = 50ms individual timeouts). The function reads
chunks from the HID device, accumulates them in a buffer, then calls
the parser (`FUN_1001e5f0` for UT8803, `FUN_1001e0a0` for UT8802)
when enough data has been received. There is **no evidence of the
host sending a per-measurement request command** — the meter appears
to **stream data continuously** after the initial 0x5A trigger.

**Frame dispatch**: The function `FUN_1001eb30` is the frame
discriminator. It scans the received HID data (which arrives in 64-byte
HID reports), reassembles complete frames by looking for `0xAB 0xCD`
(UT8803) or `0xAC` (UT8802) headers, then dispatches to the appropriate
parser.

**The `:DISPlay:DATA?` string**: This string appears in the
`PLAIN-TEXT` transport handler (`FUN_10038300` area, line 45478), which
is used for text-based instruments (oscilloscopes, signal generators).
The DMM parser path (`FUN_1001e5f0`) does not use SCPI commands.

| Aspect | Details | Confidence |
|--------|---------|------------|
| Trigger command | Single byte `0x5A` sent after UART init | [VENDOR] |
| Communication model | Meter streams continuously after trigger | [VENDOR] |
| Frame reassembly | HID reports concatenated, header-delimited | [VENDOR] |
| Per-measurement command | None — no request per sample | [VENDOR] |

### 2.5 Communication Model -- [VENDOR]

The UT8803 uses a **streaming** model, not a polled model:

1. Host opens HID device and configures CP2110 UART (9600/8N1)
2. Host sends byte `0x5A` to trigger measurement streaming
3. Meter begins sending 21-byte measurement frames continuously
4. UCI library's `FUN_1001f170` reads HID reports in a loop,
   reassembles frames, and parses each complete frame
5. The `uci_ReadX("data?;")` and `uci_ReadX("disp?;")` calls simply
   read the most recently parsed measurement from internal state

This differs from the UT61E+ polled model where each measurement
requires a request command (`AB CD 03 5E 01 D9`). The UT8803 streams
at approximately 2-3 Hz (matching the user manual's stated refresh
rate).

---

## 3. Response Data Structures -- [KNOWN]

### 3.1 DMFRM Structure (from `disp?;`)

```c
struct DMFRM {
    TCHAR MainDisp[20];       // Main display character string (20 chars)
    TCHAR AuxDisp[20];        // Secondary display character string (20 chars)
    double MainValue;         // Main display numeric value (8 bytes, IEEE 754)
    double AuxValue;          // Secondary display numeric value (8 bytes)
    unsigned long long Flags; // Flag bits (8 bytes, 64 bits)
};
```

**Size**: In ASCII mode, `TCHAR = char` (1 byte), total = 20+20+8+8+8 =
64 bytes. In Unicode mode, `TCHAR = wchar_t` (2 bytes), total =
40+40+8+8+8 = 104 bytes.

**Note**: The UT805A/UT805N buffer area can be set to 8 or 16 bytes
for main and secondary display. Other models use 1 double (8 bytes)
for `data?;`.

### 3.2 Flags Word Layout -- [KNOWN]

The 64-bit `Flags` field encodes all status information. Bit extraction
uses the macro:
```c
#define Bits(_status, _offset, _mask) ((_status >> _offset) & _mask)
```

#### Low 32 Bits (Common to All Models)

| Bits | Mask | Field | Description |
|------|------|-------|-------------|
| D0-D3 | 0xF | FuncCode | Functional coding (mode category) |
| D4-D5 | 0x3 | ACDC | AC&DC status: 0=OFF, 1=AC, 2=DC, 3=AC+DC |
| D6 | 0x1 | AutoRange | 1=Auto range active |
| D7 | 0x1 | OverLoad | 1=Overload (OL) |
| D8-D11 | 0xF | UnitType | Physical unit type (V/A/ohm/Hz/C/F/F/hFE/%) |
| D12-D14 | 0x7 | UnitMag | Physical unit magnitude (n/u/m/std/K/M/G) |
| D15 | 0x1 | LowBat | 1=Low battery |
| D16 | 0x1 | USBComm | 1=USB communication active |
| D17 | 0x1 | Under | 1=Under range |
| D18 | 0x1 | Over | 1=Over range |
| D19 | 0x1 | Minus | 1=Display shows minus sign |
| D20-D23 | 0xF | Position | Position coding (model-specific) |
| D24-D27 | 0xF | ScalePos | Scaling position (starts from 1) |
| D28 | 0x1 | MAX | 1=MAX mode active (UT8802/UT8803) |
| D29 | 0x1 | MIN | 1=MIN mode active (UT8802/UT8803) |
| D30 | 0x1 | REL | 1=REL (relative) mode active (UT8802/UT8803) |
| D31 | 0x1 | HOLD | 1=Data hold active (UT8802/UT8803) |

#### High 32 Bits (UT8802/UT8803 Only)

| Bits | Mask | Field | Description |
|------|------|-------|-------------|
| D0 | 0x1 | Error | 1=Current data error or display error |
| D1 | 0x1 | TestMode | 1=Serial (SEL), 0=Parallel (PAL) |
| D2 | 0x1 | DiodeRL | 1=Diode/thyristor direction right-to-left valid |
| D3 | 0x1 | DiodeLR | 1=Diode/thyristor direction left-to-right valid |
| D4 | 0x1 | IndQ | 1=Inductance quality element (Q) measurement |
| D5 | 0x1 | IndR | 1=Equivalent resistance measurement |
| D6 | 0x1 | CapD | 1=Capacitance loss element (D) measurement |
| D7 | 0x1 | CapR | 1=Capacitance equivalent resistance measurement |
| D8-D15 | 0xFF | FuncPos | Functional position coding (UT8803-specific) |
| D16-D31 | 0xFFFF | Hold2 | Extended hold field |

---

## 4. Coding Tables -- [KNOWN]

### 4.1 Functional Coding (Common, D0-D3)

These abstract measurement categories apply to all models:

| Code | Function | Notes |
|------|----------|-------|
| 0 | Voltage | Combined with ACDC bits for DCV/ACV/ACV+DCV |
| 1 | Resistance (OHM) | |
| 2 | Diode | |
| 3 | Continuity | |
| 4 | Capacitance | |
| 5 | Frequency (FREQ) | |
| 6 | Temperature Fahrenheit | |
| 7 | Temperature Centigrade | |
| 8 | Triode hFE | |
| 9 | Current | Combined with ACDC bits for DCI/ACI |
| 10 | % (4-20mA) | |
| 11 | Duty ratio | |
| 12 | Thyristor (SCR) | |
| 13 | Inductance | Including Q and R sub-measurements |

**Resolving AC/DC variants**: The functional coding does not
distinguish between AC and DC for voltage and current. Use the ACDC
field (D4-D5) to determine:
- DCV: FuncCode=0 (Voltage), ACDC=2 (DC)
- ACV: FuncCode=0 (Voltage), ACDC=1 (AC)
- ACV+DCV: FuncCode=0 (Voltage), ACDC=3 (AC+DC)
- DCI: FuncCode=9 (Current), ACDC=2 (DC)
- ACI: FuncCode=9 (Current), ACDC=1 (AC)

### 4.2 UT8803/UT8803N Position Coding (High D8-D15)

These model-specific position codes identify the exact measurement
mode and range position on the dial. Available only from the `disp?;`
command's high 32 bits.

| Code | Position |
|------|----------|
| 0 | AC voltage (ACV) |
| 1 | DC voltage (DCV) |
| 2 | AC current microampere (ACuA) |
| 3 | AC current milliampere (AC mA) |
| 4 | AC current standard (AC A) |
| 5 | DC current microampere (DCuA) |
| 6 | DC current milliampere (DC mA) |
| 7 | DC current standard (DC A) |
| 8 | Resistance (OHM) |
| 9 | Continuity |
| 10 | Diode (DIODE) |
| 11 | Inductance (L) |
| 12 | Inductance quality factor (Q) |
| 13 | Inductance equivalent resistance (R) |
| 14 | Capacitance (C) |
| 15 | Capacitance loss factor (D) |
| 16 | Capacitance equivalent resistance (R) |
| 17 | Triode hFE |
| 18 | Thyristor (SCR) |
| 19 | Temperature Centigrade |
| 20 | Temperature Fahrenheit |
| 21 | Frequency (FREQ) |
| 22 | Duty ratio |

### 4.3 UT8802/UT8802N Position Coding (High D8-D15)

For comparison, the UT8802 uses different codes that encode both
function and range:

| Code | Position |
|------|----------|
| 0x01 | DC voltage 200mV |
| 0x03 | DC voltage 2V |
| 0x04 | DC voltage 20V |
| 0x05 | DC voltage 200V |
| 0x06 | DC voltage 1000V |
| 0x09 | AC voltage 2V |
| 0x0A | AC voltage 20V |
| 0x0B | AC voltage 200V |
| 0x0C | AC voltage 750V |
| 0x0D | DC current microampere 200uA |
| 0x0E | DC current milliampere 2mA |
| 0x10 | AC current milliampere 2mA |
| 0x11 | DC current milliampere 20mA |
| 0x12 | DC current milliampere 200mA |
| 0x13 | AC current milliampere 20mA |
| 0x14 | AC current milliampere 200mA |
| 0x16 | DC current standard 2A |
| 0x18 | AC current standard 20A |
| 0x19 | Resistance 200 ohm |
| 0x1A | Resistance 2k ohm |
| 0x1B | Resistance 20k ohm |
| 0x1C | Resistance 200k ohm |
| 0x1D | Resistance 2M ohm |
| 0x1F | Resistance 200M ohm |
| 0x22 | Duty |
| 0x23 | Diode |
| 0x24 | Continuity |
| 0x25 | hFE |
| 0x27 | Capacitance nF |
| 0x28 | Capacitance uF |
| 0x29 | Capacitance mF |
| 0x2A | SCR |
| 0x2B | Frequency Hz |
| 0x2C | Frequency kHz |
| 0x2D | Frequency MHz |

### 4.4 Physical Unit Type (D8-D11)

| Code | Unit | Symbol |
|------|------|--------|
| 0 | Voltage | V |
| 1 | Current | A |
| 2 | Resistance | ohm |
| 3 | Frequency | Hz |
| 4 | Centigrade | C |
| 5 | Fahrenheit | F |
| 6 | RPM(rpm)hold | rpm |
| 7 | Capacitance | F |
| 8 | Triode hFE | beta |
| 9 | Percentage | % |
| 0xF | No display | (none) |

### 4.5 Physical Unit Magnitude (D12-D14)

| Code | Prefix | Multiplier |
|------|--------|-----------|
| 0 | n (nano) | 10^-9 |
| 1 | u (micro) | 10^-6 |
| 2 | m (milli) | 10^-3 |
| 3 | (standard) | 1 |
| 4 | K (kilo) | 10^3 |
| 5 | M (mega) | 10^6 |
| 6 | G (giga) | 10^9 |

### 4.6 AC/DC Status (D4-D5)

| Code | Status |
|------|--------|
| 0 | OFF |
| 1 | AC |
| 2 | DC |
| 3 | AC+DC |

---

## 5. Measurement Ranges (from User Manual) -- [KNOWN]

### DC Voltage
| Range | Resolution | Full Scale |
|-------|-----------|-----------|
| 600mV | 0.1mV | 5999 |
| 6V | 1mV | 5999 |
| 60V | 10mV | 5999 |
| 600V | 100mV | 5999 |
| 1000V | 1V | 1000 |

### AC Voltage
| Range | Resolution | Max |
|-------|-----------|-----|
| 600mV | 0.1mV | 600 |
| 6V | 1mV | 6 |
| 60V | 10mV | 60 |
| 600V | 100mV | 600 |
| 750V | 1V | 750 |

### DC Current
| Range | Resolution |
|-------|-----------|
| 600uA | 0.1uA |
| 6mA | 1uA |
| 60mA | 10uA |
| 600mA | 100uA |
| 10A | 10mA |

### AC Current
| Range | Resolution |
|-------|-----------|
| 600uA-6mA | 0.1-10uA |
| 60mA-600mA | 100uA |
| 10A | 10mA |

### Resistance
| Range | Resolution |
|-------|-----------|
| 600 ohm | 0.1 ohm |
| 6k ohm | 1 ohm |
| 60k ohm | 10 ohm |
| 600k ohm | 100 ohm |
| 6M ohm | 1k ohm |
| 60M ohm | 10k ohm |

### Capacitance
| Range | Resolution |
|-------|-----------|
| 6nF | 1pF |
| 60nF | 10pF |
| 600nF | 100pF |
| 6uF | 1nF |
| 60uF | 10nF |
| 600uF | 100nF |
| 6mF | 1uF |

### Inductance
| Range | Resolution |
|-------|-----------|
| 600uH | 0.1uH |
| 6mH | 1uH |
| 60mH | 10uH |
| 600mH | 100uH |
| 6H | 1mH |
| 60H | 10mH |
| 100H | 100mH |

### Frequency
| Range | Resolution |
|-------|-----------|
| 600Hz | 0.1Hz |
| 6kHz | 1Hz |
| 60kHz | 10Hz |
| 600kHz | 100Hz |
| 6MHz | 1kHz |
| 20MHz | 10kHz |

### Temperature
| Range | Resolution |
|-------|-----------|
| -40C to 1000C | 1C |
| -40F to 1832F | 1F |

---

## 6. Key Differences from UT61E+ Protocol

| Aspect | UT61E+ | UT8803 |
|--------|--------|--------|
| **Protocol abstraction** | Raw binary protocol | UCI SDK layer over binary protocol |
| **USB bridge** | CP2110 (0x10C4/0xEA80) | CP2110 (same VID/PID) |
| **Command format** | Binary: `AB CD 03 cmd chk_hi chk_lo` | Single byte `0x5A` trigger, then streaming |
| **Response format** | 19-byte binary frame | DMFRM struct (64-104 bytes) or 8-byte double |
| **Mode encoding** | Single mode byte (0x00-0x19) | 4-bit FuncCode + 2-bit ACDC + 8-bit Position |
| **Range encoding** | Single byte with 0x30 prefix | Embedded in Position code or separate |
| **Flag encoding** | 3 separate bytes (3 bits each) | Single 64-bit word with documented bit fields |
| **Display value** | 7 ASCII bytes, parse as float | 20-char string + IEEE 754 double |
| **Meter type** | Handheld (22000 counts) | Bench (6000 counts) |
| **Extra features** | NCV, LoZ, LPF, Peak Min/Max | Inductance L/Q/R, Capacitance C/D/R, SCR, SER/PAL |
| **Software model** | Qt app with plugin DLLs | UCI SDK-based application |
| **Communication model** | Polled (1 request per measurement) | Streaming (continuous after 0x5A trigger) |
| **Baud rate** | 9600 (confirmed) | 9600 (confirmed from uci.dll feature report 0x50) |

---

## 7. Practical Implementation Considerations

### 7.1 Without UCI SDK (Linux/Cross-platform)

The raw wire protocol has been sufficiently reverse-engineered from
the uci.dll decompilation to implement a direct driver:

**Implementation steps:**
1. Open HID device matching VID 0x10C4, PID 0xEA80
2. Send feature report 0x41: Enable UART (`[0x41, 0x01]`)
3. Send feature report 0x50: Configure 9600/8N1
   (`[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x03, 0x00, 0x00]`)
4. Send HID interrupt report with 1 data byte: `0x5A` (trigger)
5. Read HID interrupt reports continuously
6. Reassemble 21-byte frames by finding `0xAB 0xCD` headers
7. Validate checksum (alternating-byte sum, BE at bytes 19-20)
8. Parse mode (byte 4), range (byte 5 - 0x30), display (bytes 7-11),
   and flags (bytes 14-18)

**Note**: The `:DISPlay:DATA?` string in uci.dll is used by the
SCPI/text transport handler for oscilloscopes and signal generators,
**not** by the DMM binary parser. The UT8803 DMM uses a pure binary
protocol.

### 7.2 Flag Decoding Example

Given `flags = 0x2013345` (from programming manual example, 50.23 Hz):

```
Low 32 bits: 0x02013345
D0-D3  = 0x5 = Frequency
D4-D5  = 0x0 = OFF (no AC/DC for frequency)
D6     = 0x0 = Not auto range (manual shows 0 here)
D7     = 0x0 = No overload
D8-D11 = 0x3 = Hz (Frequency)
D12-D14 = 0x3 = standard (no prefix)
D15    = 0x0 = Battery OK
D16    = 0x1 = USB communication active
D17    = 0x0 = Not under
D18    = 0x0 = Not over
D19    = 0x0 = No minus sign
D20-D23 = 0x0 = Position 0
D24-D27 = 0x2 = Scaling position 2
```

---

## 8. Summary of Confidence Levels

| Aspect | Status | Source |
|--------|--------|--------|
| VID 0x10C4, PID 0xEA80 | **[KNOWN]** | Programming manual |
| USB HID interface | **[KNOWN]** | Programming manual |
| UCI SDK communication model | **[KNOWN]** | Programming manual + SDK |
| `data?;` command (returns double) | **[KNOWN]** | Programming manual |
| `disp?;` command (returns DMFRM) | **[KNOWN]** | Programming manual |
| DMFRM struct layout | **[KNOWN]** | Programming manual |
| 64-bit Flags word (all bit fields) | **[KNOWN]** | Programming manual |
| Functional coding table (14 functions) | **[KNOWN]** | Programming manual |
| UT8803 position coding (23 positions) | **[KNOWN]** | Programming manual |
| UT8802 position coding (range-specific) | **[KNOWN]** | Programming manual |
| Physical unit type/magnitude tables | **[KNOWN]** | Programming manual |
| AC/DC status coding | **[KNOWN]** | Programming manual |
| Models sharing protocol | **[KNOWN]** | Programming manual |
| UT805A uses serial (not HID) | **[KNOWN]** | Programming manual |
| Display: 6000 counts (5999 max) | **[KNOWN]** | User manual |
| All measurement ranges | **[KNOWN]** | User manual |
| Separate UT8802/UT8803 parsers in uci.dll | **[VENDOR]** | String extraction |
| Checksummed framing under UCI | **[VENDOR]** | String: `Data check sum error!` |
| Frame-based transport | **[VENDOR]** | String: `[Frame]` messages |
| HID API used directly (HidD_*) | **[VENDOR]** | String extraction |
| Frame header: AB CD (same as UT61E+) | **[VENDOR]** | Ghidra: parser condition `*param_2 != -0x3255` |
| Byte 3 = 0x02 for measurement response | **[VENDOR]** | Ghidra: `byte[3] != '\x02'` |
| Frame size: 21 bytes | **[VENDOR]** | Ghidra: `param_3 < 0x15` |
| Checksum: alternating-byte sum, BE at bytes 19-20 | **[VENDOR]** | Ghidra: checksum loop |
| Mode byte at offset 4, raw (0x00-0x16) | **[VENDOR]** | Ghidra: `bVar1`, `0x16 < bVar1` |
| Range byte at offset 5, 0x30 prefix | **[VENDOR]** | Ghidra: `bVar5 - 0x30, 6 < bVar5` |
| UT8802 uses different header (0xAC, 8-byte frames) | **[VENDOR]** | Ghidra: discriminator |
| Inductance test freq: 0=100Hz, 1=1kHz | **[VENDOR]** | Ghidra: mode 0x0B-0x10 branch |
| Baud rate 9600 | **[VENDOR]** | Ghidra: feature report 0x50 with 0x00002580 |
| CP2110 init sequence (full) | **[VENDOR]** | Ghidra: `FUN_1001d460` — 0x41, 0x50, 0x5A sequence |
| Trigger command: 0x5A byte | **[VENDOR]** | Ghidra: `FUN_1002a4d0(param_2, 0x5a, 1000)` |
| Streaming model (not polled) | **[VENDOR]** | Ghidra: read loop with no per-measurement send |
| Display bytes: raw passthrough (no 0x30 prefix) | **[VENDOR]** | Ghidra: `FUN_1001fce0` is buffer append |
| Flag byte-to-status-word mapping | **[VENDOR]** | Ghidra: bit shifts + format string verification |
| Byte 6: unused by parser (reserved/padding) | **[VENDOR]** | Ghidra: no access to byte offset 6 |
| Bytes 12-13: flag/inductance field source | **[VENDOR]** | Ghidra: `param_2[6]` combined with byte 17 |
| `:DISPlay:DATA?` is for oscilloscopes, not DMMs | **[VENDOR]** | Ghidra: only in PLAIN-TEXT handler path |
| Maximum sampling rate | **[UNVERIFIED]** | Manual says 2-3 Hz refresh |

---

## 9. Answers to Key Questions

### Q1: What is the command encoding (host to meter)?

**Answer**: The UT8803 does not use per-measurement command encoding.
Instead:
- The host sends a single byte `0x5A` over UART after CP2110 init
  to trigger continuous streaming
- The meter then sends 21-byte binary frames at ~2-3 Hz
- The UCI library reads these frames in a loop; `uci_ReadX("data?;")`
  and `uci_ReadX("disp?;")` return the most recent parsed measurement
- There is no SCPI text protocol for DMMs — the `:DISPlay:DATA?`
  string in uci.dll is part of the oscilloscope/signal generator
  handler, not the DMM path
[VENDOR]

### Q2: Is the meter streaming continuously or polled?

**Answer**: The meter **streams continuously** after receiving the
0x5A trigger byte. The Ghidra decompilation shows:
- `FUN_1001d460` sends 0x5A immediately after UART configuration
- `FUN_1001f170` implements a read loop calling `FUN_1002a380`
  (which calls HID read) repeatedly, without sending any data
  between reads
- The loop accumulates bytes and calls the parser when a complete
  frame is available
- There is no send operation in the read loop — only receives
The UCI API presents this as a polled interface (`uci_ReadX` with
timeout), but the underlying transport is streaming. [VENDOR]

### Q3: What baud rate?

**Answer**: **9600 baud**, confirmed from the Ghidra decompilation.
The function `FUN_1001d460` constructs CP2110 feature report 0x50
with bytes `[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x03, 0x00, 0x00]`.
Bytes 1-4 encode the baud rate as 0x00002580 = 9600 in big-endian,
matching the AN434 specification for CP2110 UART configuration.
The rest specifies 8N1 with no flow control. [VENDOR]

### Q4: What is the response payload layout?

**Answer**: Two levels:

**UCI API level** (documented in programming manual):
- `data?;`: 8-byte IEEE 754 double (raw measurement value)
- `disp?;`: DMFRM struct (MainDisp[20] + AuxDisp[20] + MainValue(8) +
  AuxValue(8) + Flags(8) = 64 bytes in ASCII mode)

**Raw wire protocol** (from Ghidra decompilation of uci.dll):
- 21-byte frames: `AB CD [byte2] 02 [mode] [range] [byte6] [disp x5] [flags0 x2] [flags1 x2] [flags2 x2] [flags3] [chk_hi] [chk_lo]`
- Mode byte at offset 4 (raw, 0x00-0x16)
- Range byte at offset 5 (has 0x30 prefix, mask with `- 0x30`, max 6)
- Byte 6: not accessed by parser (reserved/padding, included in checksum)
- Display bytes at offsets 7-11 (raw values, appended directly to
  string buffer — no 0x30 prefix stripping)
- Bytes 12-13: flag source combined with byte 17 for inductance
  test frequency (2-bit field) and other flags
- Bytes 14-15: primary flag byte pair (AC/DC, auto, hold, etc.)
- Bytes 16-17: secondary flags (overload indicators)
- Byte 18: tertiary flags
- Alternating-byte checksum at offsets 19-20

The UCI library parses these raw 21-byte frames and constructs the
DMFRM struct and double value returned to the application. [VENDOR]

### Q5: Which models share this protocol?

**Answer**: All models in the programming manual share the UCI protocol
layer: UT632, UT803, UT804, UT805A, UT8802, UT8803 (and their N
variants). They differ in:
- Transport: HID (0x1A86 for UT632/803/804, 0x10C4 for UT8802/8803)
  vs serial (UT805A)
- Position coding tables (model-specific)
- High 32 bits of Flags (UT8802/UT8803 only)
- Feature sets (D/Q/R sub-measurements only on UT8803)

[KNOWN]
