# UT71A–E / Voltcraft VC920/VC940/VC960: Reverse Engineering Approach

## Sources Used

### Primary (clean-room RE)

All fetched and read 2026-09-21 unless stated. Provenance files beside each
source carry URLs, sizes and MD5s.

1. **UNI-T's UT71 interface protocol sheet, "UT71通信协议.xls"** — from the
   "UT71系列接口协议" RAR on UNI-T's Chinese handheld download centre
   (https://meters.uni-trend.com.cn/menu/68.html), which also holds
   "VC920_940_960 Protocol.xls" with identical cells. Author UNIT-hwj,
   created 2005-07-04, last saved 2014-03-06. Rendered with LibreOffice
   and read cell by cell: line settings, the 11-byte layout, the function
   table (0-15) and a range table per function. Tagged [VENDOR-DOC].
   Stored as `references/ut71/ut71 系列协议/`, provenance
   `references/ut71/SOURCE.txt`.

2. **Conrad's VC920/VC940/VC960 protocol sheets** — three PDFs of an
   Excel sheet from the same template as UNI-T's UT804 sheet (high nibble
   0011, sign in bit 3): 123296DS01 (2005, 16 functions), 123297DS01
   (2006, 16 functions, "Duty Cycle" at 13) and 123298DS02 (2010, VC960,
   "7-bit character", `12:L`). Fetched from asset.conrad.com; the shop
   pages return 403. Tagged [VENDOR-DOC] with the sheet named.
   `references/vc920/other/`.

3. **UNI-T's UT804 sheet "UT804接口协议"** — read 2026-09-19 for the UT804
   work; used here through `../ut803/reverse-engineered-protocol.md` for
   what the two templates share and where they differ.

4. **UT71 manuals and product literature** — the UT71A/B/C/D/E operating
   manual in English (REV.3, 2022.8.18) and Chinese (2009), the UT71
   datasheet, the CN product sheets for A/B and C/D/E, and the UT71A/B and
   UT71C/D/E Interface Software Manuals (2011), all from UNI-T's global
   and Chinese sites. Dial tables, buttons, SEND, RECALL, counts, ranges.
   Tagged [KNOWN]. `references/ut71/manual/`, notes in
   `references/ut71/manual-notes.md`.

5. **Voltcraft manuals** — the VC920/VC940/VC960 operating instructions
   (Version 11/10 with note 01/13, DE/EN/FR/NL; the 07/05 and 12/05
   German editions; the 2006 English "Model VC940" manual), the USB
   adapter 120317 operating instructions (03/09) and the VC9x0 computer
   interface software manual (2005), from Conrad's file server. Tagged
   [KNOWN]. `references/vc920/manuals/`, `references/vc920/other/`.

6. **UNI-T's UT71 interface software CD** — "110401700522 UT71系列接口软件光盘",
   RAR from the Chinese download centre (server date 2026-05-15): two
   InstallShield installers, `UT71A_B_V3.00.exe` and `UT71C_D_E_V3.00.exe`,
   and four PDF guides. The installed apps `UT71A_B.exe` (V3.00, MD5
   1cc346ad…) and `UT71C_D_E.exe` (V3.00, MD5 85b8888f…) are Borland
   Delphi, the same project as UT804.exe V2.00. Ghidra 12.1.3
   decompilation, form resources, RTTI, annotated parser disassembly.
   Tagged [VENDOR]. `references/ut71/software/`, notes in
   `references/ut71/software-notes.md`.

7. **Our UT804 clean-room work** — `../ut803/reverse-engineered-protocol.md`
   §1-7 and `references/ut800/ut804/` (UT804.exe V2.00, its annotated
   `LcdDisplay71A`), for the instruction-level comparison.

### Avoided (clean-room boundary)

- **No community source was read for this analysis:** not sigrok's
  `ut71x` decoder, no forum, no third-party project. Every finding in
  `reverse-engineered-protocol.md` §1-6 comes from the vendor sheets, the
  manuals, UNI-T's own apps and our UT804 clean-room work.
