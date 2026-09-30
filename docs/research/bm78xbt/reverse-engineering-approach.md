# Brymen BM78xBT: Reverse Engineering Approach

Scope: the Brymen BM78xBT Bluetooth LE multimeters, the BM788BT and the
BM787BT that EEVblog sells, from the advertisement to the packets they send
and the commands they accept. It is implemented, experimentally; this pair of
documents records what the meters do on the wire. §1-11 of the spec come from vendor
sources only; §12 compares them with community sources, read after the
boundary was opened.

Brymen publishes the protocol, so the question was not what the packets are
but how far the published document can be taken at its word. The answer:
**r4 gives the whole reading path** — advertisement, GATT, password,
152-byte output, reading packet, units and function IDs. It leaves byte
orders and encodings open, and Brymen's app settles several of them: the CRC
low byte first, the command code low byte first, the firmware version as
Arg2.Arg1.Arg0, the prefix as a signed byte. The app also sends three
commands r4 does not list on every connect (0x0101, 0x0106, 0x0021), reads
bits r4 marks "don't care", contradicts r4 in five places and uses two
password digit orders where r4 states none; each is left open, and
`verification.md` lists the check.

## Sources Used

All Brymen's own, or linked from EEVblog's BM787BT store page; source 8 is
general platform reference material. Fetched and analysed 2026-09-26.
Provenance — URL, date, SHA-256, signer — is in
`references/bm78xbt/SOURCE.txt` (gitignored); the five reader reports
(`protocol-doc.md`, `protocol-r2-vs-r4.md`, `manual-bm788bt.md`,
`manual-bm787bt-diff.md`, `app.md`) are in
`references/bm78xbt/analysis/findings/`.

### Primary (clean-room RE)

1. **"BM78xBT Wireless Data Communication Protocol" r4** (the printed title;
   the PDF title says "Wireless Communication Protocol"), 15 pages, PDF
   author Gary Wang, created 2025-10-31 ("Microsoft: Print To PDF"), linked
   from
   http://www.brymen.com/PD02BM788BT_protocolDL.html. No revision history.
   Read from renders, p.12 again at 300 dpi; `pdftotext` only to locate text
   and for the r2 diff. Tagged [KNOWN], cited `r4 p.N`.
2. **The same document, r2**, 14 pages, created 2025-04-07; PDF title "BM780
   Wireless Communication Protocol-r2", printed title as r4's. eevblog.com
   serves a bot challenge to
   agents, so the user saved it from
   https://www.eevblog.com/files/BM780-Wireless-Communication-Protocol-r2.pdf,
   linked from https://eevblog.store/products/eevblog-bm787bt-bluetooth-multimeter.
   Used to date r4's changes and to read its red revision marks. Tagged
   [KNOWN], cited `r2 p.N`.
3. **"BM78xBT OTA programming" rev4**, 5 pages, same author and date as r4,
   from brymen.com. Used only for the GATT, advertising and module facts and
   the OTA entry; the bootloader protocol was not analysed. Cited `OTA p.N`.
4. **IoMBTC Wireless Data Comm 1.0.73** (`com.IOMBTC.app`, versionCode 23),
   Brymen's app, linked from http://www.brymen.com/Download4.html to Google
   Play. The user installed it from Play and pulled the four split APKs from
   the device. A Flutter build: the protocol code is Dart AOT in
   `lib/arm64-v8a/libapp.so` (package name `brymen`), BLE through
   flutter_reactive_ble. Decompiled with blutter; permissions from `aapt`.
   Tagged [VENDOR], cited `file:lines` of the blutter listing.
5. **BM780(BT) user's manual** (`BM780(BT)-Print2.pdf`), BM788BT, BM789 and
   BM785, ©MMXXVI, P/N 7M1C-1601-C000, from
   http://www.brymen.com/PD02BM780(BT)_usersmanualDL.html. Read from renders,
   re-rendered at 400-1200 dpi where needed. The QR code on p.19 was decoded
   locally with libzbar (`http://www.brymen.com/Download.html`), not visited.
   Tagged [KNOWN], cited `BM788BT manual p.N (printed M)`.
