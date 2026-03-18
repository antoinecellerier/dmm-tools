# UT61E+ Multimeter Tool

Rust workspace for communicating with the UNI-T UT61E+ multimeter via USB (CP2110 HID bridge).

## Project structure

- `crates/ut61eplus-lib/` — library: CP2110 transport, protocol framing, measurement parsing, device tables. `protocol` module is `pub(crate)` — consumers use `Dmm` API, not raw frame extraction.
- `crates/ut61eplus-cli/` — CLI binary (`main.rs` for commands, `capture.rs` for guided capture tool, `format.rs` for output formatting)
- `crates/ut61eplus-gui/` — GUI binary: real-time display and plotting (eframe/egui)

## Build & test

- `cargo build --workspace` — build everything
- `cargo test --workspace` — run all tests
- `cargo clippy --workspace -- -D warnings` — lint (must pass clean)
- `cargo fmt --check` — formatting check

## Engineering standards

### Code quality
- All code must pass `cargo clippy -- -D warnings` and `cargo fmt --check` before committing
- Write unit tests for any non-trivial logic, especially protocol parsing and byte manipulation
- Use `#[cfg(test)]` modules in the same file as the code being tested
- Prefer returning `Result` over panicking — `unwrap()` is only acceptable in tests and examples
- Use `thiserror` for error types in the library crate

### Protocol correctness
- The UT61E+ protocol is byte-level — off-by-one errors are easy to introduce
- Always validate checksums on received data
- Document byte offsets and masking operations with comments referencing the protocol spec
- Test parsing with known-good byte sequences captured from real device traces
- **Any protocol change MUST be verified against a real device before being considered done.** Use `RUST_LOG=ut61eplus_lib=trace cargo run --bin ut61eplus -- debug` to capture raw bytes. Unit tests alone are not sufficient — we've found three major bugs (frame length, mode enum, flag bits) that only showed up against real hardware.
- Byte masking rules (verified against device): mode byte is raw (no 0x30 prefix), range byte has 0x30 prefix (mask with `& 0x0F`), display bytes are ASCII (no masking), progress bytes are raw, flag bytes have 0x30 prefix (mask with `& 0x0F`). AUTO flag in byte 12 has inverted logic (bit clear = auto ON).
- Our protocol understanding comes from reverse engineering (USB traces, decompiled vendor apps, community work) — not official documentation. When adding new protocol features or encountering unexpected responses, capture raw hex dumps and verify against real device behavior before coding. See `docs/verification-backlog.md` for what's been verified and what still needs testing.
- Reference implementations: [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) (Python, most complete), [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) (Go). Cross-check against these when in doubt.

### Commit discipline
- **Every commit must include tests for new code** — write tests before or alongside the code, never defer them
- Commit logical units of work — one concept per commit
- Each commit should compile and pass tests (`cargo build && cargo test`)
- Write concise commit messages: imperative mood, explain the "why" not just the "what"
- Don't bundle unrelated changes in the same commit

### Review checklist (apply after each change)
- Does `cargo clippy --workspace -- -D warnings` pass?
- Does `cargo test --workspace` pass?
- Are new public types/functions documented?
- For protocol code: are byte offsets and masks correct? Cross-check against the protocol spec in this file.
- For unsafe or HID code: are buffer sizes correct? Can a malformed response cause a panic?
- For GUI render paths: avoid per-frame allocations in hot loops. Use cached data with dirty flags where possible. Graph segments and gap ranges are cached — invalidate via `invalidate_cache()` when data changes.
- For new buffers: ensure bounded growth. Graph history caps at 10K points, recording at 500K samples.

