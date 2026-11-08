### The parts of dmm-gui's --help built as it runs: the first lines of the
### help of --device and --mock-mode, which go on with lists made from the
### meters; and why a value the command line gave was refused. What each
### option does is in gui-help.ftl.
###
### Words in --flag form are typed by the user and stay as they are.

## The first lines of two options' long help (dmm-gui --help).

# Before the list of devices.
command-line-device-intro = 接続するデバイス。今回の実行に限り、保存された設定より優先される。
# Before the list of the simulated meter's modes. "Mock mode" is the
# setting's name in the GUI's Settings.
command-line-mock-mode-intro = モードを自動で順に切り替えずに、シミュレーション デバイスを特定の測定モードに固定する。--device mock を指定したものとして扱い、今回の実行に限り、保存された [シミュレーション モード] 設定より優先される。

## Why a value was refused: the reason alone, in lower case and with no full
## stop, after the option and the value on a line from the command-line
## parser ("error: invalid value 'ut62' for '--device <DEVICE>': …").

# A --device value that names no meter dmm-gui knows.
command-line-unknown-device = 不明なデバイスです。--help で一覧を確認できます
