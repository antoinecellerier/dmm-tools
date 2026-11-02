### The connection notices drawn in the reading column under the reading
### (or, in the big-meter modes, its title in place of the reading and the
### rest in a tooltip), and the toasts: the short messages that float under
### the top bar's right end for a few seconds.
###
### ⚙ is the settings button's icon, which the notices point at. The
### notices are hard-wrapped to fit a narrow column: keep their line breaks
### where a line would otherwise run long.

## While there is nothing to show yet, or no more to come.

# The meter is being identified: each kind of meter gets a turn to answer.
connection-detecting = 正在检测万用表…
# A recording being played back reached its last reading.
connection-ended = 回放已结束
# A stretch of a recording being played back that has no readings in it.
connection-no-frames = 回放文件的这一段没有帧
# The meter has gone quiet for a moment. $dots is one to four dots, padded
# with spaces to four characters, that grow while the wait lasts: keep it
# at the end, with nothing after it.
connection-waiting = 正在等待万用表{ $dots }
# Under the waiting title, when a meter is picked in Settings.
connection-waiting-check-selection = 检查“设置”(⚙) 中是否选择了正确的设备，或在其中选择“自动检测”
# Under the waiting title, when Settings is on Auto-detect.
connection-waiting-switch-on = 打开万用表的数据传输，或在“设置”(⚙) 中选择型号

## A failed connection: a title naming the problem, the steps to try, and a
## closing line.

# The closing line under the steps. "Connect" is the top bar's button.
connection-click-connect = 解决问题后，单击“连接”。
# The closing line after the Bluetooth steps when nothing answered at the
# address given with --adapter, a command-line option kept as it is. "It"
# is the scan command the steps above name.
connection-restart-with-address = 将 --adapter 设为它打印出的地址，然后重新启动。

# A meter that connects over Bluetooth only, and Bluetooth was not looked at.
connection-bluetooth-not-searched = 未查找蓝牙设备
# Under that title when Bluetooth is switched off. $error is the sentence
# saying why it was not searched; $hint says how to switch it
# on.
connection-bluetooth-not-searched-off = { $error }。{ $hint }
# Under that title otherwise. $error is the sentence saying why.
connection-bluetooth-not-searched-why = { $error }。

# --adapter, a command-line option, named a USB device that isn't there.
connection-adapter-not-found = 找不到适配器
# $selector is the value given with --adapter.
connection-adapter-not-matched = 没有设备与 --adapter '{ $selector }' 匹配。
# After the list of what is plugged in.
connection-adapter-restart = 使用正确的 --adapter 值重新启动。

# The cable or Bluetooth link is up but no meter answered. $link is usb (a
# USB cable), adapter (a Bluetooth adapter plugged into the meter) or radio
# (the meter's own Bluetooth). A list of meters and their steps follows the
# colon.
connection-not-identified =
    { $link ->
        [adapter] 蓝牙适配器已连接，但没有识别到万用表。
        [radio] 蓝牙链路已建立，但没有识别到万用表。
       *[usb] USB 连接线已连接，但没有识别到万用表。
    }

    打开万用表的数据传输，或在“设置”(⚙) 中选择型号:
# After that list.
connection-not-identified-report = 已在传输数据但仍无法识别? 在“设置”(⚙) 中选择它的型号，并请报告此问题。

# The meter stopped answering. $link is usb (a USB cable), adapter (a
# Bluetooth adapter plugged into the meter), radio (the meter's own
# Bluetooth) or none (a simulated meter, on no link). $model is the meter
# picked, e.g. UNI-T UT61E+; $steps are its own steps for switching data
# transmission on, several lines.
connection-no-response =
    { $link ->
        [usb]
            USB 连接线已连接，但万用表
            没有响应 (已选择 { $model })。
        [adapter]
            蓝牙适配器已连接，但万用表
            没有响应 (已选择 { $model })。
        [radio]
            蓝牙链路已建立，但万用表
            没有响应 (已选择 { $model })。
       *[none]
            连接已建立，但万用表
            没有响应 (已选择 { $model })。
    }

    如果设备选错了，请在“设置”(⚙) 中更改，或在其中选择“自动检测”。
    否则，开启数据传输:
    { $steps }
