# VC880/VC650BT Implementation Plan

Ready-to-execute plan for adding Voltcraft VC880 and VC650BT support.
Based on pylablib source analysis (MIT-licensed `VC880` class) and EEVBlog teardown data.

## Device Overview

| Property | Value |
|----------|-------|
| Devices | Voltcraft VC-880 (bench), VC650BT (handheld with Bluetooth) |
| Brand | Voltcraft (Conrad Electronics) |
| Count | 40,000 |
| Chipset | ES51966A (Cyrustek AFE) + MSP430F5418 MCU + BU9799KV LCD + CP2110 |
| USB bridge | Silicon Labs CP2110 HID-to-UART (VID `0x10C4`, PID `0xEA80`) |
| CAT rating | CAT II 600V |
| Activation | User must press `PC` button on device to enable USB communication |

**Relationship:** Both devices share the same wire protocol. pylablib's `VC880` class handles both. Conrad's protocol spec is titled "VC880 Protocol Rev 2.4 Protocol_Rev2_VC650BT_DESKTOP_DMM" — both model names appear in the document title.

## Wire Protocol

### Frame Format

Identical `0xAB 0xCD` + BE16 checksum framing as the UT61E+ — reuses `extract_frame_abcd_be16()`.

```
Offset  Size  Field         Description
0       2     Header        0xAB 0xCD (magic bytes)
2       1     Length        payload_size + 3
3       1     Command/Type  Message type (0x01 = live measurement)
4       N     Payload       Variable length, depends on message type
4+N     2     Checksum      Big-endian uint16, sum of bytes [0..4+N)
```

**Checksum:** `sum(all_preceding_bytes)` as big-endian unsigned 16-bit. Covers header + length + type + payload, excluding the checksum itself.

**Outbound message construction:**
```python
hdr = b"\xAB\xCD" + struct.pack("BB", len(data) + 3, command)
csum = struct.pack(">H", sum(hdr) + sum(data))
message = hdr + data + csum
```

**Frame recovery:** Parser searches for `0xAB 0xCD` magic with up to 100 byte-at-a-time advances, tolerating up to 3 bytes of misalignment. Minimum length validation: length field >= 3.

### Live Measurement Payload (type 0x01, 33 bytes)

```
Offset  Size  Field           Description
0       1     Function        Mode/function code (0x00-0x12)
1       1     Range           Range index (0x30-based, subtract 0x30 for array index)
2-8     7     Main value      ASCII, right-justified, "OL" for overload, "---" for no reading
9-15    7     Aux display 1   Upper right: min/max/avg/rel value (parsed as float)
16-22   7     Aux display 2   Upper left: memory display (parsed as int)
23-25   3     Aux display 3   Bottom: linear scale/bar graph (parsed as int)
26-32   7     Status flags    7 bytes of status/flag information
```

**Streaming model:** Device sends measurements continuously — no polling/trigger command needed.

### Function Code Table (19 modes)

```
Code  Function     Unit   Range Kind  Description
0x00  DCV          V      V           DC Voltage
0x01  ACDCV        V      V           AC+DC Voltage
0x02  DCmV         V      mV          DC Millivolt
0x03  freq         Hz     Hz          Frequency
0x04  duty_cycl    %      perc        Duty Cycle
0x05  ACV          V      V           AC Voltage
0x06  res          Ohm    Ohm         Resistance
0x07  diod         V      V           Diode
0x08  short        Ohm    Ohm         Continuity
0x09  cap          F      F           Capacitance
0x0A  t_cels       °C     dC          Temperature (Celsius)
0x0B  t_fahr       °F     dF          Temperature (Fahrenheit)
0x0C  DCuA         A      uA          DC Microamps
0x0D  ACuA         A      uA          AC Microamps
0x0E  DCmA         A      mA          DC Milliamps
0x0F  ACmA         A      mA          AC Milliamps
0x10  DCA          A      A           DC Amps
0x11  ACA          A      A           AC Amps
0x12  low_pass     V      V           Low-Pass Filter
```

### Range Tables (indexed by `range_byte - 0x30`)

| Range Kind | [0] | [1] | [2] | [3] | [4] | [5] | [6] | [7] |
|------------|-----|-----|-----|-----|-----|-----|-----|-----|
| V | 4 | 40 | 400 | 1000 | | | | |
| mV | 0.4 | | | | | | | |
| A | 10 | | | | | | | |
| mA | 0.04 | 0.4 | | | | | | |
| uA | 0.0004 | 0.004 | | | | | | |
| Ohm | 400 | 4k | 40k | 400k | 4M | 40M | | |
| F | 40nF | 400nF | 4uF | 40uF | 400uF | 4mF | 40mF | |
| Hz | 40 | 400 | 4k | 40k | 400k | 4M | 40M | 400M |
| perc | (no table, default [1]) | | | | | | | |
| dC | (no table, default [1]) | | | | | | | |
| dF | (no table, default [1]) | | | | | | | |

SI prefix scaling: `int(log10(range * 0.99) // 3)`.

### Status Flags

Located at payload bytes 26-32 (7 bytes). Only `stat[1]` (byte 27) is decoded in pylablib:

| Bit | Mask | Meaning |
|-----|------|---------|
| `stat[1] & 0x08` | Max hold active |
| `stat[1] & 0x04` | Min hold active |
| `stat[1] & 0x02` | Average active |
| `stat[1] & 0x01` | Relative/delta mode |

