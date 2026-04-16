# Data Flow Architecture Review

Review of the measurement data flow from hardware to GUI/CLI, with findings and recommendations.

Date: 2026-04-16

## Data Flow Overview

```
Meter (hardware)
  |
  | USB HID (interrupt transfers)
  v
Transport Layer (CP2110 / CH9329 / CH9325)
  |
  | read_timeout() / write() — raw bytes
  v
Protocol Layer (frame extractor + measurement parser)
  |
  | Measurement struct (Cow strings, f64 value, flags)
  v
Dmm<T> API — synchronous, single-threaded
  |
  +---> CLI: sync polling loop, format+flush each reading
  |
  +---> GUI: background thread -> mpsc::channel -> UI thread drains per frame
```

### CLI path

Single-threaded. `run_read_loop()` calls `dmm.request_measurement()` in a `while` loop
with absolute-tick pacing. Each measurement is formatted (text/CSV/JSON), flushed, and
discarded. Statistics (`RunningStats`, `Integrator`) accumulate in the loop.

Key file: `crates/dmm-cli/src/main.rs:603-730`

### GUI path

Background thread calls `dmm.request_measurement()` in a paced loop, sends
`DmmMessage::Measurement(m)` over `std::sync::mpsc::channel()` (unbounded).

UI thread drains all pending messages per frame via `try_iter()` in `drain_messages()`.
Each measurement is dispatched to:
- `graph.push()` — stores `(Instant, f64)` in a 10K-point `VecDeque`
- `stats.push()` — running min/max/avg
- `integrator.push()` — trapezoidal integration
- `recording.push()` — stores full `Sample` in a 500K-entry `Vec`
- `last_measurement` — keeps the most recent `Measurement` (moved, not cloned)

Key files:
- `crates/dmm-gui/src/app/connection.rs:58-164` (background thread)
- `crates/dmm-gui/src/app/mod.rs:593-702` (drain_messages)

---

## Findings

### F1. Recording `Sample` heap-allocates static strings

**Severity: Low** | **Verified**

`recording.rs:20-37`: `Sample::from_measurement()` converts `Cow<'static, str>` fields
(`mode`, `unit`, `range_label`) to owned `String` via `.to_string()`. At 10 Hz over
14 hours (500K samples), this produces ~2M unnecessary small heap allocations for values
that are almost always static borrows.

`flags` is the only field that genuinely needs formatting (it has no static string form).

### F2. Graph and Recording are separate, unsynchronized history buffers

**Severity: Medium** | **Verified**

Three parallel stores of the same measurement stream:

| Store | Type | Cap | Loses |
|-------|------|-----|-------|
| `graph.history` | `VecDeque<DataPoint>` | 10K | mode, unit, flags, display |
| `recording.samples` | `Vec<Sample>` | 500K | `Instant` timestamp, raw payload |
| `last_measurement` | `Option<Measurement>` | 1 | history |

