# Protocol Reference — UT61E+ Family

This document covers the UT61E+ / UT61B+ / UT61D+ / UT161 family protocol.
For other supported device families, see the per-family research docs:

- [UT8803 protocol](research/ut8803/reverse-engineered-protocol.md)
- [UT171 protocol](research/ut171/reverse-engineered-protocol.md)
- [UT181A protocol](research/ut181/reverse-engineered-protocol.md)

All families share the same `AB CD` header framing and UART protocol, which is
transport-agnostic — it runs identically over both the CP2110 and CH9329 USB
bridges. Only the USB HID report format differs between the two bridges (see
below). The families differ in message structure, byte ordering, value encoding,
and polled-vs-streaming behavior. Each family has its own `Protocol` trait
implementation in `crates/dmm-lib/src/protocol/`.

## USB HID Transports

The UART protocol runs over a HID-to-UART bridge chip. Two bridge chips are
supported; the tool auto-detects which one is present.

### CP2110 (Silicon Labs)

- **VID:** `0x10C4` — **PID:** `0xEA80`
- **Baud rate:** 9600 (confirmed; 19200 and 115200 tested — meter does not respond)
- **Format:** 8N1
- **HID reports:** 64 bytes. Byte 0 = UART data length, bytes 1-63 = payload.

#### Initialization

Three HID feature reports must be sent to initialize the UART:

1. **Enable UART:** `[0x41, 0x01]`
2. **Configure 9600/8N1:** `[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x00, 0x03, 0x00]`
   - Bytes 1-4: baud rate = `0x00002580` = 9600 (big-endian)
   - Byte 5: `0x00` = no parity
   - Byte 6: `0x00` = no flow control
   - Byte 7: `0x03` = 8 data bits
   - Byte 8: `0x00` = short stop bit (1 stop bit)
3. **Purge RX FIFO:** `[0x43, 0x02]` (0x01=TX only, 0x02=RX only, 0x03=both — we purge RX only since TX is empty at init)

#### Interrupt Transfers

Data is sent/received via HID interrupt reports:

- **OUT (host → device):** First byte is payload length, followed by payload bytes.
- **IN (device → host):** First byte is payload length, followed by payload bytes.

### CH9329 (WCH) — Experimental

- **VID:** `0x1A86` — **PID:** `0xE429`
- **Baud rate:** 9600 (configured at the chip level; no host-side baud rate setup needed)
- **HID reports:** 65 bytes. Byte 0 = report ID (`0x00`), byte 1 = UART data length, bytes 2-64 = payload.
- **Cable:** UT-D09 (CH9329 variant), sold by UNI-T for UT181A, UT171 series, UT243.
- **Driver:** None needed — the CH9329 is a standard driverless HID device on all platforms.
- **Initialization:** No feature reports required. The chip is ready for data transfer as soon as the HID device is opened.

CH9329 support needs real device verification. The UART protocol bytes are
identical to CP2110 — only the HID report framing differs.

## Message Format

All messages (requests and responses) use the same framing:

```
AB CD <length> <payload...> <checksum_hi> <checksum_lo>
```

- **Header:** `0xAB 0xCD` (2 bytes)
- **Length:** byte count of everything after this byte (payload + checksum) (1 byte)
- **Payload:** `length - 2` bytes
- **Checksum:** 16-bit big-endian sum of all preceding bytes (header + length + payload)

Total frame size = `2 + 1 + length` bytes.

## Request: Get Measurement

```
AB CD 03 5E 01 D9
```

- Length: `0x03` (3 = 1 byte command + 2 byte checksum)
- Command: `0x5E`
- `0x5E` is the "get measurement" command
- `0x01 0xD9` is `(0x5E + 379) = 473 = 0x01D9`

## Response: Measurement Data

```
AB CD 10 <14 payload bytes> <checksum_hi> <checksum_lo>
```

Total: 19 bytes. Length byte = `0x10` (16 = 14 payload + 2 checksum).

### Payload Layout (14 bytes)

| Offset | Content | Masking |
|--------|---------|---------|
| 0 | Mode | Raw (no masking) |
| 1 | Range | `& 0x0F` |
| 2-8 | Display value (7 ASCII chars) | None |
| 9 | Bar graph tens digit | Raw |
| 10 | Bar graph ones digit | Raw |
| 11 | Flag byte 1 | `& 0x0F` |
| 12 | Flag byte 2 | `& 0x0F` |
| 13 | Flag byte 3 | `& 0x0F` |

