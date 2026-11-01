### The reading display: the meter's reading at the top of the reading
### column (assets/gui-wide-layout.webp, assets/gui-narrow-layout.webp), and
### the same reading scaled up to fill the window in the big meter
### (assets/gui-big-meter.webp, assets/gui-minimal-meter-wide.webp).
###
### The reading is the digits and the unit, then the sub-values in rows
### (Max, Min, T2, Frequency…), then the mode and range readouts and the
### meter's badges (HOLD, AUTO…). The digits, units, mode and range names,
### sub-value names and the meter's badges come from the meter or the
### library and are not in this file; these messages are what the app adds.

## The readouts under the reading: the mode, and the range beside it. Where
## the meter can be switched from the app, each is a dropdown drawn as its
## label (V DC ▾, 6V ▾) (assets/gui-reading-controls.webp).

# The mode readout as a dropdown of the modes the meter can switch to at
# its dial position.
reading-mode =
    .hover = 将万用表切换到当前档位的其他模式
    .a11y = 模式
# The range readout as a dropdown of the meter's ranges in this mode.
reading-range =
    .hover = 将万用表切换到当前模式的其他量程
    .a11y = 量程
# The mode readout on a meter that lists its function keys instead of its
# modes: the value is the caption over the open list, which says its entries
# are key presses. A screen reader still calls the dropdown Mode.
reading-keys = 万用表按键
    .hover = 按下万用表的一个功能键
    .a11y = 模式

## The badges after the meter's own, in capitals like them and as short:
## they sit on the mode line, which a narrow window wraps.

# Shown while a Scale turns the meter's reading into another quantity.
reading-badge-scaled = 换算
# Shown, in the error colour, while the reading is above the alarm's high
# limit.
reading-badge-above = 超上限
# Shown, in the error colour, while the reading is below the alarm's low
# limit.
reading-badge-below = 超下限

## What stands where the reading goes when there is none.

# Beside or under dashes (---) while no meter is connected or none has sent
# a reading; also what a screen reader says there.
reading-none = 无读数
# The big meter's second line under the connection problem that takes the
# reading's place: hovering it shows the steps, and $keys (the shortcut that
# turns the big meter off, e.g. Ctrl+B or ⌘B) brings the whole window back.
reading-big-meter-hint = { $keys } 查看详情

## What a screen reader says for the reading: never drawn, read aloud each
## time the reading changes. It is one list of phrases, in this order: the
## value, the mode, each sub-value, the meter's badges, then the app's;
## reading-spoken-separator goes between two phrases. For example:
## "DC 1.6112 V, AC+DC V, AC 0.0123 V, hold, software scaled".

# Between two phrases of the list.
reading-spoken-separator = {"，"}
# The reading, or a sub-value, with the name drawn before it. $label is the
# name (DC, Max, T2, Frequency…); $value is the value with its unit, e.g.
# "1.6112 V", "overload Ω" or "24.10 degrees C".
reading-spoken-labelled = { $label } { $value }
# A sub-value the meter caught $seconds seconds after MIN/MAX began,
# drawn as "@12s". $label and $value as in reading-spoken-labelled.
reading-spoken-labelled-at = 第 { $seconds } 秒时 { $label } { $value }
# Said in place of the digits when the input is past the range (OL).
reading-spoken-overload = 超量程
# The non-contact voltage detector's level; $level is a number from 1 up.
reading-spoken-ncv = NCV 等级 { $level }
# A word the meter shows in place of the digits, $word (e.g. ---- or Auto),
# which says there is no reading.
reading-spoken-no-reading = 无读数 ({ $word })
# The units °C and °F spelled out, since screen readers differ on whether
# they read the ° sign. C and F stay as letters, after the number.
reading-spoken-celsius = 摄氏度
reading-spoken-fahrenheit = 华氏度

## The meter's badges spoken, one phrase each.

# HV!: the probes are on a dangerous voltage.
reading-spoken-hv-warning = 高压警示
# AUTO: the meter picks its range itself.
reading-spoken-auto-range = 自动量程
# HOLD: the display is frozen.
reading-spoken-hold = 数据保持
# REL: the reading is relative to a stored one.
reading-spoken-rel = 相对测量
# MIN: the lowest reading since MIN/MAX began.
reading-spoken-min = 最小值
# MAX: the highest reading since MIN/MAX began.
reading-spoken-max = 最大值
# AVG: the mean of the readings since MIN/MAX began.
reading-spoken-avg = 平均值
# P-MIN: the lowest peak.
reading-spoken-peak-min = 峰值最小值
# P-MAX: the highest peak.
reading-spoken-peak-max = 峰值最大值
# LOW BAT: the meter's battery is low.
reading-spoken-low-battery = 电池电量不足
# LEAD ERR: the meter reports a problem with the test leads.
reading-spoken-lead-error = 表笔插错
# COMP: the meter's compare mode, which checks the reading against limits.
reading-spoken-comp = 比较
# REC: the meter is recording readings into its own memory.
reading-spoken-record = 记录
# LoZ: the low-impedance voltage mode.
reading-spoken-loz = 低阻抗
# VOID: the reading is not valid.
reading-spoken-void = 无效

## The app's own badges spoken, after the meter's.

# reading-badge-scaled.
reading-spoken-scaled = 软件换算
# reading-badge-above.
reading-spoken-above = 高于上限
# reading-badge-below.
reading-spoken-below = 低于下限
