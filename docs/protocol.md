# Protocol Reference

## CP2110 HID-to-UART Bridge

The UT61E+ uses a Silicon Labs CP2110 chip as a USB HID-to-UART bridge.

- **VID:** `0x10C4` (Silicon Labs)
- **PID:** `0xEA80` (CP2110)
- **Baud rate:** 9600 (confirmed; 19200 and 115200 tested — meter does not respond)
- **Format:** 8N1

### Initialization

Three HID feature reports must be sent to initialize the UART:

1. **Enable UART:** `[0x41, 0x01]`
2. **Configure 9600/8N1:** `[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x00, 0x03, 0x00]`
   - Bytes 1-4: baud rate = `0x00002580` = 9600 (big-endian)
   - Byte 5: `0x00` = no parity
   - Byte 6: `0x00` = no flow control
   - Byte 7: `0x03` = 8 data bits
   - Byte 8: `0x00` = short stop bit (1 stop bit)
3. **Purge RX FIFO:** `[0x43, 0x02]` (0x01=TX only, 0x02=RX only, 0x03=both — we purge RX only since TX is empty at init)

### Interrupt Transfers

Data is sent/received via HID interrupt reports:

- **OUT (host → device):** First byte is payload length, followed by payload bytes.
- **IN (device → host):** First byte is payload length, followed by payload bytes.

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
| 9 | Bar graph high nibble | Raw |
| 10 | Bar graph low nibble | Raw |
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

Combined from two nibbles: `(byte9 << 4) | byte10`. Range 0-100.

### Flag Bytes

Verified against real device and ljakob/unit_ut61eplus (Python).

**Byte 11 (Flag 1):**
- Bit 0: REL (relative/delta) — verified
- Bit 1: HOLD — verified
- Bit 2: MIN — verified
- Bit 3: MAX — verified

**Byte 12 (Flag 2):**
- Bit 0: HV warning (>30V)
- Bit 1: Low battery — verified (intermittent on real device)
- Bit 2: **!AUTO** (inverted: bit clear = auto-range ON) — verified

**Byte 13 (Flag 3):**
- Bit 0: Bar polarity
- Bit 1: Peak MIN
- Bit 2: Peak MAX
- Bit 3: DC indicator

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
| `0x49` | SELECT2 (Hz/USB button) | Received (beep, no visible effect on DC V) |
| `0x4A` | HOLD | Yes (remote) |
| `0x4B` | LIGHT (backlight) | Yes (remote) |
| `0x4C` | SELECT (orange, cycles sub-modes) | Yes (remote, cycles DC→AC+DC) |
| `0x4D` | Peak MIN/MAX | Received (beep, no visible effect on DC V) |
| `0x4E` | Exit Peak | Sent, not visibly confirmed |
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
- **SELECT2 and Peak commands are context-dependent:** They beep (acknowledged) but only produce visible effects in specific modes (e.g., SELECT2 on AC V for frequency display).

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
