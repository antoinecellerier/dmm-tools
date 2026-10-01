# OWON Bluetooth multimeters: Reverse Engineering Approach

Scope: OWON's Bluetooth LE multimeters — the B33(T)(+), B35(T)(+),
B41T(+), OW16B, OW18B, OW18E and CM2100B — from the advertisement to the
frames they send and the commands they accept. Not implemented yet; this
pair of documents records what the meters do on the wire, from vendor
sources only. The spec covers both frame formats OWON's app decodes: the
6-byte frame of these meters in full, and the 15-byte frame of the Voltcraft
VC831/851/871/891/915/925, OWON OW65/67/69 and CMS101/061 in its own section
(§10), from the app alone. Only the 6-byte group is planned for
implementation now; that is a project decision, not a protocol fact.

The question was what the meters send, with no published protocol. The
answer came from two independent OWON programs: **OWON's app and OWON's PC
source agree on the whole 6-byte frame** — word layout, function codes 0-12,
prefixes, decimal codes 0-4, status bits 0-5, the sign bit, the FFF1
command strings and the offline dump framing — and on the GATT roles of
FFF1-FFF4. They differ on status bits 6-15, function codes 13-15, decimal
code 5 and the `*READlen?` reply width, and only the app sends an MD5
challenge; each is left open with both readings in the spec, and
`verification.md` lists the check. The manuals add the models, the keys,
the Bluetooth and power behaviour and the default name "BDM", but nothing of
the wire.

## Sources Used

All OWON's own. Fetched and analysed 2026-10-01. Provenance — URL, date,
SHA-256 — is in `references/owon/SOURCE.txt` (gitignored); the reader
reports, grounding checks and adjudication are in
`references/owon/analysis/findings/`.

OWON's file server, files.owon.com.cn, sends no intermediate certificate
(fetched with `curl -k`), and its firewall returns empty 200 responses to
non-browser clients, so the manuals were fetched with a browser User-Agent
and each file checked by its magic bytes.

### Primary (clean-room RE)

1. **iMeter 1.2.4** (`com.owon.imeter`, versionCode 2044, by `aapt`), OWON's
   Android app, downloaded by the user from
   https://files.owon.com.cn/app/com.owon.imeter.apk (landing page
   https://files.owon.com.cn/i-meter/; on Google Play as `com.owon.imeter`,
   developer "OWON"). Every OWON product page in `web/` links its "app" to
   this APK. A Flutter build, Dart 3.9.2: the protocol code is Dart AOT in
   `lib/arm64-v8a/libapp.so`, packages `owon_imeter` and `imeter_base`, BLE
   through flutter_reactive_ble. Tagged [VENDOR, app], cited `file:lines` of
   the blutter listing.
2. **pcMultimeter 1.4.5** (20210324, plug-in `com.owon.pc.multimeter`
   1.4.4), OWON's Windows 7/8 PC software ("multimeterBLE"), downloaded by the
   user from
   https://files.owon.com.cn/software/pc/owon_multimeter_pc_software_en.zip.
   The package ships OWON's Java source as an Eclipse source bundle,
   `plugins/com.owon.pc.multimeter.source_1.4.4.jar`, unpacked to
   `pc/win7/src/`. The compiled `com.owon.pc.multimeter_1.4.4.jar` was used
   only to check that the source matches the shipped build. From the
   package's `bluetooth stack installation package.zip`, only the dongle
   driver's `oem6.inf` and TI's "Vendor Specific HCI Guide" (inside
   `BLE-CC254x-1.5.0.16.exe`, extracted with innoextract) were read, for the
   dongle layer's semantics. Tagged [VENDOR, PC], cited `path:lines` under
   `src/`.
3. **Eight OWON manuals**, downloaded by the assistant from
   `https://files.owon.com.cn/probook/<name>`, linked from OWON's product pages
   on owon.com.hk. Read from renders (150 dpi; 400-450 dpi crops of dials and
   icons); `pdftotext -layout` only to find pages. Tagged [KNOWN], cited
   `<manual> p.PDF/printed`.
