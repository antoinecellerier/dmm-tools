# Protocol Verification Backlog

Items that need real components or specific setups to verify.

## Table of contents

- [Device auto-detection](#device-auto-detection)
- [Pending Verification](#pending-verification)
  - [Capture leaves the meter manually ranged](#capture-leaves-the-meter-manually-ranged)
  - [Capture: settle before HOLD, REL and MIN/MAX](#capture-settle-before-hold-rel-and-minmax)
  - [Capture: gate steps placed after other steps](#capture-gate-steps-placed-after-other-steps)
  - [Protocol families we have no meter for](#protocol-families-we-have-no-meter-for)
  - [Capture and display checks a UT181A runs](#capture-and-display-checks-a-ut181a-runs)
  - [Bluetooth search and detection with built-in meters](#bluetooth-search-and-detection-with-built-in-meters)
  - [macOS bridges other than the CH9329](#macos-bridges-other-than-the-ch9329)
  - [CP2110 FIFO counts](#cp2110-fifo-counts)
  - [CLI paced reads](#cli-paced-reads)
  - [Streaming meters: read continuously](#streaming-meters-read-continuously)
  - [Vendor sources not yet read](#vendor-sources-not-yet-read)
  - [Known defects](#known-defects)
  - [Entering NCV leaves the previous mode's trace on the graph](#entering-ncv-leaves-the-previous-modes-trace-on-the-graph)
  - [A flat trace labels its y-axis with six decimals](#a-flat-trace-labels-its-y-axis-with-six-decimals)
  - [A meter power cycle surfaces a checksum error](#a-meter-power-cycle-surfaces-a-checksum-error)
  - [A settings file this build can't parse is replaced by defaults](#a-settings-file-this-build-cant-parse-is-replaced-by-defaults)
  - [GUI accessibility — screen reader walk-through](#gui-accessibility--screen-reader-walk-through)

## Device auto-detection

Detection identifies the meter from the bytes it sends instead of being
told which family to expect (`crates/dmm-lib/src/detect.rs`; the algorithm
and its failure modes are in `docs/detection-design.md`). What each family
is probed with, and how well that probe is backed:

| Family | Detection sends | Expects back | Hardware status |
|---|---|---|---|
| UT61E+ | `AB CD 03 5F 01 DA` (Get Name) | ack `AB CD 04 FF 00 02 7B`, then an ASCII name frame | **Verified through the detector** 2026-09-11 on our UT61E+ (CP2110): identified in under a second, ack at 85 ms and name at 193 ms in the capture's `init_frames`; with the meter off the cascade ends in 2.7 s with the not-identified help |
| UT61B+ | same | same, the name being `UT61B+` | Verified over CH9329 ([issue #19](https://github.com/antoinecellerier/dmm-tools/issues/19)) |
| UT61D+, UT161B/D/E | same | same, the name being the model | Unverified — no report has named one of these meters |
| UT60BT, UT202BT | same, over their built-in Bluetooth | same, the name being `UT60BT` or `UT202BT` | Unverified — one UT60BT's `UT60BT` reply is on record from community sources (ut61-family approach doc, 2026-09-25); nothing for the UT202BT |
| UT181A | `AB CD 04 00 05 01 0A 00` (SET_MONITOR) | 2-byte-LE frames, type `0x02`, payload ≥ 31 bytes | The reply is verified on hardware ([PR #8](https://github.com/antoinecellerier/dmm-tools/pull/8), [issue #5](https://github.com/antoinecellerier/dmm-tools/issues/5)), never through the detector |
| UT171 | `AB CD 04 00 0A 01 0F 00` (connect) | 2-byte-LE frames, type `0x02`, 16- or 22-byte payload | Deduced from the vendor traces, unverified |
| UT8802 | nothing — the meter streams | two `0xAC` frames exactly 8 bytes apart | Deduced from the vendor traces, unverified |
| UT8803 | nothing — the meter streams | `AB CD` frame, byte 3 `0x02`, 21-byte checksum | Deduced from the vendor traces, unverified |
| UT803, UT804 | nothing beyond the CH9325 init's `0x5A` | any 11-byte CR LF packet, taken as a UT804; a UT803 (19200 baud) is not detected | ~~detection unverified~~ — **VERIFIED** 2026-09-17 by @clazie on a real UT804 (UT-D04 / CH9325): `detect: ut804 identified from 11 received bytes during ut80x stream`, then `connected to UNI-T UT804`. See [#16](https://github.com/antoinecellerier/dmm-tools/issues/16). The UT803 is still undetectable by design |
| UT71A–E, VC920/VC940/VC960 | nothing beyond the CH9325 init's `0x5A` | any 11-byte CR LF packet, claimed as a UT804 — the packet does not name its model, so the user names the meter | Never seen: no UT71 or VC9x0 packet has been captured ([#22](https://github.com/antoinecellerier/dmm-tools/issues/22), [#23](https://github.com/antoinecellerier/dmm-tools/issues/23)) |
| VC-880, VC650BT | nothing — the meter streams once PC is pressed; a VC650BT is reported as a VC-880, the protocol being byte-identical | `AB CD` BE16 frame, payload `[0] == 0x01`, 34 bytes | Deduced from the vendor traces, unverified |
| VC-890 | 3× `AB CD 04 FF 00 02 7B`, then `AB CD 03 5E 01 D9` | `AB CD` BE16 frame, payload `[0] == 0x01`, 61 bytes | Deduced from the vendor traces, unverified |
| ZOTEK ZT-300AB / AN9002, ZT-5566SE / AN999S, ZT-5BQ / ST207, ZT-5B / V05B | nothing — the meter streams over its built-in Bluetooth | one whole packet: on-air `1B 84`, a type byte with a layout, that type's length, every digit a listed glyph; the type byte picks the entry | Deduced from ZOTEK's apps, unverified; community captures show the packets (ZOTEK spec §11) |
| EEVblog 121GW | nothing — the meter streams over its built-in Bluetooth | one packet: 18 bytes whose XOR is `F2`, with or without the `F2` before them, a mode and range in the tables, no reserved bit set | Deduced from EEVblog's documents and apps, unverified; both community-captured packets pass (121GW spec §15.5) |
| Brymen BM788BT, BM787BT | nothing — the meter streams over its built-in Bluetooth once the transport has logged in | one 32-byte reading packet: `FF 02 20 05`, a CRC-16/MODBUS over bytes 2-27, `FF 03`, device type `01` | Deduced from Brymen's protocol document and app, unverified; no packet a meter sent is on record (bm78xbt spec §12) |
| Brymen BM869s, BM867s | `00 86 66`, the reading request, on the BU-86X alone | a reply whose model bytes 20-23 are four `86` | Deduced from Brymen's sheet and programs, unverified; community captures show the four `86` (bm86x spec §13.3) |
| Brymen BM829s, BM827s, BM822s, BM821s | `00 82 66`, after `00 86 66`, on the BU-86X alone | a reply whose model bytes 20-23 are four `82`, whichever request drew it | Deduced from Brymen's sheet and programs, unverified (bm86x spec §4.2) |
| Brymen BM525s, BM521s | `00 52 66`, after `00 86 66` and `00 82 66`, on the BU-86X alone | a reply whose model bytes 20-23 are four `52`, whichever request drew it: Brymen's programs expect one to answer `82 66` | Deduced from Brymen's sheet and programs, unverified; community code reads four `52` from a BM525s (bm86x spec §13.3) |

Open questions, each needing a meter:

- **UT181A and UT171 payload lengths overlap** (19 bytes without aux or
  bargraph against 16/22), so a short frame is attributed to whichever
  probe went out last. Splitting them by parse needs UT171 hardware.
- **The UT171 connect frame is UT181A opcode `0x0A`, start recording.** The
  cascade identifies a UT181A with Communication ON in the step before, and
  one with Communication OFF ignores everything — that no UT181A ever starts
  a recording during detection is reasoned, not observed. The ordering only
  covers a meter that answers inside its own ~600 ms window: a reply that
  finishes arriving later is classified in the connect step instead, where
  `0x0A` has already gone out and a short frame reads as a UT171. No UT181A
  reply has been timed through the detector.
- **What `0x5F` (Get Name) does to a VC-880, VC-890, UT171 or UT181A** is
  unknown; step 1 sends it to whatever is on the cable. Nor is it known what
  a UT181A answers to the VC-890 step's `0x5E` (ut181 spec §11.1).
- **What SET_MONITOR (`0x05`) does to a UT171** is unknown; it goes out
  before the UT171's own connect frame.
- **The UT61D+ and UT161B/D/E reported names are unverified.** An
  unrecognised name falls back to the UT61E+ tables and is logged, so a
  reporter's `RUST_LOG=dmm_lib=debug` output is what turns one into a
  registry alias.
- **Does a VC-890 answer `0x5E` on the first attempt?** The vendor software
  retries the name request up to 10 times with a buffer flush between
  attempts, so a single poll may not be enough.
- **What the UT61+, UT181A and UT171 probes do to a ZOTEK meter.** They go
  out on the Bluetooth link before its listen-only rule has heard a packet,
  and FFF4 takes writes; unscrambled, none is a frame the meter's command
  format (ZOTEK spec §8) would accept, but no meter has been watched.
- **What the UT61+, UT181A and UT171 probes do to a 121GW.** Only one opened
  by address with no name heard gets them; one advertising "121GW" runs its
  own rule alone. Firmware 1.02's receiver drops every frame not led by `F4`
  or `F8` (121GW spec §15.4), which none of the probes is; current firmware
  has not been watched.
- **What the UT61+, UT181A and UT171 probes do to a BM78xBT.** Only one
  opened by address with no name heard gets them (a renamed meter, `auto`
  with `--adapter`); one advertising "BM78xBT" runs its own rule alone. They
  are written to its command characteristic, CDD4, as 6- and 8-byte writes;
  r4 knows only 32-byte commands (bm78xbt spec §5), and what the meter does
  with a short one is unknown.
- **A Brymen meter on the BU-86X can answer after the three 600 ms
  windows**: a BM86x at 500000 counts, or any of them in capacitance; see
  [the BM86x checks](research/bm86x/verification.md#timing-and-replies).
- **`auto` and the other cables.** `auto` stops at a BU-86X whose meter is
  silent and does not reach Bluetooth, and with any other cable plugged in
  too it never probes the BU-86X; setup and detection docs say to name the
  meter.
- **Opening after detection runs the family's `init` again**, so a UT181A
  receives SET_MONITOR twice and a UT171 its connect frame twice per auto
  open. Harmless on paper — both are what the meter was already sent — but
  no meter has been watched doing it.
- **Relaying UART bytes is necessary, not sufficient, for a cable to carry a
  meter.** A named meter falls back to any cable that relays UART bytes
  (`architecture.md`, Opening a meter), but the cable's head must also fit the
  meter's optical port and its bridge must run the meter's line settings: our
  init sets the CP2110 to 9600 baud and the CH9325 to 2400, then 19200. A
  fallback that can't fit costs an open and a timeout, not a wrong reading.
  Describing each cable by chip, head and line settings, and each entry by
  the heads it fits, would let the fallback and detection skip cables that
  can't carry the meter. Not needed while the fallback only costs time.

## Pending Verification

### Capture leaves the meter manually ranged

The sweep restores each setting it drove, and `docs/capture-design.md` says
the baseline is auto range with the flags off. It is not, at the end of a run:
the range walk restores Auto, then the MIN/MAX walk locks the range again (as
the meter does while recording), and leaving MIN/MAX by 0x42 does not put
auto-ranging back. Seen 2026-09-10 — one `ohm_ranges` run ended manual, and
the next run opened on a manually ranged meter, which cost it the `22MΩ` rung
because that rung was then the current choice and the sweep only walks the
others.

Not harmful — the operator's next dial turn clears it — but it makes a
resumed or repeated run cover a different set of rungs than a fresh one.
Re-asserting Auto after the flag sweeps would fix it.

### Capture: settle before HOLD, REL and MIN/MAX

- **A delay after the press cannot cover HOLD, REL or MIN/MAX**, which act on
  the live reading rather than on the meter's next one. `ohm_ranges/hold:on`
  filed 12.59 kΩ three times on the 82 kΩ resistor
  (`ut61eplus-ohm-82k-settled-2.yaml`, 2026-09-10): the press followed the
  220MΩ rung handing back to auto, so the meter froze a reading still on its
  way down and every later sample read the frozen value. Settling **before**
  the press is what would fix it — the wait is on the wrong side of the button
  for these three.

### Capture: gate steps placed after other steps

The capture run drives the meter's own settings only once every gate step has
reported, and it never sweeps a gate step itself. A family that scatters its
six gate steps through its list therefore has every step *before* the last of
them go unswept — no range ladder, no flag sub-steps. The UT61+ list did, and
lost the AC V, DC mV and AC mV ladders in the 2026-09-10 UT61B+ run (issue
#19) before being reordered; `ohm_ranges` and `dcv_ranges` were added so the
two ladders the gate steps sit on get swept as well.

The UT181A's list was reordered on 2026-09-27, after @diego351's full run
(issue #5) showed its dial order; its gate now closes before any other step.

Still split, each needing that family's own dial order and a hardware run:

- **VC-880 / VC650BT / VC-890** — `acv`, `acdcv`, `dcmv`, `dcua`, `acua`,
  `dcma`, `acma`, `dca`, `aca`.

The UT8802, UT8803, UT803, UT804, UT71A/B, UT71C/D/E, VC920 and UT171 lists
are split too, but those families declare no `choices`, so nothing would be
swept whatever the order.
The allow-list in `every_device_finishes_its_gate_before_any_other_step`
(`crates/dmm-cli/src/capture/step.rs`) names all of them; deleting an entry is
how a fix lands.

### Protocol families we have no meter for

These protocols are implemented from reverse engineering (vendor software
decompilation, community implementations), and anything known on hardware
comes from a reporter's meter. One has been: the UT804 is `Verified`
(issue #16, its block below). The VC-880, VC650BT, VC-890, UT803, UT8802,
UT8803, UT171, UT71A–E, Voltcraft VC920/VC940/VC960, UT60BT and UT202BT are
`Experimental` and have **never been tested against real hardware** — every aspect needs
end-to-end verification. The UT181A, partly verified, has
[its own list](research/ut181/verification.md).

The ask in every family's issue (#3, #4, #5, #7, #12, #13, #14, #15, #16,
#22, #23, #26, #27) is
the same: `dmm-cli --device <id> capture --unverified`, attach the report.
The issue's checklist is `dmm-cli --device <id> capture --list-steps
--format md`, so a step a report confirms flips `.verified()` in code, is
credited here in the same commit, and the checklist is regenerated into the
issue. The items below are the wire-level questions those steps answer,
plus what no step reaches.

### Capture and display checks a UT181A runs

The meter's own items are in the
[UT181A verification list](research/ut181/verification.md).

- A capture sweep after an operator step that sets a flag (`rel`, `peak`)
  clears that flag again, so the following `rel_off` / `peak_off` step
  asks for something already done. A capture-tool issue, not the meter's
- Sub-value display end to end — the GUI reading panel, recording log,
  graph selector/overlay and both CSV exports consume `aux_values`; the
  capture confirms the parser, not the display. Ask for `read --format
  csv` runs in V AC and dual-thermocouple modes (checks the `auxN_*`
  columns) and a GUI screenshot in MIN/MAX

### Bluetooth search and detection with built-in meters

- **A cached built-in meter against a known UT-D07B.** The known-peer
  fallback tries whichever cached peer the platform lists first, so an
  asleep cached 121GW, BM78xBT or ZOTEK meter costs a ~10 s connect and
  "not found" before a UT-D07B is tried. Decides whether to
  [try known adapters first](future-improvements.md#known-adapters-first-in-the-bluetooth-fallback).
  Needs a report that shows it.
- **A chance 121GW or BM78xBT claim on a UT-D07B link.** Both rules run on
  the Bluetooth link and decline AB CD frames, each other's and ZOTEK
  packets and the adapter heartbeat in tests
  ([detection design](detection-design.md)). Watch a UT-D07B report for one.

### macOS bridges other than the CH9329

- **The CP2110, CH9325 and BU-86X on macOS.** Only the CH9329 has run on
  macOS (a UT181A on its UT-D09, [PR #8](https://github.com/antoinecellerier/dmm-tools/pull/8),
  [#2](https://github.com/antoinecellerier/dmm-tools/issues/2)). Whether macOS
  opens the others as plain HID decides what `docs/setup.md` says for them.
  Needs any meter on one of those cables.

### CP2110 FIFO counts

- **The byte order of report 0x42's TX/RX FIFO counts.** UNI-T's
  SLABHIDtoUART.dll reads them big-endian, `Cp2110::uart_status`
  little-endian (UT171 spec §8); idle reads give 0, which fits either.
  Decides `uart_status`. Needs a CP2110 read with bytes queued, on any meter.

### CLI paced reads

- **Ctrl-C during a paced `read`.** The cancellable sleep kept its pacing
  over 50 reads on our UT61E+ (2026-07-29), but Ctrl-C mid-sleep has not
  been timed. Decides whether the sleep needs a shorter slice. Needs any
  polled meter and `read --interval-ms` 2000 or more.

### Streaming meters: read continuously

Since 2026-09-28 the stream reads every streaming meter (UT171, UT181A,
UT8802, UT8803, UT80x, VC-880, ZOTEK, 121GW, BM78xBT, and the UT61+ over
Bluetooth) frame by frame as it arrives, and a sample interval keeps the
frame nearest each tick; `Dmm` drops what queued after 250 ms unread.
Checked on our UT61E+ over the UT-D07B (Linux, 2026-09-28): 1 s reads
spaced three or four adapter frames apart as expected; AC+DC V at 0 ms, every
notification a reading, DC and AC alternating with none dropped; after a
10-minute GUI Pause the first reading was current and the process's memory
flat; HOLD sent after a pause landed at once; the Sample interval changed
live. Over its CP2110 cable, 0 ms and 1 s reads were unchanged, also after a
replug. Still open: a UT181A in #5 for a regression read and `set hold`/`set
rel` after a Pause, and a HID streaming meter (below).

- **Linux HID drops the newest.** Confirmed in the kernel source
  (2026-09-28, `drivers/hid/hidraw.c` `hidraw_report_event`: a report that
  would fill the 64-slot ring is skipped). A HID streaming meter left unread
  therefore queued the oldest reports, not the newest. Since 2026-09-28 the
  stream reads a streaming meter continuously and `Dmm` drops the queue
  after 250 ms unread, so this only matters if a check shows readings behind
  the LCD: run a UT8803 or UT8802 at `--interval-ms 2000` on Linux and
  compare each reading with the LCD.
- **A paused Bluetooth session.** Where its notifications wait while nothing
  reads, and whether anything bounds them. Checkable on our UT-D07B: pause
  the GUI for 10 minutes, resume, and see that the first reading is current
  and memory stayed flat.
- **Wall times across a suspend.** Since 2026-09-28 each reading's wall
  time is the system time it was taken at, not a mapping from the session's
  start, whose monotonic clock stops during a suspend on Linux. Checkable:
  suspend the laptop mid-session with the mock and with our UT61E+, then
  compare the exported rows after resume with the wall clock.
- **A polled reply that comes late.** Since 2026-09-28 a request after a
  timeout starts from an empty queue, so a reply that lands after the 2 s
  deadline is dropped rather than taken for the next request's answer
  (UT61+ on its cable, VC-890; the BM86x always did). Not seen on a meter:
  does a UT61E+ reply ever take over 2 s, capacitance included?


### Vendor sources not yet read

Found by the 2026-09-19 surveys (`docs/research/new-device-candidates.md`,
"Sources"). Each is a separate task:

- **UNI-T's general-purpose PC software** ("优利德上位机软件", `1.10.zip`
  2025-05-26 and `Setup.zip` 2026-09-09, about 150 MB each), in the bench
  download centre's UT80 and UT88 results. Unopened; it may drive several
  bench meters (the UT632's check is in its
  [verification list](research/ut632/verification.md#vendor-sources)).
- **Protocol documents for families we don't support**: the older UT61E and
  UT61B (both are the chipset datasheets — ES51922 and FS9922-DMM3 — not
  UNI-T documents), and the Voltcraft VC-870 (Conrad item 124603, IN01).

### Known defects

Bugs a reader can reproduce, with the cause and fix where known.

- **A switched-off paired Bluetooth peer listed as heard.** Under BlueZ, a
  paired UT-D07B switched off at its power switch shows as heard in
  `dmm-cli list`, not "paired but not heard", and the doc-screenshot guard
  refuses to run. With no address named, the open ranks it Heard and tries
  it first; its connect times out and is reported as a Bluetooth error, not
  "No meter found", since the known-peer fallback skips a Heard peer. Cause:
  BlueZ keeps a paired device's last RSSI and reports it during our scan
  (UT-D07B spec §4), and `standing()` in `transport/ble/search.rs` takes it
  as heard. Fix, untried: count only RSSI changes seen during our own scan.
- **A UT804 LO reading is stored as 0.0.** In 4-20 mA % the LCD shows
  `- LO. %` (ut803 spec §3.3), but the parser reports `Normal(0.0)` with
  the text "L0.", so statistics, the graph and exports count a zero.
  Fix, untried: `MeasuredValue::NoReading("LO")`.
- **The `dc` flag has no definition.** `StatusFlags::dc` carries no doc,
  and families set it differently on AC+DC: the UT804 sets it from
  coupling 3, the BM86x and BM78xBT clear it, ZOTEK sets it from its DC
  annunciator, the UT61E+ within its AC+DC modes. Fix: define it in
  `flags.rs` and align the families.
- **HOLD on the AC component of AC+DC V blanks the reading.** Held there,
  a UT61E+ sends only AC frames (ut61eplus spec §2.7), which carry no main
  reading: the reading shows blank digits beside its AC row, and the Main
  view's minimap, cursors and statistics stay empty. **Plot:** AC draws it.
  Fix, untried: show the held component as the reading.
- **`--integrate` skips AC+DC V's DC readings at slow intervals.** At
  `--interval-ms` 1000 or more the DC readings can be over 2 s apart, past
  the integrator's gap limit (`Integrator::max_dt_secs`), so they are
  dropped. Fix, untried: scale the limit to the interval.
- **A software offset shifts the AC component of AC+DC V.** It applies to
  the AC sub-value like any same-unit sub-value, which means nothing for an
  RMS value. Fix, untried: exempt AC sub-values from offsets.
- **The mock offers Peak outside the AC modes.** Its Hz, Ω, capacitance,
  temperature and NCV scenarios take Peak, where the UT61E+ it stands in for
  offers it only in `AC_PEAK_MODES` (`protocol/ut61eplus/tables/mod.rs`).
  Cause: `Scenario::peak_applies` (`mock/scenarios.rs`) excludes only the DC
  scenarios. Fix: narrow it to the AC ones.

### Entering NCV leaves the previous mode's trace on the graph

`Graph::push_sample` is what detects a mode change and clears history, but in
NCV mode every sample is `MeasuredValue::NcvLevel`, which never reaches it —
the App's `resolve_plot_input` returns `None` for a level with no place on a
value axis, so nothing is pushed. So switching to NCV leaves the previous
mode's data on screen indefinitely, labelled with the old unit.

Reproducible without hardware via the mock's `ncv` scenario. Fixing it means
routing non-plottable samples through something that carries mode/unit, and
establishing the time origin without any plottable points. That is also the
prerequisite for banding NCV — see `docs/future-improvements.md`.

### A flat trace labels its y-axis with six decimals

Seen 2026-09-19 on single-frame replays of UT61E+ golden frames: a constant
118.3V trace labels two grid lines "118.300000 V" each, and a zero current
trace reads "-0.000000 A". The auto Y range presumably collapses to a tiny
span around the value and the label formatter takes its digits from that
span. Reproducible without hardware by replaying one frame from
`crates/dmm-lib/tests/golden/ut61eplus/lpfv.yaml`.

### A meter power cycle surfaces a checksum error

Seen on our UT61E+ (2026-09-17, `dmm-gui`, CP2110): powering the meter off
and back on at a different dial position showed `checksum mismatch: expected
0x3534, got 0x055a` as a UI error. The received value is ASCII `"54"` —
display digits where the checksum should be — while the computed one is a
plausible sum for a whole frame, so the frame boundary was lost rather than
the data corrupted.

`read_frame` leaves `rx_buf` untouched when a read times out
(`crates/dmm-lib/src/protocol/framing.rs`, the `n == 0` arm). A meter that
stops mid-frame leaves a partial frame there; when it comes back, the new
stream is appended to it, so `locate` finds the old `AB CD`, the length byte
points into fresh data, and the bytes at the checksum position are payload.

Recovery already works — the UT61+ propagates the error and clears the buffer
on the way out (the vendor parser's discard-and-clear, ut61eplus spec §2.1),
so the next read is clean. What it costs is one user-visible error for an
ordinary action.

Two candidate fixes: clear `rx_buf` when a read times out, which removes the
cause and also covers UT803/UT804 (the other family that propagates), or
retry once after the clear. A timeout means 2 s without a single byte
(`read_uart_bytes` returns 0 only at the deadline), and no frame we handle
takes that long to arrive, so keeping a partial frame across one buys
nothing. Reproducible without hardware: feed `read_frame` a partial frame, a
transport that returns no bytes once, then a fresh stream.

### A settings file this build can't parse is replaced by defaults

`Settings::load` (`crates/dmm-gui/src/settings.rs`) falls back to
`Settings::default()` when `settings.json` does not deserialize, and the next
`save()` — any settings click — writes the defaults over the file. The likely
trigger is a downgrade: an older build meeting a theme or colour preset
variant only a newer one knows fails the whole file, not that one key.

Left as it is for now. If it becomes worth fixing: recover key by key —
parse into a `serde_json::Value`, deserialize each field on its own and keep
the default only for the fields that fail.

### GUI accessibility — screen reader walk-through

The GUI accessibility pass wired up AccessKit labels, toggle-state
announcements, focus rings on custom widgets, modal focus trapping, a
text summary on the plot, a polite live region on the primary reading,
and landmark roles (Toolbar / Main / Status).

**Keyboard accessibility — verified.** Manual keyboard-only walk-through
confirmed:

- Tab order is sensible across every panel (top bar, graph toolbar,
  plot, stats, recording, remote controls, settings including the
  expanded Customize colors section).
- Visible focus rings appear on every Tab stop, including the color-
  picker swatches, the graph minimap, the recording-panel resize
  divider, and the left-panel resize handle.
- Arrow-key behaviour: Left/Right pans the minimap when focused;
  arrow keys adjust the saturation/value 2D area and the hue 1D
  gradient inside the color-picker popup; Up/Down resizes the
  recording-panel divider; Left/Right resizes the left panel handle.
- Modal focus trapping: opening the `?` shortcut help moves focus
  inside the modal, Tab cycles within it, and closing (via Esc, the
  × button activated with Space, or clicking outside) restores focus
  to the `?` button that opened it. The version-label → What's New
  viewport follows the same pattern.

**Screen reader walk-through — still pending.** What needs manual
verification:

- **Orca on Linux** (AT-SPI): Tab through every interactive widget and
  confirm each announces a sensible name. Toggle HOLD/REL/RANGE/AUTO/
  MIN-MAX/PEAK/LIVE and confirm "pressed"/"not pressed" is spoken.
  Check that the plot's state summary is read when focused and that
  the main reading updates are announced politely (not continuously).
  Confirm Orca's landmark-nav shortcut (Orca+Ctrl+Shift+L) lists
  Toolbar, Main, Status.
  With a sub-value-capable meter (UT181A, or the mock), also confirm:
  the graph toolbar's **Plot:** chips announce as "Plot \<name\>" radio
  buttons and the **Show:** chips as "Show \<name\> trace" toggles, each
  with its selected/pressed state, so the two rows are told apart by
  ear; the reading's live region speaks the sub-values (and a MIN/MAX
  extreme's "at N seconds") after the mode without flooding while the
  meter streams; and the plot summary's "Also showing …" phrase names
  the drawn traces.
  With a software scale applied (the **Scale** row), also confirm: the
  reading's live region ends with ", software scaled" and drops the
  phrase again when scaling is turned off; the **Scale** button
  announces its on/off state; and the three fields announce as "Scale
  factor", "Offset" and "Unit label" from their hint text.
- **NVDA or JAWS on Windows** (UI Automation): same checks, since
  AccessKit's Windows backend is separate from AT-SPI.
- **VoiceOver on macOS** (NSAccessibility): same checks on the third
  backend.
- **Hover tooltips are invisible to assistive tech.** egui 0.36 never
  calls AccessKit's `set_description`, so `on_hover_text` reaches sighted
  users only — any control whose meaning lives solely in its tooltip is
  unexplained to a screen reader. The toolbar chips work around this by
  folding the group into their accessible name. A follow-up could add an
  `a11y_description` helper mirroring `on_hover_text` and apply it where
  the tooltip carries real information.

Report findings by opening a GitHub issue; the docs should be updated
to reflect what is actually confirmed working and what still needs fixes.
