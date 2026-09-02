# Future Improvements

Ideas for features that would add meaningful value to the tool. Organized by category with rough complexity estimates. None of these are committed — they're here to capture intent and help prioritize.

Contributions and feedback welcome via [GitHub Issues](https://github.com/user/dmm-tools/issues).

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
the meter. Research (2026-09) across bench DMM math menus (Keysight Truevolt:
Null, dB/dBm, %, Mx-B, Statistics, Limits, Smoothing; Keithley DMM6500: mx+b,
percent, reciprocal), community loggers (TestController's math channels,
SmuView's six math channel types, PicoLog 6's equation builder) and handheld
conventions (Fluke 287/289 REL, REL %, dBm and temperature offset; UT181A REL,
dBV, dBm, LPF, COMP and T1−T2) found exactly one single-channel transform that
every source shares and no supported meter can do for itself: a linear scale
with a unit relabel (clamp mV/A, shunt, probe divider, sensor maps). That one
shipped — the CLI's `--scale --offset --unit` and the GUI's **Scale** row; see
[Scaling readings in software](cli-reference.md#scaling-readings-in-software)
and [Scale](gui-reference.md#scale).

A software REL was deliberately not added: every supported family already does
REL in firmware, and the UT61E+ family, VC880/VC890 and UT181A take it as a
remote command, so a client-side duplicate would confuse more than it adds.

**Model.** A *frame* is one measurement's named series — Main plus sub-values
by label. A derived series is a triple of (label, unit, op). An op that
re-expresses the reading (Linear) replaces Main and keeps the meter's own value
as `Raw`; an op that produces a new quantity is appended as a sub-value, which
the graph's **Plot:** / **Show:** chips and the CSV aux columns already handle.
Placement is explicit (a future `--as LABEL` flag / "as" combo box), never
inferred. Variables in a future formula are named from series labels — `x` for
the main reading in base units, then `frequency`, `t2`, … — with a second
meter's series prefixed `m2_`.

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
GUI). The evaluator must live outside `dmm-lib`, which stays self-contained.
Candidates checked 2026-09-02: exmex 0.21.0 (MIT OR Apache-2.0, maintained) is
the recommendation; evalexpr is popular but AGPL-3.0-only after 11.3.1 and this
workspace is GPL-3.0-or-later, so it is out; fasteval is unmaintained since
2020; meval describes itself as a toy.

Use cases: thermistor β equations, dB, anything non-linear.

### Two-channel math

**Complexity:** High

V × I, A − B and similar across two meters. Depends on
[Simultaneous dual-channel display](#simultaneous-dual-channel-display) for
timestamp-aligned frames; given those, the model above needs no new concept to
express it.

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

- [Issue #5 comment with the UT181A vendor-app screenshots](https://github.com/antoinecellerier/dmm-tools/issues/5#issuecomment-5507498410)
- [UT181A operating manual](https://www.batronix.com/pdf/uni-t/UT181A-Manual-English.pdf)
- [Fluke 287/289 users manual](https://assets.fluke.com/manuals/287_289_umeng0100.pdf)
- [Fluke 52 II dual thermometer](https://www.fluke.com/en-us/product/temperature-measurement/ir-thermometers/fluke-52-ii)
- [Keysight Truevolt math scaling](https://rfmw.em.keysight.com/bihelpfiles/Truevolt/WebHelp/US/Content/__E_Features%20and%20Functions/Math-Scaling.htm)
- [Keysight 34401A math functions KB](https://docs.keysight.com/kkbopen/can-i-have-multiple-math-functions-null-min-max-db-dbm-limit-on-at-the-same-time-on-the-34401a-588262739.html)
- [Keithley DMM6500 review (lygte-info)](https://lygte-info.dk/review/DMMKeithley%20DMM6500%20UK.html)
- [TestController math channels](https://lygte-info.dk/project/TestControllerMath%20UK.html)
- [TestController EEVblog thread](https://www.eevblog.com/forum/testgear/program-that-can-log-from-many-multimeters/)
- [SmuView manual](https://knarfs.github.io/doc/smuview/0.0.4/manual.html)
- [PicoLog 6 math channels](https://www.picotech.com/library/knowledge-bases/data-loggers/picolog-6-math-channels)
- [Fluke: using accessory current clamps with DMMs](https://www.fluke.com/en-us/learn/blog/clamps/using-accessory-current-clamps-with-fluke-dmms)
- [UNI-T UT61E software (lygte-info review)](https://lygte-info.dk/review/DMMUNI-T%20UT61E%20UK.html)
- [curioustech UT181A Windows app](https://www.curioustech.net/ut181a.html)
- [QtDMM](https://github.com/jhol/qtdmm)
- [UT61E-Toolkit](https://github.com/Jakeler/UT61E-Toolkit)
- [ut61e_plus_logger](https://github.com/kevontheweb/ut61e_plus_logger)
- [FlukeView Forms](https://www.fluke.com/en-us/product/fluke-software/fluke-fvf-sc2-flukeview-forms-software)

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

### CSV replay / offline analysis

**Complexity:** Medium

Load a previously recorded CSV file back into the GUI for analysis — graph, statistics, cursors, all working on historical data without a connected meter.

- GUI: `--replay capture.csv` flag
- CLI: `dmm-cli analyze capture.csv --stats`

Use cases: post-hoc analysis, sharing captures with colleagues, comparing measurements from different sessions.

### Graph image export

**Complexity:** Medium

Export the current graph view as PNG or SVG for reports and documentation.

Use cases: test reports, lab notebooks, sharing results.

---

## Graph Enhancements

### User annotations / event markers

**Complexity:** Medium

Drop timestamped markers on the graph with optional text labels (e.g., "applied 10A load", "switched to battery"). Markers appear as vertical lines and are included in recording exports.

Use cases: correlating measurement changes with physical events, making captured data meaningful after the fact.

### Rendering the meter's other reported conditions

**Complexity:** Medium

Overloads are drawn as a filled band, distinct from the dashed markers used
for data loss (see the GUI reference). Several other states the meter reports
are still drawn as ordinary live readings, or not at all:

- **NCV** — `MeasuredValue::NcvLevel` is shown and recorded but never reaches
  the graph at all. It could be banded, or plotted on its own 0-4 scale.
  Blocked on the mode-clear bug in the verification backlog.
- **HOLD** — the display is frozen, so the same value repeats and draws as a
  flat live trace.
- **MIN / MAX / peak** — the meter is showing a stored extreme, not the
  present reading.
- **REL** — values are deltas from a reference rather than absolutes.
- **`lead_error`** — lead placement is wrong, but the reading still plots.

Splitting data loss by cause would help too: pause, connection loss and a
sample interval longer than the gap threshold all render identically today,
though the App knows which occurred and already reports it via
`Graph::push_data_loss`.

Once several *filled* kinds coexist, hue stops being enough to tell them
apart. Hatched fills are the non-colour answer — note `egui_plot` has no
pattern support (`Span::fill` and `Polygon::fill_color` take a flat colour),
so it means hand-painting stripes with screen-space spacing via
`PlotTransform`, clipped to the band and the plot rect.

Use cases: telling "the meter said something unusual" apart from "the meter
said nothing", without having to cross-check the recording.

### Measurement rate display

**Complexity:** Low

Show actual samples/second in the status bar or connection info area.

Use cases: verifying the meter is communicating at the expected rate, detecting connection degradation early.

---

## Device-Specific

### UT181A stored data retrieval

**Complexity:** Medium-high

The UT181A has built-in recording and saved measurement features (protocol commands 0x07-0x0F) that aren't implemented yet. Download stored recordings and saved measurements from the meter, display in the GUI graph view, and export to CSV.

Use cases: retrieving field measurements logged by the meter itself, longer recording sessions than USB-tethered capture allows.

### UT181A primary/secondary display switching

**Complexity:** Medium

The vendor app's Setting panel switches the meter's primary and secondary
display function — VAC / VAC,Hz / Peak / LowPass / dBV / dBm for voltage,
T1,T2 / T2,T1 / T1−T2 / T2−T1 for temperature, and T1,T2 / REL for the
secondary display — by sending set-function commands, and the reporter on
issue #5 asked for the same here. The command bytes still have to be traced
from the vendor software and verified on hardware; nothing is designed yet.

Use cases: putting the meter in T1−T2 or dBm from the PC so the LCD and the
software agree.

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

### Session notes in exports

**Complexity:** Low

A `--note "Battery discharge test, cell #47"` option that embeds user-provided context in CSV/JSON file headers.

Use cases: organizing and identifying captures, adding test context without external documentation.
