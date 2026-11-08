### Text both dmm-cli and dmm-gui show: the shared parts of their --help
### and the words clap puts around it, their command-line errors, the
### setup help when no meter is found, and the notices both print.
###
### Command lines, file paths and web addresses are never part of a message.
### Words in --flag form, and 'dmm-cli list', are typed by the user and stay
### as they are.

## The project page link: the last line of both binaries' --help, and the
## link in the GUI's top bar.

project-link = 帮助和反馈
# $url is the project page's address.
project-link-line = { project-link }: { $url }

## The --device flag's long help, after the binary's own first line. The
## device list between the heading and the aliases is generated.

device-list-heading = 设备:
# What the "auto" device does. Also its hover in the GUI's device list.
device-auto = 检测已连接的万用表
# The "auto" row of the list: what the flag does when it isn't given.
device-auto-default = { device-auto } (默认)
device-aliases =
    也接受别名: ut61e+, ut61b, ut171a, ut181 等。
    名称含特殊字符时请加引号: --device 'ut61e+'
# Beside the simulated meters, in device lists.
device-no-hardware = 无需硬件

## How far a meter's support is verified, beside it in device lists.

stability-verified = 已验证
stability-partly-verified = 部分验证
stability-experimental = 实验性
# Both binaries show it for a meter short of verified, each followed by its
# own way to help verify it.
# $model is the meter's name, e.g. "UNI-T UT8803".
# $stability is verified, partly-verified or experimental.
stability-warning =
    { $stability ->
        [verified] 对万用表 { $model } 的支持已验证 (已在真实硬件上验证)。
        [partly-verified] 对万用表 { $model } 的支持已部分验证 (部分模式和命令尚未在真实硬件上确认)。
       *[experimental] 对万用表 { $model } 的支持尚处于实验阶段 (未在真实硬件上验证)。
    }

## The --mock-mode flag's long help, after the binary's own first line.

# $modes lists the mock's modes, separated by commas.
mock-mode-list = 模式: { $modes }
# $example is a command line.
mock-mode-example = 示例: { $example }
mock-clock-mock-only = --mock-clock-scale 和 --mock-clock-preseed 只适用于仿真设备和回放。请改用 --device mock 或 --replay 重新运行，或去掉这两个时钟选项。

## A link's short name, where the meter is already named: the GUI's
## status line, the setup help's section labels.

link-usb-cable = USB 连接线
link-bluetooth = 蓝牙

## The help shown when no meter was found: a title, then one section per
## link searched — a label line, then steps. The CLI prints them in a
## terminal, the GUI in a notice.

# No full stop: the CLI adds one, the GUI's notice titles carry none.
not-found-usb = 找不到 USB 连接线
not-found-usb-or-bluetooth = 通过 USB 或蓝牙找不到万用表
# $address is a Bluetooth address or peripheral id.
not-found-bluetooth-at = 在 { $address } 找不到蓝牙设备
# $model is the meter's name, e.g. "UNI-T UT60BT".
not-found-bluetooth-only = 通过蓝牙找不到万用表 { $model }
# A section's label line.
# $link is link-usb-cable or link-bluetooth.
# $check is setup-usb-check or setup-bluetooth-check.
setup-label = { $link }: { $check }
setup-usb-check = 检查连接线是否已插好、万用表是否已开机。
setup-bluetooth-check = 打开此计算机的蓝牙，并在万用表上打开数据传输。
# Linux. Two command lines follow.
setup-usb-linux-rule = 确认已安装 udev 规则:
# Linux. The setup guide's address follows.
setup-usb-linux-replug = 然后重新插拔连接线。在无显示器的机器上，请在规则中保留一个用户组:
# Windows. The quoted names are what Device Manager shows; use the names
# Windows shows in your language.
setup-usb-windows-open = 插好连接线，打开“设备管理器”:
# UNVERIFIED Windows name
setup-usb-windows-cp2110 = - '人体学输入设备'下的 'CP2110 USB to UART Bridge': 无需操作。
# UNVERIFIED Windows name
setup-usb-windows-input = - '人体学输入设备'下的 'USB 输入设备': 无需操作。
# The driver's download address follows.
setup-usb-windows-warning = - '其他设备'下出现黄色警告图标: 请从以下地址安装驱动程序
setup-usb-windows-nothing = - 没有任何显示: 换一个 USB 端口试试。
# macOS. Name the settings as macOS shows them in your language.
setup-usb-macos-auto = 系统应能自动识别连接线 (无需驱动程序)。
setup-usb-macos-privacy = 如果找不到设备，请检查“系统设置 > 隐私与安全 > 输入监控”。
setup-bluetooth-adapter = 如果两者都已打开，请关闭再打开适配器。
setup-bluetooth-macos-privacy = 在“系统设置 > 隐私与安全 > 蓝牙”中允许蓝牙。
# The device catalog's address follows.
setup-bluetooth-catalog = 受支持的适配器和万用表列在设备目录中:
setup-bluetooth-scan = 运行 'dmm-cli list' 扫描蓝牙范围内的适配器和万用表。

## Notices both binaries show.

