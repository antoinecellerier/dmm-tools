# Changelog

## Unreleased

### New device support

| Family | Models | Transport | Status |
|--------|--------|-----------|--------|
| **UT8802** | UT8802 | CP2110 | Experimental |
| **UT803/UT804** | UT803, UT804 | CH9325 | Experimental — FS9721 LCD protocol |
| **VC-880** | VC-880, VC650BT | CP2110 | Experimental |
| **VC-890** | VC-890 | CP2110 | Experimental |

### GUI

- **Configurable color theme** — presets (Default, Colorblind, High Contrast, Monochrome) and per-color overrides in settings.
- **Big meter mode** — `Ctrl+B` cycles off / full / minimal. Minimal mode inlines mode/flags on the same line as the value. Dynamic font scaling and responsive layout at small window sizes.
- **CLI arguments** — `--device`, `--theme`, `--mock-mode` flags for the GUI binary. `--mock-mode` implies `--device mock`.
- **Always-on-top setting** — keeps the GUI window above other windows.
- **Hide window decorations setting** — removes title bar and window borders.
- **`--renderer` flag** — select wgpu or glow backend; automatic fallback from wgpu to glow on GPU errors.
- **`--adapter` flag** — select USB adapter when multiple are connected. Shows connected devices inline when the adapter doesn't match.
- **App icon and desktop integration** — `.desktop` file and icon for Linux, GNOME `app_id` for alt-tab identification.
- **Time-integral in cursor readout** — when both cursors are placed on a current or voltage graph, the readout now shows ∫ (integral) alongside ΔT and ΔV. For current modes, this displays charge (mAh/Ah/µAh). For voltage modes, V·s.
- **Running integral in statistics** — a cumulative integral line ("∫") appears in the statistics panel for current and voltage modes. Resets with the Reset button or Ctrl+L.
- **MIN/MAX and Peak buttons now cycle without exiting** — clicking cycles MAX ↔ MIN (or P-MAX ↔ P-MIN), matching the real device's short-press behavior. A separate "x" button exits the mode.
- Auto-stop recording when buffer is full instead of silently dropping samples.
- Show filename in CSV export toast.
- Clear cursors on graph reset and mode change.
- **Minimap bracket resize handles** — drag the bracket edges to resize the viewport to an arbitrary time width. The opposite edge stays anchored. Cursor changes to a resize icon on hover.
- **"What's New" changelog popup** — shown automatically on first launch after a release upgrade. Click the version label in the top bar to re-open. Full changelog rendered with proper markdown formatting.

### CLI

- **`--adapter` flag** — select USB adapter when multiple are connected.
- **`--integrate` flag** on the `read` command — adds cumulative time-integral columns (`integral`, `integral_unit`) to CSV and JSON output. Text format appends `[∫ value unit]`. The session summary includes the total integral and elapsed time. Useful for battery capacity measurement (coulomb counting).
- Device model metadata (`device_model`, `device_serial`) in CSV and JSON exports.
- Show activation instructions on timeout errors.
- Handle EINTR in read loop so Ctrl-C prints the session summary cleanly.

### Library

- **CH9329 transport** — support for the UT-D09 USB cable (used by UT171, UT181A, and other models).
- **CH9325 transport** — HID bridge for UT803/UT804 family.
- **FS9721 frame extractor** — LCD segment protocol decoder for UT803/UT804.
- **`AuxValue` type** and `aux_values` field on `Measurement` for secondary readings.
- **`StatusFlags` additions** — `lead_error`, `comp`, `record` fields.
- **`Integrator` struct** (`stats.rs`) — trapezoidal-rule time integrator with gap detection (max_dt guard), overload gap handling, and clock-backward safety via `checked_duration_since()`.
- **`integral_unit_info()`** — maps measurement units to integral display units (A→Ah, mA→mAh, µA→µAh, V→V·s, mV→mV·s).
- UT181A: range labels, precision-byte display formatting, capture steps for format verification.
- Mock MIN/MAX and Peak behavior updated to match real device.
- Renamed QinHeng transport to CH9325 for consistency with CP2110/CH9329.
- Removed deprecated CP2110-only open functions.

