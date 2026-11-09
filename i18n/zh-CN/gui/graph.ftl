### The graph (assets/gui-wide-layout.webp, assets/gui-graph-series.webp,
### assets/gui-graph-histogram.webp): the rows of chips over it, the
### plot with its hover readout and the labels it draws on its lines, the
### histogram beside or under it, the minimap under it, and the menu its
### right-click opens.
###
### A sample is one reading the meter sent; the plotted series is the
### reading the graph draws, the meter's main reading or one of the other
### values it sends (a sub-value, e.g. Frequency or T2); a trace is a series'
### line. The chips sit in tight rows, so keep their labels short. $keys is
### a key that does the same as the chip, as this computer writes it, e.g.
### M or End; keep it as it is. Series names ($series) are the meter's own
### labels, e.g. T2 or Frequency, and units ($unit) are symbols, e.g. V or
### Hz: phrase around them.

## The first row of chips over the graph.

# The chips 5s, 10s, 30s, 1m, 5m and 10m, which pick how much time the graph
# shows; they have no text of their own. $window is the chip's own label,
# e.g. 30s; $keys the two keys that step through them, e.g. [ / ].
graph-window =
    .hover = 显示最近 { $window } 的样本 (按 { $keys } 循环切换)
# A chip, lit while the graph follows the newest samples; dragging the graph
# turns it off. Keep it short; it is set in small capitals.
graph-live = 实时
    .hover = 自动跟随最新样本: 平移时关闭 (按 { $keys } 跳回)
# The chip for the Y (vertical) axis while its bounds are typed in: Y is the
# axis, Fixed the mode. Clicking it switches to Y:Auto.
graph-y-fixed = Y:固定
    .hover = 使用固定的 Y 轴范围: 单击改为按可见数据自动调整
# The same chip while the Y axis fits the visible readings by itself.
graph-y-auto = Y:自动
    .hover = Y 轴按可见数据自动调整: 单击改为输入固定范围
# The first of the two small fields after Y:Fixed. The text shows greyed
# inside the field while it is empty; the field is about 50 pixels wide.
graph-y-min = Y 轴最小值
    .hover = 固定 Y 轴的下限
# The second field after Y:Fixed, as graph-y-min.
graph-y-max = Y 轴最大值
    .hover = 固定 Y 轴的上限
# A button that undoes zooming and panning. "Double-click graph" is the
# gesture that does the same. The second hover shows while it is greyed
# out: "live" is the LIVE chip, "auto-Y" the Y:Auto chip.
graph-reset-zoom = 重置缩放
    .hover = 恢复实时跟随和自动 Y 轴 (双击图表)
    .hover-disabled = 当前已是默认视图 (实时 + 自动 Y 轴)

## The second row, only for meters that send more than one value: Plot:,
## the series the graph plots, then Show:, the traces drawn beside it, each
## in a box of its own. Each box holds one chip per series, labelled with
## the series' name.

# The caption at the start of the first box.
graph-plot = 绘制:
# The Plot: chip for the meter's main reading while the meter has no name
# for it; its text also names that reading's trace in the plot's key, the
# hover and the Show: chip. .a11y is the chip's name for a screen reader,
# which tells it apart from the Show: chip of the same series.
graph-plot-main = 主读数
    .hover = 绘制万用表的主读数
    .a11y = 绘制主读数
# The Plot: chip for the meter's main reading, which the meter calls
# $series, e.g. T1.
graph-plot-main-named =
    .hover = 绘制万用表的 { $series } 读数
# A Plot: chip for another value the meter sends: $series is its name, e.g.
# Frequency, $unit its unit, e.g. Hz. The .a11y is also that of the main
# reading's chip when the meter names it.
graph-plot-series =
    .hover = 绘制副读数“{ $series }” ({ $unit })
    .a11y = 绘制 { $series }
# The caption at the start of the second box.
graph-show = 显示:
# A Show: chip, which draws or hides a trace beside the plotted series:
# $series is its name. $state is full when the graph already has $max axes
# in other units (the chip is greyed out; $max is 4), narrow when the window
# is too narrow for the trace's own axis, shown while it is drawn, hidden
# while it is not. .a11y is its name for a screen reader.
graph-show-chip =
    .hover =
        { $state ->
            [full] 最多显示 { $max } 种单位，请先隐藏一种再显示“{ $series }”
            [narrow] 加宽窗口以显示“{ $series }”轴，或单击隐藏它
            [shown] 隐藏“{ $series }”曲线
           *[hidden] 在所绘曲线旁显示“{ $series }”曲线
        }
    .a11y = 显示 { $series } 曲线

