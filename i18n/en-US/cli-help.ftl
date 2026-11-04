### dmm-cli's --help: what each command and option does.
###
### Generated from the doc comments in crates/dmm-cli/src/cli.rs by
### `UPDATE_L10N=1 cargo test -p dmm-cli cli_help_ftl`: edit those, not this
### file. The run names each ID whose English changed. When the meaning changed
### too, delete that message from every language's cli-help.ftl, so it shows in
### English until it is translated again.
###
### IDs follow the command tree: read.about is what the read command does,
### read--interval-ms.help what its --interval-ms option does. clap wraps each
### paragraph to the terminal's width: keep a paragraph on one line. Option
### names (--device), values (auto, mock) and command lines ('dmm-cli list')
### stay as they are. clap's own words around these texts (Usage:, Options:,
### [default: …], [possible values: …]) are in shared.ftl.

# dmm-cli
dmm-cli =
    .about = CLI tool for USB and Bluetooth digital multimeters
# dmm-cli --device
dmm-cli--device =
    .help = Device to connect to [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...]. If omitted, falls back to `device_family` in ~/.config/dmm-tools/settings.json (written by dmm-gui), then to `auto`, which detects the meter over the USB cable
# dmm-cli --adapter
dmm-cli--adapter =
    .help = Select a specific adapter when more than one is reachable. Use the serial number or HID path of a USB cable, or the address of a Bluetooth adapter or meter, as 'dmm-cli list' prints them
# dmm-cli --no-bluetooth
dmm-cli--no-bluetooth =
    .help = Turn off Bluetooth scanning for this run: nothing scans for adapters or meters and 'list' shows cables only. Overrides the saved 'bluetooth' setting. An address given to --adapter is still opened
# dmm-cli --lang
dmm-cli--lang =
    .help = Language of messages and help, or `system` for the system's. If omitted, falls back to `language` in the settings file (written by dmm-gui's Language row), then to the system's
# dmm-cli list
list =
    .about = List connected USB cables, and Bluetooth adapters and meters in range
# dmm-cli info
info =
    .about = Connect and print device info
# dmm-cli read
read =
    .about = Continuously read measurements
# dmm-cli read --interval-ms
read--interval-ms =
    .help = Keep at most one reading per interval, in milliseconds (0 = every reading the meter produces)
# dmm-cli read --format
# .value-… says what that value does, in --help's list of values.
read--format =
    .help = Output format [default: text, or what -o's extension names]
    .value-replay = The meter's own frames, for --replay to play back
# dmm-cli read --output
read--output =
    .help = Output file (stdout if not specified). Given without a name, the file is named after the meter, its mode and the run's start
# dmm-cli read --count
read--count =
    .help = Number of readings (0 = unlimited: Ctrl+C stops it, or the end of a --replay file)
# dmm-cli read --no-reconnect
read--no-reconnect =
    .help = End the run when the link to the meter drops, instead of reconnecting
# dmm-cli read --integrate
read--integrate =
    .help = Show cumulative time-integral (charge for current modes, V·s for voltage)
# dmm-cli read --scale
read--scale =
    .help = Multiply the reading, taken in base units (V, A, Ω, …), by FACTOR. A 10 mV/A clamp is --scale 100; a 100:1 probe is --scale 100
# dmm-cli read --offset
read--offset =
    .help = Add VALUE after scaling (32 with --scale 1.8 turns °C into °F)
# dmm-cli read --unit
read--unit =
    .help = Label the scaled reading with this unit instead of the meter's base unit
# dmm-cli read --alarm-high
read--alarm-high =
    .help = Warn when the reading rises above VALUE, in base units (V, A, Ω, …), or in the --unit of a scaled reading
# dmm-cli read --alarm-low
read--alarm-low =
    .help = Warn when the reading falls below VALUE, as --alarm-high
# dmm-cli read --alarm-hysteresis
read--alarm-hysteresis =
    .help = How far back inside a reading must come before the same limit alarms again: a value in the limits' unit, or a percentage of the limit (`1%`) [default: a few counts of the meter's last digit]
