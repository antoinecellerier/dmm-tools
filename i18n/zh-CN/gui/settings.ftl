### The settings panel, which the ⚙ button at the top bar's right end opens
### under the bar (assets/gui-settings.webp): rows of checkboxes and of
### chips, each row led by its caption, in groups a rule apart. The colour
### and theme rows have a file of their own.
###
### Rows wrap on a narrow window, so keep the chips short. $keys and the
### other shortcut variables are keyboard shortcuts as this computer writes
### them, e.g. Ctrl+O or ⌘O. Text in parentheses starting with -- names a
### command-line option, kept as it is, that set the row for this session
### only.

## The meter: which one, how the mock runs, and how a connection is made.

# The Device row's caption; .a11y names its dropdown for a screen reader.
settings-device = 设备:
    .a11y = 设备
# The Device dropdown, and its first entry, when no particular meter is
# picked: each kind of meter is tried in turn.
settings-auto-detect = 自动检测
# The Device dropdown while --device picked the meter for this session.
# $name is the meter, e.g. UNI-T UT61E+, or Auto-detect.
settings-device-pinned = { $name } (--device)
# Beside the greyed Device dropdown while a recording is played back:
# --replay is the command-line option that opened it.
settings-device-replay = (不带 --replay 重新启动后才能选择万用表)
# Beside the greyed Device dropdown while the session runs on a simulated
# clock, set by command-line options for testing.
settings-device-clock = (不带时钟选项重新启动后才能选择万用表)

## The open Device dropdown: Auto-detect, then every meter under its brand.

# An entry's hover and screen-reader name. $name is the meter, e.g. UNI-T
# UT61E+; $tag how far its support is verified, e.g. partly verified.
settings-device-entry = { $name } ({ $tag })

## The Mock mode row, shown while a simulated meter is picked. Its other
## chips are the simulated modes' names, kept as they are.

# The first chip: the simulated meter steps through every mode. $pinned is
# yes when --mock-mode picked it for this session, no otherwise.
settings-mock-auto =
    { $pinned ->
        [yes] 自动 (循环) (--mock-mode)
       *[no] 自动 (循环)
    }
    .hover = 依次循环切换所有合成模式，以测试 GUI
# The chip for the mode --mock-mode picked for this session. $mode is its
# name, e.g. dcv.
settings-mock-mode-pinned = { $mode } (--mock-mode)
settings-mock-mode = 仿真模式:

## How a connection is made: checkboxes on one row.

settings-auto-connect = 启动时自动连接
    .hover = 应用启动时自动打开 USB 连接
# Asking the meter its name makes it beep.
settings-device-name = 连接时显示设备名称 (会蜂鸣)
    .hover = 连接后查询万用表的名称，万用表会蜂鸣一声
# The Bluetooth checkbox while --no-bluetooth switched the search off for
# this session. $setting is the checkbox's own label, e.g. Look for
# Bluetooth devices.
settings-bluetooth-pinned = { $setting } (--no-bluetooth)
settings-bluetooth =
    .hover = 没有 USB 连接线响应时，查找范围内的蓝牙适配器或万用表。下次连接时生效。

## What is kept of the readings.

settings-sample-interval = 采样间隔:
# The first Sample interval chip; the others are durations, e.g. 300ms or 2s.
settings-interval-every-reading = 每个读数
    .hover = 按万用表自身的节奏保留它产生的每个读数
# The other Sample interval chips' hover. $unit is second for exactly one
# second, seconds for a whole number of seconds and ms otherwise; $n is how
# many seconds or milliseconds. s and ms are unit symbols.
settings-interval-chip =
    .hover =
        { $unit ->
            [second] 每秒最多保留 1 个读数，即最接近各时间点的读数
            [seconds] 每 { $n } s 最多保留 1 个读数，即最接近各时间点的读数
           *[ms] 每 { $n } ms 最多保留 1 个读数，即最接近各时间点的读数
        }

