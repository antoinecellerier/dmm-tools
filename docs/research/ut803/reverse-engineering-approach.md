# UT803 / UT804: Reverse Engineering Approach

## Sources Used

### Primary (clean-room RE)
1. **UT803.exe V1.01** — standalone PC software, Borland Delphi application.
   Ghidra decompilation (headless, x86:LE:32:default:borlanddelphi).
   Binary constant extraction via Python PE parser.

2. **UT804.exe V2.00** — standalone PC software, Borland Delphi application.
   Ghidra decompilation + binary constant extraction.

3. **CH9325 HID transport** — reverse engineered separately from uci.dll
   (documented in `../uci-bench-family/`).

### Avoided (clean-room boundary)
- No external open-source implementations were consulted during RE
- sigrok FS9721 driver was NOT referenced (to avoid contamination, since
  the protocol turned out to be non-standard FS9721)

## Key Findings

### The protocol is NOT standard FS9721

Initial assumption: UT803/UT804 use the FS9721 14-byte LCD segment protocol.
This was based on:
- The `CMP EBX, 14` frame assembly loops in the binary
- The `"123456789ABCDE"` validation string (standard FS9721 byte indices)
- The presence of 7-segment decode tables in both binaries

**Actual finding:** The meters use FS9721 **framing** (14 bytes with index
nibbles) but with a **proprietary data encoding**. The data nibbles carry
structured measurement data (mode codes, range codes, digit values, status
flags) rather than raw LCD segment bits.

Evidence:
1. Nibble 7 comparison constants in UT804.exe: `'D'`, `'A'`, `'B'`, `'C'`,
   `'E'`, `'F'` at VA 0x0055a2a0-0x0055a2f4 — these are hex digit characters
   used as mode codes (10-15), not segment data
2. Nibble 6 parsed as integer range code (FUN_00409258 = StrToInt)
3. Nibble 8 compared against 0-3 for AC/DC/AC+DC selection, with "AC+DC"
   literal string at line 224240
4. Nibbles 10-11 always contain 0x0D/0x0A as format markers
5. Digit nibbles (1-5) contain BCD values, not 7-segment bit patterns

### 7-segment decode: secondary code path

Both binaries contain 7-segment decode functions (FUN_0055a480 in UT804,
equivalent in UT803) with a verified decode table. However, the main USB
HID data path does NOT use these functions — the data is already structured.
These functions may be for:
- RS-232 serial output (standard FS9721 segment mode)
- Legacy firmware compatibility
- An alternative display mode

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

## Confidence Assessment

- **Frame format (14-byte, index nibbles):** HIGH — confirmed by assembly
  analysis, validation string, and functional code
- **Proprietary data nibbles:** HIGH — confirmed by binary constants and
  mode detection logic in both executables
- **Mode codes 1-15:** HIGH for UT804, MEDIUM for UT803 (fewer modes, exact
  list not fully enumerated)
- **Range/decimal point tables:** MEDIUM — logic identified but not all
  range values could be decoded from decompilation alone
- **Status flag bits:** MEDIUM — HOLD and AUTO confirmed, others unverified
- **Digit encoding:** MEDIUM — 0-9 confirmed as digits, 0xA as blank, sign
  encoding unknown
- **Nibbles 12-14:** LOW — purpose not determined