Checked in priority order: max > min > avg > rel. Determines which value appears on the upper-right auxiliary display.

**Undocumented:** `stat[0]` and `stat[2]`-`stat[6]` are read but not interpreted in pylablib. These 6 bytes likely contain additional flags (auto/manual range, battery, hold, etc.) — add to verification backlog.

### Commands

| Command byte | Data | Function | Notes |
|--------------|------|----------|-------|
| `0x47` | (empty) | Enable autorange | Send 3 times for reliability; exhaust read queue before each send |
| `0x46` | (empty) | Disable autorange | Send once |

Pre-exhaust pattern: drain all pending messages from read queue before sending commands.

## Infrastructure Reuse

| Component | Reuse? | Notes |
|-----------|--------|-------|
| `Cp2110Transport` | Yes, as-is | Same VID/PID/bridge chip |
| `extract_frame_abcd_be16()` | Yes, as-is | Identical framing |
| `StatusFlags` | Yes | Map VC880 bits to existing flag struct |
| `Measurement` struct | Yes | Standard output format |
| `Protocol` trait | Implement | New protocol family |
| New dependencies | None | Everything exists |

## Files to Create

| File | Description |
|------|-------------|
| `crates/dmm-lib/src/protocol/vc650bt/mod.rs` | Protocol implementation (follow UT8803 pattern) |
| `crates/dmm-lib/src/protocol/vc650bt/tables/mod.rs` | Mode/range/unit position tables |
| `docs/research/vc880/reverse-engineering-approach.md` | RE methodology documentation |
| `docs/research/vc880/reverse-engineered-protocol.md` | Protocol specification (distilled from this plan) |

## Files to Modify

| File | Change |
|------|--------|
| `crates/dmm-lib/src/protocol/mod.rs` | Add `mod vc650bt;` and `DeviceFamily::Vc650bt` variant |
| `crates/dmm-lib/src/protocol/registry.rs` | Add `SelectableDevice` entries for VC-880 and VC650BT |
| `crates/dmm-lib/src/lib.rs` | Add match arm in `open_device()` |
| `docs/supported-devices.md` | Add VC880/VC650BT entries |
| `docs/verification-backlog.md` | Add VC880/VC650BT verification items |
| `docs/architecture.md` | Add VC650BT protocol family to diagram |
| `CHANGELOG.md` | Note new device support (Experimental) |

## Commit Sequence

### Commit 1: Research documents
- `docs/research/vc880/reverse-engineering-approach.md`
- `docs/research/vc880/reverse-engineered-protocol.md`

### Commit 2: Protocol implementation
- `crates/dmm-lib/src/protocol/vc650bt/mod.rs` — `Vc650btProtocol` implementing `Protocol` trait
- `crates/dmm-lib/src/protocol/vc650bt/tables/mod.rs` — Mode/range position tables
- Unit tests using `MockTransport` with synthetic byte sequences matching the documented payload format

### Commit 3: Registry integration
- `crates/dmm-lib/src/protocol/mod.rs` — `DeviceFamily::Vc650bt`
- `crates/dmm-lib/src/protocol/registry.rs` — Two `SelectableDevice` entries (VC-880, VC650BT)
- `crates/dmm-lib/src/lib.rs` — `open_device()` match arm
- `Stability::Experimental` — no physical device for verification

### Commit 4: Documentation
- `docs/supported-devices.md`, `docs/verification-backlog.md`, `docs/architecture.md`

## Key Design Decisions

1. **Family name `vc650bt`** — matches the adding-devices convention of using protocol family names for module paths. Both VC-880 and VC650BT share this family.
2. **Follow UT8803 pattern** — new protocol family with its own `Protocol` impl, position-table-based mode mapping, streaming measurement model.
3. **`Stability::Experimental`** — mandatory until verified against real hardware per `adding-devices.md` Phase 6.
4. **Two `SelectableDevice` entries** — "Voltcraft VC-880" and "Voltcraft VC650BT" both pointing to the same `Vc650btProtocol` factory.
5. **Capture steps** — must include "Press PC button on meter" as first step, then cover all 19 modes, flag states, and autorange commands.

## Verification Backlog Items (for Phase 6)

- [ ] Basic connectivity — frames received via `debug` command
- [ ] All 19 measurement modes parse correctly
- [ ] Range byte 0x30 prefix confirmed
- [ ] Main display ASCII parsing (normal values, OL, "---")
- [ ] Aux display 1 (min/max/avg/rel values)
- [ ] Aux display 2 (memory)
- [ ] Aux display 3 (bar graph)
- [ ] Status flag bits: MAX, MIN, AVG, REL at documented positions
- [ ] Undocumented status bytes `stat[0]`, `stat[2]`-`stat[6]`
- [ ] Autorange enable (0x47) command
- [ ] Autorange disable (0x46) command
- [ ] PC button activation requirement
- [ ] VC650BT compatibility (if device available)

## Reference Implementations

| Project | Language | License | Notes |
|---------|----------|---------|-------|
| [pylablib](https://github.com/AlexShkarin/pyLabLib) `VC880` class | Python | MIT | Primary source; complete driver, tested with VC-880 |
| Conrad "Protocol Rev 2.4" spec | — | — | Official spec (PDF not fully accessible during research) |
