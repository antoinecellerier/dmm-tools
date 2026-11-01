### The library's errors: why a connection, a command or a file failed.
### They show in the GUI's connection notice under the reading and in its
### toasts (the short messages under the top bar's right end), and in the
### CLI after "Error:". Hand-written; a test checks each English against
### the library's own. The last group is the library's one notice that is
### not an error.
###
### Mode, range and state names arrive already translated, as the reading
### shows them. Button names (SELECT, RANGE, AUTO, EXIT) are printed on the
### meter: keep them as they are, as command-line options (--adapter,
### --device) and command words (auto, rel). Text the operating system or
### the Bluetooth stack wrote arrives as it is, in their language.

## What went wrong, by the part that failed. Several end in a colon and
## the reason, which comes from one of the groups below.

# The USB layer failed. $detail is what it said, in English.
error-hid = HID 错误: { $detail }
# The meter sent something that makes no sense. $detail says what, in
# English, for a bug report.
error-invalid-response = 响应无效: { $detail }
# The meter on the link is not one the meter chosen can read, worded as
# error-invalid-response. $reason is one of the "Another meter" sentences
# below.
error-wrong-meter = 响应无效: { $reason }
# A frame from the meter failed its check. $expected and $actual are
# hexadecimal numbers, e.g. 0x01a3.
error-checksum-mismatch = 校验和不匹配: 应为 { $expected }，实际为 { $actual }
error-timeout = 等待响应超时
# The meter reports a mode this tool does not know. $code is a hexadecimal
# number, e.g. 0x1f.
error-unknown-mode = 未知模式: { $code }
# $reason is one of the "Refused before it was sent" sentences below.
error-unsupported-command = 不支持的命令: { $reason }
# $reason is one of the "The meter did not do it" sentences below.
error-command-rejected = 命令被拒绝: { $reason }
# $device is the meter's name or ID, as typed with --device or as the
# settings file holds it, sometimes with an explanation in English.
error-unknown-device = 未知设备: { $device }
# A meter on a cable was opened as a simulated one. $device is its ID, e.g.
# ut61eplus.
error-not-simulated = 未知设备: { $device } 不是仿真设备
# A recording could not be played back. $detail names the file or the line
# and says what is wrong with it, in English.
error-replay = 回放文件: { $detail }
# $adapter is the value given with --adapter, sometimes with an explanation
# in English.
error-adapter-not-found = 找不到适配器: { $adapter }
# What --adapter named, as error-adapter-not-found's $adapter and the GUI's
# connection-adapter-not-matched quote it, when it is a USB device that is
# no supported cable. $path is the device's path, as given with --adapter.
error-not-an-adapter = { $path } (设备存在，但不是受支持的 USB 适配器)
# --adapter named a cable or adapter the chosen meter cannot be reached
# through. $link is bluetooth (a Bluetooth adapter) or cable (a USB cable
# whose name is $cable, e.g. BU-86X). $model is the meter, e.g. UNI-T
# UT61E+.
error-wrong-cable =
    { $link ->
        [bluetooth] --adapter 指定的是蓝牙适配器，无法用于万用表 { $model }: 请用 --adapter 指定该万用表自己的连接线，或去掉 --adapter
       *[cable] --adapter 指定的是 { $cable } 连接线，无法用于万用表 { $model }: 请用 --adapter 指定该万用表自己的连接线，或去掉 --adapter
    }
error-link-lost = 蓝牙链路中断
# $reason is one of the "Bluetooth" sentences below.
error-bluetooth = 蓝牙: { $reason }
# A cable or Bluetooth link is up but no meter answered on it. $link is usb
# (a USB cable), adapter (a Bluetooth adapter plugged into the meter) or
# radio (the meter's own Bluetooth).
error-device-not-identified =
    { $link ->
        [adapter] 通过蓝牙适配器没有万用表响应
        [radio] 通过蓝牙链路没有万用表响应
       *[usb] 通过 USB 连接线没有万用表响应
    }
# No cable was found. $bluetooth is searched when Bluetooth was searched
# too, else not-searched.
error-no-transport-found =
    { $bluetooth ->
        [searched] 找不到受支持的 USB 适配器，蓝牙范围内也没有蓝牙设备
       *[not-searched] 找不到受支持的 USB 适配器
    }
