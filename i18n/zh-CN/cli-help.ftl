### dmm-cli's --help: what each command and option does.
###
### Generated from the doc comments in crates/dmm-cli/src/cli.rs by
### `UPDATE_L10N=1 cargo test -p dmm-cli cli_help_ftl`: edit those, not this
### file. The run names each ID whose English changed. When the meaning changed
### too, delete that message from every language's cli-help.ftl, so it shows in
### English until it is translated again.
###
### IDs follow the command tree: read.about is what the read command does,
### read--interval-ms.help what its --interval-ms option does. clap wraps each
### paragraph to the terminal's width: keep a paragraph on one line. Option
### names (--device), values (auto, mock) and command lines ('dmm-cli list')
### stay as they are. clap's own words around these texts (Usage:, Options:,
### [default: …], [possible values: …]) are in cli/help.ftl.

# dmm-cli
dmm-cli =
    .about = 适用于 USB 和蓝牙数字万用表的命令行工具
# dmm-cli --device
dmm-cli--device =
    .help = 要连接的设备 [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...]。省略时，依次采用 ~/.config/dmm-tools/settings.json 中的 `device_family` (由 dmm-gui 写入)，再采用 `auto`，即通过 USB 连接线检测万用表
# dmm-cli --adapter
dmm-cli--adapter =
    .help = 有多个可用适配器时，选择指定的一个。使用 USB 连接线的序列号或 HID 路径，或蓝牙适配器或万用表的地址，写法同 'dmm-cli list' 的输出
# dmm-cli --no-bluetooth
dmm-cli--no-bluetooth =
    .help = 本次运行关闭蓝牙扫描: 不扫描适配器和万用表，'list' 只显示连接线。覆盖已保存的 'bluetooth' 设置。传给 --adapter 的地址仍会被打开
# dmm-cli --lang
dmm-cli--lang =
    .help = 消息和帮助的语言，或用 `system` 采用系统语言。省略时，依次采用设置文件中的 `language` (由 dmm-gui 的“语言”一行写入)，再采用系统语言
# dmm-cli list
list =
    .about = 列出已连接的 USB 连接线，以及蓝牙范围内的适配器和万用表
# dmm-cli info
info =
    .about = 连接并打印设备信息
# dmm-cli read
read =
    .about = 持续读取测量值
# dmm-cli read --interval-ms
read--interval-ms =
    .help = 每个间隔最多保留一个读数，单位为毫秒 (0 = 保留万用表产生的每个读数)
# dmm-cli read --format
# .value-… says what that value does, in --help's list of values.
read--format =
    .help = 输出格式 [默认值: text，或由 -o 的扩展名决定]
    .value-replay = 万用表自己的帧，供 --replay 回放
# dmm-cli read --output
read--output =
    .help = 输出文件 (未指定时输出到 stdout)。不给文件名时，文件按万用表、模式和运行的开始时间命名
# dmm-cli read --count
read--count =
    .help = 读数个数 (0 = 不限: 按 Ctrl+C 停止，或到 --replay 文件结束)
# dmm-cli read --integrate
read--integrate =
    .help = 显示累积的时间积分 (电流模式为电荷量，电压为 V·s)
# dmm-cli read --scale
read--scale =
    .help = 将以基本单位 (V, A, Ω, …) 表示的读数乘以 FACTOR。10 mV/A 的电流钳用 --scale 100；100:1 的探头也用 --scale 100
# dmm-cli read --offset
read--offset =
    .help = 换算后加上 VALUE (--scale 1.8 配合 32 可把 °C 变成 °F)
# dmm-cli read --unit
read--unit =
    .help = 用此单位标注换算后的读数，而不是万用表的基本单位
# dmm-cli read --alarm-high
read--alarm-high =
    .help = 读数升到 VALUE 以上时报警，VALUE 以基本单位 (V, A, Ω, …) 表示，换算后的读数则以 --unit 为单位
# dmm-cli read --alarm-low
read--alarm-low =
    .help = 读数降到 VALUE 以下时报警，写法同 --alarm-high
# dmm-cli read --alarm-hysteresis
read--alarm-hysteresis =
    .help = 读数要回到限值内多远，同一限值才会再次报警: 以限值的单位给出的数值，或限值的百分比 (`1%`) [默认值: 万用表末位的几个字]
