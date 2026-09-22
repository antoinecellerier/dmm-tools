# UT8805 / UT8806: Reverse Engineering Approach

Scope: the UNI-T bench multimeters UT8805N, UT8805A and UT8805E (5½
digits) and UT8806, UT8806A and UT8806E (6½ digits). UNI-T's Chinese
download centre also lists a "UT805A+" whose programming manual and
firmware listings serve the UT8805N files byte for byte, so the UT805A+
is read as a UT8805N rebrand [INFERRED]. None of these meters is
implemented; this pair of documents records what they do on the wire so
that a go/no-go and an implementation can build on it.

## Sources Used

All UNI-T's own. Fetched 2026-09-21, analysed 2026-09-21 and 2026-09-22.
Provenance — verbatim listing title, URL, size, MD5 and server date for
every file, and the tool that unpacked each archive — is in
`references/ut8805/SOURCE.txt`; the findings reports are in
`references/ut8805/analysis/findings/`.

Where the files came from: UNI-T's Chinese bench download centre
(instruments.uni-trend.com.cn, files on its Shenzhen OSS store), UNI-T's
global download centre (instruments.uni-trend.com) and UNI-T's US site
(uni-trendus.com). The last two link their files on UNI-T's own stores,
`storage.googleapis.com/uni-tdocuments` and
`unitrend.oss-cn-hongkong.aliyuncs.com`, which were fetched because the
US and global sites link them. Files on UNI-T HQ's SharePoint
(unitrendhq.sharepoint.com: UT8805E firmware V1.88.001, UT8806E firmware
V1.01.0101, Instrument Application V2.00) were listed but not fetched.

### Primary (clean-room RE)

1. **SCPI programming manuals** — UT8805N V3.0 (Chinese, 44 pp), UT8805A
   V1.0 (Chinese, 73 pp), UT8805E V1.1 (English, 49 pp), UT8806 V1.0
   (Chinese, 87 pp), UT8806A V0.1 (Chinese, 90 pp), UT8806E V1.0
   (English, 91 pp). Command trees, syntax, reply examples, VISA
   examples. Tagged [KNOWN] with the manual key and page
   (`manuals-8805.md`, `manuals-8806.md`). `references/ut8805/manuals/`.

2. **User manuals, datasheets, quick guides** — one set per model, plus
   the US site's older UT8805E user guide and UT8806E user manual, the
   UT8806 service manual and the bench-DMM calibration guide. Ports,
   RS-232 settings, LAN menu, counts, ranges, rates, display behaviour.
   Tagged [KNOWN]. Same directory.

3. **Firmware images** — UT8805N SW V1.87.014 (`UT8805.UGD`, a plain
   Cortex-M image, STM32F4-class), UT8805A V1.01.0010 (`DMM8805 ATE.UPG`,
   STM32H7-class), UT8806 V0.01.0101 and UT8806E V0.01.0100 and V0.01.0085
   (`DMM8806*.UPG`, STM32H7-class), with the UT8806E release notes and
   the upgrade guides. USB descriptors, USBTMC handling, the registered
   SCPI command tables, reply formatting, LAN services, remote/local
   logic. Tagged [VENDOR] with the image and a function address or RAM
   offset (`firmware.md`; tables and decompiles in
   `references/ut8805/firmware/analysis/`).

4. **UNI-T's PC software** — the UT8805 apps (V1.10 2025, UT8805N V2.0
   2021, UT8805E V1.09 2024; 32-bit MSVC, Qt5, VISA32 by ordinal) and
   the Instrument Application V3.0 (2026; `IA.exe`, `libcxp2_measure.dll`,
   `plugin/libUT8806.dll`, MinGW Qt6 with symbols). What UNI-T sends to
   drive the meters. Tagged [VENDOR] with the binary and function address
   (`software.md`; `references/ut8805/software/`).

5. **UT8806 drivers** — the IVI-C driver V1.0 (with its full C++ source,
   `Source/UT8806.cpp`, plus the compiled Nimbus DLLs) and the LabVIEW
   driver V1.0 (73 VIs, LabVIEW 20). Tagged [VENDOR] with file:line or VI
   name (`software.md`; `references/ut8805/drivers/`).

6. **UNI-T SDK V2.3** — the Chinese RAR (same code and libraries as the
   ZIP under `references/ut8803/`, plus Chinese manuals). Read to settle
   whether UCI covers these meters: it does not (`uci-sdk.md`).

### Avoided (clean-room boundary)

- **No community source was read:** not sigrok, not any GitHub project,
  not EEVBlog, not TestController, not pyvisa's documentation. A
  separate scoping report on Rigol and Siglent bench DMMs, from those
  makers' own documents, sits in the same findings directory
  (`rigol-siglent.md`); nothing from it is used here. Every claim in
  `reverse-engineered-protocol.md` rests on the UNI-T files above.