# As error-no-transport-found, naming the cables looked for; $bluetooth as
# there. $cables lists them with their USB IDs, e.g. "CP2110 0x10c4:0xea80,
# BU-86X 0x0820:0x0001".
error-no-transport-found-looked-for =
    { $bluetooth ->
        [searched] 找不到受支持的 USB 适配器 (已查找 { $cables })，蓝牙范围内也没有蓝牙设备
       *[not-searched] 找不到受支持的 USB 适配器 (已查找 { $cables })
    }
# A meter with Bluetooth built in, and none was in range. $model is the
# meter, e.g. UNI-T UT60BT.
error-bluetooth-only-not-in-range = 蓝牙范围内找不到万用表 { $model }
# A meter with Bluetooth built in, and Bluetooth was not searched. $model
# is the meter, e.g. UNI-T UT60BT. Each app adds how to switch it on.
error-bluetooth-only-switched-off = 万用表 { $model } 只能通过蓝牙连接，而蓝牙已关闭
# $model is the meter, e.g. UNI-T UT60BT.
error-bluetooth-only-not-built = 万用表 { $model } 只能通过蓝牙连接，而此版本不支持蓝牙
# $model is the meter, e.g. UNI-T UT60BT.
error-bluetooth-only-usb-adapter = 万用表 { $model } 只能通过蓝牙连接，而 --adapter 指定的是 USB 设备: 请传入该万用表的蓝牙地址，或去掉 --adapter

## Refused before it was sent: the meter, the link or the dial position
## cannot do it. After "unsupported command:". $setting is a setting's
## name as the CLI takes it (mode, range, rel, minmax…); $word, $target and
## $from_word are a mode's hexadecimal code, e.g. 0x3111.

# $setting is a setting's name.
error-unsupported-setting = 此万用表无法设置 { $setting }
# $mode is a mode.
error-unsupported-several-positions = { $mode } 会在多个档位上报告，请先在其他模式下取一次读数
# $mode is a mode.
error-unsupported-no-position = { $mode } 不在任何已知档位上
# $target and $from are modes.
error-unsupported-mode-unreachable = 无法从 { $from } 切换到模式 { $target }，请先转动旋钮
# $target and $from_word are mode codes, $from is the mode $from_word is.
error-unsupported-word-unreachable = 无法从 { $from } ({ $from_word }) 切换到模式 { $target }，请先转动旋钮
# $mode is a mode.
error-unsupported-no-ranges = { $mode } 在此万用表上没有可选的量程
# $id is a range's number, $count how many ranges $mode, a mode, has.
error-unsupported-no-such-range = 量程 { $id } 不在 { $mode } 提供的 { $count } 个量程之中
# $id is a range's number, $count how many ranges $mode, a mode, has.
error-unsupported-range-not-listed = 量程 { $id } 不在 { $mode } 提供的 { $count } 个量程之中
# $count is how many ranges $mode, a mode, has.
error-unsupported-auto-range-not-listed = 自动量程不在 { $mode } 提供的 { $count } 个量程之中
# $mode is a mode, $word its code.
error-unsupported-fixed-range = { $mode } ({ $word }) 中的量程: 固定量程模式
# $setting is a setting's name, $mode a mode.
error-unsupported-setting-not-in-mode = 此万用表在 { $mode } 下无法设置 { $setting }
# $setting is a setting's name, $mode a mode, $word its code.
error-unsupported-setting-not-in-word = 此万用表在 { $mode } ({ $word }) 下无法设置 { $setting }
# $setting is a setting's name, $id a state's number, $mode a mode and
# $offered the states it offers, separated by commas (on, off, MAX…).
error-unsupported-no-such-state = { $setting } 没有状态 { $id }；{ $mode } 提供 { $offered }
# $command is a command word, $mode a Peak mode, $word its code.
error-unsupported-refused-in-peak = { $mode } ({ $word }) 下的 { $command }: 万用表在峰值模式下拒绝该命令
# "rel" is the command word. $mode is a mode, $word its code.
error-unsupported-no-rel = { $mode } ({ $word }) 下的 rel
# $model is the meter's part number, e.g. VC-880; $button a button's name.
error-unsupported-no-button = 万用表 { $model } 没有 { $button } 键
# $cable is a cable's name, e.g. BU-86X; $len and $got are byte counts.
error-unsupported-request-length = { $cable } 连接线的请求长度为 { $len } 字节，而不是 { $got } 字节
# $link is usb (the USB cable), adapter (the Bluetooth adapter) or none (a
# link that does not say what it is); $baud a serial speed, e.g. 19200.
error-unsupported-baud =
    { $link ->
        [usb] USB 连接线无法把速率改为 { $baud } 波特
        [adapter] 蓝牙适配器无法把速率改为 { $baud } 波特
       *[none] 此链路无法把速率改为 { $baud } 波特
    }