# dmm-cli read --alarm-bell
read--alarm-bell =
    .help = Ring the terminal bell on each alarm
# dmm-cli read --mock-mode
read--mock-mode =
    .help = Pin mock device to a specific mode (only with --device mock). Without this, the settings file's mock mode applies, else the mock cycles through all modes
# dmm-cli read --replay
read--replay =
    .help = Play back a file written by --format replay instead of opening a meter; ends with the file
# dmm-cli read --import
read--import =
    .help = Read an exported CSV, JSON or replay file instead of opening a meter, without waiting: to convert it or print its summary
# dmm-cli command
command =
    .about = Send a button press command to the meter. Run with no arguments to list available commands for the selected device
# dmm-cli command <ACTION>
command--action =
    .help = Command name (run without arguments to see available commands)
# dmm-cli get
get =
    .about = List what the meter's settings can be switched to from where it sits now. Run with no argument for every setting that offers a choice
# dmm-cli get <SETTING>
get--setting =
    .help = Setting to list (omit for all of them)
# dmm-cli get --format
get--format =
    .help = Output format
# dmm-cli get --mock-mode
get--mock-mode =
    .help = Pin mock device to a specific mode (only with --device mock). Without this, the settings file's mock mode applies, else the mock cycles through all modes
# dmm-cli set
set =
    .about = Switch one of the meter's settings without touching the meter. Run without a choice to list what that setting reaches from here
# dmm-cli set <SETTING>
set--setting =
    .help = Setting to switch
# dmm-cli set <CHOICE>
set--choice =
    .help = Value label, or a unique fragment of one, from the listing (run without it to see them)
# dmm-cli set --mock-mode
set--mock-mode =
    .help = Pin mock device to a specific mode (only with --device mock). Without this, the settings file's mock mode applies, else the mock cycles through all modes
# dmm-cli debug
debug =
    .about = Raw hex dump mode for protocol debugging
# dmm-cli debug --count
debug--count =
    .help = Number of requests to send (0 = unlimited)
# dmm-cli debug --interval-ms
debug--interval-ms =
    .help = Keep at most one reading per interval, in milliseconds (0 = every reading)
# dmm-cli completions
completions =
    .about = Generate shell completions
# dmm-cli completions <SHELL>
completions--shell =
    .help = Shell to generate completions for
# dmm-cli capture
capture =
    .about = Guided protocol capture for bug reports and verification
# dmm-cli capture --output
capture--output =
    .help = Output file (default: capture-<device>.yaml). Overrides auto-naming
# dmm-cli capture --steps
capture--steps =
    .help = Only run specific steps (comma-separated IDs, e.g. "dcmv,temp,duty")
# dmm-cli capture --unverified
capture--unverified =
    .help = Only run steps not yet confirmed on real hardware, plus the freeform pass
# dmm-cli capture --plan
capture--plan =
    .help = Run the steps in a maintainer's plan file instead of the device's own list
# dmm-cli capture --sniff
capture--sniff =
    .help = Trust nothing the parser says: detect steps by raw byte changes and confirm each one
# dmm-cli capture --no-drive
capture--no-drive =
    .help = Don't let the tool set ranges and flags itself after each mode step
# dmm-cli capture --settle
capture--settle =
    .help = Wait MS before sampling any step, for readings that settle slowly
# dmm-cli capture --list-steps
capture--list-steps =
    .help = List all available step IDs and exit
# dmm-cli capture --format
capture--format =
    .help = How --list-steps prints the list (md is the issue checklist)
# dmm-cli capture --export-instructions
capture--export-instructions =
    .help = Write the step instructions to FILE, to translate for --instructions, and exit
# dmm-cli capture --instructions
capture--instructions =
    .help = Show the step instructions from FILE, translated from what --export-instructions wrote
