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
3. Find the vendor software — manufacturer website, product CD, or community mirrors. For UNI-T, start at the model's page on the Chinese sites, meters.uni-trend.com.cn (handhelds) and instruments.uni-trend.com.cn (bench meters): their downloads carry protocol documents, per-model PC software and the phone app that the global site lacks. `/search?keyword=<model>` on the handheld site and `/download?keyword=<model>` on the bench site search the download centre and give each file's link. Both match titles only, so also search the family prefix (`UT61`, not `UT61E+`) and 协议 (protocol); protocol documents come as .pptx, .xls or .doc as well as PDF. File links on `admin-meters.uni-trend.com.cn` fail; the same path on `meters.uni-trend.com.cn` works. For Voltcraft, Conrad's file server holds the protocol documents (`docs/research/new-device-candidates.md`, "Conrad (Voltcraft)")
4. Download the user manual
5. Store all assets in `references/<device>/` (manual PDF, installer ZIP, extracted binaries), noting the page each file came from

**Quick triage from vendor software contents:**
- `SLABHIDtoUART.dll` or `CP2110.dll` → Silicon Labs CP2110 HID-to-UART bridge
- `uci.dll` → UNI-T UCI SDK (bench DMM protocol, e.g., UT8803)
- `CH9329DLL.dll` → WCH CH9329 HID bridge (different transport than CP2110)
- QinHeng HID (VID `0x1A86`, PID `0xE008`) → WCH CH9325/CH9102 bridge, used by UT632/UT803/UT804; different from both CP2110 and CH9329, would need a third transport backend
- Direct serial port usage (`Qt5SerialPort.dll`, COM port references) → CDC/ACM or RS-232 adapter
- If none of the above match, the vendor software itself becomes the primary source for understanding the transport

**Non-CP2110 devices:** The `Transport` trait abstracts the byte-level transport. Adding a new transport backend requires implementing `Transport` — see the existing `Cp2110`, `Ch9329`, `Ch9325` and `MockTransport` for the interface, and `transport/ble.rs` for one that is not HID at all (the UT-D07B Bluetooth adapter: it owns its runtime and hands the layer above the same byte stream). The protocol layer above is transport-agnostic. Note: the UCI SDK's `uci.dll` contains a whitelist of 5 USB-to-serial bridge VID:PID pairs used by bench meters (including Owon/Hoitek, WCH CH341, and QinHeng HID), which is useful context for identifying which bridge a new bench DMM uses.

## Phase 2: Clean-Room Reverse Engineering

**Goal:** Reconstruct the wire protocol using only official, publicly available sources.

**Principle:** Use official sources first (manuals, datasheets, vendor software). Only cross-reference community implementations *after* completing independent analysis. This ensures our understanding is independently derived, so discrepancies between our analysis and community work can be identified in either direction.

### Source hierarchy