Fed independently in `drain_messages()` (mod.rs:639-663). Graph discards metadata
(can't show mode/flags in tooltips). Recording can't serve graph's needs. Mode change
clears graph but not recording.

### F3. Per-measurement heap allocations cross the channel

**Severity: Low** | **Verified**

Every `Measurement` contains `display_raw: Option<String>` (7-char alloc),
`raw_payload: Vec<u8>` (14-19 bytes), and `aux_values: Vec<AuxValue>` (usually empty).
Negligible at 10 Hz but scales linearly with sample rate.

`raw_payload` is only useful for `TRACE`-level debugging and is dead weight in production.

### F4. Transport not paired with protocol at type level

**Severity: Low** | **Verified** | **No action needed**

`Dmm<T: Transport>` accepts any transport with any protocol. Pairing happens at runtime
in `open_device_by_id_auto()`. This is correct — transports are protocol-agnostic UART
bridges. Noted for awareness only.

### F5. Background thread error propagation uses strings

**Severity: Low** | **Verified**

`DmmMessage::Disconnected(String)` and `DmmMessage::Error(String)` flatten structured
errors before crossing the channel. The UI can't distinguish USB disconnect from protocol
error without string matching.

### F6. Reconnection loop is blocking and opaque

**Severity: Medium** | **Verified**

`connection.rs:130-143`: On device error, the background thread enters
`loop { sleep(2s); try reconnect }` that:

- Doesn't notify UI of retry attempts — UI sees `Disconnected`, then silence until success
- Uses fixed 2s interval regardless of error type
- Swallows all reconnection errors (`Err(_) => continue`)
- Uses `sleep` + `try_recv` instead of `recv_timeout` — up to 2s latency on stop signal

### F7. Protocol trait conflates polling and streaming

**Severity: Low** | **Verified**

Single `request_measurement()` method serves both polled (UT61E+ sends a command then
reads) and streaming (UT8803 just reads the next frame) protocols. Method name is
misleading for streaming. Caller's fixed-interval pacing is unnecessary for streaming
protocols and can cause stale reads if interval > native stream rate.

Works today because all supported streaming protocols have rates close to the polling
interval. Would need revisiting for high-rate streaming protocols.

### F8. Unbounded mpsc channel

**Severity: Low** | **Verified**

`app/mod.rs:514`: `mpsc::channel()` is unbounded. If UI stalls, measurements accumulate
without limit. At 10 Hz with ~200 bytes per `Measurement`, would take hours to matter.
Theoretical concern only at current data rates.

### F9. `rx_buf` grows without bound

**Severity: Low** | **Verified**

`framing.rs:66`: Each transport read appends up to 64 bytes to `rx_buf` with no size cap.
In practice, successful extraction or error recovery keeps it bounded. Worst case under
continuous garbage: 64 * 64 = 4 KB per call before timeout.

### F10. CLI and GUI duplicate measurement loop logic

**Severity: Medium** | **Verified**

Both consumers implement their own:
- Timeout counting and recovery (CLI: device-specific help; GUI: WaitingForMeter message)
- Pacing with absolute-tick timing (nearly identical code)
- Stop-signal handling (CLI: AtomicBool; GUI: mpsc channel)
- Statistics accumulation (both use `RunningStats` + `Integrator`)
- Mode-change detection (CLI: integral reset; GUI: integral + graph reset)

CLI: `main.rs:603-730`. GUI: `connection.rs:91-163` + `mod.rs:625-676`.
Improvements to one don't benefit the other.

### F11. WallClock drift on clock adjustment

**Severity: Low** | **Verified**

`wall_clock.rs:21-26`: Captures `(Instant, SystemTime)` once. NTP step or VM
suspend/resume during a session shifts all subsequent wall times. Standard trade-off;
monotonic timestamps remain correct for durations and graph rendering.

### F12. Spec lookup hardcoded to UT61E+ tables

**Severity: Medium** | **Verified**

`app/mod.rs:669-672`: `use dmm_lib::protocol::ut61eplus::tables` is used regardless of
which protocol family is connected. For non-UT61E+ devices, `lookup_spec()` returns
`None` silently (specs panel just shows nothing). The GUI has a hard dependency on
`ut61eplus::tables` module internals.

Only `ut61eplus/tables/mod.rs` has `lookup_spec` / `lookup_mode_spec` functions. No other
protocol family provides spec data through this path.

### F13. Measurement Clone copies raw_payload

**Severity: Low (downgraded)** | **Verified**

`Measurement` derives `Clone`, but in the GUI the hot path uses `Some(m)` (move), not
clone. `last_measurement` is accessed via `.as_ref()`. The `recording.samples.clone()` on
line 1466 is a one-time export operation. No performance concern in practice.

---

## Severity Summary

| Severity | # | Findings |
|----------|---|----------|
| Medium | F2 | Dual history buffers (graph + recording) |
| Medium | F6 | Reconnection loop blocking and opaque |
| Medium | F10 | CLI/GUI duplicate measurement loop logic |
| Medium | F12 | Spec lookup hardcoded to UT61E+ |
| Low | F1 | Recording Sample heap-allocates static strings |
| Low | F3 | Per-measurement heap allocs cross channel |
| Low | F5 | Error propagation uses strings |
| Low | F7 | Protocol trait conflates polling/streaming |
| Low | F8 | Unbounded mpsc channel |
| Low | F9 | rx_buf grows without bound |
| Low | F11 | WallClock drift on clock adjustment |
| Low | F13 | Measurement Clone copies raw_payload (downgraded) |
| Info | F4 | Transport not paired with protocol (no action) |