6. **BM787BT manual** (`BM787BT-Manual.pdf`), BM789, BM785, BM788BT and
   BM787BT, ©MMXXV, PDF created 2025-04-09, the older edition; saved by the
   user from https://www.eevblog.com/files/BM787BT-Manual.pdf (the same store
   page). Brymen-branded; read against source 5 page by page. Tagged [KNOWN],
   cited `BM787BT manual p.N (printed M)`.
7. **BM788BT catalogue sheet** (`BM788BT_Catalog.pdf`, 4 pages), from
   http://www.brymen.com/PD02BM788BT_catalogDL.html: fetched, not analysed.
8. **General platform references**, for statements tagged [INFERRED] with
   this basis in the spec: the Bluetooth Core Specification's AD format (the
   flags byte, the company identifier low byte first) and ATT notification
   header (MTU − 3 bytes of payload), and the CRC catalogue name
   CRC-16/MODBUS for r4's routine. No BM78xBT-specific content comes from
   them.

| Source | File (under `references/bm78xbt/`) | SHA-256 |
|---|---|---|
| Protocol r4 | `protocol/BM78xBT-Wireless-Communication-Protocol-r4.pdf` | `683f7b9bf941afacb9d52ba1d49da5c05af3717e32b427a30b0625b05e31160f` |
| Protocol r2 | `protocol/BM780-Wireless-Communication-Protocol-r2.pdf` | `edb1850f7764e5b3fe3af97229590d9e14b7186cee7061993f364e426af327e0` |
| OTA rev4 | `protocol/BM78xBT-OTA-programming-rev4.pdf` | `a02d311363dfdb9ca828d88bd5385c66eee384312ec4bfda9924ce3d0c73de77` |
| BM780(BT) manual | `manuals/BM780(BT)-Print2.pdf` | `a9f99e8345e24084aa74033cd7ffc0021798bebebf71e341dfa97c07e60b388e` |
| BM787BT manual | `manuals/BM787BT-Manual.pdf` | `4ac7baf1fd34f111d27e41ad2937536cbe2d5cb65460cbc215788c067d9ec440` |
| Catalogue | `manuals/BM788BT_Catalog.pdf` | `712eb5bc8edade9bc52bd88d526ae7b32f8a2643cb544f11c337bb9e5c6e0498` |
| App, base | `app/base.apk` | `5727cf6ca62d2159c9f55317826152c8a3b1648357947fa3520a78f948ac4b14` |
| App, arm64 split | `app/split_config.arm64_v8a.apk` | `ca7cdfbc16d16b6a708b1f6f35e722f76baf0339a9fd69a5af4acf9bc1ebcc2c` |

The hashes of the other two splits (`en`, `xxhdpi`) are in `SOURCE.txt`.
The APKs are signed `CN=Android, OU=Android, O=Google Inc., L=Mountain View,
ST=California, C=US`, cert SHA-256
`1638e9666e2e68e81b3c6654de89965f05dd1a54d71ac09db3625433b67e369d`.

### Avoided during the vendor analysis

Not searched for or opened while §1-11 of the spec were written:

- sigrok (libsigrok and the wiki)
- code repositories of any kind for these meters or for Brymen's protocol
- the EEVblog forum and other forums, blog posts and videos
- eevblog.com beyond the two files the user saved, and the web in general
  beyond the brymen.com pages and files above
- `docs/research/new-device-candidates.md`,
  `docs/research/non-unit-candidates.md` and `references/testcontroller.md`:
  they hold community-sourced notes on Brymen meters written before this
  work, and were kept out of every reader's brief and out of the drafting of
  both documents

### Cross-referenced (clean-room boundary opened 2026-09-26)

