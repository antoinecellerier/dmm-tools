# Reverse Engineering Approach: UT61B+ Protocol

## Objective

Document the UT61B+ USB communication protocol using only official,
publicly available sources. Determine what differs from the UT61E+
(already reverse-engineered) and what is shared.

## Key Finding

**The UT61B+ uses the identical protocol as the UT61E+.** The same
vendor software (Software V2.02), the same CustomDmm.dll protocol
plugin, and the same DMM.exe application serve all three models
(UT61B+, UT61D+, UT61E+). No new decompilation was needed.

The only differences between models are at the application layer:
display count (6,000 vs 22,000), available measurement modes (fewer
dial positions on the UT61B+), range tables (different full-scale
values), and bar graph segment count (31 vs 46).

## Sources Used

1. **UT61+ Series User Manual** (from UNI-T, covers UT61B+/UT61D+/UT61E+)
   — Downloaded from chipdip.ru mirror of the official UNI-T manual.
   Saved at `references/ut61b-plus/ut61b_manual.pdf`.
2. **UNI-T UT61E+ Software V2.02** — Previously decompiled for the
   UT61E+ analysis. The same software handles all three models.
   Decompiled outputs at `references/ut61eplus/vendor-software/`.
3. **CP2110 Datasheet** and **AN434** (Silicon Labs) — Same transport
   layer for all models.

## What Each Source Provides

### UT61+ Series User Manual

The manual is a combined document (P/N: 110401109614X) covering the
UT61B+, UT61D+, and UT61E+. Model-specific content is clearly marked
throughout.

**UT61B+ specific information from the manual:**

- **Display**: 6,000 counts maximum (vs 22,000 for UT61E+)
- **Bar graph**: 31 segments at 30 updates/sec (vs 46 for UT61E+)
- **Refresh rate**: 2-3 times/sec (same as UT61E+)
- **Battery**: 4x 1.5V AAA (same as all models)
- **Safety**: CAT III 1000V / CAT IV 600V (same as all models)

**Modes available on UT61B+ (from function dial table, p.9):**
- AC voltage measurement (V~, with Hz/% via SELECT2)
- DC voltage measurement (via SELECT from V~ position)
- AC/DC millivolt measurement (mV, with Hz/%)
- Continuity test / Resistance measurement (shared dial position)
- Diode test / Capacitance measurement (shared dial position)
- Frequency / Duty ratio (Hz%)
- AC/DC microampere current (uA, with Hz/%)
- AC/DC milliampere current (mA, with Hz/%)
- AC/DC ampere current (A, with Hz/%) — max 10A
- NCV (non-contact voltage detection)

**Modes NOT available on UT61B+ (present on other models):**
- Temperature (UT61D+ only)
- LoZ ACV (UT61D+ only)
- hFE / transistor magnification (UT61E+ only)
- AC+DC voltage (UT61E+ only)
- LPF (Low Pass Filter) voltage (UT61E+ only)
- Peak measurement P-MAX/P-MIN (UT61D+/UT61E+ only)

**UT61B+ range tables (from specifications, pp.25-34):**
- DC Voltage: 60.00mV, 600.0mV, 6.000V, 60.00V, 600.0V, 1000V
- AC Voltage: 60.00mV, 600.0mV, 6.000V, 60.00V, 600.0V, 1000V
- Resistance: 600.0Ω, 6.000kΩ, 60.00kΩ, 600.0kΩ, 6.000MΩ, 60.00MΩ
- Capacitance: 60.00nF, 600.0nF, 6.000µF, 60.00µF, 600.0µF, 6.000mF, 60.00mF
- DC Current: 600.0µA, 6000µA, 60.00mA, 600.0mA, 6.000A, 10.00A
- AC Current: 600.0µA, 6000µA, 60.00mA, 600.0mA, 6.000A, 10.00A
- Frequency: 10.00Hz-10.00MHz (resolution 0.01Hz-0.01MHz)
- Duty Cycle: 0.1%-99.9%
- Continuity: 600Ω threshold (beep <50Ω)
- Diode: 3V open circuit

### UNI-T Official Windows Software — No New Analysis Needed

**[VENDOR]** The UT61B+ uses the same Software V2.02 installer as the
UT61E+. UNI-T provides separate download pages for each model
(meters.uni-trend.com), but the installers contain identical binaries:

| Model | Download page | Software |
|-------|--------------|----------|
| UT61B+ | `ut61b-software-v2-02-exe` | Software V2.02 |
| UT61D+ | `ut61d-software-v2-02-exe` | Software V2.02 |
| UT61E+ | `ut61e-software-v2-02-zip` | Software V2.02 |

All three install the same `DMM.exe`, `CustomDmm.dll`, `CP2110.dll`,
and supporting files.

## Model-Specific Logic in Vendor Software

### What was searched

All four decompiled binaries were searched for model-specific logic:
- `CustomDmm_decompiled.txt` (13,115 lines)
- `DMM_decompiled.txt` (48,396 lines)
- `CP2110_decompiled.txt` (3,179 lines)
- `DeviceSelector_decompiled.txt` (9,850 lines)

Plus the configuration file `options.xml`.

### Findings

**1. Model name string** — [VENDOR]

The string `"UT61B+"` appears in two places:
- `CustomDmm.dll` constructor (`FUN_100016d0`, line 440): hardcoded
  as the default model name at offset `this+0x44`
- `DMM.exe` constructor (`FUN_00412700`, line 12386): same hardcoded
  default

The model name is purely cosmetic — it is displayed in the UI title
bar and written to `options.xml`. It has no effect on protocol
behavior, table selection, or command generation.

