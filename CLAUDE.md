# dmm-tools

Rust workspace for communicating with digital multimeters via USB (CP2110, CH9329, and CH9325 HID bridges). Supports UNI-T and Voltcraft meters.

## Project structure

- `crates/dmm-lib/` — library: CP2110/CH9329/CH9325 transports, protocol framing (AB CD and 0xAC extractors), measurement parsing, device tables. Protocol families: `ut61eplus`, `ut8802`, `ut8803`, `ut171`, `ut181a`, `vc880`, `vc890`. `protocol` module is `pub(crate)` — consumers use the `Dmm` API, not raw frame extraction.
- `crates/dmm-settings/` — shared settings schema (`SharedSettings { device_family }`) used by both CLI and GUI so the config-file contract is compile-enforced. GUI-only fields (colors, panel visibility, theme) live in `dmm-gui` and merge via `#[serde(flatten)]`.
- `crates/dmm-cli/` — CLI binary `dmm-cli`.
- `crates/dmm-gui/` — GUI binary `dmm-gui` (eframe/egui).

## Build & test

- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings` (must pass clean)
- `cargo fmt --check`

## Working with the user

### Physical device interaction
- When testing requires physical device interaction (dial position, lead placement, connections), describe the setup and **wait for user confirmation** before each step. Never assume the device is in the right state.

### Specification data
- **Never fabricate specification data.** If a value cannot be directly read from the source document, mark it unknown rather than guessing.
- Verify PDF-sourced specs against the actual PDF rendering, not text-extracted versions — PDF-to-text conversion corrupts multi-column and merged-cell tables.

### Simplicity and scope
- Default to the simplest approach that meets the requirement. No speculative abstractions or "just in case" configurability.
- When told "keep it simple", revert anything that adds complexity without clear value.
- When a request is ambiguous about scope, ask before implementing.
- For multi-step work, persist progress in durable files (e.g. `docs/verification-backlog.md`) — sessions can run out of context.

### Tool usage
- Prefer Read/Grep/Glob/Write/Edit over Bash to reduce permission prompts.
- Run git commands directly — cwd is the repo. Don't use `git -C`.
- With Edit's `replace_all`: check the pattern doesn't match unintended locations (e.g. replacing `AcDcA` also catches `AcDcA2`). Prefer targeted single replacements.

## Engineering standards

### Code quality
- All code must pass `cargo clippy --workspace -- -D warnings` and `cargo fmt --check`.
- Write tests alongside non-trivial logic, especially protocol parsing and byte manipulation.
- `unwrap()` only in tests and examples; return `Result` elsewhere.
- Default new items to `pub(crate)`; widen to `pub` only for a concrete external consumer.
- Prefer `&'static str` over `String` for static lookup values — avoids allocation in measurement-parsing hot paths.
- Prefer enums over string-typed status/state values — lets the compiler catch typos and missing match arms.
- Add `#[serde(default)]` on all optional/new fields in serialized structs. Forgetting this breaks deserialization of existing user config files.
- Deduplicate user-facing error/help strings via helper functions — duplicated strings drift.
- Avoid `unsafe` unless absolutely necessary.

### Robustness
- Use `checked_duration_since()` instead of `duration_since()` — the latter panics on backward clock jumps (VM suspend, NTP correction).
- Background threads must propagate errors to the main thread via channel; wrap bodies in `catch_unwind` rather than silently failing.
- Write data files atomically: write to `.tmp`, then `fs::rename`. Protects user data (settings, captures, CSV exports) against kill signals and disk-full mid-write.
- Bound buffer growth. Current caps: graph history 10K points, recording 500K samples. New buffers need an explicit bound too.

