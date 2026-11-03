### Text both dmm-cli and dmm-gui show: the shared parts of their --help
### and the words clap puts around it, their command-line errors, the
### setup help when no meter is found, and the notices both print.
###
### Command lines, file paths and web addresses are never part of a message.
### Words in --flag form, and 'dmm-cli list', are typed by the user and stay
### as they are.

## The project page link: the last line of both binaries' --help, and the
## link in the GUI's top bar.

project-link = Help & Feedback
# $url is the project page's address.
project-link-line = { project-link }: { $url }

## The --device flag's long help, after the binary's own first line. The
## device list between the heading and the aliases is generated.

device-list-heading = Devices:
# What the "auto" device does. Also its hover in the GUI's device list.
device-auto = Detect the connected meter
# The "auto" row of the list: what the flag does when it isn't given.
device-auto-default = { device-auto } (default)
device-aliases =
    Also accepts aliases: ut61e+, ut61b, ut171a, ut181, etc.
    Quote names with special characters: --device 'ut61e+'
# Beside the simulated meters, in device lists.
device-no-hardware = no hardware required

## How far a meter's support is verified, beside it in device lists.

stability-verified = verified
stability-partly-verified = partly verified
stability-experimental = experimental
# Both binaries show it for a meter short of verified, each followed by its
# own way to help verify it.
# $model is the meter's name, e.g. "UNI-T UT8803".
# $stability is verified, partly-verified or experimental.
stability-warning =
    { $stability ->
        [verified] { $model } support is verified (verified against real hardware).
        [partly-verified] { $model } support is partly verified (some modes and commands unconfirmed on real hardware).
       *[experimental] { $model } support is experimental (unverified against real hardware).
    }

## The --mock-mode flag's long help, after the binary's own first line.

# $modes lists the mock's modes, separated by commas.
mock-mode-list = Modes: { $modes }
# $example is a command line.
mock-mode-example = Example: { $example }
mock-clock-mock-only = --mock-clock-scale and --mock-clock-preseed only apply to the mock device and to a replay. Re-run with --device mock or --replay, or without the clock flags.

## A link's short name, where the meter is already named: the GUI's
## status line, the setup help's section labels.

link-usb-cable = USB cable
link-bluetooth = Bluetooth

## The help shown when no meter was found: a title, then one section per
## link searched — a label line, then steps. The CLI prints them in a
## terminal, the GUI in a notice.

# No full stop: the CLI adds one, the GUI's notice titles carry none.
not-found-usb = No USB cable found
not-found-usb-or-bluetooth = No meter found over USB or Bluetooth
# $address is a Bluetooth address or peripheral id.
not-found-bluetooth-at = No Bluetooth device found at { $address }
# $model is the meter's name, e.g. "UNI-T UT60BT".
not-found-bluetooth-only = No { $model } found over Bluetooth
# A section's label line.
# $link is link-usb-cable or link-bluetooth.
# $check is setup-usb-check or setup-bluetooth-check.
setup-label = { $link }: { $check }
setup-usb-check = check it is plugged in and the meter is powered on.
setup-bluetooth-check = turn on Bluetooth on this computer and data transmission on the meter.
# Linux. Two command lines follow.
setup-usb-linux-rule = Ensure the udev rule is installed:
# Linux. The setup guide's address follows.
setup-usb-linux-replug = Then replug the cable. On a headless machine, keep a group on the rule:
# Windows. The quoted names are what Device Manager shows; use the names
# Windows shows in your language.
setup-usb-windows-open = Open Device Manager with the cable plugged in:
setup-usb-windows-cp2110 = - 'CP2110 USB to UART Bridge' under HID devices: no action needed.
setup-usb-windows-input = - 'USB Input Device' under HID devices: no action needed.
# The driver's download address follows.
setup-usb-windows-warning = - Yellow warning icon under 'Other devices': install the driver from
setup-usb-windows-nothing = - Nothing appears: try a different USB port.
# macOS. Name the settings as macOS shows them in your language.
setup-usb-macos-auto = The cable should be recognized automatically (no driver needed).
setup-usb-macos-privacy = If the device is not found, check System Settings > Privacy & Security > Input Monitoring.
setup-bluetooth-adapter = If both are on, switch the adapter off and on.
setup-bluetooth-macos-privacy = Allow Bluetooth in System Settings > Privacy & Security.
# The device catalog's address follows.
setup-bluetooth-catalog = Supported adapters and meters are listed in the device catalog:
setup-bluetooth-scan = Run 'dmm-cli list' to scan for adapters and meters in range.

## Notices both binaries show.

