### What 'dmm-cli capture' prints around its steps: the guided run a user
### makes to send a report about their meter. It walks the meter through a
### list of steps (set the dial to DC V, press HOLD…), reads what the meter
### sends at each, and writes a report file to attach to an issue on GitHub.
### These lines go to the terminal's error stream.
###
### Each step's own instruction, the readings (5.5687 V [AUTO]) and the
### report file stay as they are, in English: a maintainer reads the report.
### A user may translate the instructions for themselves: see
### --export-instructions below.
### Step ids (dcv, hold_on, extra_0), setting names (range, hold) and words
### in --flag form are typed or read back by those names and stay as they
### are. The keys the user presses to answer (s, q, r, n, y) arrive as
### variables: the run only understands those letters, so they can't be
### changed. "Enter" is the key's name and can be translated.

## Checking that the meter answers, before anything else.

# Dim, while the tool asks the meter for its name.
capture-checking = 正在检查与万用表的通信…
# Yellow and bold. The cable or Bluetooth link opened, but the meter sent
# nothing. $link is usb (a USB cable), adapter (a Bluetooth adapter plugged
# into the meter), radio (the meter's own Bluetooth) or other.
capture-not-responding =
    { $link ->
        [usb] USB 连接线已连接，但万用表没有响应。
        [adapter] 蓝牙适配器已连接，但万用表没有响应。
        [radio] 蓝牙链路已建立，但万用表没有响应。
       *[other] 链路已建立，但万用表没有响应。
    }
# Followed by the meter's own steps for switching its data transmission on.
capture-enable-transmission = 开启数据传输:
capture-run-again = 然后再次运行此命令。
# What the meter sent was kept in a file of its own, as evidence. $bytes is
# how many bytes the meter sent, $path the file.
capture-no-response-saved = 收到的字节数: { $bytes }，已保存到 { $path }
# $url is the web address of the meter's issue on GitHub.
capture-no-response-attach = 如果万用表已开机但仍没有响应，请将该文件附加到 { $url }
# $error is why the file could not be written.
capture-no-response-not-saved = 无法保存收到的字节: { $error }

## The banner once the meter answers, after the lines naming the meter
## (connect.ftl). Each support level is what follows "Status:"; the line
## under the partly verified and experimental ones is indented under it.

# $status is one of the capture-status-* levels below, coloured.
capture-status = 状态: { $status }
capture-status-verified = 受支持的型号
capture-status-partly-verified = 部分验证，连接和主要模式已确认
capture-status-partly-verified-detail = 其余模式尚未确认，本次运行正是为此。
# The protocol was worked out from the meter's traffic, not from its maker.
capture-status-experimental = 实验性，协议通过逆向工程得出，尚未确认
capture-status-experimental-detail = 请尽量多完成一些步骤，并分享报告。

## A report from an earlier run is already in the file: resume it, start
## again or stop. Each prompt waits for one key, after a space.

# $path is the report file. $captured and $skipped count its steps.
capture-found-existing = 找到已有的捕获: { $path } (已捕获 { $captured } 个，已跳过 { $skipped } 个)
# $resume, $fresh and $abort are the keys that answer: r, n and q.
capture-resume-prompt = { $resume }=继续，{ $fresh }=重新开始，{ $abort }=中止:
capture-aborted = 已中止。
# $yes and $no are the keys that answer: y and n.
capture-overwrite-confirm = 这将覆盖已有的捕获。确定吗? { $yes }/{ $no }:
capture-resuming = 正在继续: 已捕获的步骤将被跳过。
# The file is there but isn't a report this tool can read. $path is the
# file.
capture-unreadable = 找到 { $path }，但无法解析。
# $yes is the key that answers: y.
capture-overwrite-unreadable = 是否覆盖? { $yes }=重新开始，其他任意键=中止:
# $path is the report file the run writes.
capture-output-file = 输出文件: { $path }

## What the steps ask for besides the meter and its test leads, listed
## before the first step so the user can gather it all at once. Each item is
## on a line of its own after its number in brackets.

