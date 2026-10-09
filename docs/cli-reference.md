# dmm-cli(1) — CLI Reference

<!-- Keep this file in sync with the CLI. If you add, remove, or change
     commands or options, update the relevant section here in the same commit. -->

## Name

**dmm-cli** — command-line tool for USB and Bluetooth multimeters

## Synopsis

```
dmm-cli <COMMAND> [OPTIONS]
```

## Description

Communicates with digital multimeters over USB or Bluetooth. Supports
[listing](#dmm-cli-list) and [inspecting](#dmm-cli-info) connected meters, live
[measurement reading](#dmm-cli-read), [reading](#dmm-cli-get) and
[switching](#dmm-cli-set) settings, [button commands](#dmm-cli-command),
[protocol debugging](#dmm-cli-debug), guided [data capture](#dmm-cli-capture)
for verification, and [shell completions](#dmm-cli-completions). See
[supported devices](supported-devices.md) for the full compatibility list.

Set `NO_COLOR=1` to disable colored output.

## Global Options

| Option | Default | Description |
|---|---|---|
| `--device <DEVICE>` | `auto` | Meter model to connect to, or `auto` to work out which meter is connected. See [Devices](#devices) below. |
| `--adapter <SERIAL_PATH_OR_ADDRESS>` | | Select a specific adapter when more than one is reachable. Use the serial number or HID path of a USB cable, or the address of a Bluetooth adapter or meter, from `list` output. |
| `--no-bluetooth` | | Turn off Bluetooth scanning for this run. Overrides the `bluetooth` setting in the settings file; an address given to `--adapter` is still opened. |
| `-h, --help` | | Print help |
| `-V, --version` | | Print version |

### Devices

The `--device` flag selects the meter model. `auto` (the default) works out
which meter is connected from its replies ([how](detection-design.md));
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
| `auto` |  | [Detect the connected meter](detection-design.md) (default) |
| **Brymen** |  |  |
| `bm78xbt` | `bm788bt`, `bm787bt`, `brymen-bm788bt`, `brymen-bm787bt` | BM788BT/BM787BT (experimental) |
| `bm86x` | `bm869s`, `bm867s`, `brymen-bm869s`, `brymen-bm867s` | BM869s/BM867s (experimental) |
| `bm82x` | `bm829s`, `bm827s`, `bm822s`, `bm821s`, `brymen-bm829s`, `brymen-bm827s`, `brymen-bm822s`, `brymen-bm821s` | BM829s/BM827s/BM822s/BM821s (experimental) |
| `bm52x` | `bm525s`, `bm521s`, `brymen-bm525s`, `brymen-bm521s` | BM525s/BM521s (experimental) |
| **EEVblog** |  |  |
| `121gw` | `eevblog121gw`, `eevblog-121gw` | 121GW (experimental) |
| **OWON** |  |  |
| `ow18b` | `ow16b`, `owon-ow18b`, `owon-ow16b` | OW18B/OW16B (experimental) |
| `ow18e` | `owon-ow18e` | OW18E (experimental) |
| `b33` | `b33t`, `b33+`, `b33t+`, `owon-b33` | B33 (experimental) |
| `b35t+` | `b35+`, `owon-b35t+` | B35T+ (experimental) |
| `b41t+` | `b41t`, `owon-b41t+` | B41T+ (experimental) |
| `cm2100b` | `owon-cm2100b` | CM2100B (experimental) |
| `cms101` | `owon-cms101` | CMS101 (experimental) |
| `cms061` | `owon-cms061` | CMS061 (experimental) |
| `ow65b` | `owon-ow65b` | OW65B (experimental) |
| `ow67b` | `owon-ow67b` | OW67B (experimental) |
| `ow69b` | `owon-ow69b` | OW69B (experimental) |
| **UNI-T** |  |  |
| `ut61eplus` | `ut61e+`, `ut61e` | UT61E+ (verified) |
| `ut61b+` | `ut61bplus`, `ut61b` | UT61B+ (verified) |
| `ut61d+` | `ut61dplus`, `ut61d` | UT61D+ (experimental) |
| `ut161b` |  | UT161B (experimental) |
| `ut161d` |  | UT161D (experimental) |
| `ut161e` | `ut161` | UT161E (experimental) |
| `ut60bt` |  | UT60BT (experimental) |
| `ut202bt` |  | UT202BT (experimental) |
| `ut8802` | `ut8802n` | UT8802 (experimental) |
| `ut8803` | `ut8803e` | UT8803 (experimental) |
| `ut803` |  | UT803 (experimental) |
| `ut804` |  | UT804 (verified) |
| `ut71ab` | `ut71a`, `ut71b` | UT71A/B (experimental) |
| `ut71cde` | `ut71c`, `ut71d`, `ut71e` | UT71C/D/E (experimental) |
| `ut171` | `ut171a`, `ut171b`, `ut171c` | UT171A/B/C (experimental) |
| `ut181a` | `ut181` | UT181A (partly verified) |
| **Voltcraft** |  |  |
| `vc880` | `vc-880` | VC-880 (experimental) |
| `vc650bt` | `vc-650bt` | VC650BT (experimental) |
| `vc890` | `vc-890` | VC-890 (experimental) |
| `vc920` | `vc-920`, `vc940`, `vc-940`, `vc960`, `vc-960` | VC920/VC940/VC960 (experimental) |
| `vc871` | `vc-871` | VC871 (experimental) |
| `vc891` | `vc-891` | VC891 (experimental) |
| `vc915` | `vc-915` | VC915 (experimental) |
| `vc925pv` | `vc-925pv` | VC925 PV (experimental) |
| **ZOTEK / ZOYI / BSIDE / ANENG** |  |  |
| `zt300ab` | `zt-300ab`, `an9002`, `an-9002` | ZT-300AB / AN9002 (experimental) |
| `zt5566se` | `zt-5566se`, `zt5566s`, `zt-5566s`, `an999s`, `an-999s` | ZT-5566SE / AN999S (experimental) |
| `zt5bq` | `zt-5bq`, `st207` | ZT-5BQ / ST207 (experimental) |
| `zt5b` | `zt-5b`, `v05b` | ZT-5B / V05B (verified) |
| **Simulated** |  |  |
| `mock` |  | Mock (simulated, no hardware required) |
| `mock-zt5b` |  | Mock ZT-5B / V05B (simulated, no hardware required) |
<!-- devices:end -->

**Experimental** families were reverse-engineered from vendor software and
not yet run on real hardware; a **partly verified** one has run for its main
modes. [Supported devices](supported-devices.md) lists what each has
confirmed, along with display counts, form factor and cable. Short of
verified, the CLI prints a yellow warning with a link to the device's
verification issue on GitHub. Please report findings there.

The `mock` device generates synthetic measurements without hardware, cycling
through the scenarios listed under [Mock modes](#mock-modes); `--mock-mode`
pins one, as does `dmm-gui`'s Mock mode setting. It supports `read`, `command`, `get` and `set`.

The `mock-zt5b` device simulates a ZT-5B / V05B, to try the ZOTEK remote keys
without a meter. It supports `read` and `command`, sends about 2.6 readings a
second, and starts on the `Auto` word each run; [ZOTEK mock](#zotek-mock)
lists what its keys do.

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

List the connected USB cables, and the Bluetooth adapters and meters in range:
the ones connected to this computer or heard advertising. A paired adapter the
scan did not hear is listed dim as "paired but not heard": check it is switched
on.

```
dmm-cli list
```

Prints each device with an index number and transport type. The Bluetooth
scan takes a few seconds; `--no-bluetooth` skips it and lists cables only. If
nothing is found, prints troubleshooting steps for each link that was
searched (udev rule install on Linux, driver install on Windows).

When more than one device is reachable, use `--adapter` with a serial number,
HID path or Bluetooth address from the `list` output to select one:

```
dmm-cli list
# [0] /dev/hidraw3 [CP2110] — CP2110 HID UART Bridge (S/N: 00C5B27A)
# [1] 12:34:56:78:9A:BC [Bluetooth] — UT-D07B

dmm-cli --adapter 12:34:56:78:9A:BC read
```

### dmm-cli info

Connect to the meter and print device info: model name, transport type, and
transport-specific diagnostics (CP2110 firmware version and UART error flags
over USB; MTU, adapter heartbeats and an OWON meter's device information over
Bluetooth).

```
$ dmm-cli --adapter 12:34:56:78:9A:BC info
Device: UT61E+
Transport: Bluetooth
  UT-D07B (12:34:56:78:9A:BC)
  Status: MTU: 247 bytes, adapter heartbeats: 1
```

### dmm-cli read

Continuously read measurements from the meter.

```
dmm-cli read [OPTIONS]
```

| Option | Default | Description |
|---|---|---|
| `--interval-ms <MS>` | `0` | At most one reading per interval, in milliseconds: the one nearest each tick, at the time the meter sent it. 0 = every reading the meter produces, at its own pace (on a UT61E+, about 10 a second over USB, 3 over Bluetooth). |
| `--format <FORMAT>` | `text`, or what `-o`'s extension names | Output format: `text`, `csv`, `json` or `replay`. |
| `-o, --output [<FILE>]` | stdout | Write to FILE; with no FILE, to `measurements-<meter>-<mode>-<start>.<ext>`. |
| `--count <N>` | `0` | Number of readings to take. 0 = unlimited: Ctrl+C stops it, or the end of a `--replay` file. |
| `--replay <FILE>` | | Play back a `--format replay` file at its recorded pace instead of opening a meter; the run ends with the file, or sooner with `--count` or Ctrl+C. |
| `--import <FILE>` | | Read an exported CSV, JSON or replay file instead of opening a meter, without waiting: to convert it (`--format`, `-o`) or print its summary. Its markers come through; a CSV or JSON file refuses `--format replay` and `--interval-ms`. |
| `--mock-mode <MODE>` | `mock_mode` in `settings.json` | Pin mock device to a specific mode (only with `--device mock`). See [Mock modes](#mock-modes). |
| `--integrate` | off | Show cumulative time-integral. For current modes, this computes charge (Ah/mAh/µAh). For voltage modes, V·s. Adds `integral` and `integral_unit` columns to CSV/JSON output. |
| `--scale <FACTOR>` | `1` | Multiply the reading, taken in base units, by FACTOR. See [Scaling readings in software](#scaling-readings-in-software). |
| `--offset <VALUE>` | `0` | Add VALUE after scaling. |
| `--unit <LABEL>` | | Label the scaled reading with LABEL instead of the meter's base unit. |
| `--alarm-high <VALUE>` | | Alarm when the reading rises above VALUE. See [Alarms](#alarms). |
| `--alarm-low <VALUE>` | | Alarm when the reading falls below VALUE. |
| `--alarm-hysteresis <BAND>` | a few counts of the meter's last digit | How far back inside the reading must come before the same limit alarms again: a value in the limits' unit, or a percentage of the limit (`1%`). |
| `--alarm-bell` | off | Ring the terminal bell with each alarm. |

CSV output begins with a `# device:` comment line identifying the meter model,
followed by the column header. JSON output begins with a `_metadata` line
containing the device model, followed by one measurement object per line.
Replay output is the meter's frames themselves, for playing a whole session
back; to document what each mode shows instead, use
[`capture`](#dmm-cli-capture).

`--format replay` needs a real meter and refuses `--scale`, `--offset`,
`--unit` and `--integrate`; pass those when playing the file back. Writing one
asks the meter its name unless detection already has it (a UT61+/UT161 beeps
once). `--replay` takes the meter
and its link from the file and refuses `--device` and `--mock-mode`. A
recording's markers (a GUI export keeps them) come through in the CSV and
JSON marker fields and a replay copy, each on the first reading played at or
after it.

Meters with more than one display (the UT181A's thermocouples, frequency and
period, AC and DC parts, dB's voltage, REL, MIN/MAX and Peak; the UT171's
frequency) report those **sub-values** indented under the reading in text
output and in an `aux` array in JSON. CSV adds one
`auxN_label,auxN_value,auxN_unit` group per sub-value the meter family can
send (four for the UT181A, one for the UT171 and the UT61E+), left empty when
a reading uses fewer, so every row lines up.
Single-display meters keep the six base columns. With `--integrate`, the
`integral` columns come before the aux groups.

<!-- snippet via=ut181a-vac-hz.replay
dmm-cli read --format csv --count 1
-->
```
$ dmm-cli read --format csv --count 1
# device: UNI-T UT181A
timestamp,mode,value,unit,range,flags,aux1_label,aux1_value,aux1_unit,aux2_label,aux2_value,aux2_unit,aux3_label,aux3_value,aux3_unit,aux4_label,aux4_value,aux4_unit
2026-09-02T00:00:00+00:00,V AC Hz,239.22,V,600V,AUTO HV!,Frequency,50.01,Hz,Period,20.00,ms,,,,,,

--- 1 samples | Min: 239.2200 V | Max: 239.2200 V | Avg: 239.2200 V
```
<!-- /snippet -->

A UT61E+ in AC+DC V sends its DC and AC components in turn, each in a frame of
its own. A DC frame is the reading, printed as `DC 1.6112 V`; an AC frame
carries only the `AC` sub-value, printed where the reading goes, with an empty CSV value cell and a
`null` JSON value. `--count` counts frames, while the summary covers the DC
readings.

<!-- snippet via=acdcv-cell.replay
dmm-cli read --count 6
-->
```
$ dmm-cli read --count 6
DC 1.6112 V [AUTO]
AC 0.0000 V [AUTO]
DC 0.2045 V [AUTO]
AC 0.4406 V [AUTO]
DC 0.0264 V [AUTO]
AC 0.3376 V [AUTO]

--- 3 samples | Min: 0.0264 V | Max: 1.6112 V | Avg: 0.6140 V
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

#### Alarms

`--alarm-high` and `--alarm-low` watch the reading through an unattended run.
A reading that goes past a limit prints an `Alarm:` line on stderr and is
marked, with the limit in its note, in the CSV and JSON marker fields and a
replay copy. The summary counts the alarms. The same limit alarms again only
once the reading has come back inside by `--alarm-hysteresis`, so a reading
hovering at a limit raises one alarm; overloads don't count.

Limits are in the base unit (V, A, Ω, …), as `--scale`'s factor is, or in
`--unit` for a scaled reading. They watch the quantity of the first reading:
after a dial turn to another quantity the alarm is idle until the dial comes
back, and a stderr note says so.

```bash
dmm-cli read --alarm-low 3.0 --format csv -o discharge.csv   # cell run down
dmm-cli read --alarm-high 5.25 --alarm-bell                  # 5 V rail out of tolerance
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
| `--mock-mode <MODE>` | `mock_mode` in `settings.json` | Pin mock device to a specific mode (only with `--device mock`). See [Mock modes](#mock-modes). |

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
| `--mock-mode <MODE>` | `mock_mode` in `settings.json` | Pin mock device to a specific mode (only with `--device mock`). See [Mock modes](#mock-modes). |

After switching, `dmm-cli` waits for the meter to report the new value and
prints it (`Meter now in AC+DC V`). A refused or unconfirmed switch exits
non-zero: check the dial position, and for a range that the input is within it.

On the UT61+/UT161 and the VC-880, VC650BT and VC-890 a switch is a burst
of button presses (SELECT, Hz/% or RANGE; SHIFT/SETUP), each read back until the target
shows, so it is slower than a single command and audible on the meter. A
switch on a UT61+/UT161 in HOLD turns HOLD off.

While a UT61+/UT161 shows Hz or Duty %, `get mode` lists only those two.
Press Hz/% (`dmm-cli command select2`) until the position's voltage or current
function shows and the full list is back.

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
dmm-cli command                   # list commands for the connected device
dmm-cli --device ut181a command   # list commands for UT181A
dmm-cli command <ACTION>          # send a command
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

#### ZOTEK commands

The keys of ZOTEK's app; a ZT-5B has run all of its keys, the other models'
are not yet tried on a meter, so watch the reading for what a key did. The ZT-300AB offers no `hold`, `auto_function`, `capacitance`, `hz` or
`ncv`; the ZT-5B no `volts`, `millivolts`, `ohms` or `current`; only the
ZT-5566SE offers `minmax`.

| Command | Description |
|---|---|
| `hold` | HOLD key |
| `minmax` | MAX/MIN key |
| `auto_function` | AUTO key: the meter picks the function |
| `volts` | V key |
| `millivolts` | mV key |
| `ohms` | Ω key |
| `capacitance` | Capacitance key |
| `hz` | Hz key |
| `diode_continuity` | Diode/continuity key |
| `ncv` | NCV key |
| `current` | Current key |
| `temp_unit` | °C/°F key |
| `zero` | ZERO key; capacitance only |

#### EEVblog 121GW commands

The keys of EEVblog's and UEi's apps, not yet tried on a meter; watch the reading for
what a key did.

| Command | Description |
|---|---|
| `range` | RANGE button |
| `hold` | HOLD button: HOLD, then A-HOLD |
| `rel` | REL button |
| `select` | MODE button: steps to the dial position's next function |
| `minmax` | MIN/MAX button: cycles MIN, MAX and AVG |
| `exit_minmax` | Long MIN/MAX: leaves MIN/MAX |
| `peak` | 1ms PEAK button; AC V only |
| `light` | Long MODE: toggle backlight |
| `lpf` | Long REL: 1 kHz low-pass filter; AC modes only |

#### OWON and Voltcraft VC871 / VC891 / VC915 / VC925 PV commands

The keys of OWON's and Voltcraft's apps and OWON's PC software, not yet tried
on a meter; watch the reading for what a key did. Each model offers only its
own keys.

| Command | Description |
|---|---|
| `select` | Select key: steps to the dial position's next function |
| `range` | Range key: steps to the next manual range |
| `auto` | Long Range: returns to auto-range |
| `hold` | Hold key |
| `light` | Long Hold: backlight, and the OW18B's and OW18E's flashlight |
| `rel` | △ or REL key; on the OW16B, OW18B and OW18E, the same press as `hz_duty` |
| `exit_rel` | Long REL: leaves REL |
| `hz_duty` | Hz/Duty key: steps through frequency and duty; AC V, AC A, Hz and duty |
| `minmax` | Max/Min key: cycles MAX and MIN |
| `exit_minmax` | Long Max/Min: leaves Max/Min |
| `zero` | ZERO key: zeroes DC A, relative in capacitance and voltage |
| `peak` | Peak key |
| `lpf` | LPF key: low-pass filter |
| `inrush` | Inrush key; AC A |
| `current_loop` | 4~20mA key: a 4-20 mA loop as a percentage |
| `display` | Display key: changes what the main and second displays show |
| `compare` | Compare key: limit test |
| `ac_dc` | AC/DC key |
| `motor` | Motor key: motor rotation |

#### UT8802 / UT8803 / UT803 / UT804 / UT71 / VC920 / VC940 / VC960 / Brymen

No remote commands: these meters only send readings
([supported devices](supported-devices.md)).

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
| `--interval-ms <MS>` | `500` | At most one reading per interval, in milliseconds. 0 = every reading. |

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
| `--steps <IDS>` | all | Only run specific steps (comma-separated, e.g. `dcmv,temp,duty`); `extra` is the freeform pass, `detect` the auto-detection check. An ID no step matches is an error. |
| `--unverified` | | Only run the steps no hardware report has confirmed yet, plus the freeform pass; the auto-detection check is left out on a link where it is already confirmed. |
| `--plan <FILE>` | | Run the steps in a [plan file](#capture-plan-files) instead of the device's own list. Conflicts with `--steps`, `--unverified` and `--list-steps`. |
| `--sniff` | | Trust nothing the parser says: detect every step by raw byte changes and confirm each one by hand. |
| `--no-drive` | | Don't let the tool set ranges and flags itself after each mode step. |
| `--settle <MS>` | `0` | Wait this long before every sample, for readings that settle slowly. Costs that much per step, so pair it with `--steps`. |
| `--list-steps` | | List the selected device's step IDs and exit. `✓` marks a step confirmed on hardware, `gate` a step that checks the decoder. |
| `--format <FORMAT>` | `text` | With `--list-steps`: `text` for the terminal, `md` for the checklist the verification issues use. |

The steps come from the selected device's protocol; `--list-steps` shows what
will run for it (pass `--device` for another).

The run opens with a numbered list of what it needs on the bench (shorted
leads, a DC source, a thermocouple). Give the numbers of anything you don't
have; those steps are skipped and stay runnable later with `--steps`.

Steps advance on the meter, not on a keypress: the tool captures once the
meter settles into the state the instruction asks for; a step the readings
cannot tell from the one before waits for Enter. Enter captures now, `s`
skips, `q` finishes and saves. A meter settled in something else is
reported once and the step keeps waiting. Leave the meter in the step's mode
until the samples are printed: leaving it retakes the step. Each sample is
then read back for you to check against the screen: Enter accepts it, `n`
asks what the meter showed instead, `r` retakes the step.

The steps marked `gate` (DC V and Ω open and shorted, a negative reading)
check the decoder's digits, decimal point, OL and sign. Once all of them are
confirmed, later steps capture without stopping and are listed once at the
end for review; if any is corrected or skipped, every later step keeps asking.

On meters the tool can drive (UT61+/UT161, UT181A, VC-880/VC-890, mock),
each mode step is followed by an automatic walk through hold, REL, MIN/MAX,
Peak and every range, and the meter is left on auto range (or its starting
range, where the mode has no auto, as a UT181A's Peak) with its flags off.
`--no-drive` turns this off.

After the device's own steps, capture offers **freeform captures**: describe
any mode the list doesn't cover and the tool records the samples with your
confirmation, asked the same way. `q` on its own finishes the pass.
`--steps extra` runs just this pass.

Last, the run checks that auto-detection finds the meter: it asks you to
restart the meter, unplugging its USB cable if it has one, then detects it as
`--device auto` would and reads it at Ω. The result goes in the report. A
`--plan` run skips the check.

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

For the GUI, [`dmm-gui --completions <SHELL>`](gui-reference.md#command-line-options) prints its own script.

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
`read`, `get` or `set` pins one. Without the flag, the `mock_mode` field
`dmm-gui`'s Mock mode setting writes to `settings.json` pins it:

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

### ZOTEK mock

What each key does on `--device mock-zt5b`. The keys do what ZOTEK's app
intends; a ZT-5B has confirmed every one.

| Key | The simulated meter shows |
|---|---|
| `auto_function` | The `Auto` word, then a 9 V battery (DC V), a 4.7 kΩ resistor, open (OL) now and then, and the mains (AC V, `[HV!]`), the word between each; where it starts |
| `capacitance` | The open leads' stray capacitance, then a 100 nF capacitor |
| `zero` | In capacitance, the reading taken as zero |
| `hz` | The mains frequency |
| `diode_continuity` | A diode, reversed (OL) now and then; pressed again, swaps diode and continuity |
| `ncv` | `EF`, then one to four dashes as a live wire nears, and back |
| `temp_unit` | °C; pressed again, °F |
| `hold` | The display frozen until pressed again or a function key |

```bash
dmm-cli --device mock-zt5b read
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
| `needs` | | Bench items the step needs: `shorted_leads`, `dc_source`, `thermocouple`, `live_wire`, `power_adapter`, `transistor`, `scr`. |
| `expect.mode` | | Mode name as the family's mode table spells it. |
| `expect.flags` | | Flags by report name (`hold`, `rel`, `auto_range`, …), each `true` or `false`. |
| `expect.range` | | `auto` or `manual`. |
| `expect.value` | | `overload`, `negative`, `finite`, `ncv` or `zero`. |
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
