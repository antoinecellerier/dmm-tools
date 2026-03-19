# ut61eplus(1) — CLI Reference

<!-- Keep this file in sync with the CLI. If you add, remove, or change
     commands or options, update the relevant section here in the same commit. -->

## Name

**ut61eplus** — command-line tool for the UNI-T UT61E+ multimeter

## Synopsis

```
ut61eplus <COMMAND> [OPTIONS]
```

## Description

Communicates with the UNI-T UT61E+ multimeter over USB via its CP2110 HID
bridge. Supports live measurement reading, button commands, protocol debugging,
and guided data capture for verification.

Set `NO_COLOR=1` to disable colored output.

## Global Options

| Option | Default | Description |
|---|---|---|
| `--device <FAMILY>` | `ut61eplus` | Device family to connect to. See [Device Families](#device-families) below. |
| `-h, --help` | | Print help |
| `-V, --version` | | Print version |

### Device Families

The `--device` flag selects which protocol to use. Accepted values and aliases:

| Value | Aliases | Description |
|---|---|---|
| `ut61eplus` | `ut61e+`, `ut61e`, `ut61b+`, `ut61bplus`, `ut61d+`, `ut61dplus`, `ut161b`, `ut161d`, `ut161e`, `ut161` | UT61E+, UT61B+, UT61D+, UT161 series (default, verified) |
| `ut8803` | `ut8803e` | UT8803 / UT8803E bench multimeter (experimental) |
| `ut171` | `ut171a`, `ut171b`, `ut171c` | UT171A/B/C (experimental) |
| `ut181a` | `ut181` | UT181A (experimental) |

Non-UT61E+ families are marked **experimental** -- their protocols were reverse-engineered
from vendor software and have not been verified against real hardware. When connecting to
an experimental device, the CLI prints a yellow warning. Please report findings at
https://github.com/antoinecellerier/dmm-tools.

**Examples:**

```bash
# Default (UT61E+ family)
ut61eplus read

# Connect as UT8803
ut61eplus --device ut8803 read

# Connect as UT181A
ut61eplus --device ut181a info
```

## Commands

### ut61eplus list

List connected UT61E+ devices.

```
ut61eplus list
```

Prints each detected device with an index number. If no devices are found,
prints troubleshooting hints (udev rules on Linux, driver install on Windows).

### ut61eplus info

Connect to the meter and print device info: model name, CP2110 bridge firmware
version, and any UART error flags.

```
ut61eplus info
```

### ut61eplus read

Continuously read measurements from the meter.

```
ut61eplus read [OPTIONS]
```

| Option | Default | Description |
|---|---|---|
| `--interval-ms <MS>` | `0` | Interval between readings in milliseconds. 0 = fastest (~10 Hz). |
| `--format <FORMAT>` | `text` | Output format: `text`, `csv`, or `json`. |
| `-o, --output <FILE>` | stdout | Write output to a file instead of stdout. |
| `--count <N>` | `0` | Number of readings to take. 0 = unlimited (Ctrl+C to stop). |

When the session ends, a summary line (sample count, min, max, average) is
printed to stderr.

**Examples:**

```bash
# Stream readings to the terminal
ut61eplus read

# Record 100 CSV samples to a file
ut61eplus read --format csv --count 100 -o measurements.csv

# JSON output at 1-second intervals
ut61eplus read --format json --interval-ms 1000
```

### ut61eplus command

Send a button-press command to the meter.

```
ut61eplus command <ACTION>
```

| Action | Description |
|---|---|
| `hold` | Toggle Hold mode |
| `min-max` | Enter Min/Max recording |
| `exit-min-max` | Exit Min/Max recording |
| `rel` | Toggle Relative mode |
| `range` | Cycle manual range |
| `auto` | Return to auto-range |
| `select` | Select button (mode-dependent) |
| `select2` | Select2 button (mode-dependent) |
| `light` | Toggle backlight |
| `peak-min-max` | Enter Peak Min/Max mode |
| `exit-peak` | Exit Peak Min/Max mode |

**Example:**

```bash
ut61eplus command hold
```

### ut61eplus debug

Raw hex dump mode for protocol debugging. Prints CP2110 bridge version and UART
status on startup, then shows decoded fields alongside each parsed measurement.

```
ut61eplus debug [OPTIONS]
```

| Option | Default | Description |
|---|---|---|
| `--count <N>` | `1` | Number of requests to send. 0 = unlimited. |
| `--interval-ms <MS>` | `500` | Interval between requests in milliseconds. |

For full wire-level tracing, combine with the `RUST_LOG` environment variable:

```bash
RUST_LOG=ut61eplus_lib=trace ut61eplus debug --count 0
```

### ut61eplus completions

Generate shell completion scripts.

```
ut61eplus completions [SHELL]
```

Supported shells: `bash`, `elvish`, `fish`, `powershell`, `zsh`.

Running without a shell argument prints install instructions.

**Install completions:**

```bash
# Bash
ut61eplus completions bash > ~/.local/share/bash-completion/completions/ut61eplus

# Zsh (ensure ~/.zfunc is in fpath and compinit is called)
ut61eplus completions zsh > ~/.zfunc/_ut61eplus

# Fish
ut61eplus completions fish > ~/.config/fish/completions/ut61eplus.fish

# PowerShell
ut61eplus completions powershell >> $PROFILE
```

### ut61eplus capture

Guided protocol capture tool for bug reports and verification. Walks you
through measuring known values in each mode and records the raw protocol data.

```
ut61eplus capture [OPTIONS]
```

| Option | Default | Description |
|---|---|---|
| `-o, --output <FILE>` | `capture-<device>.yaml` | Output file path. |
| `--steps <IDS>` | all | Only run specific steps (comma-separated, e.g. `dcmv,temp,duty`). |
| `--list-steps` | | List all available step IDs and exit. |

**Examples:**

```bash
# Run all capture steps
ut61eplus capture

# Run only DC millivolt and temperature steps
ut61eplus capture --steps dcmv,temp

# List available steps
ut61eplus capture --list-steps
```

## Environment Variables

| Variable | Description |
|---|---|
| `RUST_LOG` | Controls log verbosity. Use `ut61eplus_lib=trace` for wire-level debugging. |
| `NO_COLOR` | Set to `1` to disable colored terminal output. |

## See Also

- [Setup guide](setup.md) — build prerequisites, udev rules, first-run instructions
- [Protocol reference](protocol.md) — CP2110 transport, message formats, byte-level details
- [Architecture](architecture.md) — crate layout and design decisions