capture-needs-heading = 需要准备:
# $item is one of the capture-need-* items below; $steps lists the ids of
# the steps that need it, e.g. "dcv_short, ohm_short".
capture-needs-item = { $item }  (步骤: { $steps })
# The user types the numbers in brackets of what they don't have, or just
# presses Enter.
capture-needs-prompt = 缺少的物品编号 (Enter = 全都有):
# What the user typed at a numbers prompt (this one or the review's) isn't
# one of the numbers listed. $token is what they typed, in quotes; $max the
# last number listed.
capture-not-a-number = { $token } 不是 1 到 { $max } 之间的数字
capture-need-shorted-leads = 短接的表笔
capture-need-dc-source = 电池或其他直流电源
capture-need-thermocouple = K 型热电偶
# For the non-contact voltage detector, which senses a live wire nearby.
capture-need-live-wire = 附近带电的市电火线
# The meter's power-measurement adapter, plugged into a live outlet with a
# load plugged into it.
capture-need-power-adapter = 功率测量适配器、通电的插座和负载
capture-need-transistor = 三极管
capture-need-scr = SCR (晶闸管)
capture-need-loop-source = 4-20 mA 环路及其 15-48 V 电源
capture-need-power-module = 直流或 USB 功率测量模块、电源和负载
# LX-925 is the adapter's name.
capture-need-pv-adapter = LX-925 辐照度适配器

## The steps. Each opens with its id in brackets and its instruction, and
## the tool watches the meter until it shows what the step asks for.

# Bold, between heavy lines.
capture-modes-heading = 测量模式
# Dim, under the heading. $skip and $quit are the keys: s and q.
capture-modes-keys = 万用表读数稳定后，每个步骤会自动捕获: Enter=立即捕获，{ $skip }=跳过一步，{ $quit }=跳到最后并保存
# Dim, after a step's instruction: the step the rest of the run relies on.
capture-gate-mark = (前提步骤)
# A resumed run passes over a step the report already has. $step is the
# step's id.
capture-already-captured = { $step } 已捕获，跳过
# Dim, once the tool has waited a while. $skip and $quit are the keys: s and
# q.
capture-press-enter = 万用表就绪后按 Enter ({ $skip }=跳过，{ $quit }=结束)
# When the terminal can't be read key by key. $skip and $quit are the keys:
# s and q.
capture-any-key = 任意键=捕获，{ $skip }=跳过，{ $quit }=结束:
# Red: the button the step presses on the meter by remote control was
# refused. $error is why.
capture-command-failed = 命令失败: { $error }
# Yellow: the tool pressed a button on the meter by remote control and the
# meter didn't change. $command is the button's name as the tool sends it,
# e.g. hold; $reading what the meter shows, e.g. 5.5687 V [AUTO].
capture-did-nothing-shows = { $command } 没有效果；万用表仍显示 { $reading }
# $command is the button's name as the tool sends it, e.g. hold.
capture-did-nothing-silent = { $command } 没有效果；万用表没有发送读数
# Yellow: the meter changed function while the step was reading it, so the
# step starts over. $mode is the function the step is for, $other the one
# the meter went to, e.g. Ω and Continuity.
capture-left-mode = 采样时万用表从“{ $mode }”切换到了“{ $other }”，正在重新采集…
# The reading the step took, in green. $reading is it, e.g. 5.5687 V [AUTO].
capture-we-read = 读取结果: { $reading }
# Something the meter sent couldn't be read; the step goes on. $error is
# why.
capture-frame-error = 错误: { $error }
# Yellow: the step read nothing from the meter.
capture-no-response = 万用表没有响应。
# Yellow, before capture-few-samples on the same line.
capture-few-samples-label = 警告:
# The step got fewer readings than it asks for. $got and $wanted count them.
capture-few-samples = 只采集到 { $got }/{ $wanted } 个样本

## Dim, while a step waits: the meter settled, but not in what the step
## expects. Each says what the reading has and what the step wants.

# $got and $want are functions, in quotes, e.g. "DC V" and "AC V".
capture-mismatch-mode = 万用表显示: 模式为 { $got }，需要 { $want }
# $flag is a status flag the meter shows, e.g. HOLD. $want says whether the
# step wants it on or off; the meter has it the other way.
capture-mismatch-flag =
    { $want ->
        [on] 万用表显示: { $flag } 为关，需要开
       *[off] 万用表显示: { $flag } 为开，需要关
    }
# $range is the reading's range, e.g. 22V. $shown is none (the reading has
# no range), auto (a range the meter picked itself) or plain. $want is auto
# (the step wants the meter to pick the range itself) or manual (the user
# picks it).
capture-mismatch-range =
    { $want ->
        [auto]
            { $shown ->
                [none] 万用表显示: 量程为 (无)，需要自动量程
                [auto] 万用表显示: 量程为 { $range } (自动)，需要自动量程
               *[plain] 万用表显示: 量程为 { $range }，需要自动量程
            }
       *[manual]
            { $shown ->
                [none] 万用表显示: 量程为 (无)，需要手动量程
                [auto] 万用表显示: 量程为 { $range } (自动)，需要手动量程
               *[plain] 万用表显示: 量程为 { $range }，需要手动量程
            }
    }
