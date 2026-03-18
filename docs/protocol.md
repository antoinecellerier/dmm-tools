# Protocol Reference

## CP2110 HID-to-UART Bridge

The UT61E+ uses a Silicon Labs CP2110 chip as a USB HID-to-UART bridge.

- **VID:** `0x10C4` (Silicon Labs)
- **PID:** `0xEA80` (CP2110)
- **Baud rate:** 9600
- **Format:** 8N1

### Initialization

Three HID feature reports must be sent to initialize the UART:

1. **Enable UART:** `[0x41, 0x01]`
2. **Configure 9600/8N1:** `[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x00, 0x03, 0x00, 0x00]`
   - Bytes 2-3: baud rate = `0x00002580` = 9600
   - Byte 7: `0x03` = 8 data bits, no parity, 1 stop bit
3. **Purge FIFOs:** `[0x43, 0x02]` (purge both TX and RX)

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
| 0 | Mode | `& 0x0F` |
| 1 | Range | `& 0x0F` |
| 2-8 | Display value (7 ASCII chars) | None |
| 9 | Bar graph high nibble | Raw |
| 10 | Bar graph low nibble | Raw |
| 11 | Flag byte 1 | `& 0x0F` |
| 12 | Flag byte 2 | `& 0x0F` |
| 13 | Flag byte 3 | `& 0x0F` |

**Masking (verified against real device):**
- Bytes 0, 1: mode/range — always mask with `& 0x0F` (may or may not have `0x30` high nibble)
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

**Byte 11 (Flag 1):**
- Bit 0: HOLD
- Bit 1: REL (relative/delta)

**Byte 12 (Flag 2):**
- Bit 0: Auto range
- Bit 1: MIN
- Bit 2: MAX

**Byte 13 (Flag 3):**
- Bit 0: Low battery

## Command Encoding

To send a button press command:

```
[0xAB, 0xCD, 0x03, cmd, (cmd + 379) >> 8, (cmd + 379) & 0xFF]
```

Known commands:
- `0x5E` — Get measurement
- `0x48` — HOLD
- `0x4D` — MIN/MAX
- `0x52` — REL
- `0x41` — RANGE
- `0x53` — SELECT
- `0x4C` — LIGHT (backlight)

## Mode Values

Raw byte value (no masking — mode byte does NOT have 0x30 prefix).
Verified against real device and reference implementations.

| Value | Mode | Verified |
|-------|------|----------|
| 0x00 | AC V | Yes |
| 0x01 | AC mV | Yes |
| 0x02 | DC V | Yes |
| 0x03 | DC mV | — |
| 0x04 | Hz (Frequency) | Yes |
| 0x05 | Duty Cycle % | — |
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
| 0x10 | DC A | — |
| 0x11 | AC A | — |
| 0x12 | hFE | Yes |
| 0x13 | Live | — |
| 0x14 | NCV | Yes |
| 0x15 | LoZ V | — |
| 0x16 | AC+DC A | — |
| 0x17 | AC+DC/DC A | — |
| 0x18 | LPF V | — |
| 0x19 | AC+DC V | — |
| 0x1A | LPF mV | — |
| 0x1B | AC+DC mV | — |
| 0x1C | LPF A | — |
| 0x1D | AC+DC A | — |
| 0x1E | Inrush | — |

## Known Quirks

- Responses may arrive split across multiple HID interrupt reads — accumulate in a buffer and scan for complete `AB CD` frames.
- `HidDevice::read_timeout()` returns 0 on timeout, error on USB disconnect — handle both cases.
- The meter does not stream data — each reading requires sending the request command.
- After mode change on the meter, the first response may have stale data from the previous mode.

## References

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) — Python implementation
- [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — Go implementation
- [Silicon Labs AN433](https://www.silabs.com/documents/public/application-notes/AN433-CP2110-4-Interface-Specification.pdf) — CP2110/4 HID-to-UART interface specification