- **A prior opening is on record:** sigrok libsigrok, its `ut71x` parser
  included, was opened 2026-09-16, with approval, for the UT804 work's
  cross-reference (`../ut803/reverse-engineering-approach.md`, "Avoided"
  and "Cross-Reference"). This analysis therefore leads with the vendor
  sheet, the manuals and the apps, and records its own cross-reference
  only afterwards, in the section at the end, so that what came from
  where stays auditable.
- **Opened 2026-09-21, with approval,** after §1-6 were complete:
  sigrok's `ut71x` parser and device entries, recorded in
  [Cross-Reference with Community Sources](#cross-reference-with-community-sources).
- The Conrad sheets and software are vendor material (Conrad/Voltcraft),
  inside the boundary.

## Key Findings

### The UT71 is the UT804's protocol under another name

UNI-T's UT71A/B and UT71C/D/E apps are the UT804.exe code base: same
Delphi project (`UT70BF` units, `TFCOMM` form, the same 40 LCD labels),
and their RS232 handler `H71ARData`, USB handler and parser
`LcdDisplay71A` normalise to the same instruction streams as UT804.exe's.
So the framing (11 bytes ending CR LF, low nibbles only), the field
positions, the function codes, the range-to-point tables, the coupling
and status logic and the duty override are the UT804's (spec §2, §3).

### What the UT71 sources add

- **The sign is in bit 2 on the sheet too.** UNI-T's UT71 sheet gives
  byte 8 as `x0xx` +, `x1xx` −, `xx01` AUTO, `xx10` Manual, where the UT804
  sheet (and Conrad's copies) put the sign in bit 3 and the mode in bits
  2:0. The apps and the UT804 read bit 2 (spec §3.4).
- **Code E is power.** The sheet leaves it blank; the app draws W with the
  point after digit 4; only the UT71E and VC940 have a W position (spec
  §3.2). Marked [DEDUCED] until a UT71E packet is seen.
- **Two count families share the codes.** The UT71A/B count to 20000 and
  the UT71C/D/E to 40000; the A/B app keeps the C/D/E point tables and
  halves the chart's full scale per range, matching the manual's Table B.
  The packet does not say which family sent it (spec §3.5, §4.1).
- **AC V range 4 disagrees across documents.** Sheet 750 V; every UT71
  document 1000 V; the Voltcraft manuals 750 V (spec §3.5).
- **Line and high nibble are open.** The UT71 sheet says 8N1 and leaves
  the high nibble `xxxx`; the UT804 sheet and the Conrad sheets say odd
  parity and 0011; the apps test neither (spec §1.2, §2).
- **4000-count displays blank digit 5.** Conrad's sheets state it; the
  manuals list the blue key at power-on, RANGE at power-on for Ω, and
  the VC9x0's fixed 4000-count Ω (spec §3.1).
- **Voltcraft identity.** UNI-T files the same sheet under the Voltcraft
  name; the Voltcraft manuals name no UNI-T model. VC920 ≈ UT71C,
  VC960 ≈ UT71D, VC940 ≈ UT71E from the dial layouts, counts and power
  function [DEDUCED] (spec §4.2).

## Methodology

1. **Sheet rendering.** Both UNI-T sheets (and the UT804 sheet for
   comparison) converted with `soffice --headless --convert-to html` and
   read cell by cell; the two UT71-download sheets confirmed identical by
   the MD5 of their HTML renderings.

2. **Manual reading.** The UT71 EN/CN manuals and the Voltcraft manuals
   read on the rendered pages, with each finding tied to a printed page
   (`manual-notes.md` gives the PDF-to-printed offsets). Where two
   documents disagree (AC V 750 V vs 1000 V, memory sizes, the UT71A's
   Hz position), both are recorded.

3. **Unpacking the CD.** The RAR (v4; 7-Zip could not, `unrar` can) holds
   two InstallShield 16 wrappers with encrypted embedded MSIs. Each was
   run under wine 10.0 in a throw-away `WINEPREFIX` inside a private Xvfb
   with the InstallShield cache option (`/b"<cache>" /s /v"/qn"`), and the
   MSIs unpacked with `msiextract`. Nothing else was run; the prefix
   never touched the user's display or `~/.wine`.

4. **Ghidra with Delphi RTTI seeding.** Headless Ghidra 12.1.3
   (`x86:LE:32:default`, `borlanddelphi`) decompiled every function; the
   `TFCOMM` form's published methods and the two unnamed HID data handlers
   were created and decompiled from the RTTI method tables
   (`references/ghidra-scripts/DelphiDecompileHandlers.java`), the form
   resource decoded (`delphi_rtti.py`), and conditions read from the
   disassembly, as for the UT804 (its approach doc, Methodology 5).

5. **Instruction-level comparison with UT804.exe.** The parser and the
   handlers were disassembled with `objdump`, string constants, form
   fields and RTL calls resolved (`norm.py`), and the normalised listings
   diffed against UT804.exe's `LcdDisplay71A` (0x558A7C). The diff lists
   seven differences, none in the wire decoding (`software-notes.md` §7).

6. **Reconciliation.** Each sheet field was checked against what the app
   does with it and what the manuals say the meter has; disagreements
   are stated with their sources, not resolved by choice.

## Confidence Assessment

- **Packet format (11 bytes ending CR LF, low nibbles, no checksum):**
  HIGH — the UT71 sheet, the Conrad sheets, both apps' receive paths, and
  the same layout seen on a UT804.
- **Field positions, function codes 1-D and F, range-to-point tables:**
  HIGH — sheet, apps and manuals agree; the same codes, D excepted, and
  points came from a real UT804.
- **Code E = power:** MEDIUM — app unit and the UT71E/VC940 dial; no
  packet seen.
- **UT71A/B full scales:** MEDIUM — manual and app chart scales; no
  UT71A/B packet seen, and the sheet gives only the 40000-count figures.
- **Line format, high nibble, bit-7 parity:** LOW — the sheets contradict
  each other; the apps ignore all three; only a UT804 has been measured.
- **Status byte:** MEDIUM — sign in bit 2 and AUTO in bit 0 agree across
  the UT71 sheet, the apps and the UT804; bit 3 unseen, Manual seen on
  the UT804 only.
- **Coupling on DC readings, 10 A range code, AC V range 4's full scale:**
  LOW — sources disagree or are silent; hardware decides.
- **Voltcraft correspondence:** MEDIUM — feature matching only; no
  document states it.

## Cross-Reference with Community Sources

Opened 2026-09-21, with approval, once the analysis above was complete.
Read: sigrok libsigrok's `src/dmm/ut71x.c`, its declarations in
`src/libsigrok-internal.h`, the UT71x entries (UT71A-E, UT804, Voltcraft
VC-920/940/960, Tenma 72-7730/72-7732/72-9380A) in
`src/hardware/serial-dmm/api.c` and `src/hardware/uni-t-dmm/api.c`, and
the UT71x parity masking in `src/hardware/uni-t-dmm/protocol.c`, all at
master 0bc2487778 (2025-11-20); `ut71x.c` last changed in 94b1d50642
(2018-02-18). The `ut71x` parser itself was first opened 2026-09-16, for
the UT804 (`../ut803/reverse-engineering-approach.md`).

The comparison is in `reverse-engineered-protocol.md` §7. sigrok agrees
on the framing, the function meanings (code E is power), the coupling,
the sign in bit 2 and most decimal points; it differs on the line
(7O1), the points of continuity, power and duty cycle, the 10 A range
code, and names cables and Tenma models no UT71 source does. §1-6 were
not changed; the differences go to hardware verification.