# A developer's error: the simulated meter has no bytes to decode.
error-unsupported-no-wire-format = parse_payload: 仿真设备没有传输格式

## The meter did not do it: a command it was sent did not take, or the
## meter's state says it cannot. After "command rejected:". $button is a
## button's name; $state, $target and $now are modes, ranges or setting
## states.

# $button is a button's name, $state what the meter stayed in.
error-rejected-did-nothing = 在 { $state } 下，{ $button } 没有任何作用
# $target is what was asked for, $now what the meter is in.
error-rejected-never-appeared = 一直没有出现 { $target }；万用表已回到 { $now }
# $presses is how many times $button was pressed; $state is what the meter
# is in.
error-rejected-gave-up = 按了 { $presses } 次 { $button } 后放弃；万用表处于 { $state }
# $mode is a mode.
error-rejected-mode-changed = 模式已变为 { $mode }；已停止按 RANGE 键
# $reason is another sentence of this group, $mode the mode the meter was
# in before.
error-rejected-started-in = { $reason }；起始模式为 { $mode }
# $range is a range.
error-rejected-auto-did-nothing = AUTO 没有任何作用；万用表仍处于 { $range }
# $button is a button's name, $state a setting's state.
error-rejected-exit-did-nothing = EXIT { $button } 没有任何作用；万用表仍处于 { $state }
# $setting is a setting's name, $target and $now its states.
error-rejected-switch-did-nothing = { $setting } { $target } 没有任何作用；万用表仍处于 { $now }
# $sent is what was sent: a command word, or one of the error-sent messages
# below, or a setting's name and a state.
error-rejected-by-meter = 万用表拒绝了 { $sent }
error-rejected-zero-needs-capacitance = ZERO 只能在电容模式下使用
# $code is a number, $model and $opened are meters (e.g. OWON B41T+), $id a
# meter's ID as --device takes it. "Auto-detect" is the GUI's Device
# choice.
error-rejected-other-model = 万用表报告的 OWON 型号代码为 { $code } ({ $model })，不是打开时选择的 { $opened }，因此遥控按键已关闭；请选择“自动检测”或 --device { $id }
# $code is a number.
error-rejected-unknown-model = 万用表报告的 OWON 型号代码为 { $code }，本工具尚不认识，因此遥控按键已关闭
# $command is a command word.
error-rejected-model-unread = 未能读取万用表的型号代码，因此 { $command } 已关闭: 要退出 REL，请在万用表上长按 REL 键
error-rejected-min-max-locks-range = MIN/MAX 会锁定量程；请先退出 MIN/MAX 再更改量程
# $command is a command word or a setting's name.
error-rejected-replay = { $command }: 回放不接受命令

# What a refused command switched to, as error-rejected-by-meter names it.
# $mode is a mode.
error-sent-mode = 模式 { $mode }
# $range is a range.
error-sent-range = 量程 { $range }

## Another meter: the one on the link is not one the meter chosen can
## read. After "invalid response:". Model names (OWON B35T+, Brymen BM82x)
## and --device's values (vc871, bm82x) stay as they are. Auto-detect is
## the meter choice that finds the meter itself: the GUI's Device list
## names it so, and the CLI takes it as --device auto.

# An OWON meter names its model, which sends the other kind of data frame.
# $code is OWON's model number, e.g. 87; $model the model it names, e.g.
# Voltcraft VC871; $frame the length of that model's frame in bytes, 6 or
# 15; $opened the meter chosen, e.g. OWON B35T+; $id the --device value
# for $model.
error-wrong-meter-other-frame = 万用表报告的 OWON 型号代码为 { $code } ({ $model })，不是打开时选择的 { $opened }，该型号发送 OWON 的 { $frame } 字节数据帧；请选择“自动检测”或 --device { $id }
# An OWON meter sends the other kind of data frame and names no model.
# $sent and $expected are frame lengths in bytes, 6 or 15; $opened is the
# meter chosen, e.g. Voltcraft VC871. $remedy is auto-detect, or own-model
# when the user has to pick the meter's model themselves.
error-wrong-meter-other-frame-kind =
    { $remedy ->
        [auto-detect] 万用表发送的是 OWON 的 { $sent } 字节数据帧，而不是打开时选择的 { $opened } 的 { $expected } 字节数据帧；请选择“自动检测”
       *[own-model] 万用表发送的是 OWON 的 { $sent } 字节数据帧，而不是打开时选择的 { $opened } 的 { $expected } 字节数据帧；请选择该万用表自己的型号
    }
