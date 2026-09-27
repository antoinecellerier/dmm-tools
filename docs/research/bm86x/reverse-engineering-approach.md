# Brymen BU-86X Group: Reverse Engineering Approach

Scope: the Brymen meters that use the BU-86X optical USB cable — the BM860s
series (BM867s, BM869s), the BM820s series (BM821s, BM822s, BM827s,
BM829s) and the BM520s mobile-logging series (BM521s, BM525s) — from the
USB device to the LCD-segment reply and the BM520s logged-memory download.
The live readings of all three series are implemented, experimentally; the
memory download is not. This pair of documents records
what the meters and the cable do on the wire. Every fact in §1-11 of the spec, and each
question of §12, comes from vendor sources; §13 compares them with
community sources, opened 2026-09-27 after §1-12 were committed, and §12's
"Community:" notes summarise §13.

Brymen publishes the protocol, as two-page sheets per series and a
nine-page memory document, so the question was how far the sheets can be
taken at their word. The answer: **the sheets give the whole real-time
path** — request, 27-byte reply, LCD map, model byte — but no character
table, no minus labels and no timing beyond a flowchart. They contradict
themselves twice (the BM820 byte 10/11 order; the BM860 example's point,
caption and V bit) and Brymen's programs twice (the VID/PID; the BM520s
request byte).
Brymen's two programs settle what they can: they match VID:PID
0x0820:0x0001, index the BM820 reply as its Table 1 does, read the points
as the labels say, carry a 7-segment table, and send `82 66` to BM520s
meters, never `52 66`. What remains is listed in spec §12.

## Sources Used

All Brymen's own, from www.brymen.com (download pages Download.html,
Download1.html, Download2.html), fetched 2026-09-26. Provenance — URL,
date, SHA-256 — is in `references/bm86x/SOURCE.txt` (gitignored); the three
reader reports (`protocol-docs.md`, `manuals.md`, `app.md`) are in
`references/bm86x/analysis/findings/`.

### Primary (clean-room RE)

1. **BM860 sheet**, "Protocol for 500000-count professional dual display DMM
   series", 2 pages, PDF created 2009-11-30, from
   PD02BM860s_protocolDL.html; an identical copy ships in the Bs86x
   installer. Cited `BM860 p.N`.
2. **BM820 sheet**, "Protocol for 10000-count professional dual display DMM
   series", 2 pages, created 2008-12-23, from PD02BM820s_protocolDL.html;
   identical copies in the BM520s zip and the Bs82-52x installer. Cited
   `BM820 p.N`.
3. **BM520s protocol zip**, from PD02BM520s_protocolDL.html, holding the
   BM820 sheet again, **the BM520-ML sheet** "Protocol for 10000-count
   professional dual display mobile logging DMM series" (2 pages, created
   2008-12-23, cited `BM520-ML p.N`) and **the memory document**
   `Memory-Read-Allocation-Decoder.pdf` (9 pages, untitled, created
   2008-12-23, cited `MRAD p.N`).
4. **BM250/BM250s sheet**, "6000-count Digital Multimeter Communication
   Protocol", 1 page, created 2009-01-06; for the out-of-scope note (spec
   §1.3) only.
5. **BM860s user's manual** (BM867s, BM869s), 24 pages, file
   `BM860s-manual-2-033Ed2-ES636-Print1.pdf`, PDF created 2023-11-29.
   Cited `BM860s manual p.N`, N the PDF page (printed N − 1).
6. **BM820s/BM520s user's manual** (BM821s, BM822s, BM827s, BM829s,
   BM521s, BM525s), 24 pages, `BM820s-520s-Print1(033Ed2).pdf`, created
   2023-11-29; one manual for both download pages. Cited `BM820s manual
   p.N`, same rule.
7. **Bs86x V6003s**, "Ver 6.0.0.3s", JUL 26 2012, the BM860s program
   (`BM860-Bs86x-V6003s-use-BC86X-Win7.zip`), and **Bs82-52x V6009sA**,
   "Ver 6.0.0.9s Alpha", JUL 10 2012, the BM820s/BM520s program
   (`BM820-BM520-Bs8252x-V6009s-use-BC86X-Win7.zip`). Borland C++Builder
   (VCL) builds; each installer also carries PEGRAP32.DLL, a Microsoft
   hid.dll 5.1.2600.5512, BTCDL.ini, a README and the protocol sheet.
   Tagged [VENDOR], cited by decompile line or address.
