### The Scale and Alarm chips at the end of the meter's buttons under the
### reading (assets/gui-reading-controls.webp), the row of fields each chip
### opens under them, the toasts they raise (the short messages under the
### top bar's right end) and the alarm's limit lines on the graph. No doc
### image shows a row open.
###
### Scale is a software transform: it turns the meter's reading into what
### it stands for, such as a current clamp's 10 mV/A into amps. Nothing is
### sent to the meter. Alarm watches the main reading against a low and a
### high limit, and marks each reading that crosses one.
###
### Numbers and units arrive formatted and stay as they are.

## The chips, at the end of the meter's buttons (HOLD, REL…), which a
## narrow window wraps: keep the names short. Each chip is a body that
## switches its feature on or off, joined to a caret (▸) that opens its row
## of fields; the body is filled while the feature is on.

# The Scale chip's body.
scale-chip = 换算
    .hover = 开启或关闭换算
# The caret beside Scale: opens the row of fields below.
scale-settings =
    .hover = 设置换算
    .a11y = 换算设置
# The Alarm chip's body.
alarm-chip = 报警
    .hover = 开启或关闭报警
# The caret beside Alarm: opens the row of fields below.
alarm-settings =
    .hover = 设置限值
    .a11y = 报警设置
# The hover of a greyed-out Scale or Alarm body while the window shows the
# meter alone with nothing typed in its fields yet. With the graph and the
# recording both hidden in Settings, there is no room for the row of
# fields.
alarm-scale-setup-in-settings = 请先在“设置”中显示图表或记录
# The same in the big meter, which has no room for the row. $keys is the
# shortcut that leaves the big meter, e.g. Ctrl+B or ⌘B.
alarm-scale-setup-in-normal-layout = 请先在常规布局中设置 ({ $keys })

## The Scale row, under the chips while Scale's caret is open:
## "× [Scale factor] + [Offset] → [Unit label] [Apply] [Off]". The ×, +
## and → before each field are symbols, not text. The fields are narrow:
## their placeholders are cut short with … when longer than the field.

# Placeholder in the empty field after ×: the number each reading is
# multiplied by.
scale-factor = 系数
# Placeholder in the empty field after +: the number added after
# multiplying.
scale-offset = 偏移
# Placeholder in the empty field after →: the unit the scaled reading is
# shown in, e.g. A or °F.
scale-unit = 单位
# Applies the three fields to the reading.
scale-apply = 应用
    .hover = 从现在起对每个读数应用这些值
# Turns scaling off; the fields keep their values for the next Apply.
scale-off = 关闭
    .hover = 停止换算，重新显示万用表本身的读数

## The Scale row's toasts.

# A field could not be read; nothing changes. $reason is why, lower-case
# with no full stop, e.g. “abc” is not a number.
scale-invalid-factor = 比例因子无效: { $reason }
# The same for the Offset field, with $reason as there.
scale-invalid-offset = 偏移无效: { $reason }
# Scaling turned on, or changed. $transform is the scale in force, written
# in symbols and numbers, e.g. ×100 → A or ×2 + 0.5. $also says what else
# happened: recording, a recording is under way and goes on with the scaled
# readings; alarm, the alarm was on and was turned off, since its limits
# were in the old unit; both; or none.
scale-on =
    { $also ->
        [recording] 正在换算读数: { $transform }；记录将继续使用换算后的值
        [alarm] 正在换算读数: { $transform }；报警已关闭，请用新单位重新设置限值
        [both] 正在换算读数: { $transform }；记录将继续使用换算后的值；报警已关闭，请用新单位重新设置限值
       *[none] 正在换算读数: { $transform }
    }
# Scaling turned off. $also as in scale-on.
scale-off-toast =
    { $also ->
        [recording] 换算已关闭；记录将继续使用换算后的值
        [alarm] 换算已关闭；报警已关闭，请用新单位重新设置限值
        [both] 换算已关闭；记录将继续使用换算后的值；报警已关闭，请用新单位重新设置限值
       *[none] 换算已关闭
    }

## The Alarm row, under the chips while Alarm's caret is open:
## "Low [none] High [none] V ± [auto] [Apply] [Off]". The fields are narrow.

# Caption before the low limit's field.
alarm-low = 下限
# Caption before the high limit's field.
alarm-high = 上限
# Placeholder in an empty Low or High field: that side has no limit.
alarm-no-limit = 无
# The field after ±, the hysteresis band: how far back inside a limit the
# reading must come before that limit can alarm again. Its placeholder says
# that blank picks the band automatically.
alarm-band = 自动
    .hover = 读数需回到限值内多少，该限值才会再次报警: 填写数值或百分比 (如 1%)，留空则自动设置
# After the limits, the unit they are in, e.g. V; this is its hover.
alarm-unit =
    .hover = 限值使用的单位
# The same before an alarm is on: the unit taken from the reading on
# screen.
alarm-unit-from-reading =
    .hover = 限值使用的单位，取自屏幕上的读数
# Shown in place of the unit, in the warning colour, while the reading is
# of another quantity than the limits: the dial was turned. $unit is the
# unit of the readings now arriving, e.g. Ω; on hover, $limits is the unit
# the limits are in, e.g. V. Short: it sits in the row.
alarm-idle = 待机: { $unit }
    .hover = 等待中: 读数单位是 { $unit }，限值单位是 { $limits }
# Turns the alarm on with the fields' limits.
alarm-apply = 应用
    .hover = 用这些限值开启报警
# Turns the alarm off; the fields keep their values for the next Apply.
alarm-off = 关闭
    .hover = 关闭报警

## The Alarm row's toasts. $reason is why a value was refused, lower-case
## with no full stop, e.g. “abc” is not a number.

# The Low field could not be read.
alarm-invalid-low = 下限无效: { $reason }
# The High field could not be read.
alarm-invalid-high = 上限无效: { $reason }
# A limit is not a finite number.
alarm-invalid-limit = 限值无效: { $reason }
# The two limits don't make a pair: the low one is not below the high one.
alarm-invalid-limits = 限值无效: { $reason }
# The ± field could not be read, or does not fit the limits.
alarm-invalid-band = 回差无效: { $reason }
# The alarm turned on. $limits is the limits it watches, from
# alarm-limits/alarm-hysteresis in lib-alarm.ftl, e.g. "above 5 or below 3,
# hysteresis 1%".
alarm-on-toast = 报警已开启: { $limits }
# The alarm turned off.
alarm-off-toast = 报警已关闭

## The graph: a dotted line at each limit, labelled at its right end.

# $side is high (the high limit) or low; $value is the limit with its unit,
# e.g. 3.0000 V.
alarm-limit-line =
    { $side ->
        [low] 下限: { $value }
       *[high] 上限: { $value }
    }