# $value is what the meter shows, e.g. 12.345 or OL; $shown is absent when
# the reading has no main value, else value. $want is the kind of reading the
# step wants: overload (OL, the meter's over-range sign), negative, finite
# (any number), ncv (the non-contact voltage detector's level 1 or more),
# word (a word the meter shows instead of a number) or zero.
capture-mismatch-value =
    万用表显示: { $shown ->
        [absent] 没有主读数
       *[value] 值为 { $value }
    }，需要{ $want ->
        [overload] 超量程 (OL)
        [negative] 负读数
        [ncv] 至少为 1 的 NCV 等级
        [word] 显示文字而非读数
        [zero] 读数为零
       *[finite] 数值读数
    }
# $value and $shown as in capture-mismatch-value. $min is the smallest
# number the step wants, positive or negative, e.g. 1.
capture-mismatch-at-least =
    万用表显示: { $shown ->
        [absent] 没有主读数
       *[value] 值为 { $value }
    }，需要绝对值至少为 { $min }

## Asking whether the reading matches the meter's screen. Each prompt waits
## for a line, after a space.

# Dim. $retake is yes when the step can be done over. $no and $redo are the
# keys that answer: n and r.
capture-confirm =
    { $retake ->
        [yes] 万用表显示的是这个吗? (Enter = 是，{ $no } = 否，{ $redo } = 重新采集)
       *[no] 万用表显示的是这个吗? (Enter = 是，{ $no } = 否)
    }
# Yellow, when the answer was none of those. $retake, $no and $redo as in
# capture-confirm.
capture-confirm-again =
    { $retake ->
        [yes] 请回答 { $no }、{ $redo }，或按 Enter。
       *[no] 请回答 { $no }，或按 Enter。
    }
# The reading didn't match: the user types what the meter showed.
capture-what-shown = 万用表显示的是什么?
# Dim: the user typed nothing at capture-what-shown.
capture-recorded-mismatch = 已记录: 读数不符
# Dim. $shown is what the user typed, in quotes.
capture-recorded-shown = 已记录: 万用表显示 { $shown }
# Dim: the step starts over.
capture-retaking = 正在重新采集…

## After the first steps, which check that the tool reads the meter right
## ("core semantics": mode, range and flags).

# Green.
capture-core-confirmed = 核心语义已确认，其余步骤将在最后统一复核。
# Yellow. $steps lists the ids of the steps that failed, e.g. "dcv, ohm".
capture-core-not-confirmed = 核心语义未确认 ({ $steps })，每个步骤都会要求确认。

## Once the core semantics are confirmed, the tool sets ranges and flags on
## the meter itself by remote control, and prints a dim line for each. Each
## starts with an arrow.

# $mode is the function, e.g. Continuity.
capture-drive-switching = 正在切换到“{ $mode }”(由工具发送)
# $mode is the function, e.g. Continuity; $error why the meter refused.
capture-drive-switch-failed = 无法切换到“{ $mode }”: { $error }，请手动切换
# $setting is the setting's name, rel (relative mode): the meter refuses it
# while it shows OL, its over-range sign.
capture-drive-rel-skipped-ol = 已跳过 { $setting }: 读数超量程 (OL)
# $setting is the setting's name, rel (relative mode).
capture-drive-rel-skipped-blank = 已跳过 { $setting }: 没有读数
# In place of a reading, after the setting and value tried, e.g.
# "range:22V".
capture-drive-no-reading = 没有读数
# Yellow. $setting is the setting's name, e.g. range or hold.
capture-drive-reset-failed = 无法复位 { $setting }，请按万用表上的对应按键
# Yellow: the meter refused too many of the tool's commands.
capture-drive-unreliable = 此万用表的遥控不可靠，不再自动扫描

## The end-of-run review: the readings the run took without asking, each
## with its number, to check against what the meter showed.

# Bold, between heavy lines.
capture-review-heading = 复核读数
# The user types the numbers of the wrong readings, or just presses Enter.
capture-review-prompt = 与万用表显示屏不符的读数编号 (Enter = 全部正确):