### Bug fixes

- Fix bar graph byte decoding: use decimal division, not nibble shift.
- Fix GUI error detection for missing USB cable.
- Fix big meter scaling: hash-based cache key, wrap oscillation fix.
- Fix UT181A measurement format parsing for all variants.
- Fix GUI MIN/MAX and Peak buttons immediately exiting instead of cycling through states.

### Build

- macOS builds (ARM + Intel) in CI and release workflows.
- Linux ARM and Windows ARM release builds.
- Treat compiler warnings as errors across all builds.

### Internal

- **Dependency updates** — eframe/egui 0.31→0.34 (new Panel API, font hinting, viewport improvements), egui_plot 0.31→0.35 (per-axis bounds, filled areas, grid styling), egui_commonmark 0.20→0.23, rfd 0.15→0.17, console 0.15→0.16. Replaced deprecated `serde_yaml` with `serde_yaml_ng` (drop-in, addresses RUSTSEC-2025-0068).
- Consolidate transport VID/PID definitions into `KNOWN_TRANSPORTS` array.
- Extract shared VC-880/VC-890 protocol code into `vc8x0_common` module.
- `ThemeColors::pick()` helper to reduce dark/light branching boilerplate.
- Various deduplication: specs rendering, `lookup_range()`, `CaptureStep::empty_result()`.

### Documentation

- Verified MIN/MAX, Peak, and SELECT2 protocol behavior against real UT61E+ hardware. Updated `docs/protocol.md` and `docs/verification-backlog.md`.
- Verified HV flag, DC V range table, and DC mV mode.
- Transport landscape documentation (QinHeng HID, UCI SDK).
- Experimental macOS support docs and platform hints.
- UT803/UT804 protocol findings and reverse engineering approach.

## v0.3.0

### Specifications, Keyboard Shortcuts & Mock Device

This release adds live per-range specification display from device manuals, full keyboard navigation, screen reader support, and a simulated mock device for testing without hardware. Under the hood, a central device registry simplifies adding new meters, and a large refactoring improves code organization with 282 tests (up from 209).

### GUI

- **Specifications panel** — shows per-range resolution, accuracy (with frequency bands for AC), input impedance, and notes from the device manual. Updates live as the meter changes mode/range. Covers UT61E+, UT61B+, UT61D+, UT161 family, and Mock. Includes "Manual" hyperlink when a URL is configured.
- **Keyboard shortcuts** — `Ctrl+Shift+C` connect, `Space` pause, `Ctrl+L` clear, `Ctrl+R` record, `Ctrl+E` export, `Ctrl+±/0` zoom, `[`/`]` time window, arrows/Home/End graph navigation. Press `?` for in-app reference.
- **Accessibility** — AccessKit labels on icon-only buttons (`?`, gear) and the minimap for screen reader support.
- **Responsive top bar** — wraps to two rows when the window is too narrow.
- **Mock device** — simulated device in the device selector for testing and demos. Cycles through DC V, AC V, Ohms, Capacitance, Hz, Temperature, DC mA, Overload, and NCV. Configurable via "Mock mode" setting. Remote control buttons toggle flags, SELECT advances mode.
- **Device registry** — device selector populated from a central registry. Adding a new device requires zero GUI code changes.
- Display unit now uses the same font size as the measurement value.
- Recording sample values use `display_raw` for stable formatting.

### CLI

- **Mock device** (`--device mock`) — simulated measurements without hardware. Supports `read` (with `--mock-mode` to pin a specific mode) and `command`. Useful for testing output formats and scripting.
- **Device registry** — `--device` flag resolved through central registry. Adding a new device requires zero CLI code changes.
- Validation messaging now directs users to `capture` instead of `debug` for device verification.

### Library

