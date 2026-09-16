# UT803 / UT804: Reverse-Engineered Protocol Specification

Protocol specification for the UNI-T UT803 (6000-count) and UT804 (4000-count)
bench multimeters.

Based on:
- Ghidra decompilation of UT803.exe V1.01 and UT804.exe V2.00 (standalone PC apps)
- Ghidra decompilation of both apps' form event handlers, their disassembly
  and their form resources (2026-09-16)
- Binary constant extraction from both executables
- The UT803 operating manual's serial port settings
- CH9325 HID transport analysis (see `../uci-bench-family/reverse-engineered-protocol.md`)

Confidence levels:
- **[VENDOR]** — confirmed by analyzing UNI-T's official software binaries
- **[DEDUCED]** — logical inferences from available evidence
- **[UNVERIFIED]** — requires real device testing to confirm

---

## 1. Transport Layer

### 1.1 USB HID Bridge — [VENDOR]

Both meters use the WCH CH9325 USB-to-UART HID bridge:
- VID: 0x1A86, PID: 0xE008
- 8-byte HID reports with 0xF0+length RX framing
- Already implemented in `transport/ch9325.rs`

Neither app filters on VID or PID: both list every HID device and
preselect the last one whose product string is `USB to Serial` and
that has no serial-number string (UT804 VA 0x56116A, UT803
VA 0x55D622). VID and PID only name a device with no product string.

### 1.2 UART Parameters — [VENDOR]

The UT804.exe init function (FUN_00560668) configures the CH9325 with one
10-byte feature report. From the disassembly (VA 0x5606F2-0x560728): nine
bytes from `-0x48(%ebp)` are zeroed, then `0x60`, `0x09` and `0x03` are
written at offsets 0, 1 and 4, and the buffer handed to the SetFeature
wrapper (FUN_0051afbc, length 10) starts one byte earlier, at
`-0x49(%ebp)`:

```
[xx, 0x60, 0x09, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00]
```

- **Baud rate: 2400** — `0x0960`, little-endian in bytes 1-2 (and in bytes
  1-4 read as 32 bits).
- Byte 5 = `0x03`; its meaning is not determined from the binaries.
- Byte 0, the report ID, is not written by this function. It holds what
  the stack held: whatever the `ReadBtn` click just before (below) left
  there. Its value is not determined statically.

The call is `HidD_SetFeature`: FUN_0051afbc calls through the pointer at
`0x56B4A0`, which start-up fills from the export named `HidD_SetFeature`
(VA 0x5190E3-0x5190EF). The caller discards the result (VA 0x56072D), so
a rejected report — a nonzero byte 0 would likely be one — goes unnoticed,
and the app working does not by itself show that the bridge took this
layout.

FUN_00560668 is the published method `SetFeatureClick`, the `OnClick` of a
`TButton` named `SetFeature` on the main form. The button lies below the
form's client area (only the keyboard can reach it), and in code only
`USBConClick` (VA 0x5615D8), the handler of the `USB Connect` speed
button, runs it: it calls `ReadBtn.Click` and then
`SetFeature.Click` (dynamic method index -21 through `CallDynaInst`, VA
0x5615EE-0x561607; -21 resolves to `TSpeedButton.Click` at 0x43F24C for
`ReadBtn` and to `TButton.Click` at 0x454F98 for `SetFeature`, both
calling `TControl.Click` at 0x45F070). The report is therefore sent each
time `USB Connect` is pressed or released while a device is selected.
With none selected, `SetFeatureClick` shows "USB interface cable is not
securely connected, please check and re-try." and releases the button
instead (VA 0x560788-0x5607A8).

Ghidra's decompilation of FUN_00560668 starts the buffer at `local_4c` and
drops the arguments of the SetFeature call; the offsets above are from the
disassembly.

A search for the same writes finds no other rate in UT804.exe.

**UT803.exe sends 19200 baud.** Its `SetFeatureClick` (VA 0x55CB00)
builds the same layout at five sites, each later one overriding the
earlier: 2400 unconditionally (VA 0x55CB9A), then 19200 (`00 4B 00 00 03`,
VA 0x55CBB8) if the checkbox `IFUT803` is checked, then 2400 again for
`IFUT60E`, `IFUT70B` and `IFUT61C` (VA 0x55CBD6, 0x55CBF4, 0x55CC12). The
form data checks only `IFUT803`, and no code changes these boxes (they
are hidden or out of view), so the app sends
`xx 00 4B 00 00 03 00 00 00 00`. Byte 0 is unset and the result
unused, as in UT804.exe.

**The UNI-T SDK DLL uses a different layout** for the same rate:
`00 60 09 03 00 00 00 00 00 00`, `0x03` in byte 3
(`../uci-bench-family/reverse-engineered-protocol.md` §4.3). Which layout
the CH9325 reads is [UNVERIFIED]. This section previously quoted the
UT804.exe bytes as `[0x60, 0x09, 0x03]`, which hid the difference.