4. **Seven product pages** from https://www.owon.com.hk/ and the **five
   datasheets** they link (`https://files.owon.com.cn/specifications/<name>`),
   fetched by the assistant: for the model lists, the Bluetooth and rate
   lines, and the range cross-check below. Tagged [KNOWN], cited by page. The
   B41T page's own spec sheet returns 404 (2026-10-01).
5. **PC software for win10/win11** (`PC_software_for_OWON_Blue_meter(win10
   win11 64bits system).zip`, holding `iMeterSetup_V1.2.2_build1_20250711.exe`),
   downloaded by the user from
   https://files.owon.com.cn/software/PC/PC_software_for_OWON_Blue_meter(win10%20win11%2064bits%20system).zip:
   fetched, not analysed.

| Source | File (under `references/owon/`) | SHA-256 |
|---|---|---|
| iMeter APK | `app/com.owon.imeter.apk` | `70abf8593e6611a239e11514666da73a94a61c9a56bb92ff19415e97aff11a7d` |
| PC package, Win7/8 | `pc/owon_multimeter_pc_software_en.zip` | `3416e648410ca701af3a76a180ed7835b3f6224f4d2be1acff54216c3bea99f1` |
| PC source bundle | `pc/win7/pcsw/pcMultimeter_v1.4.5_20210324.9999/plugins/com.owon.pc.multimeter.source_1.4.4.jar` | `ef96e929828b7a9846f6692b411e1e39ee9e60fb9966dd365c6cb1c50eff1ed9` |
| PC binary bundle | same directory, `com.owon.pc.multimeter_1.4.4.jar` | `5a16139fc37bbda7d24984a6b06519f3a092620d58336fddc6d0b17419caf407` |
| PC package, Win10/11 | `pc/PC_software_for_OWON_Blue_meter(win10 win11 64bits system).zip` | `2ad311779d6be61b94ddda48a9f255a19446e4ed727958d1ee76c2153dae0c3f` |
| B33-UM | `manuals/OWON_33_Series_Digital_Multimeter_USER_MANUAL.pdf` | `9d5b0e3a27739e4a4b6b35fe6dfb6fd2c09fe4c48f07e7d7a063bec4ca5b9eff` |
| B35-UM | `manuals/OWON_35_Series&B41T_USER_MANUAL.pdf` | `e1746e3fdf1ae8089f5aa4779a65e22623c1310e009f7e6497923b3d6697af07` |
| B35-QG | `manuals/OWON_35-Series&B41T_Digital_Multimeter_QUICK_GUIDE.pdf` | `d7effd29430109ba6d46574d65743d128cb62f5af79bafb0e0b9a7d492bd7d53` |
| OW16-UM | `manuals/OW16_Series_Digital_Multimeter_USER_MANUAL.pdf` | `980bb98e502c970e5094d38dc44de18a71985ed1f4394366c8dd17d7f947c292` |
| OW16-QG | `manuals/OW16_Series_Digital_Multimeter_quick_guide.pdf` | `37858ca79fe18b50504cf05a33584326f61061ac4d110d3e3ee558e3363e1daf` |
| OW18-UM | `manuals/OW18_Series_Digital_Multimeter_USER_MANUAL.pdf` | `abaff1754d0904fa1264c8da94d6eb10d1517a33a51becf1024ec736082f3452` |
| OW18-QG | `manuals/OW18_Series_Digital_Multimeter_QUICK_GUIDE.pdf` | `76e80395c8930b36ecad80733b9f382df77e39d20af8a84e3d4273cceb5b7737` |
| CM2100-UM | `manuals/CM2100-Clamp-Ammeter_User_Manual.pdf` | `dd8956ab28b7e8e333c1088188e1b3b4dc21f2d1577e1323820e19791f36d997` |

The hashes of the product pages and datasheets are in `SOURCE.txt`.

### Avoided during the vendor analysis

Not searched for or opened while §1-12 of the spec were written:

