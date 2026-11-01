### Text both dmm-cli and dmm-gui show: the shared parts of their --help,
### the setup help when no meter is found, and the notices both print.
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
       *[experimental] 对万用表 { $model } 的支持为实验性 (尚未在真实硬件上验证)。
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
setup-usb-check = 检查连接线是否已插好，且万用表已开机。
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
setup-usb-macos-auto = 连接线应会被自动识别 (无需驱动程序)。
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
input-percent-of-zero = 0 的百分之几仍是 0，请改为给出一个数值

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