settings-buffer-size = 缓冲区大小:
# A Buffer size chip's hover; the chips are sample counts written short,
# e.g. 500K. $count is the chip's number of samples and $samples the same
# number written short. $memory is what they take, e.g. ≈194 MB, and $span
# how long they last, e.g. 14 h. $pace is every-reading when the Sample
# interval keeps every reading, whose pace the estimate can't know, and
# interval otherwise.
settings-buffer-size-chip =
    .hover =
        { $pace ->
            [every-reading]
                { $count ->
                   *[other] 图表中最多保留 { $samples } 个样本供显示和导出: 占用 { $memory }，按每秒 10 个读数约可容纳 { $span }。同时保留的已停止记录最多可再占用同样多的空间
                }
           *[interval]
                { $count ->
                   *[other] 图表中最多保留 { $samples } 个样本供显示和导出: 占用 { $memory }，按当前采样间隔约可容纳 { $span }。同时保留的已停止记录最多可再占用同样多的空间
                }
        }
# After the Buffer size chips: what the picked size costs. $memory is the
# memory it takes, e.g. ≈194 MB; $span how long it lasts, e.g. 14 h.
settings-buffer-cost = ({ $memory }，约 { $span })

## Which panels show, and what the Specifications panel shows: checkboxes
## on one row.

settings-graph = 图表
    .hover = 显示滚动的时间序列图
# σ is the standard deviation, as the Statistics panel writes it.
settings-statistics = 统计
    .hover = 显示实时会话的最小值、最大值、平均值、σ 和积分
settings-recording = 记录
    .hover = 显示记录控件和样本日志
# The Specifications panel's checkbox. $fields is shown while the panel is
# on and the checkboxes for its fields follow it, hidden otherwise.
settings-specs =
    { $fields ->
        [shown] 技术指标:
       *[hidden] 技术指标
    }
    .hover = 显示当前模式的准确度和分辨力
# The Specifications fields, as that panel names them.
settings-specs-resolution = 分辨力
    .hover = 在技术指标中显示分辨力
settings-specs-accuracy = 准确度
    .hover = 在技术指标中显示准确度
# Input Z is the input impedance.
settings-specs-input-z = 输入阻抗
    .hover = 在完整的技术指标面板中显示输入阻抗
settings-specs-notes = 备注
    .hover = 在完整的技术指标面板中显示手册中的备注

## The window.

# The Language row's caption, shown once the app speaks more than one
# language. In every language but English, keep the English word in
# parentheses after yours, e.g. Langue (Language):, so someone who can't
# read the window can still find the row. .a11y names its dropdown for a
# screen reader, in your word alone.
settings-language = 语言 (Language):
    .a11y = 语言
    .hover = 界面使用的语言
# The dropdown's first entry: the window speaks the system's language.
# $language is the language that is, by its name in itself, e.g. Français
# or 简体中文. The other entries are each language's name in itself, kept
# as they are.
settings-language-system = 系统 ({ $language })
    .hover = 跟随系统语言
# Weak text after your language's name in the dropdown while no native
# speaker has reviewed it yet. Shown only next to your own language's name,
# whatever language the window speaks, so its speakers can read it: e.g.
# Português (Brasil)  tradução automática.
settings-language-machine-translated = 机器翻译
# A screen reader's name for that entry, also in your language whatever the
# window speaks. $language is your language's name in itself, e.g.
# Português (Brasil).
settings-language-machine-translated-entry = { $language }，机器翻译
# The weak line under the Language row while such a language is spoken,
# then a link that opens the translators' guide on GitHub in the web
# browser; ↗ marks a link.
settings-language-unreviewed = 机器翻译，尚未审校。
settings-language-review-link = 帮助审校 ↗
    .hover = 打开 GitHub 上的翻译指南
# A toast when Chinese is picked and the computer has no font for it. It
# is always shown in English, which the app's own fonts draw: no need to
# translate it.
settings-language-no-chinese-font = No font for Chinese characters was found. Install a Simplified Chinese font, such as Noto Sans CJK SC, then restart dmm-gui.
settings-language-no-japanese-font = No font for Japanese characters was found. Install a Japanese font, such as Noto Sans CJK JP, then restart dmm-gui.