### Protocol correctness
- Protocol code is byte-level. Always validate checksums. Document byte offsets and masks with comments referencing `docs/research/<family>/reverse-engineered-protocol.md`.
- Test parsing with known-good byte sequences captured from real device traces.
- **Any protocol change MUST be verified against a real device before being considered done.** Use `RUST_LOG=dmm_lib=trace cargo run --bin dmm-cli -- --device <id> debug` to capture raw bytes. Three major bugs (frame length, mode enum, flag bits) only surfaced against real hardware.
- For unsafe or HID parsing code: confirm a malformed response cannot panic (check buffer sizes, bounds).
- Our protocol understanding comes from reverse engineering, not official documentation. See `docs/verification-backlog.md` for what's been verified and what's pending.
- Per-family protocol specs live in `docs/research/<family>/reverse-engineered-protocol.md`. `docs/protocol.md` is only an index.
- Reference implementations to cross-check when in doubt: [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) (Python, UT61E+), [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) (Go, UT61E+), [pylablib](https://github.com/AlexShkarin/pyLabLib) (Python, VC-880).
- Mocks must match real-device behavior: no impossible flag combinations (e.g. MIN+MAX simultaneously), correct data types for stored vs live values. Mocks that diverge create false confidence.

### Commit discipline
- Commit logical units of work — one concept per commit, each compiling and passing tests.
- Include tests alongside new non-trivial code.
- Commit messages: imperative mood, explain the *why*.
- Never commit `references/` — gitignored; holds vendor software, decompilations, and manuals that must not live in the repo.

### Logging
- `log` crate, structured levels: `TRACE` for raw HID bytes, `DEBUG` for protocol events (request/response/checksum), `INFO` for connection state, `WARN` for recoverable issues (timeouts, retries), `ERROR` for failures.
- `RUST_LOG=dmm_lib=trace` should give complete wire-level debugging.
- Never log at `INFO` or above inside the measurement loop.

### GUI correctness
- **Test every visual change in both dark and light themes.** Colors tuned for dark mode routinely fail WCAG contrast on light backgrounds — the single largest source of rework in this project.
- All colors must be theme-aware (`ui.visuals().dark_mode`).
- WCAG 2.1 AA contrast: ≥4.5:1 for text, ≥3:1 for graphical elements. Verify numerically when adding/changing colors.
- Never rely on color alone — add line style, text, or bold as a secondary indicator.
- Minimum font size 11pt throughout.
- Display value strings use `display_raw` for stable width (no jitter).
- Icon-only or custom-painted interactive widgets need an AccessKit label via `accesskit_node_builder`. Buttons with text get this automatically; icon buttons and custom widgets do not.
- User-initiated actions (export, clear, connect) need visible feedback — toast, status message, or log line. Silent success is a UX bug.
- Think through boundary conditions before writing code: extreme window sizes (very wide, very narrow, quarter-screen, maximized), high zoom, empty/no-data state, mode transitions.
- Graph rendering has two tiers. The minimap uses a full-history segment cache invalidated by the monotonic `history_version` counter; the main graph builds segments from the visible slice via `visible_index_range()` binary search. Per-frame helpers (stats, y-bounds, envelope, crossings) must also iterate only the visible slice — do not regress them to full-history scans.
- egui pitfalls learned the hard way:
  - `set_plot_bounds()` overrides both axes — use `set_plot_bounds_x()` / `_y()` (egui_plot 0.33+) to constrain one axis.
  - `allow_drag(false)` also suppresses pointer position events; use `plot.reset()` per frame to pin the view while keeping events.
  - After mode changes or data clears, call `plot.reset()` to avoid stale bounds from the previous state.
  - `set_pixels_per_point()` and `set_visuals()` called every frame reset egui's internal panel state (resize positions, scroll offsets). Only call when the value changes.
  - egui API naming is inconsistent — verify method names against docs (`fill_color()` not `color()`, `Vec2b` not `Axis` for `allow_drag`/`allow_zoom`).

### Dependencies
- `dmm-lib` stays self-contained: only `hidapi`, `thiserror`, `log`. No external utility crates — this is the core that talks to hardware.
- CLI and GUI crates: prefer well-maintained community crates over hand-rolled equivalents (markdown rendering, UI widgets, date handling).
- Evaluate new dependencies on maintenance health, transitive footprint, and whether they solve a real problem vs. something achievable in a few lines.

### Clean-room reverse engineering
- Only consume sources the user has explicitly approved (manuals, vendor software, public datasheets). Do not read external implementations or community code until the clean-room analysis is complete and the user approves cross-referencing.
- Document sources used and avoided so the clean-room boundary is auditable.

## Documentation

Documentation is part of the deliverable — update affected docs in the same commit as the change, not after.

- `docs/architecture.md` — crate layout, module responsibilities, data flow, key design decisions.
- `docs/protocol.md` — index of per-family specs. Authoritative content lives in `docs/research/<family>/reverse-engineered-protocol.md`.
- `docs/setup.md`, `docs/development.md`, `docs/ux-design.md`, `docs/cli-reference.md`, `docs/gui-reference.md` — user and contributor references; update when their subject changes.
- `docs/adding-devices.md` — end-to-end guide for new device support. **Read this before starting work on any new device.**
- `docs/research/<family>/` — per-family RE methodology and wire-protocol spec.
- `docs/verification-backlog.md` — update whenever items are verified or new unknowns surface. Critical for preserving state across sessions.
- For new device support, touch all of: `README.md`, `docs/supported-devices.md`, `docs/verification-backlog.md`, `docs/architecture.md`, `docs/cli-reference.md`, `docs/gui-reference.md`. Also create a GitHub verification issue (match the pattern of #3/#4/#5/#12/#13/#14) and link it from `supported-devices.md`.
- Escape angle brackets in markdown (`\<foo\>` or `` `<foo>` ``) — bare `<tags>` render as invisible HTML on GitHub.

### Changelog
- Add entries to `## Unreleased` in the same commit as user-visible changes. The release workflow extracts the entry for the tagged version — don't defer.
- **Scope is strictly user-visible impact.** Someone running a prebuilt binary must be able to notice the difference. Phrase entries from that perspective.
- **Do NOT add entries for:** internal refactors, code reorganization, new public trait methods, `String`→`Cow` swaps, extracted helpers, added tests, dependency bumps, or CI tweaks — even if the diff is large or architecturally meaningful.
- Organized by component: GUI, CLI, Library, Bug fixes, Documentation. Reserve `Internal` for changes whose *cause* is internal but *symptom* is user-visible (e.g., a scheduling fix that removes visible jitter); lead those entries with the user-facing symptom.
- **Keep entries crisp.** Lead with a bolded user-visible summary, then at most one follow-up sentence for context. If you're writing a paragraph, that's commit-message content.
- Rule of thumb when tempted: write the "before/after a user would observe" sentence out loud. If you can't, don't add the entry.