## The freeform pass: the user sets the meter to anything the steps didn't
## cover and describes it.

# Bold, between heavy lines; "Part 4" is the run's last part.
capture-freeform-heading = 第 4 部分: 附加捕获 (可选)
capture-freeform-intro = 将万用表设到上面未涵盖的任意模式或状态。
# After the capture's id in brackets, e.g. [extra_0]; waits for a line.
# $quit is the word that ends the pass: q.
capture-freeform-prompt = 描述万用表当前的设置 (或输入 '{ $quit }' 结束):

## The last check: does the tool's automatic detection recognise the
## meter? The user switches it off and on, and the tool detects it afresh.

# Dim. $bridge is the chip in the cable or the Bluetooth link, e.g. CP2110.
capture-detect-confirmed = 已确认可通过 { $bridge } 自动检测，因此跳过此项检查。
# Bold, between heavy lines.
capture-detect-heading = 最后一项检查: 自动检测
capture-detect-off-cable = 将万用表关机，并拔下它的 USB 连接线。
capture-detect-off = 将万用表关机。
# Waits for one key. $skip is the key that skips: s.
capture-detect-off-prompt = 关机后按 Enter，或按 { $skip } 跳过:
# $mode is the function to turn the dial to, DC V.
capture-detect-on-cable = 重新插上连接线，将旋钮转到 { $mode } 档后开机。
# $mode is the function to turn the dial to, DC V.
capture-detect-on = 将旋钮转到 { $mode } 档后重新开机。
# Followed by the meter's own steps for switching its data transmission on.
capture-detect-transmission = 如果万用表关闭了数据传输，请重新开启:
# Waits for one key. $skip is the key that skips: s.
capture-detect-ready-prompt = 就绪后按 Enter，或按 { $skip } 跳过:
# Dim.
capture-detect-checking = 正在验证自动检测…
# Detection found the meter; the tool reads it once more. $instruction is
# what to do, the step's own instruction, in English.
capture-detect-now = 现在: { $instruction }
# Dim. $skip is the key that skips: s.
capture-detect-read-keys = 万用表读数稳定后自动读取: 按 Enter 立即读取，按 { $skip } 跳过。

## The line the check ends on: dim when detection found the meter, yellow
## with advice when it didn't. $found is the model detection picked, e.g.
## UNI-T UT61E+; $reading what the meter then read, e.g. 5.5687 V [AUTO].
## The advice names the meter the run was for: $device is its id on the
## command line, e.g. ut61eplus, and $meter its name, e.g. UNI-T UT61E+.

# $reason is why the link didn't open again, in English, or
# capture-detect-no-reason.
capture-detect-not-reopened = 未检查自动检测: 链路没有重新打开 ({ $reason })。
capture-detect-no-reason = 未给出原因
capture-detect-found = 自动检测找到了 { $found }。
capture-detect-found-read = 自动检测找到了 { $found }，并读到 { $reading }。
capture-detect-found-read-other = 自动检测找到了 { $found }，但随后读到 { $reading }。请继续使用 --device { $device }，或在 dmm-gui 中选择 { $meter }。
capture-detect-found-read-nothing = 自动检测找到了 { $found }，但随后没有读到任何读数。请继续使用 --device { $device }，或在 dmm-gui 中选择 { $meter }。
capture-detect-picked-other = 自动检测为此万用表选择了 { $found }。请继续使用 --device { $device }，或在 dmm-gui 中选择 { $meter }。
capture-detect-not-found = 自动检测没有找到此万用表。请继续使用 --device { $device }，或在 dmm-gui 中选择 { $meter }。

## The end of the run.

# After "Warning:" (common.ftl): the detection check stopped. $error is why.
capture-detect-failed = 自动检测检查: { $error }
# Bold and green, between equals signs.
capture-complete = 捕获完成!
# $path is the report file, in bold.
capture-report-saved = 报告已保存到: { $path }
# The run followed a list of steps a maintainer wrote (a "plan"). $plan is
# its file; $covered and $total count its steps.
capture-plan-covered = 计划 { $plan }: 已捕获 { $total } 个步骤中的 { $covered } 个
capture-nothing-new = 没有捕获到任何未验证的步骤，因此没有新内容可报告。
# $covered and $total count the meter's steps that hardware hasn't confirmed
# yet; $meter is its name, e.g. UNI-T UT61E+.
capture-covered = 已覆盖 { $meter } 的 { $total } 个未验证步骤中的 { $covered } 个。
# $resume is the key that resumes, r (capture-resume-prompt).
capture-not-finished = 未完成: 再次运行同一命令，并回答 { $resume } 以继续。
# $flag is the option as typed, --steps <ids>.
capture-steps-hint = { $flag } 可单独运行指定的步骤。
# $url is the web address of the meter's issue on GitHub.
capture-attach = 请将报告附加到 { $url }