| Priority | Source | What it provides |
|----------|--------|------------------|
| 1 | User manual | Application semantics: modes, ranges, features, display format |
| 2 | USB bridge datasheet (CP2110, CH9329, etc.) | Transport layer: HID reports, feature reports, UART/bridge config |
| 3 | Programming manual, SDK docs or vendor protocol document (if exists) | Wire protocol — rare but invaluable (e.g., UT8803 has one; UNI-T's Chinese pages carry protocol documents for the UT61+ and UT804) |
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
- Community projects listed in the family's `docs/research/<family>/reverse-engineering-approach.md` cross-reference section and in the README's References
- Forum posts with protocol traces (EEVBlog, etc.)

Document discrepancies. When our independent analysis disagrees with community work, flag it for real-device verification rather than assuming either is correct.

**Exception:** If multiple independent community implementations agree and no vendor software is available for decompilation (e.g., UT181A), treating the community consensus as `[KNOWN]` is acceptable — document the sources.

## Phase 4: Implementation

**Goal:** Working protocol support with tests.

Follow the code-level steps in `docs/development.md`:
- **Same protocol family:** Add a table in `tables/` implementing `ModeTables` (one `entry` match per mode; `DeviceTable` comes from the blanket impl), and give the model's spec tables a `SpecModel` variant
- **New protocol family:** Implement the `Protocol` trait in `protocol/<family>/mod.rs`
- **New transport:** Implement the `Transport` trait (only if the device doesn't use CP2110)

**Key rules:**
- Set `Stability::Experimental` in `DeviceProfile` until verified against real hardware
- Set `max_aux_values` in `DeviceProfile` to the most sub-values one frame can carry (0 for single-display meters) — the CLI and GUI size their fixed sub-value columns from it
- Add `SelectableDevice` entry in `protocol/registry.rs` — CLI/GUI pick it up automatically
- Export a `Fingerprint` from the family module — what its probe sends, whether its extractor checks a checksum, which families the probe has to follow, and the rule that identifies the meter from its frames — and point every one of the family's registry entries at it, with a test beside the rule over the bytes it accepts — real ones where hardware exists, the vendor trace otherwise. Nothing is added to `crates/dmm-lib/src/detect.rs`; `docs/detection-design.md` has the evidence ranking the rule has to hold its own in
- Implement `capture_steps()` on the `Protocol` trait — this defines the guided verification workflow for the device. Each step has an `id`, a user-facing `instruction` (e.g., "Set meter to DC V mode"), an optional remote `command` to send, and a `samples` count. The default implementation returns an empty list, so the capture tool will have nothing to walk through unless you define steps. Cover all measurement modes, flag states, and remote commands the device supports. This decouples implementation from testing — someone without the device can define exactly what needs verifying, and someone with the device can run `capture` and walk through it without needing to understand the protocol.
- Where the parser meets data its spec doesn't cover — display text, a mode or range code, an undefined bit, a frame type — call `protocol::unrecognised::report_unknown`, which asks the user for a report once per session; documented values, benign or not, stay silent
- Write unit tests using `MockTransport` with byte sequences from the RE phase
- Add golden test files in `tests/golden/<device id>/` using YAML format (matches capture output) — the directory is named after the registry id; only samples from a hardware capture go there, so a family gains one with its first capture

### Specification data

If the device manual includes accuracy/resolution tables per mode and range:
1. Add the spec data. Spec tables sit in the family module: `ut61eplus/specs/`, `ut80x/specs_ut803.rs` and `specs_ut804.rs`, `ut181a/specs.rs`. Each manual table is a `ModeSpecs` of `RangeSpec` rows keyed by range byte and labelled as printed, listed in manual order in `ALL`, and one `table()` match picks a reading's table. Rows that need a different impedance or overload, or belong to another mode, go in a part of the same name.
2. **Never fabricate values.** If a cell in the manual is ambiguous or you can't read it, give the row an empty accuracy list or omit the entry. Wrong specs are worse than missing specs.
3. Watch for common manual pitfalls:
   - **Merged cells** — one accuracy value spanning multiple ranges
   - **Frequency-dependent bands** — AC modes often have different accuracy for different frequency ranges (e.g., 40Hz-1kHz vs 1kHz-10kHz)
   - **LPF modes** — separate accuracy specs when Low Pass Filter is enabled
   - **Footnotes** — temperature coefficients, overrange conditions
   - **Model variants** — same manual covering multiple models with small spec differences (e.g., AC current frequency response differs between UT61B+ and UT61D+)
4. Transcribe from the rendered pages (`pdftoppm`), not extracted text: two blind transcriptions, a field-by-field diff, and an adjudication of each disagreement against the zoomed page. Then diff `dump_specs --format json <device>` against the verified transcription, and read the `--format html` review sheet beside the manual. The `/spec-data` skill (`.claude/skills/spec-data/SKILL.md`) runs this end to end; `docs/development.md` has the `dump_specs` flags
5. Test that every reading the parser accepts resolves a spec or sits on an explicit list, with its reason, of readings that take only their table's mode data or have no spec at all, and that every table row is reached (`ut804_every_reading_has_a_spec_or_is_listed`)

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

   The steps are whatever the device's `Protocol::capture_steps()` returns, so a new device gets its coverage by declaring them there — there is no separate table in the CLI; take the six gate steps from `protocol::steps::gate_steps` and add the family's own. If the meter has a range button, declare a step for it plus one that restores auto-ranging. Resist declaring a *sweep* of successive presses until you know what the range command does on that meter — the UT61E+'s does not step the range table (see docs/verification-backlog.md), and a sweep that doesn't sweep files data that reads as authoritative range coverage but isn't. Once the gate steps pass, capture walks the ranges and flags `select()` can drive on its own, and switches to a step's mode itself when `choices(Setting::Mode)` offers it from the dial position the run is already at, so declare only what the user must do by hand — and word the instruction for the family without `choices`, where the operator still presses the button. Tag a step with `.needs(&[Need::…])` when its instruction asks for something beyond the meter and its leads — shorted probes, a DC source, a thermocouple, a transistor — so the run can list it up front and drop the step for a reporter who hasn't got one. Order the steps so the dial turns one way through the run, lead changes are grouped, and a gate step follows the mode step it extends. The freeform pass runs afterwards for every device and needs no declaration. Steps start unverified; `--unverified` runs just those, and `--list-steps --format md` prints the checklist the device's verification issue carries — regenerate it, don't hand-edit it, whenever the steps change.
4. **Test remote commands** (if supported): the capture tool covers these, but ad-hoc testing via `cargo run --bin dmm-cli -- --device <id> command <cmd>` is useful for debugging
5. **Capture golden test data:** copy verified samples from the capture YAML into `tests/golden/<device id>/` for regression testing — a report's `raw_hex` and parsed fields are the fixture format, so they transfer verbatim

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
1. Change `Stability::Experimental` to `Stability::Verified` in the device profile (`Stability::PartlyVerified` once connection and the main modes are confirmed but formats or commands remain; it behaves as Experimental and only changes the label)
2. Update `docs/verification-backlog.md` — mark items as completed with date, and in the same commit mark the capture steps they cover `.verified()` so `--unverified` stops asking for them
3. Add golden test files from the capture output
4. Update `docs/supported-devices.md` with verification status

## Phase 7: Documentation

Update these in the same commit as the code (the `/add-device` skill defers
to this list):

- `README.md` — hand-edit the supported-devices table: it is editorial (abbreviated model runs, per-family status wording), and the `dmm-cli` test only checks that no family and no verification issue is missing from it
- `docs/cli-reference.md` — the `--device` table is generated, not hand-edited: run `UPDATE_DOCS=1 cargo test -p dmm-cli` once the registry entry lands, and hand-edit the surrounding prose and anything the device adds to the CLI
- `docs/supported-devices.md` — add or update the device entry. Counts, form factor, cable and VID:PID live here; the generated `--device` table links here rather than repeating them
- `docs/protocol.md` — index entry pointing at the new family's spec
- `docs/detection-design.md` — a row in the cascade table for the probe the family answers to, or in its unprompted line if the meter streams by itself
- `docs/verification-backlog.md` — add pending verification items (or mark as complete), including a line under "Device auto-detection" for what detection sends this family and what it expects back
- `docs/gui-reference.md` — if the device adds new GUI behavior
- `docs/architecture.md` — if a new protocol family or transport changes the architecture
- `CHANGELOG.md` — one `## Unreleased` entry, in user-visible phrasing

## Quick Reference: File Locations

| What | Where |
|------|-------|
| Reference materials (manuals, binaries) | `references/<device>/` |
| RE methodology and findings | `docs/research/<family>/` |
| Protocol implementation | `crates/dmm-lib/src/protocol/<family>/` |
| Device tables (mode/range) | `crates/dmm-lib/src/protocol/<family>/tables/` |
| Spec data (accuracy/resolution) | `crates/dmm-lib/src/protocol/<family>/`: `specs/`, `specs_<model>.rs` or `specs.rs` |
| Device registry entry | `crates/dmm-lib/src/protocol/registry.rs` |
| Detection fingerprint | the family module, referenced from its registry entry |
| Golden test files | `crates/dmm-lib/tests/golden/<device id>/` |
| Verification status | `docs/verification-backlog.md` |
| Device catalog | `docs/supported-devices.md` |
