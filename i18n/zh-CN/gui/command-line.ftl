### The parts of dmm-gui's --help built as it runs: the first lines of the
### help of --device and --mock-mode, which go on with lists made from the
### meters; and why a value the command line gave was refused. What each
### option does is in gui-help.ftl.
###
### Words in --flag form are typed by the user and stay as they are.

## The first lines of two options' long help (dmm-gui --help).

# Before the list of devices.
command-line-device-intro = 要连接的设备。在本次会话中覆盖已保存的设置。
# Before the list of the simulated meter's modes. "Mock mode" is the
# setting's name in the GUI's Settings.
command-line-mock-mode-intro = 将仿真设备固定在指定的测量模式，不再自动循环。同时启用 --device mock，并在本次会话中覆盖已保存的“仿真模式”设置。

## Why a value was refused: the reason alone, in lower case and with no full
## stop, after the option and the value on a line from the command-line
## parser ("error: invalid value 'ut62' for '--device <DEVICE>': …").

# A --device value that names no meter dmm-gui knows.
command-line-unknown-device = 没有此设备，可通过 --help 查看设备列表
