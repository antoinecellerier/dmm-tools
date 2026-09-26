# EEVblog 121GW: Reverse Engineering Approach

Scope: the EEVblog 121GW multimeter and its Bluetooth LE link, from the
meter's advertisement to the packets it sends and the commands it accepts.
Nothing is implemented yet; this pair of documents records what the meter
does on the wire, from vendor sources only.

The question was which of the formats in the vendor sources the meter sends
today. The answer is the **binary 19-byte packet** of EEVblog's "BLE Packet
Format" V2 document, which EEVblog's final app code decodes and whose two
additions over V1 are the manual's firmware 1.21 and 1.22 entries
(`reverse-engineered-protocol.md` §1). The ASCII formats that came before are
recorded only as far as recognising them (§12 there).

## Sources Used

Vendor sources are all EEVblog's own, or linked from EEVblog's store page as
the 121GW's software; source 5 is general platform reference material.
Fetched and analysed 2026-09-26. Provenance — URL, date, SHA-256, signer — is
in `references/121gw/SOURCE.txt` (gitignored); the four reader
reports are in `references/121gw/analysis/findings/`.

The index is EEVblog's product page, https://eevblog.store/products/121gw-multimeter
(checked 2026-09-26), which links the manual, schematic, firmware 1.00-2.05,
the "BLE Packet Format" V1 and V2, "UEI's Android app"
(`kr.co.finest.eevblog`), the legacy Android app (`eevblog.x121gw`), the
Meteor app, the Windows Store app, "Open source ports" (github.com/evotronix)
and the "App GIT" (gitlab.com/Sepps/app-121gw). Of the apps, only UEi's and
the App GIT were used; the legacy Android app, the Meteor app and the Windows
Store app were not opened.
www.eevblog.com sits behind a Cloudflare browser challenge, so the user saved
the eevblog.com files from a browser on 2026-09-26.

### Primary (clean-room RE)

1. **"BLE Packet Format" V1 and V2** — EEVblog's packet-format documents,
   three pages each (packet table, C union and sender, usage example). PDF
   author David Ledger, no title. Read from renders
   (`pdftoppm -r 200`, `protocol/render/v{1,2}-N.png`) with 400 dpi crops of
   the table and cell-border detection; `pdftotext` only to locate text.
   Tagged [KNOWN], cited `V1 p.N` / `V2 p.N`.
2. **EEVblog's app**, `git clone https://gitlab.com/Sepps/app-121gw.git`
   (2026-09-26), HEAD `48cd0fb` (2023-03-29, "Added Android APK"), MIT
   licence, © 2018 David Ledger; its README calls it the "latest and last
   official EEVblog 121GW cross-platform Visual Studio Xamarin/UWP app".
   **The checked-out `master` is not the final code**: its working tree
   (last protocol change 2017-12) still decodes an ASCII format. The final
   protocol code is on the unmerged branch `origin/PrivatePostRelease`, tip
   `aab403d` (2018-09-04, "UWP app now passes cert."), which decodes the
   binary packet. That branch and the older commits were read with
   `git show` / `git log`, leaving the clone unchanged. The bundled
   `x121GW.Android.apk` was not opened. github.com/EEVblog/EEVblog-121GW, an
   older copy (HEAD `aaafc6e`, 2017-12-06, plus one commit `4063d9b`
   "Update"), was not used. Tagged [VENDOR], cited `PPR path:line`,
   `git:<sha>:path:line` or `master path:line`.
3. **UEi's Android app**, `kr.co.finest.eevblog` 1.0.6 (versionCode 6, minSdk
   18, targetSdk 19), linked from the store page as "UEI's Android app" on
   Google Play. The user approved it on 2026-09-26 and asked for a mirror
   download, from APKPure:
   https://d.apkpure.com/b/APK/kr.co.finest.eevblog?version=latest.
   Decompiled with jadx 1.5.6 into `uei-app/jadx/` (exit 0); no smali check
   was run. Tagged [VENDOR], cited `UEi File.java:line`.
4. **The 121GW user manual**, "Last Revised: 3 March 2025", 73 pages. Read
   from renders (`pdftoppm -r 150`, `manuals/render/p-NN.png`; p.31, 39, 40,
   43, 53 and 71 again at 300-400 dpi); `pdftotext` only to locate text.
   Tagged [KNOWN], cited `manual p.N`.
