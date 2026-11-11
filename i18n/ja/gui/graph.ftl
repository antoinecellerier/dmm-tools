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
    .hover = 直近 { $window } のサンプルを表示する ({ $keys } で切り替え)
# A chip, lit while the graph follows the newest samples; dragging the graph
# turns it off. Keep it short; it is set in small capitals.
graph-live = ライブ
    .hover = 最新のサンプルに自動で追従する (スクロール中はオフ、{ $keys } で戻る)
# The chip for the Y (vertical) axis while its bounds are typed in: Y is the
# axis, Fixed the mode. Clicking it switches to Y:Auto.
graph-y-fixed = Y:固定
    .hover = Y 軸の範囲を固定中 (クリックで表示中のデータに自動調整)
# The same chip while the Y axis fits the visible readings by itself.
graph-y-auto = Y:自動
    .hover = Y 軸を表示中のデータに自動調整中 (クリックで固定範囲を入力)
# The first of the two small fields after Y:Fixed. The text shows greyed
# inside the field while it is empty; the field is about 50 pixels wide.
graph-y-min = Y 軸の最小値
    .hover = 固定 Y 軸の下限
# The second field after Y:Fixed, as graph-y-min.
graph-y-max = Y 軸の最大値
    .hover = 固定 Y 軸の上限
# A button that undoes zooming and panning. "Double-click graph" is the
# gesture that does the same. The second hover shows while it is greyed
# out: "live" is the LIVE chip, "auto-Y" the Y:Auto chip.
graph-reset-zoom = ズームのリセット
    .hover = ライブ追従と Y 軸の自動調整に戻す (グラフをダブルクリック)
    .hover-disabled = 既定の表示 (ライブ追従、Y 軸の自動調整) になっています。

## The second row, only for meters that send more than one value: Plot:,
## the series the graph plots, then Show:, the traces drawn beside it, each
## in a box of its own. Each box holds one chip per series, labelled with
## the series' name.

# The caption at the start of the first box.
graph-plot = プロット:
# The Plot: chip for the meter's main reading while the meter has no name
# for it; its text also names that reading's trace in the plot's key, the
# hover and the Show: chip. .a11y is the chip's name for a screen reader,
# which tells it apart from the Show: chip of the same series.
graph-plot-main = メイン
    .hover = マルチメーターのメイン測定値をプロットする
    .a11y = メイン測定値をプロット
# The Plot: chip for the meter's main reading, which the meter calls
# $series, e.g. T1.
graph-plot-main-named =
    .hover = マルチメーターの { $series } 測定値をプロットする
# A Plot: chip for another value the meter sends: $series is its name, e.g.
# Frequency, $unit its unit, e.g. Hz. The .a11y is also that of the main
# reading's chip when the meter names it.
graph-plot-series =
    .hover = サブ表示値 { $series } ({ $unit }) をプロットする
    .a11y = { $series } をプロット
# The caption at the start of the second box.
graph-show = 表示:
# A Show: chip, which draws or hides a trace beside the plotted series:
# $series is its name. $state is full when the graph already has $max axes
# in other units (the chip is greyed out; $max is 4), narrow when the window
# is too narrow for the trace's own axis, shown while it is drawn, hidden
# while it is not. .a11y is its name for a screen reader.
graph-show-chip =
    .hover =
        { $state ->
            [full] 表示できる単位は最大 { $max } 個です。{ $series } を表示するには、いずれかを非表示にしてください。
            [narrow] ウィンドウを広げて { $series } の軸を表示するか、クリックしてトレースを非表示にしてください。
            [shown] { $series } のトレースを非表示にする
           *[hidden] プロット中の系列に重ねて { $series } のトレースを描画する
        }
    .a11y = { $series } のトレースを表示

## The last row: what the graph draws over the trace.

# A chip that draws a line at the mean (average) of the readings in view.
graph-mean = 平均値
    .hover = 表示中のサンプルの平均値に水平線を描画する ({ $keys })
