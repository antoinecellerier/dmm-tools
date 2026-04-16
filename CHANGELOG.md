# Changelog

## Unreleased

### GUI

- **Keyboard and screen reader accessibility pass.** Every interactive widget now has a proper accessible name, including icon-only buttons, color-picker swatches, the graph minimap, the recording-panel resize bar, the left side-panel resize handle, and the clickable version label (which now announces "Show release notes"). Toggleable device buttons (HOLD, REL, RANGE, AUTO, MIN/MAX, PEAK) and the graph's LIVE button announce their on/off state instead of relying on color alone. The main reading is exposed as a polite live region so screen readers announce updates at natural pauses, and active status flags (HOLD/REL/MIN/MAX/AUTO/...) are spoken alongside the value. The graph exposes a one-line text summary (time window, Y range, sample count, live/paused, last reading using the meter's own digit string). The top bar, main content area, and connection status region are marked as Toolbar / Main / Status landmarks for flat-review navigation. The `?` help overlay traps focus inside and restores focus to the `?` button on close (the **What's New** window opens as a separate OS window with its own focus management; closing it restores focus to the version label). Custom widgets respond to arrow keys when focused: Left/Right pans the minimap, Up/Down resizes the recording divider, Left/Right resizes the left side-panel handle, and arrow keys edit the color-picker hue and saturation/value sliders. Tab focus is visible on custom-painted widgets (color swatches, minimap, split bars, Customize colors disclosure) that previously had no visible focus ring.
- **Educational hover tooltips on every control** — every button, link, toggle, checkbox, and setting in the GUI now has a descriptive hover tooltip explaining what it does, plus its keyboard shortcut when one exists. First-time users can discover every feature by hovering. Device command buttons (HOLD, REL, RANGE, AUTO, MIN/MAX, PEAK, SELECT, LIGHT) use device-agnostic wording so tooltips stay accurate across all supported meter families.
- **Graph bounding-box zoom** — hold Shift and drag on the graph to draw a rectangle; on release the view zooms to the selected time and value range. Press Escape to cancel mid-drag. A new **Reset Zoom** toolbar button (also bound to double-click on the plot) returns the graph to live follow with auto Y.
- **Help overlay now covers mouse gestures** — the `?` popup (renamed "Keyboard & Mouse") gains a Graph (mouse) section documenting drag, shift-drag zoom, scroll-wheel zoom, double-click reset, cursor placement, and minimap drag.
- **Reconnection shows progress and the latest error.** When the background thread loses the device and enters its auto-reconnect loop, the connection-status button now displays the attempt counter (e.g. "Reconnecting (attempt 3)...") and its hover tooltip shows the most recent reconnect failure reason. A Disconnect click is also picked up immediately instead of waiting up to 2 s for the next retry interval.

### Bug fixes

- **Graph refresh no longer scales with total history length.** As the capture buffer accumulated samples (e.g. several minutes of mock data with the visible window pinned to 5 s), the GUI became visibly slower frame-over-frame. Every per-frame helper (Y-bounds, statistics, envelope, crossings, nearest-point lookup) scanned the entire 10 000-point history instead of the visible slice. Now each pass binary-searches to the visible window first, reducing per-frame work from O(total) to O(log n + visible). The minimap's accidental O(n²) Y-range recomputation and the envelope's O(n²) inner scan are also fixed.

- **Graph drag-to-pan now works in live mode.** Previously the click-drag pan gesture was silently inert whenever the graph was following live data — only worked once the view was already on a non-live segment. Starting a drag in live mode now snaps the view to the current end of data and drops out of live, mirroring the scroll-wheel-in-live-mode behaviour.
- **Graph X-axis labels gain sub-second precision when zoomed in.** Previously a tight zoom (e.g. a 0.5 s span) produced duplicate labels like "9 s" / "9 s" because the formatter was hardcoded to integer seconds. Labels now read the grid step size and add decimal seconds when it's sub-second — e.g. "9.1 s", "9.25 s", "1m 30.5s".
- **Graph points now use the measurement's acquisition timestamp.** Previously each point was re-timestamped at UI message-drain time, so UI work (drag, zoom, toasts) or CPU load visibly warped the graph's X axis. Points now carry the timestamp captured at `request_measurement`, so the X axis reflects when the sample was taken, not when the UI processed it.
- **Measurement pacing uses absolute-tick scheduling.** Both the GUI background thread and the CLI `read`/`debug` loops now sleep until the next tick boundary instead of sleeping for a fixed interval after each read, eliminating cumulative drift when `request_measurement` varies in duration or the thread is delayed.
- **Recorded and exported timestamps now reflect acquisition time.** Recording samples and CLI CSV/JSON output previously timestamped rows with `Local::now()` at the moment of formatting, so UI drain latency or buffering offset the column from the real sample time. Both paths now translate the measurement's monotonic `Instant` through a session-long `WallClock` origin pair, so exported timestamps line up with when the device produced each reading.
- **Integrator surfaces silently skipped intervals.** Samples spaced further apart than the 2 s integration limit are still skipped (to avoid spikes after disconnects), but the CLI `read --integrate` summary and the GUI stats panel now flag how many intervals were skipped instead of silently returning `0`. A warning is also logged on the first skip.

### Internal

- **Mock device waveforms are now a function of elapsed time**, not a per-read step counter. Displayed mock curves stay on the ideal smooth shape regardless of read cadence or scheduling jitter — the root cause of apparent "jitter" in mock-mode graph rendering.
- **`dmm_lib::WallClock`** — small `(Instant, SystemTime)` origin helper used by the GUI recording and CLI formatter to derive wall-clock timestamps from monotonic `Instant`s.
- **Spec metadata flows through the measurement, not a GUI-side lookup.** Per-range resolution/accuracy and per-mode input-impedance/notes are now attached to each `Measurement` by `Dmm::request_measurement` via new `Protocol::spec_info` / `Protocol::mode_spec_info` trait methods. The GUI reads these fields directly instead of reaching into `ut61eplus::tables` with the device-family string, so adding spec tables for other families only requires the new protocol to override the trait method. `SpecInfo` / `ModeSpecInfo` / `AccuracyBand` moved to a top-level `dmm_lib::specs` module and are re-exported from `ut61eplus::tables` for backward compatibility.
- **Shared measurement loop.** The CLI `read`/`debug` commands and the GUI background thread now both drive acquisition through a new `dmm_lib::stream::MeasurementStream` helper that owns absolute-tick pacing and consecutive-timeout counting. Pacing and timeout-threshold behavior now lives in one place instead of three, and future tuning (backoff, adaptive rate) has a single attachment point.

## v0.4.0

### Breaking changes

- **Renamed crates and binaries** to remove the UT61E+-specific naming now that the project supports seven device families. The CLI binary is now `dmm-cli` (was `ut61eplus`), the GUI binary is now `dmm-gui` (was `ut61eplus-gui`), and the library crate is now `dmm-lib` (was `ut61eplus-lib`). Workspace directories moved to `crates/dmm-{lib,cli,gui}/`. Update any scripts, install commands, or `RUST_LOG` filters accordingly:

  ```sh
  cargo install --git <url> dmm-cli dmm-gui       # was: ut61eplus-cli / ut61eplus-gui
  RUST_LOG=dmm_lib=trace dmm-cli debug             # was: ut61eplus_lib / ut61eplus
  ```

  The GUI settings file also moves, from `~/.config/ut61eplus/settings.json` to `~/.config/dmm-tools/settings.json` (and equivalent on other platforms). **Existing settings will not be migrated automatically** — copy the old file manually or re-configure from scratch. Device IDs (`--device ut61eplus`) and protocol module names (`dmm_lib::protocol::ut61eplus::*`) are unchanged — they refer to the UT61-plus device protocol family, not the tool.

- **CLI `--device` default changed.** The CLI no longer silently defaults to `ut61eplus`. New resolution order: explicit `--device` flag → `device_family` in the shared `settings.json` (written by `dmm-gui`) → registry default (currently `ut61eplus`). When the final fallback is used, the CLI prints a dim one-line notice on stderr suggesting the user either pass `--device` or set it in the GUI settings. Scripts that relied on the implicit default still work but will print the notice; add `--device ut61eplus` to silence it.

### New device support

| Family | Models | Transport | Status |
|--------|--------|-----------|--------|
| **UT8802** | UT8802 | CP2110 | Experimental |
| **UT803/UT804** | UT803, UT804 | CH9325 | Experimental |
| **VC-880** | VC-880, VC650BT | CP2110 | Experimental |
| **VC-890** | VC-890 | CP2110 | Experimental |

**UT181A**: now **partially verified on real hardware** — init handshake, frame extraction, and VDC float32 parsing confirmed on a real UT181A (CH9329 cable) by [@alexander-magon](https://github.com/alexander-magon) ([issue #5](https://github.com/antoinecellerier/dmm-tools/issues/5), [PR #8](https://github.com/antoinecellerier/dmm-tools/pull/8)). Other modes, format variants (Relative, Min/Max, Peak, COMP), and recording features still need testing.

### GUI

- **Configurable color theme** — presets (Default, Colorblind, High Contrast, Monochrome) and per-color overrides in the settings panel. All choices respect light/dark mode.
- **Big meter mode** — `Ctrl+B` cycles off / full / minimal, with a matching toolbar toggle button. Minimal mode inlines mode and flags on the same line as the value. Font scaling and layout adapt to small window sizes.
- **Time-integral cursor readout** — when both cursors are placed on a current or voltage graph, the readout adds ∫ alongside ΔT and the value delta. Current modes display charge (Ah/mAh/µAh); voltage modes display V·s / mV·s.
- **Running integral in statistics** — cumulative ∫ row in the statistics panel for current and voltage modes. Resets with the Reset button or `Ctrl+L`.
- **MIN/MAX and Peak buttons now cycle without exiting** — clicking cycles MAX ↔ MIN (or P-MAX ↔ P-MIN), matching the meter's short-press behavior. A separate "x" button exits the mode.
- **Minimap bracket resize handles** — drag a bracket edge to resize the viewport to an arbitrary time width. The opposite edge stays anchored; the cursor changes to a resize icon on hover.
- **"What's New" changelog popup** — shown automatically on first launch after a release upgrade, and reopenable by clicking the version label in the top bar. Renders the embedded `CHANGELOG.md` with full markdown formatting.
- **Experimental device feedback links** — the EXPERIMENTAL badge is now a clickable link to the per-device GitHub verification issue, with a model-specific tooltip. The cable-not-found help panel includes the same link.
- **GUI command-line arguments** — the GUI binary now accepts `--device`, `--theme`, `--mock-mode`, `--renderer`, and `--adapter`. Overrides are session-only and don't overwrite saved settings; `--mock-mode` implies `--device mock`.
- **`--renderer` flag** — select `wgpu` or `glow` backend. Defaults to `wgpu` with automatic fallback to `glow` if GPU init fails (e.g. OpenGL 2.1-only GPUs like the Raspberry Pi 3).
- **`--adapter` flag** — select a specific USB adapter (by serial number or HID device path) when multiple are connected. Available on both the GUI and CLI; mismatch errors enumerate connected adapters inline.
- **Always-on-top and hide-decorations settings** — pin the window above others or remove the title bar and window borders.
- **App icon and Linux desktop integration** — embedded SVG/PNG icon used as the window icon on all platforms, plus auto-installed `.desktop` entry and GNOME `app_id` so Wayland compositors and alt-tab show the correct app name and icon.
- Auto-stop recording when the buffer is full (500K samples) instead of silently dropping new samples — a toast announces the stop.
- Show the saved filename in the CSV export toast.
- Clear cursors on graph reset and on mode change.

### CLI

- **`--integrate` flag** on the `read` command — adds cumulative time-integral columns (`integral`, `integral_unit`) to CSV and JSON output, appends `[∫ value unit]` in text format, and prints the total integral and elapsed time in the session summary. Useful for coulomb counting (e.g. battery capacity).
- **`--adapter` flag** — select a specific USB adapter when multiple are connected. See the GUI section above for details (same flag, same precedence).
- **Device model metadata** in exports — `device_model` and `device_serial` columns/fields in CSV and JSON output.
- Show device-specific activation instructions after repeated timeouts.
- Handle `EINTR` in the read loop so Ctrl-C prints the session summary cleanly.

### Library

- **CH9329 transport** — UT-D09 USB cable support (used by UT171, UT181A, and others).
- **CH9325 transport** — WCH/QinHeng HID bridge used by the UT803/UT804.
- **FS9721 frame extractor** — LCD-segment protocol decoder used by the UT803/UT804.
- **`Measurement::aux_values`** with new `AuxValue` type, for secondary readings (e.g. UT181A's frequency/duty alongside primary value).
- **`StatusFlags`** gains `lead_error`, `comp`, and `record` fields.
- **`Integrator`** in `stats.rs` — trapezoidal-rule time integrator with gap detection, overload handling, and clock-backward safety via `checked_duration_since()`. Powers both the GUI and CLI integral features.
- **`integral_unit_info()`** maps measurement units to display units for integrals (A→Ah, mA→mAh, µA→µAh, V→V·s, mV→mV·s).
- UT181A: range labels, precision-byte display formatting, and capture steps for format verification.
- Mock MIN/MAX and Peak behavior updated to match the real device.

### Bug fixes

- Fix UT181A device init commands (thanks to [@alexander-magon](https://github.com/alexander-magon), [PR #8](https://github.com/antoinecellerier/dmm-tools/pull/8)).
- Fix UT181A measurement format parsing for all variants.
- Fix UT61E+ bar graph byte decoding — bytes 9–10 are decimal digits (`tens*10 + ones`), not a nibble shift.
- Fix GUI cable-not-found detection so the help panel actually appears.
- Fix big meter scaling: hash-based cache key, no more wrap oscillation at threshold widths.

### Build

- macOS builds (ARM + Intel) in CI and release workflows.
- Linux ARM and Windows ARM release builds.
- Treat compiler warnings as errors across all builds.

### Internal

- **Dependency updates** — eframe/egui 0.31→0.34 (new Panel API, font hinting, viewport improvements), egui_plot 0.31→0.35 (per-axis bounds, filled areas, grid styling), egui_commonmark 0.20→0.23, rfd 0.15→0.17, console 0.15→0.16. Replaced deprecated `serde_yaml` with `serde_yaml_ng` (drop-in, addresses RUSTSEC-2025-0068).
- **New `dmm-settings` crate** owns the config-file schema that both the CLI and GUI agree on (currently just `device_family`). The GUI's full `Settings` struct flattens `SharedSettings` via `#[serde(flatten)]` so the on-disk JSON shape is unchanged, but the contract between the two binaries is now compile-enforced by a single Rust type rather than a string literal in two places. GUI-only settings (color overrides, panel visibility, theme, zoom) stay in `dmm-gui`.

### Documentation

- Verified MIN/MAX, Peak, and SELECT2 protocol behavior against real UT61E+ hardware.
- Verified HV flag, DC V range table, and DC mV mode.

**Full Changelog**: https://github.com/antoinecellerier/dmm-tools/compare/v0.3.0...v0.4.0

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