**Masking (verified against real device):**
- Byte 0: mode — raw, no masking (does not have `0x30` prefix)
- Byte 1: range — mask with `& 0x0F` (has `0x30` prefix)
- Bytes 2-8: display — valid ASCII, no masking
- Bytes 9-10: progress — raw bytes, no `0x30` prefix observed on real device
- Bytes 11-13: flags — arrive with `0x30` high nibble, mask with `& 0x0F`

### Display Value

7 ASCII characters, right-aligned with space padding. Examples:
- `" 12.345"` — normal reading
- `"    OL "` — overload
- `"-12.345"` — negative value

### Bar Graph

Decimal encoding: `byte9 * 10 + byte10`. Represents the number of lit
segments on the meter's ~46-segment LCD bar graph. The LCD has fixed
markings at 0, 5, 10, 15, 20; their meaning in real units scales with
the range (e.g., on 22V range: 0=0V, 5≈5V, 20≈20V; on 2.2V range:
0=0V, 5≈0.5V, 20≈2V).

Verified on real device (DC V, 22V range): 0V→0, 5V→9, 10V→20, 20V→39.
On 2.2V range: 1V→20. Consistent with `value / range_max * 46`.

For negative values, the bar graph holds the magnitude and flag byte 13
bit 0 (bar_pol) is set. On overload (OL), the bar graph reads 44
(near full scale).

### Flag Bytes

Verified against real device and ljakob/unit_ut61eplus (Python).

**Byte 11 (Flag 1):**
- Bit 0: REL (relative/delta) — verified
- Bit 1: HOLD — verified
- Bit 2: MIN (stored minimum displayed) — verified
- Bit 3: MAX (stored maximum displayed) — verified
- MIN/MAX cycle: MAX only → MIN only → MAX (2-state, bits never both set).
  When set, the display value is the stored min or max, not the live reading.
  AUTO flag is cleared (range locked) during MIN/MAX mode.

**Byte 12 (Flag 2):**
- Bit 0: HV warning (>30V per manual; confirmed set at 31V on DC V) — verified
- Bit 1: Low battery — verified (intermittent on real device)
- Bit 2: **!AUTO** (inverted: bit clear = auto-range ON) — verified

**Byte 13 (Flag 3):**
- Bit 0: Bar polarity (set when value is negative) — verified
- Bit 1: Peak MIN (stored instantaneous minimum peak) — verified
- Bit 2: Peak MAX (stored instantaneous maximum peak) — verified
- Bit 3: DC indicator
- Peak cycle: P-MAX only → P-MIN only → P-MAX (2-state, bits never both set).
  When set, the display value is the stored instantaneous peak (not RMS), not the live reading.
  Peak mode is context-dependent: activates on AC modes (e.g., AC mV), no effect on DC V.

## Command Encoding

To send a button press command:

```
[0xAB, 0xCD, 0x03, cmd, (cmd + 379) >> 8, (cmd + 379) & 0xFF]
```

Known commands (from ljakob/unit_ut61eplus, verified against real device):

| Byte | Command | Verified |
|------|---------|----------|
| `0x41` | MIN/MAX toggle | Yes (remote) |
| `0x42` | Exit MIN/MAX | Yes (remote) |
| `0x46` | RANGE (manual toggle) | Yes (remote) |
| `0x47` | AUTO (restore auto-range) | Yes (remote) |
| `0x48` | REL (relative/delta) | Yes (remote) |
| `0x49` | SELECT2 (Hz/USB button) | Yes (AC mV: cycles mV → Hz → Duty% → mV; no effect on DC V) |
| `0x4A` | HOLD | Yes (remote) |
| `0x4B` | LIGHT (backlight) | Yes (remote) |
| `0x4C` | SELECT (orange, cycles sub-modes) | Yes (remote, cycles DC→AC+DC) |
| `0x4D` | Peak MIN/MAX | Yes (AC mV; context-dependent, no effect on DC V) |
| `0x4E` | Exit Peak | Yes (clears peak flags, returns to live) |
| `0x5E` | Get measurement | Yes |
| `0x5F` | Get device name | — |

## Mode Values

Raw byte value (no masking — mode byte does NOT have 0x30 prefix).
Verified against real device and reference implementations.