- **Opened 2026-09-22, with approval, after the spec was committed as
  fdddfa2.** What was read, and what it changed, is in
  [Cross-Reference with Community Sources](#cross-reference-with-community-sources)
  below and in the spec's §14. Nothing in the spec's §1-§13 was
  rewritten on it: §1, §2.1, §3.1, §4 and §5.3 carry one-line pointers
  to §14 where it settles, disputes or adds a wire fact, §13 records its
  weak rows as leans, and no community source is cited inline.

## Methodology

1. **Manuals.** `pdftotext -layout` to locate passages; every table,
   syntax line and reply example that the spec cites was checked on the
   page rendered with `pdftoppm -r 110`. Printed-page offsets per file
   are in the two manuals reports. Where two manuals disagree (buffer
   size, `CONFigure?` format, trigger delay, continuity range, the
   UT8806A's "2→1" edits) both readings are recorded, not resolved.

2. **Firmware unpacking.** The `.UPG` files carry an 8-byte header
   (little-endian length, then an unidentified 32-bit value) before a
   Cortex-M vector table; `UT8805.UGD` has none. The USB descriptors
   live only in compressed initialised data: on the UT8805N a Keil
   armlink region table (0x080F298C) names an LZ77 blob unpacked by the
   image's own `__decompress` (0x080081C4); on the H7 images IAR's
   `__iar_lz77_init3` blobs unpack to 0x24000000 and 0x30000000
   (`scripts/iarlz77.py`, `iarinit.py`). The unpacked RAM images are
   `*_ram_*.bin`.

3. **Firmware analysis.** Headless Ghidra 12.1.3, one project per image
   (`ghidra_proj/`), functions decompiled by address on demand
   (`decomp.sh` with `Decomp.java`, xrefs with `xrefs.sh`, scalar scans
   with `scalars.sh`); the SCPI dispatch tables walked as
   `<keyword, handler, flag>` triples from a known entry
   (`scripts/scpitab.py` → `*.scpi.txt`) and normalised into sorted
   keyword lists (`*.norm.txt`) for diffing; the USB descriptor and
   string-descriptor builders, the USBTMC class-request and bulk-OUT
   handlers, the RTOS thread table and the socket/port constants
   decompiled (`d*.c`, `x*.txt`, `s*.txt`).

4. **IVI-C driver.** Read from its shipped C++ source; the identity
   query and model check, which live in the compiled Nimbus framework,
   from a Ghidra pass over `UT8806_64.dll`
   (`drivers/analysis/ivic_UT8806_64_decomp.c`).

5. **LabVIEW driver.** A zlib scan of the VI resources
   (`drivers/analysis/lv_vi_strings.py`) yielding the strings and the
   SCPI words per VI; no LabVIEW was run.

6. **Qt apps.** `strings`, symbol tables (the IA plugin has DWARF) and
   Ghidra with a filtered decompile (`DecompFiltered.java`) over the
   send path, the worker loop, the reply parsers and the VISA resource
   builders. No executable was run.

7. **Reconciliation.** Each manual claim was checked against the
   firmware's command tables and the tools' usage; each disagreement is
   stated with its sources in the spec.

### What the firmware report lost

The firmware findings report (`firmware.md`) was cut off during its §2
(LAN) by a safety classifier, and its SCPI, reply-format and
remote/local sections were never delivered. The spec therefore rests,
for those subjects, on the report's per-image summary table, which was
delivered, and on the analysis files:

- LAN services and ports per image: summary table only. Thread names and
  port constants are in `d8806_*.c`, `d8805a_*.c`, `s*.txt`.
- `*IDN?` format, reading format, overload literals, the UT8805A
  `READ?` sign quirk, reply terminator: summary table; the string
  neighbourhoods in `*.strings.txt` (e.g. `UNI-T`, `UT8805`,
  `SW V1.87.014` adjacent at UT8805N 0xEA768-0xEA780).
- Which commands enter remote, `*UNREMOTE`, `KEY:SET`, `SYST:LOC/REM/RWL`:
  summary table; the handler addresses and the table flag on `*IDN?`
  are in `*.scpi.txt`.
- The command tree per image, and every model difference the spec draws
  from it: `*.norm.txt`, diffed directly for this document.

## Key Findings

- **All six models are USBTMC/USB488 devices on the MCU's own USB
  port, plain SCPI, no bridge chip.** Two firmware lines: the UT8805N
  (STM32F4-class, 0483:7540, `\r\n` replies, portmapper + VXI-11 core
  only) and the H7 line (UT8805A and all UT8806, 0483:5740, `\n`
  replies, web + raw socket 5025 + VXI-11 + mDNS). [VENDOR]
- **The two VID:PIDs in UNI-T's tools are both right**, each for its
  line: the UT8805 apps filter on 0483:7540 (the UT8805N descriptor),
  the IVI examples use 0483:5740 (the H7 descriptor). The manuals'
  `0x5345:0x1234` matches no image. [VENDOR]
- **No interrupt-IN endpoint and no USB488 requests**: the interface
  claims USB488 but `GET_CAPABILITIES` advertises nothing, and
  READ_STATUS_BYTE, REN_CONTROL, GO_TO_LOCAL and LOCAL_LOCKOUT are not
  handled. [VENDOR]
- **Remote/local differs by line**: the UT8805N enters remote on any
  query, the UT8805A on `READ?`/`FETCh?`/`MEAS?`, the UT8806 on every
  command but `*IDN?`; `*UNREMOTE` on all but the UT8806E 0085;
  `SYST:LOC/REM/RWL` on the UT8806 only. [VENDOR]
- **`DATA:LAST?` is the one reading query documented as usable at any
  time, during a measurement series included**, and the only one with a
  unit suffix; `MEASurement:CONTinuous` + `READ:LAST?`, which the UT8805N
  V2.0 app uses, is registered by every image but the UT8806E 0085 and
  documented nowhere, as is the UT8805 apps' `:SYNC:DATA?`. [KNOWN]
  [VENDOR]
- **The firmware settles the `CONFigure?` and `FUNCtion?` formats**:
  `CONF?` answers `VOLT:DC +2.00000000E+01` (unquoted, space) and
  `FUNC?` answers `"VOLT:DC"` (double-quoted); the UT8805 manuals print
  `CONF?` quoted (EP, NP) or with a comma (AP), the UT8806 manuals in the
  firmware's form, and every manual prints `FUNC?` unquoted. [VENDOR]
- **The E models are the base models' export builds** (EP translates
  NP and calls the meter "UT8805", EP p.4, p.18; S6E has S6's 171
  headings with identical syntax lines; the photographed bezels read
  "UT8805", EU p.10, and "UT8806", U6E p18; the E85/E100 images report
  model `UT8806`); the UT8806A is a 1.2M-count variant (U6A p8) with its
  own ranges. [INFERRED]
- **The UNI-T SDK does not cover these meters**: UCI's device tables
  hold no UT8805/UT8806 and its readme sends SCPI instruments to
  NI-VISA. [VENDOR]

## Confidence Assessment

- **USB identity, class, endpoints:** HIGH — read from the descriptors
  of all five images, and consistent with the tools. **USBTMC subset,
  bTag rules:** HIGH for the N and UT8806 0101, whose handlers were
  decompiled; the other images share the code base [INFERRED].
- **Reply terminator, `*IDN?` shape, overload literals:** MEDIUM — from
  the firmware summary table whose supporting text was lost; the string
  tables agree; no reply has been captured.
- **LAN services per line:** MEDIUM — summary table plus the thread and
  port constants; no port has been probed.
- **Command tree per model:** HIGH for what each image registers; the
  argument syntax rests on the manuals, which contradict themselves in
  places (spec §11, §13).
- **`CONFigure?` and `FUNCtion?` reply formats:** MEDIUM — the reply
  writers of all three lines agree (unquoted with a space; double
  quotes); the UT8805 manuals print `CONF?` two other ways and every
  manual prints `FUNC?` unquoted; no reply captured.
- **RS-232 SCPI:** MEDIUM — the manuals give the line settings, the
  UT8806E release notes mention serial reception, UNI-T's apps open
  `ASRL` resources; terminator and handshake unknown.
- **UT8806A specifics:** LOW — its manual carries a mechanical "2→1"
  edit and no UT8806A firmware was available.
- **Anything about a real reply:** [UNVERIFIED] — no UT8805 or UT8806
  has been on our bench; what other people's UT8805E units have shown
  (the `*IDN?` first fields, the firmware line, SCPI over RS-232, the
  VXI-11 padding) is in the cross-reference below and the spec's §14.

## Cross-Reference with Community Sources

Opened 2026-09-22, with approval, once the spec above was committed
(fdddfa2). Read, by source type (`references/ut8805/analysis/findings/community-xref.md`
has the per-item table and URLs):

- **Device definitions:** HKJ TestController's `UT8805E.txt` (v1.0,
  2026-05-13, "tested with RS232 under linux only") and its
  device-definition documentation; the EEVBlog TestController thread
  replies that added it.
