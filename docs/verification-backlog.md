# Verification backlog

Open checks that span families — detection, the capture tool, the transports
and streaming, the GUI — and the known defects. Each family's own checks are
in its `verification.md`; what hardware has confirmed is tagged in its spec.

| Family | Open checks | Issues |
|---|---|---|
| UT61E+ frames, flags and CP2110 | [ut61eplus](research/ut61eplus/verification.md) | — (our meter) |
| UT61+ family: UT61B+/D+, UT161, UT60BT, UT202BT, commands | [ut61-family](research/ut61-family/verification.md) | [#6](https://github.com/antoinecellerier/dmm-tools/issues/6), [#7](https://github.com/antoinecellerier/dmm-tools/issues/7), [#19](https://github.com/antoinecellerier/dmm-tools/issues/19), [#20](https://github.com/antoinecellerier/dmm-tools/issues/20), [#26](https://github.com/antoinecellerier/dmm-tools/issues/26), [#27](https://github.com/antoinecellerier/dmm-tools/issues/27) |
| UT181A | [ut181](research/ut181/verification.md) | [#5](https://github.com/antoinecellerier/dmm-tools/issues/5) |
| UT171 | [ut171](research/ut171/verification.md) | [#4](https://github.com/antoinecellerier/dmm-tools/issues/4) |
| UT8803 | [ut8803](research/ut8803/verification.md) | [#3](https://github.com/antoinecellerier/dmm-tools/issues/3) |
| UT8802 | [uci-bench-family](research/uci-bench-family/verification.md) | [#12](https://github.com/antoinecellerier/dmm-tools/issues/12) |
| UT803, UT804 | [ut803](research/ut803/verification.md) | [#15](https://github.com/antoinecellerier/dmm-tools/issues/15), [#16](https://github.com/antoinecellerier/dmm-tools/issues/16) |
| UT71, Voltcraft VC920/VC940/VC960 | [ut71](research/ut71/verification.md) | [#22](https://github.com/antoinecellerier/dmm-tools/issues/22), [#23](https://github.com/antoinecellerier/dmm-tools/issues/23) |
| VC-880, VC650BT | [vc880](research/vc880/verification.md) | [#13](https://github.com/antoinecellerier/dmm-tools/issues/13) |
| VC-890 | [vc890](research/vc890/verification.md) | [#14](https://github.com/antoinecellerier/dmm-tools/issues/14) |
| UT-D07A, UT-D07B adapters | [ut-d07b](research/ut-d07b/verification.md) | [#25](https://github.com/antoinecellerier/dmm-tools/issues/25) |
| ZOTEK | [zotek](research/zotek/verification.md) | [#28](https://github.com/antoinecellerier/dmm-tools/issues/28)–[#31](https://github.com/antoinecellerier/dmm-tools/issues/31) |
| EEVblog 121GW | [121gw](research/121gw/verification.md) | [#32](https://github.com/antoinecellerier/dmm-tools/issues/32) |
| Brymen BM78xBT | [bm78xbt](research/bm78xbt/verification.md) | [#33](https://github.com/antoinecellerier/dmm-tools/issues/33) |
| Brymen BU-86X: BM86x, BM82x, BM52x | [bm86x](research/bm86x/verification.md) | [#34](https://github.com/antoinecellerier/dmm-tools/issues/34)–[#36](https://github.com/antoinecellerier/dmm-tools/issues/36) |
| UT632 (not implemented) | [ut632](research/ut632/verification.md) | — |
| UT8805, UT8806 (not implemented) | [ut8805](research/ut8805/verification.md) | — |

## Device auto-detection

Detection identifies the meter from the bytes it sends
(`crates/dmm-lib/src/detect.rs`; the algorithm and its failure modes are in
[detection-design.md](detection-design.md)). A capture report's `detection`
section holds a run through the detector on the reporter's meter: the probes,
the replies, what was picked and a reading taken through the pick. A section
with `power_cycled: true` settles a row below; its `read_back` shows whether
the other families' probes left the meter working, which the items under it
ask about, though not whether it beeped. What each family is probed
with, and how well that probe is backed:

| Family | Detection sends | Expects back | Hardware status |
|---|---|---|---|
| UT61E+ | `AB CD 03 5F 01 DA` (Get Name) | ack `AB CD 04 FF 00 02 7B`, then an ASCII name frame | Verified through the detector on our UT61E+ (CP2110) |
| UT61B+ | same | same, the name being `UT61B+` | Verified over CH9329 ([#19](https://github.com/antoinecellerier/dmm-tools/issues/19)) |
| UT61D+, UT161B/D/E | same | same, the name being the model | Unverified; no report has named one of these meters |
| UT60BT, UT202BT | same, over their built-in Bluetooth | same, the name being `UT60BT` or `UT202BT` | Unverified; a community UT60BT's `UT60BT` reply is on record (ut61-family approach doc), nothing for the UT202BT |
| UT181A | `AB CD 04 00 05 01 0A 00` (SET_MONITOR) | 2-byte-LE frames, type `0x02`, payload ≥ 31 bytes | Reply verified on hardware ([PR #8](https://github.com/antoinecellerier/dmm-tools/pull/8), [#5](https://github.com/antoinecellerier/dmm-tools/issues/5)), never through the detector |
| UT171 | `AB CD 04 00 0A 01 0F 00` (connect) | 2-byte-LE frames, type `0x02`, 16- or 22-byte payload | Deduced from the vendor traces, unverified |
| UT8802 | nothing — the meter streams | two `0xAC` frames exactly 8 bytes apart | Deduced from the vendor traces, unverified |
| UT8803 | nothing — the meter streams | `AB CD` frame, byte 3 `0x02`, 21-byte checksum | Deduced from the vendor traces, unverified |
| UT803, UT804 | nothing beyond the CH9325 init's `0x5A` | any 11-byte CR LF packet, taken as a UT804; a UT803 (19200 baud) is not detected | Verified by @clazie on a UT804 over the UT-D04 (CH9325, [#16](https://github.com/antoinecellerier/dmm-tools/issues/16)); the UT803 is undetectable by design |
| UT71A–E, VC920/VC940/VC960 | nothing beyond the CH9325 init's `0x5A` | any 11-byte CR LF packet, claimed as a UT804: the packet names no model, so the user names the meter | Never seen: no UT71 or VC9x0 packet captured ([#22](https://github.com/antoinecellerier/dmm-tools/issues/22), [#23](https://github.com/antoinecellerier/dmm-tools/issues/23)) |
| VC-880, VC650BT | nothing — the meter streams once PC is pressed; a VC650BT is reported as a VC-880, the protocol being byte-identical | `AB CD` BE16 frame, payload `[0] == 0x01`, 34 bytes | Deduced from the vendor traces, unverified |
| VC-890 | 3× `AB CD 04 FF 00 02 7B`, then `AB CD 03 5E 01 D9` | `AB CD` BE16 frame, payload `[0] == 0x01`, 61 bytes | Deduced from the vendor traces, unverified |
| ZOTEK ZT-300AB / AN9002, ZT-5566SE / AN999S, ZT-5BQ / ST207, ZT-5B / V05B | nothing — the meter streams over its built-in Bluetooth | one whole packet: on-air `1B 84`, a type byte with a layout, that type's length, every digit a listed glyph; the type byte picks the entry | Deduced from ZOTEK's apps, unverified; community captures show the packets (ZOTEK spec §11) |
| EEVblog 121GW | nothing — the meter streams over its built-in Bluetooth | one packet: 18 bytes whose XOR is `F2`, with or without the `F2` before them, a mode and range in the tables, no reserved bit set | Deduced from EEVblog's documents and apps, unverified; both community-captured packets pass (121GW spec §15.5) |
| Brymen BM788BT, BM787BT | nothing — the meter streams over its built-in Bluetooth once the transport has logged in | one 32-byte reading packet: `FF 02 20 05`, a CRC-16/MODBUS over bytes 2-27, `FF 03`, device type `01` | Deduced from Brymen's protocol document and app, unverified; no packet a meter sent is on record (bm78xbt spec §12) |
| Brymen BM869s, BM867s | `00 86 66`, the reading request, on the BU-86X alone | a reply whose model bytes 20-23 are four `86` | Deduced from Brymen's sheet and programs, unverified; community captures show the four `86` (bm86x spec §13.3) |
| Brymen BM829s, BM827s, BM822s, BM821s | `00 82 66`, after `00 86 66`, on the BU-86X alone | a reply whose model bytes 20-23 are four `82`, whichever request drew it | Deduced from Brymen's sheet and programs, unverified (bm86x spec §4.2) |
| Brymen BM525s, BM521s | `00 52 66`, after `00 86 66` and `00 82 66`, on the BU-86X alone | a reply whose model bytes 20-23 are four `52`, whichever request drew it: Brymen's programs expect one to answer `82 66` | Deduced from Brymen's sheet and programs, unverified; community code reads four `52` from a BM525s (bm86x spec §13.3) |

### Probes on the wrong meter

- What `0x5F` (Get Name) does to a VC-880, VC-890, UT171 or UT181A, which
  step 1 sends to whatever is on the cable, and what a UT181A answers to the
  VC-890's `0x5E` (ut181 spec §11.1). Needs each meter watched under `auto`.
- What SET_MONITOR (`0x05`) does to a UT171, which receives it before its own
  connect frame. Needs a UT171 watched under `auto`. (#4)
- The UT61+, UT181A and UT171 probes on a ZOTEK meter: they go out before its
  listen-only rule hears a packet, FFF4 takes writes, and none is a frame its
  command format (ZOTEK spec §8) accepts. Needs one watched. (#28–#31)
- The same probes on a 121GW opened by address with no name heard: firmware
  1.02 drops frames not led by `F4` or `F8` (121GW spec §15.4); current
  firmware is unwatched. Needs `auto --adapter` on a current 121GW. (#32)
- The same probes on a BM78xBT opened by address with no name heard (renamed,
  or `auto --adapter`): 6- and 8-byte writes to CDD4, where r4 knows only
  32-byte commands (bm78xbt spec §5). Needs one watched. (#33)
- The family's `init` run again on the open after detection: a UT181A gets
  SET_MONITOR twice, a UT171 its connect frame twice — both already sent, but
  no meter has been watched. Needs either under `auto`. (#4, #5)

### Replies and names

- The UT181A/UT171 payload overlap (19 bytes without aux or bargraph, against
  16/22): a short frame goes to whichever probe went out last. Splitting them
  by parse needs UT171 hardware. (#4)
- A UT181A reply after its ~600 ms window, read as a UT171 once `0x0A` (UT181A
  start recording) is out, and whether a UT181A ever starts recording under `auto`
  (reasoned, not observed; likelier behind a UT-D07B, see
  [Connection](#connection)) — decides the probe order. Needs a reply timed. (#5)
- The UT61D+ and UT161B/D/E reported names: an unrecognised one falls back to
  the UT61E+ tables and is logged, and a reporter's `RUST_LOG=dmm_lib=debug`
  output turns it into a registry alias. (#7)
- Whether a VC-890 answers `0x5E` on the first attempt: the vendor software
  retries up to 10 times, flushing between — decides whether one poll is
  enough. Needs a VC-890 under `auto`. (#14)
- A Brymen meter answering after the three BU-86X windows (a BM86x at 500000
  counts, any of them in capacitance): in the
  [BM86x checks](research/bm86x/verification.md#timing-and-replies).

## Capture tool

A family's first report is `dmm-cli --device <id> capture --unverified`
([capture design](capture-design.md#e-per-step-verification-status)); what
each step settles is in the family's file.

- The VC-880, VC650BT and VC-890 lists run `acv`, `acdcv`, `dcmv`, `dcua`,
  `acua`, `dcma`, `acma`, `dca`, `aca` before their gate closes, so none is
  swept. Needs each dial order and a run; a fix deletes its `SPLIT_GATE` entry.
- A sweep re-clearing an operator-set `rel` or `peak` flag: in the
  [UT181A checks](research/ut181/verification.md#capture).
- Sub-values end to end — the reading panel, log, graph and both CSV exports
  read `aux_values`, which a capture checks only in the parser. Needs UT181A
  `read --format csv` in V AC and dual thermocouple and a MIN/MAX screenshot. (#5)

## Transport and streaming

### Bluetooth

- A cached built-in meter against a known UT-D07B: the fallback tries the platform's
  first, so an asleep 121GW, BM78xBT or ZOTEK meter costs a ~10 s connect — decides
  [known adapters first](future-improvements.md#known-adapters-first-in-the-bluetooth-fallback). Needs a report that shows it.
- A chance 121GW or BM78xBT claim on a UT-D07B link: both rules decline AB CD
  frames, each other's and ZOTEK packets and the adapter heartbeat in tests
  ([detection design](detection-design.md)). Watch UT-D07B reports. (#25)

### USB bridges

- The CP2110, CH9325 and BU-86X on macOS: only the CH9329 has run there (a
  UT181A on its UT-D09, [PR #8](https://github.com/antoinecellerier/dmm-tools/pull/8), [#2](https://github.com/antoinecellerier/dmm-tools/issues/2)).
  Plain HID or not decides `docs/setup.md`. Needs any meter on one of them.
- The byte order of CP2110 report 0x42's FIFO counts: SLABHIDtoUART.dll reads
  them big-endian, `Cp2110::uart_status` little-endian (UT171 spec §8); idle
  reads give 0. Decides `uart_status`. Needs a read with bytes queued.

### Reading and streaming

- Ctrl-C mid-sleep in a paced `read`, untimed — decides whether the
  cancellable sleep needs a shorter slice. Needs any polled meter at
  `read --interval-ms` 2000 or more.
- A UT181A regression read under the continuous stream, and `set hold` /
  `set rel` after a GUI Pause — confirms the resync, whose other checks ran on
  our UT61E+ behind a UT-D07B. Needs the UT181A. (#5)
- A HID streaming meter against its LCD on Linux, whose hidraw keeps the
  oldest queued reports: decides whether the 250 ms resync covers it. Needs a
  UT8803 or UT8802 at `--interval-ms 2000`. (#3, #12)
- Wall times across a suspend, each the system time its reading was taken at —
  confirms `Clock::wall_time_for`. Suspend mid-session with the mock and with
  our UT61E+, then compare the exported rows with the wall clock.
- A polled reply later than the 2 s deadline is dropped, not taken for the
  next request's: does a UT61E+ reply ever take that long, capacitance
  included? Needs our UT61E+ on its cable.

## Vendor sources

- UNI-T's PC software ("优利德上位机软件", `1.10.zip`, `Setup.zip`, ~150 MB each,
  [candidates](research/new-device-candidates.md#uni-t-chinese-sites)): whether it drives
  the UT632, UT8802 or other bench meters. Needs the downloads unpacked; no meter.

## GUI accessibility

The keyboard half is in the [GUI reference](gui-reference.md#keyboard); no
real screen reader has walked the other half.

- Orca (AT-SPI): each interactive widget announces a sensible name on Tab,
  and HOLD, REL, RANGE, AUTO, MIN/MAX, PEAK and LIVE say "pressed" / "not
  pressed" when toggled.
- Orca: the plot's summary is read when focused, the reading's live region
  speaks politely rather than continuously, and Orca+Ctrl+Shift+L lists
  Toolbar, Main and Status.
- Orca with sub-values (a UT181A or the mock): "Plot \<name\>" radios and "Show
  \<name\> trace" toggles with their state; the live region speaks sub-values and
  "at N seconds" without flooding; "Also showing …" names the drawn traces.
- Orca with a software scale: the live region ends with ", software scaled"
  and drops it when scaling is off, **Scale** announces on/off, and its fields
  read "Scale factor", "Offset" and "Unit label" from their hints.
- The same checks under NVDA or JAWS (UI Automation) and VoiceOver
  (NSAccessibility), AccessKit's other two backends.

## Known defects

Bugs a reader can reproduce: symptom, cause, and the fix where known.

### Readings and flags

- **VOID is described as an invalid reading.** `StatusFlags::void`'s doc, the
  GUI reference's flag list and the JSON flags comment call it a reading the
  meter marked invalid, but the VC-890 manual and Conrad's document define
  the VOID symbol as empty memory (vc890 spec, byte 59). Fix: describe it as
  the meter's VOID symbol; the [VC-890 check](research/vc890/verification.md#void-with-a-live-reading) decides the rest.
- **A UT804 LO reading is stored as 0.0.** In 4-20 mA % the LCD shows
  `- LO. %` (ut803 spec §3.3), but the parser reports `Normal(0.0)` with the
  text "L0.", so statistics, the graph and exports count a zero. Fix,
  untried: `MeasuredValue::NoReading("LO")`.
- **The `dc` flag has no definition.** `StatusFlags::dc` carries no doc, and
  families set it differently on AC+DC: the UT804 from coupling 3, the BM86x
  and BM78xBT clear it, ZOTEK from its DC annunciator, the UT61E+ within its
  AC+DC modes. Fix: define it in `flags.rs` and align the families.
- **HOLD on the AC component of AC+DC V blanks the reading.** Held there, a
  UT61E+ sends only AC frames (ut61eplus spec §2.7), which carry no main
  reading: blank digits beside the AC row, and the Main view's minimap,
  cursors and statistics stay empty; **Plot:** AC draws it. Fix, untried:
  show the held component as the reading.
- **A software offset shifts the AC component of AC+DC V.** It applies to
  the AC sub-value like any same-unit sub-value, which means nothing for an
  RMS value. Fix, untried: exempt AC sub-values from offsets.

### Connection

- **A switched-off paired Bluetooth peer is listed as heard.** BlueZ shows a paired
  UT-D07B as heard in `dmm-cli list`, the doc-screenshot guard refuses to run, and an
  open with no address tries it first, failing with a Bluetooth error, not "No meter
  found". Cause: BlueZ keeps its last RSSI (UT-D07B spec §4), which `standing()`
  (`ble/search.rs`) takes as heard. Fix, untried: count only RSSI changes in our scan.
- **A meter power cycle surfaces a checksum error.** `read_frame` (`framing.rs`) keeps
  a partial frame across a timeout: our UT61E+ on CP2110 showed `checksum mismatch:
  expected 0x3534, got 0x055a`. Since `Dmm` drops a polled meter's input after a
  timeout, the CP2110 case should be gone (not retried on our UT61E+); UT803/UT804 and
  the UT61+ over the UT-D07B, streamed and propagating frame errors, still keep it.
  Fix, untried: clear `rx_buf` on the timeout, as no frame takes 2 s.
- **`auto` misses a BU-86X beside another cable, and Bluetooth behind a
  silent one.** It tries the BU-86X after every other cable and stops at the
  first bridge found ([detection design](detection-design.md#bridges-and-adapters)).
  Workaround: name the meter, or pass `--adapter`. (#34–#36)
- **Over the UT-D07B, detection sends a UT61+ meter the UT181A probe.** Our
  UT61E+ answered Get Name after 617 ms over the adapter, past the 600 ms
  `WINDOW` (`detect.rs`), so SET_MONITOR went out before the name arrived; the
  name still won and the meter ignored the probe. A UT181A as slow behind the
  adapter would get the UT171's `0x0A` (start recording) too, the UT181A item
  under [Replies and names](#replies-and-names). Fix, untried: a longer window
  on Bluetooth links. (#25)

### Capture runs

- **A run ends manually ranged.** The next run then skips the rung it opens on
  (a UT61E+ `ohm_ranges` lost `22MΩ`). Cause: the MIN/MAX walk after the range
  walk locks the range, and 0x42 leaves it locked. Fix: re-assert Auto after
  the flag sweeps.
- **HOLD, REL and MIN/MAX sweeps can freeze a settling reading.**
  `ohm_ranges/hold:on` held 12.59 kΩ of an 82 kΩ resistor on our UT61E+. Cause:
  `--settle` waits after the press, and these act on the live reading. Fix,
  untried: settle before those presses; a rerun of `ohm_ranges/hold:on` confirms.

### GUI and mock

- **A flat trace labels its y-axis with six decimals.** A constant 118.3 V
  trace labels two grid lines "118.300000 V", a zero current "-0.000000 A":
  the y-axis formatter (`graph/render.rs`) takes its decimals from the grid
  step, up to six, and the auto range presumably collapses around a constant.
  Replay one frame of `crates/dmm-lib/tests/golden/ut61eplus/lpfv.yaml`.
- **A settings file this build can't parse is replaced by defaults.**
  `Settings::load` (`dmm-gui/src/settings.rs`) falls back to the defaults and
  the next save writes them over the file — likely after a downgrade, meeting
  a theme or colour preset only a newer build knows. Fix, untried: deserialize
  field by field from a `serde_json::Value`, defaulting only those that fail.
- **The mock offers Peak outside the AC modes.** Its Hz, Ω, capacitance,
  temperature and NCV scenarios take Peak, where the UT61E+ offers it only in
  `AC_PEAK_MODES` (`protocol/ut61eplus/tables/mod.rs`). Cause:
  `Scenario::peak_applies` (`mock/scenarios.rs`) excludes only the DC
  scenarios. Fix: narrow it to the AC ones.
