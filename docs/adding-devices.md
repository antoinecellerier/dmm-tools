# Adding Device Support: End-to-End Guide

This guide covers the complete lifecycle for adding a new multimeter, from initial discovery through verified support. It captures methodology and lessons learned from adding the UT61E+, UT8803, UT171, and UT181A families, but is written to apply to any USB-connected DMM — including non-UNI-T devices.

## Phase 1: Discovery and Candidate Assessment

**Goal:** Determine if a device is a viable candidate for support.

**Minimum requirements:**
- USB connectivity with a documented or discoverable transport (HID, CDC/ACM serial, vendor-specific)
- Vendor software or SDK available (needed for protocol reverse engineering)
- User manual with measurement mode and range details

**Ideal candidates:**
- Uses CP2110 HID-to-UART bridge (VID `0x10C4`, PID `0xEA80`) — our best-supported transport
- Uses a protocol similar to an already-supported family — reduces implementation effort
- Community implementations exist for cross-referencing (sigrok, GitHub projects)

**Steps:**
1. Check `docs/supported-devices.md` — the device may already be documented as a candidate or ruled out
2. Identify the USB transport: `lsusb` to get VID:PID, then search for the chip datasheet
3. Find the vendor software — manufacturer website, product CD, or community mirrors
4. Download the user manual
5. Store all assets in `references/<device>/` (manual PDF, installer ZIP, extracted binaries)

**Quick triage from vendor software contents:**
- `SLABHIDtoUART.dll` or `CP2110.dll` → Silicon Labs CP2110 HID-to-UART bridge
- `uci.dll` → UNI-T UCI SDK (bench DMM protocol, e.g., UT8803)
- `CH9329DLL.dll` → WCH CH9329 HID bridge (different transport than CP2110)
- QinHeng HID (VID `0x1A86`, PID `0xE008`) → WCH CH9325/CH9102 bridge, used by UT632/UT803/UT804; different from both CP2110 and CH9329, would need a third transport backend
- Direct serial port usage (`Qt5SerialPort.dll`, COM port references) → CDC/ACM or RS-232 adapter
- If none of the above match, the vendor software itself becomes the primary source for understanding the transport

**Non-CP2110 devices:** The `Transport` trait abstracts the byte-level transport. Adding a new transport backend (e.g., CDC serial via `serialport` crate, or QinHeng HID for UT632/UT803/UT804) requires implementing `Transport` — see the existing `Cp2110Transport`, `Ch9329Transport`, and `MockTransport` for the interface. The protocol layer above is transport-agnostic. Note: the UCI SDK's `uci.dll` contains a whitelist of 5 USB-to-serial bridge VID:PID pairs used by bench meters (including Owon/Hoitek, WCH CH341, and QinHeng HID), which is useful context for identifying which bridge a new bench DMM uses.

## Phase 2: Clean-Room Reverse Engineering

**Goal:** Reconstruct the wire protocol using only official, publicly available sources.

**Principle:** Use official sources first (manuals, datasheets, vendor software). Only cross-reference community implementations *after* completing independent analysis. This ensures our understanding is independently derived, so discrepancies between our analysis and community work can be identified in either direction.

### Source hierarchy

| Priority | Source | What it provides |
|----------|--------|------------------|
| 1 | User manual | Application semantics: modes, ranges, features, display format |
| 2 | USB bridge datasheet (CP2110, CH9329, etc.) | Transport layer: HID reports, feature reports, UART/bridge config |
| 3 | Programming manual or SDK docs (if exists) | Wire protocol — rare but invaluable (e.g., UT8803 has one) |
| 4 | Vendor software binaries | Protocol implementation: commands, framing, byte layouts |
| 5 | SDK examples/headers (if exists) | API definitions, struct layouts, flag constants |

### Binary analysis workflow

**Extraction:**
```sh
# NSIS installers (common for UNI-T and many Chinese manufacturers)
7z x Setup.exe -o./extracted

# InstallShield installers
# May require Wine: wine "Setup.exe" to extract, or use innoextract/unshield

# macOS .dmg or .pkg
# Mount and copy, or use pkgutil --expand
```

**String extraction** (quick triage — works on any binary):
```sh
strings -a app.exe | grep -i "baud\|uart\|9600\|115200\|COM\|HID\|frame\|checksum"
strings -el app.exe | grep -i "baud\|uart"  # wide (UTF-16LE) strings
```

**Ghidra decompilation** (for deep analysis of Windows/Linux/macOS binaries):
```sh
# Headless decompilation — produces C pseudocode for all functions
# GhidraDecompile.java script source is in
# docs/research/ut61eplus/reverse-engineering-approach.md
$GHIDRA/support/analyzeHeadless /tmp/ghidra_project project_name \
  -import app.exe \
  -postScript GhidraDecompile.java \
  -deleteProject \
  -scriptPath /tmp \
  > references/<device>/vendor-software/<name>_decompiled.txt 2>&1
```

