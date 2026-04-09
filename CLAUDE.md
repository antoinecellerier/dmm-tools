# dmm-tools

Rust workspace for communicating with digital multimeters via USB (CP2110 and CH9329 HID bridges). Supports UNI-T and Voltcraft meters.

## Project structure

- `crates/ut61eplus-lib/` — library: CP2110 and CH9329 transports, protocol framing (AB CD and 0xAC extractors), measurement parsing, device tables. Protocol families: `ut61eplus`, `ut8802`, `ut8803`, `ut171`, `ut181a`, `vc880`, `vc890`. `protocol` module is `pub(crate)` — consumers use `Dmm` API, not raw frame extraction.
- `crates/ut61eplus-cli/` — CLI binary (`main.rs` for commands, `capture.rs` for guided capture tool, `format.rs` for output formatting)
- `crates/ut61eplus-gui/` — GUI binary: real-time display and plotting (eframe/egui)

## Build & test

- `cargo build --workspace` — build everything
- `cargo test --workspace` — run all tests
- `cargo clippy --workspace -- -D warnings` — lint (must pass clean)
- `cargo fmt --check` — formatting check

## Working with the user

### Physical device interaction
- When testing requires physical device interaction (dial position, lead placement, connections), describe the required setup and **wait for user confirmation** before running each step. Never assume the device is already in the right state or drive through verification steps autonomously.

### Specification data
- **Never fabricate specification data** (accuracy, resolution, frequency response). If a value cannot be directly read from the source document, mark it as unknown/missing rather than guessing. Wrong data is worse than missing data.

### Scope and clarification
- When a request is ambiguous about scope (e.g., "add zoom"), clarify before implementing. A quick "Do you mean X or Y?" is cheaper than implementing the wrong thing.
- When working on multi-step tasks, persist progress in durable files (verification-backlog.md, task lists in docs/) rather than relying on conversation context — sessions can run out of context.

### Tool usage preferences
- Prefer Read/Grep/Glob/Write/Edit tools over Bash commands wherever possible to reduce permission prompts.
- When Bash is necessary, prefer commands already in the allow-list. Avoid command substitution when a simpler form exists.
- Run git commands directly — cwd is already the repo. Don't use `git -C`.

## Engineering standards

### Code quality
- All code must pass `cargo clippy -- -D warnings` and `cargo fmt --check` before committing
- Write unit tests for any non-trivial logic, especially protocol parsing and byte manipulation
- Use `#[cfg(test)]` modules in the same file as the code being tested
- Prefer returning `Result` over panicking — `unwrap()` is only acceptable in tests and examples
- Use `thiserror` for error types in the library crate
- Default new items to `pub(crate)` visibility. Only widen to `pub` when there's a concrete external consumer.
- Prefer `&'static str` over `String` when values come from static lookup tables — avoids per-call heap allocation in hot paths like measurement parsing.
- Prefer enums over string-typed status/state values — lets the compiler catch typos and missing match arms.
- Add `#[serde(default)]` on all optional/new fields in serialized structs (settings, config). Forgetting this breaks deserialization of existing user config files.

### Robustness
- Use `checked_duration_since()` instead of `duration_since()` — the latter panics if the system clock goes backward (VM suspend, NTP correction).
- Background threads must propagate errors back to the main thread via channel, not silently fail. Wrap thread bodies in `catch_unwind` and send the error.
- Write data files atomically: write to a `.tmp` path, then `fs::rename` to the final path. Protects against kill signals and disk-full mid-write.
- After refactors: check for dead code warnings (`unused` fields, methods, imports). Remove them in the same commit rather than letting them accumulate.

