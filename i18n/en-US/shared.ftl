### Text both dmm-cli and dmm-gui show: the shared parts of their --help,
### the setup help when no meter is found, and the notices both print.
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