- the web beyond OWON's pages and files above
- code repositories of any kind for these meters, sigrok (libsigrok and the
  wiki), forums, blog posts and videos
- `docs/research/new-device-candidates.md` and
  `references/owon/analysis/findings/survey-2026-10-01.md`: they hold
  community claims about OWON meters, and were kept out of every reader's
  brief and out of the drafting of all three documents

The adjudicator — the main session, which settled the readers'
disagreements — had read the community claims in
`docs/research/new-device-candidates.md` (14-byte packets, FS9922 / CS7729CN
chips) before the readers reported. None of them is used. The 14-byte format
in spec §11 is the PC reader's own reading of commented-out code in OWON's
PC source.

### Cross-referenced

Not yet. As of **2026-10-01** the clean-room boundary stands: no community
source has been opened for this family. Spec §14 is the placeholder for the
comparison.

### Model-recall disclosure

The assistant that wrote these documents may have been trained on community
write-ups of these meters. Recall was never used as a source: every fact in
§1-12 of `reverse-engineered-protocol.md` cites a file and lines of OWON's
app or PC source, or a page of an OWON manual or product page, and a fact
without one is tagged [UNVERIFIED].

## Methodology

1. **Acquisition.** The APK and both PC packages saved by the user from
   files.owon.com.cn; the manuals, product pages and datasheets fetched by
   the assistant. Every file hashed (`SOURCE.txt`); both PC zips pass
   `unzip -t`.
2. **App extraction.** `libapp.so` unzipped from the APK and decompiled with
   blutter (github.com/worawit/blutter @ `4a60ac6`), which built its Dart
   3.9.2 runtime for the run; `strings` of `libapp.so` to confirm literal
   command strings. The listing's conventions — Smi constants printed
   doubled, list indices and map keys halved, unboxed fields as printed, the
   selectors identified for `length`, `sublist`, `padLeft` and `last` — are
   recorded in `findings/app.md`.
3. **PC extraction.** The source bundle jar unpacked; the binary jar's class
   list diffed against it and spot-checked with `javap` (series IDs, GATT
   handles, digit formats, version string `20210324.9999V1.4.5_13`).
4. **Three readers**, each given only its own sources and the tag rules of
   `.claude/rules/research-docs.md`, none shown another's report:
   - the app (`findings/app.md`);
   - the PC source (`findings/pc-source.md`), with the TI HCI guide for the
     dongle layer;
   - the manuals (`findings/manuals.md`), from renders.
5. **Adjudication** by the main session (`findings/adjudication.md`): the
   app and PC readings compared line by line, each disagreement settled
   against the cited code, not by majority; where the code cannot settle it,
   it stays [UNVERIFIED] for a capture.
6. **Grounding checks.** A fresh reader per report re-opened every cited
   line, page or render (`findings/grounding-app.md`,
   `grounding-pc.md`, `grounding-manuals.md`). Their fixes were applied in
   drafting, the grounding check winning wherever it contradicted a report:
   chiefly the app's UI error codes (−1/−2/−3, not the console's 0-3), the
   offline timeout's error value, the provenance of `*DATe` and `*RECOrd,`
   (built from code points, not literal strings), the `*RECOrd,` argument
   order's evidence, the PC's status-bit positions (inferred, not read), the
   PC's offline handling of series 55 (a signed value), the PC's key-5 label
   on B-series meters ("Duty"), the NCV mapping holding only for decimal
   code 0, the tags of OWON's example decodes, and the manuals' missed
   inconsistencies and cite slips.
7. **Drafting** from the reports, the adjudication and the grounding checks,
   re-opening the vendor source where a cite was loose: the PC's frame
   parser (`model/MultimeterClient.java:1405-1665`), its `*READlen?`
   handler and device-information reader (`kernal/BleClient.java:178-275`),
   the commented-out 14-byte parser (`:211`, `:264-330`), the app's r2w
   masks and model table, the B35 and B41T product pages' Bluetooth and rate
   lines, and the text of two manual pages (B35-UM p.40/35, B33-UM
   p.27/22-28/23).