8. **The READMEs** shipped in those installers, `README Bs86x V6003s.rtf`
   and `README Bs82-52x V6008s.rtf` (note V6008s against the program's
   V6009sA). Tagged [KNOWN], cited by their section numbers.
9. **Bs25x V5003** (`BM250-Bs25x-V5003.zip`), the BM250s program: imports
   and COM setup only, and **its README** `README Bs25x V5002.rtf` (shipped
   in the V5003 installer) for its cable (RS232C cable BC-20X, BUA-2303
   USB-to-serial adaptor); for spec §1.3 only, cited `README-25x §n`.
10. **General platform reference**, for statements tagged [INFERRED] with
    this basis: the Windows HID class driver's convention of one whole
    report per `ReadFile`/`WriteFile` with the report-ID byte first (`00`
    for unnumbered reports), and HID's reservation of report ID 0. No
    Brymen-specific content comes from it.

| Source | File (under `references/bm86x/`) | SHA-256 |
|---|---|---|
| BM860 sheet | `protocol/BM860-BM860s-protocol.pdf` | `a8a4779753df0ff2954597c2319f8dfc134b171cf51c836ab93b8bec70c7e305` |
| BM820 sheet | `protocol/BM820-BM820s-protocol.pdf` | `2add7a7d090b90405874b5ee8aefd42431dc6de01c3694920fc1ae3bc2fcd6de` |
| BM520s zip | `protocol/BM520-BM520s-protocol.zip` | `275d06e1c70508238df0616af637f549ae8ed558e046382eea252edd4c74ce21` |
| BM520-ML sheet | `protocol/BM520s-zip/10000count-professional-dual-display-mobile-logging-DMMs-protocol.pdf` | `306a0a6ccd0e5341d0fce5b659d4a485ef5e06540ee03d335f20258d68cc7a6d` |
| MRAD | `protocol/BM520s-zip/Memory-Read-Allocation-Decoder.pdf` | `b9e0a3c3c40ca7b2a87c15ddca0ee98de20ba82b0fee1d395991be629a2e42f0` |
| BM250 sheet | `protocol/BM250-BM250s-protocol-r1.pdf` | `7bbec079c4bcb20363e759bae6fbab059c690a74c5fac6eb539bcd1e4544211a` |
| BM860s manual | `manuals/BM860s-manual.pdf` | `487bbd6078bed5c03beb2a8c6a8156ce843bbee981b8164e5ae32ca2945d733c` |
| BM820s/BM520s manual | `manuals/BM820s-520s-manual.pdf` | `a3c44e3650ca66b826e3bcef686f913ba2ba189f6a2bc1ebd90597e1436b58dc` |
| Bs86x zip | `software/BM860-Bs86x-V6003s-use-BC86X-Win7.zip` | `1cb4024e189b8ca91e236283fd0fe11813717d9ae3f4ec0cb93f44f28b1be253` |
| Bs82-52x zip | `software/BM820-BM520-Bs8252x-V6009s-use-BC86X-Win7.zip` | `7d4c766a605f0ab0ac3d9ccb0d689963990b4a4aaef3604e9e10cac33c3c1ed1` |
| Bs25x zip | `software/BM250-Bs25x-V5003.zip` | `eba0a80dff386758c93b7fafd8b9175af0cf9bb81e5758b4e6517e3d8a09db5b` |
| Bs86x program | `app/Bs86xV6003s.exe` | `393d4655f0f2d29037798da29b0b34beb24abc49f5851a6b39b08de98eaa6b20` |
| Bs82-52x program | `app/Bs8252xV6009sA.exe` | `37e01bb089fac819aa719c2e40311efee1aa8db55d877e995275402577441ef1` |
| Bs25x program | `app/Bs25xV5003.exe` | `afb36693a8040efd2dacae2cbc4ce962528c03eba5a3452bddb583fa5fb0793e` |
| README, Bs86x | `app/README Bs86x V6003s.rtf` | `bad52cc8c2c7253c1da0342e2578788e5fdd577ba968b18b88fccd2bec756908` |
| README, Bs82-52x | `app/README Bs82-52x V6008s.rtf` | `d450df2f8e750a2cac910538fea59cd12ff44b821c1af205000c50912561ac61` |
| hid.dll (both installers) | `software/*/…/hid.dll` | `c0834c7362e010250d95d2453900f4ca7e559047b464574cd293d4a84825efa6` |
| README, Bs25x | `software/BM250-Bs25x-V5003/5003-win32-x64/README Bs25x V5002.rtf` | `6429f5982053592179edf0102de0db925a3e0b97c63e8371f37fa46cfae7b558` |

