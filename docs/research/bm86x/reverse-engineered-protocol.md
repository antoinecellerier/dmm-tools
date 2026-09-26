# Brymen BU-86X Group (BM860s, BM820s, BM520s): Reverse-Engineered Protocol Specification

What the Brymen meters that talk through the BU-86X optical USB cable send
and accept: the BM860s series (BM867s, BM869s), the BM820s series (BM821s,
BM822s, BM827s, BM829s) and the BM520s mobile-logging series (BM521s,
BM525s). The cable is a USB HID 1.1 device. The host writes a 4-byte request
(a report-ID byte `00` and three bytes, the second naming the series) and
reads 27 bytes: three 9-byte input reports, each a `00` report-ID byte and 8
data bytes. The reply is a map of LCD segments, not a number: digits arrive
as 7-segment patterns, and function, unit, prefix, sign and decimal point as
annunciator bits, in one layout for the BM860s and another for the BM820s
and BM520s. The BM521s and BM525s also answer three commands that page out
their logged memory. Not implemented. No meter from this group has been on
our bench: every fact in §1-11, and each question of §12, comes from
Brymen's protocol documents, Brymen's two Windows programs and their
READMEs, and the two user manuals; §13 compares them with community
sources, and §12's "Community:" notes summarise §13. The approach doc
beside it records the sources, the method and the clean-room boundary.

Based on:
- Brymen's protocol sheets "Protocol for 500000-count professional dual
  display DMM series" (BM860s), "Protocol for 10000-count professional dual
  display DMM series" (BM820s) and "Protocol for 10000-count professional
  dual display mobile logging DMM series" (BM520s), two pages each, and the
  untitled 9-page logged-memory document shipped with the last, read from
  the rendered pages
- Brymen's Windows programs Bs86x "Ver 6.0.0.3s" (BM860s) and Bs82-52x
  "Ver 6.0.0.9s Alpha" (BM820s and BM520s), Borland C++Builder builds,
  decompiled with Ghidra 12.1.3 and read in the disassembly where the
  decompile was unclear; and the READMEs shipped with them
- The BM860s user's manual (BM867s, BM869s) and the BM820s/BM520s user's
  manual (BM821s, BM822s, BM827s, BM829s, BM521s, BM525s), read from the
  rendered pages
- For §1.3 only: Brymen's BM250/BM250s protocol sheet and its Bs25x V5003
  program and README
- For §13 only: six community code repositories and TestController's
  supported-equipment list, opened 2026-09-27

Citation keys (paths under `references/bm86x/`, gitignored):
- **BM860 p.N** — `protocol/BM860-BM860s-protocol.pdf`
- **BM820 p.N** — `protocol/BM820-BM820s-protocol.pdf`, byte-identical to
  `protocol/BM520s-zip/10000count-professional-dual-display-DMMs-protocol.pdf`
- **BM520-ML p.N** —
  `protocol/BM520s-zip/10000count-professional-dual-display-mobile-logging-DMMs-protocol.pdf`
- **MRAD p.N** — `protocol/BM520s-zip/Memory-Read-Allocation-Decoder.pdf`
- **BM250 p.1** — `protocol/BM250-BM250s-protocol-r1.pdf`
- **BM860s manual p.N** — `manuals/BM860s-manual.pdf`; **BM820s manual
  p.N** — `manuals/BM820s-520s-manual.pdf`. N is the PDF page; the printed
  page number is N − 1 in both, the cover being unnumbered
- **README-86x §n**, **README-8252x §n** — the numbered sections of
  `app/README Bs86x V6003s.rtf` and `app/README Bs82-52x V6008s.rtf`;
  **README-25x §n** — `software/BM250-Bs25x-V5003/5003-win32-x64/README
  Bs25x V5002.rtf` (§1.3 only)
- **Bs86x `:N`** — line N of `ghidra/Bs86xV6003s-decompiled.txt`;
  **Bs86x sigfix `:N`** — `ghidra/Bs86xV6003s-sigfix-annot.txt` (chosen
  functions re-decompiled, string literals filled in); **Bs86x `@ 0xADDR`**
  — an address in `app/Bs86xV6003s.exe`, read with objdump. **Bs8252x**
  likewise for `ghidra/Bs8252xV6009sA-*` and `app/Bs8252xV6009sA.exe`
- **Community sources**, §13 only — paths under
  `references/bm86x/community/` at the commits of §13.1; a bare `bm86x.c`,
  `bm52x.c` or `api.c` is in `libsigrok/src/dmm/` or
  `libsigrok/src/hardware/serial-dmm/`, a bare `BM869S.py` in
  `BM869S-remote-access/`, `brymen-BM869s.py` in `Brymen-BM869s/`,
  `Brymen869.cpp` in `Brymen869s-XmlLib/Source/Brymen869/`, `main.S` in
  `869log/firmware/`, and `decoder.cpp`, `main.cpp` or `config.h` in
  `brymen-867-interface-cable/firmware/BrymenConnector_new/`

The protocol sheets name no model, only a count and a series title; the
series names come from the file names and Brymen's download pages
(`SOURCE.txt`) [KNOWN].

Byte and bit numbering: **byte n** counts from 1, as Brymen's tables do,
byte 1 being the first report-ID byte. **R[n−1]** is the same byte 0-based in
the 27-byte buffer Brymen's programs fill (three reads of 9 bytes at offsets
0, 9 and 18). **n.b** is bit b of byte n, bit 0 the least significant, as
Brymen's "bit 0"-"bit 7" columns. Brymen's tables print the micro prefix as
`u`; below it is written µ. Circled numbers ①-⑤ are printed beside many
table cells and on the LCD figures; no legend is printed. Read against the
figures, ① marks main-display symbols, ② secondary-display symbols, and
③-⑤ single small segments, numbered differently per sheet: on BM860, ③
the T1–T2 dash (4.2) and ④ the bar-graph minus (4.5); on BM820 and
BM520-ML, ③ the bar-graph minus (no table cell), ④ the T1–T2 dash (4.1)
and ⑤ the MAX-MIN dash (3.2) [INFERRED from the figure positions].

Confidence levels:
- **[KNOWN]** — stated in a Brymen document (protocol sheet, logged-memory
  document, README, manual), cited by page or section
- **[VENDOR]** — read from Brymen's programs, cited by file and line or
  address
- **[INFERRED]** — deduction from the above, reason given
- **[UNVERIFIED]** — no source confirms it, or the sources disagree; needs a
  real meter (all in §12)
- **[HARDWARE]** — seen on a real meter: none yet
- **[COMMUNITY]** — stated in or implied by a community source, §13 only
  (opened 2026-09-27), summarised in §12's "Community:" notes; not a vendor
  fact

---

## 1. Models, series and cable

### 1.1 Models

| Series | Models (manual p.1) | Protocol sheet | Brymen's program | Real-time request (sheet) | Model bytes (sheet) |
|---|---|---|---|---|---|
| BM860s, 500000 counts | BM867s, BM869s | BM860 | Bs86x | `00 00 86 66` | byte 23 = `86` |
| BM820s, 10000 counts | BM821s, BM822s, BM827s, BM829s | BM820 | Bs82-52x | `00 00 82 66` | bytes 20-23 = `82` |
| BM520s, 10000 counts, mobile logging | BM521s, BM525s | BM520-ML (+ MRAD) | Bs82-52x | `00 00 52 66` | bytes 20-23 = `52` |

Neither manual uses the names "BM860s" or "BM820s"; the BM860s manual covers
BM867s and BM869s, the BM820s manual the six others (BM860s manual p.1,
BM820s manual p.1) [KNOWN]. That the BM521s and BM525s are the BM520s series
of the "mobile logging" sheet is [INFERRED]: they are the only models with
data logging (BM820s manual p.14-15). The one manual is linked from both
Brymen's BM820s and BM520s manual pages, while the Bs82-52x zip is linked
from the BM820s software page (`SOURCE.txt`) [KNOWN]. The BM820 sheet ships again inside
the BM520s zip, byte-identical (`SOURCE.txt`) [KNOWN]. The Bs82-52x README
names the program "Bs82-52x Data Recording System" and its import function
"for datalogging series only" (README-8252x §3-2-1-4) [KNOWN]. Community:
see §13.8.

### 1.2 The cable

| Source | Name given | Text |
|---|---|---|
| BM860s manual p.13, p.18 | BU-86X | "Optional purchase PC USB interface kit BU-86X is required"; "Optional Accessories: BU-86X PC interface kit" |
| BM820s manual p.13 | **BU-82X** | "Optional purchase PC USB interface kit BU-82X is required to connect the meter to the PC computer" |
| BM820s manual p.20 | BU-86X | "Optional purchase accessories: USB interface kit BU-86X" |
| README-86x §1-2 | BC-86X | "This software uses only the BC-86X optical USB adapter cable. It is NOT compatible with the BC-82X, BC-81X, BC-83X, BC-85X or BC-20X cables." |
| README-8252x §1-2 | BC-86X | the same sentence, listing BC-81X, BC-83X, BC-85X and BC-20X; BC-82X is not in its list |
| Software zip names | BC86X | `…-use-BC86X-Win7.zip`, both programs (`SOURCE.txt`) |

All [KNOWN]. The BM820s manual contradicts itself; Brymen's program for the
BM820s and BM520s names only BC-86X, so three sources to one point to the
BU-86X kit [INFERRED]; which cable a BM820s actually needs is open (§12).
Community: see §13.3.
That a BU-86X kit contains the BC-86X cable is [INFERRED from the names]:
the manuals name the kit, the READMEs the cable.

Both READMEs call it "the microprocessor embedded cable BC-86X" using a
"USB compliance protocol that conforms to USB V2.0 and HID V1.1"
(README-86x §0-4, README-8252x §0-4) [KNOWN]. The meter end is "an optical
isolated interface port at the meter back" (BM860s manual p.13; BM820s
manual p.13), labelled "PC-Comm" on the panel drawings (BM860s manual p.12
figures; BM820s manual p.1, p.4) [KNOWN]. A key, key combination or menu
step to start PC output: none found in either manual, 2026-09-26. Whether
the cable relays each request to the meter or answers from a reading it
holds is not stated in any source [UNVERIFIED]. Community: see §13.6.

### 1.3 BM250s: a separate serial protocol

The BM250/BM250s sheet describes a serial link: "COM Port Communication
Protocol: (Baud rate, Parity, Data bits, Stop bit) = (9600, N, 8, 1)",
activated by holding HOLD while turning the dial on, and a 15-byte stream,
each byte's high nibble its sequence number 0-14, with no request (BM250
p.1) [KNOWN]. Its program's README names the "RS232C optical RS232C
interface cable BC-20X", used on a COM port or through the "BUA-2303
USB-to-Serial Adaptor" (README-25x §1-2, §1-3, §1-4, §2-2) [KNOWN]. The
program, Bs25x V5003, imports no HID.DLL or SETUPAPI and sets 9600 8N1 on a
COM port (`Bs25xV5003.exe @ 0x407bf0-0x407c0e`) [VENDOR], and README-86x
§1-2 lists BC-20X among the incompatible cables [KNOWN]. It is out of this
spec's scope.

## 2. USB

| Item | Value | Source | Tag |
|---|---|---|---|
| Class | "\*USB communication protocol: Conform to USB HID1.1" | BM860 p.1, BM820 p.1, BM520-ML p.1 | [KNOWN] |
| USB version | "conforms to USB V2.0 and HID V1.1" | README-86x §0-4, §1-3 | [KNOWN] |
| VID:PID, programs | **0x0820:0x0001**, 16-bit compares against `HIDD_ATTRIBUTES` VendorID and ProductID. Two openers: `FUN_0042247c`, called with `mov ax,0x820; mov dx,0x1` (`@ 0x422cce-0x422cd6`), and `FUN_004a7c40`, the live thread's, called with `mov cx,0x1; mov dx,0x820` (`@ 0x4a478c-0x4a4797`; `FUN_004a7c40(…,0x820,1,…)` at Bs86x `:109970`) | Bs86x `@ 0x4224c9-0x4224db`; Bs86x `:14721-14750`, `:111677-111710`; Bs8252x `:14154-14190` | [VENDOR] |
| VID/PID, sheets | flowchart box "Open HID device with Vendor_ID: 0x82, Product_ID: 0x01" | BM860 p.2, BM820 p.2, BM520-ML p.2, MRAD p.4 | [KNOWN] |
| Device selection | the programs walk HID interfaces 0-19 and keep the first whose VID and PID match; no product-string or version check | Bs86x `:14681-14750` | [VENDOR] |
| Product string | never read: HID.DLL is imported only by ordinals 3, 6 and 15, which are HidD_GetAttributes, HidD_GetHidGuid and HidD_GetSerialNumberString per the export table of the bundled hid.dll 5.1.2600.5512 | imports of both programs; `objdump -p` of the bundled hid.dll | [VENDOR] |
| Version | `HIDD_ATTRIBUTES.VersionNumber`, shown only in a hidden About-box handler as "Cable Firmware ver.: XX.YY" (`IntToHex(VersionNumber,4)`, a dot inserted) | Bs86x sigfix `:862-1020`, `@ 0x408e4e` | [VENDOR] |
| Serial | `HidD_GetSerialNumberString(handle, buf, 8)`; the 8 bytes are shown in hex after "Cable SN : " | Bs86x `@ 0x408ff1-0x408ffa`, sigfix `:862-1020` | [VENDOR] |
| Output | 4 bytes per write: report ID `00` and three bytes; `WriteFile(h, buf, 4)` | BM860 p.1; Bs86x `:109983-109995` | [KNOWN], [VENDOR] |
| Input | 27 bytes as "3 Input Reports"; the programs read with `ReadFile(h, buf+n, 9)` until 27 bytes arrive; memory replies 36 bytes, "4 Input Reports" | BM860 p.2; MRAD p.4; Bs86x `:109997-110015` | [KNOWN], [VENDOR] |
| Feature reports | none: HidD_SetFeature and HidD_SetOutputReport are not imported | imports | [VENDOR] |

