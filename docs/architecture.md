# Architecture

## Crate Layout

```
dmm-tools/
├── crates/
│   ├── dmm-lib/       # Core library
│   ├── dmm-shared/    # App-only shared code (CLI ↔ GUI)
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
| `transport/ble/mod.rs` | Bluetooth LE transport for a UART-over-BLE peer, an adapter or a meter with the radio built in: connects it, picks the GATT profile from its services (ISSC first, else FFF0), subscribes to that profile's notify characteristic, and turns notifications and writes into the byte stream the cables carry. Behind the default-on `bluetooth` feature; `ble_disabled.rs` stands in without it |
| `transport/ble/search.rs` | Finds a Bluetooth peer by its advertised name, or the one an address names |
| `transport/ble/issc.rs` | The ISSC transparent-UART profile's UUIDs, and the adapter's name prefix and heartbeat frame |
| `transport/ble/fff0.rs` | The FFF0 profile's UUIDs: one characteristic, FFF4, carries both directions |
| `protocol/mod.rs` | `Protocol` trait (object-safe), `DeviceFamily` enum, `DeviceProfile`, `Stability`, `Setting`/`Choice` for absolute setting selection |
| `protocol/registry.rs` | Device registry: `SelectableDevice` entries, factory functions, `resolve_device()` lookup. CLI and GUI use the registry for device selection — no device-specific code in app crates. |
| `protocol/cycle.rs` | Cycle-to-target driver shared by the UT61+ and Voltcraft families: presses a ring button (SELECT, Hz/%, SHIFT/SETUP, RANGE, MIN/MAX, PEAK) and reads back until the named mode, rung or flag state shows; mode walks are planned over a per-model dial table because the meter never reports the dial |
| `protocol/unrecognised.rs` | `report_unknown()`, what every parser calls on data its spec doesn't cover: the first call of the process warns and says how to report it, every call logs at DEBUG; `capture_reports()` lets tests (the golden fixtures among them) check that known data reports nothing |
| `protocol/framing.rs` | Message framing: the read loop every family shares, and per frame shape an extractor that finds the header, cuts the payload and validates its checksum (byte positions, for the shape without one); builds the command frame the UT61+ and Voltcraft families send |
| `protocol/expect.rs` | `Expect`: what a correctly parsed reading looks like once a capture step's instruction is carried out (mode, range, value, flags), so a capture run tests the parser instead of collecting samples to read by eye |
| `protocol/steps.rs` | `gate_steps()`: the six DC-volts and resistance steps every family's capture run starts with, worded once |
| `protocol/ut61eplus/` | UT61E+ family: `Ut61PlusProtocol`, `Mode` enum, `Command` enum, `tables/` (per-model `ModeTables` impls — one match per mode returning its ranges — behind the `DeviceTable` trait), `specs/` (the manual's spec tables per model, which `SpecModel` looks a reading up in) |
| `protocol/ut8802/` | UT8802 family: `Ut8802Protocol` — streaming, read-only |
| `protocol/ut8803/` | UT8803 family: `Ut8803Protocol` — streaming, read-only |
| `protocol/ut80x/` | UT803/UT804, and the UT71A–E and Voltcraft VC920/VC940/VC960 that send the UT804's packets: `Ut80xProtocol` — streaming over the CH9325 cable, one payload parser per bench model, the handhelds reusing the UT804's over their own range labels; `specs_ut803.rs`/`specs_ut804.rs` hold the spec tables |
| `protocol/ut171/` | UT171 family: `Ut171Protocol` — streaming once the user turns communication on at the meter |
| `protocol/ut181a/` | UT181A: `Ut181aProtocol` in `mod.rs` (streaming driver, device-sent unit strings); `parse.rs` decodes the normal, REL, MIN/MAX, Peak and COMP payloads, `command.rs` builds the AB CD command frames and reads the OK/ER reply, `mode.rs` holds the dial families SET_MODE and SET_RANGE move within |
| `protocol/vc8x0/` | Voltcraft VC-880/VC650BT and VC-890: `Vc8x0Protocol<M>` in `mod.rs` implements `Protocol` and `CycleMeter` once over a `Vc8x0Model`; `vc880.rs` (streaming) and `vc890.rs` (polled) hold each family's tables, dial, frame layout and the drain or ack around its I/O, and name the driver over their model `Vc880Protocol` / `Vc890Protocol` |
| `protocol/zotek/` | ZOTEK Bluetooth meters (ZOYI, BSIDE, ANENG): `ZotekProtocol` — streaming, one registry entry per packet layout, every packet decoded by its own layout; `frame.rs` finds and descrambles packets in the notification stream, `glyph.rs` reads the seven-segment digits and the words spelled in them, `layout.rs` holds each layout's annunciator table and builds the reading from what is lit, `keys.rs` the remote keys each layout offers and their frames, sent without waiting for a reply (a key whose code follows the display uses the last packet decoded, reading one first if none has arrived), `capture.rs` the capture steps per layout |
| `measurement.rs` | `Measurement` struct: mode, value, unit, flags (protocol-agnostic); `AuxValue` sub-values, with `AuxValue::export_cells` + `Measurement::export_aux_slots` supplying the cells and slot order `export.rs` lays out (the slot helper keeps a software-appended sub-value in a fixed column as the meter's own count changes) |
| `export.rs` | `CsvLayout`: the CSV header and row cells shared by the CLI and GUI exporters, so the two writers cannot disagree on columns (cells only — the `csv` crate stays in the binaries) |
| `transform.rs` | `Transform`: opt-in software scale/offset/unit-relabel over the main reading (shunt and clamp factors, °C→°F). `si_prefix()` converts to the base SI unit first so a factor survives auto-ranging; the meter's own reading is kept as the `Raw` sub-value |
| `stats.rs` | `RunningStats` (min/max/avg), `Integrator` (trapezoidal time-integral with gap handling), and `SeriesStats` — the mode/unit-keyed session both the CLI read loop and the GUI drain accumulate into, so the two agree on what starts a new series |
| `mock/` | `MockProtocol`, the hardware-free meter the GUI, CLI demos and screenshots run against: `scenarios.rs` (one waveform-driven scenario per `MockMode`), `state.rs` (HOLD/REL/range/MIN-MAX/Peak and how they filter a reading), `mod.rs` (the `Protocol` impl). It stands in for a UT61E+ — same mode and range bytes, same spec table — and reaches its settings through `protocol/cycle.rs`, so a choice list that works here works on hardware |
| `replay.rs` | `Replay`: a recorded session opened as a device. Parses the `# dmm-replay` file (device id, recording time, the link it was recorded over, one payload per sample) and plays the payloads back through the family's own `Protocol::parse_payload`, paced by the session clock; the writer half lives here too, so a recorder cannot drift from the parser. Read-only calls delegate to the family's protocol, commands are refused — nothing is on the far end of the cable |
| `specs.rs` | Spec metadata types: `SpecInfo` (a range's resolution and `AccuracyBand`s), `ModeSpecInfo` (per-mode impedance, protection, notes), the keyed `ModeSpecs` rows the families' tables are written in, and `SpecSheetTable` for dumping a whole sheet |
| `clock.rs` | `Clock`: the session time base every reading is stamped with — real, scaled with an instant pre-seed burst, or manual for tests (decision 16) |
| `wall_clock.rs` | `WallClock`: an `(Instant, SystemTime)` origin pair that turns a reading's monotonic timestamp into the wall time shown and exported |
| `stream.rs` | `MeasurementStream`: absolute-tick pacing and consecutive-timeout counting around a `Dmm`, the acquisition loop the CLI `read`/`debug` commands and the GUI thread share; cancellation stays with the caller |
| `detect.rs` | `detect_device()`: the probe cascade behind `"auto"` — runs the families' `Fingerprint`s on an opened transport and ranks what answers (see below and `docs/detection-design.md`) |
| `flags.rs` | `StatusFlags`: Hold, Rel, Auto, Min/Max/AVG, Peak, Low Battery |
| `error.rs` | `Error` enum via `thiserror` |
| `binary_help.rs` | `--version` / `--device` / `--mock-mode` help text, the per-link sections of the "nothing found" help and the experimental-protocol warning, shared by both binaries. Lives here because the lists come from the registry and `MockMode::ALL`, so a new device or mock scenario reaches both `--help` outputs automatically. Build values (`CARGO_PKG_VERSION`, `GIT_HASH`) are passed in by the caller. |
| `docs_tables.rs` | Renders the `--device` table in `docs/cli-reference.md` from the registry; a `dmm-cli` test keeps the file's `devices:start`/`devices:end` block in sync and rewrites it under `UPDATE_DOCS=1` (see `docs/development.md`) |
| `lib.rs` | `Dmm` struct: top-level API tying everything together |

**Data flow:**

```
CLI/GUI ──► registry::resolve_device()
                       │
                       └──► SelectableDevice.new_protocol()
                                           │
USB HID ──► Cp2110, Ch9329 or Ch9325 (Box<dyn Transport>) ──► Box<dyn Protocol> ──► Measurement { mode, value, unit, flags }
Bluetooth ──► Ble (Box<dyn Transport>) ──────────────────────┘
                                           │
                                           ├── Ut61PlusProtocol            (polled, per-model DeviceTable)
                                           ├── Ut8802Protocol              (streaming)
                                           ├── Ut8803Protocol              (streaming)
                                           ├── Ut80xProtocol               (streaming, per-model parser)
                                           ├── Ut171Protocol               (streaming)
                                           ├── Ut181aProtocol              (streaming, device-sent units)
                                           ├── Vc8x0Protocol<Vc880Model>   (streaming)
                                           ├── Vc8x0Protocol<Vc890Model>   (polled)
                                           └── ZotekProtocol               (streaming, per-layout LCD image)
```

`Dmm<T: Transport>` holds a `Box<dyn Protocol>`. The `Protocol` trait provides `init()`,
`request_measurement()`, `parse_payload()`, `send_command()`, `choices()`/`select()`,
`get_name()`, `profile()`, and `capture_steps()`. Each family implements its own framing,
parsing, and command encoding internally, but all produce the same `Measurement` struct.

`Dmm` keeps the meter's name, the one copy for every family: `Dmm::get_name()` returns the
name the meter already gave on this link, else asks and keeps the answer, and
`Dmm::known_name()` never asks. Detection's answer seeds it (`Dmm::from_detected()`), and a
protocol whose `name_before_init()` says the meter wants its name first gets it through that
same cache before `init()`, so a meter detection already asked is not asked again. The binaries
call these and carry no name of their own.

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
CP2110 and time out on every read — and the fallback keeps unusual cable pairings working. An
entry marked `bluetooth_only`, a meter with the radio built in, is looked for over Bluetooth alone,
and fails with its own error (`Error::BluetoothOnly`): not in range, or the radio not searched.

The same opener reaches Bluetooth. `OpenOptions` carries the `--adapter` selector and whether
Bluetooth may be scanned: the `bluetooth` setting, which `--no-bluetooth` overrides. A selector
shaped like a Bluetooth address or peripheral identifier goes straight to `transport/ble/`.
Otherwise the USB bus is tried first. If nothing answers there, scanning is allowed and the family
lists Bluetooth among its links, the transport looks for an adapter or a Bluetooth meter: one
already connected, else one heard in a short scan, else a paired one by address. The caller names
the peers it takes, by advertised name (`bluetooth_peers()` in `lib.rs`): an entry behind an
adapter takes adapters only, a `bluetooth_only` entry its own `bluetooth_names`, and `"auto"` and
`list` all of them, so an open for one meter never lands on another unless `--adapter` names an
address, which opens whatever answers there. Every step has a time limit. When that
finds nothing, the caller gets the USB error, marked with whether Bluetooth was searched; both
binaries title their help from that mark. The GUI reconnects a lost Bluetooth link to the same
adapter by address, without scanning; switching adapters is an explicit Disconnect and Connect.
A cable with a silent meter wins over a live meter on an adapter: unplug it or name the adapter.
Listing is split the same way: `list_devices()` stays instant because it backs a GUI control,
while `list_bluetooth_devices()` scans.

Handed `"auto"` instead of an entry, the same opener identifies the meter first: `detect.rs` runs
a probe cascade on the opened transport, sending each family's trigger in turn and classifying
whatever comes back, and a UT61+ name frame resolves to its registry entry. The families own that
knowledge: each exports a `Fingerprint` — its probe, the families that probe has to follow, and
its recognition rule, built from the constants it already puts on the wire — and `detect.rs` is
the engine that runs them, deriving which run from the registry entries pointing at them and
ranking what they answer by how strong the evidence is. On a Bluetooth peer whose advertised
name (`Transport::advertised_name()`) belongs to registry entries, only those entries'
fingerprints run: the transport reports the name, the registry says which meters carry it, and
a name several entries share narrows the cascade without picking a model. The cascade and its
failure modes are in `docs/detection-design.md`. `open_auto()` is that path with the `Detected`
entry handed back, so a caller can name the meter it picked; `open_transport()` is its split
half — a bridge and its name, no protocol chosen — for a caller that must wrap the transport
before the probe bytes flow, and pairs with `detect::detect_device()` and `Dmm::from_detected()`; `open_device_transport()` is
the same for a named entry. `devices_on_bridge()` inverts
those links to list the meters that could have been on a bridge nothing answered on,
and `find_by_model_name()` maps an open session's `model_name` back to its entry.
Adding a new device requires only a registry entry, a `Protocol` implementation and — to be found
by `"auto"` — a `Fingerprint` the entry points at; nothing in `detect.rs`, zero app code changes.

### dmm-shared

What `dmm-cli` and `dmm-gui` must agree on and `dmm-lib` must not carry — the layering is `dmm-lib` ← `dmm-shared` ← `dmm-cli`, `dmm-gui`. Anything both binaries need that isn't the meter library's business belongs here; device, protocol and transport code stays in `dmm-lib`, and anything one binary alone needs stays in that binary. Holds the `SharedSettings` struct — `device_family`, and `bluetooth`, whether an open may scan for Bluetooth adapters and meters. Depends on `serde` + `serde_json` + `directories` + `chrono` + `env_logger` + `log` + `dmm-lib` (for the `Measurement` its JSON builder reads); no UI, no transport, no hardware code. Owns `config_path()` (the canonical `~/.config/dmm-tools/settings.json` location), `SharedSettings::load_if_exists()` for reading the file, `resolve_device_family()` — the `--device` flag → `device_family` → caller's default precedence both binaries apply, returning a `DeviceSource` so the CLI can print its fallback notice — and `write_atomic()`, the `.tmp` + fsync + rename helper both binaries use to persist user data (settings, capture reports, CSV exports) without risking a torn file. The `export` module is the same idea for what the two write out: `default_name()` is the `measurements-<meter>-<mode>-<start>.<ext>` name the GUI's Export… dialog opens on and `dmm-cli read -o` (given no file name) writes to, and `metadata_line()` + `measurement_json()` are the JSON both the GUI's export and `dmm-cli read --format json` emit — one builder, so a field added to a reading reaches both binaries at once. `logging::init()` is the logger both install: warnings from `dmm_lib` and errors from the rest unless `RUST_LOG` says otherwise. The fallback is passed into `resolve_device_family()` rather than looked up — a registry id, or `AUTO_DEVICE_ID` to let detection settle it — so the settings half of the crate does not reach into the registry.

The GUI's full `Settings` struct includes `SharedSettings` via `#[serde(flatten)]` so the on-disk JSON stays flat (`device_family` at the top level alongside `theme`, `show_graph`, etc.). The CLI deserializes the same file directly into `SharedSettings`, silently ignoring any GUI-only fields. Because both sides reference exactly one Rust type for the shared fields, renaming or retyping `device_family` breaks both compilations simultaneously — the contract is compile-enforced.

### dmm-cli

CLI binary using `clap`. Its modules:

| Module | Responsibility |
|--------|---------------|
| `main.rs` | CLI framework, command dispatch, `list`/`info`/`read`/`get`/`set`/`command`/`debug`/`completions` subcommands |
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
| `format.rs` | What a run writes per reading: text, CSV, JSON, or the meter's own frames as a replay file |
| `output.rs` | Where it goes: stdout, `-o FILE`, or a file the run names itself once the first reading has arrived |

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
sample log (exporting the graph's samples when nothing was recorded),
persistent settings.

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
| `app/recording_panel.rs` | Record/Export/Discard row, sample log or the line saying Export… saves the graph's samples, discard prompt, and the graph/recording split |
| `app/export.rs` | Export: which format the menu picked, rendering the sample buffer, the save dialog and write off the UI thread, and the result toast |
| `app/transform_ui.rs` | The **Scale** row and its editor for the software transform |
| `app/shortcuts.rs` | The keyboard binding table, its dispatcher, and the rows the help modal shows |
| `app/shortcut_help.rs` | The keyboard and mouse help modal |
| `app/whats_new.rs` | The "What's New" release-notes viewport |
| `graph/` | Scrolling graph: history buffer, view navigation, toolbar, main plot, minimap, visible-slice analysis |
| `display.rs` | The reading itself in its three sizes, with the mode and range dropdowns and the sub-value rows |
| `recording.rs` | The bounded sample buffer — the graph's history until Record, then the recording — and its CSV, JSON and replay rendering |
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
6. **Device tables via trait** — within the UT61E+ family, adding a new meter model = adding one file implementing `ModeTables` (`DeviceTable` is derived from it), plus its spec tables in `specs/`.
7. **No nom** — each family's payload is a fixed-size struct. Direct indexing is clearer.
8. **Measurement fields use `&'static str`** — `unit` and `range_label` reference static table data, avoiding heap allocation per measurement.
9. **Graph two-tier rendering** — the minimap needs the full history, so it keeps it as a min/max level (`graph/level.rs`): one bucket per fixed span of session time about a physical pixel wide (`bucket_secs`), holding that span's vertical extent. A sample folds into the last bucket, an evicted one out of the first, and the level is recut only when the strip's bucket width steps — never per frame, never per push. Each run of buckets between two interruptions is projected through `decimate_columns` and painted as a single polyline, and the auto Y range folds the bucket extremes instead of scanning the points, so the strip's cost follows its width rather than the history length. Buckets are cut in session time, not screen columns: the strip rescales on every sample, so column buckets changed members each frame and the trace flickered. The main graph reads none of it: it binary-searches the history for the visible time window (`visible_index_range`), then builds segments from that ~150-point slice each frame. All per-frame helpers (Y-bounds, statistics, envelope, crossings, nearest-point) also operate on the visible slice only, keeping frame cost independent of total history size. Sub-value overlay traces are stored as `VecDeque<Option<f64>>` running in lockstep with the history, so the same visible slice indexes them and the single-display case pays nothing; the minimap stays main-series-only so its level is not multiplied by the overlay count.
10. **Bounded buffers** — the graph history and the sample buffer share one bound (default 500K samples, settable in Settings), and the background channel is drained every frame, so memory cannot grow without limit during sustained use. The sample buffer holds full readings in one of two roles: with nothing recorded it follows the graph — cut to the graph's oldest point after each push, dropping its own oldest at the bound — for Export… to save; Record empties it for the recording, which keeps every sample until full. One buffer, so full readings are paid for once.
11. **Settings schema evolution** — `#[serde(default)]` on `Settings` allows adding new fields without breaking existing config files.
12. **Device registry** — all device metadata (display names, aliases, activation instructions, protocol factories, manual URLs) lives in a single `DEVICES` slice in the library. CLI and GUI consume the registry without device-specific knowledge, so adding a new device family requires zero app code changes.
13. **Static spec data** — per-range specifications (resolution, accuracy bands) and per-mode metadata (input impedance, notes) are `&'static` data in each family's `specs*.rs` files, transcribed from device manuals as rows keyed by range byte (`ModeSpecs`). `Dmm::request_measurement` attaches them to each reading through `Protocol::spec_info(&Measurement)` / `mode_spec_info(&Measurement)`, which take the whole reading so a family can pick the table by more than its mode and range. Use `cargo run -p dmm-lib --example dump_specs` to verify spec data against manuals.
14. **Derived series model** — a *frame* is one measurement's named series: `Main` plus each sub-value by label. A *derived series* is a (label, unit, op) triple over those names. `Op::Linear` re-expresses the main reading, so it replaces `Main` and keeps the meter's own value as the `Raw` sub-value — the convention meters themselves use (Fluke REL, the UT181A's relative and dBm formats). Planned ops that produce a *new* quantity (`Binary` for V×I or A−B, `Formula`) will instead be appended as sub-values, which the graph selector, the overlay traces and the CSV aux columns already handle. Each consumer applies transforms at exactly one point — the CLI read loop, the GUI message drain — after acquisition and before any fan-out to display, graph, recording and export, so every output shows the same numbers. The scale is applied in base units (`si_prefix()`) so a factor typed once stays correct when the meter auto-ranges from mV to V.
15. **Name the target, confirm from the stream** — `select()` never trusts a press. The cycle driver presses once, waits for a fresh frame, re-reads rather than re-presses on a stale or unanswered one, and gives up after ring length + 1 presses; a range walk aborts if the mode byte moves under it. A mode or range press that changes nothing while HOLD is lit is sent again once HOLD is pressed off: a press a held meter ignores is dropped rather than deferred, so the second press cannot overshoot. The flag settings never release HOLD. Choice id 0 is auto-range on every family and is sent as the meter's own command, never walked to. A setting the family cannot drive fails as `UnsupportedCommand` before any I/O; a switch the meter did not perform fails as `CommandRejected` after it. Both are `ErrorKind::Configuration`, so the GUI shows a toast instead of reconnecting.
16. **Session clock** — `dmm_lib::Clock` stamps every reading in `Dmm::request_measurement` and is cloned into the mock's waveform and the pacing loop, so all three read the same time. `Clock::real()` is wall time; `Clock::scaled(f)` runs session time at `f` times real time and `with_preseed(secs)` spends a burst of it instantly, so screenshots and performance runs start with history; `Clock::manual()` moves only when a test advances it. Stamping in one place means stats, graph, recording and export follow without knowing a clock exists, and `WallClock::from_clock` backdates its origin by the burst so exported wall times stay true. A replay pins the clock's wall origin to the time its recording was made and stamps each reading with the session time it was recorded at, so exported timestamps are the recording's own. Hardware timeouts, settle delays and transport bring-up sleeps stay on real time: they pace physical USB.
17. **Unrecognised data asks for a report, once** — data a parser's spec doesn't cover goes through `report_unknown()`, which warns on its first call of the process, with where to report it and how to trace more, and logs every later call at DEBUG. Once per process, because a meter parked on an unknown value would otherwise repeat it every frame and a reconnect every attempt; the parsers are free functions that replay and tests call directly, so the state is process-wide rather than per driver. Warnings from `dmm_lib` show by default, so anything that fires per frame logs at DEBUG, and the golden fixtures and bundled recordings are asserted to report nothing.
18. **Bluetooth behind a default-on feature, driven only inside its own calls** — `btleplug` (with `tokio` and `futures`) sits beside `hidapi`, behind the `bluetooth` feature; `ble_disabled.rs` has the same three functions, so callers carry no `cfg`. The transport owns a current-thread runtime and runs every stack call in `block_on` on the caller's thread: no background thread, no channel. As with HID, nothing runs between two transport calls, so a streaming meter's notifications wait in the platform's queue. The `Transport` trait gains two methods: `bluetooth_selector()`, the address a reconnect reuses, and `advertised_name()`, the name the peer goes by, which the caller looks up in the registry — the transport knows no model. Two error variants cover what USB lacks: `LinkLost` (`ErrorKind::Transport`, reconnected like a pulled cable) and `Bluetooth(String)` (`ErrorKind::Configuration`, a stack that is off or refuses; the GUI's reconnect loop still retries it).
