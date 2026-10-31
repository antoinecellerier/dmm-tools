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

## Dim, under a listing of more than one device.

# $selector is the option as typed, --adapter <serial-path-or-address>.
list-select-tip = Tip: use { $selector } to select a specific device
