# UT803 / UT804: Reverse-Engineered Protocol Specification

Protocol specification for the UNI-T UT803 (6000-count) and UT804 (40000-count)
bench multimeters.

Based on:
- Ghidra decompilation of UT803.exe V1.01 and UT804.exe V2.00 (standalone PC apps)
- Ghidra decompilation of both apps' form event handlers, their disassembly
  and their form resources (2026-09-16)
- Binary constant extraction from both executables
- The UT803 operating manual's serial port settings and RS232 button
- The UT804 operating manual's display counts, rotary switch, buttons and
  ranges (Tables 2-1 to 2-3)
- Bytes a UT804 sent over its CH9325 cable (issue #16, 2026-09-16)
- Two capture reports from a UT804 walked through every dial position, its
  LCD read back beside each (issue #16, 2026-09-18)
- CH9325 HID transport analysis (see `../uci-bench-family/reverse-engineered-protocol.md`)
- UNI-T's UT804 interface protocol sheet "UT804接口协议", published on the
  UT800 series page of instruments.uni-trend.com.cn — see
  `reverse-engineering-approach.md`

Confidence levels:
- **[VENDOR]** — confirmed by analyzing UNI-T's official software binaries
- **[VENDOR-DOC]** — stated in UNI-T's UT804 interface protocol sheet (UT804
  only)
- **[DEDUCED]** — logical inferences from available evidence
- **[UNVERIFIED]** — requires real device testing to confirm
- **[HARDWARE]** — seen on a real meter (the UT804 of issue #16)

---

## 1. Transport Layer

### 1.1 USB HID Bridge — [VENDOR]

Both meters use the WCH CH9325 USB-to-UART HID bridge:
- VID: 0x1A86, PID: 0xE008
- 8-byte HID reports with 0xF0+length RX framing

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

Observed on a UT804 (issue #16, 2026-09-16): a host sending the DLL's
layout (`00 60 09 03 00 00 00 00 00 00`) and one sending the apps'
(`00 60 09 00 00 03 00 00 00 00`) both got clean bytes at 2400
baud. That does not show the bridge reads either layout: 2400 may be its
default.

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
is 0 (§2). The UT804 manual gives no line format. Observed on a UT804
(issue #16, 2026-09-16): the line format is 7O1 — bit 7 of all 363
bytes received is odd parity over the other seven, delivered by the
bridge as an eighth bit.

**[VENDOR-DOC]** The UT804 sheet gives the line as 2400 baud, 7 data bits,
odd parity. It gives no stop-bit count.

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

Observed on a UT804 (issue #16, 2026-09-16): the data bytes are
`0x30`-`0x3F` plus the parity bit (§1.2); CR arrives as `0D`, LF as
`8A`.

**[VENDOR-DOC]** The UT804 sheet gives the same frame: 11 bytes, the high
nibble of the 9 data bytes fixed at `0011`, the low nibble carrying the
data, and a fixed `0D 0A` end marker. It lists no checksum. It numbers the
bytes from 0, so its byte n is position n+1 below.

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

Observed on a UT804 (issue #16, 2026-09-16): the CH9325 delivers one
byte per report, about 50 `F0` reports between packets, and no empty
report inside a packet.

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
decoder. Its own `H60BRData` (VA 0x5580F0), installed by the `UT632` and
`IFUT60E` boxes, ends each frame at a byte whose high nibble is E and
decodes nothing (`../ut632/reverse-engineered-protocol.md`).

---

## 3. Data Format — **PROPRIETARY, NOT LCD Segments**

> **2026-06 corrections:** several subsections below were corrected by
> the protocol-correctness review — see §7.4 for the full list with
> evidence. In particular: nibble 9 bit 2 is the **sign** (not HOLD);
> nibble 1 = 0xA is an **overload frame** (not an AC flag mode); the
> decimal position counts **from the left**; the UT804 mode table was
> wrong for codes 6/7/8/9/A/B/C/D/F; and the UT803 uses an entirely
> different layout (§5). The tables below retain their original
> [VENDOR] markers where still accurate.

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
| 1 | Digit | `0`-`9` = digit, `A` = blank: an overload or LO packet | See §3.3 |
| 2 | Digit | `0`-`9` = digit, `A` = blank, `C` = `L` | See §3.3 |
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

**[VENDOR-DOC]** The UT804 sheet names positions 1-9 `LCD(1)`-`LCD(5)`,
`Range`, `Function`, `State` and `State2`. It gives `LCD(1)` as 0-4 and the
other four digits as 0-9, which fits a 40000-count display, and says nothing
of the blank (`A`) and `L` (`C`) values, which come from the app and the
meter (§3.2, §3.3).

### 3.2 Digit Encoding — [VENDOR]

Digit nibbles (1-5) carry BCD-like values:
- `0`-`9`: digit character '0'-'9'
- `A` (0x0A): blank
- `C` (0x0C): drawn as `L` in overload and LO packets (§3.3) [HARDWARE]
- `B`, `D`-`F`: the vendor shows zeros for `B` in nibble 4 (§7.3), and
  `F` appears in the 4-20 mA "HI" pattern (§8); what the LCD draws for
  either is unknown

The decimal point position is determined by the range code (nibble 6)
within each mode (§3.7).

**Negative values:** nibble 9 bit 2 is the sign (§3.6, §7.4 item 2)
[HARDWARE]: a UT804 sent it with -12.041 V and -24.196 V DC that its LCD
showed (issue #16, 2026-09-18), and on zero readings, where the LCD shows
the minus too (`-000.00 µA`, `-00.000 mA`, `-00.000 A`). The history of
the search is in §7.4.1.

### 3.3 Overload and LO Packets — [VENDOR]

Nibble 1 = `A` marks an overload or LO packet (§7.4 item 6): the vendor
reads nibble 2 = `C` as 0.0 (LO) and anything else as an overload,
negative with the sign bit. *Corrected 2026-06: earlier revisions read
nibble 2 as an AC/DC flag.*

The UT804's LCD draws nibbles 1-5 as they are — `A` blank, `C` as `L`, a
digit as itself — with the range's decimal point after digit position
`pos+1`, blank positions counted (§3.7), and the sign in front
[HARDWARE]. Issue #16's reporter read four such packets off the LCD
(2026-09-18, the diode's on 2026-09-19); the 7-segment O is the digit 0:

| Nibbles 1-9 | Mode, range | LCD |
|---|---|---|
| `A A 0 C A 6 4 0 1` | Ω, range 6 (40 MΩ) | `.OL MΩ` |
| `A A 0 C A 0 B 0 0` | Diode | `. OL` |
| `A A 0 C A 0 A 0 0` | Continuity | `0.L Ω` |
| `A C 0 A A 0 F 0 4` | 4-20 mA %, sign bit set | `- LO. %` |

The diode's point sits where it does in a reading, `0.6314`. The vendor
app forces the text to `0L` or `L0` instead (§7.4 item 6).

`LcdDisplay71A` checks nibble 10 = `D` and nibble 11 = `A` before
parsing. Its callers guarantee both: the USB handler checks them, and
the RS232 handler appends a literal `DA` (§2.2, §2.3). So the check
always passes. `LcdDisplay70B` has no such check; the UT803
handlers make it.

### 3.4 Mode Codes (Nibble 7) — [VENDOR]

#### UT804 — 15 Modes

Codes from the vendor parser (§7.4 item 7); dial positions from the UT804
manual's Table 2-1 (p.14), where SELECT reaches a position's alternates.
Every code but D and E has been sent by a real UT804 from the position
given (issue #16, 2026-09-18):

| Code | Mode | Unit | Dial position | Confirmed |
|------|------|------|---------------|-----------|
| 1 | DC V | V | V⎓ | [HARDWARE] |
| 2 | AC V; AC+DC V with coupling 3 (§3.5) | V | V~ | [HARDWARE] |
| 3 | DC mV | mV | mV⎓ / Hz Duty | [HARDWARE] |
| 4 | Resistance | Ω, kΩ, MΩ | Ω | [HARDWARE] |
| 5 | Capacitance | nF, µF, mF | capacitance | [HARDWARE] |
| 6 | Temperature | °C | °C °F | [HARDWARE] |
| 7 | Current | µA | µA | [HARDWARE] |
| 8 | Current | mA | mA % | [HARDWARE] |
| 9 | Current | A | A | [HARDWARE] |
| A (10) | Continuity | Ω | Ω, SELECT | [HARDWARE] |
| B (11) | Diode | V | Ω, SELECT | [HARDWARE] |
| C (12) | Frequency; duty cycle with the sign bit (§3.6) | Hz, kHz, MHz; % | mV⎓ / Hz Duty, SELECT | [HARDWARE] |
| D (13) | Temperature | °F | °C °F, SELECT | [VENDOR] |
| E (14) | Power (`../ut71/reverse-engineered-protocol.md` §3.2) | W | none | [VENDOR] |
| F (15) | 4-20 mA loop current as % | % (vendor string `mA%`) | mA %, SELECT | [HARDWARE] |

Code E is power [DEDUCED]: UNI-T's UT71 apps, which run this same
parser, draw the unit W with the point after digit 4, and the UT71E has a
power position (`../ut71/reverse-engineered-protocol.md` §3.2). The UT804 has no power,
ADP, logic, AC mV or tachometer position (Table 2-1), and its reporter
found none: code E is in the vendor app only. *Corrected 2026-09-21;
earlier revisions called E an unknown glyph, possibly hFE.*

**[VENDOR-DOC]** The UT804 sheet's function table agrees on codes 1-D and
F. It names code A `Fm` (likely 蜂鸣, buzzer: the meter sends it on
continuity) and code F `%(4-20mA)`, and leaves code E blank. It adds code 0,
`AC_mV`, which no UT804 dial position reaches.

#### UT803 — Modes [DEDUCED]

The UT803's mode code is its parser's position 7, with meanings of its
own (§7.4 item 4; *corrected 2026-06: earlier revisions gave it the
UT804's scheme*). Unit strings found in UT803.exe binary:
- `V`, `mV` — voltage
- `uA`, `mA` — current (µA, mA)
- `*`, `k*`, `M*` — resistance (Ω, kΩ, MΩ in custom font)
- `Hz`, `kHz`, `MHz` — frequency
- `nF`, `uF`, `mF` — capacitance
- `kRPM` — tachometer/RPM (unique to UT803)
- `#`, `?` — °C and °F by the 2026-06 font rendering (§6); earlier
  revisions read them as diode and continuity

The UT803 likely has fewer than 15 modes (no ADP/Logic mode). Exact mode
list [UNVERIFIED] without hardware.

### 3.5 AC/DC Indicator (Nibble 8) — [VENDOR]

| Value | Meaning | Display string (UT804) | Seen on a UT804 (#16) |
|-------|---------|------------------------|-----------------------|
| 0 | Default (mode-dependent) | "DC" for V, mV, µA, mA and A; blank for others | DC V, mV, µA, mA and A, and every mode without coupling |
| 1 | AC | "AC" | AC V, µA, mA and A |
| 2 | DC (explicit) | "DC" | never |
| 3 | AC+DC | "AC+DC" | V and µA |

The "AC+DC" string at value 3 was found as a literal in UT804.exe
(line 224240 in decompilation). The modes that value 0 labels "DC" come
from ut804-decompiled.txt:224195-224215. On the meter, value 3 comes from
the AC/AC+DC button, pressed in an AC mode (UT804 manual Table 2-2, p.17).

**[VENDOR-DOC]** The UT804 sheet gives the same four values and names 0
`OFF`. The meter sends 0 on its DC readings and never 2, so the "DC" that
value 0 gets on V, mV and the currents is the app's default.

### 3.6 Status Flags (Nibble 9) — [VENDOR]

Nibble 9 is decomposed as individual bits in the UT804 parser
(FUN_00558a7c, lines 224244-224283):

| Bit | Mask | Flag | Confirmed |
|-----|------|------|-----------|
| bit 3 | 0x8 | Unknown (stripped first, no visible effect). The UT804 sheet makes it the sign; the meter never sets it (below) | [UNVERIFIED] |
| bit 2 | 0x4 | **Negative sign** (duty-% selector in frequency mode). Corrected 2026-06 — previously misread as HOLD; the "'-' indicator" it lights is the sign (`LcdFH`), and the bit's value is prepended to the parsed number (see §7.4). Set on zero readings too, which the LCD shows with a minus | [HARDWARE] |
| bit 1 | 0x2 | Manual range [VENDOR-DOC]: set by RANGE, and by REL (below) | [HARDWARE] + [VENDOR-DOC] |
| bit 0 | 0x1 | AUTO | [HARDWARE] + [VENDOR-DOC] — shows "AUTO" text |

On a UT804 (issue #16, 2026-09-18), AUTO is set on V, Ω, capacitance,
frequency, µA and mA, and clear on mV, A, diode, continuity, temperature
and the 4-20 mA %, the positions with a single range. Bits 1 and 3 were
never set that day. On 2026-09-19, on DC V's 40 V range, the same meter
went from status 1 to 2 when RANGE was pressed, and stayed at 2 through
MAX MIN, whose reading kept following the input as the manual says the
primary display does (p.24). REL sent the LCD's primary display, the
present value less the stored one (`-00.001` with both at 7.19 V), with
status 6: sign and Manual. Neither MAX MIN nor REL has a bit of its own,
and neither's secondary displays are sent. EXIT then SEND brought status 1
back.

The bit decomposition logic:
```
value = parseInt(nibble9)  // 0-15
if value >= 8: value -= 8  // strip bit 3
if value >= 4:
    value -= 4             // strip bit 2 → sign
if value == 1:             // bit 0 → AUTO active
```

**[VENDOR-DOC]** The UT804 sheet reads bits 0-2 as one field, `000` OFF,
`001` AUTO, `010` Manual, and bit 3 as the sign, `0` plus and `1` minus.
The field fits the meter: the single-range positions send OFF, the others
AUTO, and RANGE sets bit 1. The sign does not:
every negative reading and signed zero from the #16 meter had bit 2 set and
bit 3 clear (status `5`, or `4` without AUTO), which is also where the
vendor app reads it (§7.4 item 2). The meter sends the sign in bit 2.
UNI-T's UT71 sheet, made from the same template, puts the sign in bit 2
(`x1xx`), where the meter sends it
(`../ut71/reverse-engineered-protocol.md` §3.4).

Where low battery shows, if at all, is [UNVERIFIED]; the packet has no
nibbles 12-14 (§2.1). HOLD sends nothing (§4.2).

### 3.7 Range Code (Nibble 6) — [VENDOR]

The range code selects the sub-range within each mode and so the decimal
point: the UT804.exe mode switch (§7.4 item 7) puts the point after digit
position `pos+1`, where the full range less one count reads, and the UT804
manual's Table 2-3 (p.18-19) gives the same full ranges. The last column is
the codes a UT804 sent (issue #16, 2026-09-18), autoranging included.

| Mode | Range code: full range | Seen |
|------|------------------------|------|
| DC V (1) | 1: 4 V, 2: 40 V, 3: 400 V, 4: 1000 V | 1-2 |
| AC V, AC+DC V (2) | 1: 4 V, 2: 40 V, 3: 400 V, 4: 1000 V | 1-4 (AC), 1-3 (AC+DC) |
| DC mV (3) | 400 mV, any code | 0 |
| Resistance (4) | 1: 400 Ω, 2: 4 kΩ, 3: 40 kΩ, 4: 400 kΩ, 5: 4 MΩ, 6: 40 MΩ | 1-6 |
| Capacitance (5) | 1: 40 nF, 2: 400 nF, 3: 4 µF, 4: 40 µF, 5: 400 µF, 6: 4 mF, 7: 40 mF | 1-2 |
| µA (7) | 0: 400 µA, 1: 4000 µA | 0-1 |
| mA (8) | 0: 40 mA, 1: 400 mA | 0-1 |
| A (9) | 10 A, any code | 1 |
| Continuity (A) | 400 Ω | 0 |
| Diode (B) | 4 V | 0 |
| Frequency (C) | 0: 40 Hz, 1: 400 Hz, 2: 4 kHz, 3: 40 kHz, 4: 400 kHz, 5: 4 MHz, 6: 40 MHz, 7: 400 MHz | 0 |
| Temperature (6, D) | 1000 °C, 1832 °F; the point sits after digit 4 | 0 (°C) |
| Duty cycle (C, sign bit), 4-20 mA % (F) | no range given; the point sits after digit 3 | 0 |

For example, 40 V reads `39.999` and 400 µA `399.99`. Table 2-3 gives AC
V's top range as 750 V, where the AC voltage spec table (p.59), the basic
specifications (p.57), its remark b) (p.60) and the datasheet give 1000 V,
as above. It gives AC mA as 400 and 4000, the µA row's figures. Table 2-1
gives the mA position 40 mA and 400 mA, and the meter shows `00.000 mA` on
AC mA range 0, so AC mA has DC mA's ranges.

UNI-T's UCI SDK manual has a UT804 range table
(`../uci-bench-family/reverse-engineered-protocol.md` §6.2). It differs
from this one for resistance (codes 0-5 there) and 10 A (code 0); the
meter sent the codes above.

**[VENDOR-DOC]** The UT804 sheet's range table matches this one code for
code, with two differences:
- AC V's range 4 is 750 V, as in the manual's Table 2-3;
- 10 A is range 0, as in the UCI SDK manual, where the meter sent 1.

Neither moves a decimal point: a 750 V range keeps the point after the
fourth digit, as 1000 V has it (`1000.0`), and the A position has one range. The sheet gives no full scale for continuity,
diode or the 4-20 mA %, and 400 mV for code 0 (AC mV).

---

## 4. Transport Initialization — [VENDOR]

### 4.1 CH9325 Configuration

One feature report per `USB Connect` press or release (§1.2): 2400 baud
from UT804.exe, 19200 baud from UT803.exe, both as
`xx, rate LE, 00, 00, 03, 00, 00, 00, 00`.

### 4.2 Data Streaming

Once its data output is on, the meter streams measurement packets
continuously. The UT803/UT804 manuals give 2-3 display updates per
second. Observed on a UT804 (issue #16, 2026-09-16): a packet about every
656 ms. A 2026-09-17 run on the same meter measured 16 consecutive packets
652-684 ms apart, with one 1236 ms gap while the dial was being turned.

On the UT804 the SEND button turns the output on and the LCD shows SEND
(manual Table 2-2, p.16) [HARDWARE]: SEND is off at power-on, and nothing
arrives until it is pressed. With HOLD on, the meter sends nothing, SEND
still lit, and EXIT, which leaves HOLD, turns SEND off as well (issue #16,
2026-09-18).

The feature report is the only thing either app sends: its
`HidD_SetFeature` call is the only HID output in either binary, and no
form code writes to the HID device or the serial port. `WriteFile`'s
only callers are the runtime's file and console output; the apps'
serial component never calls it. No trigger byte and no command reach the meter. The UT803
manual has the user press the meter's RS232 button to start data output
[KNOWN]. The UT804 sheet describes only what the meter sends, and lists no
command [VENDOR-DOC].

---

## 5. Differences Between UT803 and UT804

| Feature | UT803 | UT804 |
|---------|-------|-------|
| Display count | 6000 (3¾ digit, max 5999) | 40000 (4¾ digit), 4000 when RANGE is held at power-on [KNOWN] (UT804 manual) |
| Mode count | V, mV, µA, mA and A (DC and AC), Ω, continuity, diode, capacitance, Hz, °C, °F, hFE, tachometer, ADP [KNOWN] (UT803 manual) | 15 codes, 14 on the dial (§3.4) |
| RPM mode | Yes (`kRPM` unit string) | No (manual Table 2-1) [HARDWARE] |
| ADP/Logic mode | Not seen | No dial position (§3.4) [HARDWARE]; code 14, in the app only, is power |
| Temperature | Yes, °C and °F [KNOWN] (UT803 manual p.48) | Yes (modes 6 and D) |
| AC+DC mode | On the meter, not on the wire: "+DC, hFE and β cannot output to the computer" [KNOWN] (UT803 manual p.37) | Yes (nibble 8 = 3) [HARDWARE] |
| Data output on | RS232 button (UT803 manual p.36) | SEND button (§4.2) [HARDWARE] |
| CH9325 and RS232 rate | 19200 (§1.2) | 2400 (§1.2) |
| Parser position k | byte k-1 (§2.1) | byte k (§2.1) |
| 7-segment decoder in the app | None | Unused UT60A/B/C path (§2.4) |

Both send 11-byte packets ending CR LF (§2.1); the payload layouts
differ (§7.4 item 4). UNI-T's interface protocol sheet covers the UT804
alone: its rate and layout are the UT804's.

---

## 6. Custom Font Mapping — [VENDOR]

Both apps use custom TrueType fonts (unit.ttf, unit_a2.ttf, unit_a3.ttf,
unit_372.ttf) where ASCII characters map to measurement symbols:

| ASCII | Visual symbol |
|-------|---------------|
| `*` | Ω (Ohm) |
| `#` | °C |
| `?` | °F |
| `)` | Diode symbol |
| `&` | Beeper symbol |
| `@` | `L` (overload text, §7.4 item 6) |
| `$` | Battery symbol |
| `W` | W, the unit of code E, power (§3.4) |

*Corrected 2026-06 from the rendered fonts (§7.4 item 7). Earlier
revisions, from string extraction and the mode detection logic alone,
gave `#` as diode, `?` as continuity, `@` as the AC indicator and `&` and
`$` as unknown. The `W` row is theirs; §7.4 item 7 does not cover it.*

*Corrected 2026-09-21: the `W` row read "Unknown (ADP mode unit)"; the
UT71 apps, which share this parser, draw it as the power unit
(`../ut71/reverse-engineered-protocol.md` §3.2).*

---

## 7. Implementation Notes

### 7.1 Packet Extraction

Packets are 11 bytes ending in CR LF, with the data in the low nibbles
(§2.1). Bit 7 is parity (§1.2): the UT804's LF arrives as `8A` (§2.1).
The apps rely on empty reports between packets instead (§2.2); CR LF
ends a packet whatever the report timing. There is no checksum.

### 7.2 Data Parsing

The data nibbles are proprietary, NOT LCD segments (UT804 layout, §3.1):
1. Nibbles 10 and 11 are the format markers 0x0D and 0x0A
2. Nibble 7 is the mode code
3. Nibble 6 is the range code
4. Nibble 8 is AC/DC
5. Nibble 9 holds the status flags
6. Nibbles 1-5 are the digits; nibble 1 = 0x0A marks an overload or LO
   packet (§3.3)
7. The decimal point is not sent: the range code gives its position (§3.7)

### 7.3 What Needs Hardware Verification

The UT804's sign, mode codes, coupling, AUTO and Manual bits, REL and
the ranges it sent are confirmed (§3). Still open:

- The UT803's sign, mode list and range tables
- Status flag bits: bit 3 (the sheet's sign, never sent), Low Battery
  (§3.6)
- Whether the meter needs anything sent (the apps send nothing, §4.2)
- Streaming rate: a packet about every 656 ms on a UT804 (§4.2); the
  UT803's is open
- Line format on the wire (§1.2): 7O1 on a UT804; the UT803's is open
- Digit nibbles `B`, `D`-`F` (§3.2)
- Whether nibble 4 = 'B' guard condition has meaning
- The UT804's °F packets (code D), and whether code E (power, no UT804
  position) or 0 (AC mV, the sheet only) is ever sent

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
   the UT803 is nibble 9 bit 3 (`LCDHold`, ut803-decompiled.txt:225086);
   HOLD's wire encoding on the UT804 appears in **neither** parser and
   remains unknown.
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
   mode-code meanings — see §5), with its mode and range tables at
   ut803-decompiled.txt:224441-225068.
5. **Decimal positions count from the left** (point after digit
   `pos+1`), per the display assembly at ut804-decompiled.txt:
   224289-224356 (point slots LcdP0-LcdP3 interleaved with the digit
   labels). The previous places-from-right reading inverted every
   table.
6. **Nibble 1 = 0xA marks an overload frame** (digits forced to
   "0L"/"L0" via the LCD font where '@' renders as 'L';
   ut804-decompiled.txt:223810-223823, 224361-224391); `"0@"` → +OL,
   `"-0@"` → −OL, `"@0"` → 0.0. It is not an "AC flag mode".
   *2026-09-16: nibble 2 = C gives `"@0"` (0.0, shown "L0."), any other
   nibble 2 gives `"0@"` (overload, negative with the sign bit);
   nibbles 3-5 are ignored (VA 0x558B3B-0x558B98, 0x559ABE-0x559B2A).*
7. **The UT804 mode table was wrong for 9 of 15 codes.** Corrected via
   unit-string constants + font glyphs (`#`=°C, `?`=°F, `)`=diode,
   `&`=beeper, `*`=Ω, `@`='L', `$`=battery): 6=Temp °C, 7=µA, 8=mA,
   9=A, A=Continuity, B=Diode, C=Frequency (duty-% via nibble 9 bit 2,
   reused since negative frequency is impossible;
   ut804-decompiled.txt:224271-224283), D=Temp °F,
   E=unknown glyph (possibly hFE), F="mA%" (likely 4-20 mA loop).
   *2026-09-21: E is power, unit W (§3.4).*
   Frequency unit boundaries: ranges 0-1 Hz, 2-4 kHz, 5-7 MHz; Ω:
   range 1 Ω, 2-4 kΩ, 5-6 MΩ. The unit strings are appended at
   ut804-decompiled.txt:224075-224184, and the range switches are at
   223961-224033 and 224129-224170.

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

Clean-room note: approval was given to consult the sigrok FS9721 decoder
and the FS9721-LP3 datasheet for this family, but the 2026-06 resolution
above required neither — it is derived entirely from the vendor
binaries, their fonts, and the existing decompiles. The clean-room
boundary was opened later, on 2026-09-16, with approval: §8 records what
was consulted.

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

At that date the spec left nibbles 12-14 as `[UNVERIFIED]` and the sign
unlocated. Both are superseded: the sign is nibble 9 bit 2 (§3.2,
§7.4 item 2) and the packet has no nibbles 12-14 (§2.1).

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
| Function codes | 1-F (§3.4); E is power [DEDUCED], F 4-20 mA % | 0-15, 14 = power, 15 = loop current | 1-9, `:` continuity, `;` diode, `<` Hz, `=` °F, `?` 4-20 mA %; no power on the UT804 | ✓; 1 = V DC, 2 = V AC new |
| Range tables | §3.7 | — | Per function | ✓ (log) |
| Coupling (nibble 8) | 0 = per mode, 1 AC, 2 DC, 3 AC+DC (§3.5) | Bit 0 AC, bit 1 DC | Same | ✓ |
| Status (nibble 9) | Bit 0 AUTO, bit 1 manual range [VENDOR-DOC], bit 2 sign, bit 3 unknown (§3.6) | Bit 0 AUTO, bit 1 MAN, bit 2 sign | Same | ✓; bit 1 set by RANGE (§3.6) |
| Duty cycle | Hz mode with the sign bit (§7.4) | Same | Same | ✓ |
| Digit values A, C, F | A = blank or flag, B-F unknown (§3.2) | — | `:` blank, `<` 'L', `?` 'H' | ✓ A; C, F new |
| Overload | Nibble 1 = A: overload unless nibble 2 = C, which gives 0.0 shown "L0." (§7.4) | `::0<:` overload, `:<0::` underload | `::0<:` overload; 4-20 mA `:<0::` "L0", `:?1::` "HI" | ✓³ |
| Nibbles 12-14 | No such bytes (§2.1) | No such bytes | No such bytes | ✓ |
| HOLD | Wire encoding unknown (§7.4) | — | Nothing transmitted while HOLD is on | New |
| REL | No bit of its own: the relative reading, with bit 1 (§3.6) | — | Never transmitted | ✓ |
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
- UT803 operating manual — RS232 settings and RS232 button (p.36)
- UT804 operating manual — display counts; Table 2-1 rotary switch (p.14),
  Table 2-2 buttons (p.15-17), Table 2-3 ranges (p.18-19), MAX MIN (p.24)
- Issue #16 — a UT804's CH9325 reports under dmm-tools 0.6.0 and 0.7.0-dev
  (2026-09-16), two capture reports under 0.7.0-dev (3806742) with the
  LCD read back beside each step (2026-09-18), and RANGE, MAX MIN and REL
  captures under 0.7.0-dev (720072d, 959adfc) with LCD photos of diode mode
  and REL (2026-09-19)
- CH9325 transport analysis — see `../uci-bench-family/reverse-engineered-protocol.md`
- UNI-T "UT804接口协议" (listed as V1.0, uploaded 2023-11-15; the file dates
  from 2005, last saved 2019) — line format, frame layout, function and
  range tables, coupling and status fields