**VID:PID.** The programs match 0x0820:0x0001 and would not open a device
enumerating as 0x0082:0x0001 [VENDOR]. The sheets' "0x82" is the programs'
value short of one hex digit; whether it is a typo or shorthand is not
stated [INFERRED]. The value the cable enumerates with is [UNVERIFIED]; the
programs' value is the stronger evidence, since they run against real
cables. Community: see §13.2.

**Report shape** [INFERRED from the Windows HID class convention: one
whole report per `ReadFile`/`WriteFile`, the report-ID byte first, `00`
when the device does not number its reports]: input reports are 8 data
bytes and output reports 3, unnumbered. The sheets' "Report ID= I", "II",
"III" name the three reads, not three IDs. On the USB wire the reports then
carry no ID byte. The HID report descriptor, endpoints and polling interval
are not given in any source [UNVERIFIED]. Community: see §13.2.

## 3. Requests and timing

### 3.1 Requests

| Bytes (report ID first) | Sheet | Sent by Brymen's programs |
|---|---|---|
| `00 00 86 66` | BM860 p.1, real-time | Bs86x live loop (`:109983-109995`) |
| `00 00 82 66` | BM820 p.1, real-time | Bs8252x live loop (`:21520-21533`); both programs' import probe (Bs86x `:14788-14800`; Bs8252x `:14225-14240`) |
| `00 00 52 66` | BM520-ML p.1, "request Real-Time Data (Cs_RTD)" | neither (§3.3) |
| `00 00 52 88`, `…89`, `…8A` | BM520-ML p.1, MRAD p.1, memory (§10) | both programs' import (§10.4) |

[KNOWN] for the sheets, [VENDOR] for the programs. In every request the
byte after the report ID is `00` ("Command 1"), "Command 2" is the series
code, the value the reply carries in byte 23, and "Command 3" is `66` for a
real-time reading [KNOWN].

The sheets do not say whether a meter answers a request carrying another
series' code, 2026-09-26. What the programs imply [INFERRED]: Bs82-52x
serves both series with `82 66` for live readings, and both programs gate
the memory import on an `82 66` reply whose byte 23 is `52` (§10.4). So
Brymen's programs expect a BM521s or BM525s to answer `82 66`, with `52` in
byte 23. Whether it also answers `52 66`, as its own sheet says, and whether
a BM860s answers `82 66` or a BM820s `86 66`, are [UNVERIFIED]. Bs86x sends
its `82 66` probe to whatever is on the cable, a BM860s included (§10.4)
[VENDOR]. Community: see §13.3.

### 3.2 The sheets' flowchart

"Proposal program design flowchart", the same on BM860 p.2, BM820 p.2 and
BM520-ML p.2 but for the command bytes [KNOWN]:

1. Open the HID device (VID 0x82, PID 0x01).
2. "Allocate Buffers for 3 Input Reports (27 bytes)".
3. Send `0x00, 0x00, 0x86 (0x82, 0x52), 0x66`.
4. Clear and start a time-out counter.
5. Wait until all 3 reports are available. After more than 4 s, close the
   device and start again at 1.
6. Close the device; decode (Table 1, Fig 1); display.
7. Wait until the counter passes 0.5 s, then go to 1 for the next reading.

So the sheets reopen the device for every reading and ask at most about
twice a second [INFERRED from steps 1, 6 and 7]. No other timing is stated,
2026-09-26.

### 3.3 What Brymen's programs do

[VENDOR], the same in both but where noted (Bs86x `:109975-110080`;
Bs8252x `:21520-21600`):

1. **Link** test-opens the cable, closes it and starts a worker thread
   (Bs86x `:1806-1837`).
2. The thread writes the 4-byte request on an overlapped handle and waits up
   to 2000 ms; the wait's result is overwritten and ignored.
3. It issues `ReadFile(…, 9)` until 27 bytes have arrived: up to 4000 ms for
   the first read, 100 ms for each later one.
4. On success it decodes and sends the next request at once: no pacing, no
   Sleep, and the handle stays open between readings.
5. On a timeout it re-enumerates. Cable gone: it signals an event whose
   waiter shows string 9015, "USB adapter cable not found." (Bs86x
   `:110023`; waiter `:255-303`, `FUN_0040172c`). Cable present: close,
   `Sleep(10)`, reopen, retry. The third failure shows "No data received."
   captioned "Error[E9015]", then signals the same event, so "USB adapter
   cable not found." follows, and the link ends (Bs86x `:110030-110045`).
   Bs86x counts failures over the
   whole link session; Bs8252x resets the count on every good reply
   (`@ 0x4358ca`).
6. The live path checks nothing in the reply: not the report-ID bytes, not
   the model bytes, no checksum.

The live request byte is fixed per program: `86` in Bs86x, `82` in
Bs8252x; there is no choice in the UI and no probing (both decompiles)
[VENDOR]. A write of `00 00 52 66`: none found in either program (every
store of `0x66` is at the sites of §3.1 and in the two timer-driven
readers below, which write `86 66` (Bs86x `:14924-14927`) and `82 66`
(Bs8252x `:14428-14431`)), 2026-09-26 [VENDOR]. A second,
timer-driven reader (`TimerGetSdTimer`) exists in both programs but its
timer is disabled in the form resource and re-enabled only from inside
itself (Bs86x dfm `:2408-2412`, handlers `:557,615,1021`) [VENDOR]; it is
dead code [INFERRED].

The READMEs add [KNOWN]:
- "The meter Auto-Power-Off (APO) feature is disabled when linked."
  (README-86x §2-5, README-8252x §2-5). How the meter learns it is linked
  is not stated.
- "The actual data interval may vary up to seconds depending on the meter
  function selected" (README-8252x §3-2-2-4).
- Capacitance: "the software performance will be slowed down significantly
  waiting for the DMM measurement … the software may give an incorrect
  warning message of data receiving error after waiting for a long period"
  (README-86x §6-1, README-8252x §6-1).

So a reply waits for the meter's next measurement, and in capacitance can
take longer than the programs' 4 s [INFERRED from the READMEs]. The display
updates 5 times a second nominal, 1.25 in the BM860s 500000-count mode
(BM860s manual p.17; BM820s manual p.19), and REC runs at "fast 20/s" on the
BM820s (BM820s manual p.13) [KNOWN]. How the reply rate relates to these is
[UNVERIFIED]. Community: see §13.4.

### 3.4 No meter, meter off

The programs treat "the cable enumerates but 27 bytes do not arrive within
the waits" as meter off or absent: "No data received." on the live path,
"Your meter is power off, disconnected or without built-in DATA LOGGING
function." (E9017) on the import path, and README text E9016 "Data
transmission timeout. Please check meter and cable connection as well as
meter power." (Bs86x `:110030-110045` for the live path, Bs86x sigfix
`:2237-2330` for E9017; README-8252x §4-2) [VENDOR],
[KNOWN]. What the cable itself sends with no meter — nothing, or reports of
filler — is [UNVERIFIED]. The import probe maps a byte 23 of `FF`, `F7`,
`F3`, `F1`, `F0`, `E0`, `C0`, `80` or `00` to `82` (Bs86x
`@ 0x42273a-0x4227c2`), and the disabled Bs8252x reader accepts byte 23 only
in {`82`, `52`, `FF`, `F7`, `F3`, `F1`, `F0`, `E0`, `C0`, `80`, `00`}
(Bs8252x `@ 0x424f75-0x425051`) [VENDOR]. Why these values: not stated
[UNVERIFIED]. Community, on the cable with the meter off: see §13.4.

## 4. The 27-byte reply

### 4.1 Reports and numbering

| Report | Byte (1-based) | R (0-based) | Content | Source |
|---|---|---|---|---|
| I | 1 | 0 | `00` "Report ID= I" | BM860 p.1, BM820 p.1 |
| I | 2-9 | 1-8 | data; byte 2 "don't care" in both real-time maps | |
| II | 10 | 9 | `00` "Report ID= II" (BM520-ML differs, §4.3) | |
| II | 11-18 | 10-17 | data | |
| III | 19 | 18 | `00` "Report ID= III" | |
| III | 20-27 | 19-26 | data | |
| IV, memory only | 28 | 27 | `00` "Report ID" | MRAD p.1-2 |
| IV | 29-36 | 28-35 | checksum in 29-30; 31-36 "don't care" | MRAD p.1-2 |

[KNOWN]. The live decoders copy R[2..8], R[10..17] and R[19..23] into a
20-byte buffer and read nothing else: bytes 1, 2, 10, 19 and 25-27 are
never looked at (Bs86x `:110297-110322`; Bs8252x `:21836-21861`) [VENDOR].
Community: see §13.3.

### 4.2 Model bytes

| Sheet | Bytes 20-22 | Byte 23 |
|---|---|---|
| BM860 p.1 | "don't care" | "Model ID3: 0x86" |
| BM820 p.1 | "Model ID0: 0x82", "Model ID1: 0x82", "Model ID2: 0x82" | "Model ID3: 0x82" |
| BM520-ML p.1 | `0x52`, unlabelled | `0x52`, unlabelled |

[KNOWN]. Byte 23 is the one position that names the series in all three
sheets, and it equals the request's Command 2 [INFERRED from the tables].
The live path of either program reads no model byte; only the import probe
reads byte 23 (§10.4) [VENDOR]. Nothing tells models within a series apart,
2026-09-26 (sheets and programs) [KNOWN], [VENDOR]; the functions a meter
shows are the only hint (§11.1) [INFERRED]. Community: see §13.3.

### 4.3 Where report II starts in a BM820s/BM520s reply

The documents disagree [KNOWN]:

| Evidence | Byte 10 | Byte 11 |
|---|---|---|
| BM820 p.1 Table 1 | Report ID II | digit 5 (`5b 5g 5c 4p 5a 5f 5e 5d`) |
| BM820 p.1 example, table and hex string | digit 5 (`6Dh`) | `00h` Report ID II |
| BM520-ML p.1 Table 1 and example | digit 5 | Report ID II |
| BM860 p.1 Table 1 | Report ID II | digit 6 |
| MRAD p.1-2 (same zip as BM520-ML) | Report ID | data |

The BM820 Table 1 layout is the one the report structure allows: three
equal 9-byte reports put the report-ID bytes at 1, 10 and 19, as BM860
Table 1, the "3 Input Reports (27 bytes)" flowchart box and MRAD's 36-byte
replies with IDs at 1, 10, 19 and 28 all show [INFERRED]. Bs8252x reads
the secondary digits from bytes 11-14 and skips byte 10 (R[9]) as a report
ID (Bs8252x `:21836-21861`, `:22207-22424`) [VENDOR]: it decodes the BM820
Table 1 layout. Read with that decoder, the BM820 example's secondary
display comes out as " 0.12" instead of 50.12 (§9.2). The meter's layout is
[UNVERIFIED]; the example and the BM520-ML table look like a drafting error
[INFERRED]. Community: see §13.2.

---

## 5. BM860s LCD map

### 5.1 Table 1

BM860 p.1 [KNOWN]; the last column is what Bs86x reads (Bs86x
`:110484-111205`; bit sources checked in the flag and exponent functions
`:111085-111205`) [VENDOR]. "—" means the program does not read it.