**Serial port.** Both apps read RS232 through a `TCommPortDriver`
(`COMM2`) whose constructor sets 8 data bits, 1 stop bit and no parity
(UT804 VA 0x4801E4-0x4801EC); the form data sets only the port and
`br2400`. UT803.exe's `FormCreate` raises the rate to 19200 when
`IFUT803` is checked (VA 0x5586D7). So UT804.exe opens its port at 2400
8N1 and UT803.exe at 19200 8N1.

The UT803 manual gives the meter's RS232 port as 19200 baud, 7 data
bits, odd parity, 1 stop bit [KNOWN]. The apps still read such a meter
correctly: their receive paths use only low nibbles, except that the
RS232 handlers match CR as the whole byte 0x0D, and CR's odd-parity bit
is 0 (§2). The UT804 manual gives no line format; the UT804's is
[UNVERIFIED].

---

## 2. Packet Format

> **2026-09-16 correction:** earlier revisions gave the wire format as
> 14-byte FS9721 frames with index high nibbles. Both apps' live paths
> take 11-byte packets (§2.1); the 14-byte check belongs to a path the
> user cannot reach (§2.4). Established by decompiling the form's event
> handlers, which the full-program decompile had missed.

### 2.1 11-Byte Packets — [VENDOR]

Both meters send 11-byte packets: 9 data bytes, CR (0x0D), LF (0x0A).
Every live receive path (§2.2, §2.3) keeps only the low nibble of each
byte; no high nibble is tested. **No checksum.**

The two parsers index the packet differently:

- **UT804** (`LcdDisplay71A`, FUN_00558a7c) gets the low nibbles of
  bytes 1-11: position k is byte k, and positions 10-11 are `D`, `A`.
- **UT803** (`LcdDisplay70B`, VA 0x558B3C) gets `A`, the low nibbles of
  bytes 1-9, then `D`: position k is byte k-1. It reads positions 2-10,
  that is bytes 1-9.

§3 gives the UT804 layout by position, §7.4 item 4 the UT803's.

### 2.2 USB Receive Path — [VENDOR]

`USB Connect` (`USBConClick`, UT804 VA 0x5615D8, UT803 VA 0x55DA80)
installs the HID data handler: UT804 VA 0x560CD0 (VA 0x56168F), UT803
VA 0x55D19C (VA 0x55DB51, chosen by the checked `IFUT803` box). `Read`,
`USB Connect` and `COM Connect` are speed buttons in one group, so `Read`
is up while `USB Connect` is down, and the `ReadBtn.Click` call just
before clears any other handler.

The handler formats each report as hex text and reads the low digit of
the report's first byte as the payload count (UT804 VA 0x560D66):

1. A report with 1-9 payload bytes is appended to a buffer
   (VA 0x560DDC-0x560E02).
2. The next report with any other count, normally an empty `F0` report,
   ends the packet. The first 11 buffered bytes are used; later ones
   are dropped with the buffer (VA 0x560F32). Nothing resynchronises
   inside the buffer, so a packet must start right after an empty
   report.
3. UT804: if the low nibbles of bytes 10-11 are `D`, `A`
   (VA 0x560F0C), `LcdDisplay71A` runs (VA 0x560F1B) and checks them
   again (VA 0x558AD3-0x558B10). UT803: the nibbles are rotated so that
   byte 11's `A` comes first (VA 0x55D353-0x55D389); if positions 1 and 11 are
   `A`, `D` (VA 0x55D3EA), `LcdDisplay70B` runs (VA 0x55D3F9).

UT804.exe shows the high nibbles only in a hidden label (`USBtxt`);
UT803.exe drops them. Both handlers count consecutive reports without
payload. A 100 ms timer
(`ConTimerTimer`, UT804 VA 0x55FD4C) acts on that count once no packet
has parsed for about 300 ms: at 1900 it releases `USB Connect` and shows
"USB interface cable is securely connected, please check if the meter
is ready." (VA 0x55FDB8). The apps therefore depend on the bridge
sending reports while the meter is silent.

### 2.3 RS232 Receive Path — [VENDOR]

`FormCreate` sets `COMM2`'s `OnReceiveData` according to hidden
checkboxes that the form data sets and no code changes: UT804
`H71ARData` (VA 0x55822C, installed at VA 0x558634 because `IFVC920` is
checked), UT803 `H70BRData` (VA 0x558290, `IFUT803`).

- **UT804** `H71ARData` converts each byte to two hex digits and
  appends the low and high digits to separate strings. On a byte equal
  to 0x0D, the low-digit string holds the previous packet's LF, bytes
  1-9 and this CR. If its positions 1 and 11 are `A`, `D`, bytes 1-9
  plus `DA` go to `LcdDisplay71A`. The first packet after opening the
  port has no LF before it and is dropped. The high digits are never
  read.
- **UT803** `H70BRData` works the same way but passes the low-digit
  string as it stands (`A`, bytes 1-9, `D`) to `LcdDisplay70B`, after
  the same check (VA 0x55841E).

### 2.4 Unused FS9721 Path in UT804.exe — [VENDOR]