## --export-instructions: the step instructions written to a file the user
## translates for themselves, then passes back with --instructions. The
## file is Fluent source, like this one.

# The comment at the top of the exported file, a line of it per line here.
# $meter is the meter's name, e.g. Brymen BM829s/BM827s/BM822s/BM821s.
# Keep "=", "-device" and "#" as they are: they are the file's
# syntax.
capture-instructions-header =
    { $meter } 的捕获步骤说明，供翻译。
    将每个 "=" 之后的文字替换为你的译文，写在同一行。
    保留 "=" 之前的步骤 ID、"-device" 行和以 "#" 开头的行不变。
    删除的步骤会以英文显示。
# A line of the header, when the steps quote words from the meter: its
# key labels and what its screen shows. $marks lists them, e.g. "AC, HOLD,
# MIN/MAX, SELECT". A translation, by a person or a machine, must leave them
# as they are, so say so plainly.
capture-instructions-keep = 以下内容是万用表按键和屏幕上的原样文字，请保留不译: { $marks }
# The header's last line. $command is the command that runs the capture
# with the translated file.
capture-instructions-run = 然后运行: { $command }
# In place of writing a file, for a meter with no steps of its own (a
# simulated one). $meter is its name, e.g. Mock UT61E+.
capture-instructions-none = { $meter } 没有自己的捕获步骤: 没有需要翻译的内容。
# After writing the file. $count is the number of steps; $path the file.
capture-instructions-written = 已将 { $count } 条步骤说明写入 { $path }。
# $command is the command that runs the capture with the translated file.
capture-instructions-next = 翻译完成后运行: { $command }
# Yellow, before the first step: steps whose English changed after the
# file was exported, so they show a translation of an older wording.
# $path is the file; $steps lists the step ids, e.g. "dcv, hold".
capture-instructions-changed = 以下步骤在导出 { $path } 之后有改动，显示的是你对其旧文本的翻译: { $steps }。导出新文件即可查看新的英文原文。
# One problem in the file, on a line of its own under
# capture-error-instructions-unreadable. $line is a line number.
capture-instructions-problem-syntax = 第 { $line } 行: 应为步骤 ID、"=" 和其后的文字
# Another problem. $id is a step id, e.g. dcv.
capture-instructions-problem-not-text = { $id }: 文字中含有花括号，请删除
# Another problem. $id is a step id, e.g. dcv.
capture-instructions-problem-twice = { $id }: 该步骤出现了两次
# Another problem: the file couldn't be read at all.
capture-instructions-problem-unparsable = 无法读取该文件，请检查其中的花括号和引号

## The line that closes a failed run, after "Error:" (common.ftl).

capture-error-not-responding = 万用表没有响应
capture-error-input-gone = 键盘输入已失效；请结束捕获并重新运行
# --steps named steps this meter doesn't have. $steps lists them, e.g.
# "dcmv, tmep"; $command is the command line that lists the meter's steps.
capture-error-unknown-steps =
    未知的捕获步骤: { $steps }
    运行 { $command } 查看此设备支持的步骤。
# The --instructions file can't be read. $path is the file; $problems the
# problems, one per line (capture-instructions-problem-*).
capture-error-instructions-unreadable =
    无法将 { $path } 读取为捕获步骤说明:
    { $problems }
# The --instructions file was exported for another meter. $path is the
# file; $named the meter id in it, e.g. ut61eplus; $device this run's, e.g.
# bm82x; $command the command that exports a file for this meter.
capture-error-instructions-device = { $path } 中是 { $named } 的步骤说明，不是 { $device } 的。请用 { $command } 导出此万用表的说明。
# The --instructions file names steps this run doesn't have, most likely a
# step id that was translated. $path is the file; $ids the ids not found,
# e.g. "vcc, retencao"; $known the ids the run has, e.g. "dcv, acv, hold".
capture-error-instructions-ids = { $path } 中有本次运行没有的步骤: { $ids }。请保持每个步骤 ID 与导出时一致；可用的步骤为: { $known }。