# A cable or Bluetooth link is open but no meter on it identified itself.
# A notice title in the GUI; dmm-cli starts a sentence with it, naming the
# link after it (cli/connect.ftl).
no-meter-answered = 没有万用表响应
# The meter picked or found has stopped answering. A notice title in the
# GUI; dmm-cli starts a sentence with it (cli/connect.ftl).
no-response = 万用表没有响应
# Why a dropped link is not reconnected. $count (2 or more) USB cables the
# open could take are plugged in, with no serial number of their own to tell
# them apart. Each binary follows it with what to do.
several-cables =
    { $count ->
       *[other] 检测到 { $count } 根可供本次会话打开的 USB 连接线，它们无法相互区分。重新连接时可能会打开其他万用表的连接线。
    }
# While a dropped link is being reopened: its cable has gone and another is
# plugged in. Each binary follows it with how to use that other cable.
waiting-for-own-cable = 已插入另一根 USB 连接线，但本次会话正在等待原先打开的连接线。
# Under a reading the meter shows on its own screen only, its frames carrying
# none: a CMS or UT217's oscilloscope mode.
screen-only-reading = 请在万用表显示屏上查看示波器读数
# The GUI checkbox that lets a connection search over Bluetooth. The CLI
# reads the same setting, so its help names the checkbox too.
bluetooth-setting = 查找蓝牙设备
bluetooth-off-cli-flag = 去掉 --no-bluetooth。
bluetooth-off-cli-setting = 在 dmm-gui 的设置中勾选“{ bluetooth-setting }”，dmm-cli 也读取该设置。
bluetooth-off-gui = 在“设置”(⚙) 中勾选“{ bluetooth-setting }”。

## What is connected, when an --adapter value matched nothing.

adapters-none = 当前没有已连接的设备。
adapters-heading = 已连接的设备:
adapters-unavailable = 运行 'dmm-cli list' 查看已连接的设备。
# $address is the value the user typed, written with colons, e.g. 12:34:56:78:9A:BC.
colonless-address-hint = 如果这是蓝牙地址，请写成 { $address }。

## Why a number typed for a scale, an offset or an alarm was refused: the
## reason alone, in lower case and with no full stop. dmm-gui puts it after
## the field's name in a toast ("Invalid scale: …"); dmm-cli after the
## option and the value, at the end of a line from its command-line parser,
## which stays English ("error: invalid value '0' for '--scale <FACTOR>': …").

# $text is what was typed, as it was typed.
input-not-a-number = “{ $text }”不是数字
# Infinity or "not a number" (NaN).
input-not-finite = 必须是有限数
# A scale factor of 0 would turn every reading into the offset.
input-zero-scale = 比例因子为 0 会使读数始终为 0
# The alarm's low limit and its high limit.
input-low-not-below-high = 下限必须低于上限
# The alarm's hysteresis band: a number in the limits' unit, or a percentage
# of the limit. $text is what was typed, as it was typed.
input-band-not-a-number = “{ $text }”不是数字，也不是 1% 这样的百分数
# A hysteresis band below zero.
input-band-negative = 必须大于或等于零
# A percentage band with a limit of 0, of which every percentage is 0.
input-percent-of-zero = 0 的任何百分比都是 0，请改为输入一个数值

## Why an exported file or a recording did not read: in dmm-gui's toast
## after "Import failed:" and the file, in dmm-cli after "Error:" and the
## file. Lower case, with no full stop. The backquotes (`) set off text
## from the file, or a name it must have, which stays as it is: keep them.
## A parser's own words ($detail) arrive in English.

# Where the file stopped: $line is a line number, $reason one of the
# reasons below.
read-error-at-line = 第 { $line } 行: { $reason }
read-error-empty = 文件为空
# A CSV export without its first row naming the columns.
read-error-no-header-row = 没有标题行 ({ $detail })
# $column is the name of a column every CSV export has, e.g. timestamp.
read-error-no-column = 没有 `{ $column }` 列
# A JSON export whose first line is not the one starting {"_metadata":.
read-error-no-metadata = 没有 `_metadata` 行 ({ $detail })
# $text is what the file has where a reading's value should be.
read-error-not-a-value = `{ $text }` 不是数值
# $text is a cell starting NCV: whose level is no number. NCV is
# non-contact voltage detection, as the meter prints it.
read-error-not-an-ncv-level = `{ $text }` 不是 NCV 等级
# $text is the part of a flags cell that names no status flag.
read-error-not-a-flag = `{ $text }` 不是状态标志
# $text is what the file has where a marker's number should be.
read-error-not-a-marker-number = `{ $text }` 不是标记编号
# $text is a timestamp from the file, or a recording's "# recorded:" line;
# RFC 3339 is the standard date format the files use.
read-error-not-a-date = `{ $text }` 不是 RFC 3339 日期: { $detail }

## The words clap puts around every page of dmm-cli's and dmm-gui's --help:
## its headings, what follows an option, and the help of the built-in -h,
## -V and help. They are the same on every page.