## The last row: what the graph draws over the trace.

# A chip that draws a line at the mean (average) of the readings in view.
graph-mean = 平均值
    .hover = 在可见样本的平均值处绘制水平线 ({ $keys })
# A chip that draws a shaded band between the lowest and highest readings
# over a sliding window of time.
graph-envelope = 最小/最大
    .hover = 在滚动最小值和最大值之间绘制阴影带 ({ $keys })
# The small field after Min/Max, about 30 pixels wide: how many seconds the
# band's window spans. The text shows greyed inside the field while it is
# empty; "s" after the field is the symbol for seconds.
graph-envelope-window = 最小/最大窗口 (秒)
    .hover = 计算最小/最大包络所用的窗口大小 (秒)
# A chip that shows a histogram, bars of how many readings fell at each
# value, beside or under the graph. $state is hidden while the plotted
# trace itself is switched off, so the histogram has nothing to show;
# shown otherwise.
graph-histogram = 直方图
    .hover =
        { $state ->
            [hidden] 显示所绘曲线后才能查看其直方图，或单击关闭
           *[shown] 显示可见读数的分布 ({ $keys })
        }
# A chip that draws reference lines: horizontal lines at the values typed
# in the field after it. Short for reference.
graph-ref = 参考
    .hover = 在右侧输入框中的各值处绘制水平参考线 ({ $keys })
# The field after Ref, about 80 pixels wide. The text shows greyed inside
# the field while it is empty; the numbers in the hover are an example of
# what to type, kept as they are.
graph-ref-values = 参考值
    .hover = 参考值，以逗号或分号分隔 (例如 3.3, 5, 12)
# A chip, after the reference values, that marks where the trace crosses a
# reference line.
graph-triggers = 触发
    .hover = 标记信号穿越参考线的点 ({ $keys })
# A chip that lets clicks on the graph place two vertical lines, A and B,
# and measure between them.
graph-cursors = 光标
    .hover = 单击图表放置两个光标 ({ $keys })
# Small text after Cursors until both are placed.
graph-cursors-hint = 单击图表放置光标
# After Cursors once both are placed, among figures such as ΔT=12.50 s and
# σ=0.0012 V: the mean of the readings from A to B. $value is a number with
# its unit, e.g. 3.2401 V. Keep it short, with no spaces around =. ΔT=,
# Δ= and σ= beside it are notation and stay as they are in every language.
graph-readout-mean = 平均值={ $value }
# The same row: how many readings lie from A to B. $count is that number.
graph-readout-count = 读数={ $count }

## The plot: the readout by the pointer, the labels on its lines, and the
## flags of markers along its bottom edge.

# The readout by the pointer, under the time, inside a stretch where the
# meter read over range (showed OL).
graph-hover-overload = 超量程
# The same, with several series listed: $series is the plotted series'
# name.
graph-hover-overload-series = { $series }: 超量程
# One series' row in the readout when several are listed: $series is its
# name, e.g. T2; $value a number; $unit its unit, e.g. V.
graph-hover-series = { $series }: { $value } { $unit }
# The label at the right end of the Mean line. $value is the mean with its
# unit, e.g. 4.5045 V.
graph-mean-line = 平均值: { $value }
# A marker's flag (a numbered tag under its line) when hovered, and its name
# for a screen reader; a click opens the note. $number is the marker's
# number.
graph-marker-flag = 填写标记 { $number } 的备注

## What a screen reader hears of the plot: the sentences below, one after
## the other, in this order. Each ends with its own full stop.

