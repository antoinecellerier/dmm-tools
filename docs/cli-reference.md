# dmm-cli(1) — CLI Reference

<!-- Keep this file in sync with the CLI. If you add, remove, or change
     commands or options, update the relevant section here in the same commit. -->

## Name

**dmm-cli** — command-line tool for UNI-T and Voltcraft multimeters

## Synopsis

```
dmm-cli <COMMAND> [OPTIONS]
```

## Description

Communicates with UNI-T and Voltcraft multimeters over USB. Supports live
measurement reading, button commands, settings switching, protocol debugging, and
guided data capture for verification. See [supported devices](supported-devices.md) for
the full compatibility list.

Set `NO_COLOR=1` to disable colored output.

## Global Options

| Option | Default | Description |
|---|---|---|
| `--device <FAMILY>` | see below | Device family to connect to. See [Device Families](#device-families) below. |
| `--adapter <SERIAL_OR_PATH>` | | Select a specific USB adapter when multiple are connected. Use serial number or HID device path from `list` output. |
| `-h, --help` | | Print help |
| `-V, --version` | | Print version |

### Devices

The `--device` flag selects which device model and protocol to use. Each model has
its own entry with model-specific protocol tables (e.g., UT61B+ uses different
mode/range mappings than UT61E+).

**Device resolution precedence** (highest to lowest):

1. `--device <FAMILY>` on the command line
2. `device_family` field in `~/.config/dmm-tools/settings.json` (written by `dmm-gui` when you pick a device in its settings panel — the CLI reads it but never writes to it)
3. `ut61eplus` as a final fallback

When the CLI falls through to the final fallback (you passed no `--device` and have no setting saved), a dim one-line notice is printed to stderr before the command runs, so silent use of the wrong protocol is less likely. The notice is suppressed for commands that don't open a device (`list`, `completions`).

| Value | Aliases | Description |
|---|---|---|
| `ut61eplus` | `ut61e+`, `ut61e` | UT61E+ (default, verified) |
| `ut61b+` | `ut61bplus`, `ut61b` | UT61B+ (experimental) |
| `ut61d+` | `ut61dplus`, `ut61d` | UT61D+ (experimental) |
| `ut161b` | | UT161B (experimental, same protocol as UT61B+) |
| `ut161d` | | UT161D (experimental, same protocol as UT61D+) |
| `ut161e` | `ut161` | UT161E (experimental, same protocol as UT61E+) |
| `ut8802` | `ut8802n` | UT8802 / UT8802N bench multimeter (experimental) |
| `ut8803` | `ut8803e` | UT8803 / UT8803E bench multimeter (experimental) |
| `ut803` | | UT803 bench multimeter, 6000 counts (experimental) |
| `ut804` | | UT804 bench multimeter, 4000 counts (experimental) |
| `ut171` | `ut171a`, `ut171b`, `ut171c` | UT171A/B/C (experimental) |
| `ut181a` | `ut181` | UT181A (confirmed on real hardware; still warns as experimental, see below) |
| `vc880` | `vc-880` | Voltcraft VC-880 handheld DMM (experimental) |
| `vc650bt` | `vc-650bt` | Voltcraft VC650BT bench DMM (experimental, same protocol as VC-880) |
| `vc890` | `vc-890` | Voltcraft VC-890 handheld DMM, 60K counts, OLED (experimental) |
| `mock` | | Simulated device (no hardware required) |

Non-UT61E+ families are marked **experimental** -- their protocols were reverse-engineered
from vendor software, and most have not yet been verified against real hardware. The UT181A
is the exception: two reporters have run it on a real meter over the CH9329 (UT-D09) cable,
confirming V DC, V AC + Hz and dual-probe temperature. It keeps the experimental warning
until the REL, MIN/MAX, Peak and COMP formats, the remote commands and the older CP2110
cable are verified too. When connecting to an experimental device, the CLI prints a yellow
warning with a link to the device's verification issue on GitHub. Please report findings
there.

The `mock` device generates synthetic measurements cycling through multiple modes
(DC V, AC V, Ohms, Capacitance, Hz, Temperature, DC mA, Overload, NCV). It requires
no USB hardware and is useful for development, demos, and testing output formats.
Supports the `read`, `command`, `get` and `set` subcommands. The `info`, `debug`, and `capture`
subcommands require real hardware and will exit with an error when used with `mock`.

#### Mock Modes

By default, the mock device cycles through all modes automatically. Use
`--mock-mode` with `read` to pin to a specific mode:

| Mode | Description |
|---|---|
| `dcv` | DC Voltage (sine wave around 5V) |
| `acv` | AC Voltage (sine wave around 120V) |
| `ohm` | Resistance (step 1-10 kΩ) |
| `cap` | Capacitance (ramp 1-20 µF) |
| `hz` | Frequency (sine wave around 60Hz) |
| `temp` | Temperature (ramp 20-30°C) |
| `dcma` | DC mA (sine wave around 50mA) |
| `ohm-ol` | Resistance overload (OL) |
| `ncv` | NCV (cycling levels 0-4) |
| `acv-hz` | AC Voltage with frequency and period sub-displays |
| `temp2` | Temperature with a second thermocouple (T2) |
| `temp-diff` | Temperature difference T1-T2 |
| `temp-diff-rev` | Temperature difference T2-T1 |

**Examples:**

```bash
# Default (UT61E+ family)
dmm-cli read

# Connect as UT8803
dmm-cli --device ut8803 read

# Connect as UT181A
dmm-cli --device ut181a info

# Use simulated device (no hardware)
dmm-cli --device mock read

# Pin mock to DC voltage mode
dmm-cli --device mock read --mock-mode dcv
```

## Commands

### dmm-cli list

List connected USB adapters.

```
dmm-cli list
```

Prints each detected device with an index number and transport type. If no
devices are found, prints troubleshooting hints (udev rule install on Linux,
driver install on Windows).

When multiple devices are connected, use `--adapter` with a serial number or
HID path from the `list` output to select a specific device:

```
dmm-cli list
# [0] /dev/hidraw3 [CP2110] — CP2110 HID UART Bridge (S/N: 00C5B27A)
# [1] /dev/hidraw5 [CP2110] — CP2110 HID UART Bridge (S/N: 00D8F132)

dmm-cli --adapter 00C5B27A read
```

### dmm-cli info

Connect to the meter and print device info: model name, transport type, and
transport-specific diagnostics (e.g., CP2110 firmware version and UART error flags).

```
dmm-cli info
```

### dmm-cli read

Continuously read measurements from the meter.

```
dmm-cli read [OPTIONS]
```

| Option | Default | Description |
|---|---|---|
| `--interval-ms <MS>` | `0` | Interval between readings in milliseconds. 0 = fastest (~10 Hz). |
| `--format <FORMAT>` | `text` | Output format: `text`, `csv`, or `json`. |
| `-o, --output <FILE>` | stdout | Write output to a file instead of stdout. |
| `--count <N>` | `0` | Number of readings to take. 0 = unlimited (Ctrl+C to stop). |
| `--mock-mode <MODE>` | | Pin mock device to a specific mode (only with `--device mock`). See [Mock Modes](#mock-modes). |
| `--integrate` | off | Show cumulative time-integral. For current modes, this computes charge (Ah/mAh/µAh). For voltage modes, V·s. Adds `integral` and `integral_unit` columns to CSV/JSON output. |
| `--scale <FACTOR>` | `1` | Multiply the reading, taken in base units, by FACTOR. See [Scaling readings in software](#scaling-readings-in-software). |
| `--offset <VALUE>` | `0` | Add VALUE after scaling. |
| `--unit <LABEL>` | | Label the scaled reading with LABEL instead of the meter's base unit. |

CSV output begins with a `# device:` comment line identifying the meter model,
followed by the column header. JSON output begins with a `_metadata` line
containing the device model, followed by one measurement object per line.

Meters that report sub-values alongside the reading — the UT181A's second
thermocouple, its frequency and period displays, and its REL, MIN/MAX and Peak
modes; the UT171's frequency aux — show them indented under the reading in text
output and in an `aux` array in JSON. Capture reports carry them per sample
under `aux`.

CSV appends one `auxN_label,auxN_value,auxN_unit` group per sub-value the meter
family can send: four for the UT181A, one for the UT171, two for the mock
device. Single-display meters (UT61E+, UT61B+/D+, UT161x, UT8802, UT8803,
UT803/UT804, VC-880, VC-890) send none, so their files carry the six original
columns and are unchanged. The count is per meter family, not per mode, so a
reading that fills fewer slots leaves the rest empty and every row of a file
lines up. With `--integrate`, the `integral` and `integral_unit` columns come
first, ahead of the aux groups. Software scaling adds one more group, always
the last one, for the meter's own reading, so a scaled run has family slots + 1
— see [Scaling readings in software](#scaling-readings-in-software).

```
# device: UNI-T UT181A
timestamp,mode,value,unit,range,flags,aux1_label,aux1_value,aux1_unit,aux2_label,aux2_value,aux2_unit,aux3_label,aux3_value,aux3_unit,aux4_label,aux4_value,aux4_unit
2026-09-02T09:33:56.123+02:00,V AC Hz,239.22,VAC,600V,AUTO HV!,Frequency,50.01,Hz,Period,20.00,ms,,,,,,
2026-09-02T09:34:10.456+02:00,°C,25.4,°C,,AUTO,T2,24.6,°C,,,,,,,,,
```

When the session ends, a summary line (sample count, min, max, average, each
with its unit) is printed to stderr. When `--integrate` is active, the total
integral is also shown.

Statistics and the integral cover a single mode and unit: if either changes
mid-run — by turning the dial, or by auto-range crossing a decade — both reset
and a note is printed to stderr, so the summary always describes one comparable
series. Both are watched, not just the unit: `--unit` pins the label, so a dial
turn would otherwise go unnoticed.

**Examples:**

```bash
# Stream readings to the terminal
dmm-cli read

# Record 100 CSV samples to a file
dmm-cli read --format csv --count 100 -o measurements.csv

# JSON output at 1-second intervals
dmm-cli read --format json --interval-ms 1000

# Measure battery discharge capacity (coulomb counter)
dmm-cli read --integrate --format csv -o discharge.csv
```

#### Scaling readings in software

`--scale`, `--offset` and `--unit` re-express the reading on the PC for
sensors the meter knows nothing about — a current clamp's mV/A, a shunt, a
probe divider, °C to °F. Nothing is sent to the meter.

The factor is per **base unit**: the reading is converted to V, A, Ω, …
before scaling, so a factor survives auto-ranging between mV and V. A
10 mV/A clamp is `--scale 100` (0.010 V/A → 100 A per volt). Order: strip
the SI prefix, multiply by `--scale`, add `--offset`, relabel with `--unit`;
without `--unit` the reading is shown in the base unit.

The meter's own reading rides along as a `Raw` sub-value — indented in text,
in the JSON `aux` array, and in the last CSV aux group (family slots + 1).
That group is reserved for it, so `Raw` stays in the same columns whether or
not the meter sent sub-values of its own on a given frame. Sub-values that
measure the same quantity as the reading — a second thermocouple, a REL
reference, the MIN/MAX extremes — are scaled with it and shown in the same
unit; sub-values in another unit, such as a frequency beside a voltage, are
left as the meter sent them. `OL` and NCV rows pass through with only the
unit relabelled. Statistics and `--integrate` use the scaled reading, so a
clamp relabelled to `A` integrates to Ah. A dim note on stderr marks a
scaled run; stdout is untouched.

```bash
dmm-cli read --scale 100 --unit A                 # 10 mV/A clamp → amps
dmm-cli read --scale 100                          # 100:1 HV probe, stays in V
dmm-cli read --scale 1.8 --offset 32 --unit °F    # °C → °F
```

### dmm-cli command

Send a remote command to the meter. Available commands depend on the
device family. Run with no arguments to list available commands:

```
dmm-cli command              # list commands for default device
dmm-cli --device ut181a command  # list commands for UT181A
dmm-cli command <ACTION>     # send a command
```

#### UT61E+ commands

| Command | Description |
|---|---|
| `hold` | Toggle Hold mode |
| `minmax` | Enter Min/Max recording |
| `exit_minmax` | Exit Min/Max recording |
| `rel` | Toggle Relative mode |
| `range` | Cycle manual range |
| `auto` | Return to auto-range |
| `select` | Select button (mode-dependent) |
| `select2` | Select2 / Hz button (mode-dependent) |
| `light` | Toggle backlight |
| `peak` | Enter Peak Min/Max mode |
| `exit_peak` | Exit Peak Min/Max mode |

`select` and `select2` are raw presses — each steps the dial position's ring
one function on, whatever that turns out to be. To name the mode, range, HOLD,
REL, MIN/MAX or Peak value you want instead of stepping to it, use
[`dmm-cli set`](#dmm-cli-set).

#### UT181A commands

| Command | Description |
|---|---|
| `hold` | Toggle Hold mode |
| `range` | Step to the next manual range for the current dial position |
| `auto` | Return to auto-range |
| `rel` | Toggle relative (REL) mode |
| `minmax` | Enable Min/Max recording |
| `exit_minmax` | Disable Min/Max recording |
| `monitor` | Enable streaming (SET_MONITOR) |
| `save` | Save current measurement to device memory |

`range` is refused in fixed-range modes; `rel` in continuity, diode,
differential temperature and any mode's Hz or Peak variant. Naming the function
within a dial position (V AC → V AC Hz, …), the range, HOLD, REL or MIN/MAX you
want is [`dmm-cli set`](#dmm-cli-set).

#### UT171 commands

| Command | Description |
|---|---|
| `connect` | Start measurement streaming |
| `pause` | Stop measurement streaming |

#### VC-880 / VC650BT / VC-890 commands

| Command | Description |
|---|---|
| `hold` | Toggle Hold mode |
| `rel` | Toggle relative (REL) mode |
| `max_min_avg` | Cycle Max/Min/Avg recording |
| `exit_max_min_avg` | Exit Max/Min/Avg recording |
| `range_manual` | Switch to manual ranging |
| `range_auto` | Return to auto-range |
| `light` | Toggle backlight |
| `select` | SHIFT/SETUP button (steps the dial position's functions) |

None of these is confirmed on hardware (issues #13 and #14). Naming the
function, range, HOLD, REL or MIN/MAX value you want instead of stepping to it
is [`dmm-cli set`](#dmm-cli-set).

#### UT8803

No remote commands — the meter streams continuously after connection.

**Example:**

```bash
dmm-cli command hold
dmm-cli --device ut181a command hold
```

### dmm-cli get

List what the meter's settings can be switched to from where it sits now —
mode, range, HOLD, REL, MIN/MAX and Peak, without touching the dial
(UT61+/UT161, UT181A, VC-880/VC650BT, VC-890 and mock). Run with no argument
for every setting that offers a choice.

```
dmm-cli get                  # one row per setting, * = the live value
dmm-cli get <SETTING>        # that setting alone, with what to type for each value
```

| Argument | Default | Description |
|---|---|---|
| `<SETTING>` | all of them | `mode`, `range`, `hold`, `rel`, `minmax` or `peak`. |

| Option | Default | Description |
|---|---|---|
| `--format <FORMAT>` | `text` | Output format: `text` or `json`. |
| `--mock-mode <MODE>` | | Pin mock device to a specific mode (only with `--device mock`). See [Mock Modes](#mock-modes). |

Every listing is relative to where the meter sits: it names what can be reached
without turning the dial. A setting the meter offers no choice in — Peak on a
meter that has none, a dial position with a single function — is left out of
the whole-meter listing, and on its own prints a note and exits 0. While the
meter is autoranging, the range row says which rung it picked.

```
$ dmm-cli get
Settings for UT61E+ (DC V):
  mode   * DC V  AC+DC V
  range  * Auto  2.2V  22V  220V  1000V  (auto-ranging in 22V)

Tip: switch one by name, e.g. dmm-cli set range 22V
```

`--format json` prints one object per invocation — not one per line, unlike
`read` — on stdout, with any note on stderr. `get <SETTING>` is flat:

```json
{
  "device": "UT61E+",
  "mode": "DC V",
  "range": "22V",
  "setting": "range",
  "current": "Auto",
  "choices": [
    { "id": 0, "label": "Auto", "current": true },
    { "id": 1, "label": "2.2V", "current": false }
  ]
}
```

`get` with no setting nests the same blocks under `settings`, one per setting
that offers a choice:

```json
{
  "device": "UT61E+",
  "mode": "DC V",
  "range": "22V",
  "settings": [
    { "setting": "mode", "current": "DC V", "choices": [] },
    { "setting": "range", "current": "Auto", "choices": [] }
  ]
}
```

| Field | Description |
|---|---|
| `device` | Model name of the connected meter. |
| `mode` | What it is measuring, as `read` reports it. |
| `range` | The live range label, autoranging included. |
| `setting` | Which setting the block is about. |
| `current` | Label the meter sits on, or `null` when the list names none. |
| `choices[].id` | The number `set` resolves a label to; `0` is Auto for `range`, off for `hold`, `rel`, `minmax` and `peak`. |
| `choices[].label` | Display label — what `set` takes. |
| `choices[].current` | Whether the meter is on this value. |

**Example:**

```bash
dmm-cli get                        # everything switchable from where it sits
dmm-cli get range                  # the ranges the current mode offers
dmm-cli get --format json          # one object, for scripts
dmm-cli --device ut181a get mode
```

### dmm-cli set

Switch one of the meter's settings by name. Run without a choice to list what
that setting reaches from here:

```
dmm-cli set <SETTING>            # list the values (* = live) and what to type for each
dmm-cli set <SETTING> <CHOICE>   # switch, by label
```

| Argument | Default | Description |
|---|---|---|
| `<SETTING>` | | `mode`, `range`, `hold`, `rel`, `minmax` or `peak`. |
| `<CHOICE>` | list them | Label to switch to, or a unique fragment of one. |

| Option | Default | Description |
|---|---|---|
| `--mock-mode <MODE>` | | Pin mock device to a specific mode (only with `--device mock`). See [Mock Modes](#mock-modes). |

`<CHOICE>` is a label from the listing, case-insensitive, or a fragment of one
that matches a single label — the listing prints the shortest such fragment
beside each value, quoted where a shell needs it. `on`, `off` and `auto` are
labels like any other. After switching, `dmm-cli` waits up to 2 s for the meter
to report the new value and prints what it now is (`Meter now in AC+DC V`,
`Meter now auto-ranging (22V)`, `Meter now HOLD on`); a refused or unconfirmed
switch exits non-zero — check the dial position, and for a range that the input
is within it. A setting with nothing to switch to prints a note and exits 0.

The UT61+/UT161 meters take no set-mode command, so a switch there is a short
burst of SELECT, Hz/% or RANGE presses, each one read back from the meter until
the target shows — slower than a single command, and audible on the meter. One
caveat comes with that: Hz and Duty % are reported with the same mode byte from
every dial position, and each `dmm-cli` run starts without history, so while the
meter shows one of them `get mode` lists only Hz and Duty %. To get back to the
position's voltage or current function, press SELECT once
(`dmm-cli command select`); the next `get mode` lists everything again. The GUI
keeps track across readings, so it only has this gap until it has seen one
other mode from the position.

The Voltcraft VC-880/VC650BT and VC-890 work the same way through their
SHIFT/SETUP button. Their dial tables come from the manuals and no meter has
confirmed them yet — see `docs/verification-backlog.md`.

**Example:**

```bash
dmm-cli set mode                   # a UT61E+ on the V⎓ dial: DC V, AC+DC V
dmm-cli set mode "AC+DC V"
dmm-cli set range 22V              # pin the range
dmm-cli set range auto
dmm-cli set hold on
dmm-cli --device ut181a set mode "V AC Hz"
```

### dmm-cli debug

Raw hex dump mode for protocol debugging. Prints transport info (bridge type and
version) on startup, then shows decoded fields alongside each parsed measurement.

```
dmm-cli debug [OPTIONS]
```

| Option | Default | Description |
|---|---|---|
| `--count <N>` | `1` | Number of requests to send. 0 = unlimited. |
| `--interval-ms <MS>` | `500` | Interval between requests in milliseconds. |

When the reading carries sub-values, they are listed on an indented
`sub-values:` line under it.

For full wire-level tracing, combine with the `RUST_LOG` environment variable:

```bash
RUST_LOG=dmm_lib=trace dmm-cli debug --count 0
```

### dmm-cli completions

Generate shell completion scripts.

```
dmm-cli completions [SHELL]
```

Supported shells: `bash`, `elvish`, `fish`, `powershell`, `zsh`.

Running without a shell argument prints install instructions.

**Install completions:**

```bash
# Bash
dmm-cli completions bash > ~/.local/share/bash-completion/completions/dmm-cli

# Zsh (ensure ~/.zfunc is in fpath and compinit is called)
dmm-cli completions zsh > ~/.zfunc/_dmm-cli

# Fish
dmm-cli completions fish > ~/.config/fish/completions/dmm-cli.fish

# PowerShell
dmm-cli completions powershell >> $PROFILE
```

### dmm-cli capture

Guided protocol capture tool for bug reports and verification. Walks you
through measuring known values in each mode and records the raw protocol data.

```
dmm-cli capture [OPTIONS]
```

| Option | Default | Description |
|---|---|---|
| `-o, --output <FILE>` | `capture-<device>.yaml` | Output file path. |
| `--steps <IDS>` | all | Only run specific steps (comma-separated, e.g. `dcmv,temp,duty`). An ID no step matches is an error. |
| `--list-steps` | | List the selected device's step IDs and exit. |

The steps come from the selected device's protocol, so `--list-steps` shows
exactly what will run for that meter — pass `--device` to see another one's.
Steps cover the measurement modes and the flag and button commands. Note that
the per-range tables are still thinly covered: each mode step captures
whatever range auto-ranging happened to pick, so ranges the meter doesn't
select on its own go unverified.

Each sample is read back for you to confirm against the meter's screen. On a
meter with secondary displays, that confirmation line lists the sub-values in
parentheses after the reading — `239.22 VAC [AUTO HV!] (Frequency 50.01 Hz,
Period 20.00 ms)` — so the whole screen is checked, not just the main value.

After the device's own steps, capture always offers **freeform captures**:
describe any mode the step list doesn't cover, and the tool records the
samples alongside your confirmation or correction of what it read. Filter to
just this pass with `--steps extra`.

**Examples:**

```bash
# Run all capture steps
dmm-cli capture

# Run only DC millivolt and temperature steps
dmm-cli capture --steps dcmv,temp

# Run only the range/auto command steps
dmm-cli capture --steps range,auto

# List the steps available for the selected device
dmm-cli capture --list-steps
```

## Environment Variables

| Variable | Description |
|---|---|
| `RUST_LOG` | Controls log verbosity. Use `dmm_lib=trace` for wire-level debugging. |
| `NO_COLOR` | Set to `1` to disable colored terminal output. |

## See Also

- [GUI reference](gui-reference.md) — real-time graphing interface
- [Setup guide](setup.md) — build prerequisites, udev rules, first-run instructions
- [Supported devices](supported-devices.md) — full compatibility list and device families