| Byte (R) | bit 7 | bit 6 | bit 5 | bit 4 | bit 3 | bit 2 | bit 1 | bit 0 | Bs86x reads |
|---|---|---|---|---|---|---|---|---|---|
| 1 (0) | Report ID I = `00` | | | | | | | | skipped |
| 2 (1) | don't care | | | | | | | | skipped |
| 3 (2) | AVG | MIN | MAX | ⎓ (DC) | [H] | [C] | [R] | [AUTO] | b4 main DC; others — |
| 4 (3) | ▭ ① | VFD | ▭ ④ (thin) | bar scale | T2 ① | ▭ ③ (small) | T1 | ∿ (AC) ① | b0 main AC, b1 T1, b3 T2, b7 main minus; others — |
| 5 (4) | 1b | 1g | 1c | 1d | 1a | 1f | 1e | △ | b7-b1 main digit 1; b0 — |
| 6 (5) | 2b | 2g | 2c | 2d | 2a | 2f | 2e | 1p | digit 2; b0 point code 8 |
| 7 (6) | 3b | 3g | 3c | 3d | 3a | 3f | 3e | 2p | digit 3; b0 point code 4 |
| 8 (7) | 4b | 4g | 4c | 4d | 4a | 4f | 4e | 3p | digit 4; b0 point code 2 |
| 9 (8) | 5b | 5g | 5c | 5d | 5a | 5f | 5e | 4p | digit 5; b0 point code 1 |
| 10 (9) | Report ID II = `00` | | | | | | | | skipped |
| 11 (10) | 6b | 6g | 6c | 6d | 6a | 6f | 6e | V ① | digit 6 (checked for `C`/`F`); b0 main V |
| 12 (11) | [battery] | T2 ② | ∿ (AC) ② | ▭ ② | %4~20mA | A ② | m ② | µ ② | b7 low battery, b6 sub T2, b5 sub AC, b4 sub minus, b3 sub %, b2 sub A, b1 sub m, b0 sub µ |
| 13 (12) | 7b | 7g | 7c | 7d | 7a | 7f | 7e | •))) | sub digit 1; b0 continuity |
| 14 (13) | 8b | 8g | 8c | 8d | 8a | 8f | 8e | 7p | sub digit 2; b0 sub point code 4 |
| 15 (14) | 9b | 9g | 9c | 9d | 9a | 9f | 9e | 8p | sub digit 3; b0 sub point code 2 |
| 16 (15) | 10b | 10g | 10c | 10d | 10a | 10f | 10e | 9p | sub digit 4; b0 sub point code 1 |
| 17 (16) | A ① | n | F | S | V ② | Hz ② | k ② | M ② | b7 main A, b6 main n, b5 main F, b4 main S, b3 sub V, b2 sub Hz, b1 sub k, b0 sub M |
| 18 (17) | D% | k ① | M ① | Ω | µ ① | m ① | dB | Hz ① | b7 main %, b6 main k, b5 main M, b4 main Ω, b3 main µ, b2 main m, b1 dB, b0 main Hz |
| 19 (18) | Report ID III = `00` | | | | | | | | skipped |
| 20-22 (19-21) | don't care | | | | | | | | — |
| 23 (22) | Model ID3: `0x86` | | | | | | | | — (live); import probe only (§10.4) |
| 24-27 (23-26) | don't care | | | | | | | | — |

▭ is a rounded-rectangle segment, △ an outlined triangle, [H] [C] [R]
[AUTO] boxed outline letters; n, F and S in byte 17 carry no circled number
but sit in the main unit row of the figure (BM860 p.1) [KNOWN]. The map
encodes no dial position, function code or range code (BM860 p.1) [KNOWN].

### 5.2 Digits and decimal points

- **Main display**: digits 1-6, large, bottom; points 1p-4p drawn at the
  bottom right of digits 1-4; no 5p. **Secondary display**: digits 7-10,
  small, top right; points 7p-9p at the bottom right of digits 7-9 (BM860
  p.1, Fig 1) [KNOWN].
- **Point Np sits in bit 0 of digit N+1's byte** (Table 1) and is drawn
  after digit N (Fig 1) [KNOWN]. Bs86x agrees: its main point code
  (6.0, 7.0, 8.0, 9.0 as 8, 4, 2, 1) puts the point after digit 1, 2, 3 or
  4, and its sub code (14.0, 15.0, 16.0 as 4, 2, 1) after sub digit 1, 2 or
  3; any other combination gives no point (Bs86x `:110484-111077`)
  [VENDOR]. The worked example is the one place that disagrees (§9.1).
  Community: see §13.5.
- **Digit 6.** In the 50000-count mode the sixth digit is blank in the
  example (byte 11 = `00h`, BM860 p.1), and the manual's temperature
  figures print the unit letter there ("0250.8C", "0483.4F", BM860s manual
  p.9) [KNOWN]; Bs86x looks for `C` or `F` in digit 6 (Bs86x
  `:111171-111205`) [VENDOR]. How the 500000-count mode fills the six digits
  is not shown in the sheet [UNVERIFIED].

### 5.3 Signs

The sheet never labels a segment "minus". 4.7 (▭ ①, left of digit 1's
middle segment) and 12.4 (▭ ②, beside the secondary digits) are the minus
signs [INFERRED]: they are horizontal bars placed before each display, the
circled numbers tie them to the main and secondary displays, the manual's
LCD drawing has a minus bar at both places (BM860s manual p.1), and Bs86x
reads 4.7 as the main "-" and 12.4 as the sub "-" (Bs86x `:110484-110857`,
`:110861-111077`) [VENDOR]. A small "1" printed just left of segment 1g,
beside the ① bar, is neither a table cell nor explained (BM860 p.1)
[UNVERIFIED]. Community: see §13.5.

### 5.4 Bar graph

"41 Segments Bar graph: 60 per second max" (BM860s manual p.17) [KNOWN]. The
figure draws a scale 0-5, a ▷ at the right end and a "−" marked ④ at the
left (BM860 p.1) [KNOWN]. The bar pointers are mapped to no bit; the only
bar bits are 4.4 "bar scale" and 4.5 (▭ ④), which is the bar's minus by its
circled number [INFERRED]. 4.4 lights the scale numerals [INFERRED: it is
set in the example, whose figure highlights the numerals]. Neither program
reads 4.4 or 4.5, or draws a bar from any byte (both decoders; the raw
27-byte copy is never read, Bs86x `:110065`) [VENDOR]. Community: see
§13.5, §13.6.

### 5.5 Annunciators Bs86x does not read

| Bit | Label | Meaning per the manual |
|---|---|---|
| 3.0 | [AUTO] | auto-ranging; off in manual range (BM860s manual p.14) |
| 3.1 | [R] | REC (MAX/MIN/AVG) mode (p.13) |
| 3.2 | [C] | CREST (p.13) |
| 3.3 | [H] | HOLD [INFERRED: the HOLD key is marked with a boxed H, p.1] |
| 3.5, 3.6, 3.7 | MAX, MIN, AVG | the REC and CREST readout in use (p.13) |
| 4.2 | ▭ ③ | the dash between T1 and T2 [INFERRED by ③ and position] |
| 4.4, 4.5 | bar scale, ▭ ④ | §5.4 |
| 4.6 | VFD | lit in the VFD functions (BM869s; figures p.6) |
| 5.0 | △ | relative zero (Δ) [INFERRED: the Δ key toggles relative zero, p.14, and this is the only Δ-shaped segment] |

[KNOWN] for the labels (BM860 p.1). A dBm reference annunciator or a CREST
peak marker other than [C] and MAX/MIN: none in Table 1, 2026-09-26. Bs86x
reads 12.3 %4~20mA, but only as the sub unit "%" (§8.1) [VENDOR].
Community: see §13.5.

## 6. BM820s and BM520s LCD map

### 6.1 Table 1

BM820 p.1 [KNOWN]; BM520-ML p.1 is the same cell for cell apart from bytes
10/11 (§4.3) and the model bytes (§4.2) [KNOWN]. The last column is what
Bs8252x reads (Bs8252x `:21986-22567`; bit sources checked in the flag and
exponent functions `:22434-22567`) [VENDOR]. Circled numbers are printed
before the symbol in this sheet.

| Byte (R) | bit 7 | bit 6 | bit 5 | bit 4 | bit 3 | bit 2 | bit 1 | bit 0 | Bs8252x reads |
|---|---|---|---|---|---|---|---|---|---|
| 1 (0) | Report ID I = `00` | | | | | | | | skipped |
| 2 (1) | don't care | | | | | | | | skipped |
| 3 (2) | Hi | Lo | ① ⎓ (DC) | ① ∿ (AC) | MIN | ⑤ ▭ | AVG | MAX | b5 main DC, b4 main AC; others — |
| 4 (3) | ① ▭ | △ | % | LoZ | ① T2 | LPF | ④ ▭ | ① T1 | b7 main minus, b3 T2, b0 T1; others — |
| 5 (4) | 1b | 1g | 1c | 1p | 1a | 1f | 1e | 1d | main digit 1; b4 point code 4 |
| 6 (5) | 2b | 2g | 2c | 2p | 2a | 2f | 2e | 2d | digit 2; b4 point code 2 |
| 7 (6) | 3b | 3g | 3c | 3p | 3a | 3f | 3e | 3d | digit 3; b4 point code 1 |
| 8 (7) | 4b | 4g | 4c | dB | 4a | 4f | 4e | 4d | digit 4 (checked for `C`/`F`); b4 dB |
| 9 (8) | ② ⎓ (DC) | ② ∿ (AC) | ② ▭ | @ | [battery] | ② T1 | ② T2 | •))) | b7 sub DC, b6 sub AC, b5 sub minus, b3 low battery, b2 sub T1, b1 sub T2, b0 continuity; b4 — |
| 10 (9) | Report ID II = `00` (§4.3) | | | | | | | | skipped |
| 11 (10) | 5b | 5g | 5c | 4p | 5a | 5f | 5e | 5d | sub digit 1; b4 sub point code 4 |
| 12 (11) | 6b | 6g | 6c | 5p | 6a | 6f | 6e | 6d | sub digit 2; b4 sub point code 2 |
| 13 (12) | 7b | 7g | 7c | 6p | 7a | 7f | 7e | 7d | sub digit 3; b4 sub point code 1 |
| 14 (13) | 8b | 8g | 8c | %4~20mA | 8a | 8f | 8e | 8d | sub digit 4 (checked for `C`/`F`); b4 — |
| 15 (14) | ② m | ② µ | ② A | ② V | ② D% | ② n | ② S | ② F | b7 sub m, b6 sub µ, b5 sub A, b4 sub V, b2 sub n, b1 sub S, b0 sub F; b3 — |
| 16 (15) | ① k | ① M | ① Ω | ① Hz | ② M | ② k | ② Ω | ② Hz | all: main k, M, Ω, Hz; sub M, k, Ω, Hz |
| 17 (16) | ① µ | ① m | ① V | ① A | ① n | ① D% | ① S | ① F | all: main µ, m, V, A, n, %, S, F |
| 18 (17) | don't care | | | | | | | | b3 only: the "mV" variant (§8.1) |
| 19 (18) | Report ID III = `00` | | | | | | | | skipped |
| 20-23 (19-22) | Model ID0-3: `0x82` (BM820); `0x52` (BM520-ML) | | | | | | | | — (live); byte 23 import probe only |
| 24 (23) | [H] | [C] | [R] | [AUTO] | don't care | | | | — |
| 25-27 (24-26) | don't care | | | | | | | | — |

The map encodes no dial position, function code or range code (BM820 p.1)
[KNOWN].

### 6.2 Digits and decimal points

- **Main display**: digits 1-4; points 1P-3P drawn after digits 1-3.
  **Secondary display**: digits 5-8; points 4P-6P drawn after digits 5-7, so
  a secondary point after digit k is labelled (k−1)P. The figure prints the
  points with a capital P (BM820 p.1, Fig 1; BM520-ML p.1 identical)
  [KNOWN].
- **Point NP sits in bit 4 of the byte of the digit it follows** (Table 1)
  [KNOWN]. Bs8252x agrees: 5.4, 6.4, 7.4 put the main point after digit 1,
  2 or 3; 11.4, 12.4, 13.4 the secondary point after sub digit 1, 2 or 3;
  any other combination gives no point (Bs8252x `:21986-22424`) [VENDOR].
  8.4 is dB and 14.4 %4~20mA, not points [KNOWN].
- **Figure typo**: digit 4's middle segment is labelled "1g" in Fig 1 of
  both sheets; Table 1 says 4g (BM820 p.1, BM520-ML p.1) [KNOWN].
- Neither decoder has a fifth digit: the main display is bytes 5-8, the
  secondary bytes 11-14 [VENDOR]. The manual's temperature figures put the
  unit letter in the last digit ("205C", "401F", BM820s manual p.11)
  [KNOWN], where Bs8252x looks for it [VENDOR].

### 6.3 Signs

4.7 (① ▭, left of the main digits) and 9.5 (② ▭, beside the secondary
digits) are the minus signs [INFERRED], on the same evidence as §5.3: their
position, their circled numbers, the manual's drawing (BM820s manual p.1)
and Bs8252x reading them as main and sub "-" (Bs8252x `:21986-22424`)
[VENDOR]. Bs8252x also keeps a separate "sub negative" flag, taken from
12.4, which is the 5P point; its recorder uses that flag (Bs8252x
`:21610`, `:10862`, `:11158`) [VENDOR]; that this is a leftover from the
BM860 decoder, where 12.4 is the sub minus, is [INFERRED]. Community: see
§13.5.

### 6.4 Bar graph

