### What 'dmm-cli read' prints besides the readings: why it refuses to
### start, the notes as it starts and runs, and the summary when it ends.
### These lines go to the terminal's error stream; the readings themselves,
### and any file they are written to, stay as they are.
###
### Words in --flag form, and their values ('max', replay), are typed by the
### user and stay as they are, as do unit symbols (V, Ω, s). Command lines
### arrive as variables.

## Refusals before anything is opened, after "Error:" (common.ftl).

# An option, or several, then why its value can't be used. $flags is the
# option as typed, e.g. --alarm-hysteresis, or several of them, e.g.
# "--alarm-low, --alarm-high"; $reason is the reason, in lower case.
read-flag-refused = { $flags }: { $reason }
read-alarm-bell-needs-limit = --alarm-bell 需要 --alarm-high 或 --alarm-low
# The hysteresis band says how far back a reading must come before a limit
# alarms again, so it needs a limit.
read-alarm-hysteresis-needs-limit = --alarm-hysteresis 需要 --alarm-high 或 --alarm-low
# --mock-clock-scale and --mock-clock-preseed are contributor options that
# speed a simulated run up: 'max' plays a recording as fast as it decodes, a
# preseed starts a run with readings already behind it. After
# read-flag-refused's options.
read-clock-max-no-preseed = 'max' 不接受预置读数: 每一帧本来就会立即回放
read-clock-max-needs-replay = 'max' 需要 --replay: 只有回放文件才会自行结束
# A recording names the meter it was made with. $command is the command line
# that plays one (dmm-cli read --replay FILE).
read-replay-names-its-device = --replay 的文件中已指明万用表；请去掉 --device ({ $command })
# An exported CSV or JSON file names the meter it was read from.
read-import-names-its-device = --import 的文件中已指明万用表；请去掉 --device
# --format replay writes the frames a meter sent. $device is the simulated
# meter's id, e.g. mock.
read-replay-format-needs-meter = --format replay 需要真实万用表；--device { $device } 没有可记录的内容
# $flag is the option given that would change the readings, e.g. --scale or
# --integrate.
read-replay-format-keeps-frames = 回放文件保存的是万用表自己的帧；回放时请传入 { $flag }
read-import-no-frames = CSV 或 JSON 文件不含万用表的帧，无法写成回放文件；需要时请用 --import 读取回放文件
read-import-no-interval = --interval-ms 按万用表发送的节奏保留读数；文件中的读数已是全部，无法再抽取

## Why --mock-clock-scale's value was refused, at the end of a line from the
## command-line parser, which stays English: "error: invalid value 'abc' for
## '--mock-clock-scale <MOCK_CLOCK_SCALE>': …".

# $value is what was typed.
read-clock-scale-invalid = 应为数字或 'max'，实际为 '{ $value }'

## Notes as the run starts, each after "Note:" (common.ftl).

# --format named one format and the -o file's name another; the format wins.
# $format is the format given, e.g. csv; $path is the file.
read-format-mismatch = 已按 --format { $format } 写入 { $path }
# A recording played into a file takes as long as the recording did. $flag
# is the option that skips the wait (--mock-clock-scale max).
read-paced-replay = 按记录的节奏回放；添加 { $flag } 可不等待直接转换
# --scale, --offset and --unit apply arithmetic to every reading. $transform
# is that arithmetic, e.g. ×2 - 1 → A.
read-scaled = 读数已在软件中换算 ({ $transform })
# $limits is what the alarm watches for (alarm-limits in lib-alarm.ftl),
# e.g. "above 5 or below 3, hysteresis 1%".
read-alarm-on = 报警已启用: { $limits }

## While the readings arrive.

# After "Note:". The statistics (and with --integrate the integral) start
# again when the readings change what they measure. $change is mode (the
# dial was turned), unit (the range moved), or either of them followed by
# -integral when the integral restarts too. $before and $after are the mode
# names, e.g. DC V and Ω, or the units, e.g. mV and V.
read-series-reset =
    { $change ->
        [mode-integral] 模式已更改 ({ $before } → { $after })，统计和积分已重置
        [unit] 单位已更改 ({ $before } → { $after })，统计已重置
        [unit-integral] 单位已更改 ({ $before } → { $after })，统计和积分已重置
       *[mode] 模式已更改 ({ $before } → { $after })，统计已重置
    }
