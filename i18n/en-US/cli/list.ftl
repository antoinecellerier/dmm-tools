### What 'dmm-cli list' prints around the devices it finds. Each device line
### (its path or address, chip, product and serial number, as the system
### reports them) is on the standard output and stays as it is; --adapter
### takes those values. The rest goes to the terminal's error stream.

## While listing.

# Dim, before the radio scan, which takes a few seconds.
list-scanning = Scanning over Bluetooth…
# A Bluetooth adapter the system has paired that the scan didn't hear, dim,
# after the line's number in brackets. $device is the device as listed, e.g.
# "AA:BB:CC:DD:EE:FF [Bluetooth] — UT-D07B"; keep it first.
list-not-heard = { $device }, paired but not heard: check it is switched on

## Nothing was found: one of these, in yellow, then the setup steps for each
## link searched (shared.ftl).

list-none-found = No devices found.
# Paired Bluetooth adapters are listed above, but the scan heard none.
list-none-heard = No devices heard in range.

## The radio scan found no adapter or meter this tool knows by name, but
## heard other named devices: this heading, one line per device (what
## --adapter takes, then its name, as the system reports them), then how to
## open one. A meter that goes by an unexpected name is among them.

list-unrecognised-heading = Other Bluetooth devices the scan heard:
# --device <id> and --adapter are command-line options, kept as they are;
# each device's line starts with its address. $url is the issue tracker.
list-unrecognised-open = If one is your meter, open it with --device <id> and --adapter followed by its address (--help lists the ids), and report its name at { $url } so this tool finds it.

## Dim, under a listing of more than one device.

# $selector is the option as typed, --adapter <serial-path-or-address>.
list-select-tip = Tip: use { $selector } to select a specific device