# The headings, each before its list.
help-usage = 用法:
help-commands = 命令:
help-arguments = 参数:
help-options = 选项:
# After an option's help, in brackets. $value is what the option is when not
# given, e.g. 500.
help-default = [默认值: { $value }]
# After an option's help, in brackets. $values is the values it takes,
# separated by commas.
help-possible-values = [可能的值: { $values }]
# Before the list of values an option takes, each with what it does.
help-possible-values-heading = 可能的值:
# What -h and --help do: on a command with more detail under --help, the two
# notes say how to see more or less.
help-flag = 打印帮助
help-flag-short = 打印帮助 (使用 '--help' 查看更多)
help-flag-long = 打印帮助 (使用 '-h' 查看摘要)
# What -V and --version do.
help-version-flag = 打印版本
# The help command, listed among the commands, and its argument.
help-command = 打印此消息或给定子命令的帮助
help-command-subcommand = 打印子命令的帮助

## Mistakes on dmm-cli's and dmm-gui's command line: an unknown option, a
## missing command, a value the program doesn't take. Printed on the
## terminal as
##
##     error: invalid value 'xml' for '--format <FORMAT>'
##       [possible values: csv, json, replay]
##
##     Usage: dmm-cli read [OPTIONS]
##
##     For more information, try '--help'.
##
## The usage line's syntax stays English after its heading. Names in quotes
## ($arg, $value, $command, $names) are what the user typed or must type:
## they arrive as they are, sometimes coloured, and stay in the quotes. $arg
## is an option, often with its value's name, e.g. --format <FORMAT>.

# At the start of every message, as the terminal shows errors.
args-error = 错误:

# An option that takes no value after "=" was given without one.
args-equals-needed = 为 '{ $arg }' 赋值时需要等号
# An option was given without its value.
args-value-required = '{ $arg }' 需要一个值，但没有提供
# A value the option doesn't take. A list of the ones it takes may follow
# on the next line (args-possible-values).
args-invalid-value = 为 '{ $arg }' 提供的值 '{ $value }' 无效
# The same, with the reason the value was refused. $reason may be English.
args-invalid-value-because = 为 '{ $arg }' 提供的值 '{ $value }' 无效: { $reason }
# The values an option takes, on a line of their own under the error. $values
# is the list, separated by commas.
args-possible-values = [可能的值: { $values }]
# A command the program doesn't have, e.g. 'lst'.
args-unrecognized-subcommand = 无法识别的子命令 '{ $command }'
# Options that must be given, listed one per line under this.
args-missing-required = 未提供以下必需的参数:
# The program was run without a command. $command is the program, e.g.
# dmm-cli. The commands follow on the next line (args-subcommands).
args-missing-subcommand = '{ $command }' 需要一个子命令，但没有提供
# The commands, on a line of their own under the error. $values is the list,
# separated by commas.
args-subcommands = [子命令: { $values }]
# A value after an option that takes no more.
args-unexpected-value = '{ $arg }' 收到了多余的值 '{ $value }'，不再接受更多的值
# Too few values after an option. $needed and $given are numbers; $count is
# $given, for choosing the verb's form ([0] and [one]: one value or none).
args-too-few-values =
    { $count ->
       *[other] '{ $arg }' 需要 { $needed } 个值，但只提供了 { $given } 个
    }
# The wrong number of values after an option. $needed and $given are
# numbers; $count is $given, for choosing the verb's form.
args-wrong-number-of-values =
    { $count ->
       *[other] '{ $arg }' 需要 { $needed } 个值，但提供了 { $given } 个
    }
# An option the program doesn't have, or one this command doesn't take.
args-unexpected-argument = 发现意外的参数 '{ $arg }'
# An option given twice that can only be given once.
args-repeated = 参数 '{ $arg }' 不能重复使用
# Two options, or an option and a command, that can't go together. $what is
# argument or subcommand; $name and $other are what the user typed.
args-conflict-with =
    { $what ->
        [subcommand] 子命令 '{ $name }' 不能与 '{ $other }' 一起使用
       *[argument] 参数 '{ $name }' 不能与 '{ $other }' 一起使用
    }
# Two options, or an option and a command, that can't go together, with
# the others listed one per line under this. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-list =
    { $what ->
        [subcommand] 子命令 '{ $name }' 不能与以下内容一起使用:
       *[argument] 参数 '{ $name }' 不能与以下内容一起使用:
    }
# The same, when the others aren't named. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-others =
    { $what ->
        [subcommand] 子命令 '{ $name }' 不能与指定的某些其他参数一起使用
       *[argument] 参数 '{ $name }' 不能与指定的某些其他参数一起使用
    }

## Hints after the error.

# Before a hint, indented on its own line.
args-tip = 提示:
# A name close to what was typed. $what is subcommand, argument or value;
# $names is one or more quoted names, separated by commas; $count is how
# many.
args-similar =
    { $what ->
        [subcommand]
            { $count ->
               *[other] 存在类似的子命令: { $names }
            }
        [argument]
            { $count ->
               *[other] 存在类似的参数: { $names }
            }
       *[value]
            { $count ->
               *[other] 存在类似的值: { $names }
            }
    }
# The last line. $help is --help.
args-more-info = 如需更多信息，请运行 '{ $help }'。