# After "Note:". The readings are now of another quantity than the alarm's
# limits, which it stops judging them against. $now is the readings' unit,
# $watched the limits' unit, e.g. Ω and V.
read-alarm-idle = 报警待机: 读数的单位是 { $now }，限值的单位是 { $watched }
# After "Note:". The readings are back in the limits' unit, $watched, e.g. V.
read-alarm-watching = 报警恢复监视: 读数单位已恢复为 { $watched }
# Red and bold, opening the line that says a reading crossed a limit; the
# terminal may ring. Keep the colon, or your language's equivalent: the
# sentence follows after a space.
read-alarm-label = 报警:
# After "Alarm:". $breach says which limit was crossed (alarm-breach in
# lib-alarm.ftl), e.g. "Above high limit 5 V"; $reading is the reading with
# its unit, e.g. 5.012 V.
read-alarm-breach = { $breach } ({ $reading })
# After "Warning:". A frame the meter sent couldn't be read and the run goes
# on without it. $error says why (lib-errors.ftl); $count is how many have
# been skipped so far.
read-unreadable-frame = { $error } (目前已跳过 { $count } 帧)
# After "Error:" (common.ftl), ending a --format replay run: a reading came
# with no frame from the meter behind it, which a replay can't hold. $mode is
# the reading's mode name (lib.ftl), e.g. DC V.
read-replay-no-frame = { $mode } 读数没有对应的万用表帧，无法记入回放文件

## The summary when the run ends, under a blank line.

# After "Note:". $count is how many readings couldn't be read.
read-skipped-frames =
    { $count ->
       *[other] 已跳过 { $count } 个读数 (帧无法读取)
    }
## After a dim "---": the statistics of the last series of readings, since
## the mode or the unit last changed, one cell per message, joined by " | ":
##
##     --- 3 readings | Min: 0.0264 V | Max: 1.6112 V | Mean: 0.6140 V | σ: 0.8682 V
##
## Min, Max and Mean are the statistics' names in dmm-gui too.

# $count is how many readings.
read-summary-count =
    { $count ->
       *[other] { $count } 个读数
    }
# $min is a number with its unit, e.g. 4.0010 V; the same for the next three.
read-summary-min = 最小值: { $min }
# $max: the largest reading.
read-summary-max = 最大值: { $max }
# $mean: the readings' mean.
read-summary-mean = 平均值: { $mean }
# $sd: the readings' standard deviation, σ. Left out after a single reading.
read-summary-sd = σ: { $sd }
# Under the summary, with --integrate: the readings summed over time. $value
# is the integral with its unit, e.g. 0.7720 V·s.
read-integral = 积分: { $value }
# The same, with how long the integral ran: $value as there, and $seconds,
# e.g. 1.8, before "s", the symbol for seconds.
read-integral-timed = 积分: { $value } ({ $seconds }s)
# After "Note:", under the integral: gaps in the readings the integral leaves
# out. $count is how many; $max the longest interval it bridges, in seconds
# (s). The rule's other figures, 5 intervals and 2 s, are fixed.
read-skipped-intervals =
    { $count ->
       *[other] 已跳过 { $count } 个间隔 (超过 { $max } s 没有读数，该时长为 5 个采样间隔，且不少于 2 s；积分不完整)
    }
# Under the summary: how many times the readings crossed each limit.
# $limits is both, high or low: the limits the alarm had. $above and $below
# are the counts; $high and $low the limits with their unit, e.g. 5 V.
read-alarm-counts =
    { $limits ->
        [both] 报警: 高于 { $high } 共 { $above } 次，低于 { $low } 共 { $below } 次
        [high] 报警: 高于 { $high } 共 { $above } 次
       *[low] 报警: 低于 { $low } 共 { $below } 次
    }
# -o without a file name: the run named the file after its first reading.
# $path is the file.
read-written-to = 已写入 { $path }
# -o without a file name, and no reading came to name the file after.
read-nothing-written = 没有收到读数，因此未写入文件