# A cable or Bluetooth link is open but no meter on it identified itself.
# A notice title in the GUI; dmm-cli starts a sentence with it, naming the
# link after it (cli/connect.ftl).
no-meter-answered = No meter answered
# The meter picked or found has stopped answering. A notice title in the
# GUI; dmm-cli starts a sentence with it (cli/connect.ftl).
no-response = No response from meter
# The GUI checkbox that lets a connection search over Bluetooth. The CLI
# reads the same setting, so its help names the checkbox too.
bluetooth-setting = Look for Bluetooth devices
bluetooth-off-cli-flag = Leave out --no-bluetooth.
bluetooth-off-cli-setting = Tick "{ bluetooth-setting }" in dmm-gui's settings, which dmm-cli reads too.
bluetooth-off-gui = Tick "{ bluetooth-setting }" in Settings (⚙).

## What is connected, when an --adapter value matched nothing.

adapters-none = No devices currently connected.
adapters-heading = Connected devices:
adapters-unavailable = Run 'dmm-cli list' to see connected devices.
# $address is the value the user typed, written with colons, e.g. 12:34:56:78:9A:BC.
colonless-address-hint = If that is a Bluetooth address, write it { $address }.

## Why a number typed for a scale, an offset or an alarm was refused: the
## reason alone, in lower case and with no full stop. dmm-gui puts it after
## the field's name in a toast ("Invalid scale: …"); dmm-cli after the
## option and the value, at the end of a line from its command-line parser,
## which stays English ("error: invalid value '0' for '--scale <FACTOR>': …").

# $text is what was typed, as it was typed.
input-not-a-number = “{ $text }” is not a number
# Infinity or "not a number" (NaN).
input-not-finite = must be a finite number
# A scale factor of 0 would turn every reading into the offset.
input-zero-scale = a scale of zero would erase the reading
# The alarm's low limit and its high limit.
input-low-not-below-high = the low limit must be below the high one
# The alarm's hysteresis band: a number in the limits' unit, or a percentage
# of the limit. $text is what was typed, as it was typed.
input-band-not-a-number = “{ $text }” is not a number or a percentage such as 1%
# A hysteresis band below zero.
input-band-negative = must be zero or more
# A percentage band with a limit of 0, of which every percentage is 0.
input-percent-of-zero = a percentage of a limit of 0 is 0; give a value instead

## Why an exported file or a recording did not read: in dmm-gui's toast
## after "Import failed:" and the file, in dmm-cli after "Error:" and the
## file. Lower case, with no full stop. The backquotes (`) set off text
## from the file, or a name it must have, which stays as it is: keep them.
## A parser's own words ($detail) arrive in English.

# Where the file stopped: $line is a line number, $reason one of the
# reasons below.
read-error-at-line = line { $line }: { $reason }
read-error-empty = the file is empty
# A CSV export without its first row naming the columns.
read-error-no-header-row = no header row ({ $detail })
# $column is the name of a column every CSV export has, e.g. timestamp.
read-error-no-column = no `{ $column }` column
# A JSON export whose first line is not the one starting {"_metadata":.
read-error-no-metadata = no `_metadata` line ({ $detail })
# $text is what the file has where a reading's value should be.
read-error-not-a-value = `{ $text }` is not a value
# $text is a cell starting NCV: whose level is no number. NCV is
# non-contact voltage detection, as the meter prints it.
read-error-not-an-ncv-level = `{ $text }` is not an NCV level
# $text is the part of a flags cell that names no status flag.
read-error-not-a-flag = `{ $text }` is not a flag
# $text is what the file has where a marker's number should be.
read-error-not-a-marker-number = `{ $text }` is not a marker number
# $text is a timestamp from the file, or a recording's "# recorded:" line;
# RFC 3339 is the standard date format the files use.
read-error-not-a-date = `{ $text }` is not an RFC 3339 date: { $detail }

## The words clap puts around every page of dmm-cli's and dmm-gui's --help:
## its headings, what follows an option, and the help of the built-in -h,
## -V and help. They are the same on every page.

# The headings, each before its list.
help-usage = Usage:
help-commands = Commands:
help-arguments = Arguments:
help-options = Options:
# After an option's help, in brackets. $value is what the option is when not
# given, e.g. 500.
help-default = [default: { $value }]
# After an option's help, in brackets. $values is the values it takes,
# separated by commas.
help-possible-values = [possible values: { $values }]
# Before the list of values an option takes, each with what it does.
help-possible-values-heading = Possible values:
# What -h and --help do: on a command with more detail under --help, the two
# notes say how to see more or less.
help-flag = Print help
help-flag-short = Print help (see more with '--help')
help-flag-long = Print help (see a summary with '-h')
# What -V and --version do.
help-version-flag = Print version
# The help command, listed among the commands, and its argument.
help-command = Print this message or the help of the given subcommand(s)
help-command-subcommand = Print help for the subcommand(s)