- **User reports from real meters:** the NI forum thread "UT8805E Pyvisa
  Communication" (2024-10 to 2025-12), with raw VXI-11 tests and a
  support-sent firmware.
- **Reviews and teardowns:** Voltlog's UT8805E review and "New Revision"
  videos and blog post (web page and About screens read from frames);
  Kerry Wong's UT8805E and UT8806E teardowns and UT8806E review.
- **Code:** a B&R PLC VXI-11 client, the psytestbench README, libsigrok's
  `scpi-dmm` and USBTMC backend, the Linux `usbtmc` and `cdc-acm` drivers.
- **Hardware databases:** linux-hardware.org and `usb.ids` for
  0483:7540 and 0483:5740.
- **For the rebrand question only:** EEVBlog buying-advice threads and
  Teledyne's and Siglent's firmware pages (T3DMM = Siglent SDM3000; no
  UT8805/UT8806 rebrand found, no sign of a Rigol/Siglent/UNI-T OEM
  link; recorded in `docs/research/new-device-candidates.md`).
- **Searched, nothing found:** sigrok's wiki and udev rules, GitHub
  issues and code, pymeasure, QCoDeS, python-ivi, InstrumentKit,
  pyvisa-py and lxi-tools issues, GitLab, PyPI, npm.

The comparison — what it settles, disputes and adds — is the spec's §14.
