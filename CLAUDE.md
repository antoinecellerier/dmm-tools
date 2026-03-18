# UT61E+ Multimeter Tool

Rust workspace for communicating with the UNI-T UT61E+ multimeter via USB (CP2110 HID bridge).

## Project structure

- `crates/ut61eplus-lib/` — library: CP2110 transport, protocol framing, measurement parsing, device tables
- `crates/ut61eplus-cli/` — CLI binary: data logging, device enumeration
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
- Test parsing with known-good byte sequences captured from reference implementations
- Flag bytes require `& 0x0F` masking (high nibble is 0x30); display value bytes do not
- Our protocol understanding comes from reverse engineering (USB traces, decompiled vendor apps, community work) — not official documentation. Verify assumptions against real device behavior whenever possible. When adding new protocol features or encountering unexpected responses, capture raw hex dumps and use them to refine our understanding before coding around assumptions.

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

### Dependencies
- Keep dependencies minimal and well-maintained
- Library crate: only `hidapi`, `thiserror`, `log`
- Avoid pulling in large frameworks for small tasks

## Protocol reference

CP2110 HID bridge: VID 0x10C4, PID 0xEA80, 9600 baud 8N1.

Init sequence (feature reports): enable UART [0x41,0x01], configure [0x50,0x00,0x00,0x25,0x80,0x00,0x00,0x03,0x00,0x00], purge [0x43,0x02].

Request measurement: AB CD 03 5E 01 D9. Response: AB CD + length + data, where length byte counts everything after itself (payload + 2-byte checksum). For measurements: length=0x10 (16), payload=14 bytes, checksum=2 bytes, total frame=19 bytes. Checksum: 16-bit BE sum of all bytes before checksum.

Payload: byte0=mode(raw, no 0x30), byte1=range(&0x0F), bytes2-8=display(ASCII), bytes9-10=progress(raw), byte11-13=flags(&0x0F).

Mode bytes (verified): 0x00=ACV, 0x01=ACmV, 0x02=DCV, 0x03=DCmV, 0x04=Hz, 0x05=Duty%, 0x06=Ohm, 0x07=Continuity, 0x08=Diode, 0x09=Capacitance, 0x0A=TempC, 0x0B=TempF, 0x0C=DCuA, 0x0D=ACuA, 0x0E=DCmA, 0x0F=ACmA, 0x10=DCA, 0x11=ACA, 0x12=hFE, 0x14=NCV.

Flag bytes: byte11(bit0=REL, bit1=HOLD, bit2=MIN, bit3=MAX), byte12(bit0=HV, bit1=LowBat, bit2=!AUTO inverted), byte13(bit0=bar_pol, bit1=PeakMIN, bit2=PeakMAX, bit3=DC).

Command encoding: [AB, CD, 03, cmd, (cmd+379)>>8, (cmd+379)&0xFF]. Commands: 0x41=MinMax, 0x42=ExitMinMax, 0x46=Range, 0x47=Auto, 0x48=Rel, 0x49=Select2, 0x4A=Hold, 0x4B=Light, 0x4C=Select, 0x4D=PeakMinMax, 0x4E=ExitPeak, 0x5E=GetMeasurement, 0x5F=GetName.
