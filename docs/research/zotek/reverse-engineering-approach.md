# ZOTEK Bluetooth meters: Reverse Engineering Approach

Scope: the Bluetooth LE multimeters made by Shenzhen ZOTEK Instruments
(深圳市众仪电测科技有限公司) and sold as ZOYI, ZOTEK, BSIDE and ANENG — the
ZT-300AB, ZT-5B, ZT-5BQ, ZT-5566 family and ZT-6S, and ANENG's AN9002, V05B,
ST207 and AN999S. The ZT-300AB's, ZT-5566SE's and ZT-5BQ's layouts are implemented,
experimentally; this
pair of documents records what they do on the wire, which the implementation
builds on.

The goal was one question first: do the three vendor apps speak one protocol
or several? They speak one — one descramble key, one header, one command
frame — with a type byte that picks one of **four packet layouts**
(`reverse-engineered-protocol.md` §1). The spec covers all four, since they
share every parser file. **Type 3** is the priority: its layout is named for
the ZT-300AB, which with its ANENG AN9002 rebrand ranked first in our
discovery pass (listing ratings, the number of resellers, forum threads).

## Sources Used

All ZOTEK's own, or ZOTEK's distributors'. Fetched 2026-09-25, analysed
2026-09-25. Provenance — URL, date, SHA-256, signer — is in
`references/zotek/{e-bull-v2,e-bull-v1,bluetooth-dmm,manuals}/SOURCE.txt`
(gitignored); the findings reports are in
`references/zotek/analysis/findings/`.

### Primary (clean-room RE)

1. **e-Bull V2 1.1.2** (电牛V2), package `com.zoyi.bleapp`, versionCode 112 —
   ZOTEK's current app, listed on its download page for the ZT-300AB, ZT-5B,
   ZT-5BQ, ZT-5566 and ZT-5566SE. From ZOTEK's Google Drive, linked on
   https://zotektools.com/?support/ (data:
   https://zotektools.com/support_api.php): `e-Bull V2_V1.1.2.apk`
   (https://drive.google.com/file/d/1O8AyuKv6mwLUidQS-u_SYfjh8Zji6eu1/view,
   listed for the ZT-5566) and `ZOYI e-Bull V2_V1.1.2.apk.zip`
   (https://drive.google.com/file/d/1BtqT4fu2wMKZMK2DiSO-qFUgkR42ciV6/view,
   listed for the ZT-300AB, ZT-5B, ZT-5BQ and ZT-5566SE), which holds the
   byte-identical APK. A **uni-app** (DCloud) build: the app's own package
   (`com.zoyi.bleapp`) holds only BuildConfig, R and CustomTrustMgr, GATT goes
   through DCloud's generic Bluetooth plugin
   (`io.dcloud.feature.bluetooth.BluetoothBaseAdapter` in `classes2.dex`,
   decompiled with jadx only to check write type and CCCD), and all protocol
   logic is plain JavaScript in `assets/apps/__UNI__59084C7/www/app-service.js` (2,955,234
   bytes, ASCII). Tagged [VENDOR] with `V2@<byte offset>` into that file.