The user opened the boundary on **2026-09-26**, after §1-11 were written,
grounding-checked and committed, for code repositories only: no forums, no
LastDigit. The findings are §12 of the spec, marked [COMMUNITY]; nothing was
merged into §1-11 beyond pointers. Provenance is in
`references/bm78xbt/community/SOURCE.txt` (gitignored). The repositories are
cited as their files stand at the commits below. No community code was copied.

| Source | Commit | Licence | Notes |
|---|---|---|---|
| [milksplash/brymenble](https://github.com/milksplash/brymenble) | `02e28b6` (2026-08-31) | MIT © 2026 Martin Chan | Python SDK on bleak, with capture and probe tools |
| [milksplash/brymenble-tc-bridge](https://github.com/milksplash/brymenble-tc-bridge) | `6355bbd` (2026-08-31) | MIT, same holder | forwards SDK readings to TestController |
| [milksplash/brymenble-overlay](https://github.com/milksplash/brymenble-overlay) | `af59fec` (2026-08-31) | MIT, same holder | video overlay built on the SDK |

All three are one author's. Their protocol source is r2, by the author's own
file name for it (`brymenble/.gitignore:18`), and their command set lacks
r4's 0x0040, as r2 does (spec §12). Searched on 2026-09-26 with nothing
further found: GitHub repository search for "BM78xBT" (only the three above)
and for "BM788BT OR BM787BT OR BM786BT", GitHub code search for the
characteristic UUIDs, and libsigrok code search for "BM78" and "cdd5".

Still not opened: forums of any kind, LastDigit, and the three in-repo files
listed above, which were also kept out of the drafting of §12.

### Model-recall disclosure

The assistant that wrote these documents may have been trained on community
write-ups of this protocol. Recall was never used as a source: every fact in
§1-11 of `reverse-engineered-protocol.md` cites a page of a Brymen document
or a file and lines of Brymen's app, a general platform reference is named
where one is used, and a fact without either is tagged [UNVERIFIED]. Every
fact in §12 cites a file and line of a community repository at the commit
above.

## Methodology

1. **Acquisition.** The protocol, OTA, manual and catalogue PDFs from
   brymen.com; the r2 document and the BM787BT manual saved by the user from
   eevblog.com; the app's split APKs pulled by the user from a Play install.
   Every file hashed (`SOURCE.txt`).
2. **Renders and app extraction.** `pdftoppm -r 150 -png` of r4, r2, the
   OTA document and both manuals into `renders/`; `pdfinfo` for the metadata.
   `libapp.so` and `libflutter.so` unzipped from the arm64 split,
   `strings -n 6` of `libapp.so`, and blutter
   (github.com/worawit/blutter @ `4a60ac6`) run on `libapp.so`: Dart 3.6.2,
   the app's own package in `asm/brymen/`. The listing's conventions (unboxed
   integers, Smi constants printed doubled, list indices halved, enum names
   from `objs.txt`) are recorded in `findings/app.md`.
3. **Five readers**, each given its sources and the tag rules of
   `.claude/rules/research-docs.md`:
   - r4 and the OTA document (`findings/protocol-doc.md`), every page read
     from the render; p.14's struck rows checked there, as `pdftotext` drops
     the strikethrough;
   - r2 against r4 (`findings/protocol-r2-vs-r4.md`): a `pdftotext` diff, a
     pixel diff of aligned pages (ImageMagick `compare`, 10 % fuzz), a count
     of coloured pixels on every page, and the differing regions read at
     zoom. Given `protocol-doc.md` for its list of ambiguities;
   - the app (`findings/app.md`), given `protocol-doc.md` so it could answer
     the document's open questions; its reading of the app is therefore not
     blind to r4;
   - the BM780(BT) manual (`findings/manual-bm788bt.md`);
   - the BM787BT manual against it (`findings/manual-bm787bt-diff.md`,
     renders, `pdfimages` for the embedded images, a word diff of the two
     texts).
4. **Adjudication and drafting** by the main session, from the five reports.
   Where two reports disagreed or a cite was loose, the vendor source was
   re-read: r4 p.7, 8, 10, 12, 13, 14 and 15, and the app's AutoCheck and EF
   sub tables (`bluetooth_command.dart:12139-12161`, `:13985-14012`). The
   worked examples were built from the layouts and their CRCs computed with
   r4's routine, then checked by recomputing each CRC over [2] through the
   CRC bytes, which gives 0.
5. **Grounding check.** A fresh reader checked both documents against the
   same vendor sources (`findings/grounding-check.md`). Each item was re-read
   in the source before it was applied: chiefly page cites (the r2 page map,
   the password default, the manual pages), the app's category lookup (its
   result is unused), the value formula for [24] = 0, the list of main IDs
   outside r4, tags on deduced statements, and absences rephrased as dated
   search results.
6. **Community cross-reference** (boundary opened 2026-09-26, above). A
   fresh reader compared each claim of spec §1-11 with the three
   repositories. The main session then re-read every cited line at the
   commits above, sorted each fact by what it rests on (the author's
   statement, or the SDK working at all, since no bytes a meter sent were
   found in the repositories' files), and re-read the vendor source
   for each disputed point: r4 p.1, p.7, p.10 and p.15 and the app. No
   verdict in §1-11 changed; eleven open questions were narrowed and two
   added (now in `verification.md`).
7. **§12 check.** A fresh reader checked the §12 changes against the same
   commits and the vendor sources. Each item was re-read in the source before
   it was applied: chiefly exact quotes and cites, evidence labels ("author"
   against "SDK"), the body's pointers cut to bare references to §12, and
   absences rephrased as dated search results.

Resolved disagreements:

1. **r2 against r4**: r2 adds nothing r4 lacks. r4 adds 0x0040, the firmware
   version example and flowchart footnote 2, and drops "…… to be continued"
   after the error codes; its other pages match r2 pixel for pixel or in text
   (`protocol-r2-vs-r4.md`). r2's red rows date the struck "Hz of …"
   sub-functions and main 0x23 to one edit; what the firmware sends stays
   open.
2. **CRC byte order**: low byte first, from the app's writer and checker in
   three places; r4 and r2 are silent and hold no complete frame.
3. **Command code byte order**: low byte in [11], from r4's
   `[Command1:Command0]` notation and the app's code table.
4. **Firmware version**: Arg2.Arg1.Arg0, from r4's example and the app's
   formatter.
5. **Decimal point**: [24] counts integer digits, from r4's tables and the
   app's arithmetic.
6. **Prefix encoding**: a signed byte, from the app; r4 gives only the values.
7. **Response channel**: the app reads CDD4; r4's flowchart order fits it.
   Whether the meter also notifies responses stays open.
8. **Left open, both sides cited** (`verification.md`): the five
   contradictions — AutoCheck sub `02` OHM (app) against `03` AUTO (r4);
   unit `4F` (r4) missing from the app; name length 12 (r4) against 11
   (app); [13] always `01` (r4) against `00` in four app commands; line
   frequency as main `23` (r4) against sub `03` "HZ", which r4 strikes
   (app). Separately, the password digit order: reversed in the app's
   0x0151 and in order in its 0x0140, with r4 silent.
9. **Manuals**: the BM787BT has T1 only and no %4-20mA (BM787BT manual p.12,
   p.15); the other differences from the BM780(BT) manual (continuity
   threshold, REC AC rate, accessories, radio certification) may be edition
   changes, since the BM787BT manual is the older edition. The APO
   contradiction (30 against 15 minutes) is in both.

## Tag legend

As used in `reverse-engineered-protocol.md`:

- **[KNOWN]** — stated in a Brymen document (protocol, OTA, a manual), cited
  by page
- **[VENDOR]** — read from Brymen's app, cited by file and lines
- **[INFERRED]** — deduction from the above, reason given
- **[UNVERIFIED]** — no source confirms it, or the sources disagree; needs a
  real meter
- **[HARDWARE]** — seen on a real meter: none yet
- **[COMMUNITY]** — stated in or implied by a community source, spec §12
  only; not a vendor fact