## Mistakes on dmm-cli's and dmm-gui's command line: an unknown option, a
## missing command, a value the program doesn't take. Printed on the
## terminal as
##
##     error: invalid value 'xml' for '--format <FORMAT>'
##       [possible values: csv, json, replay]
##
##     Usage: dmm-cli read [OPTIONS]
##
##     For more information, try '--help'.
##
## The usage line's syntax stays English after its heading. Names in quotes
## ($arg, $value, $command, $names) are what the user typed or must type:
## they arrive as they are, sometimes coloured, and stay in the quotes. $arg
## is an option, often with its value's name, e.g. --format <FORMAT>.

# At the start of every message, as the terminal shows errors.
args-error = error:

# An option that takes no value after "=" was given without one.
args-equals-needed = equal sign is needed when assigning values to '{ $arg }'
# An option was given without its value.
args-value-required = a value is required for '{ $arg }' but none was supplied
# A value the option doesn't take. A list of the ones it takes may follow
# on the next line (args-possible-values).
args-invalid-value = invalid value '{ $value }' for '{ $arg }'
# The same, with the reason the value was refused. $reason may be English.
args-invalid-value-because = invalid value '{ $value }' for '{ $arg }': { $reason }
# The values an option takes, on a line of their own under the error. $values
# is the list, separated by commas.
args-possible-values = [possible values: { $values }]
# A command the program doesn't have, e.g. 'lst'.
args-unrecognized-subcommand = unrecognized subcommand '{ $command }'
# Options that must be given, listed one per line under this.
args-missing-required = the following required arguments were not provided:
# The program was run without a command. $command is the program, e.g.
# dmm-cli. The commands follow on the next line (args-subcommands).
args-missing-subcommand = '{ $command }' requires a subcommand but one was not provided
# The commands, on a line of their own under the error. $values is the list,
# separated by commas.
args-subcommands = [subcommands: { $values }]
# A value after an option that takes no more.
args-unexpected-value = unexpected value '{ $value }' for '{ $arg }' found; no more were expected
# Too few values after an option. $needed and $given are numbers; $count is
# $given, for choosing the verb's form ([0] and [one]: one value or none).
args-too-few-values =
    { $count ->
        [0] { $needed } values required by '{ $arg }'; only { $given } was provided
        [one] { $needed } values required by '{ $arg }'; only { $given } was provided
       *[other] { $needed } values required by '{ $arg }'; only { $given } were provided
    }
# The wrong number of values after an option. $needed and $given are
# numbers; $count is $given, for choosing the verb's form.
args-wrong-number-of-values =
    { $count ->
        [0] { $needed } values required for '{ $arg }' but { $given } was provided
        [one] { $needed } values required for '{ $arg }' but { $given } was provided
       *[other] { $needed } values required for '{ $arg }' but { $given } were provided
    }
# An option the program doesn't have, or one this command doesn't take.
args-unexpected-argument = unexpected argument '{ $arg }' found
# An option given twice that can only be given once.
args-repeated = the argument '{ $arg }' cannot be used multiple times
# Two options, or an option and a command, that can't go together. $what is
# argument or subcommand; $name and $other are what the user typed.
args-conflict-with =
    { $what ->
        [subcommand] the subcommand '{ $name }' cannot be used with '{ $other }'
       *[argument] the argument '{ $name }' cannot be used with '{ $other }'
    }
# Two options, or an option and a command, that can't go together, with
# the others listed one per line under this. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-list =
    { $what ->
        [subcommand] the subcommand '{ $name }' cannot be used with:
       *[argument] the argument '{ $name }' cannot be used with:
    }
# The same, when the others aren't named. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-others =
    { $what ->
        [subcommand] the subcommand '{ $name }' cannot be used with one or more of the other specified arguments
       *[argument] the argument '{ $name }' cannot be used with one or more of the other specified arguments
    }

## Hints after the error.

# Before a hint, indented on its own line.
args-tip = tip:
# A name close to what was typed. $what is subcommand, argument or value;
# $names is one or more quoted names, separated by commas; $count is how
# many.
args-similar =
    { $what ->
        [subcommand]
            { $count ->
                [one] a similar subcommand exists: { $names }
               *[other] some similar subcommands exist: { $names }
            }
        [argument]
            { $count ->
                [one] a similar argument exists: { $names }
               *[other] some similar arguments exist: { $names }
            }
       *[value]
            { $count ->
                [one] a similar value exists: { $names }
               *[other] some similar values exist: { $names }
            }
    }
# The last line. $help is --help.
args-more-info = For more information, try '{ $help }'.
