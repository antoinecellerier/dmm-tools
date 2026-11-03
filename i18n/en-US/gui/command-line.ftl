### The parts of dmm-gui's --help built as it runs: the first lines of the
### help of --device and --mock-mode, which go on with lists made from the
### meters; and why a value the command line gave was refused. What each
### option does is in gui-help.ftl.
###
### Words in --flag form are typed by the user and stay as they are.

## The first lines of two options' long help (dmm-gui --help).

# Before the list of devices.
command-line-device-intro = Device to connect to. Overrides saved settings for this session.
# Before the list of the simulated meter's modes. "Mock mode" is the
# setting's name in the GUI's Settings.
command-line-mock-mode-intro = Pin the mock device to a specific measurement mode instead of auto-cycling. Implies --device mock, and overrides the saved Mock mode setting for this session.

## Why a value was refused: the reason alone, in lower case and with no full
## stop, after the option and the value on a line from the command-line
## parser ("error: invalid value 'ut62' for '--device <DEVICE>': …").

# A --device value that names no meter dmm-gui knows.
command-line-unknown-device = no such device; --help lists them