UT804.exe keeps a UT60A/B/C path; its `FormCreate` branch, behind the
unchecked `IFUT60AT` box, would set the caption "UT60A/B/C Interface
Program _Ver: 1.00":

- `Read` (`ReadBtnClick`, VA 0x56083C) installs the handler at
  VA 0x560A20. It buffers reports the same way, then takes 14 bytes,
  requires their high nibbles to spell `123456789ABCDE` (VA 0x560C11)
  and passes the low nibbles to `LcdDisplay60B` (FUN_0055a480), which
  matches 7-bit LCD segment patterns. In the form layout the `Read`
  button lies under the chart. `ConTimerTimer` also installs this handler (VA 0x55FD8C), but
  only if `USB Connect` is still down right after it releases it, which
  never happens.
- `H60BRData` (VA 0x557E54) is the RS232 counterpart, installed only
  for `IFUT60AT`.

UT803.exe's `Read` handler (VA 0x55CF20) collects 14 low nibbles for
the hidden label and calls no parser. UT803.exe has no 7-segment
decoder.

---

## 3. Data Format — **PROPRIETARY, NOT LCD Segments**

> **2026-06 corrections:** several subsections below were corrected by
> the protocol-correctness review — see §7.4 for the full list with
> evidence. In particular: nibble 9 bit 2 is the **sign** (not HOLD);
> nibble 1 = 0xA is an **overload frame** (not an AC flag mode); the
> decimal position counts **from the left**; the UT804 mode table was
> wrong for codes 6/7/8/9/A/B/C/D/F; and the UT803 uses an entirely
> different layout (§5). The Rust parser follows the corrected
> derivation; the tables below retain their original [VENDOR] markers
> where still accurate.

**Critical finding:** the data nibbles do NOT contain raw LCD segment
data. The firmware sends **structured measurement
data** with explicit mode codes, range codes, and digit values.

This was confirmed by:
1. Nibble 7 contains integer mode codes 1-15 (verified from UT804.exe binary
   constants at VA 0x0055a2a0-0x0055a2f4)
2. Nibble 6 contains integer range codes 0-7
3. Nibbles 1-5 contain digit values 0-9 or flag codes, not 7-segment patterns
4. Nibbles 10-11 contain fixed format markers (0x0D, 0x0A)
5. The mode detection code in both UT803.exe and UT804.exe reads nibble 7
   directly as a mode code, never as segment data

UT804.exe also contains a 7-segment decoder (`LcdDisplay60B`,
FUN_0055a480), reached only from its unused UT60A/B/C path (§2.4).
UT803.exe has none. *Corrected 2026-09-16: earlier revisions said UT803.exe
had one too.*

### 3.1 Nibble Layout — [VENDOR]

| Nibble | Content | Values | Notes |
|--------|---------|--------|-------|
| 1 | Flag or digit | `0`-`9` = digit, `A` = AC/DC flag indicator | See §3.3 |
| 2 | Flag or digit | `0`-`9` = digit, `C` = AC (when nib1=`A`) | See §3.3 |
| 3 | Digit | `0`-`9` | |
| 4 | Digit | `0`-`9` | |
| 5 | Digit or blank | `0`-`9` = digit, `A` = blank/not displayed | |
| 6 | Range code | `0`-`7` | Selects sub-range within mode |
| 7 | Mode code | `1`-`F` | See §3.4 |
| 8 | AC/DC indicator | `0`-`3` | See §3.5 |
| 9 | Status flags | `0`-`F` | See §3.6 |
| 10 | Format marker | `D` (0x0D) | Always this value for valid data |
| 11 | Format marker | `A` (0x0A) | Always this value for valid data |

The packet ends after nibble 11; there are no nibbles 12-14 (§2.1).

### 3.2 Digit Encoding — [VENDOR]

Digit nibbles (1-5) carry BCD-like values:
- `0`-`9`: digit character '0'-'9'
- `A` (0x0A): blank/flag indicator (context-dependent)
- `B`-`F`: may encode sign or other flags — [UNVERIFIED]

The display value is constructed from nibbles 1-5 (or 2-5 when nibble 1
is a flag indicator). The decimal point position is determined by the
range code (nibble 6) within each mode.

**Negative values:** Sign encoding is [UNVERIFIED]. Two Ghidra passes
over `ut803-decompiled.txt` and `ut804-decompiled.txt` (226K / 227K
lines each) together establish:

- The display formatter (`FUN_00490730` in UT803.exe / `FUN_0049091c`
  in UT804.exe) switches on a single 0–15 value and prepends `-` in
  exactly four of the sixteen cases: **1, 5, 8, 9** (cases 2/3/6/7/13
  place the minus in the middle of the formatted string, and
  0/4/14/15 use a parenthesised format). UT803 and UT804 are
  byte-identical here.
- That 0–15 value is read from a **global pointer** at display time —
  UT803 `*PTR_DAT_005659c4` (line 101186), UT804 `*PTR_DAT_005699c4`
  (line 101276). A full cross-reference grep for those addresses
  returns **exactly one hit each**: the read above. No writer appears
  anywhere in the decompile, including near the HID / USB plumbing.