**2. options.xml Model field** — [VENDOR]

The extracted `options.xml` contains `<Model>UT61D+</Model>`. This is
a user-configurable setting (the XML is read/written by DMM.exe). The
value stored here does not affect protocol behavior — it is only used
for the window title and data export headers.

**3. Mode/range table builder** — [VENDOR]

The table builder function (`FUN_100027e0` in CustomDmm.dll,
`FUN_00413f30` in DMM.exe) was analyzed via disassembly for the UT61E+
research. It builds a single, flat lookup table containing ALL
mode/range combinations for ALL three models. There are NO model
conditionals — no `if (model == "UT61B+")` branches, no model
parameter passed to the function.

The table contains entries for modes that only exist on specific models
(e.g., temperature at 0x0A/0x0B, LoZ at 0x15/0x16, hFE at 0x12). The
meter itself determines which modes are available via its physical dial
and firmware — the PC software simply accepts whatever mode byte the
meter sends.

**4. Protocol code** — [VENDOR]

All protocol functions are model-independent:
- Frame builder (`FUN_10002460`): no model parameter
- Frame parser (`FUN_10002540`): no model parameter
- Response parser (`FUN_10007d50`): no model parameter
- Mode/range lookup (`FUN_100023f0`): searches flat table, no model
  filtering
- Command construction: same 0x5E (GetMeasurement), 0x4A (Hold),
  0x46 (Range) for all models

**5. CP2110 transport** — [VENDOR]

Same VID (0x10C4), PID (0xEA80), baud rate (9600), format (8N1) for
all models. The CP2110 chip is the same across the UT61+ series.

**6. UI-only model check** — [VENDOR]

DMM.exe line 606 sets a default model string `"UT61E+"` and version
`"2.02"`. Lines 3625-3639 contain the only model-conditional logic
found in any of the four binaries: the UI initialization function
checks whether the model name string contains `"B"`:

```c
// Pseudocode from DMM.exe lines 3625-3639
if ("UT61E+".indexOf("B") == -1) {
    action_0x68.setVisible(true);   // show action (e.g., Peak)
    action_0x6c.setVisible(true);   // show action
}
// When model is "UT61B+", indexOf("B") returns 4, so actions are hidden
```

This hides two UI menu actions when the model name contains "B"
(i.e., for the UT61B+). The hidden actions are likely Peak and/or
other features absent from the UT61B+. **This is purely cosmetic** —
it hides buttons in the GUI. It has no effect on protocol behavior,
command generation, or data parsing. The model name comes from
`options.xml`, which is user-editable.

### Conclusion

**The vendor software contains zero model-specific protocol logic.**
The only model check found is a UI visibility toggle that hides two
menu actions when the model name contains "B". All protocol functions
(framing, parsing, command generation, mode/range lookup) are shared
across all three models. Model differentiation happens entirely in the
meter's firmware (which modes the dial exposes, what mode bytes are
sent, what range values are used). The PC software is a generic
decoder for the shared protocol.

## UT161 Series Software

The UT161B/D/E series (a related product line also sold as a
CE-certified variant for European markets) has separate software
downloads on meters.uni-trend.com:

| Model | File size | Version label |
|-------|----------|---------------|
| UT161B | ~40.96 MB (.zip) | Not labeled V2.02 |
| UT161D | ~40.72 MB (.exe) | Not labeled V2.02 |
| UT161E | ~40.72 MB (.exe), ~40.96 MB (.zip) | Not labeled V2.02 |

**[VENDOR] Confirmed by binary comparison.** The UT161E installer was
downloaded (43,129,397 bytes) and extracted. Results:

- **67 of 69 files are byte-for-byte identical** to the UT61E+ V2.02
  installer, including all protocol-critical binaries: `CustomDmm.dll`,
  `CP2110.dll`, `DeviceSelector.dll`, `SLABHIDtoUART.dll`
- **`DMM.exe`**: 8 bytes differ — only the embedded model name string
  (`"UT161E"` vs `"UT61E+"`)
- **`options.xml`**: Only the `<Model>` tag differs
  (`"UT161E"` vs `"UT61D+"`)
- **`uninst.exe`**: Differs only in NSIS build nonces

All three UT161 variants (B, D, E) serve the same zip (43,129,397
bytes) from meters.uni-trend.com. The UT161 series uses the identical
protocol, identical binaries, and is differentiated only by the model
name string in DMM.exe and options.xml.

Note: The UT61E+ installer's options.xml says `<Model>UT61D+</Model>`
(not UT61E+), suggesting UNI-T built the UT61E+ package from the
UT61D+ config. This further confirms the model string is purely
cosmetic.

## File Inventory

| Source | File | What it provides |
|--------|------|-----------------|
| UNI-T | `references/ut61b-plus/ut61b_manual.pdf` | UT61+ Series manual (all 3 models) |
| UNI-T | `references/ut161/UT161E-Software.zip` | UT161E installer (confirmed identical to V2.02) |
| UNI-T | `references/ut161/UT161-UserManual.pdf` | UT161 Series manual (all 3 models) |
| UNI-T | `references/ut61eplus/vendor-software/extracted/` | Same software for all models |
| Analysis | `references/ut61eplus/vendor-software/CustomDmm_decompiled.txt` | Protocol plugin (shared) |
| Analysis | `references/ut61eplus/vendor-software/DMM_decompiled.txt` | Main application (shared) |
| Analysis | `references/ut61eplus/vendor-software/extracted/options.xml` | Config with Model="UT61D+" |