# Between two of the sentences.
graph-plot-a11y-separator = {""}
# The first sentence.
graph-plot-a11y-title = 测量绘图。
# The first sentence when the plotted series has a name: $series, e.g. T2.
graph-plot-a11y-title-series = { $series } 的测量绘图。
# How much time the plot shows: $seconds is a whole number of seconds.
graph-plot-a11y-window = 时间窗口 { $seconds } 秒。
# The range of the left axis: $min is a number, $max a number with the
# plotted unit (reading-spoken-amount), e.g. 5.000 volts, or a bare number.
graph-plot-a11y-y-axis = Y 轴范围 { $min } 到 { $max }。
# $count is how many samples the graph holds.
graph-plot-a11y-samples = { $count } 个样本。
# Only when other traces are drawn beside the plotted series: $series lists
# them, separated by graph-plot-a11y-also-separator; each is a name, or
# graph-plot-a11y-also-axis.
graph-plot-a11y-also = 同时显示 { $series }。
# Between two traces in graph-plot-a11y-also.
graph-plot-a11y-also-separator = {"、"}
# One trace in graph-plot-a11y-also, drawn against an axis of its own on the
# right: $series is its name, $unit its unit, e.g. Hz.
graph-plot-a11y-also-axis = { $series } ({ $unit }，右轴)
# Only with markers on the readings in view: $count is how many.
graph-plot-a11y-markers = 视图中有 { $count } 个标记。
# Whether the plot follows the newest samples ($state is live) or stays
# where it was panned to (paused). Lowercase in English, as it was written.
graph-plot-a11y-state =
    { $state ->
        [live] 实时。
       *[paused] 已暂停。
    }
# The last sentence: $value is the meter's last reading as it showed it,
# with its unit (reading-spoken-amount), e.g. 1.6112 volts.
graph-plot-a11y-reading = 最新读数 { $value }。
# The last sentence while the meter reads over range (shows OL).
graph-plot-a11y-over-range = 当前超量程。
# The last sentence before any reading.
graph-plot-a11y-no-data = 无数据。

## The histogram, beside the plot or under the minimap.

# The bar's range in its hover, and in graph-histogram-a11y-most: from
# $low, a number, to $high, a number with its unit: 3.299 V in the hover,
# 3.299 volts spoken (reading-spoken-amount).
graph-histogram-range = { $low } 到 { $high }
# A bar when hovered. $range is the values it spans, e.g. "3.200 to 3.299
# V" (see graph-histogram-range); $count how many readings in view fall
# there; $percent what share of them, a number with one decimal.
graph-histogram-bin =
    .hover = { $range }: { $count } 个读数 ({ $percent }%)
# The line under it while both cursors are placed: the readings from cursor
# A to cursor B (A–B) in that bar. $count and $percent as above.
graph-histogram-bin-interval =
    .hover = A–B: { $count } 个读数 ({ $percent }%)
# The divider between the plot and the histogram beside it, which is
# dragged sideways to resize it: its name for a screen reader. Left/Right
# are the arrow keys.
graph-histogram-divider-side =
    .a11y = 调整直方图大小 (按向左键/向右键调整)
# The divider over the histogram under the minimap, dragged up or down.
# Up/Down are the arrow keys.
graph-histogram-divider-below =
    .a11y = 调整直方图大小 (按向上键/向下键调整)

## What a screen reader hears of the histogram, one sentence among the
## plot's: graph-histogram-a11y, then the phrases after it, each after
## graph-histogram-a11y-separator (graph-histogram-a11y-interval after
## graph-histogram-a11y-interval-separator), in graph-histogram-a11y-sentence.

# The sentence: $clauses is graph-histogram-a11y and the phrases after it.
graph-histogram-a11y-sentence = { $clauses }。
# Between two phrases.
graph-histogram-a11y-separator = {"，"}
# Before graph-histogram-a11y-interval.
graph-histogram-a11y-interval-separator = {"；"}
# $count is how many readings are in view.
graph-histogram-a11y = { $count } 个可见读数的直方图
# After graph-histogram-a11y, and after graph-histogram-a11y-interval: how
# far the readings spread. $value is a number with its unit
# (reading-spoken-amount), e.g. 0.002 volts.
graph-histogram-a11y-sd = 标准差 { $value }
# The bar holding the most readings: $range is its values (see
# graph-histogram-range), $count how many it holds.
graph-histogram-a11y-most = 读数最多的区间为 { $range } ({ $count })
# How many readings in view were over range (OL). $count is that number.
graph-histogram-a11y-over-range = { $count } 个读数超量程
# How many readings in view lie above and below the Y axis' ends, out of
# the histogram's reach. $above and $below are those numbers.
graph-histogram-a11y-beyond = 轴上方 { $above } 个，轴下方 { $below } 个
# While both cursors are placed: $count is how many samples lie between
# them.
graph-histogram-a11y-interval = 两个光标之间有 { $count } 个读数