"41 Segments Bar-graph: 60 per second max" (BM820s manual p.19) [KNOWN]. The
figure draws a scale "0" at the left, "6" and "10" at the right, a ▷ and a
"−" marked ③, and ③ appears in no table cell (BM820 p.1, BM520-ML p.1)
[KNOWN]. No bar bit exists in this map at all; neither program draws a bar
[VENDOR]. In EF detection the bar shows the field strength, and while
logging it becomes a swinging pointer (BM820s manual p.12, p.16) [KNOWN];
neither is in the data [INFERRED from the map]. Community: see §13.5.

### 6.5 Annunciators Bs8252x does not read

| Bit | Label | Meaning per the manual |
|---|---|---|
| 3.0, 3.3, 3.1 | MAX, MIN, AVG | REC and CREST readouts (BM820s manual p.13) |
| 3.2 | ⑤ ▭ | the dash of "MAX-MIN" [INFERRED by ⑤]; so MAX, MIN and the dash are separate segments [INFERRED from the map] |
| 3.7, 3.6 | Hi, Lo | none found in the BM820s manual text, 2026-09-27 |
| 4.6, 4.5 | △, % | the "△%" pair left of the main display; △ is relative zero [INFERRED as in §5.5]; "%": none found in the manual text, 2026-09-27 |
| 4.4 | LoZ | lit in AutoCheck (p.7) |
| 4.2 | LPF | none found in the manual text, 2026-09-27 |
| 4.1 | ④ ▭ | the dash between T1 and T2 [INFERRED by ④] |
| 9.4 | @ | none found in the manual text, 2026-09-27 |
| 14.4 | %4~20mA | a %4-20mA function: none found in the BM820s manual, 2026-09-27 (the current functions, p.12, list none); the segment is in the map |
| 15.3 | ② D% | — |
| 24.7-24.4 | [H], [C], [R], [AUTO] | HOLD, CREST, REC, auto-ranging (p.13-14); Recall lights R and C together (p.16) |

[KNOWN] for the labels (BM820 p.1). The disabled Bs8252x reader rejects a
reply with 9.4 (@) set, with both 15.7 and 15.6 (sub m and µ) set, or with
both 17.7 and 17.6 (main µ and m) set (Bs8252x `@ 0x424f75-0x425051`)
[VENDOR]. Community: see §13.5.

## 7. Seven-segment characters

### 7.1 Segment bits

| Map | bit 7 | bit 6 | bit 5 | bit 4 | bit 3 | bit 2 | bit 1 | bit 0 |
|---|---|---|---|---|---|---|---|---|
| BM860s | b | g | c | d | a | f | e | point or annunciator |
| BM820s, BM520s | b | g | c | point or annunciator | a | f | e | d |

[KNOWN] (Table 1 of each sheet). The figures label the segments a top, b
upper right, c lower right, d bottom, e lower left, f upper left, g middle
(BM860 p.1, BM820 p.1, BM520-ML p.1) [KNOWN]. Bs86x masks a digit byte with
`FE` and Bs8252x with `EF` before looking it up [VENDOR].

### 7.2 Character table

None of the sheets has a digit or letter table, 2026-09-26 [KNOWN]. Brymen's
programs have one each; an unlisted pattern becomes `?` (Bs86x
`:109403-109540`; Bs8252x `:22573-22716`) [VENDOR]. The two tables give the
same segments for every character (cross-checked against §7.1)
[INFERRED]. Community: see §13.5.

| Char | Segments | Bs86x byte & `FE` | Bs8252x byte & `EF` | In a sheet's example |
|---|---|---|---|---|
| blank | — | `00` | `00` | yes |
| 0 | a b c d e f | `BE` | `AF` | yes |
| 1 | b c | `A0` | `A0` | yes |
| 2 | a b d e g | `DA` | `CB` | yes |
| 3 | a b c d g | `F8` | `E9` | yes |
| 4 | b c f g | `E4` | `E4` | — |
| 5 | a c d f g | `7C` | `6D` | yes (BM820) |
| 6 | a c d e f g | `7E` | `6F` | yes (BM860) |
| 7 | a b c | `A8` | `A8` | yes (BM860) |
| 8 | a b c d e f g | `FE` | `EF` | yes (BM820) |
| 9 | a b c d f g | `FC` | `ED` | — |
| - | g | `40` | `40` | — |
| L | d e f | `16` | `07` | — |
| C | a d e f | `1E` | `0F` | — |
| F | a e f g | `4E` | `4E` | — |
| E | a d e f g | `5E` | `4F` | — |
| r | e g | `42` | `42` | — |
| n | c e g | `62` | `62` | — |
| o | c d e g | `72` | `63` | — |
| i | c | `20` | `20` | — |
| t | d e f g | `56` | `47` | — |
| b | c d e f g | `76` | `67` | — |
| U | b c d e f | `B6` | `A7` | — |
| P | a b e f g | `CE` | `CE` | — |
| H | b c e f g | `E6` | `E6` | — |
| g | a b c f g | `EC` | `EC` | — |
| A | a b c e f g | `EE` | `EE` | — |
| d | b c d e g | `F2` | `E3` | — |
| y | b c d f g | `F4` | `E5` | — |
| u | c d e | not in the table (`?`) | `23` | — |

The digits in the examples (§9) confirm 0, 1, 2, 3, 5, 6, 7 and 8 from
Brymen's documents; 4, 9 and every letter are [VENDOR] only. "O" has no
entry of its own and draws as 0; "S" draws as 5 [INFERRED: same segments].
"I" and "_" (segment d alone) are in neither table [VENDOR].

### 7.3 Words the manuals show

| Word | Where (manual) [KNOWN] | What the programs' tables make of it |
|---|---|---|
| OL, .OL | open diode or reverse bias, both series (BM860s manual p.10; BM820s manual p.11-12) | "0L"; the recorder treats any value containing "L" as overload (Bs86x sigfix `:1600-1640`, `:1760`) [VENDOR] |
| InEr | Beep-Jack warning (BM860s manual p.14; BM820s manual p.14) | "?nEr" [VENDOR], or "1nEr" if the meter draws I as b c [INFERRED]; how it draws I is [UNVERIFIED] |
| diod | secondary display, diode (BM860s manual p.10; BM820s manual p.11) | renders [VENDOR]; but both programs blank the secondary panel when no secondary unit bit is set (§8.2) [VENDOR] |
| Auto | AutoCheck idle and on the secondary display (BM820s manual p.6) | Bs8252x renders it; Bs86x has no "u" [VENDOR] (the BM860s has no AutoCheck [KNOWN, §11.1]) |
| E.F. | EF ready, BM827s/BM829s (BM820s manual p.12) | E and F render; with more than one point bit set no point code matches and no point is printed [VENDOR]; which digits and points the meter uses is [UNVERIFIED] |
| rE-O, C_Er | power-on self-diagnosis (BM860s manual p.15; BM820s manual p.17) | "rE-0"; "C?Er" [VENDOR] |
| t0.05 … | logging interval (BM820s manual p.15) | renders [VENDOR] |
| LEFt, Strt, PAUS, Cont, StoP | logging (BM820s manual p.15-16) | "LEFt", "5trt", "PAU5", "Cont", "5toP" [VENDOR] |
| P.001-P.999 | Recall session page (BM820s manual p.16) | renders [VENDOR] |
| dashes | EF field strength (BM820s manual p.12) | "-" renders [VENDOR] |

## 8. How Brymen's programs derive the reading

### 8.1 Function words

Each program builds a main word F and a secondary word S by OR-ing codes
for the bits of §5.1 and §6.1 (Bs86x `:111150-111205`; Bs8252x
`:22505-22567`) [VENDOR]:

| Code | Meaning | Bs86x source (main / sub) | Bs8252x source (main / sub) |
|---|---|---|---|
| `0x1` | AC | 4.0 / 12.5 | 3.4 / 9.6 |
| `0x2` | DC | 3.4 / — | 3.5 / 9.7 |
| `0x4` | V | 11.0 / 17.3 | 17.5 / 15.4 |
| `0x8` | F (farad) | 17.5 / — | 17.0 / 15.0 |
| `0x20`, `0x40` | °C, °F | digit 6 shows `C` or `F` / — | digit 4 / digit 8 shows `C` or `F` |
| `0x80` | Ω | 18.4 / — | 16.5 / 16.1 |
| `0x100` | continuity | 13.0 (main) | 9.0 (main) |
| `0x200` | A | 17.7 / 12.2 | 17.4 / 15.5 |
| `0x400` | Hz | 18.0 / 17.2 | 16.4 / 16.0 |
| `0x800` | % (duty) | 18.7 / — | 17.2 / — |
| `0x1000` | S (siemens) | 17.4 / — | 17.1 / 15.1 |
| `0x2000` | dB | 18.1 (main) | 8.4 (main) |
| `0x4000` | T2 | 4.3 / 12.6 | 4.3 / 9.1 |
| `0x8000` | T1 | 4.1 / — | 4.0 / 9.2 |
| `0x200000` | % (%4~20mA) | — / 12.3 | — |
| `0x1000000` | "mV" variant, only when F is `5` or `6` | — | 18.3 |
| bit 31 | low battery | 12.7 | 9.3 |

- **The `0x1000000` variant.** Bs8252x sets it on a main word of AC V or DC
  V when 18.3 is set, a bit the sheet marks "don't care" [VENDOR]. In the
  memory import, the same program maps MRAD's DC mV and AC mV (Bfunction
  `03`, `04`) to `0x1000006` and `0x1000005`, DC+AC mV to plain `7`, and
  the V functions (`01`, `02`) to `5`, `6` and `7` (`FUN_00434c78`, Bs8252x `:21036-21130`, called from
  the import handler; Bs86x `FUN_0042f0e0` `:20537-20630`) [VENDOR].
  So the program takes 18.3 to mean a millivolt function [INFERRED]. What
  the meter sends in 18.3 is [UNVERIFIED].
- The word is masked with `0x7FFFFFFF` before the lookups below [VENDOR].

### 8.2 Units, AC/DC text and panels

Unit lookup, shared by both programs and applied to F and to S (Bs86x
sigfix `:7682-7919`) [VENDOR]:

| Word | Unit shown | Main AC/DC text | Reading [INFERRED] |
|---|---|---|---|
| `0` | "" | | nothing lit |
| `4` | V | | V with neither AC nor DC lit, e.g. diode |
| `5`, `0x1000005` | V | AC | AC V (mV) |
| `6`, `0x1000006` | V | DC | DC V (mV) |
| `7` | V | DC+AC | DC+AC V |
| `0x14` | V | | no decoder bit produces `0x10` |
| `8` | F | | capacitance |
| `0x80` | Ω (see below) | | resistance |
| `0x180` | Ω | | continuity |
| `0x2080` | Ω, prefix panel blank | | dBm reference impedance |
| `0x201`, `0x202`, `0x203` | A | AC, DC, DC+AC | current |
| `0x400` | Hz | | frequency |
| `0x800`, `0x200000` | % | | duty; %4-20mA |
| `0x1000` | S | | conductance |
| `0x2000` | dB | | dBm, with the m prefix |
| `0x4000`, `0x8000`, `0xC000` (+`0x20`/`0x40`) | "" | T2, T1, T1-T2 | temperature, the letter in the digits |
| anything else, e.g. `0x200` (A with neither AC nor DC) | "?" | | |

- The Ω codes return the string "W" and switch the unit panel to the
  "Symbol" font, where W draws as Ω (Bs86x sigfix2 `:111-116`) [VENDOR],
  [INFERRED].
- The secondary AC/DC text knows only AC (`5`, `0x201`) and T1/T2/T1-T2;
  a secondary DC gets no text (Bs86x sigfix `:6420-6575`) [VENDOR].
- Panels: the main value, prefix and unit go to three panels, swapped for dB
  so that it reads "dBm"; the secondary value, prefix and unit go to one
  panel, **blank when S is 0**; "LB" shows in red when bit 31 is set
  (`FUN_004098f8`) [VENDOR].
- T1 with T2 both lit reads as "T1-T2" whether or not the T1–T2 dash (4.2
  on the BM860s, 4.1 on the BM820s) is lit [VENDOR]; the manual's T1 + T2
  selection (T1 on the main, T2 on the secondary) and its T1 − T2 selection
  differ by that dash [INFERRED from BM860s manual p.9].
- A shared legacy table (`Getunitfuncstr`) has further codes neither
  decoder produces: `0x205` "VA", `0x20205` "VAR", `0x40001` "W",
  `0x10800` (Bs86x sigfix `:1174-1300`) [VENDOR].

### 8.3 Prefix

| Program | k | M | m | µ | n | Source |
|---|---|---|---|---|---|---|
| Bs86x main | 18.6 | 18.5 | 18.2 | 18.3 | 17.6 | `:111085-111119` |
| Bs86x sub | 17.1 | 17.0 | 12.1 | 12.0 | — | `:111121-111135` |
| Bs8252x main | 16.7 | 16.6 | 17.6 | 17.7 | 17.3 | `:22434-22470` |
| Bs8252x sub | 16.2 | 16.3 | 15.7 | 15.6 | 15.2 | `:22470-22490` |