5. **General platform references**, for two statements only, both tagged
   [INFERRED] with this basis in the spec: the Android `BluetoothGattDescriptor`
   API constants (`ENABLE_INDICATION_VALUE` is `02 00`, spec §2) and the
   Bluetooth Core Specification (the CCCD values, and the default ATT MTU of
   23 bytes, a 20-byte payload, spec §2 and §4). Neither is in the jadx tree;
   no 121GW-specific content comes from them.
6. **Firmware 2.05** (`EEVBlog2_05.zip`, holding only `EEVBlog2_05.bin`,
   2021-04-22, no release notes): archived, not analysed.

| Source | File | SHA-256 | Notes |
|---|---|---|---|
| Packet Format V1 | `protocol/121GW-BLE-Packet-Format-V1.pdf` | `c459d784143c29cf5e557dd361f3fd15d1e9f490799781da059c74a3914480c9` | store link "Packet Format V1", https://www.eevblog.com/wp-content/plugins/download-attachments/includes/download.php?id=11663; created 2018-03-22 |
| Packet Format V2 | `protocol/Revised-Packet-Format-Blob-V2.pdf` | `48b9c78c6d09d33e8a3d839e0baf556f56dfcc0faa75df33d29447d83ee740de` | store link "Packet Format V2", …/download.php?id=13664; created 2018-07-03 |
| Manual | `manuals/EEVblog-121GW-Manual.pdf` | `36707e2360ea2cca21d8d9d54134560fd8bf8bd1c6c87cd61791544cd6448f69` | https://www.eevblog.com/files/EEVblog-121GW-Manual.pdf; 73 pages, PDF created 2025-03-15 |
| Firmware 2.05 | `firmware/EEVBlog2_05.zip` | `86c16b56d70c9bf743dd805e3241307cd055394ab764968884deeec461f4746e` | https://www.eevblog.com/wp-content/uploads/2017/11/EEVBlog2_05.zip; not analysed |
| UEi app 1.0.6 | `uei-app/EEVBlog_121GW_1.0.6_APKPure.apk` | `3f63c3ab593448b25e2aad3fa4fd189a75d94d9dd3eb01453150baa114311ef1` | signer `CN=Finest Co., O=Finest, L=Songdo, ST=Incheon, C=KO`, cert SHA-256 `b3610e35c9c269401cf7501e0acef0784e80e1a295d6a6d61151ffb5a904193d` |
| EEVblog app | `official-app/` (git) | — | HEAD `48cd0fbfb8a51f82f47a3c1cb76cc65bc0b4de9e`; final protocol on `origin/PrivatePostRelease` @ `aab403d` |

### Avoided

Not opened while the spec was written (closed until a later cross-reference
pass):