- The `*-gap-decompiled.txt` files contain only Ghidra build logs, no
  additional code.
- The visible frame-parsing path (around `FUN_00558a7c` in UT804)
  never accesses nibbles 12, 13, or 14.

The write to the sign global therefore lives outside what Ghidra
reconstructed — most likely in an HID-receive callback Ghidra marked as
non-returning, or in an inline-assembly stub. Progressing from here
requires either a raw disassembler pass (not a decompiler) over the
binary in that region, or a real-device capture of a known negative
reading.

Possibilities still open, none of which have decompile evidence picking
between them:

- A specific nibble value in the digit slots (e.g. `0x0B` = minus sign)
- A bit in nibble 9 or another status nibble
- A bit in one of nibbles 12-14 (never read in the visible path)

Until a real-device trace of a known negative reading arrives, the Rust
parser treats every reading as positive; implementing a speculative
decode risks negating valid positive readings, which would be worse
than the current "display magnitude only" behaviour.

### 3.3 Nibble 1-2 Flag Mode — [VENDOR]

When nibble 1 = `A` (0x0A), the frame uses flag mode for nibbles 1-2:
- Nibble 2 = `C` (0x0C): AC measurement (AC indicator shown)
- Nibble 2 ≠ `C`: DC measurement (DC indicator shown)

When nibble 1 ≠ `A`, nibbles 1-5 are all digit values (5 digits total).

`LcdDisplay71A` checks nibble 10 = `D` and nibble 11 = `A` before
parsing. Its callers guarantee both: the USB handler checks them, and
the RS232 handler appends a literal `DA` (§2.2, §2.3). So the check
always passes. `LcdDisplay70B` has no such check; the UT803
handlers make it.

### 3.4 Mode Codes (Nibble 7) — [VENDOR]

#### UT804 — 15 Modes

| Code | Mode | UT804 unit string | Confirmed |
|------|------|-------------------|-----------|
| 1 | DC V | `V` | [VENDOR] |
| 2 | AC V | `V` | [VENDOR] |
| 3 | DC mV | `mV` | [VENDOR] |
| 4 | Resistance (Ω) | `*` (Ω in custom font) | [VENDOR] |
| 5 | Capacitance | (nF/µF/mF from range) | [VENDOR] |
| 6 | Diode | `#` (diode in custom font) | [VENDOR] |
| 7 | Frequency (Hz) | `Hz` | [VENDOR] |
| 8 | Duty Cycle (%) | `%` | [VENDOR] |
| 9 | hFE | | [VENDOR] |
| A (10) | Temperature | | [VENDOR] |
| B (11) | DC µA | | [VENDOR] |
| C (12) | Current (A) | `Hz` (likely bug in font table) | [VENDOR] |
| D (13) | Continuity | `?` (beep in custom font) | [VENDOR] |
| E (14) | ADP / Logic | `W` | [VENDOR] |
| F (15) | AC mA | `mA%` | [VENDOR] |

Note: Some unit strings appear incorrect (e.g., mode 12 = "Hz" for current).
This is because the UT804.exe uses custom TrueType fonts (unit_a2.ttf,
unit_a3.ttf, unit.ttf) where ASCII characters map to measurement symbols.
The raw ASCII values don't correspond to their visual appearance.

#### UT803 — Modes [DEDUCED]

The UT803 uses the same nibble 7 mode code scheme. Unit strings found in
UT803.exe binary:
- `V`, `mV` — voltage
- `uA`, `mA` — current (µA, mA)
- `*`, `k*`, `M*` — resistance (Ω, kΩ, MΩ in custom font)
- `Hz`, `kHz`, `MHz` — frequency
- `nF`, `uF`, `mF` — capacitance
- `kRPM` — tachometer/RPM (unique to UT803)
- `#` — diode (custom font)
- `?` — continuity (custom font)

The UT803 likely has fewer than 15 modes (no ADP/Logic mode, possibly no
Temperature mode). Exact mode list [UNVERIFIED] without hardware.

### 3.5 AC/DC Indicator (Nibble 8) — [VENDOR]

| Value | Meaning | Display string (UT804) |
|-------|---------|------------------------|
| 0 | Default (mode-dependent) | "DC" for V/mV modes, blank for others |
| 1 | AC | "AC" |
| 2 | DC (explicit) | "DC" |
| 3 | AC+DC | "AC+DC" |

The "AC+DC" string at value 3 was found as a literal in UT804.exe
(line 224240 in decompilation).

### 3.6 Status Flags (Nibble 9) — [VENDOR]

Nibble 9 is decomposed as individual bits in the UT804 parser
(FUN_00558a7c, lines 224244-224283):

| Bit | Mask | Flag | Confirmed |
|-----|------|------|-----------|
| bit 3 | 0x8 | Unknown (stripped first, no visible effect) | [UNVERIFIED] |
| bit 2 | 0x4 | **Negative sign** (duty-% selector in frequency mode). Corrected 2026-06 — previously misread as HOLD; the "'-' indicator" it lights is the sign (`LcdFH`), and the bit's value is prepended to the parsed number (see §7.4) | [VENDOR] |
| bit 1 | 0x2 | Unknown | [UNVERIFIED] |
| bit 0 | 0x1 | AUTO | [VENDOR] — shows "AUTO" text |