[VENDOR]. Exactly one bit must be set; two or more give no prefix
[VENDOR]. The prefix is shown beside the value, not multiplied into it: k
3, M 6, m −3, µ −6 ("u"), n −9 (Bs86x `FUN_004a70e0`, `@ 0x4a7115`)
[VENDOR].

### 8.4 Value string

Digits are looked up (§7.2), the point inserted by the point code (§5.2,
§6.2), and "-" put in front when the minus bit is set (§5.3, §6.3)
[VENDOR]. The string is shown as it is; the recorder converts it to a
number unless it holds "L" (overload), stripping a trailing `C` or `F`
first (Bs86x sigfix `:1600-1640`, `:1760`) [VENDOR]. OL therefore needs no
flag: it is the characters the meter lights [INFERRED].

---

## 9. Worked examples

The sheets' examples, decoded through their own Table 1 and through the
programs' decoders by hand [INFERRED]. `xx` is "don't care" (BM860 p.1,
BM820 p.1).

### 9.1 BM860: "AC 312.17V / 60.11Hz"

Caption and hex column (BM860 p.1) [KNOWN]:

```
00 xx 01 11 F8 A0 DA A9 A0 00 00 00 7E BF A0 A0 04 00 00 xx xx xx 86 xx xx xx xx
```

| Bytes | Through Table 1 |
|---|---|
| 3 = `01` | AUTO |
| 4 = `11` | bar scale, AC ① |
| 5-9 = `F8 A0 DA A9 A0` | digits 3, 1, 2, 7, 1; 8.0 = 3p |
| 11 = `00` | digit 6 blank; V ① **not** set |
| 12 = `00` | nothing on the secondary side |
| 13-16 = `7E BF A0 A0` | digits 6, 0, 1, 1; 14.0 = **7p** |
| 17 = `04` | Hz ② |
| 18 = `00` | no main unit |
| 23 = `86` | model |

Main **312.71**, secondary **6.011** Hz. Bs86x would show "312.71" with
unit "?" (F = `0x1`, AC alone) and "6.011 Hz" [INFERRED from §8].

Three inconsistencies in the sheet [KNOWN; the figure checked at 800 dpi by
the protocol reader and again for this spec]:
1. The caption says 312.17; the hex and the highlighted figure show
   3-1-2-7-1 with 3p, 312.71.
2. The caption and figure light the main V; 11.0 is `0` and its cell is not
   highlighted.
3. The hex sets 7p (after digit 7), and the table highlights 7p; the example
   figure lights **8p**, giving the caption's 60.11. Fig 1 and Bs86x put 7p
   after digit 7, so the hex and the figure disagree, not the labels
   [INFERRED]. Community: see §13.5.

The figure lights 26 bar pointers, about 3.13 of the 0-5 scale, with no bit
in the data [INFERRED, count by the protocol reader].

### 9.2 BM820 and BM520-ML: "AC 380.1V / 50.12Hz"

BM820 p.1 (BM520-ML p.1 is the same with `52` for `82`) [KNOWN]:

```
00 xx 10 00 E9 EF BF A0 00 6D 00 BF A0 CB 00 01 20 xx 00 82 82 82 82 1x xx xx xx
```

Through the example's own table (digit 5 at byte 10, report ID II at byte
11), which is self-consistent and matches its figure [KNOWN]:

| Bytes | Decode |
|---|---|
| 3 = `10` | AC ① |
| 5-8 = `E9 EF BF A0` | 3, 8, 0 with 3P, 1 → **380.1** |
| 9 = `00` | nothing |
| 10 = `6D` | 5 |
| 11 = `00` | report ID II |
| 12-14 = `BF A0 CB` | 0 with 5P, 1, 2 → **50.12** |
| 16 = `01` | Hz ② |
| 17 = `20` | V ① |
| 20-23 = `82` | model |
| 24 = `1x` | AUTO |

Through Bs8252x, which takes byte 10 as the report ID and bytes 11-14 as the
secondary digits (§4.3) [INFERRED from §6 and §8]: main "380.1 V" AC, as
the sheet says; secondary `00 BF A0 CB` → blank, 0 with 5P, 1, 2 →
" 0.12 Hz". 12.4 is set, so the recorder's "sub negative" flag (§6.3) is
set too. With digit 5 moved to byte 11 as BM820 Table 1 has it, the
program would read "50.12 Hz".

---

## 10. Logged-memory download (BM521s, BM525s)

### 10.1 Commands

| Name (sheet) | Bytes | Remark (MRAD p.1) |
|---|---|---|
| Cs_HMD, "request Head of Memory Data sets" | `00 00 52 88` | "Reset Memory Pointer to "A0_007Ah" and read 1'st "24 bytes" data with "Memory Pointer + 1" step" |
| Cs_NMD, "request Next Memory Data set" | `00 00 52 89` | "Read "24 bytes" with "Memory Pointer + 1" step" |
| Cs_CMD, "request Current Memory Data set again" | `00 00 52 8A` | ""Memory Pointer - 24" and then read "24 bytes" with "Memory Pointer + 1"" |

[KNOWN] (BM520-ML p.1, MRAD p.1). Each returns "27 bytes + 9 bytes". No BM860
or BM820 sheet mentions logging commands, 2026-09-26 [KNOWN]. The manual
does not describe the download (BM820s manual, none found, 2026-09-26)
[KNOWN]. README-8252x §3-2-1-4: "Simply turn the meter power on to any
function (except capacitance function) and start this function" [KNOWN].

### 10.2 The 36-byte reply

MRAD p.1-2 [KNOWN]:
- Bytes 1, 10, 19, 28: `00` report ID, "useless byte for data decoding".
- Bytes 2-9, 11-18, 20-27: 24 memory bytes in address order, "Useful data
  bytes … Put them as Table 2".
- Bytes 29-30: "Checksum byte0 / Checksum byte1 = Sum of returned byte No.
  1 ~ 27", "useful for verifying data tansmission only". The report-ID bytes
  in that range are `00`, so this is the sum of the 24 data bytes
  [INFERRED]. Byte order: not stated in MRAD.
- Bytes 31-36: "don't care".

Both programs compare the 16-bit sum of the 24 data bytes with byte 29 +
byte 30 × 256: byte 29 low, byte 30 high (Bs86x `@ 0x422dcc-0x422de4`)
[VENDOR]. Unlike the live path, byte 2 is data here (Model_Id, §10.5)
[KNOWN], and the programs read it (R[1..8], R[10..17], R[19..26]) [VENDOR].
Community: see §13.7.

### 10.3 MRAD's flowchart

MRAD p.4 [KNOWN]: open VID 0x82 / PID 0x01; allocate "4 Input Reports (36
bytes)"; send `…52 88`; wait for all 4 reports, closing and reopening after
more than 1 s; on a bad checksum close and reopen; "Allocate useful bytes as
Table. 2"; close and reopen if a 1.6 s counter has run out, otherwise "Wait
0.001 second". Then loop: send `…52 89`; on a > 1 s timeout or a bad
checksum send `…52 8A`; allocate; if the 1.6 s counter has run out, close
the device and restart from the top (reopen and send `…52 88` again),
otherwise wait 0.001 s; "Last data?" — no: `…89` again; yes: close and
"Decode (see Table 2)".

### 10.4 What the programs do

[VENDOR], both programs, summary level:
- **Trigger.** Bs86x: the Graphical Recorder's import button (`@ 0x41b6e8`);
  Bs8252x: the FormImport button (`@ 0x42eb68`). A confirm dialog comes
  first (strings 8010, 6001).
- **Gate.** Send `00 00 82 66` and read byte 23 of the reply, mapped as in
  §3.4; the import goes on only if it is `52`, otherwise E9017 (Bs86x
  sigfix `:2237-2330`; Bs8252x sigfix `:2843-3100`, `@ 0x42ecb3`). So even
  Bs86x imports only from a BM520s meter.
- **Sequence.** `52 88` for the first 24 bytes; its data[1..3] is a 24-bit
  little-endian total length and data[4..5] a 16-bit little-endian count;
  then `52 89` per further 24-byte block until the total, rounded up to a
  multiple of 24, is read; a failed block is asked again with `52 8A`.
- **Retries.** `52 88`, both programs: a bad checksum reopens and resends
  without limit, a timeout allows 3 attempts (Bs86x
  `@ 0x422de2-0x422e19`; Bs8252x `@ 0x426de0-0x426e1e`). `52 89`: Bs86x
  resends `8A` on a bad checksum without limit and on a timeout up to 3
  attempts (`@ 0x422f71-0x422f9b`); Bs8252x treats a bad checksum as a
  timeout, so both count towards 4 attempts (`@ 0x426f7b-0x426fa9`).
- **Timing.** Bs86x: 2000 ms for the write, 1000 ms per 9-byte read, no
  pause. Bs8252x: 500 ms, 500 ms, and a 200 ms Sleep after `52 88`
  (`@ 0x426d0c`).
- **Records.** A 3-byte function header whose third byte's bit 7 means dual
  display and whose low nibble indexes an interval table of 0.05, 0.1, 0.5,
  1, 2, 3, 4, 5, 10, 15, 30, 60, 120, 180, 300 and 600 s (Bs86x
  `@ 0x42ecd9-0x42edd7`, doubles in the disassembly; the decompile
  `FUN_0042ec78` shows empty cases), then a 24-bit little-endian byte
  count, then 3-byte samples (6 when dual). In a sample the first byte's
  bit 6 is OL (shown "----" for temperature), bit 7 negative, low nibble
  the range, which the program turns into a decimal-point position per
  function (`FUN_0042ede4`, Bs86x sigfix `:4640-4700`); bytes 2-3 are read as one 16-bit little-endian integer and
  formatted with "%04.4i" (Bs86x sigfix `:2333-3700`, `:3031`). The exact
  chaining of records is garbled in the decompile [UNVERIFIED].

The same interval list is MRAD's Table 3-1 (MRAD p.8) [KNOWN]. The
programs' function mapping follows MRAD Table 3 (§8.1).

### 10.5 MRAD's tables

**Table 2, memory map** (MRAD p.5-7), header at A0_007Ah [KNOWN]:

| Address | Field | Printed meaning |
|---|---|---|
| A0_007Ah | Model_Id | "DMM Model ID: = 00h or 01h"; which model is which is not stated |
| A0_007B-7Dh | TotalBytes_0-2 | "Total bytes of logged data" |
| A0_007E-7Fh | TotalSession_0-1 | "Total logged Session Pages (MAX: 999 Session Pages)" |
| A0_0080-81h | DLE `EE`, STX `A0` | "Head of each session identifier" |
| A0_0082-84h | PS1_Addr0-2 | head address of the previous session page |
| A0_0085-87h | NS1_Addr0-2 | head address of the next session page; "NS1_Addr = (NS1_Addr2, NS1_Addr1, NS1_Addr0)" |
| A0_0088-8Ah | Bfunction, Bselect, Bstatus | Table 3 |
| A0_008B-8Dh | SP1_Length0-2 | first session page data length |

Then data sets of 6 bytes (ML_0-2, SL_0-2, dual) or 3 (ML_0-2, single); a
page ends with DLE `EE`, ETX `C0`, and the next page repeats the header
from DLE/STX. Suffix `_0` is the low byte: the programs read the lengths so
[VENDOR], and NS1_Addr2 is the most significant by MRAD's notation
[INFERRED]. Community: see §13.7.

**Table 3, function encoder** (MRAD p.8) [KNOWN]:

| Bfunction | Bselect → main (+ second) display |
|---|---|
| `01` | `00` ACV (+Hz); `01` Hz (+ACV) |
| `02` | `00` DCV; `01` DCV (+ACV); `02` DCV+ACV (+ACV) |
| `03` | `00` DCmV; `01` DCmV (+ACmV); `02` DCmV+ACmV (+ACmV); `03` Hz; `04` Duty |
| `04` | `00` ACmV (+Hz); `01` Hz (+ACmV) |
| `05` | `00` OHM; `01` Continuity; `02` nS |
| `06` | `00` T1; `01` T2; `02` T1 (+T2); `03` T1-T2 (+T2) |
| `07` | `00` Capacitance; `01` Diode |
| `08` | `00` DC; `01` DC (+AC); `02` DC+AC (+AC); `03` AC (+Hz) — A or mA by Bstatus bit 5 |
| `09` | `00` DCµA; `01` DCµA (+ACµA); `02` DCµA+ACµA (+ACµA); `03` ACµA (+Hz) |

Bstatus: bit 7 "0" single, "1" dual display; bit 5 "0" mA, "1" A (mA and A
only); bit 4 "0" °C, "1" °F (temperature only); low nibble the logging
interval, Table 3-1 [KNOWN]. An AutoCheck, dBm or EF code: none in Table 3,
2026-09-27 [KNOWN]; the BM521s and BM525s have AutoCheck (BM820s manual
p.6), so what a session logged in AutoCheck carries is [UNVERIFIED].
Community: see §13.7 D3.