# A chip that draws a shaded band between the lowest and highest readings
# over a sliding window of time.
graph-envelope = 最小/最大
    .hover = 時間幅をずらしながら求めた最小値と最大値の間を網掛けで描画する ({ $keys })
# The small field after Min/Max, about 30 pixels wide: how many seconds the
# band's window spans. The text shows greyed inside the field while it is
# empty; "s" after the field is the symbol for seconds.
graph-envelope-window = 最小/最大の時間幅 (秒)
    .hover = 最小/最大エンベロープの計算に使う時間幅 (秒)
# A chip that shows a histogram, bars of how many readings fell at each
# value, beside or under the graph. $state is hidden while the plotted
# trace itself is switched off, so the histogram has nothing to show;
# shown otherwise.
graph-histogram = ヒストグラム
    .hover =
        { $state ->
            [hidden] ヒストグラムを表示するには、プロット中のトレースを表示してください。クリックしてオフにすることもできます。
           *[shown] 表示中の測定値の分布を示す ({ $keys })
        }
# A chip that draws reference lines: horizontal lines at the values typed
# in the field after it. Short for reference.
graph-ref = 基準
    .hover = 右のフィールドに入力した値の位置に水平の基準線を描画する ({ $keys })
# The field after Ref, about 80 pixels wide. The text shows greyed inside
# the field while it is empty; the numbers in the hover are an example of
# what to type, kept as they are.
graph-ref-values = 基準値
# A chip, after the reference values, that marks where the trace crosses a
# reference line.
graph-triggers = トリガー
    .hover = 信号が基準線と交差する点をマークする ({ $keys })
# A chip that lets clicks on the graph place two vertical lines, A and B,
# and measure between them.
graph-cursors = カーソル
    .hover = グラフをクリックして 2 本のカーソルを配置する ({ $keys })
# Small text after Cursors until both are placed.
graph-cursors-hint = グラフをクリックしてカーソルを配置

## The plot: the readout by the pointer, the labels on its lines, and the
## flags of markers along its bottom edge.

# The readout by the pointer, under the time, inside a stretch where the
# meter read over range (showed OL).
graph-hover-overload = オーバーレンジ
# The same, with several series listed: $series is the plotted series'
# name.
graph-hover-overload-series = { $series }: オーバーレンジ
# One series' row in the readout when several are listed: $series is its
# name, e.g. T2; $value a number; $unit its unit, e.g. V.
graph-hover-series = { $series }: { $value } { $unit }
# The label at the right end of the Mean line. $value is the mean with its
# unit, e.g. 4.5045 V.
graph-mean-line = 平均値: { $value }
# A marker's flag (a numbered tag under its line) when hovered, and its name
# for a screen reader; a click opens the note. $number is the marker's
# number.
graph-marker-flag = マーカー { $number } のメモを書く

## What a screen reader hears of the plot: the sentences below, one after
## the other, in this order. Each ends with its own full stop.

# Between two of the sentences.
graph-plot-a11y-separator = {""}
# The first sentence.
graph-plot-a11y-title = 測定値のプロット。
# The first sentence when the plotted series has a name: $series, e.g. T2.
graph-plot-a11y-title-series = { $series } の測定値のプロット。
# How much time the plot shows: $seconds is a whole number of seconds.
graph-plot-a11y-window = 時間幅 { $seconds } 秒。
# The range of the left axis: $min is a number, $max a number with the
# plotted unit (reading-spoken-amount), e.g. 5.000 volts, or a bare number.
graph-plot-a11y-y-axis = Y 軸は { $min } から { $max }。
# $count is how many samples the graph holds.
graph-plot-a11y-samples = サンプル { $count } 件。
# Only when other traces are drawn beside the plotted series: $series lists
# them, separated by graph-plot-a11y-also-separator; each is a name, or
# graph-plot-a11y-also-axis.
graph-plot-a11y-also = { $series } も表示中。
# Between two traces in graph-plot-a11y-also.
graph-plot-a11y-also-separator = {"、"}
# One trace in graph-plot-a11y-also, drawn against an axis of its own on the
# right: $series is its name, $unit its unit, e.g. Hz.
graph-plot-a11y-also-axis = 右側の軸に { $series } ({ $unit })
# Only with markers on the readings in view: $count is how many.
graph-plot-a11y-markers = 表示中のマーカー { $count } 個。
# Whether the plot follows the newest samples ($state is live) or stays
# where it was panned to (paused). Lowercase in English, as it was written.
graph-plot-a11y-state =
    { $state ->
        [live] ライブ。
       *[paused] 一時停止中。
    }