- **Device registry** (`protocol/registry.rs`) — single source of truth for all selectable devices with IDs, aliases, display names, activation instructions, and protocol factory functions.
- **Specification data** — per-range accuracy, resolution, input impedance, and notes for UT61E+, UT61B+, UT61D+, and UT161 family, transcribed from device manuals. Accessible via `lookup_spec()` and `lookup_mode_spec()`.
- **Mock protocol** — `MockProtocol` implementing the `Protocol` trait with configurable scenarios, flag toggling, and mode cycling.
- `Measurement` string fields use `Cow<'static, str>` to avoid heap allocation for static table data.
- `RunningStats` moved to lib crate for reuse across CLI and GUI.
- Shared `read_frame()` helper in framing module.
- Golden tests switched from JSON to capture-compatible YAML format.

### Bug fixes

- Use `checked_duration_since()` instead of `duration_since()` in graph gap detection — prevents panic on backward clock jumps (VM suspend, NTP correction).
- Fix tab order for top bar right-side items (settings gear, help link).
- Fix angle brackets in docs rendered as invisible HTML on GitHub.

### Internal

- Large-scale refactoring: split `app.rs` into focused modules (graph, recording, display, settings, theme, controls), extracted shared helpers, deduplicated test utilities, replaced magic numbers with named constants.
- 282 tests (up from 209 in v0.2.0).

### Documentation

- End-to-end guide for adding device support (`docs/adding-devices.md`).
- Non-UNI-T device candidate research and VC880/VC650BT implementation plan.
- GUI reference, CLI reference, UX design, and architecture docs updated for all new features.
- `dump_specs` example for verifying specification data against manuals.

**Full Changelog**: https://github.com/antoinecellerier/dmm-tools/compare/v0.2.0...v0.3.0

## v0.2.0

### Multi-Device Protocol Support

Rearchitects the library to support multiple UNI-T multimeter families behind a common Protocol trait.

### New device support

| Family | Models | Status |
|--------|--------|--------|
| **UT61+/UT161** | UT61E+, UT61B+, UT61D+, UT161B/D/E | Verified (UT61E+), device tables for all |
| **UT8803** | UT8803, UT8803E | Experimental — streaming protocol, 21-byte frames |
| **UT171** | UT171A/B/C | Experimental — streaming, float32 values, 26 modes |
| **UT181A** | UT181A | Experimental — streaming, float32 + unit strings, 97 modes |

### CLI

- `--device` flag for selecting device family.
- `command` subcommand accepts free-form command names; run with no args to list available commands per device.
- JSON output includes `"experimental": true/false` field.
- Experimental warning on connect for unverified protocols.
- Device-specific activation instructions shown after 5 consecutive timeouts.

### GUI

- Device selector in settings panel.
- EXPERIMENTAL badge in top bar for unverified protocols.
- Device-dependent remote controls — buttons only shown for supported commands.
- Device name shown in top bar.
- Float value display fallback for protocols without ASCII display strings.

### Library

- `Protocol` trait: `init`, `request_measurement`, `send_command`, `get_name`, `profile`, `capture_steps`.
- Unified `Measurement` type with string-based mode/unit/range fields.
- Shared framing functions for BE16, alternating-byte, 1-byte LE16, and 2-byte LE16 checksums.
- Golden file test infrastructure.
- 209 tests passing.

**Full Changelog**: https://github.com/antoinecellerier/dmm-tools/compare/v0.1.0...v0.2.0

## v0.1.0

First release of dmm-tools — CLI and GUI for the UNI-T UT61E+ multimeter over USB.

### CLI

- Live measurement streaming with text, CSV, and JSON output.
- Remote control — send button presses (hold, rel, range, min/max, peak, light, select).
- Shell completions for bash, zsh, fish, and PowerShell.
- Raw hex dump mode for protocol debugging.
- Guided protocol capture wizard for bug reports and verification.

### GUI

- Real-time value display with large monospace readout and flag badges.
- Time-series graph with 10K-point scrollable history and minimap.
- Graph overlays: mean line, min/max envelope, reference lines with trigger markers, measurement cursors.
- Statistics (min/max/avg) for all data and visible window.
- Recording with CSV export (up to 500K samples).
- Remote control buttons with live state highlighting.
- Big meter mode, responsive layout, light/dark themes, UI zoom, persistent settings.
- Auto-connect and auto-reconnect.