**If analysis hangs** (100% CPU, never completes): the most likely cause
is a **symlink loop** under the `-scriptPath` directory. Ghidra's
`GhidraSourceBundle.findPackageDirs()` recursively walks the script path
without cycle detection. Wine prefixes under `/tmp` are a common culprit
(Wine creates `z: -> /` which makes `/tmp` contain a path back to itself).

Diagnosis: run `jstack <java-pid>` while Ghidra is stuck. If the thread
dump shows `findPackageDirs` in deep recursion, check for symlink loops:
```sh
find /tmp -maxdepth 3 -type l -exec test -d {} \; -print
```
Fix: remove the loop source, or use a dedicated script directory.

**What to look for in decompiled code:**
- Baud rate constants (`9600`, `115200`, `0x2580` = 9600 in big-endian)
- Frame header/magic bytes (e.g., `0xAB`, `0xCD`)
- Command byte tables or switch statements dispatching on command IDs
- Mode/range enum definitions
- Checksum calculation functions (sum, CRC, XOR)
- Struct definitions for measurement data (look for float/double fields, flag bitmasks)
- USB initialization sequences (VID/PID matching, interface claiming, endpoint selection)

**Binary comparison** (to identify shared protocols across device variants):
```sh
# Compare two extracted installers file-by-file
diff <(cd references/device-a/extracted && find . -type f | sort) \
     <(cd references/device-b/extracted && find . -type f | sort)

# Check if protocol-critical DLLs are identical
md5sum references/device-a/extracted/Lib/CustomDmm.dll \
       references/device-b/extracted/Lib/CustomDmm.dll
```

If the protocol libraries are byte-identical across device variants, they share the same wire protocol and differ only in mode/range tables. This is how we confirmed UT61B+/D+/E+ and UT161B/D/E all use one protocol.

### Documentation deliverables

Create two files in `docs/research/<family>/`:

1. **`reverse-engineering-approach.md`** — Methodology: sources used, analysis steps taken, specific commands run, what each source revealed. Tag findings with confidence levels:
   - `[KNOWN]` — directly stated in official documentation
   - `[VENDOR]` — derived from vendor software decompilation
   - `[INFERRED]` — logically deduced from other findings
   - `[UNVERIFIED]` — requires real device testing to confirm

2. **`reverse-engineered-protocol.md`** — Protocol specification: frame format, byte layouts, mode tables, command encoding, flag bits, checksum algorithm. This becomes the authoritative reference for implementation.

## Phase 3: Cross-Reference

**Goal:** Validate independent findings against community implementations.