# The same, on Auto-detect: no meter is picked to name.
connection-no-response-auto =
    打开万用表的数据传输，
    或在“设置”(⚙) 中选择型号。

# A meter that stopped answering in the middle of a session. $title is the
# shared notice title, "No response from meter".
connection-no-response-check = { $title }，请检查设备选择和数据传输
# The reading thread stopped by itself: it crashed, or gave up connecting.
connection-acquisition-stopped = 采集意外停止，请重新连接以继续

# The last line of a notice for a meter whose support isn't fully verified;
# a link. $warning is the sentence saying how far it is verified.
connection-feedback = { $warning }提供反馈。
    .hover = 打开 GitHub 议题页面以提供反馈

## The notice under the reading when the Bluetooth link delivers readings
## two at a time, which can end with a command that fixes it.

# Copies the command to the clipboard.
connection-late-copy = 复制
# A toast, once the command is copied.
connection-late-copied = 命令已复制

## Toasts.

# A meter identified itself while Settings was on Auto-detect. $meter is
# its name, e.g. UNI-T UT61E+. $saved is yes when Settings now names it, no
# for a session that detects every time.
connection-detected =
    { $saved ->
        [yes] 已检测到 { $meter }，并保存为你的设备。在“设置”中选择“自动检测”可重新探测。
       *[no] 已检测到 { $meter }
    }
# The same, when the name the meter gave for itself differs; $meter and
# $saved as there. $reported is that name, e.g. UT216XD.
connection-detected-reports =
    { $saved ->
        [yes] 已检测到 { $meter } (万用表报告的名称为“{ $reported }”)，并保存为你的设备。在“设置”中选择“自动检测”可重新探测。
       *[no] 已检测到 { $meter } (万用表报告的名称为“{ $reported }”)
    }
# The meter named another model than the one picked in Settings. $reported
# is the name it gave, e.g. UT161E; $picked the model picked, $named the
# model that name belongs to, e.g. UNI-T UT161E.
connection-other-model = 万用表报告的名称为“{ $reported }”，与所选的 { $picked } 不符。请在“设置”中选择 { $named } 或“自动检测”。
# Detection settled on a meter, then the meter named another. $reported is
# the name it gave, $named the model it belongs to. $saved is yes when
# Settings now names that model.
connection-named-after-detection =
    { $saved ->
        [yes] 万用表报告的名称为“{ $reported }”，已保存为你的设备: 请重新连接，以按 { $named } 读取。
       *[no] 万用表报告的名称为“{ $reported }”: 请重新连接，以按 { $named } 读取。
    }

# The recording stopped because Buffer size, in Settings, was lowered below
# what it held. $count is how many samples it took, $kept the same number
# written short, e.g. 1.2K. Export… is a button.
connection-buffer-shrunk =
    { $count ->
       *[other] 记录已停止，其中 { $kept } 个样本已保留，单击“导出…”可保存
    }
# The recording filled the buffer. $count is its size in samples, $kept the
# same number written short, e.g. 500K.
connection-buffer-full =
    { $count ->
       *[other] 记录已停止: 缓冲区已满 ({ $kept } 个样本)
    }

# A button command sent to the meter failed. $command is its name as the
# meter's protocol knows it, e.g. hold; $error says why.
connection-command-failed = 命令 '{ $command }' 失败: { $error }
# Switching a meter setting failed. $setting is mode, range or the
# setting's lowercase name as the command line takes it (hold, rel, minmax,
# peak); $error says why.
connection-switch-failed =
    { $setting ->
        [mode] 切换模式失败: { $error }
        [range] 切换量程失败: { $error }
       *[other] 切换 { $setting } 失败: { $error }
    }

# The × that closes a toast or a notice.
connection-dismiss =
    .hover = 关闭
    .a11y = 关闭通知