2. **e-Bull V1.0.12** (电牛), package `com.yscoco.multimeter`, versionCode 12 —
   native Android. From the ZOYI UK distributor's downloads page
   (https://zoyi.co.uk/content/4-manuals-downloads →
   https://www.dropbox.com/scl/fi/sw5jsn21sr08zs5syeoxj/e-Bull-V1.0.12.rar?rlkey=mgb8t564gopchuh2wf7aadrbj&dl=0),
   `e-Bull-V1.0.12.rar` holding an APK dated 2023-11-29. Not on Google Play;
   ZOTEK's own site hosts only V2. Tagged [VENDOR] with `file:line` in the jadx
   tree.
3. **Bluetooth DMM 1.0.13**, package `com.yscoco.wyboem`, versionCode 13 — the
   white-label build of V1 that the ANENG AN9002 manual points to. From the ZOYI
   Taiwan distributor (https://zoyi-tw.com/BLUETOOTH-METER-APP →
   https://u.pcloud.link/publink/show?code=XZhh0JJZQ8i3hRhNVkFb35hHaRKDFzkJHBAk).
   Its Play listing is gone (404) and the manual's link
   http://multimeter.szoyi.com:8081/share/OEM.html is dead; no mirror was used.
   After renaming the package its protocol classes are byte-identical to V1's;
   only UI, chat, logging and network code differ. Tagged [VENDOR] with V1's
   `file:line`.

| App | File | SHA-256 | Signer | Signer cert SHA-256 |
|---|---|---|---|---|
| e-Bull V2 1.1.2 | `e-Bull V2_V1.1.2.apk` | `5ed42f140dca4921962b9140ad3789dc99cfd4dd2735e11077ffce7a86cf34d3` | `CN=dekvY8Bx…, OU=Android, O=Android, C=CN` (DCloud cloud-build style) | `d7d13f4d39c46f059cac7991f4e2556b622623279183bda25593393857b0afc9` |
| e-Bull V1.0.12 | `e-Bull-V1.0.12.apk` | `afed9ae5f94b35cc8186a031eab44e87a84ae4bc819cb3dff7f233ec0e9d3978` | `CN=mark, OU=yuanshang` | `11e870b3ab2d86ec1f2165437ff4d6c03158486939997f43c6bd102aad6a0e02` |
| Bluetooth DMM 1.0.13 | `Bluetooth_DMM_1.0.13.apk` | `5ac27dabaec68f113d30147acff05b6c6cf028592d9207c8d4f0b2a1b02e14e6` | `CN=mark, OU=yuanshang` | same as e-Bull V1 |

4. **ZOTEK user manuals**, from ZOTEK's Google Drive (linked on
   https://zotektools.com/?support/, file `https://drive.google.com/file/d/<id>/view`).
   Read from the rendered pages; text extraction only to locate passages.
   Tagged [KNOWN] with file and page. None prints its own model number (the
   app sections in the ZT-300AB and ZT-5566SE manuals list the ZT-300AB,
   ZT-5BQ and ZT-5B); the file names are the link.

| File | Pages | Drive id | SHA-256 |
|---|---|---|---|
| `ZT-300AB-EN.pdf` | 32 | `1pJe2OvAj9fxoLVoOJNjBVcD-Xo8pc4Qr` | `c4f7761dd4af63300e3f82a0bc7388cc84c9ec39df06238b30b0ea86eced5d6e` |
| `ZT-5B.pdf` | 2 | `1P8qGNLYWRS7Ki9ApuXdG5RKBtUWy-Fpc` | `655221d43b9da67c81535a25cf552a1e3da81c6b36bf35f2f52e7f793e39d522` |
| `ZT-5BQ.pdf` | 3 | `1IzUEkgD5i_HTxR27fYyBF1GpC0pMH1YS` | `b28bbfb6ab87c638f2cdc6b248399b4c33d95941604b06e5c76fdc0a87c83ab2` |
| `ZT-6S.pdf` | 2 | `1w4Oss9PLnGGFUYt1Hcy8ApkyOb7zbxwx` | `9d17815ce24c1c47abff13f98c45104d27f803411c3eb0147e8a6a4a8b6ee9c5` |
| `ZT-5566.pdf` | 29 | `1XKDgJ0Kx1f-zTQS2_PrMLxVcMlXCPjNY` | `af2be800fe505749286f036b9aacffa2c005dc93d06d7aab36bf14e64e9b6bbc` |
| `ZT-5566SE.pdf` | 36 | `1laFQxTLV4RnTR2qTB45ySc1J9WvsrKn4` | `4cf8d0edb69f85a7fbb6d755261f796163dec5b3e6613d467a698cc060344dc4` |
| `ZT-5566S.pdf` | 36 | `1Ijp-K4ey770sHOo7BEYeNLb337BWfdin` | same file as `ZT-5566SE.pdf` |

5. **Product and distributor pages**, for the model list, the rebrand
   pairings and popularity only — no protocol content: zotektools.com product
   and support pages, szzotek.com (h-nd-39, h-col-159), zoyi-tw.com,
   zoyi.co.uk, bsidemeter.com, iTunes lookups of the iOS apps, manuals.plus
   copies of the ANENG AN9002 and V05B manuals (not archived), and reseller
   listings (Amazon .com/.in/.de, Banggood, electroslab, electric-b2c,
   mickcara), rickmakes.com's ZT-5566SE review, voltlog.com's AN888S review
   summary, and Google Play pages (the 404 checks).
6. **The QR code** printed in the ZT-300AB, ZT-5BQ and ZT-5566S/SE manuals and
   the one on szzotek.com, decoded with zxing-cpp: the manuals' code gives the
   dead `OEM.html` above; its directory, http://multimeter.szoyi.com:8081/share/,
   and the szzotek.com code now lead to e-Bull V2 (iOS id6755057944, Android
   https://fir.xcxwo.com/yt7h8e). Which build fir serves was not retrieved.

### Avoided during the vendor analysis

These turned up in searches and were **not opened** while §1-10 of the spec
were written:

- github.com/ludwich66/Bluetooth-DMM and its wiki
- github.com/webspiderteam/Bluetooth-DMM-For-Windows and its YouTube video
- github.com/libreble/multimeter
- the GitHub topics zt-300ab, zt-5b, zt-5bq and v05b
- justanotherelectronicsblog.com/?p=930
- ts-software-jp.net's TSDMMView page (title only)
- wiki.seeedstudio.com/Bluetooth_Multimeter
- blog.jj5.net's AN-999S post; the jj5.net AN-999S wiki page was fetched with
  a stop-on-protocol instruction, access was denied and nothing was read
- the BudgetLightForum V05B thread: a summariser flagged protocol content and
  only the reviewer's opinion and date were taken
- EEVBlog threads: titles only (the site returned 403)
- APK mirrors (Aptoide, APKCombo, APKPure, APKSum): listing pages only, no
  APK taken from one

webspiderteam and libreble had been opened on 2026-09-22 and 2026-09-25 for
the UNI-T adapter and meters, limited to the files listed in
`../ut-d07b/reverse-engineering-approach.md` and
`../ut61-family/reverse-engineering-approach.md`; nothing from them was used
for §1-10. The earlier lines on these meters in `../new-device-candidates.md`
(packet lengths, the `fff4` UUID) came from the April 2026 survey and were
replaced with what the vendor apps show.

### Cross-referenced (clean-room boundary opened 2026-09-25)

The user opened the boundary on **2026-09-25**, after the vendor-only spec
was committed and grounding-checked. The findings are §11 of the spec,
marked [COMMUNITY]; nothing was merged into §1-10. Working notes:
`findings/community-crossref.md` (the comparison) and
`findings/vendor-recheck.md` (methodology step 8). Repositories were
cloned into a scratch directory and deleted afterwards; no community code
was copied.

Read on 2026-09-25:

1. [ludwich66/Bluetooth-DMM](https://github.com/ludwich66/Bluetooth-DMM) and
   its wiki: Home, Bluetooth---Analyses, Protocol-all-Variants, the 10- and
   11-byte pages, Technical-Data-Multimeter-(4), BT-Module-F9788Scematic,
   File-Export.
2. [webspiderteam/Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows),
   `2b83d9e` (2026-05-01) with its history: README, LICENSE,
   `Binary raw data.md`, `Decoders/DecoderBluetoothDMM.cs`, `GattMonitor.cs`,
   the test data in `Utilities.cs`, `HeartRateMonitor.cs` at `9cc2307^`; the
   wiki page Remote-Features; issues #2, #3, #29, #33, #36, #45, #58 and #69
   and discussions #30, #35, #41 and #65 with their comments.
3. [libreble/multimeter](https://github.com/libreble/multimeter) `d26ba48`:
   `docs/protocols/bdm.md`, `docs/HARDWARE.md`,
   `packages/protocol/src/drivers/bdm.ts`.
4. riktw: https://justanotherelectronicsblog.com/?p=930 and
   [riktw/AN9002_info](https://github.com/riktw/AN9002_info).
5. [olegv142/ut61xpy](https://github.com/olegv142/ut61xpy), `adapters/aneng.py`
   and README.
6. [meijerwynand/bt-multimeter-cli](https://github.com/meijerwynand/bt-multimeter-cli),
   [hoeulm/ble_aneng](https://github.com/hoeulm/ble_aneng),
   [bendtherules/multimeter-connect-web](https://github.com/bendtherules/multimeter-connect-web),
   [Shiro-Nek0/Bluetooth-DMM.py](https://github.com/Shiro-Nek0/Bluetooth-DMM.py),
   [840922704/BLE_DMM_Client](https://github.com/840922704/BLE_DMM_Client),
   [blackPantherOS/AN9002](https://github.com/blackPantherOS/AN9002) and
   [anszom's gist](https://gist.github.com/anszom/732b5b7dda9ccb624980153dff1d7c1f).
7. Searched with nothing found: [libsigrok](https://github.com/sigrokproject/libsigrok)
   `0bc2487` and the sigrok wiki's API search; the GitHub topics zt-300ab,
   zt-5b, zt-5bq, v05b, an9002, zt-5566, an999s, st207, zoyi, aneng and
   bluetooth-dmm (which found the repositories above).
8. Opened in full this time, no protocol content: wiki.seeedstudio.com's
   Bluetooth multimeter page, blog.jj5.net's AN-999S post (the jj5.net wiki
   again denied access), the BudgetLightForum V05B thread (pages 1 and 2) and
   ts-software-jp.net's TSDMMView pages.

Still not opened: BLE_DMM_Client's `Reference/*.zip` (a vendor APK and its
decompilation; we have our own copies from ZOTEK), issue attachments
(`log.txt`, `btsnoop_hci.log`, `.ods`), YouTube videos, EEVblog,
lab.fawno.com, m5.8266.de, mysku, blackPantherOS/Aneng-Bluetooth-DMM, the
libreble demo, and APK mirrors.

### Model-recall disclosure

The assistant that wrote these documents was likely trained on community
write-ups of this protocol (the projects above among them). Recall was never
used as a source: every fact in §1-10 of `reverse-engineered-protocol.md`
cites a vendor app location (`file:line` or `V2@offset`) or a manual page,
and a fact without one is tagged [UNVERIFIED].

## Methodology

1. **Discovery.** Which brands, models and apps exist, which manuals name
   which app, and how popular each model is — from the product and listing
   pages above, with protocol sources left unopened
   (`findings/discovery.md`).
2. **Acquisition.** The three APKs and seven manual files (six distinct;
   ZT-5566S = ZT-5566SE), from ZOTEK or a named
   ZOTEK distributor only, hashed on arrival. The two V1-line APKs were
   decompiled with jadx 1.5.6:

   ```sh
   ~/stuff/jadx/bin/jadx -q -d <dir>/jadx-out <apk>   # exit 3 = per-class warnings
   ```

   V2 needed no decompiler: the APK was unzipped and `app-service.js` read
   with `grep -bo` and byte slices. Afterwards `pretty.js` (the acorn
   tokenizer) wrote a readable copy whose lines keep their original offsets,
   `app-service.pretty.js`, and `v2loc.py` resolves a `V2@N` citation to it
   (both in `references/zotek/e-bull-v2/`). A second blind decode of V2 from
   that copy found nothing new about the wire; it added the `Buffer.from`
   masking and the blank-digit note (`findings/pretty-vs-recorded.md`).
3. **Grouping.** One pass over all three apps for scan, GATT, writes, parser
   entry, descramble and model dispatch (`findings/protocol-groups.md`), with
   V1 and Bluetooth DMM `diff -rq`'d after a package rename. Result: one
   protocol, four layouts, no model chosen by name.
4. **Two blind decodes from different implementations.** Decode A read only
   the V1/Bluetooth DMM Java (`findings/decode-java.md`) and checked its worked
   examples with a scratch Python port of the vendor methods. Decode B read
   only the V2 JavaScript (`findings/decode-js.md`), extracted the vendor
   modules `3eb8`, `409a`, `437c`, `62b4` and `d3ca` verbatim into a
   webpack-style require shim and **ran them in node** on every worked example
   and edge case. Neither saw the other's report.
5. **Adjudication.** The two decodes compared line by line, every
   disagreement settled against the vendor code and the manuals
   (`findings/adjudication.md`, authoritative where A and B differ).
6. **Manual cross-check.** Two further passes read the manuals from rendered
   pages: the ZT-300AB, ZT-5B, ZT-5BQ and ZT-6S
   (`findings/manuals-small.md`) and the ZT-5566 family
   (`findings/manuals-5566.md`). Each type's flag set was compared with the
   LCD legend of the model its identifier names.
7. **Community cross-reference** (boundary opened 2026-09-25, above). Each
   claim of spec §1-10 compared with the community sources, noting for each
   source whether it rests on hardware captures or on the vendor app
   (`findings/community-crossref.md`).
8. **Vendor re-check.** Each disputed point went back to the vendor code as
   a neutral question, without the community claim, answered from V1, BD and
   V2 alone, with baksmali (from the jadx jar) wherever jadx output was
   garbled (`findings/vendor-recheck.md`). The spec's reading of the apps
   held in every case; the one misreading it found, V1's dash tests as jadx
   renders them, is already corrected in the committed spec. So each
   disagreement in spec §11.3 sets a meter capture against the vendor code
   or the spec's own example, or is a community error.

Resolved disagreements (`findings/adjudication.md`):

1. Bits V1 never reads: V2's names adopted; where they drive no UI, the
   meaning is [INFERRED] from the name.
2. Type-3 prefixes: V2's three groups (capacitance, mV, current), which match
   the ZT-300AB legend; V1 merges them.
3. Two DP bits in one packet: V1 takes the leftmost, V2 the rightmost; one DP
   expected, two [UNVERIFIED].
4. Special displays: each app's rule recorded separately (V2 scans from the
   right, V1 checks fixed positions; they disagree on the EF and `----`
   examples); the on-LCD `0L` from the manuals; which positions the meters
   use [UNVERIFIED].
5. Type-4 byte 13 bit 4: a colon, [INFERRED] from V1 and the ZT-5566's
   `1.8.8:8.8` readout (a community capture disputes it: spec §11.3 D1).
6. Type-4 secondary display (bytes 5–8): V2's decode, which V1 ignores.
7. Unknown type bytes: only 1–4 defined (V1 parses others with the type-1 layout,
   labelled `S_5G`; V2 drops them).
8. Clock set: V1/Bluetooth DMM only, after the first type-4 packet and then
   every half hour; [UNVERIFIED] whether the meter needs or honours it.
9. Current key codes: V1's type-4 mapping and V2's dead branch swap the AC
   codes; the per-code meanings [UNVERIFIED].
10. Type-3 HOLD at byte 7 bit 4: kept (V2 shows it, V1 reads it but hides it).

## Tag legend

As used in `reverse-engineered-protocol.md`:

- **[KNOWN]** — stated in a ZOTEK manual, cited by file and page
- **[VENDOR]** — read from a ZOTEK app, with `file:line` or `V2@offset`
- **[INFERRED]** — logical inference from the above, reason given
- **[UNVERIFIED]** — no source confirms it; needs a real meter
- **[HARDWARE]** — seen on a real meter: none yet for this family
- **[COMMUNITY]** — from a community source, spec §11 only
