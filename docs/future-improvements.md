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

## Lab Integration & Automation

### Network measurement server

**Complexity:** Medium-high

Expose live measurements over TCP as newline-delimited JSON (e.g., `ut61eplus serve --port 5025`). Clients connect and receive a stream of measurement objects.

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
- CLI: `ut61eplus analyze capture.csv --stats`

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

### Auxiliary value display

**Complexity:** Low-medium

Surface the `aux_values` field in the GUI for meters that report multiple simultaneous values. When the meter is in REL or MIN/MAX mode, show both the live reading and the stored reference/min/max values.

Use cases: seeing both relative and absolute values at a glance, monitoring min/max without exiting the mode.

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
