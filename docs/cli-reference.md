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
measurement reading, button commands, protocol debugging, and guided data
capture for verification. See [supported devices](supported-devices.md) for
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
| `ut161e` | `ut161` | UT161E (same protocol as UT61E+) |
| `ut8802` | `ut8802n` | UT8802 / UT8802N bench multimeter (experimental) |
| `ut8803` | `ut8803e` | UT8803 / UT8803E bench multimeter (experimental) |
| `ut803` | | UT803 bench multimeter, 6000 counts (experimental) |
| `ut804` | | UT804 bench multimeter, 4000 counts (experimental) |
| `ut171` | `ut171a`, `ut171b`, `ut171c` | UT171A/B/C (experimental) |
| `ut181a` | `ut181` | UT181A (experimental) |
| `vc880` | `vc-880` | Voltcraft VC-880 handheld DMM (experimental) |
| `vc650bt` | `vc-650bt` | Voltcraft VC650BT bench DMM (experimental, same protocol as VC-880) |
| `vc890` | `vc-890` | Voltcraft VC-890 handheld DMM, 60K counts, OLED (experimental) |
| `mock` | | Simulated device (no hardware required) |

Non-UT61E+ families are marked **experimental** -- their protocols were reverse-engineered
from vendor software and have not been verified against real hardware. When connecting to
an experimental device, the CLI prints a yellow warning with a link to the device's
verification issue on GitHub. Please report findings there.

The `mock` device generates synthetic measurements cycling through multiple modes
(DC V, AC V, Ohms, Capacitance, Hz, Temperature, DC mA, Overload, NCV). It requires
no USB hardware and is useful for development, demos, and testing output formats.
Supports `read` and `command` subcommands. The `info`, `debug`, and `capture`
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
devices are found, prints troubleshooting hints (udev rules and `plugdev`
group membership on Linux, driver install on Windows).

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

CSV output begins with a `# device:` comment line identifying the meter model,
followed by the column header. JSON output begins with a `_metadata` line
containing the device model, followed by one measurement object per line.

When the session ends, a summary line (sample count, min, max, average) is
printed to stderr. When `--integrate` is active, the total integral is also shown.

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

#### UT181A commands

| Command | Description |
|---|---|
| `hold` | Toggle Hold mode |
| `range` | Set manual range 1 |
| `auto` | Return to auto-range |
| `minmax` | Enable Min/Max recording |
| `exit_minmax` | Disable Min/Max recording |
| `monitor` | Enable streaming (SET_MONITOR) |
| `save` | Save current measurement to device memory |

#### UT171 commands

| Command | Description |
|---|---|
| `connect` | Start measurement streaming |
| `pause` | Stop measurement streaming |

#### UT8803

No remote commands — the meter streams continuously after connection.

**Example:**

```bash
dmm-cli command hold
dmm-cli --device ut181a command hold
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
| `--steps <IDS>` | all | Only run specific steps (comma-separated, e.g. `dcmv,temp,duty`). |
| `--list-steps` | | List all available step IDs and exit. |

**Examples:**

```bash
# Run all capture steps
dmm-cli capture

# Run only DC millivolt and temperature steps
dmm-cli capture --steps dcmv,temp

# List available steps
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