# dmm-cli read --alarm-bell
read--alarm-bell =
    .help = 每次报警时响终端铃声
# dmm-cli read --mock-mode
read--mock-mode =
    .help = 将仿真设备固定在指定模式 (仅适用于 --device mock)。未指定时，采用设置文件中的仿真模式，否则仿真设备会循环所有模式
# dmm-cli read --replay
read--replay =
    .help = 回放由 --format replay 写入的文件，而不是打开万用表；文件结束时随之结束
# dmm-cli read --import
read--import =
    .help = 读取导出的 CSV、JSON 或回放文件，而不是打开万用表，且不等待: 用于转换文件或打印其摘要
# dmm-cli command
command =
    .about = 向万用表发送按键命令。不带参数运行可列出所选设备的可用命令
# dmm-cli command <ACTION>
command--action =
    .help = 命令名 (不带参数运行可查看可用命令)
# dmm-cli get
get =
    .about = 列出万用表的设置在当前状态下可切换到的值。不带参数运行则列出所有提供选择的设置
# dmm-cli get <SETTING>
get--setting =
    .help = 要列出的设置 (省略则列出全部)
# dmm-cli get --format
get--format =
    .help = 输出格式
# dmm-cli get --mock-mode
get--mock-mode =
    .help = 将仿真设备固定在指定模式 (仅适用于 --device mock)。未指定时，采用设置文件中的仿真模式，否则仿真设备会循环所有模式
# dmm-cli set
set =
    .about = 无需操作万用表，直接切换它的某项设置。不指定 CHOICE 运行可列出该设置在当前状态下能切换到的值
# dmm-cli set <SETTING>
set--setting =
    .help = 要切换的设置
# dmm-cli set <CHOICE>
set--choice =
    .help = 列表中的值标签，或其唯一的片段 (不带它运行可查看列表)
# dmm-cli set --mock-mode
set--mock-mode =
    .help = 将仿真设备固定在指定模式 (仅适用于 --device mock)。未指定时，采用设置文件中的仿真模式，否则仿真设备会循环所有模式
# dmm-cli debug
debug =
    .about = 用于协议调试的原始十六进制转储模式
# dmm-cli debug --count
debug--count =
    .help = 要发送的请求个数 (0 = 不限)
# dmm-cli debug --interval-ms
debug--interval-ms =
    .help = 每个间隔最多保留一个读数，单位为毫秒 (0 = 保留每个读数)
# dmm-cli completions
completions =
    .about = 生成 shell 补全脚本
# dmm-cli completions <SHELL>
completions--shell =
    .help = 要为哪种 shell 生成补全
# dmm-cli capture
capture =
    .about = 用于错误报告和验证的引导式协议捕获
# dmm-cli capture --output
capture--output =
    .help = 输出文件 (默认值: capture-<device>.yaml)。覆盖自动命名
# dmm-cli capture --steps
capture--steps =
    .help = 只运行指定的步骤 (以逗号分隔的 ID，例如 "dcmv,temp,duty")
# dmm-cli capture --unverified
capture--unverified =
    .help = 只运行尚未在真实硬件上确认的步骤，外加自由测试环节
# dmm-cli capture --plan
capture--plan =
    .help = 运行维护者计划文件中的步骤，而不是设备自带的列表
# dmm-cli capture --sniff
capture--sniff =
    .help = 不信任解析器的任何结论: 根据原始字节的变化检测步骤，并逐一确认
# dmm-cli capture --no-drive
capture--no-drive =
    .help = 每个模式步骤之后，不让工具自行设置量程和标志
# dmm-cli capture --settle
capture--settle =
    .help = 在对任何步骤采样之前等待 MS，适用于稳定较慢的读数
# dmm-cli capture --list-steps
capture--list-steps =
    .help = 列出所有可用的步骤 ID 并退出
# dmm-cli capture --format
capture--format =
    .help = --list-steps 如何打印列表 (md 为 issue 检查清单)
# dmm-cli capture --export-instructions
capture--export-instructions =
    .help = 将步骤说明写入 FILE，翻译后供 --instructions 使用，然后退出
# dmm-cli capture --instructions
capture--instructions =
    .help = 显示 FILE 中的步骤说明，即 --export-instructions 所写内容的译文