The bit decomposition logic:
```
value = parseInt(nibble9)  // 0-15
if value >= 8: value -= 8  // strip bit 3
if value >= 4:
    value -= 4             // strip bit 2 → HOLD active
if value == 1:             // bit 0 → AUTO active
```

Additional flags (MIN, MAX, REL, Low Battery) may be in nibbles 12-14
— [UNVERIFIED].

### 3.7 Range Code (Nibble 6) — [VENDOR]

The range code (0-7) selects the sub-range within each mode and determines
the decimal point position. From the UT804 mode switch statement
(lines 223961-224185):

#### DC V / AC V (modes 1-2):
| Range | Decimal position | Full-scale | Confirmed |
|-------|------------------|------------|-----------|
| 1 | 0 | | [VENDOR] |
| 2 | 1 | | [VENDOR] |
| 3 | 2 | | [VENDOR] |
| 4 | 3 | | [VENDOR] |

#### Resistance (mode 4):
Range values 1-6 select Ω, kΩ, MΩ sub-ranges with varying decimal
positions. Exact mapping [UNVERIFIED] without hardware.

#### Capacitance (mode 5):
Range values 1-7 select nF, µF, mF with varying decimal positions.

#### Current modes:
Range values select µA, mA, A sub-ranges.

Detailed range-to-unit/decimal tables require hardware verification for
each mode.

---

## 4. Transport Initialization — [VENDOR]

### 4.1 CH9325 Configuration

One feature report per `USB Connect` press or release (§1.2): 2400 baud
from UT804.exe, 19200 baud from UT803.exe, both as
`xx, rate LE, 00, 00, 03, 00, 00, 00, 00`.

### 4.2 Data Streaming

After init, the meter streams measurement packets continuously at ~2-3 Hz
(per UT803/UT804 manuals).

The feature report is the only thing either app sends: its
`HidD_SetFeature` call is the only HID output in either binary, and no
form code writes to the HID device or the serial port. `WriteFile`'s
only callers are the runtime's file and console output; the apps'
serial component never calls it. No trigger byte and no command reach the meter. The UT803
manual has the user press the meter's RS232 button to start data output
[KNOWN].

---

## 5. Differences Between UT803 and UT804

| Feature | UT803 | UT804 |
|---------|-------|-------|
| Display count | 6000 (3¾ digit, max 5999) | 4000 (3¾ digit, max 3999) |
| Mode count | Fewer (exact list TBD) | 15 modes |
| RPM mode | Yes (`kRPM` unit string) | Not seen |
| ADP/Logic mode | Not seen | Yes (mode 14) |
| Temperature | TBD | Yes (mode 10) |
| AC+DC mode | TBD | Yes (nibble 8 = 3) |
| CH9325 and RS232 rate | 19200 (§1.2) | 2400 (§1.2) |
| Parser position k | byte k-1 (§2.1) | byte k (§2.1) |
| 7-segment decoder in the app | None | Unused UT60A/B/C path (§2.4) |

Both send 11-byte packets ending CR LF (§2.1); the payload layouts
differ (§7.4 item 4).

---

## 6. Custom Font Mapping — [VENDOR]

Both apps use custom TrueType fonts (unit.ttf, unit_a2.ttf, unit_a3.ttf,
unit_372.ttf) where ASCII characters map to measurement symbols:

| ASCII | Visual symbol |
|-------|---------------|
| `*` | Ω (Ohm) |
| `#` | Diode symbol |
| `?` | Continuity/beep symbol |
| `&` | Unknown symbol |
| `@` | AC indicator |
| `$` | Unknown (flag-related) |
| `W` | Unknown (ADP mode unit) |

These mappings were determined from binary string extraction and
cross-referencing with mode detection logic.

---

## 7. Implementation Notes

### 7.1 Packet Extraction

Split the byte stream after each CR LF into 11-byte packets and keep the
low nibbles (§2.1). The apps rely on empty reports between packets
instead (§2.2); splitting on CR LF does not depend on report timing. No
checksum validation.

### 7.2 Data Parsing

Parse the proprietary data nibbles, NOT LCD segments:
1. Validate format markers: nibble 10 = 0x0D, nibble 11 = 0x0A
2. Read mode code from nibble 7
3. Read range code from nibble 6
4. Read AC/DC from nibble 8
5. Read status flags from nibble 9
6. Extract digits from nibbles 1-5 (handling flag mode when nibble 1 = 0x0A)
7. Construct display value with decimal point from range table

### 7.3 What Needs Hardware Verification

- Negative value encoding (sign bit location) — see §3.2
- Exact mode list for UT803
- Range-to-decimal-point tables for all modes
- Status flag bits (MIN, MAX, REL, Low Battery) — see §7.4
- Whether the meter needs anything sent (the apps send nothing, §4.2)
- Streaming rate
- Line format on the wire (§1.2)
- Digit encoding for values > 9 (0xA = blank confirmed, others unknown)
- Whether nibble 4 = 'B' guard condition has meaning

