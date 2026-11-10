### The parts of dmm-gui's --help built as it runs: the first lines of the
### help of --device and --mock-mode, which go on with lists made from the
### meters. What each option does is in gui-help.ftl.
###
### Words in --flag form are typed by the user and stay as they are.

## The first lines of two options' long help (dmm-gui --help).

# Before the list of devices.
command-line-device-intro = Device to connect to. Overrides saved settings for this session.
# Before the list of the simulated meter's modes. "Mock mode" is the
# setting's name in the GUI's Settings.
command-line-mock-mode-intro = Pin the mock device to a specific measurement mode instead of auto-cycling. Implies --device mock, and overrides the saved Mock mode setting for this session.