# The Zoom chips are percentages, e.g. 150%.
settings-zoom = 缩放:
# A Zoom chip's hover. $level is its scale in percent, e.g. 150; $step the
# keys that step the zoom.
settings-zoom-chip =
    .hover = 将 GUI 缩放到 { $level }% (用 { $step } 逐级调整)
# The 100% chip's hover. $reset is the keys that go back to 100%, $step
# those that step the zoom.
settings-zoom-reset-chip =
    .hover = 将 GUI 缩放到 100% (用 { $reset } 恢复，用 { $step } 逐级调整)
# Window decorations are the title bar and border.
settings-hide-decorations = 隐藏标题栏和边框
    .hover = 无边框窗口: 用 { $keys } 切换回来
# On Wayland the checkbox is greyed: .wayland is its hover there, and
# settings-always-on-top-wayland the note beside it. Keep both the same as
# shortcuts-always-on-top-toast, which Ctrl+T shows there.
settings-always-on-top = 窗口置顶
    .hover = 让窗口保持在其他桌面窗口之上 ({ $keys })
    .wayland = 无法在 Wayland 上使用: 右键单击标题栏可让窗口保持在其他窗口之上
# The note after the greyed checkbox on Wayland, in small grey text.
settings-always-on-top-wayland = (无法在 Wayland 上使用: 右键单击标题栏可让窗口保持在其他窗口之上)

## The update check, in builds downloaded from GitHub.

settings-check-updates = 检查新版本
    .hover = 每天向 GitHub 查询一次是否有新版本。GitHub 会看到你的 IP 地址，不会发送任何有关万用表的信息。
# The link the top bar shows when a newer release is out; ↗ marks a link.
# $version is the release, e.g. v0.10.0.
settings-update-release = { $version } 已发布 ↗
# The same for a newer nightly build, which has no version number.
settings-update-dev = 更新的开发版本 ↗
# Its shorter form, when the top bar is short of room.
settings-update-dev-short = 更新 ↗

## How a screen reader hears the reading change.

# A screen reader speaks the reading each time it changes; this row sets how
# often at most. Its chips are durations like the Sample interval row's,
# Every reading taking that row's first chip's label, then Never.
settings-announce = 屏幕阅读器播报:
settings-announce-every-reading =
    .hover = 播报每个新读数。如果读数更新快于屏幕阅读器的朗读速度，播报可能会被中途打断。
# The duration chips' hover. $n is a whole number of seconds.
settings-announce-chip =
    .hover = 最多每 { $n } 秒播报一次读数。模式改变时立即播报。
settings-announce-never = 从不
    .hover = 读数变化时不播报。连接断开时仍会播报。
# A checkbox at the end of the same row, or under it when the row wraps.
# Focus is keyboard focus, or where the screen reader's cursor is.
settings-announce-focused = 仅在读数获得焦点时
    .hover = 仅在读数获得焦点时，按选定的频率播报读数。按 Tab 键可将焦点移到读数上。

## How the graph's lines look, the last row.

settings-graph-lines = 图表线条:
# Sub-value lines are the extra traces a meter that shows several values
# at once draws beside the main one.
settings-graph-lines-patterned = 虚线
    .hover = 副读数曲线用虚线和点线绘制，无需靠颜色区分
settings-graph-lines-solid = 实线
    .hover = 连续的线条，靠颜色和图例区分
# The file could not be understood, so a copy of it was kept before it is
# written over. $path is the copy's full path, e.g.
# /home/ana/.config/dmm-tools/settings.json.bad.
settings-load-failed-kept = 无法读取设置文件，已改用默认设置。已将该文件的副本保存为 { $path }。
# The file could not be opened at all. $path is its full path, e.g.
# /home/ana/.config/dmm-tools/settings.json.
settings-load-failed = 无法读取设置文件 { $path }，已改用默认设置。