**Tables 4 and 5, data sets** (MRAD p.9) [KNOWN]: ML_0 bit 7 "+"/"-", bit 6
OL, bit 5 LB ("logged at low-battery status"), bit 4 x, bits 3-0 MLRange;
SL_0 the same with bit 5 x; ML_1 and SL_1 hold D1 (bits 7-4) and D0 (bits
3-0), ML_2 and SL_2 D3 and D2. MRAD does not say whether D3-D0 are decimal
digits or the nibbles of a binary count; the programs read ML_2:ML_1 as one
binary integer (§10.4) [VENDOR]; which the meter stores is [UNVERIFIED].
Community: see §13.7.

**Table 6, range bits** (MRAD p.9) [KNOWN]:

| Function | Ranges by code `0000`, `0001`, … |
|---|---|
| ACV, DCV, ACV+DCV | 9.999V, 99.99V, 999.9V, 9999V ("\*not actual hardware range, software range only") |
| ACmV, DCmV, ACmV+DCmV | 60.00mV, 600.0mV |
| Hz | 9.999Hz, 99.99Hz, 999.9Hz, 9.999kHz, 99.99kHz, 999.9kHz, 9.999MHz |
| Duty | 99.99%, 100.0% |
| Ω | 600.0Ω, 6.000kΩ, 60.00kΩ, 600.0kΩ, 6.000MΩ, 60.00MΩ |
| continuity; nS; DIODE | 600.0Ω; 99.99nS; 2.000V |
| CAP | 60.00nF, 600.0nF, 6.000µF, 60.00µF, 600.0µF, 6.000mF, 25.00mF |
| ACA, DCA, ACA+DCA | 6.000A, 60.00A |
| ACmA, DCmA, ACmA+DCmA | 60.00mA, 600.0mA |
| ACµA, DCµA, ACµA+DCµA | 600.0µA, 6000µA |

Temperature has no row [KNOWN]. The manual's top current range is 10.00A,
not 60.00A (BM820s manual p.22) [KNOWN]; which the meter logs is
[UNVERIFIED].

### 10.6 MRAD's inconsistencies

[KNOWN], read from the pages; each is [UNVERIFIED] until a meter answers:
- the memory-pointer numbers differ between p.1 (2nd ML_0 = 26) and p.5
  (2nd ML_0 = 20);
- addresses A0_00A1h-A6h repeat for the 4th-7th data sets on p.1 and p.5;
- Table 2's bit header lacks "Bit3";
- the function cross-reference reads "see Table 3" on p.5-6 and "see
  Table 2" on p.7;
- "2'nd Session Page Data Length" is reused for later pages;
- the checksum byte order is not stated (the programs: byte 29 low).

---

## 11. Meter behaviour (manuals)

Functions, keys and displays a test checklist needs, condensed; the figures
behind each row are on the cited pages.

### 11.1 Functions by model

| Function | 867s | 869s | 821s | 822s | 827s | 829s | 521s | 525s |
|---|---|---|---|---|---|---|---|---|
| AC sensing | TRMS AC, AC+DC | TRMS AC, AC+DC | average | AC TRMS | AC TRMS | AC+DC TRMS | AC+DC TRMS | AC+DC TRMS |
| VFD ACV + Hz | — | ✓ | — | — | — | — | — | — |
| dBm | ✓ | ✓ | — | — | — | ✓ | — | — |
| %4-20mA (sub, DC mA) | ✓ | ✓ | — | — | — | — | — | — |
| AutoCheck (LoZ) | — | — | — | — | — | ✓ | ✓ | ✓ |
| DC+ACV (+ACV), DC+ACmV (+ACmV) | ✓ | ✓ | — | — | — | ✓ | ✓ | ✓ |
| nS | ✓ | ✓ | — | — | ✓ | ✓ | ✓ | ✓ |
| Temperature | — | T1, T2, T1−T2 | — | — | T1 | T1, T2, T1−T2 | T1 | T1, T2, T1−T2 |
| EF detection | — | — | — | — | ✓ | ✓ | — | — |
| REC MAX/MIN/AVG | ✓ | ✓ | — | — | ✓ | ✓ | no AVG | no AVG |
| CREST | ✓ | ✓ | — | — | ✓ | ✓ | ✓ | ✓ |
| 500000-count DCV | ✓ | ✓ | — | — | — | — | — | — |
| Data logging | — | — | — | — | — | — | 5400 dual / 10800 single | 43500 dual / 87000 single |

Sources: BM860s manual p.4, p.6, p.9-10, p.13, p.17-18; BM820s manual p.6-15,
p.19 [KNOWN]. The BM860s rows without a model tag in the manual are taken
for both models, and the 821s/822s lack of REC and CREST is read from their
absence in every "only" list [INFERRED]. The backlight is "Models 525s,
521s, 829s only" (BM820s manual p.13) [KNOWN], so the 821s, 822s and 827s
have none. Whether the 821s and
822s offer DC+AC current is not addressed (BM820s manual p.12)
[UNVERIFIED].

### 11.2 BM860s dial (BM869s drawn, BM860s manual p.1)

| Position | SELECT order (main + secondary) | Pages |
|---|---|---|
| VFD Hz Ṽ (869s) | VFD ACV + Hz ⇄ VFD Hz + ACV; manual 500V range by default; low-pass filter on | p.6 |
| Hz dBm Ṽ | ACV + Hz → dBm + Hz → Hz + ACV; dBm first shows the reference impedance for 1 s ("600", Ω and dBm lit) | p.7 |
| V⎓ | DCV → DCV + ACV → DC+ACV + ACV | p.8 |
| mV⎓, Hz, D% | DCmV → DCmV + ACmV → DC+ACmV + ACmV → logic Hz → Duty % | p.8 |
| Hz dBm mṼ | ACmV + Hz → dBm + Hz → Hz + ACmV | p.9 |
| T1 T2 (869s) | SELECT °C ⇄ °F; RANGE (T1-T2) cycles T1 → T2 → T1 (+T2) → T1−T2 (+T2) | p.9 |
| diode / capacitance | 869s: SELECT Cap ⇄ Diode (secondary "diod"); 867s: separate positions | p.10 |
| nS Ω •))) | Ω → continuity → nS | p.11 |
| A mA ⎓ Hz | DC (+%4-20mA on mA) → DC + AC → DC+AC + AC → AC + Hz | p.11-12 |
| µA ⎓ Hz | same sequence | p.11 |

[KNOWN]; the last SELECT choice at each position is its power-up default
(p.6-12). The BM867s dial is not drawn [UNVERIFIED]. The VFD heading lists
ACV + Hz first and the figure Hz + ACV first (p.6) [UNVERIFIED].

### 11.3 BM820s/BM520s dial (BM829s and a logging model drawn, BM820s manual p.1, p.4)

| Position | SELECT order | Pages |
|---|---|---|
| Auto Check (below OFF; 829s, 521s, 525s) | "Auto" when idle; DCV, ACV or Ω chosen from the input, "Auto" on the secondary, LoZ lit; RANGE or SELECT locks | p.6-7 |
| Hz Ṽ (829s: Hz dBm Ṽ) | ACV + Hz → dBm + Hz (829s) → Hz + ACV | p.7 |
| V⎓ | DCV → DCV + ACV → DC+ACV + ACV (525s, 521s, 829s) | p.8 |
| mV⎓, Hz, D% | DCmV → DCmV + ACmV → [DC+ACmV + ACmV] → logic Hz → Duty % | p.8-9 |
| Hz mṼ (829s: + dBm) | ACmV + Hz → dBm + Hz (829s) → Hz + ACmV | p.9 |
| nS Ω •))) | Ω → continuity → nS; 821s/822s have separate Ω and continuity positions | p.10 |
| T1 T2 | SELECT °C ⇄ °F; RANGE cycles T1, T2, T1 (+T2), T1−T2 (+T2) on dual-channel models | p.10-11 |
| capacitance / diode | Cap → Diode | p.11 |
| A mA ⎓ Hz | DC → DC + AC → DC+AC + AC → AC + Hz | p.12 |
| µA ⎓ Hz | same sequence | p.12 |

[KNOWN]. The 821s, 822s and 827s dials are not drawn [UNVERIFIED].

### 11.4 Keys

| Key | Short press | Hold ≥ 1 s | At power-on |
|---|---|---|---|
| SELECT | next function at this position; logging models: live ⇄ item number | backlight (BM867s, BM869s; BM829s, BM521s, BM525s) | disable APO until OFF |
| RANGE | manual range, next range (AUTO off); dBm: next reference Ω; T: T1/T2 cycle; BM860s Hz: trigger level | auto-range | beeper off until OFF |
| Δ | relative zero on the main reading | BM860s: 50000 ⇄ 500000 counts (single-display DCV, 1.25/s) | — |
| HOLD | hold | BM827s/829s: EF detection ("E.F.") | — |
| REC | MAX/MIN/AVG record; presses step MAX, MIN (MAX-MIN, AVG) | exit | — |
| CREST | 1 ms peak ("C", "MAX"); presses step MAX, MIN (MAX-MIN) | exit | — |
| Yes ▲ (logging models) | new session; pause/continue ("PAUS"/"Cont"); Recall: up | start logging ("LEFt"); stop ("StoP") | — |
| ▼ Erase (logging models) | at the start prompt: erase all, start at P.001; Recall: down | show/set the interval ("t0.05") | — |
| ▲ + ▼ | Recall (page "P.nnn" for 0.5 s, "R" and "C" lit); again: next page | fast page scroll | — |

BM860s manual p.13-14; BM820s manual p.12-17 [KNOWN]. REC and CREST disable
APO in both series; REC keeps the BM860s ranging choice and auto-ranging on
the BM820s (BM860s manual p.13; BM820s manual p.13) [KNOWN].

### 11.5 Ranges and counts

| | BM860s (BM860s manual p.17, p.19-22) | BM820s/BM520s (BM820s manual p.19-23) |
|---|---|---|
| Counts | 50000 fast; 500000 stable DCV; 99999 Hz | 9999 ACV, DCV, Hz, nS; 6000 mV, µA, mA, A, Ω, capacitance |
| DC/AC V | 500.00mV, 5.0000V, 50.000V, 500.00V, 1000.0V | 60.00mV, 600.0mV; 9.999V, 99.99V, 999.9V |
| VFD V | 5.0000V-1000.0V, 10-440 Hz (869s) | — |
| Ω | 500.00Ω-50.000MΩ (6 ranges) | 600.0Ω-60.00MΩ (6 ranges) |
| nS | 99.99nS | 99.99nS |
| Continuity beep | 20-200 Ω | 20-300 Ω |
| Diode | 2.0000V | 2.000V |
| Capacitance | 50.00nF-25.00mF (7 ranges) | 60.00nF-25.00mF (7 ranges) |
| Current | 500.00µA, 5000.0µA, 50.000mA, 500.00mA, 5.0000A, 10.000A | 600.0µA, 6000µA, 60.00mA, 600.0mA, 6.000A, 10.00A |
| Logic Hz | 5.000Hz-1.0000MHz | 5.00Hz-1.000MHz |
| Duty | 0.1-99.99 % | 0.00-100.0 % |
| Temperature | −50.0 to 1000.0 °C, −58.0 to 1832.0 °F | −50 to 1000 °C, −58 to 1832 °F |
| dBm references | 4-1200 Ω, 20 values | the same list (829s) |
| %4-20mA | 4 mA = 0 %, 20 mA = 100 %, 0.01 % | — |

[KNOWN].

### 11.6 Messages and APO

- Messages: see §7.3. Open-thermocouple indication, "OPEn" or "E" codes:
  none found in either manual, 2026-09-26 [KNOWN].
- Low battery: below about 7 V (BM860s manual p.17; BM820s manual p.19)
  [KNOWN]; logging stops when it shows (BM820s manual p.16) [KNOWN].
- APO: about 17 minutes (BM860s manual p.14, p.17), about 30 minutes
  (BM820s manual p.14, p.19); activity is dial or key operation or a
  significant reading; SELECT at power-on disables it until OFF [KNOWN].
  Linked to a PC it is disabled (README-86x §2-5) [KNOWN]; a statement on
  PC communication and APO: none found in either manual, 2026-09-26.
  Community: see §13.4.
- Logging at intervals of 30 s or more enters a 50 % power-down mode about
  4.2 min after the start (BM820s manual p.16) [KNOWN]; what a real-time
  request gets meanwhile is [UNVERIFIED].

---

## Implementation Notes

What the wire requires of any decoder:

- The device is HID, VID:PID 0x0820:0x0001 as Brymen's programs match it
  (the sheets print 0x82/0x01). Requests are one 3-byte output report,
  `00 cc 66` with cc the series code; the reply is three 8-byte input
  reports, 24 data bytes, which Windows presents as 27 bytes with a `00`
  before each report [INFERRED report shape, §2].
- The reply is LCD segments. The value is rebuilt from 7-segment digits,
  one decimal-point bit, a minus segment per display and unit and prefix
  annunciators; OL is the letters "0L" on the digits.
