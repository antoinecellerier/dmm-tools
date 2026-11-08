### dmm-gui's --help: what the program and each option do.
###
### Generated from the doc comments in crates/dmm-gui/src/main.rs by
### `UPDATE_L10N=1 cargo test -p dmm-gui gui_help_ftl`: edit those, not this
### file. The run names each ID whose English changed. When the meaning changed
### too, delete that message from every language's gui-help.ftl, so it shows in
### English until it is translated again.
###
### IDs follow the options: dmm-gui--replay.help is what --replay does. clap
### wraps each paragraph to the terminal's width: keep a paragraph on one line.
### Option names (--device), values (auto, mock) and command lines ('dmm-cli
### list') stay as they are. The help built as the program runs is in
### gui/command-line.ftl, and clap's own words around these texts (Usage:,
### Options:, [possible values: …]) are in shared.ftl.

# dmm-gui
dmm-gui =
    .about = 适用于 USB 和蓝牙数字万用表的图形界面应用程序
# dmm-gui --device
dmm-gui--device =
    .help = 要连接的设备 [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...]
# dmm-gui --mock-mode
dmm-gui--mock-mode =
    .help = 将仿真设备固定在指定模式 (同时启用 --device mock)
# dmm-gui --theme
dmm-gui--theme =
    .help = 覆盖主题设置
# dmm-gui --renderer
dmm-gui--renderer =
    .help = 图形渲染器
# dmm-gui --adapter
dmm-gui--adapter =
    .help = 有多个可用适配器时，选择指定的一个。使用 USB 连接线的序列号或 HID 路径，或蓝牙适配器或万用表的地址，写法同 'dmm-cli list' 的输出
# dmm-gui --no-bluetooth
dmm-gui--no-bluetooth =
    .help = 在本次会话中关闭蓝牙扫描。覆盖“查找蓝牙设备”设置。仍会打开传给 --adapter 的地址
# dmm-gui --replay
dmm-gui--replay =
    .help = 回放由 'dmm-cli read --format replay' 或“导出…”→“回放…”写入的文件，而不是连接万用表
# dmm-gui --import
dmm-gui--import =
    .help = 打开导出的 CSV、JSON 或回放文件及其标记，而不是连接万用表
# dmm-gui --completions
dmm-gui--completions =
    .help = 打印 shell 补全脚本后退出
