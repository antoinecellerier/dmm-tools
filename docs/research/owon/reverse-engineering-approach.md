# OWON Bluetooth multimeters: Reverse Engineering Approach

Scope: OWON's Bluetooth LE multimeters — the B33(T)(+), B35(T)(+),
B41T(+), OW16B, OW18B, OW18E and CM2100B — from the advertisement to the
frames they send and the commands they accept. Both frames are
implemented in `protocol/owon/`, experimental; this
pair of documents records what the meters do on the wire, from vendor
sources only; community projects and their captures are compared in spec
§14 alone. The spec covers both frame formats OWON's app decodes: the
6-byte frame of these meters in full, and the 15-byte frame of the OWON
CMS101/061 and OW65B/67B/69B and the Voltcraft VC871/891/915/925 PV in its
own section (§10), with those meters' manuals in §9.4. The 15-byte round
(2026-10-02) added Voltcraft's app, the manuals of those meters, and the
VC831/VC851 manuals, which show no Bluetooth.

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

All OWON's own, and for the 15-byte round Voltcraft's (Conrad's). Fetched
and analysed 2026-10-01, the 15-byte round 2026-10-02. Provenance — URL, date,
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

**The 15-byte round** (2026-10-02; the user approved each source):

6. **Voltcraft "VC800 VC900 Series" 1.2.5** (`com.voltcraft.series800`,
   versionCode 2050), Voltcraft's Android app, downloaded by the assistant
   as an XAPK from APKPure (`d.apkpure.com/b/XAPK/com.voltcraft.series800`)
   and accepted by the user as the vendor's: signed with a Google Play App
   Signing key (SHA-256 `140763bd…32a4`), Play source stamp verified, Play
   developer "CEI Conrad Electronic International (HK) Limited" per a
   mirror's listing. `libapp.so` comes from the `config.arm64_v8a.apk`
   split. Tagged [VENDOR, app], cited `V:file:lines`.
7. **OWON's CMS101, CMS061, OW65, OW67 and OW69 manuals** and quick guides:
   the OW6x user manuals downloaded by the user, the rest by the assistant,
   from `https://files.owon.com.cn/probook/<name>`. The file served as
   `OW65_multimeter_quick_guide.pdf` is the OWH65 power supply's guide and
   was not used.
8. **Conrad's Voltcraft manuals** for the VC831, VC851, VC871, VC891, VC915
   and VC925 PV, datasheets for the first four, and the VC871/VC891 app
   manual, downloaded by
   the assistant from `https://asset.conrad.com/media10/add/160267/c1/-/GL/<id>/<id>-GL.pdf`.
   The later app manual bundled in the app (VAPP) was read from the APK.

| Source | File (under `references/owon/`) | SHA-256 |
|---|---|---|
| Voltcraft XAPK | `voltcraft-app/com.voltcraft.series800_1.2.5_2050.xapk` | `341508ca10b30541b31d8a841347f75bd75251163fa18ca3364a3b9572ebf0cb` |
| Voltcraft native split | `voltcraft-app/splits/config.arm64_v8a.apk` | `d2b21615ebb4e3c6b05417b3f2dad3bf5f74d984fd21875ab550e06034d2c6d3` |
| CMS101-UM | `manuals/CMS_User_Manual.pdf` | `2f9700148864f720115fb54fc474f147c2f438276a7d7b7fac76e2a4c7f5a35e` |
| CMS061-UM | `manuals/CMS061_User_Manual.pdf` | `2f392dcd84693b4f3f5de3af0024c9449ce4b7604400926b9d6cb365e149d61e` |
| OW65-UM | `manuals/OW65_multimeter_user_manual.pdf` | `924fa76f2609bc1ca8a5119587539b995041729aef56ae89f9a7f80060885432` |
| OW67-UM | `manuals/OW67_multimeter_user_manual.pdf` | `b1194f42df58c9b059eb9a2c2e953416cd3f1a893b230f51061377dbcbc39ada` |
| OW69-UM | `manuals/OW69_multimeter_user_manual.pdf` | `62eea75c71ab5aad697029778ffd72563635ac6958aa89f0f059a872104fdbb1` |
| VC871-UM | `voltcraft/002576867ML00.pdf` | `5f9198e01d6020f2d2221431b2567687139ff77cab35c18eed3aefdca8ba29a8` |
| VC891-UM | `voltcraft/002576866ML00.pdf` | `c51e24adee940e1853e27c7bd9eb2b247a6e2022d990ac4bb82a4b00e1f5a4a5` |
| VC915-UM | `voltcraft/003072347ML00.pdf` | `7f5ac4d2b074d2e730b12d789be7b9eb60059a90a062390a0eded0b222ac1d8c` |
| VC925-UM | `voltcraft/003072348ML00.pdf` | `a43a272791e5e3f37e447f91f4cd16e65eed0d73f9f14bdbd2279611d25e9f82` |
| VC-APP | `voltcraft/002576866ML04.pdf` | `fd3b16f75d323936e9d54ec65f261dcec2858feb27493341b0e46082f9d186b8` |
| VC831-UM | `voltcraft/002576864ML00.pdf` | `5c8c75acb6f98882b37f8a81ebc8b006d6bfc8423eecbc7c9e7d326c338367bc` |
| VC851-UM | `voltcraft/002576865ML00.pdf` | `e91045360f14167614e3d0dc128e73f528cde85941508825f841f714ea779de6` |

The quick guides, the datasheets and the other Conrad files are in
`SOURCE.txt`.

### Avoided during the vendor analysis

Not searched for or opened while §1-12 of the spec were written:

- the web beyond OWON's pages and files above
- code repositories of any kind for these meters, sigrok (libsigrok and the
  wiki), forums, blog posts and videos
- `docs/research/new-device-candidates.md` and
  `references/owon/analysis/findings/survey-2026-10-01.md`: they hold
  community claims about OWON meters, and were kept out of every reader's
  brief and out of the drafting of all three documents
- in the 15-byte round, also spec §14 (community VC871 captures and GATT
  notes) and `findings/survey-15byte-2026-10-02.md`, kept out of every
  reader's and checker's brief

The adjudicator — the main session, which settled the readers'
disagreements — had read the community claims in
`docs/research/new-device-candidates.md` (14-byte packets, FS9922 / CS7729CN
chips) before the readers reported. None of them is used. The 14-byte format
in spec §11 is the PC reader's own reading of commented-out code in OWON's
PC source. In the 15-byte round the adjudicator had likewise read spec §14
and the 15-byte survey; the new text of spec §1, §2, §9.4 and §10 cites only
the vendor sources above.

### Cross-referenced (clean-room boundary opened 2026-10-01)

The user opened the boundary on **2026-10-01**, after the vendor-only spec
was committed (`41ac399d`). The findings are §14 of the spec, marked
[COMMUNITY]; nothing was merged into §1-12, which only point to it.
Repositories were cloned into a scratch directory and deleted afterwards;
no community code was copied. Real Bluetooth addresses in those sources are
left out.

Read on 2026-10-01, at the commit given:

1. [DeanCording/owonb35](https://github.com/DeanCording/owonb35) `dbbc4e1`:
   README and `owonb35.c`; and [inflex/owon-b35](https://github.com/inflex/owon-b35)
   `73c6baf` (README, `owoncli.c`) with its
   [issue #1](https://github.com/inflex/owon-b35/issues/1) and that issue's
   attachment `packets.txt` (a Wireshark listing of OWON's 2018 Android app).
2. [sercona/Owon-Multimeters](https://github.com/sercona/Owon-Multimeters)
   `1718fda`: README, `code/`, `esp32/`, `test_txt/`;
   [sercona/owon-cm2100b-clamp-meter](https://github.com/sercona/owon-cm2100b-clamp-meter)
   `bc683b2`: README only (the repository moved; its deleted files were not
   used).
3. [JayTee42/ow18b](https://github.com/JayTee42/ow18b) `ce4e131` and its fork
   [kwasmich/ow18e](https://github.com/kwasmich/ow18e) `6dfe32e`
   (`ow18e.txt` included).
4. [rbelnienk/OWON-OW18B-BLE-Connector](https://github.com/rbelnienk/OWON-OW18B-BLE-Connector)
   `2de51d2`, [MartMet/OW18B](https://github.com/MartMet/OW18B) `c1c52e8`,
   [JAQUBA/OWON_OW18B](https://github.com/JAQUBA/OWON_OW18B) `c3277c7`.
5. [reaper7/M5Stack_BLE_client_Owon_B35T](https://github.com/reaper7/M5Stack_BLE_client_Owon_B35T)
   `540e576`, [cransom/b35t-reader](https://github.com/cransom/b35t-reader)
   `2912481`, [akemnade/owon-tools](https://github.com/akemnade/owon-tools)
   `2787c4c`, [ondras12345/B35T](https://github.com/ondras12345/B35T)
   `8842d9b` (README and test fixtures),
   [53845714nF/OWON_B35T](https://github.com/53845714nF/OWON_B35T) `cbf1051`.
6. [jtcash/OwonB41T](https://github.com/jtcash/OwonB41T) `9d880c2` and its
   fork art-ya/OwonB41T `1f23cba`;
   [likeablob/owon-bdm-webui](https://github.com/likeablob/owon-bdm-webui)
   `00a61b8` with its history; [PBrunot/owonb41t](https://github.com/PBrunot/owonb41t)
   `db9dbbe`; [palmerr23/Owon_B41T](https://github.com/palmerr23/Owon_B41T)
   `dec8a5b`; [pjpa365/owon-suite](https://github.com/pjpa365/owon-suite)
   `693b534` (`docs/protocol-spec.md`, `backend/app/owon_ble/`,
   `poc/tests/test_protocol.py`);
   [luissantos/multimeter_gui](https://github.com/luissantos/multimeter_gui)
   `44d6c4b`; [VYD3N/Mult-AI-Meter](https://github.com/VYD3N/Mult-AI-Meter)
   `89620da`.
7. [webspiderteam/Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows)
   `2b83d9e`, OWON parts only: `Decoders/DecoderOwon.cs`, the OWON test data
   in `Utilities.cs` and its history (`be440f0`, `fb9ff02`, `b65294e`), and
   discussions #40, #49 and #66 with their comments; from #66 the attachment
   `VC871 BLE GATT.txt`.
8. [libreble/multimeter](https://github.com/libreble/multimeter) `d26ba48`:
   `docs/protocols/owon-plus.md`, `owon-old.md`.
9. sigrok: [libsigrok](https://github.com/sigrokproject/libsigrok) `0bc2487`
   (no OWON Bluetooth support; OWON appears only in `scpi-dmm`), the sigrok
   wiki's search (no hits), and the sigrok-devel post
   ["Add support for Owon B35T"](https://sourceforge.net/p/sigrok/mailman/message/35691836/)
   (2017-02-27).
10. EEVblog: search-result snippets only, no forum page fetched; they added
    nothing beyond the projects above.

Read on 2026-10-02 for the 15-byte meters, after their vendor text was
committed (`ca23ba9d`), the user having opened the boundary for them that
day:

11. webspiderteam discussion #66 again, with its attachments `uuids.txt`,
    RefuCire's 2025-09-24 log and FireBird3314's `TestLogs.zip` (its
    `Debug.zip` attachments are executables and were not opened); the
    repository unchanged at `2b83d9e`.
12. [libreble/multimeter](https://github.com/libreble/multimeter) `d26ba48`:
    `docs/protocols/voltcraft.md`, `drivers/voltcraft.ts`.
13. [ble-multimeter/fakemeter](https://github.com/ble-multimeter/fakemeter)
    `4cca1d5`: `docs/PROGRESS.md`, `docs/owon-voltcraft-handshake.md`,
    `docs/voltcraft-measurement-protocol.md` (one author's adapter address
    in `PROGRESS.md` left out).
14. GitHub repository and code search, and web search (EEVblog and Reddit as
    result snippets only), for each model, `com.voltcraft.series800`,
    `#TIMEsync`, `imeter_base` and `owon_imeter`: nothing else with data on
    these meters, 2026-10-02.

Opened, not about this family: [bialybudyn/Owon-Multimeter-Manager](https://github.com/bialybudyn/Owon-Multimeter-Manager)
`65da106` (the XDM2041 bench meter over SCPI).

Unreachable or not opened: the hackaday.io project "Bluetooth Data Owon
B35T Multimeter" (12922; 502 to the fetch tool, no connection with curl,
2026-10-01); the other issue and discussion attachments (logs, `Debug.zip`,
`TestLogs.zip`, `uuids.txt`); granzscientific/b35t-reader (a fork of
cransom's, not opened); videos; Reddit; mikrocontroller.net.

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
8. **Community cross-reference** (boundary opened 2026-10-01, above). Each
   claim of spec §1-12 compared with the community sources, noting for each
   whether it rests on a meter capture or on OWON's code, and every captured
   frame decoded with the spec's tables (spec §14.5).
9. **Vendor re-check.** Each disputed point re-read in OWON's app listing or
   PC source alone: the bit-6 OL name (MC:1594-1598), the `*READ?`/`*READ1?`
   header's bytes 12-15 (stored as `OfflineRecordConfig.recordLen`, unused by
   the parse, R2W:687-1017), the OW key for Hz/Duty (MD:5989;
   `frame/MainFrame.java:330-348`), decimal codes 6 and 7, and the app's
   FFF0 scan filter. The spec's reading held in every case, so no body
   statement changed; each disagreement in spec §14.3 sets a capture against
   OWON's code, or is a community error.
10. **The 15-byte round** (2026-10-02). Voltcraft's `libapp.so` decompiled
    with the same blutter build (Dart 3.9.2). Three readers, none shown
    another's report: the manuals (`findings/manuals-15byte.md`, from
    renders), iMeter's 15-byte code (`findings/app-15byte-gaps.md`), and
    Voltcraft's app against iMeter (`findings/voltcraft-app.md`, a diff with
    addresses, branch targets and pool offsets masked and each pool object
    replaced by a hash of its contents). Adjudication
    (`findings/adjudication-15byte.md`), four grounding checks
    (`findings/grounding-*.md`; chiefly cite lines, the order of the app-side
    keys, the speech text for LPF, the tags on Dart-runtime readings, and
    the manuals' screenshot key lists), and a narrow re-check of the fixed
    rows.

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
16. **Voltcraft's advertised name**: "VC871", "VC891" and "VCxxx" in the
    meter manuals, "VC8xx_1" in the 2022 app manual, "BDM" in the app manual
    bundled with app 1.2.5 (on a VC925, by its key list). All recorded; no
    app code tests a name; open.
17. **Key lists against the manuals' app screenshots**: the app's list per
    model code is what the app sends; the OWON manuals' screenshots show
    Hz/Duty on an OW65 and OW67, whose lists lack it, and their caption says
    the keys match the meter's, which has no Hz/Duty or light key. Taken as
    generic screens; open for `05 01`.
18. **Hold/Light long press `09 01` on the CMS**, whose manual gives HOLD's
    long press as "DCA to zero" and no light key; the bytes recorded, the
    effect open.
19. **Spec corrections from the app**: Compare (`0F`) is on the VC925's list
    too, and the VC915/925 Hold long press is `03 00`, not `09 01`; code
    223's full-scale lines are 1095-1101.

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
- OW65: "<" and ">" keys in the setup and 4-20 mA procedures that its panel
  lacks (OW65-UM p.17/12, p.25/20); the torch key opens a menu (p.11/6) or
  toggles the light (p.13/8). OW67: p.11/6 still shows the OW65's torch key.
- "750 Vac" in the voltage warnings against 1000 V AC on the jacks and in the
  specifications (OW65-UM p.19/14 against p.15/10; OW67-UM p.20/15; OW69-UM
  p.19/14).
- VC871 frequency: "10 Hz to 10 MHz" in the procedure (VC871-UM p.78)
  against 60.000 MHz in the specification (p.106) and 60 MHz in the
  intended use (p.59).
- VC915 20 A: 10 s "in 10-minute intervals" (VC915-UM p.77) against
  15-minute (p.101-102); RECORD "sent to the measurement app" (p.68) against
  "onto the device" (p.89).
- VC925 rating: CAT III 1500 V, CAT IV 1000 V in the technical data
  (VC925-UM p.100) against CAT III 2000 V for the leads (p.66, p.101) and
  on the jack print.
- Phone requirements: Android 4.3 and iOS 7.0 (VC871-UM p.69) against Android
  6.0 and iOS 11.0 (VC-APP p.4; VC891-UM p.63). The app's "+" is "in
  the top left corner" (VC-APP p.11), a "large plus sign in the middle" in
  the meter manuals (VC871-UM p.92).

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
- **[COMMUNITY]** — from a community source, spec §14 only