- github.com/evotronix (the store page's "Open source ports")
- tpwrules/121gw-re and 121gw-88mph
- sigrok (libsigrok and the wiki)
- zonque/121gw-qt5
- chlordk/121gwcli
- forum protocol posts (EEVblog forum and others)
- the web in general, beyond the store page and the files above
- `docs/research/new-device-candidates.md` §"EEVBlog 121GW": it held
  community-derived notes on this meter written before this work, and was
  kept out of every reader's brief and out of the drafting of both documents

### Model-recall disclosure

The assistant that wrote these documents was likely trained on community
write-ups of this protocol (the projects above among them). Recall was never
used as a source: every fact in `reverse-engineered-protocol.md` cites a page
of the documents or the manual, or a file and line of one of the two apps,
and a fact without one is tagged [UNVERIFIED].

One slip is on record: the main session recognised the service and
characteristic UUIDs on sight, from prior knowledge, before UEi's app had
been read. Both UUIDs in the spec are sourced from UEi's app,
`BLEService.java:39-40`, and nothing else in the spec rests on that
recognition.

## Methodology

1. **Acquisition.** The store page listed the sources; the eevblog.com files
   were saved from a browser by the user (Cloudflare), the app repository
   cloned, UEi's APK taken from APKPure with the user's approval, and every
   file hashed on arrival (`SOURCE.txt`). The PDFs were rendered to PNG.
2. **Branch check.** `git log --all` on EEVblog's repository showed that the
   binary-packet code (commit `43b3e94`, 2018-03-22, onwards) never reached
   `master`, and located the final state at `PrivatePostRelease` @ `aab403d`.
3. **Four independent readers**, each given one source, the tag rules of
   `.claude/rules/research-docs.md` and no one else's report:
   - the packet-format documents (`findings/packet-doc.md`), kept away from
     the app code so the documents' own content stays separate from how the
     app reads it;
   - EEVblog's app, branch and history (`findings/app.md`);
   - UEi's app (`findings/uei-app.md`);
   - the manual (`findings/manual.md`).
4. **Adjudication** by the main session, comparing the four reports; the
   decisions are listed below.
5. **Drafting and citation check.** Both documents were written from the
   reports, with every carried-over citation re-read in the vendor source
   (`git show aab403d:<path>`, the jadx tree, the rendered pages); the
   manual's calibration table (p.71) and LCD drawing (p.31) were re-read at
   300 dpi, and the worked examples' checksums recomputed.
6. **Grounding check.** A fresh reader checked both documents against the
   same vendor sources, about 230 cites (`findings/grounding-check.md`). Its
   24 items were applied after each was re-read in the source: chiefly
   EEVblog's LCD against its chart multiplier (spec §6.2), the count of UEi's
   special sub codes, what the zero bytes of the worked examples decode to,
   tags on deduced statements, and absences rephrased as dated search
   results.

Resolved disagreements:

1. **Current wire format**: the binary 19-byte packet. V2 is a superset of V1
   and EEVblog's final branch decodes it; the manual's firmware 1.21 (°C/°F
   in the Bluetooth packet) and 1.22 (two more main-value bits) entries match
   V1 → V2 and the app's "packet format 1.22" commit.
2. **Checksum**: XOR of bytes 0-17 equals byte 18, as the documents' table and
   EEVblog's code say; the documents' p.2 sender, which XORs all 19 bytes,
   is a document error.
3. **Byte order**: big-endian, as the table and EEVblog's code; the p.2 C
   union's native `u32`/`u16` is a document error.
4. **Earlier formats**: the ASCII formats in EEVblog's history (26, 52 and
   37-40 characters after `F2`), the 54-byte format UEi's app decodes, and
   the V1 document's "previous packet was 54 bytes" are recorded by how to
   recognise them only; which firmware sent which is [UNVERIFIED].
5. **GATT**: service and characteristic UUIDs from UEi's app only; EEVblog's
   app subscribes to and writes every characteristic, which is consistent
   with them. Its UWP path subscribes only on Indicate, which with UEi's
   indication write supports "indicate". The advertised name stays
   [UNVERIFIED]: EEVblog's filter accepts "121GW" or "Bluegiga", UEi's matches
   the service UUID in AD type 0x07.
6. **Mode and range tables**: the two apps agree on every mode name; their
   range tables are set side by side with the manual's, every disagreement
   row is flagged, no winner picked, and each is an open question. After the
   grounding check (step 6), EEVblog's column gives what its LCD shows, which
   lights units by mode: there it agrees with UEi and the manual in modes 7,
   13/22, 14/23 and 16/17 (ms, µVA, mVA, µA), and the ×1/VA figures an earlier
   draft set against UEi are only its chart multiplier, an inconsistency inside
   that app. The diode 3 V resolution and the top capacitance range stay open.
7. **Bar-graph sign polarity**: left open; EEVblog's app inverted it in 2018
   and the documents state none.
8. **Commands**: both apps agree on key codes 01-08 and 81-88; the buzzer
   code 09 and the `F8` clock set are UEi's only. Whether current firmware
   honours these ASCII-framed commands is [UNVERIFIED]; EEVblog's final
   branch still sending them is evidence, recorded as such.
9. **Bring-up**: connect, discover, enable indications; neither app sends a
   start command or a keep-alive.

## Tag legend

As used in `reverse-engineered-protocol.md`:

- **[KNOWN]** — stated in the packet-format documents or the manual, cited
  by page
- **[VENDOR]** — read from EEVblog's or UEi's app, with file and line
- **[INFERRED]** — logical inference from the above, reason given
- **[UNVERIFIED]** — no source confirms it, or the sources disagree; needs a
  real meter
- **[HARDWARE]** — seen on a real meter: none yet