# The last sentence: $value is the meter's last reading as it showed it,
# with its unit (reading-spoken-amount), e.g. 1.6112 volts.
graph-plot-a11y-reading = 最後の測定値 { $value }。
# The last sentence while the meter reads over range (shows OL).
graph-plot-a11y-over-range = 現在オーバーレンジです。
# The last sentence before any reading.
graph-plot-a11y-no-data = データなし。

## The histogram, beside the plot or under the minimap.

# The bar's range in its hover, and in graph-histogram-a11y-most: from
# $low, a number, to $high, a number with its unit: 3.299 V in the hover,
# 3.299 volts spoken (reading-spoken-amount).
graph-histogram-range = { $low } から { $high }
# A bar when hovered. $range is the values it spans, e.g. "3.200 to 3.299
# V" (see graph-histogram-range); $count how many readings in view fall
# there; $percent what share of them, a number with one decimal.
graph-histogram-bin =
    .hover = { $range }: 測定値 { $count } 件 ({ $percent }%)
# The line under it while both cursors are placed: the readings from cursor
# A to cursor B (A–B) in that bar. $count and $percent as above.
graph-histogram-bin-interval =
    .hover = A–B: 測定値 { $count } 件 ({ $percent }%)
# The divider between the plot and the histogram beside it, which is
# dragged sideways to resize it: its name for a screen reader. Left/Right
# are the arrow keys.
graph-histogram-divider-side =
    .a11y = ヒストグラムのサイズ変更 (左右方向キーで調整)
# The divider over the histogram under the minimap, dragged up or down.
# Up/Down are the arrow keys.
graph-histogram-divider-below =
    .a11y = ヒストグラムのサイズ変更 (上下方向キーで調整)

## What a screen reader hears of the histogram, one sentence among the
## plot's: graph-histogram-a11y, then the phrases after it, each after
## graph-histogram-a11y-separator (graph-histogram-a11y-interval after
## graph-histogram-a11y-interval-separator), in graph-histogram-a11y-sentence.

# The sentence: $clauses is graph-histogram-a11y and the phrases after it.
graph-histogram-a11y-sentence = { $clauses }。
# Between two phrases.
graph-histogram-a11y-separator = {"、"}
# Before graph-histogram-a11y-interval.
graph-histogram-a11y-interval-separator = {"。"}
# $count is how many readings are in view.
graph-histogram-a11y = 表示中の測定値 { $count } 件のヒストグラム
# After graph-histogram-a11y, and after graph-histogram-a11y-interval: how
# far the readings spread. $value is a number with its unit
# (reading-spoken-amount), e.g. 0.002 volts.
graph-histogram-a11y-sd = 標準偏差 { $value }
# The bar holding the most readings: $range is its values (see
# graph-histogram-range), $count how many it holds.
graph-histogram-a11y-most = 最多は { $range } ({ $count } 件)
# How many readings in view were over range (OL). $count is that number.
graph-histogram-a11y-over-range = オーバーレンジ { $count } 件
# How many readings in view lie above and below the Y axis' ends, out of
# the histogram's reach. $above and $below are those numbers.
graph-histogram-a11y-beyond = 軸より上に { $above } 件、下に { $below } 件
# While both cursors are placed: $count is how many samples lie between
# them.
graph-histogram-a11y-interval = カーソル間の測定値 { $count } 件

## The minimap: the strip under the graph showing every sample kept, with
## brackets around the part the graph shows.