The hashes of the Bs86x and Bs82-52x READMEs and of hid.dll are in
`SOURCE.txt`; the Bs25x README's was taken for this document. Brymen's
page for a Prolific PL23XX driver installer was seen and not fetched
(`SOURCE.txt`).

### Avoided during the vendor analysis

Not searched for or opened while §1-12 of the spec were written:

- sigrok (the libsigrok brymen-bm86x driver and the wiki)
- code repositories of any kind for these meters, the cable or Brymen's
  protocol, GitHub included
- TestController and `references/testcontroller.md`
- the EEVblog forum and other forums, blog posts and videos; the web beyond
  the brymen.com pages and files above
- `docs/research/new-device-candidates.md` and
  `docs/research/non-unit-candidates.md`

**Exposure note.** Before this work, our two candidates docs already
carried community-sourced claims about this cable: its VID:PID, its trigger
bytes, its report shape and its chip. The three readers and the drafter
were told not to open them, did not, and every value in the spec was
derived again from Brymen's sources and cites them: VID:PID from the
programs' compares, the request bytes from the sheets and the programs'
writes, the report shape from the sheets' flowchart and the programs'
9-byte reads. The spec names no bridge chip: a search of the sheets' and
manuals' text, the READMEs and the programs' string tables for chip and
vendor names found none, 2026-09-27; the READMEs say only
"microprocessor embedded cable".

### Cross-referenced (clean-room boundary opened 2026-09-27)

The user opened the boundary on **2026-09-27**, after §1-12 were written,
grounding-checked and committed, for code repositories and TestController's
supported-equipment list only. The findings are §13 of the spec, marked
[COMMUNITY]; nothing was merged into §1-11 beyond pointers, and §12's
questions gained only "Community:" notes that summarise §13. Provenance is in
`references/bm86x/community/SOURCE.txt` (gitignored).