Resolved disagreements (`findings/adjudication.md`, and drafting):

1. **The challenge**: the app sends it on every connect; the PC never does
   and still parses readings. Both stand: it authenticates the meter to the
   app. Whether newer firmware requires it stays open.
2. **CCCD**: the app subscribes, the PC writes no CCCD. A standard host
   subscribes; whether notifications flow unsubscribed stays open.
3. **Status bits 6-15**: only bit 7 (RMR) agrees; both tables are given,
   meanings open.
4. **Function 13**: NCV (app, every model) against "ADP" (PC, B-series). The
   manuals give NCV only to the OW16, OW18 and CM2100 and neither to the
   B-series; open for the B-series.
5. **Functions 14-15**: Power W / VA (app) against nothing (PC); no dial has
   them, the B35 LCD has unexplained RPM and bolt segments; open.
6. **Decimal code 5**: ÷10⁵ (app) against "err point" (PC); open.
7. **Model codes**: 21 (CM2100) only in the app, 55 only in the PC; the PC's
   `OW18_16` for 18 is the only link to the OW16B. No product name is paired
   with a code that no source pairs.
8. **Digits for code 20**: 5 (app) against 4 (PC); the app matches the
   OW18E's 19999 counts. Display only: the value decode does not depend on
   it.
9. **GATT handles**: fixed per layout in the PC, discovered by UUID in the
   app; recorded, irrelevant to a host that discovers.
10. **Service UUID**: FFF0 from the app only.
11. **Identity before connecting**: neither program filters by name; "BDM"
    comes from the manuals and a PC comment.
12. **The 14-byte "chip protocol"**: commented-out PC code only; open.
13. **Series 55's sign**: function-word bit 10 in the PC; recorded.
14. **The 15-byte frame**: app only, given its own section.
15. **`*READlen?` reply** (found in drafting): bytes 0-1 as a u16 (app)
    against bytes 0-3 as a u32 (PC); open.

### Source disagreements: manuals

Recorded, not adjudicated; the ones a meter can settle are in
`verification.md`.

- B41T(+) capacitance: a separate ⊣⊢ position in the terminal table, but
  "rotate the rotary switch to ∘)))⊣▶Ω" in the procedure (B35-UM p.17/12-18/13
  against p.21/16).
- OW16 µA: the dial table says "up to 600 microamperes" (OW16-UM p.13/8);
  the spec table has 600.0µA and 6000µA (p.49/44). The OW18 dial table says
  "up to 6000" (OW18-UM p.13/8).
- Continuity threshold: 30 Ω in the procedure, about 50 Ω in the buzzer
  section (OW16-UM and OW18-UM p.18/13 against p.22/17).
- B33 Bluetooth range: about 10 m (B33-UM p.22/17) against 7-8 m (p.37/32).
- B33 backlight: separate ☀ and Hold keys on the panel and keypad table
  (B33-UM p.12/7, p.13/8-14/9), a combined ☀/H icon in the backlight paragraph
  (p.11/6); and a Max/Min soft key in an app screenshot of a meter with no
  Max/Min (p.25/20).
- CM2100 offline data: saved "in the zip format" (CM2100-UM p.24/21) against
  CSV (p.16/13); the "Enable automatic shutdown" heading over text that
  cancels it (p.14/11-15/12).
- OW18 PC section: the quick guide lists Windows 10/8/7/Vista/XP and driver
  BLE-CC254x-1.4.1.43908 (OW18-QG p.15/13-16/14); the user manual Windows
  11/10/8/7 and 1.5.0.16 (OW18-UM p.32/27).
- Spec-table oddities, as printed: B41T capacitance ">220mF" against its
  footnote's "220 mF range"; B41T 22MΩ resolution "1.2 kΩ"; 35-series
  capacitance ranges in 4000-count steps on a 6000-count meter; CM2100
  2.0000A at 0.001 A; a "Function" column header over the accuracies in
  every OW16 and OW18 spec table and the OW18 datasheet.
