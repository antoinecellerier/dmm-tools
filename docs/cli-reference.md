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
| `--device <DEVICE>` | `auto` | Meter model to connect to, or `auto` to work out which meter is on the cable. See [Devices](#devices) below. |
| `--adapter <SERIAL_OR_PATH>` | | Select a specific USB adapter when multiple are connected. Use serial number or HID device path from `list` output. |
| `-h, --help` | | Print help |
| `-V, --version` | | Print version |

### Devices

The `--device` flag selects the meter model. `auto` (the default) works out
which meter is on the cable from its replies ([how](detection-design.md));
naming a model skips the probe. The probe makes a UT61+/UT161 beep once. If
nothing answers, the CLI lists what each meter needs switched on.

**Device resolution precedence** (highest to lowest):

1. `--device <DEVICE>` on the command line
2. `device_family` field in `~/.config/dmm-tools/settings.json` (written by `dmm-gui` when you pick a device or Auto-detect finds one — the CLI reads it but never writes to it)
3. `auto` as a final fallback

A detected run prints one dim stderr line naming the meter and the `--device <id>` that pins it.

<!-- devices:start -->
| Value | Aliases | Description |
|---|---|---|
| `auto` |  | [Detect the meter over the USB cable](detection-design.md) (default) |
| `ut61eplus` | `ut61e+`, `ut61e` | UT61E+ (verified) |
| `ut61b+` | `ut61bplus`, `ut61b` | UT61B+ (verified) |
| `ut61d+` | `ut61dplus`, `ut61d` | UT61D+ (experimental) |
| `ut161b` |  | UT161B (experimental) |
| `ut161d` |  | UT161D (experimental) |
| `ut161e` | `ut161` | UT161E (experimental) |
| `ut8802` | `ut8802n` | UT8802 (experimental) |
| `ut8803` | `ut8803e` | UT8803 (experimental) |
| `ut803` |  | UT803 (experimental) |
| `ut804` |  | UT804 (partly verified) |
| `ut171` | `ut171a`, `ut171b`, `ut171c` | UT171A/B/C (experimental) |
| `ut181a` | `ut181` | UT181A (partly verified) |
| `vc880` | `vc-880` | Voltcraft VC-880 (experimental) |
| `vc650bt` | `vc-650bt` | Voltcraft VC650BT (experimental) |
| `vc890` | `vc-890` | Voltcraft VC-890 (experimental) |
| `mock` |  | Mock (simulated, no hardware required) |
<!-- devices:end -->

**Experimental** families were reverse-engineered from vendor software and
not yet run on real hardware; a **partly verified** one has run for its main
modes. [Supported devices](supported-devices.md) lists what each has
confirmed, along with display counts, form factor and cable. Short of
verified, the CLI prints a yellow warning with a link to the device's
verification issue on GitHub. Please report findings there.

The `mock` device generates synthetic measurements without hardware, cycling
through the scenarios listed under [Mock modes](#mock-modes); `--mock-mode`
pins one. It supports `read`, `command`, `get` and `set`.

**Examples:**

```bash
# Detect the meter on the cable
dmm-cli read

# Connect as UT8803
dmm-cli --device ut8803 read

# Connect as UT181A
dmm-cli --device ut181a info

# Use simulated device (no hardware)
dmm-cli --device mock read
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
| `--format <FORMAT>` | `text` | Output format: `text`, `csv`, `json` or `replay`. A replay file holds the meter's own frames for `--replay`, so `--scale`, `--offset`, `--unit` and `--integrate` are refused with it; writing one asks the meter its name (a UT61+ beeps once). |
| `-o, --output [<FILE>]` | stdout | Write to FILE, whose extension — `.csv`, `.json`, `.replay`, `.txt` — picks the format unless `--format` names one; a `--format` that disagrees with the extension wins, and is noted on stderr. Given with no FILE, the run names the file `measurements-<meter>-<mode>-<start>.<ext>` and prints its path when it ends. |
| `--count <N>` | `0` | Number of readings to take. 0 = unlimited (Ctrl+C to stop). |
| `--replay <FILE>` | | Play back a `--format replay` file instead of opening a meter; `--count` or Ctrl+C ends it. The file names the device. |
| `--mock-mode <MODE>` | | Pin mock device to a specific mode (only with `--device mock`). See [Mock modes](#mock-modes). |
| `--integrate` | off | Show cumulative time-integral. For current modes, this computes charge (Ah/mAh/µAh). For voltage modes, V·s. Adds `integral` and `integral_unit` columns to CSV/JSON output. |
| `--scale <FACTOR>` | `1` | Multiply the reading, taken in base units, by FACTOR. See [Scaling readings in software](#scaling-readings-in-software). |
| `--offset <VALUE>` | `0` | Add VALUE after scaling. |
| `--unit <LABEL>` | | Label the scaled reading with LABEL instead of the meter's base unit. |

CSV output begins with a `# device:` comment line identifying the meter model,
followed by the column header. JSON output begins with a `_metadata` line
containing the device model, followed by one measurement object per line.
Replay output is the meter's frames themselves, for playing a whole session
back; to document what each mode shows instead, use
[`capture`](#dmm-cli-capture).

Meters with more than one display (the UT181A's second thermocouple,
frequency and period, REL, MIN/MAX and Peak; the UT171's frequency) report
those **sub-values** indented under the reading in text output and in an `aux`
array in JSON. CSV adds one `auxN_label,auxN_value,auxN_unit` group per
sub-value the meter family can send (four for the UT181A, one for the UT171),
left empty when a reading uses fewer, so every row lines up. Single-display
meters keep the six base columns. With `--integrate`, the `integral` columns
come before the aux groups.

<!-- snippet via=ut181a-vac-hz.replay
dmm-cli read --format csv --count 1
-->
```
$ dmm-cli read --format csv --count 1
# device: UNI-T UT181A
timestamp,mode,value,unit,range,flags,aux1_label,aux1_value,aux1_unit,aux2_label,aux2_value,aux2_unit,aux3_label,aux3_value,aux3_unit,aux4_label,aux4_value,aux4_unit
2026-09-02T00:00:00+00:00,V AC Hz,239.22,VAC,600V,AUTO HV!,Frequency,50.01,Hz,Period,20.00,ms,,,,,,

--- 1 samples | Min: 239.2200 VAC | Max: 239.2200 VAC | Avg: 239.2200 VAC
```
<!-- /snippet -->

When the session ends, a summary line (sample count, min, max, average, each
with its unit) is printed to stderr. When `--integrate` is active, the total
integral is also shown.

Statistics and the integral cover a single mode and unit: if either changes
mid-run — by turning the dial, or by auto-range crossing a decade — both reset
and a note is printed to stderr, so the summary always describes one comparable
series.

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

# Let the run name the file after the meter, the mode and the start time
dmm-cli read --format csv -o

# Keep a session to replay later
dmm-cli read --format replay -o bench.replay --count 600
```

#### Scaling readings in software

`--scale`, `--offset` and `--unit` re-express the reading on the PC for
sensors the meter knows nothing about — a current clamp's mV/A, a shunt, a
probe divider, °C to °F. Nothing is sent to the meter.

The reading is converted to its base unit (V, A, Ω, …) before scaling, so a
factor survives auto-ranging between mV and V: a 10 mV/A clamp is
`--scale 100`. Then `--offset` is added and `--unit` relabels the result;
without `--unit` the reading is shown in the base unit.

The meter's own reading is kept as a `Raw` sub-value: indented in text, in
the JSON `aux` array, and in one extra CSV aux group, always the last.
Sub-values in the same unit as the reading (a second thermocouple, a REL
reference, MIN/MAX) are scaled with it; sub-values in another unit are left as
sent. Statistics and `--integrate` use the scaled reading, so a clamp
relabelled to `A` integrates to Ah. A dim stderr note marks a scaled run.

```bash
dmm-cli read --scale 100 --unit A                 # 10 mV/A clamp → amps
dmm-cli read --scale 100                          # 100:1 HV probe, stays in V
dmm-cli read --scale 1.8 --offset 32 --unit °F    # °C → °F
```

### dmm-cli get

List what the meter's settings can be switched to from the current dial
position: mode, range, HOLD, REL, MIN/MAX and Peak (UT61+/UT161, UT181A,
VC-880/VC650BT, VC-890 and mock).

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
| `--mock-mode <MODE>` | | Pin mock device to a specific mode (only with `--device mock`). See [Mock modes](#mock-modes). |

A setting with no choice from the current position (Peak on a meter without
it, a dial position with one function) is left out of the whole-meter listing;
asked for alone, it prints a note and exits 0.

<!-- snippet via=mock:acv
dmm-cli get
-->
```
$ dmm-cli get
Settings for Mock UT61E+ (AC V):
  mode    * AC V  AC V Hz
  range   * Auto  2.2V  22V  220V  1000V  (auto-ranging in 220V)
  hold    * off   on
  rel     * off   on
  minmax  * off   MAX  MIN
  peak    * off   P-MAX  P-MIN

Tip: switch one by name, e.g. dmm-cli set mode hz
```
<!-- /snippet -->

`--format json` prints one object per invocation. `get <SETTING>` gives one
block; `get` alone nests one such block per setting under `settings`.
`current` is `null` when the meter sits on none of the listed values.

<!-- snippet via=mock:acv
dmm-cli get mode --format json
-->
```
$ dmm-cli get mode --format json
{
  "device": "Mock UT61E+",
  "mode": "AC V",
  "range": "220V",
  "setting": "mode",
  "current": "AC V",
  "choices": [
    {
      "label": "AC V",
      "current": true
    },
    {
      "label": "AC V Hz",
      "current": false
    }
  ]
}
```
<!-- /snippet -->

**Example:**

```bash
dmm-cli get                        # everything switchable from where it sits
dmm-cli get range                  # the ranges the current mode offers
dmm-cli get --format json          # one object, for scripts
dmm-cli --device ut181a get mode
```

### dmm-cli set

Switch one of the meter's settings by name.

```
dmm-cli set <SETTING>            # list the values (* = live) and what to type for each
dmm-cli set <SETTING> <CHOICE>   # switch, by label
```

| Argument | Default | Description |
|---|---|---|
| `<SETTING>` | | `mode`, `range`, `hold`, `rel`, `minmax` or `peak`. |
| `<CHOICE>` | list them | Label to switch to, case-insensitive, or a unique fragment of one (`on`, `off`, `auto` included). |

| Option | Default | Description |
|---|---|---|
| `--mock-mode <MODE>` | | Pin mock device to a specific mode (only with `--device mock`). See [Mock modes](#mock-modes). |

After switching, `dmm-cli` waits for the meter to report the new value and
prints it (`Meter now in AC+DC V`). A refused or unconfirmed switch exits
non-zero: check the dial position, and for a range that the input is within it.

On the UT61+/UT161 and the Voltcraft meters a switch is a burst of button
presses (SELECT, Hz/% or RANGE; SHIFT/SETUP), each read back until the target
shows, so it is slower than a single command and audible on the meter. A
switch on a UT61+/UT161 in HOLD turns HOLD off. One
gap follows from that: while a UT61+/UT161 shows Hz or Duty %, `get mode`
lists only those two. Press Hz/% (`dmm-cli command select2`) until the
position's voltage or current function shows and the full list is back.

**Example:**

```bash
dmm-cli set mode                   # a UT61E+ on the V⎓ dial: DC V, AC+DC V
dmm-cli set mode "AC+DC V"
dmm-cli set range 22V              # pin the range
dmm-cli set range auto
dmm-cli set hold on
dmm-cli --device ut181a set mode "V AC Hz"
```

### dmm-cli command

Press one of the meter's buttons. Available commands depend on the device
family; run with no arguments to list them. To switch to a mode, range or
flag value by name instead of stepping to it with button presses, use
[`dmm-cli set`](#dmm-cli-set).

```
dmm-cli command              # list commands for the connected device
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
| `select` | Select button: steps to the dial position's next function |
| `select2` | Select2 / Hz button: steps to the dial position's next function |
| `light` | Toggle backlight |
| `peak` | Enter Peak Min/Max mode |
| `exit_peak` | Exit Peak Min/Max mode |

#### UT181A commands

| Command | Description |
|---|---|
| `hold` | Toggle Hold mode |
| `range` | Step to the next manual range for the current dial position |
| `auto` | Return to auto-range |
| `rel` | Toggle relative (REL) mode |
| `minmax` | Enable Min/Max recording |
| `exit_minmax` | Disable Min/Max recording |
| `monitor` | Enable streaming |
| `save` | Save current measurement to device memory |

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
| `select` | SHIFT/SETUP button: steps to the dial position's next function |

#### UT8803

No remote commands — the meter streams continuously after connection.

**Example:**

```bash
dmm-cli command hold
dmm-cli --device ut181a command hold
```

### dmm-cli debug

Raw hex dump mode for protocol debugging. Prints transport info (bridge type and
version) on startup, then shows decoded fields alongside each parsed measurement,
with any sub-values on an indented `sub-values:` line. It is a live dump: for
bytes worth keeping, use [`capture`](#dmm-cli-capture) per mode or
[`read --format replay`](#dmm-cli-read) for a whole session.

```
dmm-cli debug [OPTIONS]
```

| Option | Default | Description |
|---|---|---|
| `--count <N>` | `1` | Number of requests to send. 0 = unlimited. |
| `--interval-ms <MS>` | `500` | Interval between requests in milliseconds. |

For full wire-level tracing, combine with the `RUST_LOG` environment variable:

```bash
RUST_LOG=dmm_lib=trace dmm-cli debug --count 0
```

### dmm-cli capture

Guided protocol capture tool for bug reports and verification. Walks you
through measuring known values in each mode and records the raw protocol data
to a YAML report ([format](capture-design.md)). For a continuous session
rather than per-mode evidence, use [`read --format replay`](#dmm-cli-read).

```
dmm-cli capture [OPTIONS]
```

| Option | Default | Description |
|---|---|---|
| `-o, --output <FILE>` | `capture-<device>.yaml` | Output file path. |
| `--steps <IDS>` | all | Only run specific steps (comma-separated, e.g. `dcmv,temp,duty`). An ID no step matches is an error. |
| `--unverified` | | Only run the steps no hardware report has confirmed yet, plus the freeform pass. |
| `--plan <FILE>` | | Run the steps in a [plan file](#capture-plan-files) instead of the device's own list. Conflicts with `--steps`, `--unverified` and `--list-steps`. |
| `--sniff` | | Trust nothing the parser says: detect every step by raw byte changes and confirm each one by hand. |
| `--no-drive` | | Don't let the tool set ranges and flags itself after each mode step (for a receive-only cable). |
| `--settle <MS>` | `0` | Wait this long before every sample, for readings that settle slowly. Costs that much per step, so pair it with `--steps`. |
| `--list-steps` | | List the selected device's step IDs and exit. `✓` marks a step confirmed on hardware, `gate` a step that checks the decoder. |
| `--format <FORMAT>` | `text` | With `--list-steps`: `text` for the terminal, `md` for the checklist the verification issues use. |

The steps come from the selected device's protocol; `--list-steps` shows what
will run for it (pass `--device` for another).

The run opens with a numbered list of what it needs on the bench (shorted
leads, a DC source, a thermocouple). Give the numbers of anything you don't
have; those steps are skipped and stay runnable later with `--steps`.

Steps advance on the meter, not on a keypress: the tool captures once the
meter settles into the state the instruction asks for. Enter captures now,
`s` skips, `q` finishes and saves. A meter settled in something else is
reported once and the step keeps waiting. Each sample is then read back for
you to check against the screen: Enter accepts it, `n` asks what the meter
showed instead, `r` retakes the step.

The steps marked `gate` (DC V and Ω open and shorted, a negative reading)
check the decoder's digits, decimal point, OL and sign. Once all of them are
confirmed, later steps capture without stopping and are listed once at the
end for review; if any is corrected or skipped, every later step keeps asking.

On meters the tool can drive (UT61+/UT161, UT181A, VC-880/VC-890, mock),
each mode step is followed by an automatic walk through hold, REL, MIN/MAX,
Peak and every range, and the meter is left on auto range with its flags
off. `--no-drive` turns this off.

After the device's own steps, capture offers **freeform captures**: describe
any mode the list doesn't cover and the tool records the samples with your
confirmation, asked the same way. `q` on its own finishes the pass.
`--steps extra` runs just this pass.

The run ends with how many unverified steps the report covers and the issue
to attach it to.

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

# Run only the steps still lacking hardware evidence
dmm-cli --device vc890 capture --unverified

# Print the verification issue's checklist
dmm-cli --device vc890 capture --list-steps --format md

# Check every step by hand, ignoring what the decoder says
dmm-cli --device vc890 capture --sniff

# Wait three seconds before each Ω sample
dmm-cli capture --steps ohm --settle 3000

# Run a maintainer's step list from an issue
dmm-cli --device vc890 capture --plan edge.yaml
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

## Environment Variables

| Variable | Description |
|---|---|
| `RUST_LOG` | Log filter. Unset, warnings from the meter library and errors from everything else are shown. Use `dmm_lib=trace` for wire-level debugging. |
| `NO_COLOR` | Set to `1` to disable colored terminal output. |

## Appendix

### Mock modes

`--device mock` cycles through these scenarios; `--mock-mode <MODE>` on
`read`, `get` or `set` pins one:

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
| `noise` | DC mV, noisy with spikes (for graph and minimap checks) |

```bash
dmm-cli --device mock read --mock-mode dcv
```

### Capture plan files

`dmm-cli capture --plan <FILE>` runs a step list a maintainer wrote for one
investigation, typically attached to a GitHub issue, in place of the device's
own steps. Plan steps never act as gate steps, and the report goes to
`capture-<device>-<plan file stem>.yaml` by default.

| Key | Required | Meaning |
|---|---|---|
| `id` | yes | Step ID, unique in the file. `extra` is reserved. |
| `instruction` | yes | What to do on the bench. |
| `command` | | A button to press first, as `dmm-cli command` names it. |
| `samples` | | Readings to record (default 5). |
| `needs` | | Bench items the step needs: `shorted_leads`, `dc_source`, `thermocouple`, `live_wire`, `transistor`, `scr`. |
| `expect.mode` | | Mode name as the family's mode table spells it. |
| `expect.flags` | | Flags by report name (`hold`, `rel`, `auto_range`, …), each `true` or `false`. |
| `expect.range` | | `auto` or `manual`. |
| `expect.value` | | `overload`, `negative`, `finite` or `ncv`. |
| `expect.at_least` | | Magnitude a numeric reading must reach, sign aside. |

Any other key, unknown name or repeated id is an error naming the file and
step.

```yaml
steps:
  - id: dcv_open
    instruction: Set the meter to DC V with the probes open
    expect:
      mode: DC V
      value: finite
  - id: dcv_hold
    instruction: Leave it there
    command: hold
    samples: 3
    expect:
      flags:
        hold: true
  - id: dcv_release
    instruction: Press HOLD again on the meter itself
```

## See Also

- [GUI reference](gui-reference.md) — real-time graphing interface
- [Setup guide](setup.md) — build prerequisites, udev rules, first-run instructions
- [Supported devices](supported-devices.md) — full compatibility list and device families
