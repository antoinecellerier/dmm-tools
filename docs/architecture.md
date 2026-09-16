# Architecture

## Crate Layout

```
dmm-tools/
├── crates/
│   ├── dmm-lib/       # Core library
│   ├── dmm-settings/  # Shared config schema (CLI ↔ GUI)
│   ├── dmm-cli/       # CLI binary
│   └── dmm-gui/       # GUI binary
```

### dmm-lib

The library crate handles all device communication and data parsing. It has no UI dependencies.

**Module responsibilities:**

| Module | Responsibility |
|--------|---------------|
| `transport/mod.rs` | `Transport` trait abstracting HID I/O; `Box<dyn Transport>` delegation for runtime transport selection; `MockTransport` for tests |
| `transport/cp2110.rs` | CP2110 HID transport: open device, init UART, read/write interrupt reports |
| `transport/ch9329.rs` | CH9329 HID transport: open device, read/write 65-byte HID reports |
| `transport/ch9325.rs` | CH9325 HID transport: 8-byte reports with 0xF0+len framing, dual baud rate probing (2400/19200) |
| `protocol/mod.rs` | `Protocol` trait (object-safe), `DeviceFamily` enum, `DeviceProfile`, `Stability`, `Setting`/`Choice` for absolute setting selection |
| `protocol/registry.rs` | Device registry: `SelectableDevice` entries, factory functions, `resolve_device()` lookup. CLI and GUI use the registry for device selection — no device-specific code in app crates. |
| `protocol/cycle.rs` | Cycle-to-target driver shared by the UT61+ and Voltcraft families: presses a ring button (SELECT, Hz/%, SHIFT/SETUP, RANGE, MIN/MAX, PEAK) and reads back until the named mode, rung or flag state shows; mode walks are planned over a per-model dial table because the meter never reports the dial |
| `protocol/framing.rs` | Message framing: find `AB CD`, `0xAC`, or FS9721 index-nibble header, extract payload, validate checksum (or position/index validation); build the `AB CD` command frame the UT61+ and Voltcraft families send |
| `protocol/ut61eplus/` | UT61E+ family: `Ut61PlusProtocol`, `Mode` enum, `Command` enum, `tables/` (per-model `ModeTables` impls — one match per mode returning ranges and specs — behind the `DeviceTable` trait) |
| `protocol/ut8802/` | UT8802 family: `Ut8802Protocol` — streaming protocol with 0x5A trigger, 0xAC 8-byte BCD frames |
| `protocol/ut8803/` | UT8803 family: `Ut8803Protocol` — streaming protocol with 0x5A trigger |
| `protocol/ut80x/` | UT803/UT804: `Ut80xProtocol` — streaming, proprietary structured data (CH9325 HID) |
| `protocol/ut171/` | UT171 family: `Ut171Protocol` — streaming protocol, float32 LE values |
| `protocol/ut181a/` | UT181A: `Ut181aProtocol` in `mod.rs` (streaming driver, device-sent unit strings); `parse.rs` decodes the normal, REL, MIN/MAX, Peak and COMP payloads, `command.rs` builds the AB CD command frames and reads the OK/ER reply, `mode.rs` holds the dial families SET_MODE and SET_RANGE move within |
| `protocol/vc8x0/` | Voltcraft VC-880/VC650BT and VC-890: `Vc8x0Protocol<M>` in `mod.rs` implements `Protocol` and `CycleMeter` once over a `Vc8x0Model`; `vc880.rs` (streaming) and `vc890.rs` (polled 0x5E, 60K counts, 66-byte frames) hold each family's tables, dial, frame layout and the drain or ack around its I/O |
| `measurement.rs` | `Measurement` struct: mode, value, unit, flags (protocol-agnostic); `AuxValue` sub-values, with `AuxValue::export_cells` + `Measurement::export_aux_slots` supplying the cells and slot order `export.rs` lays out (the slot helper keeps a software-appended sub-value in a fixed column as the meter's own count changes) |
| `export.rs` | `CsvLayout`: the CSV header and row cells shared by the CLI and GUI exporters, so the two writers cannot disagree on columns (cells only — the `csv` crate stays in the binaries) |
| `transform.rs` | `Transform`: opt-in software scale/offset/unit-relabel over the main reading (shunt and clamp factors, °C→°F). `si_prefix()` converts to the base SI unit first so a factor survives auto-ranging; the meter's own reading is kept as the `Raw` sub-value |
| `stats.rs` | `RunningStats` (min/max/avg), `Integrator` (trapezoidal time-integral with gap handling), and `SeriesStats` — the mode/unit-keyed session both the CLI read loop and the GUI drain accumulate into, so the two agree on what starts a new series |
| `mock/` | `MockProtocol`, the hardware-free meter the GUI, CLI demos and screenshots run against: `scenarios.rs` (one waveform-driven scenario per `MockMode`), `state.rs` (HOLD/REL/range/MIN-MAX/Peak and how they filter a reading), `mod.rs` (the `Protocol` impl). It stands in for a UT61E+ — same mode and range bytes, same spec table — and reaches its settings through `protocol/cycle.rs`, so a choice list that works here works on hardware |
| `flags.rs` | `StatusFlags`: Hold, Rel, Auto, Min/Max/AVG, Peak, Low Battery |
| `error.rs` | `Error` enum via `thiserror` |
| `binary_help.rs` | `--version` / `--device` / `--mock-mode` help text, the "USB cable not found" setup hint and the experimental-protocol warning, shared by both binaries. Lives here because the lists come from the registry and `MockMode::ALL`, so a new device or mock scenario reaches both `--help` outputs automatically. Build values (`CARGO_PKG_VERSION`, `GIT_HASH`) are passed in by the caller. |
| `docs_tables.rs` | Renders the `--device` table in `docs/cli-reference.md` from the registry; a `dmm-cli` test keeps the file's `devices:start`/`devices:end` block in sync and rewrites it under `UPDATE_DOCS=1` (see `docs/development.md`) |
| `lib.rs` | `Dmm` struct: top-level API tying everything together |

**Data flow:**

```
CLI/GUI ──► registry::resolve_device()
                       │
                       └──► SelectableDevice.new_protocol()
                                           │
USB HID ──► Cp2110 or Ch9329 (Box<dyn Transport>) ──► Box<dyn Protocol> ──► Measurement { mode, value, unit, flags }
                                           │
                                           ├── Ut61PlusProtocol  (polled, AB CD framing, per-model DeviceTable)
                                           ├── Ut8802Protocol    (streaming, 0xAC 8-byte BCD, no checksum)
                                           ├── Ut8803Protocol    (streaming, AB CD 21-byte, BE checksum)
                                           ├── Ut171Protocol     (streaming, float32 LE)
                                           ├── Ut181aProtocol    (streaming, device-sent units)
                                           ├── Vc880Protocol     (streaming, AB CD framing, ASCII values)
                                           └── Vc890Protocol     (polled, AB CD framing, 60K counts)
```

`Dmm<T: Transport>` holds a `Box<dyn Protocol>`. The `Protocol` trait provides `init()`,
`request_measurement()`, `parse_payload()`, `send_command()`, `choices()`/`select()`,
`get_name()`, `profile()`, and `capture_steps()`. Each family implements its own framing,
parsing, and command encoding internally, but all produce the same `Measurement` struct.

Remote control has two paths. `send_command()` sends a named button press and reads nothing
back. `choices(Setting, &Measurement)` lists the values a setting (`Mode`, `Range`, `Hold`,
`Rel`, `MinMax`, `Peak`) can take from where the meter sits, each a `Choice { id, label,
current }`, and `select(Setting, id)` switches to one, confirmed from the stream. The UT181A
answers with direct commands; the UT61+/UT161 and Voltcraft families go through
`protocol/cycle.rs`, and so does the mock, for everything but its mode selector. Both default
to "unsupported", and the CLI and GUI hide any setting whose list has fewer than two entries.

**Device registry** (`protocol/registry.rs`) is the single source of truth for all selectable
devices. Each `SelectableDevice` entry contains an ID, display name, aliases, activation
instructions, and a factory function that creates the correct `Protocol` instance. The CLI
and GUI resolve user input via `resolve_selection()` — `Selection::Auto` for `AUTO_DEVICE_ID`
(`"auto"`), `Selection::Device` for anything `resolve_device()` knows — and connect via
`open_device_by_id_auto()`; they never match on `DeviceFamily` variants or instantiate protocol
types directly. That opener returns a `Box<dyn Transport>`, trying the cable the selected entry's
`DeviceFamily` ships with first (`preferred_transports()` in `lib.rs`, sourced from the cable table
in `supported-devices.md`) and falling back to the remaining bridges. The preference only matters
when more than one adapter is plugged in — without it a UT803 selection would open a UT61E+'s
CP2110 and time out on every read — and the fallback keeps unusual cable pairings working.

Handed `"auto"` instead of an entry, the same opener identifies the meter first: `detect.rs` runs
a probe cascade on the opened transport, sending each family's trigger in turn and classifying
whatever comes back, and a UT61+ name frame resolves to its registry entry. The families own that
knowledge: each exports a `Fingerprint` — its probe, the families that probe has to follow, and
its recognition rule, built from the constants it already puts on the wire — and `detect.rs` is
the engine that runs them, deriving which run from the registry entries pointing at them and
ranking what they answer by how strong the evidence is. The cascade and its failure modes are in
`docs/detection-design.md`. `open_auto()` is that path with the `Detected` entry handed back, so a
caller can name the meter it picked; `open_transport()` is its split half — a bridge and its name,
no protocol chosen — for a caller that must wrap the transport before the probe bytes flow, and
pairs with `detect::detect_device()`. `devices_on_bridge()` inverts
`preferred_transports()` to list the meters that could have been on a bridge nothing answered on,
and `find_by_model_name()` maps an open session's `model_name` back to its entry.
Adding a new device requires only a registry entry, a `Protocol` implementation and — to be found
by `"auto"` — a `Fingerprint` the entry points at; nothing in `detect.rs`, zero app code changes.

### dmm-settings

Tiny shared crate holding the `SharedSettings` struct — currently just one field, `device_family`, but the natural home for anything the CLI and GUI both need to agree on. Depends on `serde` + `serde_json` + `directories` only; no UI, no device, no hardware code. Owns `config_path()` (the canonical `~/.config/dmm-tools/settings.json` location), `SharedSettings::load_if_exists()` for reading the file, `resolve_device_family()` — the `--device` flag → `device_family` → caller's default precedence both binaries apply, returning a `DeviceSource` so the CLI can print its fallback notice — and `write_atomic()`, the `.tmp` + fsync + rename helper both binaries use to persist user data (settings, capture reports, CSV exports) without risking a torn file. The fallback is passed into `resolve_device_family()` rather than looked up — a registry id, or `AUTO_DEVICE_ID` to let detection settle it — keeping the crate free of a `dmm-lib` dependency.

The GUI's full `Settings` struct includes `SharedSettings` via `#[serde(flatten)]` so the on-disk JSON stays flat (`device_family` at the top level alongside `theme`, `show_graph`, etc.). The CLI deserializes the same file directly into `SharedSettings`, silently ignoring any GUI-only fields. Because both sides reference exactly one Rust type for the shared fields, renaming or retyping `device_family` breaks both compilations simultaneously — the contract is compile-enforced.

### dmm-cli

CLI binary using `clap`. Its modules:

| Module | Responsibility |
|--------|---------------|
| `main.rs` | CLI framework, command dispatch, `list`/`info`/`read`/`get`/`set`/`command`/`debug` subcommands |
| `capture/mod.rs` | The `capture` command: opens the report, runs the passes, prints the coverage epilogue |
| `capture/report.rs` | Report schema and serde (`CaptureReport`, `StepResult`, `SampleData`), trust tiers, report file read/write |
| `capture/step.rs` | One capture step: its definition, the wait for the state it asks for, the frames that wait sends |
| `capture/session.rs` | The passes of a run: meter handshake, the device's steps and their equipment, end-of-run review, freeform captures |
| `capture/input.rs` | Keyboard reader thread polled between readings; per-step log of parse rejections |
| `capture/listing.rs` | `--list-steps` output (text and issue checklist); validation of `--steps` names |
| `drive.rs` | Automatic sweeps of driveable settings after a mode step, so every range and flag reaches the report unprompted |
| `plan.rs` | Maintainer-written step list from YAML, run with `capture --plan` |
| `recording.rs` | Wire-byte recorder around a transport, including bytes the framing layer rejected |
| `watch.rs` | Capture step advance logic: when the meter has settled into the state a step asked for, semantic (`expect`) or raw payload diff against the previous step |
| `format.rs` | Measurement output formatting (text/csv/json) |

All protocol logic lives in the library crate. The `capture` subcommand provides a guided
interactive wizard for protocol verification, outputting YAML reports with raw bytes.
Uses `console` crate for colored output and single-key input, `serde_yaml_ng` for report format.
Capture reports are written atomically (temp file + rename) for crash safety.
The capture workflow's design — detectors, trust tiers, report schema — is in
`docs/capture-design.md`.

### dmm-gui

`eframe`/`egui` application. Runs a background `std::thread` for device I/O,
communicates with the UI via three `mpsc` channels: measurements and
connection events out of the thread, a `ThreadControl` channel in (stop and
pause — pause halts polling in the thread, it is not a UI-side freeze), and a
device-command channel in. Main graph via `egui_plot`,
minimap via custom painter. Uses `clap` for CLI argument parsing (`--device`,
`--theme`, `--mock-mode`) — overrides are session-only and don't persist to
`settings.json`. Features: responsive layout with resizable panels,
dark/light themes with WCAG-compliant colors, PPK2-style minimap navigation,
continuous timeline across reconnects, pause/resume capture, graph overlays
(mean line, reference lines, measurement cursors, min/max envelope, trigger markers),
a series selector plus same-unit sub-value traces for multi-display meters,
remote control buttons, UI zoom (Ctrl+/-), CSV recording/export with scrollable
sample log, persistent settings.

`App` is declared once in `app/mod.rs`; every module under `app/` adds `impl App`
methods to it, so no panel owns state of its own.

| Module | Responsibility |
|--------|---------------|
| `app/mod.rs` | The `App` struct, `ConnectionState`, construction, and the per-frame `eframe::App::ui` that lays the panels out |
| `app/appearance.rs` | Font chain and text styles, theme and colour overrides, zoom levels, always-on-top and decoration commands |
| `app/connection.rs` | The background acquisition thread: open, poll, reconnect, the per-setting choice lists (re-listed only when the reading they are keyed on moves), and the `DmmMessage`/`ThreadControl` channel types |
| `app/messages.rs` | The UI side of that channel: connect/disconnect, the message drain, and the connection-help text |
| `app/plot_input.rs` | Reducing one measurement to what the graph plots — series, unit, and same-unit overlays |
| `app/top_bar.rs` | Device label, connection buttons, status landmark, and the version/Help/shortcuts/settings group |
| `app/toast.rs` | The transient status message, floated over the window's top-right corner in every layout |
| `app/controls.rs` | The settings panel and the meter's remote-command buttons |
| `app/layout.rs` | The reading column shared by the wide and narrow layouts, the specs sections, and the big meter toggle |
| `app/meter_fit.rs` | Big-meter sizing arithmetic: minimum window size, panel margin, the wide/narrow threshold, and the re-measure cache |
| `app/stats_panel.rs` | Session and visible-window min/max/avg/count and the running integral |
| `app/recording_panel.rs` | Record/Export row, sample log, discard prompt, and the graph/recording split |
| `app/export.rs` | CSV export: rendering the buffer, the save dialog and write off the UI thread, and the result toast |
| `app/transform_ui.rs` | The **Scale** row and its editor for the software transform |
| `app/shortcuts.rs` | The keyboard binding table, its dispatcher, and the rows the help modal shows |
| `app/shortcut_help.rs` | The keyboard and mouse help modal |
| `app/whats_new.rs` | The "What's New" release-notes viewport |
| `graph/` | Scrolling graph: history buffer, view navigation, toolbar, main plot, minimap, visible-slice analysis |
| `display.rs` | The reading itself in its three sizes, with the mode and range dropdowns and the sub-value rows |
| `recording.rs` | The bounded sample buffer and its CSV rendering |
| `settings.rs` | Persisted settings and the colour presets |
| `specs.rs` | Per-range specification rendering |
| `theme.rs` | Theme colour tables (WCAG-checked in both modes) |
| `a11y.rs` | AccessKit label/role extension traits, focus rings, arrow-key resize |
| `changelog.rs` | The embedded `CHANGELOG.md` shown in the What's New viewport |

## Key Design Decisions

1. **Sync, not async** — 9600 baud, single device, request/response. No benefit to async complexity.
2. **Direct hidapi, no cp211x_uart** — the cp211x_uart crate is unmaintained (2017). Our CP2110 layer is ~120 lines.
3. **hidraw backend** — required for HID feature reports on Linux (libusb backend doesn't support them).
4. **Transport trait** — enables `MockTransport` for testing without hardware.
5. **Protocol trait** — each device family implements `Protocol` (object-safe, `Send`). `Dmm` dispatches through `Box<dyn Protocol>`, so callers don't need to know the family at compile time.
6. **Device tables via trait** — within the UT61E+ family, adding a new meter model = adding one file implementing `ModeTables` (`DeviceTable` is derived from it).
7. **No nom** — payload is a fixed 14-byte struct. Direct indexing is clearer.
8. **Measurement fields use `&'static str`** — `unit` and `range_label` reference static table data, avoiding heap allocation per measurement.
9. **Graph two-tier rendering** — the minimap needs the full history, so it keeps it as a min/max level (`graph/level.rs`): one bucket per fixed span of session time about a physical pixel wide (`bucket_secs`), holding that span's vertical extent. A sample folds into the last bucket, an evicted one out of the first, and the level is recut only when the strip's bucket width steps — never per frame, never per push. Each run of buckets between two interruptions is projected through `decimate_columns` and painted as a single polyline, and the auto Y range folds the bucket extremes instead of scanning the points, so the strip's cost follows its width rather than the history length. Buckets are cut in session time, not screen columns: the strip rescales on every sample, so column buckets changed members each frame and the trace flickered. The main graph reads none of it: it binary-searches the history for the visible time window (`visible_index_range`), then builds segments from that ~150-point slice each frame. All per-frame helpers (Y-bounds, statistics, envelope, crossings, nearest-point) also operate on the visible slice only, keeping frame cost independent of total history size. Sub-value overlay traces are stored as `VecDeque<Option<f64>>` running in lockstep with the history, so the same visible slice indexes them and the single-display case pays nothing; the minimap stays main-series-only so its level is not multiplied by the overlay count.
10. **Bounded buffers** — the graph history and the recording share one bound (default 500K samples, settable in Settings), and the background channel is drained every frame, so memory cannot grow without limit during sustained use.
11. **Settings schema evolution** — `#[serde(default)]` on `Settings` allows adding new fields without breaking existing config files.
12. **Device registry** — all device metadata (display names, aliases, activation instructions, protocol factories, manual URLs) lives in a single `DEVICES` slice in the library. CLI and GUI consume the registry without device-specific knowledge, so adding a new device family requires zero app code changes.
13. **Static spec data** — per-range specifications (resolution, accuracy bands) and per-mode metadata (input impedance, notes) are `&'static` arrays in `tables/specs_*.rs` files, transcribed from device manuals. The GUI caches spec lookups keyed on `(mode_raw, range_raw)` and re-looks up only on mode/range changes — zero per-frame allocations. Use `cargo run -p dmm-lib --example dump_specs` to verify spec data against manuals.
14. **Derived series model** — a *frame* is one measurement's named series: `Main` plus each sub-value by label. A *derived series* is a (label, unit, op) triple over those names. `Op::Linear` re-expresses the main reading, so it replaces `Main` and keeps the meter's own value as the `Raw` sub-value — the convention meters themselves use (Fluke REL, the UT181A's relative and dBm formats). Planned ops that produce a *new* quantity (`Binary` for V×I or A−B, `Formula`) will instead be appended as sub-values, which the graph selector, the overlay traces and the CSV aux columns already handle. Each consumer applies transforms at exactly one point — the CLI read loop, the GUI message drain — after acquisition and before any fan-out to display, graph, recording and export, so every output shows the same numbers. The scale is applied in base units (`si_prefix()`) so a factor typed once stays correct when the meter auto-ranges from mV to V.
15. **Name the target, confirm from the stream** — `select()` never trusts a press. The cycle driver presses once, waits for a fresh frame, re-reads rather than re-presses on a stale or unanswered one, and gives up after ring length + 1 presses; a range walk aborts if the mode byte moves under it. A mode or range press that changes nothing while HOLD is lit is sent again once HOLD is pressed off: a held UT61E+ drops Hz/% rather than deferring it, so the second press cannot overshoot. The flag settings never release HOLD. Choice id 0 is auto-range on every family and is sent as the meter's own command, never walked to. A setting the family cannot drive fails as `UnsupportedCommand` before any I/O; a switch the meter did not perform fails as `CommandRejected` after it. Both are `ErrorKind::Configuration`, so the GUI shows a toast instead of reconnecting.
16. **Session clock** — `dmm_lib::Clock` stamps every reading in `Dmm::request_measurement` and is cloned into the mock's waveform and the pacing loop, so all three read the same time. `Clock::real()` is wall time; `Clock::scaled(f)` runs session time at `f` times real time and `with_preseed(secs)` spends a burst of it instantly, so screenshots and performance runs start with history; `Clock::manual()` moves only when a test advances it. Stamping in one place means stats, graph, recording and export follow without knowing a clock exists, and `WallClock::from_clock` backdates its origin by the burst so exported wall times stay true. Hardware timeouts, settle delays and transport bring-up sleeps stay on real time: they pace physical USB.
