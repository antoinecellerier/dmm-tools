### The parts of dmm-gui's --help built as it runs: the first lines of the
### help of --device and --mock-mode, which go on with lists made from the
### meters. What each option does is in gui-help.ftl.
###
### Words in --flag form are typed by the user and stay as they are.

## The first lines of two options' long help (dmm-gui --help).

# Before the list of devices.
command-line-device-intro = 接続するデバイス。今回の実行に限り、保存された設定より優先される。
# Before the list of the simulated meter's modes. "Mock mode" is the
# setting's name in the GUI's Settings.
command-line-mock-mode-intro = モードを自動で順に切り替えずに、シミュレーション デバイスを特定の測定モードに固定する。--device mock を指定したものとして扱い、今回の実行に限り、保存された [シミュレーション モード] 設定より優先される。