### 7.4 RESOLVED (2026-06): Sign, Nibbles 12-14, and the Two-Model Split

The 2026-06 protocol-correctness review closed this section's open
questions by recovering the analyzed binaries (wine administrative
install of the vendor installers; MD5s match §9 exactly), raw-
disassembling the cross-referenced globals, resolving every string
constant, and rendering the bundled LCD fonts. Headline results, each
re-derived independently by an adversarial second pass:

1. **The "sign global" was a red herring.** `*PTR_DAT_005659c4`
   (UT803) / `*PTR_DAT_005699c4` (UT804) is Delphi SysUtils'
   `NegCurrFormat` locale global — its writer is the RTL locale init
   (UT803 VA 0x40E217, reading `GetLocaleInfo(LOCALE_INEGCURR)`), and
   the 16-case `-` switch in `FUN_00490730`/`FUN_0049091c` is the RTL's
   negative-**currency** formatter. Nothing to do with the wire
   protocol. (The cluster: 0x566688 CurrencyFormat, 0x566689
   NegCurrFormat, 0x56668A ThousandSeparator, 0x56668B
   DecimalSeparator.)
2. **The real sign is in-band.** UT804: nibble 9 bit 2 (the bit §3.6
   previously labeled HOLD — the vendor lights the `LcdFH` sign
   indicator and prepends `"-"`, VA 0x55a3dc; negative overload
   comparand `"-0@"` at 0x55a434; ut804-decompiled.txt:224244-224374).
   UT803: nibble 8 bit 2 (ut803-decompiled.txt:224458-224469). HOLD on
   the UT803 is nibble 9 bit 3 (`LCDHold`, line 225086); HOLD's wire
   encoding on the UT804 appears in **neither** parser and remains
   unknown.
3. **Nibbles 12-14 are genuinely never read** — confirmed with the
   correct access pattern: the frame arrives as a Delphi string of hex
   characters parsed via 1-based `Copy(s, idx, 1)` (which is why
   pointer-arithmetic greps found nothing). The UT804 parser reads
   indices 1-11 only; the UT803 parser reads 2-10. The 0xD/0xA markers
   are the low nibbles of CR/LF — an ASCII-protocol trailer, which
   explains the unused tail. *2026-09-16: the live receive paths take
   11 bytes, so there are no nibbles 12-14, and the UT803's positions
   2-10 are bytes 1-9 (§2.1).*
4. **The frame parse is model-specific.** UT804.exe contains three
   protocol paths selected by UI control (Delphi RTTI method table:
   `H71ARData`/`LcdDisplay71A` = the UT804 structured parser;
   `H60BRData`/`LcdDisplay60B` = a 7-segment decoder for legacy
   UT60A/B/C support; `H70BRData`/`LcdDisplay70B` = dead). *2026-09-16:
   hidden checkboxes and buttons select them: `H71ARData` on RS232 and
   `LcdDisplay71A` behind `USB Connect`; `LcdDisplay60B` only on the
   unused UT60A/B/C path (§2.2-§2.4).* The UT803
   uses its own layout (range=nibble 2, digits=nibbles 3-6, different
   mode-code meanings — see §5).
5. **Decimal positions count from the left** (point after digit
   `pos+1`), per the display assembly at ut804-decompiled.txt:
   224289-224356 (point slots LcdP0-LcdP3 interleaved with the digit
   labels). The previous places-from-right reading inverted every
   table.
6. **Nibble 1 = 0xA marks an overload frame** (digits forced to
   "0L"/"L0" via the LCD font where '@' renders as 'L'); `"0@"` → +OL,
   `"-0@"` → −OL, `"@0"` → 0.0. It is not an "AC flag mode".
   *2026-09-16: nibble 2 = C gives `"@0"` (0.0, shown "L0."), any other
   nibble 2 gives `"0@"` (overload, negative with the sign bit);
   nibbles 3-5 are ignored (VA 0x558B3B-0x558B98, 0x559ABE-0x559B2A).*
7. **The UT804 mode table was wrong for 9 of 15 codes.** Corrected via
   unit-string constants + font glyphs (`#`=°C, `?`=°F, `)`=diode,
   `&`=beeper, `*`=Ω, `@`='L', `$`=battery): 6=Temp °C, 7=µA, 8=mA,
   9=A, A=Continuity, B=Diode, C=Frequency (duty-% via nibble 9 bit 2,
   reused since negative frequency is impossible), D=Temp °F,
   E=unknown glyph (possibly hFE), F="mA%" (likely 4-20 mA loop).
   Frequency unit boundaries: ranges 0-1 Hz, 2-4 kHz, 5-7 MHz; Ω:
   range 1 Ω, 2-4 kΩ, 5-6 MΩ.