### Protocol correctness
- Protocol code is byte-level — off-by-one errors are easy to introduce. Always validate checksums on received data.
- Document byte offsets and masking operations with comments referencing the relevant protocol spec in `docs/research/<family>/reverse-engineered-protocol.md`.
- Test parsing with known-good byte sequences captured from real device traces.
- **Any protocol change MUST be verified against a real device before being considered done.** Use `RUST_LOG=ut61eplus_lib=trace cargo run --bin ut61eplus -- --device <id> debug` to capture raw bytes. Unit tests alone are not sufficient — we've found three major bugs (frame length, mode enum, flag bits) that only showed up against real hardware.
- Our protocol understanding comes from reverse engineering — not official documentation. See `docs/verification-backlog.md` for what's been verified and what still needs testing.
- Per-family protocol specs live in `docs/research/`: `ut61-family/`, `ut8803/`, `uci-bench-family/`, `ut171/`, `ut181/`, `vc880/`, `vc890/`. The UT61E+ also has a legacy `docs/protocol.md`.
- Reference implementations: [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) (Python, most complete for UT61E+), [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) (Go, UT61E+), [pylablib](https://github.com/AlexShkarin/pyLabLib) (Python, VC-880). Cross-check against these when in doubt.

### Commit discipline
- **Every commit must include tests for new code** — write tests before or alongside the code, never defer them
- Commit logical units of work — one concept per commit
- Each commit should compile and pass tests (`cargo build && cargo test`)
- Write concise commit messages: imperative mood, explain the "why" not just the "what"
- Don't bundle unrelated changes in the same commit

### Review checklist (apply *while writing code*, not just after)
This checklist exists to prevent issues, not to find them after the fact. Mentally walk through each item for the code you just wrote before committing.
- Does `cargo clippy --workspace -- -D warnings` pass?
- Does `cargo test --workspace` pass?
- Are new public types/functions documented?
- For CLI changes: is `docs/cli-reference.md` updated to reflect new/changed/removed commands and options?
- For GUI changes: is `docs/gui-reference.md` updated to reflect new/changed/removed features, panels, or controls?
- For protocol code: are byte offsets and masks correct? Cross-check against the relevant `docs/research/<family>/reverse-engineered-protocol.md`.
- For unsafe or HID code: are buffer sizes correct? Can a malformed response cause a panic?
- For GUI render paths: avoid per-frame allocations in hot loops. Use cached data with dirty flags where possible. Graph segments and gap ranges are cached — invalidate via `invalidate_cache()` when data changes.
- For new buffers: ensure bounded growth. Graph history caps at 10K points, recording at 500K samples.
- For serialized structs: do new fields have `#[serde(default)]`? Missing this breaks existing users' config files on upgrade.
- For file writes: is the write atomic? User data (captures, settings, CSV exports) should use write-to-tmp-then-rename.
- For user-initiated actions (export, clear, connect): is there visible feedback (toast, status message, log line)? Silent success is a UX bug.
- For icon-only or custom-painted interactive widgets: does it have an AccessKit label? Use `accesskit_node_builder` to set one. Buttons with descriptive text get this automatically; icon buttons and custom widgets do not.
- For new device support: update `README.md`, `docs/supported-devices.md`, `docs/verification-backlog.md`, `docs/architecture.md`, `docs/cli-reference.md`, `docs/gui-reference.md`. Create a GitHub verification issue (match the pattern of existing issues #3/#4/#5/#12/#13/#14) and link it from `supported-devices.md`.

### Documentation
- Keep `docs/` up to date as you go — documentation is part of the deliverable, not an afterthought
- After completing a feature or fix, proactively update all affected documentation in the same commit. Don't wait to be asked "are docs up to date?"
- `docs/architecture.md` — crate layout, module responsibilities, data flow diagrams, key design decisions and rationale
- `docs/protocol.md` — CP2110 transport details, message formats, mode/range/unit tables, checksum algorithm, known quirks. Update this whenever real device behavior reveals new information.
- `docs/ux-design.md` — CLI command interface, output formats, GUI layout and interaction patterns
- `docs/setup.md` — build prerequisites, udev setup, first-run instructions, troubleshooting common issues
- `docs/development.md` — how to run tests, add new device models, coding conventions, release process
- `docs/adding-devices.md` — end-to-end guide: discovery, clean-room RE, implementation, spec data extraction, hardware verification, common pitfalls. **Read this before starting work on any new device.**
- `docs/research/<family>/reverse-engineering-approach.md` — per-family RE methodology, sources, confidence tags
- `docs/research/<family>/reverse-engineered-protocol.md` — per-family wire protocol specification (authoritative reference for implementation)
- `docs/verification-backlog.md` — update whenever items are verified or new unknowns are discovered. This is critical for preserving state across sessions.
- `CHANGELOG.md` — add entries for user-visible changes when preparing a release. Organized by component (GUI, CLI, Library, Bug fixes, Internal, Documentation). The release workflow extracts the entry for the tagged version.
- Escape angle brackets in markdown (`\<foo\>` or `` `<foo>` ``) — bare `<tags>` render as invisible HTML on GitHub.

### Logging
- Use the `log` crate with structured levels: `TRACE` for raw HID byte dumps, `DEBUG` for protocol events (request/response/checksum), `INFO` for connection state, `WARN` for recoverable issues (timeouts, retries), `ERROR` for failures
- `RUST_LOG=ut61eplus_lib=trace` should give complete wire-level debugging
- Never log at `INFO` or above in hot paths (measurement loop)

### GUI design
- **Test every visual change in both dark and light themes.** This has been the single largest source of rework — colors tuned for dark mode routinely fail WCAG contrast on light backgrounds.
- All colors must be theme-aware — use `ui.visuals().dark_mode` to select darker variants for light backgrounds
- WCAG 2.1 AA contrast ratios: ≥4.5:1 for text, ≥3:1 for graphical elements (lines, markers)
- Verify contrast ratios numerically when adding/changing colors (use relative luminance formula)
- Never rely on color alone — use bold/style, line patterns, or text labels as secondary indicators
- Minimum font size 11pt throughout
- Display value strings should use `display_raw` from the meter for stable width (no jitter)
- **Think through boundary conditions before writing code:** extreme window sizes (very wide, very narrow, maximized, quarter-screen), high zoom levels, empty/no-data state, and mode transitions. Don't ship the happy path and iterate.
- Start with the simplest visual approach. Prefer minimal rendering (lines, text labels) over complex shapes (filled polygons, gradients). Offer to enhance later if the user wants more.
- egui pitfalls learned the hard way:
  - `set_plot_bounds()` overrides both axes — if you only want to constrain X, compute Y range manually from visible data with padding.
  - `allow_drag(false)` also suppresses pointer position events; use `plot.reset()` per frame instead to pin the view while keeping events.
  - After mode changes or data clears, call `plot.reset()` to prevent stale bounds from the previous state.

### Dependencies
- Keep dependencies minimal and well-maintained
- Library crate: only `hidapi`, `thiserror`, `log`
- CLI crate: adds `clap`, `serde`, `serde_json`, `chrono`, `env_logger`, `ctrlc`, `console`, `serde_yaml`
- Avoid pulling in large frameworks for small tasks
