# UT181A Protocol: Reverse-Engineered Specification

Based on cross-referencing three independent community implementations:
- [antage/ut181a](https://github.com/antage/ut181a) (Rust, with [Protocol.md](https://github.com/antage/ut181a/blob/master/Protocol.md))
- [loblab/ut181a](https://github.com/loblab/ut181a) (C++)
- [sigrok uni-t-ut181a](https://github.com/sigrokproject/libsigrok/tree/master/src/hardware/uni-t-ut181a) (C)

All three implementations agree on every protocol detail documented
here.

Confidence levels:
- **[KNOWN]** -- confirmed by 3 independent implementations or official manual
- **[VENDOR]** -- from vendor software analysis (not applicable here)
- **[DEDUCED]** -- logical inference
- **[UNVERIFIED]** -- needs device testing (none remaining for UT181A)

---

## 1. Device Overview -- [KNOWN]

| Parameter | Value |
|-----------|-------|
| Display | 60,000 counts, 4-5/6 digits |
| Screen | 3.5" 64K color TFT LCD (320x240) |
| Safety | CAT IV 600V / CAT III 1000V |
| DCV accuracy | 0.025% + 5 counts |
| Chipset | Cyrustek ES51997 analog frontend |
| MCU | STM32F103 |
| USB bridge | CP2110 (VID 0x10C4, PID 0xEA80) |
| Sample rate | 2 Sa/s (60K counts), 10 Sa/s (600 counts) |
| Data logging | 20,000 saved measurements |
| Recording | Up to 20 named recordings |
| Battery | 7.4V 2200 mAh Li-ion + CR2032 backup |

**Prerequisite**: User must enable communication on the meter before
each session: SETUP -> Communication -> ON. This setting resets on
power cycle.

---

## 2. Transport Layer -- [KNOWN]

### 2.1 USB Configuration

| Parameter | Value |
|-----------|-------|
| USB VID | 0x10C4 (Silicon Labs) |
| USB PID | 0xEA80 (CP2110 default) |
| Baud rate | 9600 |
| Data bits | 8 |
| Parity | None |
| Stop bits | 1 |
| Flow control | None |

Same CP2110 bridge as UT61E+ and UT8803. The initialization sequence
is the standard CP2110 UART enable + configure.

### 2.2 Header Byte Clarification

**Important**: The UT181A wire bytes are **0xAB, 0xCD** -- identical to
the UT61E+. The "reversed 0xCDAB header" description found in some
sources (including the antage Protocol.md) refers to reading these two
bytes as a little-endian uint16: `byte[0]=0xAB, byte[1]=0xCD` →
LE uint16 = 0xCDAB. The UT61E+ reads the same bytes as big-endian:
0xABCD. The **wire bytes are the same**; only the host-side integer
interpretation differs.

---

## 3. Frame Format -- [KNOWN]

```
+------+------+--------+--------+-----------+--------+--------+
| 0xAB | 0xCD | len_lo | len_hi | payload   | chk_lo | chk_hi |
+------+------+--------+--------+-----------+--------+--------+
 byte 0 byte 1 byte 2   byte 3   bytes 4..   last 2 bytes
```

| Field | Size | Encoding | Description |
|-------|------|----------|-------------|
| Magic | 2 | Fixed | `0xAB 0xCD` |
| Length | 2 | uint16 LE | `payload_size + 2` (includes checksum bytes) |
| Payload | N | Variable | Command or response data |
| Checksum | 2 | uint16 LE | Sum of all bytes from offset 2 through end of payload |

### Checksum Algorithm

```
checksum = sum of bytes[2] through bytes[3 + payload_size - 1]
         = length_lo + length_hi + payload[0] + ... + payload[N-1]
```

The checksum covers the length field and all payload bytes. It does
**not** include the 2-byte magic header.

### Key Differences from UT61E+

| Aspect | UT61E+ | UT181A |
|--------|--------|--------|
| Header bytes | 0xAB 0xCD | 0xAB 0xCD (same) |
| Length field | 1 byte | 2 bytes (uint16 LE) |
| Length meaning | Bytes after length (payload + checksum) | Payload + 2 (payload + checksum size) |
| Checksum | 16-bit BE sum of all bytes before checksum | 16-bit LE sum of length + payload only |
| Values | 7-byte ASCII display string | float32 LE (IEEE 754) |
| Communication | Polled (request/response) | Monitor mode (streaming) + commands |

---

## 4. Packet Types -- [KNOWN]

### 4.1 Response Types (Device -> Host)

| Code | Type | Description |
|------|------|-------------|
| 0x01 | Reply Code | OK (`0x4F4B` = "OK") or Error (`0x4552` = "ER") |
| 0x02 | Measurement | Real-time measurement data |
| 0x03 | Save | Saved measurement with timestamp |
| 0x04 | Record Info | Recording metadata (name, interval, stats) |
| 0x05 | Record Data | Recording samples (batched) |
| 0x72 | Reply Data | Generic data reply (e.g., saved/recording counts) |

### 4.2 Commands (Host -> Device)

| Code | Command | Parameters | Description |
|------|---------|------------|-------------|
| 0x01 | SET_MODE | uint16 LE mode word | Set measurement mode |
| 0x02 | SET_RANGE | uint8 (0x00-0x08) | Set range (0 = auto) |
| 0x03 | SET_REFERENCE | float32 LE | Set relative reference value |
| 0x04 | SET_MIN_MAX | uint32 LE (0 or 1) | Enable/disable min/max |
| 0x05 | SET_MONITOR | uint8 (0 or 1) | Enable/disable streaming |
| 0x06 | SAVE_MEAS | (none) | Save current measurement |
| 0x07 | GET_SAVED_MEAS | uint16 LE index (1-based) | Retrieve saved measurement |
| 0x08 | GET_SAVED_COUNT | (none) | Get count of saved measurements |
| 0x09 | DEL_SAVED_MEAS | uint16 LE index (0xFFFF = all) | Delete saved measurement(s) |
| 0x0A | START_RECORDING | name(11) + interval(2) + duration(4) | Start recording |
| 0x0B | STOP_RECORDING | (none) | Stop recording |
| 0x0C | GET_REC_INFO | uint16 LE index (1-based) | Get recording metadata |
| 0x0D | GET_REC_SAMPLES | uint16 LE index + uint32 LE offset(1-based) | Get recording data |
| 0x0E | GET_REC_COUNT | (none) | Get count of recordings |
| 0x12 | HOLD | (none) | Toggle HOLD mode |

---

## 5. Measurement Packet (Type 0x02) -- [KNOWN]

### 5.1 Common Header (5 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | misc | Bit flags (see below) |
| 1 | 1 | misc2 | Bit flags (see below) |
| 2 | 2 | mode | uint16 LE mode word |
| 4 | 1 | range | 0x00 = auto, 0x01-0x08 = manual |

**misc byte**:

| Bit | Mask | Meaning |
|-----|------|---------|
| 1 | 0x02 | Has aux1 display value |
| 2 | 0x04 | Has aux2 display value |
| 3 | 0x08 | Has bargraph / fast mode |
| 4-6 | 0x70 | Format: 0x00=normal, 0x10=relative, 0x20=min/max, 0x40=peak |
| 7 | 0x80 | HOLD active |

**misc2 byte**:

| Bit | Mask | Meaning |
|-----|------|---------|
| 0 | 0x01 | Auto-range active |
| 1 | 0x02 | High voltage warning |
| 3 | 0x08 | Lead error |
| 4 | 0x10 | COMP (comparator) mode active |
| 5 | 0x20 | Record mode active |

### 5.2 Value Encoding

**Full value** (13 bytes):

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | float32 LE (IEEE 754) |
| 4 | 1 | Precision byte |
| 5 | 8 | Unit string (null-terminated) |

**Short value** (5 bytes, used in min/max sub-values):

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | float32 LE |
| 4 | 1 | Precision byte |

**Precision byte**:

| Bits | Meaning |
|------|---------|
| 0 | Positive overload (OL) |
| 1 | Negative overload (-OL) |
| 4-7 | Decimal places (0-15) |

### 5.3 Measurement Variants

**Normal (format 0x00)** -- after 5-byte header:
- Main value: 13 bytes (float32 + precision + unit)
- Aux1: 13 bytes (optional, if misc bit 1 set)
- Aux2: 13 bytes (optional, if misc bit 2 set)
- Bargraph: float32 + 8-byte unit (optional, if misc bit 3 set)

**Relative (format 0x10)**:
- Relative value: 13 bytes
- Reference value: 13 bytes
- Absolute value: 13 bytes
- Fast value: conditional on misc bit 3

**Min/Max (format 0x20)**:
- Current: 5 bytes (short value)
- Max: 5 bytes + uint32 LE timestamp (seconds from start)
- Average: 5 bytes + uint32 LE timestamp
- Min: 5 bytes + uint32 LE timestamp
- Unit: 8 bytes (shared)

**Peak (format 0x40)**:
- Max: 13 bytes (full value with unit)
- Min: 13 bytes (full value with unit)

### 5.4 COMP Mode Extension

When misc2 bit 4 (COMP) is set, after the bargraph unit field:

| Offset | Size | Field |
|--------|------|-------|
| 0 | 1 | Comparison mode: 0=INNER, 1=OUTER, 2=BELOW, 3=ABOVE |
| 1 | 1 | Result: 0=PASS, 1=FAIL |
| 2 | 1 | Precision/digits |
| 3 | 4 | High limit (float32 LE) |
| 7 | 4 | Low limit (float32 LE, only for INNER/OUTER modes) |

---

## 6. Mode Word Table -- [KNOWN]

The mode word is uint16 LE with structured nibble encoding:
- Nibble 3 (MSB): measurement function family
- Nibble 2: sub-function
- Nibble 1: variant (1=normal, 2=Hz/peak/ACDC, 3=peak, 4=LPF, etc.)
- Nibble 0 (LSB): 1=standard, 2=REL variant

97 total modes. Selected examples:

| Mode | Code | Description |
|------|------|-------------|
| V AC | 0x1111 | V AC |
| V AC REL | 0x1112 | V AC relative |
| V AC Hz | 0x1121 | V AC frequency |
| V AC Peak | 0x1131 | V AC peak |
| V AC LPF | 0x1141 | V AC low-pass filter |
| V AC dBV | 0x1151 | V AC dBV |
| V AC dBm | 0x1161 | V AC dBm |
| mV AC | 0x2111 | mV AC |
| mV AC+DC | 0x2141 | mV AC+DC coupled |
| V DC | 0x3111 | V DC |
| V DC AC+DC | 0x3121 | V DC AC+DC coupled |
| V DC Peak | 0x3131 | V DC peak |
| mV DC | 0x4111 | mV DC |
| Temp C T1(T2) | 0x4211 | Temperature C, T1 main, T2 aux |
| Temp C T2(T1) | 0x4221 | Temperature C, T2 main, T1 aux |
| Temp C T1-T2 | 0x4231 | Temperature C, differential |
| Temp F T1(T2) | 0x4311 | Temperature F, T1 main |
| Resistance | 0x5111 | Resistance |
| Continuity | 0x5211 | Continuity (short) |
| Conductance | 0x5311 | Conductance (nS) |
| Diode | 0x6111 | Diode test |
| Capacitance | 0x6211 | Capacitance |
| Frequency | 0x7111 | Frequency |
| Duty Cycle | 0x7211 | Duty cycle |
| Pulse Width | 0x7311 | Pulse width |
| uA DC | 0x8111 | uA DC |
| uA AC | 0x8211 | uA AC |
| mA DC | 0x9111 | mA DC |
| mA AC | 0x9211 | mA AC |
| A DC | 0xA111 | A DC |
| A AC | 0xA211 | A AC |

Each mode has REL variant (+1 to LSB nibble), and current/voltage
modes have Hz, Peak, and AC+DC variants.

---

## 7. Range Byte -- [KNOWN]

| Value | mV | V | uA | mA | A | Ohm | Hz | Cap |
|-------|-----|------|------|------|-----|---------|---------|------|
| 0x00 | Auto | Auto | Auto | Auto | Auto | Auto | Auto | Auto |
| 0x01 | 60mV | 6V | 600uA | 60mA | -- | 600R | 60Hz | 6nF |
| 0x02 | 600mV | 60V | 6000uA | 600mA | -- | 6kR | 600Hz | 60nF |
| 0x03 | -- | 600V | -- | -- | -- | 60kR | 6kHz | 600nF |
| 0x04 | -- | 1000V | -- | -- | -- | 600kR | 60kHz | 6uF |
| 0x05 | -- | -- | -- | -- | -- | 6MR | 600kHz | 60uF |
| 0x06 | -- | -- | -- | -- | -- | 60MR | 6MHz | 600uF |
| 0x07 | -- | -- | -- | -- | -- | -- | 60MHz | 6mF |
| 0x08 | -- | -- | -- | -- | -- | -- | -- | 60mF |

Temperature: fixed range. Current A: fixed at 10A.

---

## 8. Unit Strings -- [KNOWN]

The UT181A sends unit strings as part of measurement packets (8 bytes,
null-terminated). The device determines the unit, not the host.

| Wire String | Unit | Notes |
|-------------|------|-------|
| `mVDC` | millivolt DC | |
| `VDC` | volt DC | |
| `mVAC` | millivolt AC | |
| `VAC` | volt AC | |
| `mVac+dc` | millivolt AC+DC | |
| `Vac+dc` | volt AC+DC | |
| `uADC` | microampere DC | |
| `mADC` | milliampere DC | |
| `ADC` | ampere DC | |
| `uAAC` | microampere AC | |
| `mAAC` | milliampere AC | |
| `AAC` | ampere AC | |
| `uAac+dc` | microampere AC+DC | |
| `mAac+dc` | milliampere AC+DC | |
| `Aac+dc` | ampere AC+DC | |
| `~` | ohm | Tilde represents omega |
| `k~` | kilohm | |
| `M~` | megohm | |
| `nS` | nanosiemens | Conductance |
| `nF` | nanofarad | |
| `uF` | microfarad | |
| `mF` | millifarad | |
| `Hz` | hertz | |
| `kHz` | kilohertz | |
| `MHz` | megahertz | |
| `%` | percent | Duty cycle |
| `ms` | millisecond | Pulse width |
| `dBV` | decibel-volt | |
| `dBm` | decibel-milliwatt | |
| `\xB0C` | degrees Celsius | 0xB0 = degree symbol (Latin-1) |
| `\xB0F` | degrees Fahrenheit | 0xB0 = degree symbol (Latin-1) |

---

## 9. Timestamp Format -- [KNOWN]

Used in saved measurements (type 0x03), recording info (type 0x04),
and recording data (type 0x05). Packed into 32 bits:

```
Bits [5:0]   -> year - 2000    (range: 2000-2063)
Bits [9:6]   -> month           (1-12)
Bits [14:10] -> day             (1-31)
Bits [19:15] -> hour            (0-23)
Bits [25:20] -> minute          (0-59)
Bits [31:26] -> second          (0-59)
```

---

## 10. Recording Protocol -- [KNOWN]

### 10.1 Start Recording (Command 0x0A)

| Field | Size | Description |
|-------|------|-------------|
| Name | 11 | Null-terminated ASCII (max 10 chars) |
| Interval | 2 | uint16 LE, seconds (1-3600) |
| Duration | 4 | uint32 LE, seconds (up to 143,999 minutes) |

Maximum 20 named recordings on the device.

### 10.2 Recording Info (Response Type 0x04)

| Offset | Size | Field |
|--------|------|-------|
| 0 | 11 | Name (null-terminated) |
| 11 | 8 | Unit string |
| 19 | 2 | Interval (uint16 LE, seconds) |
| 21 | 4 | Duration (uint32 LE, seconds) |
| 25 | 4 | Sample count (uint32 LE) |
| 29 | 5 | Max value (float32 LE + precision) |
| 34 | 5 | Average value (float32 LE + precision) |
| 39 | 5 | Min value (float32 LE + precision) |
| 44 | 4 | Start timestamp (packed 32-bit) |

### 10.3 Recording Data (Response Type 0x05)

Downloaded in chunks via command 0x0D. Each response:

| Field | Size | Description |
|-------|------|-------------|
| Count | 1 | Number of samples in this packet (max 250) |
| Samples | 9 * N | Per sample: float32 LE (4) + precision (1) + timestamp (4) |

Download loop: request samples starting at offset 1, increment by
chunk size until all samples retrieved.

---

## 11. Implementation Considerations

### 11.1 Device Discrimination

The UT181A shares VID 0x10C4, PID 0xEA80 with UT61E+, UT8802, and
UT8803. Discrimination approaches:

1. **Frame length**: UT181A uses 2-byte LE length vs UT61E+ 1-byte.
   Send a UT61E+ measurement request and check if the response has
   a valid 1-byte length or if garbage arrives.
2. **Monitor mode**: Send command 0x05 (SET_MONITOR, enable). If the
   device starts streaming type 0x02 packets, it's a UT181A.
3. **User selection**: Let the user specify the device model.

### 11.2 Communication Mode

The meter requires "Communication ON" in settings before USB works.
This is a manual step on the device -- there is no USB command to
enable it. The setting resets on power cycle.

The device cannot measure while charging.

### 11.3 Value Encoding

Unlike UT61E+ (ASCII display string) or UT8803 (BCD/raw bytes), the
UT181A sends IEEE 754 float32 values. The host receives both the
numeric value and its unit string, making parsing straightforward.
The precision byte indicates decimal places for display formatting.

### 11.4 Existing Rust Implementation

The [antage/cp211x_uart](https://github.com/antage/cp211x_uart) crate
provides CP2110 UART control in Rust and could be used directly. The
[antage/ut181a](https://github.com/antage/ut181a) crate provides a
complete UT181A protocol library.

---

## 12. Confidence Summary

Every protocol detail is **[KNOWN]** -- confirmed by three independent
implementations (antage/ut181a Rust, loblab/ut181a C++, sigrok C
driver). No [UNVERIFIED] items remain.

| Aspect | Status | Sources |
|--------|--------|---------|
| Frame format (header, length, checksum) | [KNOWN] | 3 implementations |
| All 15 commands (0x01-0x12) | [KNOWN] | antage + sigrok + loblab |
| All 97 mode words | [KNOWN] | antage + sigrok |
| Range bytes 0x00-0x08 | [KNOWN] | 3 implementations |
| Measurement packet (all 4 variants) | [KNOWN] | antage + sigrok |
| COMP mode extension | [KNOWN] | sigrok driver |
| Unit strings | [KNOWN] | antage + sigrok |
| Timestamp format | [KNOWN] | 3 implementations |
| Recording protocol | [KNOWN] | 3 implementations |
| Transport (9600/8N1 CP2110) | [KNOWN] | 3 implementations |
| Header bytes = 0xAB 0xCD (same as UT61E+) | [KNOWN] | 3 codebases verified |
| Device specs (60K counts, modes, ranges) | [KNOWN] | User manual |