8. **The frame-string-builder is confirmed (2026-06 follow-up).** The
   one remaining inferred link — that the parser's positional
   `Copy(s, idx, 1)` reads wire nibbles in FS9721 index order — was the
   receive handler `H71ARData` (VA 0x55822c; *2026-09-16: the serial
   port's handler, not the HID one; it keeps digits in arrival order,
   syncs on the CR byte and never tests high nibbles — see §2.3*), which RTTI names but
   Ghidra's call graph never reached (it appears in *neither*
   decompile). Raw-disassembling it from the recovered binary shows it
   converts each received byte to a 2-char hex string
   (`FUN_00409230` → `FUN_004090d0`, an `IntToHex`-style formatter with
   `add dl,0x30`), peels the data nibble keyed by the byte's high-nibble
   index, accumulates it into the Delphi string global `DAT_0056b698`,
   and frame-syncs on the hex markers `"0D"`/`"DA"`/`"AD"` (= the 0x0D/
   0x0A trailer). So the string the parser reads positionally *is* the
   ordered sequence of wire data nibbles — the structured-nibble model
   (not LCD segments) and the 1-based-`Copy` indexing are both
   vendor-confirmed, not assumed.

The Rust `ut80x` module now implements separate UT803/UT804 parsers
with these corrections. Clean-room note: approval was given to consult
the sigrok FS9721 decoder and the FS9721-LP3 datasheet for this family,
but the resolution above required neither — it is derived entirely
from the vendor binaries, their fonts, and the existing decompiles, so
the clean-room boundary for UT803/UT804 remains unopened.

The section below is kept for the historical record of the gap.

### 7.4.1 Historical: Nibbles 12-14 and Secondary Status Bits (superseded)

Two Ghidra passes over `ut803-decompiled.txt` and `ut804-decompiled.txt`
(226K / 227K lines each, 2026-04-19) have established a negative
finding about the upper nibbles:

- **Nibbles 12, 13, 14 are never read in the visible decompile.** A
  full grep for `param_1 + 0xB / 0xC / 0xD / 0xE` (and the short-
  pointer equivalents) against the frame-parse function (`FUN_00558a7c`
  in UT804 / its UT803 peer) returns no hits. The visible parser only
  consumes nibbles 1-11.
- **The display formatter (`FUN_00490730` / `FUN_0049091c`) reads a
  precomputed 0-15 byte from a global pointer** (`*PTR_DAT_005659c4` /
  `*PTR_DAT_005699c4`) and uses it to choose between sixteen format
  strings — four of which (cases 1, 5, 8, 9) prepend `-`. The cross-
  reference grep finds exactly one hit per global: the read above. No
  writer appears anywhere, including at the HID-receive sites.
- **The `*-gap-decompiled.txt` files are Ghidra build logs, not code**,
  and contain no additional function bodies.

The implication: the write path that populates the sign global — and
plausibly the secondary status bits implied by the "may carry
additional flags" note at the top of this file — lives in code Ghidra
reconstructed as non-returning / inlined / as an assembly stub. A
future investigation should either:

1. Re-run Ghidra with call-graph and data-flow recovery tuned to be
   more aggressive, specifically around the HID transfer callbacks;
2. Use a raw disassembler (not a decompiler) on the regions
   cross-referenced by the globals, to see the asm-level store; or
3. Capture a real UT803 / UT804 reading a known negative value (and a
   second reading with MIN, MAX, REL, and low-battery each toggled in
   turn) — four of those captures would nail down exactly which
   nibble/bit carries each flag.

Until one of those happens, this spec leaves nibbles 12-14 as
`[UNVERIFIED]` and the Rust `ut80x` parser reports every reading as
positive.

---

## 8. Cross-Reference with Community Sources