**Only after completing Phase 2.** Look for:
- [sigrok](https://sigrok.org/) drivers — broad device coverage, well-tested
- GitHub projects for the specific device (search by model number)
- Community projects listed in `docs/supported-devices.md`
- Forum posts with protocol traces (EEVBlog, etc.)

Document discrepancies. When our independent analysis disagrees with community work, flag it for real-device verification rather than assuming either is correct.

**Exception:** If multiple independent community implementations agree and no vendor software is available for decompilation (e.g., UT181A), treating the community consensus as `[KNOWN]` is acceptable — document the sources.

## Phase 4: Implementation

**Goal:** Working protocol support with tests.

Follow the code-level steps in `docs/development.md`:
- **Same protocol family:** Add a `DeviceTable` implementation in `tables/`
- **New protocol family:** Implement the `Protocol` trait in `protocol/<family>/mod.rs`
- **New transport:** Implement the `Transport` trait (only if the device doesn't use CP2110)

**Key rules:**
- Set `Stability::Experimental` in `DeviceProfile` until verified against real hardware
- Add `SelectableDevice` entry in `protocol/registry.rs` — CLI/GUI pick it up automatically
- Implement `capture_steps()` on the `Protocol` trait — this defines the guided verification workflow for the device. Each step has an `id`, a user-facing `instruction` (e.g., "Set meter to DC V mode"), an optional remote `command` to send, and a `samples` count. The default implementation returns an empty list, so the capture tool will have nothing to walk through unless you define steps. Cover all measurement modes, flag states, and remote commands the device supports. This decouples implementation from testing — someone without the device can define exactly what needs verifying, and someone with the device can run `capture` and walk through it without needing to understand the protocol.
- Write unit tests using `MockTransport` with byte sequences from the RE phase
- Add golden test files in `tests/golden/<family>/` using YAML format (matches capture output)

### Specification data

If the device manual includes accuracy/resolution tables per mode and range:
1. Add spec data in `protocol/<family>/tables/specs_<model>.rs`
2. **Never fabricate values.** If a cell in the manual is ambiguous or you can't read it, use `Accuracy::NONE` or omit the entry. Wrong specs are worse than missing specs.
3. Watch for common manual pitfalls:
   - **Merged cells** — one accuracy value spanning multiple ranges
   - **Frequency-dependent bands** — AC modes often have different accuracy for different frequency ranges (e.g., 40Hz-1kHz vs 1kHz-10kHz)
   - **LPF modes** — separate accuracy specs when Low Pass Filter is enabled
   - **Footnotes** — temperature coefficients, overrange conditions
   - **Model variants** — same manual covering multiple models with small spec differences (e.g., AC current frequency response differs between UT61B+ and UT61D+)
4. Verify with `cargo run -p dmm-lib --example dump_specs -- <device>` and compare side-by-side with the PDF manual

## Phase 5: Testing Without Hardware

**Goal:** Verify correctness to the extent possible without a physical device.

1. **Unit tests** — parse known byte sequences from vendor software analysis
2. **Golden tests** — capture-format YAML files with expected parse results
3. **Smoke test the CLI/GUI** — build and launch to confirm the new device appears in the device selector and the app doesn't crash.
4. **`cargo clippy --workspace -- -D warnings`** and **`cargo test --workspace`** must pass

## Phase 6: Real Device Verification

**Goal:** Confirm the implementation against actual hardware. This is mandatory for removing the `Experimental` flag.

### Preparation
- Update `docs/verification-backlog.md` with items to verify for this device
- Ensure `RUST_LOG=dmm_lib=trace` logging captures raw bytes

### Testing protocol (requires user with physical device)
1. **Always describe the required physical setup** before each step and wait for confirmation
2. **Start with basic connectivity:** `cargo run --bin dmm-cli -- --device <id> debug` to confirm frames are received and parseable
3. **Use the guided capture tool:** `cargo run --bin dmm-cli -- --device <id> capture` walks the user through each mode, flag, and command step-by-step, recording raw bytes and parsed results. Use `--steps` to filter to specific items. This is the primary verification workflow — it produces a YAML report that documents exactly what was tested and can be shared in bug reports.
4. **Test remote commands** (if supported): the capture tool covers these, but ad-hoc testing via `cargo run --bin dmm-cli -- --device <id> command <cmd>` is useful for debugging
5. **Capture golden test data:** copy verified samples from the capture YAML into `tests/golden/<family>/` for regression testing

### Common issues found during hardware verification
These are real bugs we discovered only through device testing — expect similar issues with any new device:
- **Frame length off-by-one** — length byte may count payload+checksum, not just payload
- **Mode byte encoding** — may be raw or have a prefix byte (e.g., 0x30) depending on the device
- **Flag bit positions** — bit assignments in vendor software may not match community documentation
- **Inverted flag logic** — some flags use inverted logic (bit clear = feature ON)
- **Bridge byte-at-a-time delivery** — USB HID bridges like CP2110 may deliver UART data one byte per interrupt report; frame assembly needs enough retries (we use 64 attempts)
- **Command ACK frames** — device may send short ACK responses that must be drained before the next measurement read
- **Display string encoding** — internal spaces for alignment (e.g., `"- 55.79"` for -55.79), trailing spaces, or device-specific overload strings

### Verification sign-off
Once verified:
1. Change `Stability::Experimental` to `Stability::Verified` in the device profile
2. Update `docs/verification-backlog.md` — mark items as completed with date
3. Add golden test files from the capture output
4. Update `docs/supported-devices.md` with verification status

## Phase 7: Documentation

Update these files in the same commit as the code:
- `docs/supported-devices.md` — add or update the device entry
- `docs/verification-backlog.md` — add pending verification items (or mark as complete)
- `docs/cli-reference.md` — if the device adds new CLI options or behavior
- `docs/gui-reference.md` — if the device adds new GUI behavior
- `docs/architecture.md` — if a new protocol family or transport changes the architecture

## Quick Reference: File Locations

| What | Where |
|------|-------|
| Reference materials (manuals, binaries) | `references/<device>/` |
| RE methodology and findings | `docs/research/<family>/` |
| Protocol implementation | `crates/dmm-lib/src/protocol/<family>/` |
| Device tables (mode/range) | `crates/dmm-lib/src/protocol/<family>/tables/` |
| Spec data (accuracy/resolution) | `crates/dmm-lib/src/protocol/<family>/tables/specs_*.rs` |
| Device registry entry | `crates/dmm-lib/src/protocol/registry.rs` |
| Golden test files | `crates/dmm-lib/tests/golden/<family>/` |
| Verification status | `docs/verification-backlog.md` |
| Device catalog | `docs/supported-devices.md` |
