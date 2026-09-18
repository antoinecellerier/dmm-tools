# UT803 / UT804: Reverse Engineering Approach

## Sources Used

### Primary (clean-room RE)
1. **UT803.exe V1.01** — standalone PC software, Borland Delphi application.
   Ghidra decompilation (headless, x86:LE:32:default:borlanddelphi).
   Binary constant extraction via Python PE parser.

2. **UT804.exe V2.00** — standalone PC software, Borland Delphi application.
   Ghidra decompilation + binary constant extraction.

   For both apps (2026-09-16): the Delphi RTTI (class, method, field and
   dynamic-method tables), the form resource, and a Ghidra 12.1.3
   decompilation of the form's event handlers.

3. **CH9325 HID transport** — reverse engineered separately from uci.dll
   (documented in `../uci-bench-family/`).

4. **UT803 operating manual** — RS232 port settings and data output
   button; the spec data, from "Accuracy Specifications" (REV.3, PDF
   pp. 39-48), cross-checked against the UT803 datasheet (identical to the
   one the UT800 series page links).

5. **UT804 operating manual** — display counts; the spec data, from
   "Detailed Accuracy Specifications" (P/N:110401108661X Jul.2019 REV.1,
   PDF pp. 59-67), cross-checked against its basic specifications (PDF
   p. 58), the UT804 datasheet and the UT800 series page.

### Avoided (clean-room boundary)
- No external open-source implementations were consulted during RE
- sigrok FS9721 driver was NOT referenced (to avoid contamination, since
  the protocol turned out to be non-standard FS9721)
- Opened 2026-09-16, with approval, after the analysis below — see
  [Cross-Reference with Community Sources](#cross-reference-with-community-sources)

## Key Findings

### The protocol is NOT standard FS9721

Initial assumption: UT803/UT804 use the FS9721 14-byte LCD segment protocol.
This was based on:
- The `CMP EBX, 14` frame assembly loops in the binary
- The `"123456789ABCDE"` validation string (standard FS9721 byte indices)
- The presence of 7-segment decode tables in both binaries

**Actual finding:** the data nibbles carry structured measurement data
(mode codes, range codes, digit values, status flags) rather than raw LCD
segment bits. *Corrected 2026-09-16: the meters send 11-byte packets
ending CR LF, not 14-byte FS9721 frames. The 14-byte loops belong to
each app's unreachable `Read` handler; the validation string and the
7-segment table exist only in UT804.exe, as part of its unused
UT60A/B/C path — `reverse-engineered-protocol.md` §2.*

Evidence:
1. Nibble 7 comparison constants in UT804.exe: `'D'`, `'A'`, `'B'`, `'C'`,
   `'E'`, `'F'` at VA 0x0055a2a0-0x0055a2f4 — these are hex digit characters
   used as mode codes (10-15), not segment data
2. Nibble 6 parsed as integer range code (FUN_00409258 = StrToInt)
3. Nibble 8 compared against 0-3 for AC/DC/AC+DC selection, with "AC+DC"
   literal string at line 224240
4. Nibbles 10-11 always contain 0x0D/0x0A as format markers
5. Digit nibbles (1-5) contain BCD values, not 7-segment bit patterns

### 7-segment decode: unused code path

UT804.exe's 7-segment decoder (FUN_0055a480, `LcdDisplay60B`) is reached
only from the `Read` button's HID handler and from the RS232 handler
`H60BRData`. The shipped form selects neither: the checkbox that would
install `H60BRData` is unchecked, and the `Read` button lies under the
chart. UT803.exe has no 7-segment decoder
(`reverse-engineered-protocol.md` §2.4).

## Methodology

1. **Ghidra headless decompilation** — full auto-analysis of both Delphi
   executables. Key functions identified by string cross-references
   (error messages like "USB interface cable is not securely connected").

2. **Data flow tracing** — followed DAT_0056b698 (UT804) and DAT_005676a0
   (UT803) from HID receive callback through mode detection and display
   update functions.

3. **Binary constant extraction** — Python PE parser to read string constants
   at addresses referenced in the decompiled code. This resolved the actual
   comparison values (e.g., confirming 'D' at VA 0x0055a2a0) that Ghidra's
   decompiler couldn't show.

4. **Cross-referencing** — verified that both UT803 and UT804 use identical
   data format by comparing function structures, constant patterns, and
   mode detection logic.

5. **RTTI-seeded handler decompile (2026-09-16).** The full auto-analysis
   had missed most of the form's event handlers, because only the RTTI
   refers to them: the 12.0.4 decompile held 19 of UT804.exe's 78
   published methods and 11 of UT803.exe's 64, and none of the HID
   callbacks or connect handlers. A scan of the VMTs listed:
   - the form's published methods and fields, with each field's class;
   - the dynamic-method tables;
   - the unnamed data handlers the form installs.

   A Ghidra 12.1.3 script created and decompiled those functions. The
   form resource gave each control's class, visibility, check state
   and event bindings. Conditions were read from the disassembly,
   because the decompiler drops the flag branches after Delphi string
   compares. A search from every HID and `WriteFile` call site back to
   the form code, following direct calls, found what the apps send. The `*-gap-decompiled.txt`
   files from the first pass held only Ghidra logs.

## Confidence Assessment

- **Packet format (11 bytes ending CR LF, low nibbles):** HIGH — both
  apps' USB and RS232 handlers (2026-09-16). The earlier 14-byte reading
  came from an unused path.
- **Proprietary data nibbles:** HIGH — confirmed by binary constants and
  mode detection logic in both executables
- **Mode codes 1-15:** HIGH for UT804, MEDIUM for UT803 (fewer modes, exact
  list not fully enumerated)
- **Range/decimal point tables:** MEDIUM — logic identified but not all
  range values could be decoded from decompilation alone
- **Status flag bits:** MEDIUM — AUTO and sign confirmed, others unverified
- **Digit encoding:** MEDIUM — 0-9 confirmed as digits, 0xA as blank, sign
  encoding unknown
- **Nibbles 12-14:** none; the packet is 11 bytes

## Cross-Reference with Community Sources

Consulted after the vendor analysis above, for validation only (approved
2026-09-16, after issue #16's first UT804 report). The finding-by-finding
comparison is in `reverse-engineered-protocol.md` §8. In short: sigrok
and `UT804.LOG` give the UT804 11-byte UT71x packets, which our handler decompile now finds on both the
vendor's USB and RS232 paths; the payload matches §3 and §7.4 item 6,
overloads included; and every community CH9325 driver sends the UT803/UT804
apps' report layout.

Reference implementations:

- [sigrok libsigrok](https://github.com/sigrokproject/libsigrok) — C; UT71x parser, UT804 entries, CH9325 set-up
- [sigrok wiki, WCH CH9325](https://sigrok.org/wiki/WCH_CH9325) — CH9325 configuration bytes and report framing
- [tmatejuk/ut804_linux_logger](https://github.com/tmatejuk/ut804_linux_logger) — C, UT804 RS232 logger; `UT804.LOG` lists real UT804 packets
- [Lukas Schwarz, UT61B analysis](https://lukasschwarz.de/ut61b) — HE2325U/CH9325 set-up and report format
- [thomasf/uni-trend-ut61d](https://github.com/thomasf/uni-trend-ut61d) — C++, HE2325U/CH9325 reader
