# Reverse Engineering Approach: UT61+/UT161 Protocol Family

## Objective

Determine whether the UT61B+, UT61D+, UT161B, UT161D, and UT161E share
the same USB communication protocol as the UT61E+ (already
reverse-engineered). Use only official, publicly available sources.

## Key Finding

**All six models use the identical wire protocol.** The same vendor
software binary (`Software V2.02`) serves all models with zero
model-specific protocol logic. The only differences are at the
application layer: display count, available measurement modes, range
tables, and bar graph segment count.

## Sources Used

1. **UT61+ Series User Manual** (UNI-T, P/N: 110401109614X) — single
   manual covering UT61B+, UT61D+, and UT61E+
2. **UT161 Series User Manual** (UNI-T) — single manual covering
   UT161B, UT161D, and UT161E
3. **UNI-T UT61E+ Software V2.02** — previously decompiled for the
   UT61E+ analysis (see `docs/research/ut61eplus/`)
4. **UT161E Software** — downloaded and binary-compared against V2.02
5. **CP2110 Datasheet** and **AN434** (Silicon Labs) — transport layer

No community implementations, forum posts, or third-party reverse
engineering work.

## Evidence: Single Shared Protocol

### Vendor software analysis — [VENDOR]

All four decompiled binaries from Software V2.02 were searched for
model-specific protocol logic:

- `CustomDmm_decompiled.txt` (13,115 lines) — protocol plugin
- `DMM_decompiled.txt` (48,396 lines) — main application
- `CP2110_decompiled.txt` (3,179 lines) — transport plugin
- `DeviceSelector_decompiled.txt` (9,850 lines) — device discovery

**Findings:**

1. **No model conditionals in protocol code.** Frame builder
   (`FUN_10002460`), frame parser (`FUN_10002540`), response parser
   (`FUN_10007d50`), mode/range lookup (`FUN_100023f0`), and command
   construction all have zero model parameters or model-name checks.

2. **Single flat mode/range table.** The table builder (`FUN_100027e0`
   in CustomDmm.dll, `FUN_00413f30` in DMM.exe) constructs one table
   containing ALL mode/range entries for ALL models. No model
   filtering — the meter firmware determines which modes are sent.

3. **Model name is cosmetic.** The string `"UT61B+"` is hardcoded as
   a default in the constructor. `options.xml` overrides it (e.g.,
   `<Model>UT61D+</Model>`). The value affects only the window title
   and export headers.

4. **One UI-only model check** (DMM.exe lines 3625-3639): two menu
   actions are hidden when the model name contains `"B"` (for
   UT61B+). This is purely cosmetic — likely hides Peak buttons.
   No protocol effect.

### UT161 binary comparison — [VENDOR]

The UT161E installer (43,129,397 bytes) was downloaded and extracted:

- **67 of 69 files are byte-for-byte identical** to the UT61E+ V2.02
  installer, including all protocol-critical binaries:
  `CustomDmm.dll`, `CP2110.dll`, `DeviceSelector.dll`,
  `SLABHIDtoUART.dll`
- **`DMM.exe`**: 8 bytes differ — only the model name string
  (`"UT161E"` vs `"UT61E+"`)
- **`options.xml`**: Only the `<Model>` tag differs
- **`uninst.exe`**: NSIS build nonces only

All three UT161 variants (B, D, E) serve the same zip from
meters.uni-trend.com. The model customization is purely cosmetic.

Note: The UT61E+ installer's `options.xml` says `<Model>UT61D+</Model>`
(not UT61E+), confirming UNI-T treats the model string as a UI label
with no protocol significance.

### LoZ mode disambiguation — [VENDOR]

The vendor software mode table has two entries both labeled "LozV":
mode 0x15 and mode 0x16. Code analysis found they are treated
differently:

- **Mode 0x16**: SI prefix multiplication applied (like Ohm/Cap/Hz
  modes). In CustomDmm.dll line 1519: `cVar1 == '\x16'` is in the
  multiplier group.
- **Mode 0x15**: No SI prefix multiplication (like voltage modes).
  Not in the multiplier group.
- Both show numeric bar graph (neither in the "bar graph = dash"
  group).
- Both share the same "LozV" display name string.

Which byte the UT61D+ sends for its single LoZ dial position requires
device testing.

### Mode 0x17 (LPF) behavior — [VENDOR]

Mode 0x17 appears in the "bar graph = dash" group (line 1600:
`cVar1 == '\x17'`) but not in the SI multiplier group. This means LPF
mode displays "-" for bar graph and uses raw display values — consistent
with a voltage measurement mode. Mode 0x18 has no special handling
anywhere in the code.

## Commands Reference

All analysis commands are documented in
`docs/research/ut61eplus/reverse-engineering-approach.md`. No new
decompilation was needed for the other models.

The UT161 binary comparison used:

```sh
# Extract and compare
7z x -o"references/ut161/extracted-nsis" \
    "references/ut161/UT161E-Software.zip"
7z x -o"references/ut161/extracted-nsis" \
    "references/ut161/extracted-nsis/Setup.exe"

# MD5 comparison of all files
find references/ut61eplus/vendor-software/extracted/ -type f \
    -exec md5sum {} \; | sort > /tmp/ut61e_hashes.txt
find references/ut161/extracted-nsis/ -type f \
    -exec md5sum {} \; | sort > /tmp/ut161e_hashes.txt
```

## File Inventory

| Source | File | What it provides |
|--------|------|-----------------|
| UNI-T | `references/ut61b-plus/ut61b_manual.pdf` | UT61+ Series manual (all 3 models) |
| UNI-T | `references/ut61d-plus/ut61d_manual.pdf` | Same UT61+ Series manual |
| UNI-T | `references/ut161/UT161E-Software.zip` | UT161E installer (confirmed identical) |
| UNI-T | `references/ut161/UT161-UserManual.pdf` | UT161 Series manual |
| UNI-T | `references/ut61eplus/vendor-software/extracted/` | Software V2.02 (shared) |
| Analysis | `references/ut61eplus/vendor-software/CustomDmm_decompiled.txt` | Protocol plugin |
| Analysis | `references/ut61eplus/vendor-software/DMM_decompiled.txt` | Main application |
