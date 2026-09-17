---
paths:
  - "crates/dmm-lib/**"
---

# Protocol and library rules (dmm-lib)

## Protocol correctness

- Protocol code is byte-level. Always validate checksums. Document byte offsets and masks with comments referencing `docs/research/<family>/reverse-engineered-protocol.md`.
- Test parsing with known-good byte sequences captured from real device traces.
- **Any protocol change MUST be verified against a real device before being considered done.** Use `RUST_LOG=dmm_lib=trace cargo run --bin dmm-cli -- --device <id> debug` to capture raw bytes. Three major bugs (frame length, mode enum, flag bits) only surfaced against real hardware.
- For unsafe or HID parsing code: confirm a malformed response cannot panic (check buffer sizes, bounds).
- Our protocol understanding comes from reverse engineering, not official documentation. See `docs/verification-backlog.md` for what's been verified and what's pending.
- Per-family protocol specs live in `docs/research/<family>/reverse-engineered-protocol.md`. `docs/protocol.md` is only an index.
- Reference implementations to cross-check when in doubt: [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) (Python, UT61E+), [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) (Go, UT61E+), [pylablib](https://github.com/AlexShkarin/pyLabLib) (Python, VC-880).
- Session time comes from `Dmm::clock()` — `Dmm::request_measurement` stamps every reading with it, so anything that reads elapsed session time takes it from there instead of `Instant::now()`. Hardware timeouts, settle delays and transport bring-up sleeps stay on `Instant` / `thread::sleep`: they pace physical USB, which no clock speeds up.
- Mocks must match real-device behavior: no impossible flag combinations (e.g. MIN+MAX simultaneously), correct data types for stored vs live values. Mocks that diverge create false confidence.

## Logging

- `log` crate, structured levels: `TRACE` for raw HID bytes, `DEBUG` for protocol events (request/response/checksum), `INFO` for connection state, `WARN` for recoverable issues (timeouts, retries), `ERROR` for failures.
- `RUST_LOG=dmm_lib=trace` should give complete wire-level debugging.
- Never log at `INFO` or above inside the measurement loop; per-frame trouble logs at `DEBUG`.
- Data the spec doesn't cover (display text, a mode or range code, an undefined bit, a frame type) goes through `protocol::unrecognised::report_unknown`, not a bare `warn!` or `debug!`: it warns once per process with a report hint. Only report what is outside the spec and the captures — a documented value, benign or not, stays silent, or every user of that meter sees the warning each session.

## Dependencies

- `dmm-lib` stays self-contained: only `hidapi`, `thiserror`, `log`. No external utility crates — this is the core that talks to hardware.