- Two maps: the BM860s map (byte 23 = `86`, segments b g c d a f e in bits
  7-1) and the BM820s/BM520s map (byte 23 = `82` or `52`, segments b g c a f
  e d in bits 7-5 and 3-0, the point in bit 4).
- Point Np is in the next digit's byte on the BM860s and in its own digit's
  byte on the BM820s/BM520s; in both, Np is drawn after digit N.
- Temperature units come as a `C` or `F` in the last main digit (and the
  last secondary digit on the BM820s/BM520s), not as annunciators.
- No function or range code, no bar-graph bits, no checksum and no sequence
  number in either Table 1; that no byte carries the bar is [UNVERIFIED]
  (§12.15).
- Memory replies are four reports, 24 data bytes and a 16-bit sum of them,
  low byte first as Brymen's programs read it.

## 12. Open questions — [UNVERIFIED]

Each question comes from Brymen's sources. Its "Community:" note, where
there is one, summarises §13 [COMMUNITY] with the strength of the evidence;
"settled" there means community evidence, not a meter on our bench, so
every item stays on this list.

1. **VID:PID**: 0x0820:0x0001 (programs) or 0x82/0x01 as printed (sheets)
   (§2). Community: settled toward 0x0820:0x0001 (code+hw, three
   projects, and DawOp's code), §13.2.
2. **Report shape on the wire**: 8-byte input and 3-byte output reports,
   unnumbered; the report descriptor (§2). Community: the shape is settled
   (code+hw, one capture); the descriptor stays open, §13.2.
3. **Report II position** in a BM820s/BM520s reply: byte 10 (BM820 Table
   1, MRAD, the program) or byte 11 (BM820 example, BM520-ML) (§4.3).
   Community: settled toward byte 10 (code+hw, BM525s and BM829s), §13.2.
4. **BM52x real-time request**: whether a BM521s/BM525s answers `52 66`
   (its sheet), `82 66` (Brymen's programs) or both, and with which byte 23
   (§3.1). Community: narrowed; a BM525s answers `52 66` with `52` in bytes
   20-23 (code+hw); `82 66` is in dispute, §13.3 D1.
5. **Cross-series requests**: what a BM860s does with `82 66` (Bs86x's
   import probe sends it) and a BM820s with `86 66` (§3.1). Community: none
   found in these sources, 2026-09-27; the series code is the cable's to
   interpret [INFERRED], §13.6.
6. **Cable name for the BM820s/BM520s**: BU-86X/BC-86X (manual p.20,
   READMEs) or BU-82X (manual p.13) (§1.2). Community: settled toward the
   BU-86X (code+hw), §13.3.
7. **Timing**: whether a reply waits for a fresh measurement; the reply rate
   against the 5/s, 1.25/s and 20/s display rates; whether the device must
   be reopened per reading as the sheets' flowchart does, or can be polled
   continuously as the programs do; capacitance replies beyond 4 s (§3.2,
   §3.3). Community: narrowed; no reopen is needed (code+hw), and about 5
   replies a second [INFERRED from capture timestamps]; the 1.25/s and
   20/s modes and capacitance stay open, §13.4.
8. **No meter, meter off, APO**: what the cable sends without a meter; the
   byte 23 values the programs map to `82`; whether linking really disables
   APO (§3.4, §11.6). Community: narrowed; nothing with the meter off
   (comment, [INFERRED]); APO in dispute (§13.4 D2); the byte 23 values
   stay open.
9. **Model bytes**: bytes 20-22 on a BM860s; whether any byte tells models
   within a series apart (§4.2). Community: bytes 20-22 settled as `86`
   (captures); telling models apart stays open, §13.3.
10. **BM860 secondary point**: the example's hex (7p, "6.011") against its
    figure (8p, "60.11"); the main V bit absent in the example (§9.1).
    Community: narrowed; the BU-86X decoders follow Table 1 (code+hw); the
    captures fit it but do not record the LCD; the V bit stays open, §13.5.
11. **500000-count mode**: how the six main digits and the points are used
    (§5.2).
12. **Minus segments**: 4.7 and 12.4 (BM860s), 4.7 and 9.5 (BM820s/BM520s)
    identified by position and program use (§5.3, §6.3). Community:
    corroborated (code+hw, one capture), §13.5.
13. **Byte 18 bit 3** on the BM820s/BM520s: the programs' "mV" variant,
    "don't care" in the sheet (§8.1).
14. **Unexplained segments**: △ (5.0; 4.6), "%" (4.5), Hi/Lo, LPF, @, the
    ③/④/⑤ dashes, the BM860s "bar scale" bit and the small "1" beside 1g
    (§5.3-§5.5, §6.5). Community: narrowed; △ as relative zero and ⑤ as
    the MAX-MIN dash (code+hw); ③ as the T1-T2 dash (code+hw, one
    project); "%", Hi/Lo and @ reported unsupported on a BM525s (comment);
    LPF, ④, bar scale and the "1" stay open, §13.5.
15. **Bar graph**: that no byte carries it, in any mode (§5.4, §6.4).
    Community: no data; the two BM860s captures are near-zero readings with
    bar-scale off, so they don't show where the bar is, §13.5.
16. **Glyphs**: how the meter draws I (InEr), _ (C_Er), the points of
    "E.F." and the EF dashes (§7.3). Community: InEr is shown on a BM525s
    (comment); the glyphs stay open, §13.5.
17. **Don't-care bytes**: what the BM860s sends in bytes 2, 20-22 and
    24-27, and the BM820s/BM520s in bytes 2, 18, 24 bits 3-0 and 25-27
    (§4.1, §5.1, §6.1). Community: BM860s narrowed; byte 2 `00` (`FF` in
    resistance, comment), 20-22 `86` and 24-27 `00` (captures); the
    BM820s/BM520s bytes stay open, §13.3.
18. **T1 + T2 against T1 − T2**: the dash bit as the only difference (§8.2).
    Community: the projects disagree; open, §13.5.
19. **Memory download**: checksum byte order on the meter; D3-D0 as digits
    or a binary count; MRAD's pointer and address inconsistencies; the
    Model_Id values 00h/01h; the 60.00A range against the manual's 10.00A;
    AutoCheck sessions; behaviour in capacitance and during 50 % power-down
    (§10). Community: narrowed; checksum low byte first, TotalBytes from
    Model_Id, addresses low byte first and Model_Id `01` on a BM525s
    (capture); the rest stays open, §13.7.
20. **Manual gaps**: the BM867s, 821s, 822s and 827s dials; DC+AC current
    on the 821s/822s; the VFD SELECT order; the BM820s manual's "1V range"
    Hz sensitivity (p.7), a range the meter does not have (§11).

---

## 13. Cross-reference with community sources [COMMUNITY]

Read 2026-09-27, after §1-12 were written from Brymen's sources,
grounding-checked and committed; nothing here was merged into §1-12 beyond
pointers. The boundary covered code repositories and TestController's
supported-equipment list, no forums (`reverse-engineering-approach.md`).
Community paths below are relative to `references/bm86x/community/`, at the
commits of §13.1.

How the sources were made matters more than how many agree:

- **Most restate the BM860 sheet.** The copy DawOp ships
  (`Brymen869s-XmlLib/Docs/500000count-professional-dual-display-DMMs-protocol.pdf`)
  is byte-identical to our BM860 sheet (same SHA-256); sigrok names the
  BM860 sheet and the BM520s zip as its sources
  (`libsigrok/src/dmm/bm86x.c:24-29`, `bm52x.c:24-29`); 869log thanks
  Brymen for it (`869log/README.md:12`); freedaun credits DawOp
  (`Brymen-BM869s/README.md:41`). Where their LCD maps match §5-§6 they
  restate Table 1; what they add is that the code was run against meters.
- **Two ways in.** sigrok, TheHWcave, freedaun and DawOp talk to a real
  BU-86X (hidapi, or the AHid library). 869log and MartinD-CZ replace the
  cable with a DIY board on the meter's infrared port: the only view below
  the cable.
- **Three captures.** Bytes a meter sent, at the commits of §13.1: one live
  BM869s reply in a code comment (`Brymen-BM869s/brymen-BM869s.py:295`);
  one optical frame from a BM867/869, printed raw six times, with
  timestamped decoded lines (`brymen-867-interface-cable/console.png`); and
  an excerpt of one memory download (`libsigrok/src/dmm/bm52x.c:556-592`),
  from the BM525s the file names as its test meter. None records what the
  LCD showed.
- **Meters named**: BM869s (TheHWcave, freedaun, DawOp, 869log), BM867/869
  (MartinD-CZ), BM525s and BM829s (sigrok, `bm52x.c:31-39`). A BM867s,
  821s, 822s, 827s or 521s named as tested: none found in these sources,
  2026-09-27.

Strength, per finding: **capture** (bytes above), **code+hw** (code its
author ran on a named meter), **code** (no hardware claim for that point),
**comment** (the author's prose).

### 13.1 Sources

| Source | Commit | What it is | Link to the meter | Licence |
|---|---|---|---|---|
| [sigrokproject/libsigrok](https://github.com/sigrokproject/libsigrok), sparse: `src/serial_hid_bu86x.c`, `src/serial_hid.c`, `src/dmm/bm86x.c`, `src/dmm/bm52x.c`, `src/hardware/serial-dmm/`, `README.devices`, `NEWS` | `0bc2487` (2025-11-20) | C drivers brymen-bm86x, brymen-bm52x, brymen-bm82x | BU-86X, hidapi | GPL-3.0-or-later |
| [TheHWcave/BM869S-remote-access](https://github.com/TheHWcave/BM869S-remote-access) | `b4d6aa4` (2021-11-05) | Python class and logger | BU-86X, hidapi | MIT |
| [freedaun/Brymen-BM869s](https://github.com/freedaun/Brymen-BM869s) | `ccd693d` (2021-07-09) | Python logger, up to two meters | BU-86X, hidapi | none stated |
| [DawOp/Brymen869s-XmlLib](https://github.com/DawOp/Brymen869s-XmlLib) | `516592c` (2023-05-08) | C++ DLL, readings to XML | BU-86X, AHid | MIT |
| [kittennbfive/869log](https://github.com/kittennbfive/869log) | `f612bdf` (2024-09-19) | ATtiny25 firmware and a Linux decoder | DIY infrared board | AGPL-3.0+ (code) |
| [MartinD-CZ/brymen-867-interface-cable](https://github.com/MartinD-CZ/brymen-867-interface-cable) | `8ea4f0b` (2021-04-28) | ATtiny45 firmware | DIY infrared board | GPL-3.0 (LICENSE file); the `main.cpp` header says CC BY-SA 4.0 |
| TestController supported equipment, `testcontroller-supported-equipment.html` | fetched 2026-09-27 | model list | not stated | — |

### 13.2 USB, reports and reply layout

| Spec § | Community | Evidence | Verdict |
|---|---|---|---|
| §2 VID:PID | 0x0820:0x0001 in all four BU-86X projects (`libsigrok/src/serial_hid_bu86x.c:56-59`; `BM869S-remote-access/BM869S.py:34-35`; `Brymen-BM869s/brymen-BM869s.py:196-198`; `Brymen869s-XmlLib/Source/Brymen869/Brymen869.cpp:19-20`) | code+hw (sigrok, TheHWcave, freedaun); code (DawOp) | agrees with the programs |
| §2 strings | the enumerated cable prints as "Brymen Superior DMM", manufacturer and product string joined (`Brymen-BM869s/README.md:14-15`; `brymen-BM869s.py:246`); which part is which: none found in these sources, 2026-09-27 | comment (program output) | new: the cable has string descriptors |
| §2 report shape | "only report number 0 is involved, which carries a mere byte stream in 8 byte chunks each" (`libsigrok/src/serial_hid_bu86x.c:22-25`); reads of at most 8 bytes (`:54`, `:66-68`); the request goes to `hid_write` as `00 00 86 66`, report ID 0 and three bytes (`bm86x.c:43`; `libsigrok/src/serial_hid.c:572-585`; `BM869S.py:101`, `:236`); DawOp writes three bytes (`Brymen869.cpp:338-340`). The freedaun capture is 24 bytes with no report-ID bytes | code+hw; capture | agrees: unnumbered 8-byte input and 3-byte output reports |
| §2 stream | "slight offsets, which were seen in the field": sigrok resynchronizes on the four model bytes (`bm86x.c:63-78`) | comment | new: the 24-byte stream can arrive misaligned |
| §4.3 report II | sigrok's BM52x parser takes the secondary sign and annunciators from stream byte 7 and the secondary digits from bytes 8-11, the first four data bytes of report II, i.e. Table 1's bytes 9 and 11-14 (`bm52x.c:315-320`, `:450-453`), tested on a BM525s and a BM829s | code+hw | supports BM820 Table 1: byte 10 is the report ID |

### 13.3 Requests and model bytes

| Spec § | Community | Evidence | Verdict |
|---|---|---|---|
| §3.1 BM52x request | sigrok sends `52 66` to a BM52x and `82 66` to a BM82x (`bm52x.c:123-124`, `:154-162`; `libsigrok/src/hardware/serial-dmm/api.c:384-397`); a BM52x reply is taken only with `52` in bytes 20-23 (`bm52x.c:172-184`) | code+hw (BM525s) | new: a BM525s answers `52 66` |
| §3.1 cross-series | "the 'wrong' packet request will end up without a response" (`bm52x.c:34-37`) | comment | **D1**, below |
| §4.2 bytes 20-23 | BM860s: `86 86 86 86` in the freedaun capture (BM869s) and in the MartinD-CZ capture (BM867/869); "The devices that we have seen in the field do provide four bytes" (`bm86x.c:63-78`). BM82x: four `82`, BM52x: four `52` (`bm52x.c:172-198`) | capture; code+hw | new: the BM860 sheet's "don't care" bytes 20-22 carry `86` |
| §4.1 bytes 2, 24-27 | freedaun capture: byte 2 `00`, bytes 24-27 `00`; MartinD-CZ capture: byte 24 `00`. Byte 2 is `00`, and `FF` in resistance mode (`869log/software/decoder.c:23`, `:331-333`; `869log/software/uart_worker.c:119-121`) | capture; comment | new |
| §1.2 cable | sigrok's BU-86X driver serves the BM52x and BM82x drivers, tested with a BM525s and a BM829s (`bm52x.c:31-39`; `api.c:384-397`) | code+hw | supports BU-86X for the BM820s and BM520s |

**D1.** Brymen's Bs82-52x writes `00 00 82 66` in its live loop (Bs8252x
`:21520-21533`, re-read) and gates the import on byte 23 = `52` in the reply
to `82 66` (§10.4), so it expects a BM521s or BM525s to answer `82 66`.
sigrok's comment says a request of the other series goes unanswered, without
saying which direction was tried. Verdict: unresolved; a vendor design
assumption against an author's statement.

### 13.4 Timing, meter off, APO

| Spec § | Community | Evidence | Verdict |
|---|---|---|---|
| §3.2-3.3 | every BU-86X project keeps one handle open and polls: TheHWcave with a 4000 ms read timeout, once a second (`BM869S.py:236-248`, `:264-265`, `:297-299`); sigrok re-requests after 500 ms and waits 100 ms after a reply on the BM86x, 4000 ms and 500 ms on the BM52x and BM82x (`api.c:384-417`; `libsigrok/src/hardware/serial-dmm/protocol.h:38-47`) | code+hw (DawOp: code) | new: reopening per reading, as the sheets' flowchart does, is not needed |
| §3.3 rate | MartinD-CZ's board with no pause between reads ("F - 5 samples per second", `brymen-867-interface-cable/firmware/BrymenConnector_new/main.cpp:34`) logs a reading about every 195 ms; with its 920 ms pause (`:48-49`), every 1.18 s (`brymen-867-interface-cable/console.png`) | capture (timestamps) | the meter answers at about 5 per second, waiting about 100-180 ms for its next measurement [INFERRED from the timestamps less the board's known delays] |
| §3.4 meter off | "The software will hang if you turn the meter off" (`BM869S-remote-access/README.md:44`); sigrok treats a timed-out read as no data (`libsigrok/src/serial_hid_bu86x.c:68-70`) | comment | new: with the meter off the cable sends nothing [INFERRED from the hang: the logger loops on empty reads] |
| §11.6 APO | the same sentence goes on: "(or if it turns itself off after being idle for too long!)" | comment | **D2**, below |

**D2.** Both READMEs say "The meter Auto-Power-Off (APO) feature is disabled
when linked." (README-86x §2-5, README-8252x §2-5, re-read). TheHWcave
reports the meter powering off while its logger polls once a second.
Brymen's programs poll back to back (§3.3), which may matter [INFERRED].
Verdict: unresolved; needs a meter.

### 13.5 LCD maps

| Spec § | Community | Evidence | Verdict |
|---|---|---|---|
| §7 segments and characters | BM86x: b g c d a f e in bits 7-1 (`bm86x.c:82-125`; `BM869S.py:55-88`; `brymen-BM869s.py:35-60`; `Brymen869.cpp:153-175`; `869log/software/decoder.c:174-193`); BM52x: b g c, point, a f e d (`bm52x.c:200-248`). Every pattern they list (0-9, -, C, F, L, d, i, o, n, E, r, A, u, t) matches §7.2 byte for byte | code+hw | agrees |
| §5.2, §6.2 points | Np in bit 0 of digit N+1's byte, 5.0, 11.0 and 13.0 excluded (`bm86x.c:139-157`; `BM869S.py:195-222`); bit 4 of the digit's own byte on the BM52x (`bm52x.c:275-279`). The captures fit this reading (freedaun: "-00.001" mA DC main, "00.00" mA AC secondary; MartinD-CZ: "00.000" Hz, "0.009" V), but the LCD was not recorded | code+hw; capture | agrees with Table 1 and the programs |
| §5.3, §6.3 minus | 4.7 and 12.4 on the BM86x (`bm86x.c:191-195`, `:302-305`; `BM869S.py:184-192`); 4.7 and 9.5 on the BM52x (`bm52x.c:319-320`, `:452-453`); 4.7 is set in the freedaun capture | code+hw; capture | agrees |
| §5.5, §6.5 annunciators | △ (5.0; BM52x 4.6) is relative zero (`bm86x.c:263-264`; `bm52x.c:410-411`; `869log/software/decoder.c:202-203`, code; `brymen-867-interface-cable/firmware/BrymenConnector_new/decoder.cpp:81-82`). BM860s 3.0 AUTO, 3.3 HOLD (`bm86x.c:249-256`), 3.1 REC, 3.2 CREST (`decoder.cpp:67-80`). BM52x 24.4 AUTO, 24.7 HOLD (`bm52x.c:400-403`); 3.2 is the MAX-MIN (Vp-p) dash (`bm52x.c:371-393`, `:55`). "@, 4-20mA loop, % (main display, left hand side), Hi/Lo" are not supported by the BM525s (`bm52x.c:59-63`) | code+hw; code | agrees with §5.5 and §6.5's readings; new: @, % and Hi/Lo reported unused on the BM525s (comment) |
| §5.5 T1-T2 | TheHWcave reads 4.2 (the ③ dash) as T1-T2 (`BM869S.py:141`); freedaun, DawOp and 869log read T1 and T2 lit together as T1-T2 (`brymen-BM869s.py:108`; `Brymen869.cpp:225`; `869log/software/decoder.c:54-61`) | code+hw (TheHWcave, freedaun); code (DawOp, 869log) | the community disagrees with itself; open |
| §8.1 %4-20mA | 12.3 (BM86x, `bm86x.c:316-318`; `libsigrok/NEWS:413-414`), 14.4 (BM52x, `bm52x.c:467-472`) | code | agrees |
| §7.3 words | "0L"/"0.L" is overload (`bm86x.c:196`; `bm52x.c:321`); "diod" on the secondary is diode (`bm86x.c:191-193`); "Auto" is AutoCheck (`bm52x.c:318`); "---C" and "---F" are skipped as no temperature (`bm52x.c:322-323`, `:456-461`); "InEr" shows on the BM525s secondary (`bm52x.c:69-70`) | code+hw; comment | agrees; new: "---C"/"---F" on the BM525s |
| §5.4, §6.4 bar graph | both BM860s captures are near-zero readings (-00.001 mA; 00.000 Hz) with bar-scale 4.4 `0`, where a bar would light nothing; bytes 24-27 are `00` in the freedaun capture. Byte 2 (`FF` in resistance, §13.3) and byte 24 are in the optical window (§13.6) and unexplained. sigrok guesses the BM52x's undocumented bits are the bar (`bm52x.c:64-65`) | capture; comment | the captures don't show where the bar is; byte 2 and byte 24 stay candidates; open for both maps |

Three points in the community code are errors, not findings: sigrok's BM52x
secondary duty reads 14.3, segment 8a (`bm52x.c:493-495`), where BM820 p.1
has ② D% at 15.3; DawOp names 12.7 "+-" (`Brymen869.cpp:125-129`), which
BM860 p.1 draws as a battery with + and − in it; 869log takes bit 0 of
digits 2-6 as the main point (`869log/software/decoder.c:200-209`), so it
reads V ① (11.0) as a point before digit 6. 869log's PC decoder is, by its
author, "almost untested" (`869log/software/decoder.c:19`;
`869log/README.md:59`), so its mappings count as code.

### 13.6 The link under the cable

Seen only through the two DIY boards [COMMUNITY, code+hw]:

- The meter's port is infrared, 940 nm (`869log/README.md:16`).
- It is not a UART. The host lights its LED for 10 ms or more, waits for
  the meter's LED (timeouts 300 ms, `869log/firmware/main.S:79`, `:276-291`;
  about 510 ms, `decoder.cpp:32-42`), then sends 160 light pulses and reads
  one bit per pulse, least significant bit first: 20 bytes
  (`869log/README.md:22`; `main.S:298-316`, 250 µs half-periods;
  `decoder.cpp:12-28` with `config.h:23`, 100 µs). The 115200 and 9600
  baud figures in the two projects are their own serial links to the PC.
- Window: 869log's 20 bytes are Table 1's bytes 2-9, 11-18 and 20-23, the
  model byte last and checked (`869log/software/decoder.c:332-366`;
  `869log/software/uart_worker.c:119-121`);
  MartinD-CZ's are bytes 3-9, 11-18 and 20-24 (`decoder.cpp:66-180`;
  `brymen-867-interface-cable/console.png`, byte 24 `00`). The two differ
  by one byte; unresolved.

What follows [INFERRED]: the model byte comes from the meter, not the
cable; the report-ID bytes and at least bytes 25-27 are the cable's; and a
live request carries no data to the meter, so the series code in the HID
request is the cable's to interpret. Whether the cable asks the meter once per
request or answers from a frame it holds (§1.2) stays open. How the memory
commands (§10) reach a BM52x: not shown by these sources, 2026-09-27.

### 13.7 Logged-memory download

| Spec § | Community | Evidence | Verdict |
|---|---|---|---|
| §10.2 checksum | the 16-bit sum of the 24 data bytes, low byte first (`bm52x.c:637-647`, `:709-718`); the capture's blocks carry `7c 05`, `80 03`, `00 03`, `ae 04`, which are their sums | capture | agrees with the programs; settles MRAD's byte order |
| §10.5 header | capture: Model_Id `01` (the BM525s), TotalBytes `e6 02 00` (742), one session, `ee a0`, PS1 `8a 03 a0`, NS1 `60 03 a0`, Bfunction/Bselect/Bstatus `02 00 00`, page length `d0 02 00` (720), samples `00 00 00` and `80 00 00`, `ee c0`. "Recording session total byte counts … include this field and the model ID" (`bm52x.c:43-45`) | capture | new: TotalBytes counts from Model_Id (6 + 736 = 742); addresses are 24-bit, low byte first, `A0` the high byte, and NS1 = A0_0080h + 736 is the next session's head; Model_Id `01` on a BM525s |
| §10.5 samples | the value is read as one binary 16-bit integer (`bm52x.c:849-878`); the capture's samples are all zero | code | digits or binary: open |
| §10.5 Table 3 | Bfunction `05`: Bselect `01` Siemens, `02` continuity (`bm52x.c:1056-1073`); the recording code is "mostly untested" (`bm52x.c:71-76`) | code | **D3**, below |
| §10.4 intervals | the same 16 values as MRAD Table 3-1 (`bm52x.c:598-618`) | code | agrees |

**D3.** MRAD p.8, re-read from the render: `05` `01` is Continuity and `02`
nS. Bs8252x `FUN_00434c78` agrees, mapping `01` to `0x180` (continuity) and
`02` to `0x1000` (S) (Bs8252x `:21132-21150`, re-read). Verdict: Brymen's
order stands; sigrok's differs from both with no capture behind it.

### 13.8 Models and rebrands

- TestController lists the eight models of §1.1 under Brymen
  (`testcontroller-supported-equipment.html:113`), and "Elma BM525s, Elma
  BM821s, Elma BM829s, Elma BM869s" (`:163`): Elma-branded units of the same
  model names [INFERRED from the names]. It lists Greenlee DM-210A, DM-810A,
  DM-820A, DM-830A, DM-860A and DML-430A (`:165`) with no Brymen model or
  interface named; a link to this group: none found in these sources,
  2026-09-27.
- sigrok names its drivers "BM86x", "BM52x" and "BM82x", all on the BU-86X
  (`api.c:384-417`), and lists no key press to start PC output for them
  (`libsigrok/README.devices:445-447` lists one for the BM257s).

Not answered by the community sources (none found in these sources,
2026-09-27): §12.5 beyond the inference of §13.6, §12.11, §12.13, §12.15,
§12.16's glyphs, §12.20, and every question about the BM867s, 821s, 822s,
827s and 521s.
