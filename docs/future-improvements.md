# Future Improvements

Ideas for features that would add meaningful value to the tool. Organized by category with rough complexity estimates. None of these are committed — they're here to capture intent and help prioritize.

Contributions and feedback welcome via [GitHub Issues](https://github.com/antoinecellerier/dmm-tools/issues).

---

## Monitoring & Alerts

### Threshold alarms

**Complexity:** Medium

Configurable high/low thresholds that trigger visual and audible alerts when a measurement crosses a boundary.

- CLI: `--alarm-high 5.0 --alarm-low 3.0` flags, warning lines to stderr
- GUI: threshold lines on graph with active monitoring, toast and optional sound on breach, breach count in stats panel

Use cases: unattended battery discharge testing, thermal monitoring, production go/no-go checks.

### Pass/fail testing mode

**Complexity:** Medium

Define a nominal value and tolerance (e.g., `5.0V +/-2%` or `4.9V..5.1V`), display live pass/fail status with color coding. Log results to CSV with timestamps.

Use cases: production testing, incoming inspection, calibration verification.

---

## Multi-Meter Support

### Simultaneous dual-channel display

**Complexity:** High

Connect two meters and display both with a synchronized timeline — overlaid or stacked graphs. Derived math channels (e.g., V \* A = W for power measurement).

- CLI: multiple `--device` / `--adapter` pairs
- GUI: split or overlaid graph view with per-channel controls

Use cases: power measurement (voltage + current simultaneously), differential measurements, comparison testing.

---

## Data Analysis

### Standard deviation in statistics

**Complexity:** Low

Add standard deviation to the existing min/max/avg statistics panel using an incremental algorithm (Welford's method). No extra memory required.

Use cases: noise assessment, measurement stability evaluation.

### Histogram / distribution view

**Complexity:** Medium

Toggleable panel showing a live histogram of recorded values with bin count, mean, and standard deviation. Reveals measurement distribution, noise characteristics, and outliers at a glance.

Use cases: stability assessment ("is this 5V rail actually stable?"), QA workflows, metrology.

### Allan deviation

**Complexity:** Low-medium

Compute and display Allan deviation (ADEV) — the standard metric for measurement stability vs. averaging time. Shows how long to average for a given precision.

Use cases: precision measurement, oscillator characterization, sensor evaluation.

---

## Software transforms

Software-side transforms re-express or derive readings on the PC rather than in
the meter. A 2026-09 survey of bench DMM math menus, community loggers' math
channels and handheld conventions (sources below) found one single-channel
transform that every source shares and no supported meter can do for itself: a
linear scale with a unit relabel (clamp mV/A, shunt, probe divider, sensor
maps). That one shipped: the CLI's `--scale --offset --unit` and the GUI's
**Scale** row; see
[Scaling readings in software](cli-reference.md#scaling-readings-in-software)
and [Scale](gui-reference.md#scale). The entries below are the survey's other
candidates.

A software REL is left out on purpose: every supported family but the UT171
does REL in firmware and flags it in its readings, and the UT61E+ family,
VC880/VC890 and UT181A take it as a remote command, so a client-side duplicate
would confuse more than it adds.

The planned transforms fit the **Derived series model** in
[architecture.md](architecture.md). One that produces a new quantity is
appended as a sub-value, which the graph's **Plot:** / **Show:** chips and the
CSV aux columns already handle. Where it goes is chosen explicitly (a future
`--as LABEL` flag / "as" combo box), never inferred.

### Moving average / smoothing

**Complexity:** Low

Window-N average of the main reading, exposed as a derived series (Keysight
"Smoothing", SmuView "Moving average", TestController Average/FilterLP). The
graph already offers a mean line and a min/max envelope, so this is only worth
building if users want the smoothed value itself in the statistics panel and
the CSV.

Use cases: noisy sensors, slow thermal drift.

### dBV / dBm

**Complexity:** Low-medium

20·log10(V) and 10·log10(V² / R / 1 mW), the latter against a configurable
reference impedance (UT181A presets 4–1200 Ω, Fluke defaults to 600 Ω).
Offered only in voltage modes.

Use cases: audio and RF level checks on meters without a dB function (UT61E+).

### Percent deviation

**Complexity:** Low

(x − nominal) / nominal × 100 (Fluke REL %, Keysight %, Keithley percent).
Overlaps the Pass/fail testing mode item above; worth considering as part of
that item rather than a separate control.

Use cases: tolerance checks.

### Formula transform

**Complexity:** Medium

A free-form expression as a second mode of the same Scale control
(`--formula '20*log10(x)' --unit dBV`, a **(Linear)(Formula)** chip pair in the
GUI). Variables are named from series labels: `x` for the main reading in base
units, then `frequency`, `t2`, …, with a second meter's series prefixed `m2_`.

The evaluator must live outside `dmm-lib`, which stays self-contained. Checked
2026-09: exmex 0.21.0 (MIT OR Apache-2.0, maintained) is the candidate.
evalexpr is AGPL-3.0-only after 11.3.1, which this GPL-3.0-or-later workspace
can't take; fasteval is unmaintained since 2020 and meval calls itself a toy.

Use cases: thermistor β equations, dB, anything non-linear.

### Two-channel math

**Complexity:** High

V × I, A − B and similar across two meters. Depends on
[Simultaneous dual-channel display](#simultaneous-dual-channel-display) for
timestamp-aligned frames; given those, the **Derived series model** in
[architecture.md](architecture.md) needs no new concept to express it.

Use cases: power, differential temperature with two single-input meters.

### Per-mode transforms

**Complexity:** Medium

A transform is applied to every reading regardless of what the dial is on,
so a clamp factor set up for mV DC also scales the ohms reading after a
dial turn, and a `--unit` relabel hides the unit change that would otherwise
reset the statistics. Binding a transform to the mode and base unit it was
defined for — applying it only while the meter is in that mode, and showing
it as armed but idle otherwise — is the prerequisite for any persistence or
preset, since a stored factor is only safe once it cannot fire on the wrong
quantity.

Use cases: leaving a clamp factor configured while using the meter for other
checks; presets that survive a dial turn.

### Named transform presets

**Complexity:** Low

Explicitly applied presets (`--preset clamp10`, a preset combo box in the GUI)
rather than persisting the last transform, so a stale factor is never silently
applied at startup.

Use cases: recurring bench setups.

### Sources

Handhelds:
- [Issue #5 comment with the UT181A vendor-app screenshots](https://github.com/antoinecellerier/dmm-tools/issues/5#issuecomment-5507498410)
- [UT181A operating manual](https://www.batronix.com/pdf/uni-t/UT181A-Manual-English.pdf)
- [Fluke 287/289 users manual](https://assets.fluke.com/manuals/287_289_umeng0100.pdf)
- [Fluke 52 II dual thermometer](https://www.fluke.com/en-us/product/temperature-measurement/ir-thermometers/fluke-52-ii)
- [Fluke: using accessory current clamps with DMMs](https://www.fluke.com/en-us/learn/blog/clamps/using-accessory-current-clamps-with-fluke-dmms)

Bench meters:
- [Keysight Truevolt math scaling](https://rfmw.em.keysight.com/bihelpfiles/Truevolt/WebHelp/US/Content/__E_Features%20and%20Functions/Math-Scaling.htm)
- [Keysight 34401A math functions KB](https://docs.keysight.com/kkbopen/can-i-have-multiple-math-functions-null-min-max-db-dbm-limit-on-at-the-same-time-on-the-34401a-588262739.html)
- [Keithley DMM6500 review (lygte-info)](https://lygte-info.dk/review/DMMKeithley%20DMM6500%20UK.html)

Logging software:
- [TestController math channels](https://lygte-info.dk/project/TestControllerMath%20UK.html)
- [TestController EEVblog thread](https://www.eevblog.com/forum/testgear/program-that-can-log-from-many-multimeters/)
- [SmuView manual](https://knarfs.github.io/doc/smuview/0.0.4/manual.html)
- [PicoLog 6 math channels](https://www.picotech.com/library/knowledge-bases/data-loggers/picolog-6-math-channels)
- [FlukeView Forms](https://www.fluke.com/en-us/product/fluke-software/fluke-fvf-sc2-flukeview-forms-software)
- [UNI-T UT61E software (lygte-info review)](https://lygte-info.dk/review/DMMUNI-T%20UT61E%20UK.html)
- [curioustech UT181A Windows app](https://www.curioustech.net/ut181a.html)
- [QtDMM](https://github.com/jhol/qtdmm)
- [UT61E-Toolkit](https://github.com/Jakeler/UT61E-Toolkit)
- [ut61e_plus_logger](https://github.com/kevontheweb/ut61e_plus_logger)

---

## Lab Integration & Automation

### Network measurement server

**Complexity:** Medium-high

Expose live measurements over TCP as newline-delimited JSON (e.g., `dmm-cli serve --port 5025`). Clients connect and receive a stream of measurement objects.

Use cases: LabVIEW/Python script integration, Grafana dashboards, headless Raspberry Pi monitoring setups, custom test automation.

### MQTT publishing

**Complexity:** Low-medium

Publish measurements to an MQTT broker for integration with IoT and lab automation ecosystems.

Use cases: Home Assistant, Node-RED, InfluxDB/Grafana pipelines, multi-meter aggregation.

---

## Data Replay & Export

### Compare against an imported run

**Complexity:** Medium

An import replaces the session. Overlaying a saved run on the live graph instead — a reference boot sequence against today's, say — needs a second trace with its own time alignment (start, a marker, a threshold crossing) and its own entry in the plot key, and leaves the live session's statistics and recording alone.

Use cases: checking a board's power-up against a known-good capture; regression testing a firmware change.

### Graph image export

**Complexity:** Medium

Export the current graph view as PNG or SVG for reports and documentation.

Use cases: test reports, lab notebooks, sharing results.

### Export without freezing the window

**Complexity:** Medium

The GUI renders an export on the UI thread once its save dialog returns, so the window stops for as long as the render takes: a fraction of a second at the default Buffer size, seconds at a few million samples. Moving the render off the UI thread needs the pinned samples shared with that thread rather than borrowed from the store.

---

## Graph Enhancements

### Placing markers in the CLI

**Complexity:** Low

The GUI marks readings (`N`, `Ctrl+N`) and exports the markers as `marker,note` CSV columns and JSON keys; `dmm-cli read` cannot place any yet. Pressing Enter, with optional typed text, would mark the latest reading through the same shared writers.

### The graph's readings in the recording log

**Complexity:** Low

With nothing recorded the log lists only markers, though the graph's full readings are kept for Export…. A toggle showing them there would let the log's `+` mark any of them, as it does a recording's.

### More entries in the plot's right-click menu

**Complexity:** Low

The plot's right-click menu only offers **Add marker here**. Candidates: **Delete marker N** when right-clicking a flag (flags are widgets of their own over the plot, so each needs its menu and a way to report the delete), **Place cursor here**, and **Back to live** / **Reset zoom**, which repeat the double-click and `End`.

Neither the menu nor the log row's `+` is reachable from the keyboard, so only `N` and `Ctrl+N` mark readings without a pointer. Opening the menu with `Shift+F10` or the Menu key on the focused plot would give the keyboard a way to earlier readings. The menu also searches only the plotted series: where only sub-values are drawn it has no reading to offer.

### Rendering the meter's other reported conditions

**Complexity:** Medium

Overloads are drawn as a filled band, distinct from the dashed edges used for
data loss (see the GUI reference). Several other states the meter reports are
still drawn as ordinary readings:

- **NCV** — the level plots as steps on its own whole-number axis. It could be
  drawn as a band instead.
- **HOLD** — the display is frozen, so the same value repeats and draws as a
  flat live trace.
- **MIN / MAX / peak** — the meter is showing a stored extreme, not the
  present reading.
- **REL** — values are deltas from a reference rather than absolutes.
- **`lead_error`** — lead placement is wrong, but the reading still plots.

Splitting data loss by cause would help too: pause, connection loss and a
sample interval longer than the gap threshold all render identically today,
though the App knows which occurred when it calls `Graph::push_data_loss`.

Once several *filled* kinds coexist, hue stops being enough to tell them
apart. Hatched fills are the non-colour answer; `egui_plot` fills only in a
flat colour, so the stripes would be hand-painted.

Use cases: telling "the meter said something unusual" apart from "the meter
said nothing", without having to cross-check the recording.

### Cursor readouts clear of lines and the trace

**Complexity:** Low

A cursor readout prefers a corner around its point that the trace doesn't cross, but settles for one it does before trying a row further out, and it ignores marker and cursor lines (`cursor_label_rect`). Next to a marker at a step in the trace, it sits on the step with the marker's line through it. Weighing a row further out against a crossed corner, and stepping along the row past the lines as the mean and reference labels do, would keep it clear.

Use cases: reading a cursor at a marked event.

### One bound for AC+DC V's two traces

**Complexity:** Low

In AC+DC V the graph bounds DC points and AC points separately, so it can reach back about twice as far as the History buffer before either drops, bending the one shared `max_samples` bound. Counting both components against one bound keeps the graph and History in step.

### Steadier dense traces in live view

**Complexity:** Medium

Zoomed out to many samples per pixel, a noisy trace's edges shimmer slightly as live view scrolls: each new reading moves the view by a fraction of a pixel and changes which samples land in each column. It shows most at 100 % display scale. Drawing every sample shimmers too, and re-cutting the thinning spans, by session time or by the screen's pixel columns, doesn't help. Candidates: advance the live view in whole-pixel steps rather than by each reading's fraction, or cut the drawn line down to the minimap's per-bucket extents, which don't move within a bucket.

### A sub-value's line style at wide zoom

**Complexity:** Medium

Zoomed out on a noisy sub-value, its dashes merge into a solid band, so only colour tells it from the plotted series, against the rule that colour is never the only cue. A fix must keep every extreme visible: dashes laid along time were tried and drop a one-sample spike or a vertical Min/Max step that falls in a gap. Candidates are a lighter fill or an outline for a dense band. **Graph lines: Solid** sidesteps it for those who chose it.

### Sub-value traces across a prefix step

**Complexity:** Medium

A sub-value whose unit steps a decade mid-capture (a 121GW or ZOTEK frequency flipping between Hz and kHz) restarts its trace, as the plotted series does, and a scaled reading's **Raw** in mV gets an axis apart from a V reading. Keeping each trace in its base unit and choosing the axis prefix from the range shown would keep the history and share the axis.

Use cases: a frequency near 1 kHz over an afternoon; Raw beside its scaled reading.

### Fixed bounds for a right axis

**Complexity:** Low

**Y:Fixed** holds every axis where it is, and a box zoom pins each to its share of the box, but the min/max fields set the plotted unit's axis only. Typing a right axis's bounds would need a pair of fields per unit in the toolbar, or a menu on the axis; its range is otherwise reached by switching **Plot:** to it.

Use cases: a frequency held at 49.5–50.5 Hz while the voltage beside it auto-scales.

### XY plot of two series

**Complexity:** Medium

The graph draws every series against time. Plotting one recorded series against another — V against Hz, or the two thermocouples of a UT181A against each other — shows how one quantity follows the other rather than how each varies. Open questions: pairing samples that arrive at different times (nearest frame, as the hover does), and showing time on the trace, with a colour ramp or markers along it.

Use cases: a supply's output voltage against its load current; a sensor's reading against temperature.

### Sampling in step with streaming meters

**Complexity:** Medium

A meter with `Delivery::Streamed` is read continuously, and a sample interval keeps the frame nearest each tick. At **Every reading** (0 ms) each frame is kept, repeats included: a UT181A sends a frame every 100 ms but changes its reading every 500 ms, so each reading lands about five times. A repeated frame and a new reading of the same value arrive as identical bytes, so nothing can drop the repeats afterwards.

A design should sample once per meter update by default; a UT181A's changes land on fixed 500 ms boundaries once one is seen. It must cope with irregular updates (the UT181A's temperature dial: every 700–900 ms). The graph's gap detector and the **Buffer size** estimate assume today's frame rate and would have to change with it.

Use cases: a CSV whose rows are the meter's readings, not its frames. Raised by @diego351's UT181A at 2 Sa/s ([issue #5](https://github.com/antoinecellerier/dmm-tools/issues/5)).

### Measurement rate display

**Complexity:** Low

Show actual samples/second in the status bar or connection info area.

Use cases: verifying the meter is communicating at the expected rate, detecting connection degradation early.

---

## Specifications Panel

### Overload protection

**Complexity:** Low

Every family's spec data carries each mode's overload protection ("1000V", "Fuse 1A 240V"), but the panel shows only resolution, accuracy, input impedance and notes. Show it as a line of its own, with the fuse's full printed text (designator, size, type), kept in the spec tables' `// Printed` comments, as a tooltip or in `dump_specs`. Mock-ups first: the one-line layouts already leave out the impedance.

Use cases: knowing what an input survives before probing, and which fuse to buy after one blows.

### Frequency and duty specs from the dial position

**Complexity:** Medium

On the UT61+, Hz (0x04) and Duty % (0x05) send the same mode byte from every dial position, so a reading taken with Hz/% on the V~, mV, µA, mA or A position shows the Hz/% position's Frequency/Duty Ratio row. The manual gives those readings terms of their own: the AC V remarks (PDF p. 15) take frequency over 40Hz~500Hz (UT61B+), 40Hz~1kHz (UT61D+) or 40Hz~10kHz (UT61E+) at ≥10% of the range and call duty "for reference only"; the AC current remarks (PDF p. 18) ask for ≥50% of the range and give the UT61B+/UT61D+ frequency ±(0.1%+4) at 0.1Hz. Showing them means inferring the dial position from the reading history, as `DialState` in `protocol/cycle.rs` does for mode selection.

---

## Device-Specific

### UT181A stored data retrieval

**Complexity:** Medium-high

The UT181A has built-in recording and saved measurement features (protocol commands 0x07-0x0F) that aren't implemented yet. Download stored recordings and saved measurements from the meter, display in the GUI graph view, and export to CSV. It also needs response types 0x03-0x05 and 0x72, the packed timestamps they carry (spec §9), and SET_REFERENCE (0x03) to type a REL reference; none has run against a meter. The vendor app caps a recording name at 9 characters; whether the meter takes 10 is untested (spec §10.1).

Use cases: retrieving field measurements logged by the meter itself, longer recording sessions than USB-tethered capture allows.

### 121GW SD-card log import

**Complexity:** Medium

The 121GW logs to its micro SD card as CSV: sample, then the main and the secondary function, value and unit (manual p.54). Importing such a file would open it as a session, as `--replay` opens a recording.

Use cases: reviewing a field log taken without Bluetooth, on the same graph and statistics as a live session.

### OWON offline-record read-back

**Complexity:** Medium

OWON's "+" B-series meters, the OW16B, OW18B, OW18E and CM2100B log up to 10,000 readings while disconnected; FFF1 commands start a recording and read it back as a dump on FFF4 (owon spec §7.2, §8). Read it into the GUI graph and the CSV/JSON export; what a meter still has to answer is in the family's [verification list](research/owon/verification.md#offline-records).

Use cases: a log taken away from the computer, on the same graph and statistics as a live session.

### BM520s logged-memory download

**Complexity:** Medium

The BM521s and BM525s log up to 10800 and 87000 readings, which Brymen's memory commands (`00 52 88`, `…89`, `…8A`) page out in 24-byte blocks (bm86x spec §10). Download them into the GUI graph and the CSV/JSON export; what a meter still has to answer is in the family's [verification list](research/bm86x/verification.md#logging-models).

Use cases: logging sessions recorded in the field and retrieved later, as with the UT181A.

### Where Bluetooth streaming is decided

**Complexity:** Low

The UT61+ protocol sends the UT-D07B's start command on every Bluetooth link, adapter or built-in radio (`init` in `protocol/ut61eplus/mod.rs`), so the family and the link decide streaming today. Whether it belongs to the link, the peer or the family is settled when a meter needs otherwise: the UT117C/UT197/UT219PV group ([candidates](research/new-device-candidates.md)), or a UT171 or UT181A behind a UT-D07A ([UT-D07B checks](research/ut-d07b/verification.md#meters-behind-the-adapter)).

### Known adapters first in the Bluetooth fallback

**Complexity:** Low

With no meter named, the Bluetooth search falls back to peers the platform knows (paired or cached) when the scan misses. It tries whichever the platform lists first, so a cached built-in meter that is asleep costs a ~10 s connect and "not found" before a known UT-D07B is tried. Trying known adapters ahead of known built-in meters fixes that, once a report shows it matters.

### Cables described by head and line settings

**Complexity:** Medium

A named meter falls back to any cable that relays UART bytes ([Opening a meter](architecture.md#opening-a-meter)), but relaying is not enough: the cable's head must fit the meter's optical port, and its bridge must run the meter's line settings, where our init sets the CP2110 to 9600 baud and the CH9325 to 2400, then 19200. A fallback that cannot fit costs an open and a timeout, not a wrong reading. Describing each cable by chip, head and line settings, and each entry by the heads it fits, would let the fallback and detection skip those cables, once the time matters.

### Bluetooth notifications read in the background

**Complexity:** Medium

Under **Bluetooth runs only inside its own calls** in [architecture.md](architecture.md), a streaming meter's notification is read and stamped as it arrives, except while the caller does something else between two reads: a remote-control walk, a slow terminal. If a reading ever shows a stamp late enough to matter for that reason, revisit the decision: a task that stamps each notification on arrival and keeps a bounded buffer would date every frame correctly, at the cost of the thread and channel the decision avoids.

---

## Usability

### Configurable CSV columns

**Complexity:** Low

Let users choose which columns appear in CSV export (e.g., drop flags, include raw hex, reorder columns). Different workflows need different formats.

Use cases: spreadsheet import, database ingestion, test report generation.

### Log file rotation

**Complexity:** Low-medium

Auto-rotate log files by size or time (e.g., new file every hour or every 100 MB) for long-term unattended monitoring.

Use cases: multi-day environmental monitoring, production line logging.

### Maybe: egui's special-cased colours

**Complexity:** Low

Two colours stay on egui's shipped values on purpose, as egui's own idiom
rather than the app's: the text caret and hyperlinks. Should it ever be
wanted: the light hyperlink colour is 2.77:1 on the panel, and caret and links
could follow Accent the way the selection does (7.80:1 dark / 5.41:1 light in
the Default preset).

Use cases: matching a bench's colour conventions, high-contrast setups, colour-vision needs beyond the two stock themes.

### Session notes in exports

**Complexity:** Low

A `--note "Battery discharge test, cell #47"` option that embeds user-provided context in CSV/JSON file headers.

Use cases: organizing and identifying captures, adding test context without external documentation.

### A newer release's notes in the app

**Complexity:** Low-medium

The top bar links to a newer release's page on GitHub. The What's New window could show that release's notes instead, from the same response the update check already reads — at the cost of rendering Markdown fetched from the network.