- MIN/MAX segments on the OW18 and CM2100 LCD figures, which describe no
  MAX/MIN feature; RPM and a bolt on the B35 LCD (B35-UM p.15/10).

### Source disagreements: manual against product page or datasheet

Ranges and resolutions; recorded for the `/spec-data` pass, which waits for a
first hardware capture.

| Model | Manual | Product page / datasheet |
|---|---|---|
| OW16 DC V | 600.0mV/6.000V/60.00V/600.0V → 0.1 mV (OW16-UM p.49/44) | no 600.0mV row (page and datasheet p.1) |
| OW16 AC V | 600.0mV → 0.1 mV; 6.000V/60.00V/600.0V → 1 mV | no 600.0mV row; 6.000V-600.0V → 0.1 mV |
| OW16 µA | 600.0µA/6000µA | "600.0uA/6000μA (EU)" on both |
| OW18A/B DC V, V position | 600.0mV-600.0V → 0.1 mV (OW18-UM p.47/42) | page: 6.000V-600.0V → 1 mV, no 600.0mV; datasheet p.3 as the manual |
| OW18A/B AC V, mV position | 600.0mV → 0.01 mV | page omits the row; datasheet as the manual |
| OW18D/E frequency | 200.00Hz-20.000MHz, 6 ranges (p.46/41) | page adds 20.000Hz; datasheet p.2 as the manual |
| OW18D/E AC V top | 750.0V | page 750.00V; datasheet 750.0V |
| OW18D/E counts | 19999 (p.48/43) | page "20000counts"; datasheet 19999 |
| B33 frequency | …/49.99kHz/499.9kHz/4.999MHz (B33-UM p.49/44) | page and datasheet "…/49.9kHz/…" |
| B33 temperature | resolution 1 ℃ | none given |
| B33 frequency response | no T: 40-400 Hz; T: 40-1000 Hz (p.50/45) | 40-400 Hz only; models D33/B33/B33+, no T |
| B41T 22MΩ | 1.2 kΩ (B35-UM p.53/48) | page 1kΩ; datasheet 1.2 kΩ |
| B41T duty (≥1 kHz) | 5.0%-94.9% | page 0.1%-99.9%; datasheet as the manual |
| B41T capacitance | ">220mF [3]" → Undefined | page ">220mF / /" |
| B41T frequency top | ≤220MHz | page ≤220.00MHz |
| B41T rates | "Sample rate for digital data" 2/s (p.54/49) | page "shift rate(BLE link) 2 times/s", "Update rate 3 times/s" |
| CM2100 A | 2.0000A → 0.001 A; 20.000A; 100.00A (CM2100-UM p.32/29) | page 2.000A, 20.00A, 100.0A; datasheet as the manual |
| CM2100 Ω | 200.00Ω | page 200.0Ω; datasheet as the manual |
| CM2100 frequency | includes 2.0000kHz (p.33/30) | page omits it; datasheet as the manual |
| CM2100 counts | 19999 (p.33/30) | page "20000 conuts"; datasheet 19999 |
| B35 (35 series) | — | page and datasheet p.1 match the manual |

The pages also say what the manuals do not: the B35 page lists a "Bluetooth
2.0 version" beside the 4.0 one (spec §11); the B35, B41T and B33 pages give
"Recording Duration 168 hours (7 days)" and "Record Length 10,000 points";
the B41T page lists "Auto Test" and a diode row the manual lacks; the B33 and
CM2100 pages offer no PC software, although the B33 manual has a PC chapter.

## Tag legend

As used in `reverse-engineered-protocol.md`:

- **[KNOWN]** — stated in an OWON manual or on OWON's site, cited by page
- **[VENDOR, app]**, **[VENDOR, PC]**, **[VENDOR, both]** — read from OWON's
  app, OWON's PC source, or both, cited by file and lines
- **[INFERRED]** — deduction from the above, reason given
- **[UNVERIFIED]** — no source confirms it, or the sources disagree; needs a
  real meter
- **[HARDWARE]** — seen on a real meter: none yet
- **[COMMUNITY]** — from a community source, spec §14 only: none yet