Consulted after the vendor analysis above, for validation only (approved
2026-09-16, after issue #16's first UT804 report). The sigrok column is
libsigrok's code unless it says "wiki".

| Finding | Our RE | sigrok | `UT804.LOG` | Agreement |
|---------|--------|--------|-------------|:---------:|
| Wire framing | 11 bytes ending CR LF, low nibbles only (§2.1) | UT71x: 11 bytes ending CR LF, for the RS232 and the UT-D04 (CH9325) cable | 11 bytes ending CR LF | ✓¹ |
| Line format | 2400 baud; the app opens its port at 8N1 but reads only low nibbles, except CR (§1.2) | 2400 7O1 | 2400 7O1 | ✓ rate; 7O1 new |
| Payload layout | Nibbles 1-11 = bytes 1-11 (§2.1, §3.1) | Bytes 0-4 digits, 5 range, 6 function, 7 coupling, 8 flags, 9-10 CR LF | Same | ✓² |
| Function codes | 1-F (§3.4); E and F uncertain | 0-15, 14 = power, 15 = loop current | 1-9, `:` continuity, `;` diode, `<` Hz, `=` °F, `?` 4-20 mA %; no power on the UT804 | ✓; 1 = V DC, 2 = V AC new |
| Range tables | §3.7 | — | Per function | ✓ (log) |
| Coupling (nibble 8) | 0 = per mode, 1 AC, 2 DC, 3 AC+DC (§3.5) | Bit 0 AC, bit 1 DC | Same | ✓ |
| Status (nibble 9) | Bit 0 AUTO, bit 2 sign, bits 1 and 3 unknown (§3.6) | Bit 0 AUTO, bit 1 MAN, bit 2 sign | Same | ✓; bit 1 new |
| Duty cycle | Hz mode with the sign bit (§7.4) | Same | Same | ✓ |
| Digit values A, C, F | A = blank or flag, B-F unknown (§3.2) | — | `:` blank, `<` 'L', `?` 'H' | ✓ A; C, F new |
| Overload | Nibble 1 = A: overload unless nibble 2 = C, which gives 0.0 shown "L0." (§7.4) | `::0<:` overload, `:<0::` underload | `::0<:` overload; 4-20 mA `:<0::` "L0", `:?1::` "HI" | ✓³ |
| Nibbles 12-14 | No such bytes (§2.1) | No such bytes | No such bytes | ✓ |
| HOLD | Wire encoding unknown (§7.4) | — | Nothing transmitted while HOLD is on | New |
| REL | Unknown | — | Never transmitted | New |
| 4000-count display | Nibble 5 = A is blank (§3.1) | Byte 4 = `:` | — | ✓ |
| CH9325 report layout | Apps: rate, `00 00`, `03`; SDK DLL: rate, `03` (§1.2) | `[lo, hi, 00, 00, 03]` | — | ✓ apps⁴ |
| CH9325 report byte 5 | `03` in the apps' report, `00` in the SDK DLL's (§1.2); meaning unknown | Wiki: data-bit count, 0-3 = 5-8 bits | — | New⁴ |
| Idle CH9325 reports | The apps end each packet at a report without payload and give up after 1900 in a row (§2.2) | `F0` carries no data; wiki: at least one every 12 ms | — | ✓; 12 ms new⁴ |
| Parity bit through the CH9325 | Ignored: only low nibbles are used (§2.1) | Cleared (bit 7) for UT71x | — | ✓ |

¹ Agreement since the 2026-09-16 handler decompile (§2). Earlier
revisions gave the 14-byte frames of UT804.exe's unused UT60A/B/C path
(§2.4) as the wire format, which both sources contradicted.

² The §3 nibbles are the low nibbles of the 11 bytes, in order.
`UT804.LOG` lists 36 real packets across 9 dial positions and their
sub-functions.

³ The vendor code reads `::0<:` (A A 0 C A) as an overload and
`:<0::` (A C 0 A A) as 0.0 shown "L0." (§7.4 item 6). Earlier revisions
of this table read the rule the other way round.

⁴ Lukas Schwarz sends `60 09 00 00 03`, `he2325u.cpp` the rate as 32 bits
then `0x03` ("3 = enable?"); libsigrok calls the byte "unknown, always
0x03". The sigrok wiki's WCH CH9325 page reads bytes 3-4 as probably
parity and stop bits (often omitted or zero) and byte 5 as the data-bit
count, and notes vendor software sending `00 00 03`; by that reading the
SDK DLL's layout asks for 5 data bits. The page also says the chip falls
back to 2400 baud yet needs the rate set, and sends an `F0` report at
least every 12 ms while the UART is silent. Lukas Schwarz also describes
the `F0` idle reports.

Reference implementations:

- [sigrok libsigrok](https://github.com/sigrokproject/libsigrok) — C; `src/dmm/ut71x.c`, the UT804 entries in `src/hardware/uni-t-dmm/api.c` and `src/hardware/serial-dmm/api.c` (commit ca7d442692, 2020-02-08, no hardware note), CH9325 set-up in `src/hardware/uni-t-dmm/protocol.c`
- [tmatejuk/ut804_linux_logger](https://github.com/tmatejuk/ut804_linux_logger) — C, RS232 logger; its `UT804.LOG` is a German write-up of real UT804 packets (author unknown)
- [sigrok wiki, WCH CH9325](https://sigrok.org/wiki/WCH_CH9325) — CH9325 configuration bytes and report framing
- [Lukas Schwarz, UT61B analysis](https://lukasschwarz.de/ut61b) — HE2325U/CH9325 set-up and report format
- [thomasf/uni-trend-ut61d](https://github.com/thomasf/uni-trend-ut61d) — C++, `he2325u/he2325u.cpp` HE2325U/CH9325 reader

---

## 9. Sources

- UT804.exe V2.00 (MD5: 9ef22cff570ba9e8b79e6f1867aad2e5) — Ghidra
  decompilation + binary constant extraction
- UT803.exe V1.01 (MD5: 6dd98644d82edaa4fb0e2e230cf68bc6; the app
  calls itself Ver 1.10) — Ghidra decompilation + binary constant
  extraction
- Both executables, 2026-09-16 — Ghidra 12.1.3 decompilation of the form's
  published methods and the handlers they install, seeded from the Delphi
  RTTI; disassembly of the conditions; the form resources
- UT803 operating manual — RS232 settings and RS232 button
- CH9325 transport analysis — see `../uci-bench-family/reverse-engineered-protocol.md`
