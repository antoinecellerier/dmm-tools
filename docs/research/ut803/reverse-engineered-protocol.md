# UT803 / UT804: Reverse-Engineered Protocol Specification

Protocol specification for the UNI-T UT803 (6000-count) and UT804 (4000-count)
bench multimeters.

Based on:
- Ghidra decompilation of UT803.exe V1.01 and UT804.exe V2.00 (standalone PC apps)
- Binary constant extraction from both executables
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
- Already implemented in `ch9325.rs`

### 1.2 UART Parameters — [VENDOR]

The UT804.exe init function (FUN_00560668) sends bytes `[0x60, 0x09, 0x03]`
to the CH9325. The value `0x0960` (little-endian) = 2400 decimal, confirming:
- **Baud rate: 2400**
- Data bits: 7 (inferred from FS9721 convention)
- Parity: Odd (inferred from FS9721 convention)
- Stop bits: 1

The CH9325 bridge handles parity stripping — the application receives clean
data bytes.

---

## 2. Frame Format

### 2.1 FS9721-Style 14-Byte Framing — [VENDOR]

Both meters send 14-byte frames using the Fortune Semiconductor FS9721
framing scheme:

```
Byte N: [index:4][data:4]
         ^^^^^^   ^^^^^^
         bits 7-4  bits 3-0
         = N       = payload nibble
```

Each byte's high nibble is its index (1 through 14 = 0x1 through 0xE).
This provides frame synchronization — the receiver validates that 14
consecutive bytes have sequential indices 1-14.

**No checksum.** Frame integrity relies on the index nibble validation.

### 2.2 Frame Validation — [VENDOR]

The UT804.exe HID callback at VA 0x560A20 validates frames using the string
`"123456789ABCDE"` — each byte's high nibble (shifted right 4) must match
the corresponding character in this sequence. Confirmed by assembly analysis
at VA 0x560BC1/0x560BF0 (`CMP EBX, 14` loop).

---

## 3. Data Format — **PROPRIETARY, NOT LCD Segments**

**Critical finding:** Despite using FS9721 framing, the data nibbles do NOT
contain raw LCD segment data. The firmware sends **structured measurement
data** with explicit mode codes, range codes, and digit values.

This was confirmed by:
1. Nibble 7 contains integer mode codes 1-15 (verified from UT804.exe binary
   constants at VA 0x0055a2a0-0x0055a2f4)
2. Nibble 6 contains integer range codes 0-7
3. Nibbles 1-5 contain digit values 0-9 or flag codes, not 7-segment patterns
4. Nibbles 10-11 contain fixed format markers (0x0D, 0x0A)
5. The mode detection code in both UT803.exe and UT804.exe reads nibble 7
   directly as a mode code, never as segment data

Both UT803.exe and UT804.exe also contain 7-segment decode functions
(FUN_0055a480 in UT804, similar in UT803), but these appear to be for an
alternative display mode or legacy compatibility — the primary USB HID data
path uses the proprietary format.

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
| 12-14 | Unknown | | [UNVERIFIED] — may carry additional flags |

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

The guard condition at the start of both UT803.exe and UT804.exe parsers
checks nibble 10 = `D` AND nibble 11 = `A` before entering the main parse
path. When this guard fails, the code takes an alternate path — purpose
[UNVERIFIED].

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
| bit 2 | 0x4 | HOLD | [VENDOR] — shows '-' indicator |
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

The UT804.exe init function (FUN_00560668) sends to the CH9325:
```
[0x60, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]  // 9 bytes zeroed first
 local_4c[0] = 0x60  // baud rate low byte
 local_4c[1] = 0x09  // baud rate high byte → 0x0960 = 2400
 local_48 = 0x03     // config byte (7-O-1?)
```

### 4.2 Data Streaming

After init, the meter streams measurement frames continuously at ~2-3 Hz
(per UT803/UT804 manuals). No trigger byte is needed — the meter starts
streaming as soon as the CH9325 baud rate is configured.

This differs from UT8802/UT8803 (which require a 0x5A trigger byte) and
UT171/UT181A (which require a connect command).

**[UNVERIFIED]:** The 0x5A trigger byte may still be useful or required.
The CH9325 transport's dual-baud probing (2400 primary with 0x5A, 19200
fallback) should be tested against real hardware.

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

Both use the same proprietary data format over FS9721 framing.

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

### 7.1 Frame Extraction

Use the standard FS9721 14-byte frame extractor: scan for 14 consecutive
bytes where byte N has high nibble = N (1-14). No checksum validation.

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

- Negative value encoding (sign bit location)
- Exact mode list for UT803
- Range-to-decimal-point tables for all modes
- Status flag bits (MIN, MAX, REL, Low Battery)
- Nibbles 12-14 content
- Whether 0x5A trigger byte is needed
- Streaming rate
- Digit encoding for values > 9 (0xA = blank confirmed, others unknown)
- Whether nibble 4 = 'B' guard condition has meaning

---

## 8. Sources

- UT804.exe V2.00 (MD5: 9ef22cff570ba9e8b79e6f1867aad2e5) — Ghidra
  decompilation + binary constant extraction
- UT803.exe V1.01 (MD5: 6dd98644d82edaa4fb0e2e230cf68bc6) — Ghidra
  decompilation + binary constant extraction
- CH9325 transport analysis — see `../uci-bench-family/reverse-engineered-protocol.md`