## The minimap: the strip under the graph showing every sample kept, with
## brackets around the part the graph shows.

# The hover, and its name for a screen reader. Left/Right are the arrow
# keys.
graph-minimap =
    .hover = 缩略图: 单击或拖动可平移，拖动括号边缘可调整视图大小
    .a11y = 图表缩略图: 单击或拖动可浏览时间线 (按向左键/向右键平移)，拖动括号边缘可调整大小

## The menu a right-click on the plot, its axes or a marker's flag opens:
## what can be done to the reading nearest the click, under a heading
## naming it, then what can be done at the height clicked, under a heading
## naming that. Each entry's .a11y is its name for a screen reader, which
## does not read the headings. A cursor is one of the two vertical lines
## Cursors places, A and B ($cursor); a reference line is one the Ref chip
## draws; a marker is a numbered flag on a reading, with a note.

# The heading when no reading is in view, and the hover of the entries
# greyed out then.
graph-menu-no-reading = 视图中没有读数
# The heading over the reading's entries. $time is the reading's time on the
# graph, e.g. 35.0 s, drawn in a stronger colour.
graph-menu-reading-heading = { $time } 处的读数
# The heading over the height's entries. $height is the height clicked, a
# number with its unit, e.g. 3.29 V, drawn in a stronger colour.
graph-menu-height-heading = { $height } 处
# Puts a marker on the reading. $reading is some, with $time the reading's
# time, e.g. 35.0 s; none while no reading is in view (the entry is greyed
# out).
graph-menu-mark = 标记
    .a11y =
        { $reading ->
            [some] 标记 { $time } 处的读数
           *[none] 标记读数
        }
# Copies the reading while none is in view (greyed out).
graph-menu-copy = 复制
# Copies the reading: $digits are its digits as the meter showed them and
# its unit, e.g. 3.249 V.
graph-menu-copy-digits = 复制 { $digits }
# Deletes the marker under the pointer, or the one whose flag was
# right-clicked. $number is its number.
graph-menu-delete-marker = 删除标记 { $number }
# Puts the first cursor on the reading, while none is placed. $reading and
# $time are as for Mark.
graph-menu-put-cursor = 放置光标
    .a11y =
        { $reading ->
            [some] 将光标放在 { $time } 处的读数上
           *[none] 将光标放在读数上
        }
# Puts the cursor not yet placed, $cursor, on the reading. $reading and
# $time are as for Mark.
graph-menu-put-cursor-named = 放置光标 { $cursor }
    .a11y =
        { $reading ->
            [some] 将光标 { $cursor } 放在 { $time } 处的读数上
           *[none] 将光标 { $cursor } 放在读数上
        }
# Moves cursor $cursor, already placed, to the reading. $reading and $time
# are as for Mark.
graph-menu-move-cursor = 移动光标 { $cursor }
    .a11y =
        { $reading ->
            [some] 将光标 { $cursor } 移到 { $time } 处的读数上
           *[none] 将光标 { $cursor } 移到读数上
        }
# Removes the cursor under the pointer, $cursor.
graph-menu-remove-cursor = 移除光标 { $cursor }
# Removes both cursors and turns Cursors off.
graph-menu-remove-cursors = 移除光标
# Draws a reference line at the height clicked, $height (see
# graph-menu-height-heading).
graph-menu-add-ref = 添加参考线
    .a11y = 在 { $height } 处添加参考线
# Removes the reference line under the pointer, at $height, a number with
# its unit.
graph-menu-remove-ref = 移除参考线 { $height }
# Sets one of the alarm's two limits at the height clicked: $side is high
# for the upper limit, low for the lower. The alarm, set up beside the
# reading, warns when a reading goes past a limit. $height as above.
graph-menu-set-alarm =
    { $side ->
        [high] 设置报警上限
       *[low] 设置报警下限
    }
    .a11y =
        { $side ->
            [high] 在 { $height } 处设置报警上限
           *[low] 在 { $height } 处设置报警下限
        }
# Removes the alarm limit whose line is under the pointer; $side as above.
graph-menu-remove-alarm =
    { $side ->
        [high] 移除报警上限
       *[low] 移除报警下限
    }