| Value | Mode | Verified |
|-------|------|----------|
| 0x00 | AC V | Yes |
| 0x01 | AC mV | Yes |
| 0x02 | DC V | Yes |
| 0x03 | DC mV | — |
| 0x04 | Hz (Frequency) | Yes (V~ and mA via SELECT2) |
| 0x05 | Duty Cycle % | Yes (mA via SELECT2) |
| 0x06 | Ω (Resistance) | Yes |
| 0x07 | Continuity | Yes |
| 0x08 | Diode | Yes |
| 0x09 | Capacitance | Yes |
| 0x0A | Temperature °C | — |
| 0x0B | Temperature °F | — |
| 0x0C | DC µA | Yes |
| 0x0D | AC µA | — |
| 0x0E | DC mA | Yes |
| 0x0F | AC mA | — |
| 0x10 | DC A | Yes (A⎓ dial) |
| 0x11 | AC A | Yes (A⎓ + SELECT) |
| 0x12 | hFE | Yes |
| 0x13 | Live | — |
| 0x14 | NCV | Yes |
| 0x15 | LoZ V | — (UT61D+ only?) |
| 0x16 | LoZ V (2) | — (not on UT61E+; vendor SW names it "LozV") |
| 0x17 | LPF | — (not on UT61E+; vendor SW names it "LPF") |
| 0x18 | LPF V | Yes (V~ + SELECT; no signal needed) |
| 0x19 | AC+DC V | Yes (V⎓ + SELECT; no signal needed) |
| 0x1A | LPF mV | — |
| 0x1B | AC+DC mV | — |
| 0x1C | LPF A | — |
| 0x1D | AC+DC A | — |
| 0x1E | Inrush | — |

## Sampling Rate

Maximum effective sampling rate is **~10 Hz** (~100ms per request-response cycle). This is a hard limit of the 9600 baud firmware — tested and confirmed that the meter does not respond at 19200 or 115200 baud.

Measured throughput (2026-03-18, CLI `--interval-ms` over 10s):

| Configured delay | Samples/10s | Effective Hz |
|-----------------|-------------|-------------|
| 0ms (fastest) | 101 | ~10.1 |
| 100ms | 56 | ~5.6 |
| 200ms | ~40 | ~4.0 |
| 300ms | 25 | ~2.5 |
| 500ms | 18 | ~1.8 |
| 1000ms | 9 | ~0.9 |
| 2000ms | 5 | ~0.5 |

The configured delay adds on top of the ~100ms wire round-trip time.

## Known Quirks

- **Byte-at-a-time delivery:** CP2110 at 9600 baud delivers response bytes one at a time via HID interrupt reports. Accumulate in a buffer and scan for complete `AB CD` frames. A full measurement response requires ~19 individual reads.
- **Timeout vs disconnect:** `HidDevice::read_timeout()` returns 0 on timeout, error on USB disconnect — handle both cases.
- **Request-response only:** The meter does not stream data — each reading requires sending the `0x5E` request command.
- **Mode byte reflects active unit, not dial position:** On DC V dial with auto-range, the meter reports mode 0x02 (DCV) even when showing mV-scale values. The range byte determines the actual scale.
- **AUTO flag has inverted logic:** Flag byte 12 bit 2 clear = auto-range ON.
- **SELECT2 and Peak commands are context-dependent:** They beep (acknowledged) but only produce visible effects in specific modes (e.g., SELECT2 on AC V for frequency display, Peak on AC modes).
- **MIN/MAX and Peak report stored values, not live:** When MIN, MAX, P-MIN, or P-MAX flags are set, the display value field contains the stored statistic, not the current live reading. The bar graph may still reflect the live signal.

## CP2110 Diagnostic Reports

These are CP2110 HID feature reports (not meter protocol), documented in AN434.

### Get Version Information (report 0x46)

Direction: Get (device → host). Returns 2 data bytes:

| Offset | Size | Description |
|--------|------|-------------|
| 1 | 1 | Part number (0x0A for CP2110) |
| 2 | 1 | Device firmware version |

### Get UART Status (report 0x42)

Direction: Get (device → host). Returns 6 data bytes:

| Offset | Size | Description |
|--------|------|-------------|
| 1-2 | 2 | TX FIFO byte count (LE, max 480) |
| 3-4 | 2 | RX FIFO byte count (LE, max 480) |
| 5 | 1 | Error Status (bit 0 = parity, bit 1 = overrun) |
| 6 | 1 | Break Status (0x00 = inactive, 0x01 = active) |

Reading this report clears the error flags. Useful for detecting overrun errors that would otherwise only manifest as checksum failures in the meter protocol.

### Set Reset Device (report 0x40)

Direction: Set (host → device). Payload: `[0x40, 0x00]`.

Resets the CP2110 and re-enumerates on USB. All UART config is lost — must re-initialize after re-opening.

**UT61E+ note:** This report is rejected with a HID protocol error on the UT61E+'s CP2110. UNI-T likely locked this report out in the device's HID descriptor.

## References

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) — Python implementation
- [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — Go implementation
- [Silicon Labs AN434](https://www.silabs.com/documents/public/application-notes/an434-cp2110-4-interface-specification.pdf) — CP2110/4 HID-to-UART interface specification
