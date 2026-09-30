# Adding Device Support: End-to-End Guide

This guide covers the complete lifecycle for adding a new multimeter, from initial discovery through verified support. It captures methodology and lessons learned from every family added so far, and applies to any DMM reached over USB or Bluetooth — including non-UNI-T devices.

## Phase 1: Discovery and Candidate Assessment

**Goal:** Determine if a device is a viable candidate for support.

**Minimum requirements:**
- USB or Bluetooth connectivity with a documented or discoverable transport (HID, CDC/ACM serial, vendor-specific, Bluetooth LE)
- Vendor software or SDK available (needed for protocol reverse engineering)
- User manual with measurement mode and range details

**Ideal candidates:**
- Uses a cable an existing transport already handles ([cable table](supported-devices.md#cables-and-adapters)); the CP2110 HID-to-UART bridge (VID `0x10C4`, PID `0xEA80`) is the best-tested
- Uses a protocol similar to an already-supported family — reduces implementation effort
- Community implementations exist for cross-referencing (sigrok, GitHub projects)

**Steps:**
1. Check `docs/supported-devices.md` — the device may already be documented as a candidate or ruled out
2. Identify the USB transport: `lsusb` to get VID:PID, then search for the chip datasheet
3. Find the vendor software — manufacturer website, product CD, or community mirrors. Take phone apps from the vendor or Google Play, and a mirror only when neither has them.
   - **UNI-T:** start at the model's page on the Chinese sites, meters.uni-trend.com.cn (handhelds) and instruments.uni-trend.com.cn (bench meters). Their downloads carry protocol documents, per-model PC software and the phone app that the global site lacks.
   - `/search?keyword=<model>` on the handheld site and `/download?keyword=<model>` on the bench site search the download centre and give each file's link. Both match titles only, so also search the family prefix (`UT61`, not `UT61E+`) and 协议 (protocol). Protocol documents come as .pptx, .xls or .doc as well as PDF.
   - File links on `admin-meters.uni-trend.com.cn` fail; the same path on `meters.uni-trend.com.cn` works.
   - UNI-T's US site, uni-trendus.com, is fast for manuals and datasheets. The global uni-trend.com is slow, so note its links rather than download from it.
   - **Voltcraft:** Conrad's file server holds the protocol documents (`docs/research/new-device-candidates.md`, "Conrad (Voltcraft)").
4. Download the user manual
5. Store all assets in `references/<device>/` (manual PDF, installer ZIP, extracted binaries), noting the page each file came from

**Quick triage from vendor software contents:**
- `SLABHIDtoUART.dll` or `CP2110.dll` → Silicon Labs CP2110 HID-to-UART bridge
- `uci.dll` → UNI-T UCI SDK (bench DMM protocol, e.g., UT8803). It whitelists 5 USB-to-serial bridge VID:PID pairs used by bench meters (including Owon/Hoitek, WCH CH341, and QinHeng HID), which helps identify the bridge a new bench DMM uses
- `CH9329DLL.dll` → WCH CH9329 HID bridge (different transport than CP2110)
- QinHeng HID (VID `0x1A86`, PID `0xE008`) → WCH CH9325/CH9102 bridge, used by UT632/UT803/UT804; different from both CP2110 and CH9329, handled by the `Ch9325` transport
- Only `hid.dll` imports and a fixed VID:PID, with requests carrying no baud or UART setup → a cable that speaks the meter's protocol itself, like Brymen's BU-86X (`0x0820:0x0001`, the `Bu86x` transport); it relays no UART bytes (`relays_uart: false`), so only meters that list it are opened on it
- Direct serial port usage (`Qt5SerialPort.dll`, COM port references) → CDC/ACM or RS-232 adapter
- If none of the above match, the vendor software itself becomes the primary source for understanding the transport. A bridge no existing transport handles means a new one (Phase 4)

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

**Android apps:** `jadx -d <out> app.apk` decompiles to Java; an exit code of 3 only means some classes failed, and the tree is complete. `jadx` can mis-decompile, so check a decode that matters against the smali.

**Minified JavaScript** (uni-app or webpack bundles, often one line of several MB): beautify it first (`jsbeautifier`, `prettier`), prefixing each output line with its byte offset in the original so citations stay offsets into the file as shipped. Record the command in the approach doc.

If the protocol libraries are byte-identical across device variants, they share the same wire protocol and differ only in mode/range tables. This is how we confirmed UT61B+/D+/E+ and UT161B/D/E all use one protocol.

### Documentation deliverables

Create two files in `docs/research/<family>/`:

1. **`reverse-engineering-approach.md`** — Methodology: sources used, analysis steps taken, specific commands run, what each source revealed. Tag findings with confidence levels:
   - `[KNOWN]` — directly stated in official documentation
   - `[VENDOR]` — derived from vendor software decompilation
   - `[INFERRED]` — logically deduced from other findings
   - `[UNVERIFIED]` — requires real device testing to confirm
   - `[HARDWARE]` — seen on a real meter; names the issue, the cable and the reporter

2. **`reverse-engineered-protocol.md`** — Protocol specification: frame format, byte layouts, mode tables, command encoding, flag bits, checksum algorithm. This becomes the authoritative reference for implementation.

## Phase 3: Cross-Reference

**Goal:** Validate independent findings against community implementations.

**Only after completing Phase 2.** Look for:
- [sigrok](https://sigrok.org/) drivers — broad device coverage, well-tested
- GitHub projects for the specific device (search by model number)
- Community projects listed in the family's `docs/research/<family>/reverse-engineering-approach.md` cross-reference section and in [protocol.md](protocol.md#external-reference-implementations)
- Forum posts with protocol traces (EEVBlog, etc.)

Document discrepancies. When our independent analysis disagrees with community work, flag it for real-device verification rather than assuming either is correct.

**Exception:** If multiple independent community implementations agree and no vendor software is available for decompilation (e.g., UT181A), treating the community consensus as `[KNOWN]` is acceptable — document the sources.

## Phase 4: Implementation

**Goal:** Working protocol support with tests.

The protocol code lives in `crates/dmm-lib/src/protocol/<family>/`. The CLI and GUI pick new devices up from the registry, so no app code changes are needed.

**A new UT61+ model** (the same protocol, its own dial and ranges):

1. Add `protocol/ut61eplus/tables/<model>.rs` implementing `ModeTables`: one `entry` match returning the range table per mode, and `DeviceTable` comes from the blanket impl. Declare it in `tables/mod.rs`.
2. Map the model name to its table and its `SpecModel` in `Ut61PlusProtocol::for_model` (`ut61eplus/mod.rs`). Until its spec tables exist, the model takes `SpecModel::Untranscribed`.
3. Add its `SelectableDevice` entry in `ut61eplus/devices.rs` and list it in `DEVICES`.

`ModeTables` and `SpecModel` belong to the UT61+ family alone. Other families lay out their models their own way, such as `Vc8x0Model` in `vc8x0/` or the per-model parsers in `ut80x/`; a new model there follows its family's `mod.rs`.

**A new family** gets its own module, `protocol/<family>/`:

1. Implement the `Protocol` trait in its `mod.rs`. The trait, in `crates/dmm-lib/src/protocol/mod.rs`, says which methods are required and which have a default.
2. In `protocol/mod.rs`, declare the module and add a `DeviceFamily` variant with its `Display` arm.
3. Add its `SelectableDevice` entries in `protocol/<family>/devices.rs` and list them in `DEVICES`. Their `links` name the links the family is seen on: its USB cables by their transport's `NAME` (the transports themselves are in `KNOWN_TRANSPORTS`), and `BLUETOOTH` if a UT-D07B carries it. A meter with the radio built in and no cable sets the `bluetooth_names` it advertises and `BLUETOOTH` alone.
4. For a family on Bluetooth, add it to the lists of Bluetooth families in the `lib.rs` and `detect.rs` tests.
5. Keep its research docs in `docs/research/<family>/` (Phase 2).

**A new transport**, only when no existing link carries the meter: the [cable table](supported-devices.md#cables-and-adapters) lists the cables and adapters handled today. The protocol layer above is transport-agnostic.

- A USB-HID bridge or cable implements the `Transport` trait in `crates/dmm-lib/src/transport/`, taking `Cp2110`, `Ch9329`, `Ch9325`, `Bu86x` and `MockTransport` as models. It takes an entry in `KNOWN_TRANSPORTS` (`transport/open.rs`) and a line in `udev/70-dmm-tools.rules`; a test in `transport/open.rs` checks that the two list the same VID:PIDs.
- `transport/ble/` is not HID: it owns its async runtime and hands the layer above the same byte stream, for the UT-D07B and for meters with the radio built in. A Bluetooth meter on a GATT layout no profile covers takes a `GattProfile` in a file of its own under `transport/ble/`, listed in `PROFILES` (`profile.rs`).

**Key rules:**
- Set `Stability::Experimental` in `DeviceProfile` until verified against real hardware. The CLI prints a warning for every model short of `Stability::Verified`
- Set `max_aux_values` in `DeviceProfile` to the most sub-values one frame can carry (0 for single-display meters) — the CLI and GUI size their fixed sub-value columns from it
- `DEVICES` in `protocol/registry.rs` sets the picker order. An entry's `manual_url` points at a manufacturer-owned page, not a file-sharing mirror
- Set `DeviceProfile.verification_issue` to the model's verification issue (see Phase 7). The issue is posted once the code has been reviewed, so until then leave the field `None` and have `only_hardware_backed_models_are_verified` in `protocol/registry.rs` skip the new ids through an `ISSUE_TO_OPEN` list; the commit that links the issue deletes the list. Never put in a number that isn't a real issue
- Export a `Fingerprint` from the family module and point every one of the family's registry entries at it; its doc comment says what it holds. Test its rule beside it over the bytes it accepts: real ones where hardware exists, the vendor trace otherwise. `detect.rs` needs no change; [detection-design.md](detection-design.md) has the evidence ranking the rule has to hold its own in
- Implement `Protocol::capture_steps()`, the device's guided verification workflow: a `CaptureStep` per mode, flag state and remote command the device supports (the rules are in Phase 6). Someone without the device can then define exactly what needs verifying, and someone with it can run `capture` without understanding the protocol
- Where the parser meets data its spec doesn't cover — display text, a mode or range code, an undefined bit, a frame type — call `protocol::unrecognised::report_unknown`, which asks the user for a report once per session; documented values, benign or not, stay silent
- Write unit tests using `MockTransport` with byte sequences from the RE phase. Golden files wait for a hardware capture ([Golden file tests](development.md#golden-file-tests))

### Specification data

Add spec tables once a first hardware capture has confirmed the model; until then the Specifications panel shows the manual link, and `docs/verification-backlog.md` carries the task. If the device manual includes accuracy/resolution tables per mode and range:
1. Add the spec tables in the family module, following `ut61eplus/specs/`, `ut80x/specs_ut804.rs` or `ut181a/specs.rs`. How a manual table maps onto rows is in `crates/dmm-lib/src/specs.rs` and those files' module docs.
2. **Never fabricate values.** If a cell in the manual is ambiguous or you can't read it, give the row an empty accuracy list or omit the entry. Wrong specs are worse than missing specs.
3. Watch for common manual pitfalls:
   - **Merged cells** — one accuracy value spanning multiple ranges
   - **Frequency-dependent bands** — AC modes often have different accuracy for different frequency ranges (e.g., 40Hz-1kHz vs 1kHz-10kHz)
   - **LPF modes** — separate accuracy specs when Low Pass Filter is enabled
   - **Footnotes** — temperature coefficients, overrange conditions
   - **Model variants** — same manual covering multiple models with small spec differences (e.g., AC current frequency response differs between UT61B+ and UT61D+)
4. Transcribe from the rendered pages (`pdftoppm`), never extracted text. The `/spec-data` skill (`.claude/skills/spec-data/SKILL.md`) runs the workflow end to end: two blind transcriptions, an adjudication of each disagreement against the zoomed page, a diff of `dump_specs --format json` against the result and the `--format html` review sheet read beside the manual. [development.md](development.md#verifying-specification-data) has the `dump_specs` flags
5. Test that every reading the parser accepts resolves a spec or sits on an explicit list, with its reason, of readings that take only their table's mode data or have no spec at all, and that every table row is reached. The UT804's coverage test in `ut80x/mod.rs` is the model

## Phase 5: Testing Without Hardware

**Goal:** Verify correctness to the extent possible without a physical device.

1. **Unit tests** — parse known byte sequences from vendor software analysis
2. **Simulated meter** — only when the generic `mock` can't show what the meter adds (its own remote keys, a layout's display words): a `mock-<model>` entry like `protocol/zotek/sim.rs`, an arm in `mock::open_simulated`, and the mock list in the registry's tests. It offers only the keys the meter has, so a key that does nothing on the mock is a key the meter lacks
3. **Smoke test the CLI/GUI** — build and launch to confirm the new device appears in the device selector and the app doesn't crash.
4. **`cargo clippy --workspace --all-targets -- -D warnings`** and **`cargo test --workspace`** must pass

## Phase 6: Real Device Verification

**Goal:** Confirm the implementation against actual hardware. This is mandatory for removing the `Experimental` flag.

### Preparation
- Update `docs/verification-backlog.md` with items to verify for this device
- Ensure `RUST_LOG=dmm_lib=trace` logging captures raw bytes

### Testing protocol (needs someone with the meter)
1. **Start with basic connectivity:** `cargo run --bin dmm-cli -- --device <id> debug` to confirm frames are received and parseable
2. **Use the guided capture tool:** `cargo run --bin dmm-cli -- --device <id> capture` walks the user through each step and writes a YAML report of the raw bytes and parsed results, which can be shared in bug reports. It is the primary verification workflow; its design is [capture-design.md](capture-design.md).

   The steps are whatever the device's `Protocol::capture_steps()` returns; there is no separate table in the CLI. Take the six gate steps from `protocol::steps::gate_steps` and add the family's own:

   - **Cover every mode the dial or buttons reach, including AUTO** where the meter selects the function itself, plus every flag and remote command.
   - **Word each instruction from this model's manual** — its dial labels, button names and symbols (⎓ for DC). A step copied from a sibling carries the sibling's wording and its `.verified()`; reset both. Say "if the meter has it" for a feature only some models of the entry have, and never guess a threshold the manual doesn't give.
   - **Never direct anyone to touch mains** with the leads or probe an outlet. NCV held near a live wire is fine.
   - **Range button:** declare a step for it plus one that restores auto-ranging. Don't declare a *sweep* of successive presses until you know what the range command does on that meter — the UT61E+'s does not step the range table (see docs/verification-backlog.md), and a sweep that doesn't sweep files data that reads as authoritative range coverage but isn't.
   - **Declare only what the user must do by hand.** Once the gate passes, capture walks the ranges and flags `select()` reaches, and switches to a mode a button reaches from the current dial position ([capture-design.md](capture-design.md#d-autonomous-sub-steps-through-select)). Word such an instruction for a family without `choices`, where the operator still presses the button.
   - **Tag equipment** with `.needs(&[Need::…])` when an instruction asks for something beyond the meter and its leads — shorted probes, a DC source, a thermocouple — so the run can list it up front ([capture-design.md](capture-design.md#g-preparation-up-front)). The `Need`'s label has to fit the step.
   - **Order:** the six gate steps come first, as one block, before any other step. Capture sweeps nothing until the last gate step has reported, so a step placed among them is never swept (issue #19); a test in `crates/dmm-cli/src/capture/step.rs` enforces the order, with the older families still to fix in its allow-list. A mode the gate sits on still needs a plain step after the block for its ranges to be walked. After the gate, the dial turns one way through the run and lead changes are grouped.

   The freeform pass runs afterwards for every device and needs no declaration. `--list-steps --format md` prints the checklist the device's verification issue carries; regenerate it, don't hand-edit it, whenever the steps change.
3. **Test remote commands** (if supported): the capture tool covers these, but ad-hoc testing via `cargo run --bin dmm-cli -- --device <id> command <cmd>` is useful for debugging

### Common issues found during hardware verification
These are real bugs we discovered only through device testing — expect similar issues with any new device:
- **Frame length off-by-one** — length byte may count payload+checksum, not just payload
- **Mode byte encoding** — may be raw or have a prefix byte (e.g., 0x30) depending on the device
- **Flag bit positions** — bit assignments in vendor software may not match community documentation
- **Inverted flag logic** — some flags use inverted logic (bit clear = feature ON)
- **Bridge byte-at-a-time delivery** — USB HID bridges like CP2110 may deliver UART data one byte per interrupt report; frame assembly must keep reading until the frame completes (`read_frame` caps reads per frame, not per request)
- **Command ACK frames** — device may send short ACK responses that must be drained before the next measurement read
- **Display string encoding** — internal spaces for alignment (e.g., `"- 55.79"` for -55.79), trailing spaces, or device-specific overload strings

### Verification sign-off
Once verified:
1. Change `Stability::Experimental` to `Stability::Verified` in the device profile (`Stability::PartlyVerified` once connection and the main modes are confirmed but formats or commands remain; it behaves as Experimental and only changes the label)
2. Update `docs/verification-backlog.md` — mark items as completed with date, and in the same commit mark the capture steps they cover `.verified()` so `--unverified` stops asking for them
3. Add golden files from the capture report ([Golden file tests](development.md#golden-file-tests))
4. Update `docs/supported-devices.md` with verification status

## Phase 7: Documentation

Update these in the same commit as the code:

- The verification issue — the exception to "same commit": it is posted after the code and linked in a commit of its own (Phase 4). One issue per model name prefix: UT71A–E share one, a rebrand under another name gets its own, and a meter that reports only its packet layout gets one per layout entry, shared by the brands that send it. Its checklist is the `--list-steps --format md` output; `DeviceProfile.verification_issue`, the README row and the catalog's Status link it
- `README.md` — hand-edit the supported-devices table: one row per verification issue, abbreviated model runs, a USB and a Bluetooth status. What its test checks is in [Generated doc tables](development.md#generated-doc-tables)
- `docs/cli-reference.md` — regenerate the `--device` table ([Generated doc tables](development.md#generated-doc-tables)), and hand-edit the surrounding prose and anything the device adds to the CLI
- `docs/supported-devices.md` — add or update the device entry. Counts, form factor, cable and VID:PID live here; the generated `--device` table links here rather than repeating them
- `docs/protocol.md` — index entry pointing at the new family's spec
- `docs/detection-design.md` — a row in the cascade table for the probe the family answers to, or in its unprompted line if the meter streams by itself
- `docs/verification-backlog.md` — add pending verification items (or mark as complete), including a line under "Device auto-detection" for what detection sends this family and what it expects back
- `docs/gui-reference.md` — if the device adds new GUI behavior
- `docs/architecture.md` — for a new transport or GATT profile (its module map), or a new concept
- `docs/setup.md` — if the meter needs a new link or activation step (a Bluetooth adapter, a pairing quirk)
- `docs/research/new-device-candidates.md` — mark the model's candidate entry as supported
- `CHANGELOG.md` — one `## Unreleased` entry, in user-visible phrasing

## Quick Reference: File Locations

| What | Where |
|------|-------|
| Reference materials (manuals, binaries) | `references/<device>/` |
| RE methodology and findings | `docs/research/<family>/` |
| Protocol implementation | `crates/dmm-lib/src/protocol/<family>/` |
| Device tables (mode/range) | the family module; UT61+ models in `crates/dmm-lib/src/protocol/ut61eplus/tables/` |
| Spec data (accuracy/resolution) | `crates/dmm-lib/src/protocol/<family>/`: `specs/`, `specs_<model>.rs` or `specs.rs` |
| Device registry entry | `crates/dmm-lib/src/protocol/<family>/devices.rs`, ordered in `protocol/registry.rs` |
| Detection fingerprint | the family module, referenced from its registry entry |
| Golden test files | `crates/dmm-lib/tests/golden/<device id>/` |
| Verification status | `docs/verification-backlog.md` |
| Device catalog | `docs/supported-devices.md` |