| Source | Commit | Licence | Notes |
|---|---|---|---|
| [sigrokproject/libsigrok](https://github.com/sigrokproject/libsigrok), sparse clone | `0bc2487` (2025-11-20) | GPL-3.0-or-later | the BU-86X HID transport and the brymen-bm86x, -bm52x, -bm82x parsers |
| [TheHWcave/BM869S-remote-access](https://github.com/TheHWcave/BM869S-remote-access) | `b4d6aa4` (2021-11-05) | MIT | Python, BU-86X |
| [freedaun/Brymen-BM869s](https://github.com/freedaun/Brymen-BM869s) | `ccd693d` (2021-07-09) | none stated | Python, BU-86X |
| [DawOp/Brymen869s-XmlLib](https://github.com/DawOp/Brymen869s-XmlLib) | `516592c` (2023-05-08) | MIT | C++ DLL, BU-86X |
| [kittennbfive/869log](https://github.com/kittennbfive/869log) | `f612bdf` (2024-09-19) | AGPL-3.0+ (code) | DIY infrared board and decoder |
| [MartinD-CZ/brymen-867-interface-cable](https://github.com/MartinD-CZ/brymen-867-interface-cable) | `8ea4f0b` (2021-04-28) | GPL-3.0 (LICENSE file); the `main.cpp` header says CC BY-SA 4.0 | DIY infrared board |
| TestController supported-equipment page (lygte-info.dk) | fetched 2026-09-27, SHA-256 in `SOURCE.txt` | — | model list only |

- **Licences.** The community sources are cited, never copied: no code or
  table of theirs is in this repository, and the spec quotes only short
  phrases, each with its file and line. sigrok is GPL, so its code is not
  transcribed into dmm-lib; freedaun states no licence, so its capture is
  cited by file and line, not reproduced.
- **Tip state only.** The repositories are cited as their files stand at
  the commits above. Files an author deleted, present only in a
  repository's history, were listed to keep them out and classified for
  offline validation only; the spec does not cite, quote or describe them.
- **Still not opened**: forums of any kind (EEVblog included) and the
  articles and pages the repositories link to; TestController beyond its
  supported-equipment list, and `references/testcontroller.md`;
  `docs/research/new-device-candidates.md` and
  `docs/research/non-unit-candidates.md`; the web in general.

**Model-recall disclosure.** The assistant that wrote these documents may
have been trained on community write-ups of this protocol. Recall was never
used as a source: every fact in §1-11 and every question of §12 cites a
page of a Brymen document or a line or address of a Brymen program, a
platform convention is named where one is used, and a fact without either
is tagged [UNVERIFIED]. Every fact in §13, and so every "Community:" note in
§12, cites a community file (and line where it has one) at the commit or
fetch date above, or a re-read of a vendor source.

## Methodology

1. **Acquisition.** The sheets, the BM520s zip, the manuals and the three
   program installers from brymen.com, 2026-09-26; every file hashed
   (`SOURCE.txt`). The installers (InstallShield-style self-extractors)
   unpacked with `7z x`; the program executables and READMEs copied to
   `app/`.
2. **Renders.** `pdftoppm -r 150 -png` and `pdftotext -layout` of every PDF
   into `renders/`; `pdfinfo` for the metadata. The readers re-rendered
   cells at 300-1200 dpi from the same PDFs; the LCD figures and table
   crops kept in `analysis/renders-hires/`. The `.txt` files served only to
   locate text: they drop every icon cell and circled number and misplace
   BM820/BM520 row 9.
3. **Decompilation.** Ghidra 12.1.3 headless, x86:LE:32:default, compiler
   spec `borlandcpp`, `GhidraDecompileAll.java` (in
   `references/ghidra-scripts/`) for the full listings. Published VCL event
   handlers the auto-analysis missed were found by a published-method table
   scan (`published_methods.py`) and an RTTI published-field scan
   (`field_tables.py`), then decompiled with `DelphiDecompileHandlers.java`.
   Functions whose Borland RTL calls decompiled badly were re-decompiled
   with `SigFixDecompile.java`, which gives the RTL helpers register-
   convention signatures (`*-sigs.txt`); string literals were filled in by
   `annotate_dat.py`. String tables dumped with `7z x` and
   `dump_strings.py`; imports and exact instructions read with
   `objdump -d -p` (`disasm_annot.py`, `jt_blocks.py` for jump tables).
4. **Three readers**, each given its sources and the tag rules of
   `.claude/rules/research-docs.md`, none given another's report:
   - the four HID sheets and BM250 (`protocol-docs.md`), every table read
     from the renders;
   - the two manuals (`manuals.md`), from the renders;
   - the programs (`app.md`), which did not open the sheets or the
     manuals; its brief carried seven questions drawn from them (VID/PID,
     request bytes, reply indexing, model byte, the BM860 points, letters,
     bar graph), so its reading is not fully blind to the documents.
5. **Adjudication and drafting** by one drafter from the three reports.
   Where they disagreed or a claim carried weight, the source was re-read:
   the BM860, BM820 and BM520-ML first pages and the BM860 example's
   secondary point (hi-res crop); MRAD p.1, p.8, p.9; both READMEs in full;
   the programs' flag, exponent and unit functions (every bit of both maps
   checked against Table 1), the VID/PID compares, the live request loop,
   the probe's model-byte mapping and the interval table in the
   disassembly, and the import's function-code mapping. The 7-segment
   tables of both programs were checked against each other through the
   segment bits of §7.1 (all agree). The manuals were searched again for
   the unexplained annunciators (2026-09-27).
6. **Grounding check.** A fresh reader checked both documents against the
   same vendor sources (`findings/grounding-check.md`, 20 rows). Each row
   was re-read in the source before it was applied: chiefly the mV code
   pairing, the `52 89` retry counts (the disassembly showed Bs8252x
   counting a bad checksum as a timeout, where the check had both programs
   alike), the second VID/PID opener, the E9015 waiter, MRAD's 1.6 s
   restart, per-sheet circled numbers, manual page cites and absence
   wording. The name of the BM250s cable was replaced by the ones in the
   Bs25x README.

Resolved disagreements:

1. **Report II position** (spec §4.3): BM820 Table 1, BM860, MRAD and the
   9-byte reads put it at byte 10; the BM820 example and BM520-ML Table 1
   at byte 11. Bs8252x indexes byte 10 as the report ID and bytes 11-14 as
   the secondary digits. Recorded as the stronger reading; still open for
   hardware.
2. **BM860 secondary point** (spec §9.1): the example's hex sets 7p, its
   figure lights 8p and its caption reads 60.11. Fig 1 and Bs86x both put
   7p after digit 7, so the example, not the labels, is inconsistent.
3. **VID:PID**: 0x0820:0x0001 in both programs' compares; "0x82"/"0x01"
   in every flowchart. Both recorded; the programs' value tagged [VENDOR].
4. **BM520s request byte**: the sheet's `52 66` is never sent by either
   program; Bs82-52x uses `82 66` for both series and gates the import on
   byte 23 = `52` of an `82 66` reply. Left open.
5. **Logging interval code 0**: `app.md` gave the programs' table as
   "0.1…600 s"; the disassembly has 0.05 s for code 0
   (`@ 0x42ecd9`), matching MRAD Table 3-1.
6. **The `0x1000000` variant**: `app.md` guessed LoZ. The import's
   function mapping gives it to MRAD's AC and DC millivolt functions, so the
   program takes byte 18 bit 3 as "mV"; what the meter sends there stays
   open.
7. **Memory digits**: `protocol-docs.md` read MRAD's D3-D0 as decimal
   digits; MRAD does not say so, and the programs format the two bytes as
   one binary integer. Left open.
8. **Cable for the BM820s**: the manual says BU-82X on p.13 and BU-86X on
   p.20; both READMEs, which the readers had not reported, name BC-86X for
   both programs. Recorded as three sources to one; left open.
9. **MAX-MIN on the BM820s**: `manuals.md` could not tell whether MAX, MIN
   and AVG are separate segments; BM820 Table 1 has MAX, MIN, AVG and the
   ⑤ dash as separate bits.
10. **APO when linked**: the manuals are silent; both READMEs say linking
    disables it.

## Confidence

| Area | Confidence | Why |
|---|---|---|
| Request bytes, 27-byte reply, report-ID positions (BM860s) | High | sheet, flowchart and program agree |
| Report-ID positions (BM820s/BM520s) | Medium | BM820 Table 1, MRAD and the program agree; the BM820 example and BM520-ML Table 1 do not |
| VID:PID | Medium | the programs are unambiguous; the sheets print another value |
| LCD maps, bit by bit | High | every bit the programs read matches Table 1 of its sheet, except 18.3, which the BM820 sheet marks "don't care" |
| Decimal points | High, but for the BM860 example | labels, figures and programs agree |
| Minus segments | Medium | position and program use; never labelled |
| Character table | Medium | 0-3 and 5-8 in the sheets' examples; the rest program-only |
| Annunciators the programs ignore | Low to medium | labels only; several meanings not stated anywhere |
| Timing, rates, no-meter behaviour, APO | Low | flowchart and program disagree; READMEs describe symptoms only |
| BM520s real-time request | Low | the sheet and the programs disagree |
| Memory download | Medium | MRAD and the programs agree on commands, reply and checksum; MRAD is internally inconsistent and the record chaining is not read |
| Manual facts | High for what is drawn | four models' dials are not drawn |

## Tag legend

As used in `reverse-engineered-protocol.md`:

- **[KNOWN]** — stated in a Brymen document (protocol sheet, logged-memory
  document, README, manual), cited by page or section
- **[VENDOR]** — read from Brymen's programs, cited by file and line or
  address
- **[INFERRED]** — deduction from the above, reason given
- **[UNVERIFIED]** — no source confirms it, or the sources disagree; needs a
  real meter
- **[HARDWARE]** — seen on a real meter: none yet
- **[COMMUNITY]** — stated in or implied by a community source, spec §13
  only, summarised in §12's "Community:" notes; not a vendor fact