# An OWON model this tool does not read yet. $code is OWON's model number,
# e.g. 83. "This message" is the whole error, for the issue.
error-wrong-meter-not-yet = 万用表报告的 OWON 型号代码为 { $code }，本工具尚无法读取；请附上此消息提交 issue
# An OWON model whose readings carry their sign where this tool does not
# read it yet. $code is OWON's model number, e.g. 55.
error-wrong-meter-sign-in-function-word = 万用表报告的 OWON 型号代码为 { $code }: 它发送的 OWON 6 字节数据帧把符号放在功能字中，本工具尚无法读取
# A Brymen meter of another series answered. $found and $opened are series,
# e.g. Brymen BM82x and Brymen BM86x; $id is the --device value for $found.
error-wrong-meter-other-series = 连接线上的万用表是 { $found }，不是 { $opened }；请选择“自动检测”或 --device { $id }
# An OWON meter chosen by its model sends bytes, but never its 6-byte data
# frame. B35 and B35T are OWON models.
error-wrong-meter-no-six-byte-frames = 万用表没有发送 OWON 的 6 字节数据帧；它可能是较旧的 B35 或 B35T，本工具尚无法读取它们的 14 字节数据帧
# The same for a meter whose data frame is 15 bytes long.
error-wrong-meter-no-fifteen-byte-frames = 万用表没有发送 OWON 的 15 字节数据帧；请选择“自动检测”
# An OWON meter that names no model sends frames of neither length. "This
# message" is the whole error, for the issue.
error-wrong-meter-untold-frame-kind = 无法从万用表的型号代码或数据帧判断它发送的是 OWON 的 6 字节还是 15 字节数据帧；请附上此消息提交 issue

## Bluetooth: what the Bluetooth stack could not do. After "Bluetooth:".

# $detail is what the operating system said.
error-bluetooth-runtime = 无法启动蓝牙运行时: { $detail }
error-bluetooth-no-adapter = 此计算机上无法使用
error-bluetooth-powered-off = 此计算机上已关闭
# $role is write (the characteristic commands are written to) or notify
# (the one the meter's data arrives on).
error-bluetooth-no-characteristic =
    { $role ->
        [write] 该蓝牙设备没有可识别的数据写入特征: 它不是受支持的适配器或万用表，或其服务始终未能解析
       *[notify] 该蓝牙设备没有可识别的数据通知特征: 它不是受支持的适配器或万用表，或其服务始终未能解析
    }
# A Brymen meter refused the password this tool sends.
error-bluetooth-password-refused = 万用表拒绝了连接密码 0000: 要重置密码，请{ error-bluetooth-reset-gesture }
# How a Brymen BM788BT/BM787BT's password and name go back to the factory
# settings. The meter's activation steps in lib.ftl (activation-bm78xbt)
# end with the same sentence: word them alike. "Org" is what the display
# shows.
error-bluetooth-reset-gesture = 按住 Hz 键，同时在 0.6 s 内将旋钮从 OFF 转到电容档: 万用表显示 "Org"，连接密码和蓝牙名称恢复为出厂设置
# A Brymen meter refused the login with error $code, a number.
error-bluetooth-login-refused =
    { $code ->
        [0] 万用表拒绝了登录: 校验和错误 (代码 { $code })
        [1] 万用表拒绝了登录: 通道 ID 无效 (代码 { $code })
        [2] 万用表拒绝了登录: 超出设置范围 (代码 { $code })
        [5] 万用表拒绝了登录: 参数无效 (代码 { $code })
        [6] 万用表拒绝了登录: 权限不足 (代码 { $code })
       *[other] 万用表拒绝了登录: 未知错误 (代码 { $code })
    }

## Not an error: the notice under the reading in the GUI when a Bluetooth
## adapter hands over a reading late, together with the next one.

late-readings-title = 部分蓝牙读数延迟到达
# $fix is command when a command follows that fixes it for this connection
# (it ends in a colon, and the command shows on the next line as it is), or
# none.
late-readings-advice =
    { $fix ->
        [command] 使用此适配器时，有时一个读数会和下一个读数一起到达，但不会丢失。要在万用表断开连接前获得准确的时间戳，请运行:
       *[none] 使用此适配器时，有时一个读数会和下一个读数一起到达，但不会丢失。
    }