# The hover, and its name for a screen reader. Left/Right are the arrow
# keys.
graph-minimap =
    .hover = ミニマップ: クリックまたはドラッグでスクロール、ビューポートの端をドラッグして表示範囲の幅を変更
    .a11y = グラフのミニマップ: クリックまたはドラッグでタイムラインを移動 (左右方向キーでスクロール)、ビューポートの端をドラッグしてサイズを変更

## The menu a right-click on the plot, its axes or a marker's flag opens:
## what can be done to the reading nearest the click, under a heading
## naming it, then what can be done at the height clicked, under a heading
## naming that. Each entry's .a11y is its name for a screen reader, which
## does not read the headings. A cursor is one of the two vertical lines
## Cursors places, A and B ($cursor); a reference line is one the Ref chip
## draws; a marker is a numbered flag on a reading, with a note.

# The heading when no reading is in view, and the hover of the entries
# greyed out then.
graph-menu-no-reading = 表示中の測定値なし
# The heading over the reading's entries. $time is the reading's time on the
# graph, e.g. 35.0 s, drawn in a stronger colour.
graph-menu-reading-heading = { $time } の測定値
# The heading over the height's entries. $height is the height clicked, a
# number with its unit, e.g. 3.29 V, drawn in a stronger colour.
graph-menu-height-heading = { $height } の位置
# Puts a marker on the reading. $reading is some, with $time the reading's
# time, e.g. 35.0 s; none while no reading is in view (the entry is greyed
# out).
graph-menu-mark = マーカーの追加
    .a11y =
        { $reading ->
            [some] { $time } の測定値にマーカーを追加
           *[none] 測定値にマーカーを追加
        }
# Copies the reading while none is in view (greyed out).
graph-menu-copy = コピー
# Copies the reading: $digits are its digits as the meter showed them and
# its unit, e.g. 3.249 V.
graph-menu-copy-digits = { $digits } のコピー
# Deletes the marker under the pointer, or the one whose flag was
# right-clicked. $number is its number.
graph-menu-delete-marker = マーカー { $number } の削除
# Puts the first cursor on the reading, while none is placed. $reading and
# $time are as for Mark.
graph-menu-put-cursor = カーソルの配置
    .a11y =
        { $reading ->
            [some] { $time } の測定値にカーソルを配置
           *[none] 測定値にカーソルを配置
        }
# Puts the cursor not yet placed, $cursor, on the reading. $reading and
# $time are as for Mark.
graph-menu-put-cursor-named = カーソル { $cursor } の配置
    .a11y =
        { $reading ->
            [some] { $time } の測定値にカーソル { $cursor } を配置
           *[none] 測定値にカーソル { $cursor } を配置
        }
# Moves cursor $cursor, already placed, to the reading. $reading and $time
# are as for Mark.
graph-menu-move-cursor = カーソル { $cursor } の移動
    .a11y =
        { $reading ->
            [some] カーソル { $cursor } を { $time } の測定値に移動
           *[none] カーソル { $cursor } を測定値に移動
        }
# Removes the cursor under the pointer, $cursor.
graph-menu-remove-cursor = カーソル { $cursor } の削除
# Removes both cursors and turns Cursors off.
graph-menu-remove-cursors = 全カーソルの削除
# Draws a reference line at the height clicked, $height (see
# graph-menu-height-heading).
graph-menu-add-ref = 基準線の追加
    .a11y = { $height } に基準線を追加
# Removes the reference line under the pointer, at $height, a number with
# its unit.
graph-menu-remove-ref = 基準線 { $height } の削除
# Sets one of the alarm's two limits at the height clicked: $side is high
# for the upper limit, low for the lower. The alarm, set up beside the
# reading, warns when a reading goes past a limit. $height as above.
graph-menu-set-alarm =
    { $side ->
        [high] アラーム上限の設定
       *[low] アラーム下限の設定
    }
    .a11y =
        { $side ->
            [high] アラーム上限を { $height } に設定
           *[low] アラーム下限を { $height } に設定
        }
# Removes the alarm limit whose line is under the pointer; $side as above.
graph-menu-remove-alarm =
    { $side ->
        [high] アラーム上限の削除
       *[low] アラーム下限の削除
    }