### Documentation
- Keep `docs/` up to date as you go — documentation is part of the deliverable, not an afterthought
- `docs/architecture.md` — crate layout, module responsibilities, data flow diagrams, key design decisions and rationale
- `docs/protocol.md` — CP2110 transport details, message formats, mode/range/unit tables, checksum algorithm, known quirks. Update this whenever real device behavior reveals new information.
- `docs/ux-design.md` — CLI command interface, output formats, GUI layout and interaction patterns
- `docs/setup.md` — build prerequisites, udev setup, first-run instructions, troubleshooting common issues
- `docs/development.md` — how to run tests, add new device models, coding conventions, release process
- When making changes that affect documented behavior, update the relevant docs in the same commit

### Logging
- Use the `log` crate with structured levels: `TRACE` for raw HID byte dumps, `DEBUG` for protocol events (request/response/checksum), `INFO` for connection state, `WARN` for recoverable issues (timeouts, retries), `ERROR` for failures
- `RUST_LOG=ut61eplus_lib=trace` should give complete wire-level debugging
- Never log at `INFO` or above in hot paths (measurement loop)

### GUI accessibility
- All colors must be theme-aware — use `ui.visuals().dark_mode` to select darker variants for light backgrounds
- WCAG 2.1 AA contrast ratios: ≥4.5:1 for text, ≥3:1 for graphical elements (lines, markers)
- Verify contrast ratios numerically when adding/changing colors (use relative luminance formula)
- Never rely on color alone — use bold/style, line patterns, or text labels as secondary indicators
- Minimum font size 11pt throughout
- Display value strings should use `display_raw` from the meter for stable width (no jitter)

### Dependencies
- Keep dependencies minimal and well-maintained
- Library crate: only `hidapi`, `thiserror`, `log`
- CLI crate: adds `clap`, `serde`, `serde_json`, `chrono`, `env_logger`, `ctrlc`, `console`, `serde_yaml`
- Avoid pulling in large frameworks for small tasks

## Protocol reference

CP2110 HID bridge: VID 0x10C4, PID 0xEA80, 9600 baud 8N1.

Init sequence (feature reports): enable UART [0x41,0x01], configure [0x50,0x00,0x00,0x25,0x80,0x00,0x00,0x03,0x00,0x00], purge [0x43,0x02].

Request measurement: AB CD 03 5E 01 D9. Response: AB CD + length + data, where length byte counts everything after itself (payload + 2-byte checksum). For measurements: length=0x10 (16), payload=14 bytes, checksum=2 bytes, total frame=19 bytes. Checksum: 16-bit BE sum of all bytes before checksum.

Payload: byte0=mode(raw, no 0x30), byte1=range(&0x0F), bytes2-8=display(ASCII), bytes9-10=progress(raw), byte11-13=flags(&0x0F). Display value may contain internal spaces for alignment (e.g. "- 55.79" for -55.79) — strip all spaces before parsing as f64. 9600 baud confirmed as only supported rate (~10 Hz max sampling).

Mode bytes (verified): 0x00=ACV, 0x01=ACmV, 0x02=DCV, 0x03=DCmV, 0x04=Hz, 0x05=Duty%, 0x06=Ohm, 0x07=Continuity, 0x08=Diode, 0x09=Capacitance, 0x0A=TempC, 0x0B=TempF, 0x0C=DCuA, 0x0D=ACuA, 0x0E=DCmA, 0x0F=ACmA, 0x10=DCA, 0x11=ACA, 0x12=hFE, 0x14=NCV.

Flag bytes: byte11(bit0=REL, bit1=HOLD, bit2=MIN, bit3=MAX), byte12(bit0=HV, bit1=LowBat, bit2=!AUTO inverted), byte13(bit0=bar_pol, bit1=PeakMIN, bit2=PeakMAX, bit3=DC).

Command encoding: [AB, CD, 03, cmd, (cmd+379)>>8, (cmd+379)&0xFF]. Commands: 0x41=MinMax, 0x42=ExitMinMax, 0x46=Range, 0x47=Auto, 0x48=Rel, 0x49=Select2, 0x4A=Hold, 0x4B=Light, 0x4C=Select, 0x4D=PeakMinMax, 0x4E=ExitPeak, 0x5E=GetMeasurement, 0x5F=GetName.
