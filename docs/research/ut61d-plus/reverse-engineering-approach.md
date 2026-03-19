# Reverse Engineering Approach: UT61D+ Protocol

## Objective

Document the UT61D+ USB communication protocol using only official,
publicly available sources. Determine what differs from the UT61E+
(already reverse-engineered) and what is shared.

## Key Finding

**The UT61D+ uses the identical protocol as the UT61E+.** The same
vendor software (Software V2.02), the same CustomDmm.dll protocol
plugin, and the same DMM.exe application serve all three models
(UT61B+, UT61D+, UT61E+). No new decompilation was needed.

The only differences between models are at the application layer:
display count (6,000 vs 22,000), available measurement modes (the
UT61D+ has temperature and LoZ but lacks hFE, LPF, and AC+DC), range
tables (different full-scale values), and bar graph segment count
(31 vs 46).

## Sources Used

1. **UT61+ Series User Manual** (from UNI-T, covers UT61B+/UT61D+/UT61E+)
   — Downloaded from chipdip.ru mirror of the official UNI-T manual.
   Saved at `references/ut61d-plus/ut61d_manual.pdf` (identical to
   `references/ut61b-plus/ut61b_manual.pdf` — single manual covers
   all three models).
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

**UT61D+ specific information from the manual:**

- **Display**: 6,000 counts maximum (vs 22,000 for UT61E+)
- **Bar graph**: 31 segments at 30 updates/sec (vs 46 for UT61E+)
- **Refresh rate**: 2-3 times/sec (same as UT61E+)
- **Battery**: 4x 1.5V AAA (same as all models)
- **Safety**: CAT III 1000V / CAT IV 600V (same as all models)

**Modes available on UT61D+ (from function dial table, p.9):**
- AC/DC voltage measurement (V~, with Hz/% via SELECT2)
- DC voltage measurement (via SELECT from V~ position)
- AC/DC millivolt measurement (mV, with Hz/%)
- LoZ (low impedance) ACV measurement — **UT61D+ only**
- Continuity test / Resistance measurement / Capacitance (shared dial position)
- Diode test (shared dial position with continuity)
- Frequency / Duty ratio (Hz%)
- Temperature measurement (°C/°F) — **UT61D+ only**
- AC/DC microampere current (µA, with Hz/%)
- AC/DC milliampere current (mA, with Hz/%)
- AC/DC ampere current (A, with Hz/%) — max 20A
- NCV (non-contact voltage detection)

**Modes NOT available on UT61D+ (present on other models):**
- hFE / transistor magnification (UT61E+ only)
- AC+DC voltage (UT61E+ only)
- LPF (Low Pass Filter) voltage (UT61E+ only)

**UT61D+ has but UT61B+ lacks:**
- Temperature (°C/°F)
- LoZ ACV
- Peak measurement (P-MAX/P-MIN) — long press MAX/MIN button

**UT61D+ range tables (from specifications, pp.25-34):**
- DC Voltage: 60.00mV, 600.0mV, 6.000V, 60.00V, 600.0V, 1000V
- AC Voltage: 60.00mV, 600.0mV, 6.000V, 60.00V, 600.0V, 1000V
  - Plus LoZ ACV: 600.0V, 1000V
- Resistance: 600.0Ω, 6.000kΩ, 60.00kΩ, 600.0kΩ, 6.000MΩ, 60.00MΩ
- Capacitance: 60.00nF, 600.0nF, 6.000µF, 60.00µF, 600.0µF, 6.000mF, 60.00mF
- DC Current: 600.0µA, 6000µA, 60.00mA, 600.0mA, 6.000A, 20.00A
- AC Current: 600.0µA, 6000µA, 60.00mA, 600.0mA, 6.000A, 20.00A
- Temperature: -40°C to 1000°C (0.1°C-1°C resolution)
  - Also: -40°F to 1832°F (0.2°F-2°F resolution)
- Frequency: 10.00Hz-10.00MHz (resolution 0.01Hz-0.01MHz)
- Duty Cycle: 0.1%-99.9%
- Continuity: 600Ω threshold (beep <50Ω)
- Diode: 3V open circuit

### UNI-T Official Windows Software — No New Analysis Needed

**[VENDOR]** The UT61D+ uses the same Software V2.02 installer as the
UT61E+. Separate download pages exist on meters.uni-trend.com, but the
installers contain identical binaries. The extracted `options.xml`
actually defaults to `<Model>UT61D+</Model>`, suggesting the software
may have been originally packaged for the UT61D+.

| Model | Download page | Software |
|-------|--------------|----------|
| UT61B+ | `ut61b-software-v2-02-exe` | Software V2.02 |
| UT61D+ | `ut61d-software-v2-02-exe` | Software V2.02 |
| UT61E+ | `ut61e-software-v2-02-zip` | Software V2.02 |

## Model-Specific Logic in Vendor Software

### Findings

The analysis is identical to the UT61B+ findings. See
`docs/research/ut61b-plus/reverse-engineering-approach.md` for the
complete search methodology and evidence.

**Summary**: The vendor software contains **zero model-specific
protocol logic**. All protocol functions (framing, parsing, command
generation, mode/range lookup) are shared across all three models.
The mode/range table builder constructs a single flat table containing
entries for ALL modes from ALL models, with no filtering.

The only model-conditional code found in any binary is a UI visibility
check in DMM.exe (lines 3625-3639): two menu actions are hidden when
the model name string contains `"B"` (i.e., for the UT61B+). Since
`"UT61D+"` does not contain `"B"`, all UI actions are visible for
the UT61D+. This is purely cosmetic and has no effect on protocol
behavior. See `docs/research/ut61b-plus/reverse-engineering-approach.md`
for details.

### options.xml Model Field

The extracted `options.xml` contains `<Model>UT61D+</Model>`. This
is notable because:
1. The CustomDmm.dll constructor hardcodes `"UT61B+"` as the default
2. The options.xml overrides this to `"UT61D+"`
3. Neither value affects protocol behavior — only the UI title

This suggests the software was configured for a UT61D+ at the time
of extraction, which is purely a cosmetic/labeling difference.

## UT161 Series Relationship

The UT161 series (UT161B/D/E) is a related product line marketed as
CE-certified for European markets. Like the UT61+ series:
- UT161B and UT161D have 6,000-count displays
- UT161E has a 22,000-count display
- Separate software downloads exist (~40.7-41.0 MB each)

**[VENDOR] Confirmed by binary comparison** of the UT161E installer
(43,129,397 bytes). 67 of 69 files are byte-for-byte identical to the
UT61E+ V2.02 installer, including all protocol-critical binaries
(`CustomDmm.dll`, `CP2110.dll`). Only `DMM.exe` (model name string,
8 bytes) and `options.xml` (model tag) differ. See
`docs/research/ut61b-plus/reverse-engineering-approach.md` for full
comparison details.

## File Inventory

| Source | File | What it provides |
|--------|------|-----------------|
| UNI-T | `references/ut61d-plus/ut61d_manual.pdf` | UT61+ Series manual (all 3 models) |
| UNI-T | `references/ut61eplus/vendor-software/extracted/` | Same software for all models |
| Analysis | `references/ut61eplus/vendor-software/CustomDmm_decompiled.txt` | Protocol plugin (shared) |
| Analysis | `references/ut61eplus/vendor-software/DMM_decompiled.txt` | Main application (shared) |
| Analysis | `references/ut61eplus/vendor-software/extracted/options.xml` | Config with Model="UT61D+" |
