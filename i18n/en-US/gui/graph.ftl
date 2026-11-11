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
    .hover = Show the last { $window } of samples ({ $keys } to cycle)
# A chip, lit while the graph follows the newest samples; dragging the graph
# turns it off. Keep it short; it is set in small capitals.
graph-live = LIVE
    .hover = Auto-follow the newest samples — off while panning ({ $keys } to jump back)
# The chip for the Y (vertical) axis while its bounds are typed in: Y is the
# axis, Fixed the mode. Clicking it switches to Y:Auto.
graph-y-fixed = Y:Fixed
    .hover = Using fixed Y-axis bounds — click to auto-scale to visible data
# The same chip while the Y axis fits the visible readings by itself.
graph-y-auto = Y:Auto
    .hover = Auto-scaling Y to visible data — click to enter fixed bounds
# The first of the two small fields after Y:Fixed. The text shows greyed
# inside the field while it is empty; the field is about 50 pixels wide.
graph-y-min = Y axis minimum
    .hover = Lower bound of the fixed Y axis
# The second field after Y:Fixed, as graph-y-min.
graph-y-max = Y axis maximum
    .hover = Upper bound of the fixed Y axis
# A button that undoes zooming and panning. "Double-click graph" is the
# gesture that does the same. The second hover shows while it is greyed
# out: "live" is the LIVE chip, "auto-Y" the Y:Auto chip.
graph-reset-zoom = Reset Zoom
    .hover = Return to live follow and auto Y (double-click graph)
    .hover-disabled = Already at the default live + auto-Y view

## The second row, only for meters that send more than one value: Plot:,
## the series the graph plots, then Show:, the traces drawn beside it, each
## in a box of its own. Each box holds one chip per series, labelled with
## the series' name.

# The caption at the start of the first box.
graph-plot = Plot:
# The Plot: chip for the meter's main reading while the meter has no name
# for it; its text also names that reading's trace in the plot's key, the
# hover and the Show: chip. .a11y is the chip's name for a screen reader,
# which tells it apart from the Show: chip of the same series.
graph-plot-main = Main
    .hover = Plot the meter's main reading
    .a11y = Plot main reading
# The Plot: chip for the meter's main reading, which the meter calls
# $series, e.g. T1.
graph-plot-main-named =
    .hover = Plot the meter's { $series } reading
# A Plot: chip for another value the meter sends: $series is its name, e.g.
# Frequency, $unit its unit, e.g. Hz. The .a11y is also that of the main
# reading's chip when the meter names it.
graph-plot-series =
    .hover = Plot the { $series } sub-value ({ $unit })
    .a11y = Plot { $series }
# The caption at the start of the second box.
graph-show = Show:
# A Show: chip, which draws or hides a trace beside the plotted series:
# $series is its name. $state is full when the graph already has $max axes
# in other units (the chip is greyed out; $max is 4), narrow when the window
# is too narrow for the trace's own axis, shown while it is drawn, hidden
# while it is not. .a11y is its name for a screen reader.
graph-show-chip =
    .hover =
        { $state ->
            [full] { $max } units at most — hide one to show { $series }
            [narrow] Widen the window to show the { $series } axis, or click to hide it
            [shown] Hide the { $series } trace
           *[hidden] Draw the { $series } trace beside the plotted series
        }
    .a11y = Show { $series } trace

## The last row: what the graph draws over the trace.

# A chip that draws a line at the mean (average) of the readings in view.
graph-mean = Mean
    .hover = Draw a horizontal line at the mean of visible samples ({ $keys })
# A chip that draws a shaded band between the lowest and highest readings
# over a sliding window of time.
graph-envelope = Min/Max
    .hover = Draw a shaded band between the rolling min and max ({ $keys })
# The small field after Min/Max, about 30 pixels wide: how many seconds the
# band's window spans. The text shows greyed inside the field while it is
# empty; "s" after the field is the symbol for seconds.
graph-envelope-window = Min/Max window, seconds
    .hover = Window size (seconds) used to compute the Min/Max envelope
# A chip that shows a histogram, bars of how many readings fell at each
# value, beside or under the graph. $state is hidden while the plotted
# trace itself is switched off, so the histogram has nothing to show;
# shown otherwise.
graph-histogram = Histogram
    .hover =
        { $state ->
            [hidden] Show the plotted trace to see its histogram, or click to turn it off
           *[shown] Show how the visible readings spread ({ $keys })
        }
# A chip that draws reference lines: horizontal lines at the values typed
# in the field after it. Short for reference.
graph-ref = Ref
    .hover = Draw horizontal reference lines at the values in the next field ({ $keys })
# The field after Ref, about 120 pixels wide. The text shows greyed inside
# the field while it is empty; the numbers in the hover are an example of
# what to type, kept as they are.
graph-ref-values = Reference values
    .hover = Reference values, comma- or semicolon-separated, with a unit or in the plotted one (e.g. 3.3, 5 V, 4.7k)
# A chip, after the reference values, that marks where the trace crosses a
# reference line.
graph-triggers = Triggers
    .hover = Mark the points where the signal crosses a reference line ({ $keys })
# A chip that lets clicks on the graph place two vertical lines, A and B,
# and measure between them.
graph-cursors = Cursors
    .hover = Click the graph to place two cursors ({ $keys })
# Small text after Cursors until both are placed.
graph-cursors-hint = click graph to place cursors
# After Cursors once both are placed, among figures such as ΔT=12.50 s and
# σ=0.0012 V: the mean of the readings from A to B. $value is a number with
# its unit, e.g. 3.2401 V. Keep it short, with no spaces around =. ΔT=,
# Δ= and σ= beside it are notation and stay as they are in every language.
graph-readout-mean = Mean={ $value }
# The same row: how many readings lie from A to B. $count is that number.
graph-readout-count = Readings={ $count }

## The plot: the readout by the pointer, the labels on its lines, and the
## flags of markers along its bottom edge.

# The readout by the pointer, under the time, inside a stretch where the
# meter read over range (showed OL).
graph-hover-overload = overload
# The same, with several series listed: $series is the plotted series'
# name.
graph-hover-overload-series = { $series }: overload
# One series' row in the readout when several are listed: $series is its
# name, e.g. T2; $value a number; $unit its unit, e.g. V.
graph-hover-series = { $series }: { $value } { $unit }
# The label at the right end of the Mean line. $value is the mean with its
# unit, e.g. 4.5045 V.
graph-mean-line = Mean: { $value }
# A marker's flag (a numbered tag under its line) when hovered, and its name
# for a screen reader; a click opens the note. $number is the marker's
# number.
graph-marker-flag = Write marker { $number }'s note

## What a screen reader hears of the plot: the sentences below, one after
## the other, in this order. Each ends with its own full stop.

# Between two of the sentences.
graph-plot-a11y-separator = {" "}
# The first sentence.
graph-plot-a11y-title = Measurement plot.
# The first sentence when the plotted series has a name: $series, e.g. T2.
graph-plot-a11y-title-series = Measurement plot of { $series }.
# How much time the plot shows: $seconds is a whole number of seconds.
graph-plot-a11y-window =
    { $seconds ->
        [one] { $seconds } second window.
       *[other] { $seconds } second window.
    }
# The range of the left axis: $min is a number, $max a number with the
# plotted unit (reading-spoken-amount), e.g. 5.000 volts, or a bare number.
graph-plot-a11y-y-axis = Y axis { $min } to { $max }.
# $count is how many samples the graph holds.
graph-plot-a11y-samples =
    { $count ->
        [one] { $count } sample.
       *[other] { $count } samples.
    }
# Only when other traces are drawn beside the plotted series: $series lists
# them, separated by graph-plot-a11y-also-separator; each is a name, or
# graph-plot-a11y-also-axis.
graph-plot-a11y-also = Also showing { $series }.
# Between two traces in graph-plot-a11y-also.
graph-plot-a11y-also-separator = {", "}
# One trace in graph-plot-a11y-also, drawn against an axis of its own on the
# right: $series is its name, $unit its unit, e.g. Hz.
graph-plot-a11y-also-axis = { $series } in { $unit } on a right axis
# Only with markers on the readings in view: $count is how many.
graph-plot-a11y-markers =
    { $count ->
        [one] { $count } marker in view.
       *[other] { $count } markers in view.
    }
# Whether the plot follows the newest samples ($state is live) or stays
# where it was panned to (paused). Lowercase in English, as it was written.
graph-plot-a11y-state =
    { $state ->
        [live] live.
       *[paused] paused.
    }
# The last sentence: $value is the meter's last reading as it showed it,
# with its unit (reading-spoken-amount), e.g. 1.6112 volts.
graph-plot-a11y-reading = last reading { $value }.
# The last sentence while the meter reads over range (shows OL).
graph-plot-a11y-over-range = currently over range.
# The last sentence before any reading.
graph-plot-a11y-no-data = no data.

## The histogram, beside the plot or under the minimap.

# The bar's range in its hover, and in graph-histogram-a11y-most: from
# $low, a number, to $high, a number with its unit: 3.299 V in the hover,
# 3.299 volts spoken (reading-spoken-amount).
graph-histogram-range = { $low } to { $high }
# A bar when hovered. $range is the values it spans, e.g. "3.200 to 3.299
# V" (see graph-histogram-range); $count how many readings in view fall
# there; $percent what share of them, a number with one decimal.
graph-histogram-bin =
    .hover =
        { $count ->
            [one] { $range }: { $count } reading ({ $percent }%)
           *[other] { $range }: { $count } readings ({ $percent }%)
        }
# The line under it while both cursors are placed: the readings from cursor
# A to cursor B (A–B) in that bar. $count and $percent as above.
graph-histogram-bin-interval =
    .hover =
        { $count ->
            [one] A–B: { $count } reading ({ $percent }%)
           *[other] A–B: { $count } readings ({ $percent }%)
        }
# The divider between the plot and the histogram beside it, which is
# dragged sideways to resize it: its name for a screen reader. Left/Right
# are the arrow keys.
graph-histogram-divider-side =
    .a11y = Resize the histogram (Left/Right to adjust)
# The divider over the histogram under the minimap, dragged up or down.
# Up/Down are the arrow keys.
graph-histogram-divider-below =
    .a11y = Resize the histogram (Up/Down to adjust)

## What a screen reader hears of the histogram, one sentence among the
## plot's: graph-histogram-a11y, then the phrases after it, each after
## graph-histogram-a11y-separator (graph-histogram-a11y-interval after
## graph-histogram-a11y-interval-separator), in graph-histogram-a11y-sentence.

# The sentence: $clauses is graph-histogram-a11y and the phrases after it.
graph-histogram-a11y-sentence = { $clauses }.
# Between two phrases.
graph-histogram-a11y-separator = {", "}
# Before graph-histogram-a11y-interval.
graph-histogram-a11y-interval-separator = {"; "}
# $count is how many readings are in view.
graph-histogram-a11y =
    { $count ->
        [one] Histogram of { $count } visible reading
       *[other] Histogram of { $count } visible readings
    }
# After graph-histogram-a11y, and after graph-histogram-a11y-interval: how
# far the readings spread. $value is a number with its unit
# (reading-spoken-amount), e.g. 0.002 volts.
graph-histogram-a11y-sd = standard deviation { $value }
# The bar holding the most readings: $range is its values (see
# graph-histogram-range), $count how many it holds.
graph-histogram-a11y-most = most at { $range } ({ $count })
# How many readings in view were over range (OL). $count is that number.
graph-histogram-a11y-over-range =
    { $count ->
        [one] { $count } over range
       *[other] { $count } over range
    }
# How many readings in view lie above and below the Y axis' ends, out of
# the histogram's reach. $above and $below are those numbers.
graph-histogram-a11y-beyond = { $above } above and { $below } below the axis
# While both cursors are placed: $count is how many samples lie between
# them.
graph-histogram-a11y-interval =
    { $count ->
        [one] { $count } reading between the cursors
       *[other] { $count } readings between the cursors
    }

## The minimap: the strip under the graph showing every sample kept, with
## brackets around the part the graph shows.

# The hover, and its name for a screen reader. Left/Right are the arrow
# keys.
graph-minimap =
    .hover = Minimap — click or drag to pan, drag the bracket edges to resize the view
    .a11y = Graph minimap — click or drag to navigate timeline (Left/Right to pan), drag bracket edges to resize

## The menu a right-click on the plot, its axes or a marker's flag opens:
## what can be done to the reading nearest the click, under a heading
## naming it, then what can be done at the height clicked, under a heading
## naming that. Each entry's .a11y is its name for a screen reader, which
## does not read the headings. A cursor is one of the two vertical lines
## Cursors places, A and B ($cursor); a reference line is one the Ref chip
## draws; a marker is a numbered flag on a reading, with a note.

# The heading when no reading is in view, and the hover of the entries
# greyed out then.
graph-menu-no-reading = No reading in view
# The heading over the reading's entries. $time is the reading's time on the
# graph, e.g. 35.0 s, drawn in a stronger colour.
graph-menu-reading-heading = Reading at { $time }
# The heading over the height's entries. $height is the height clicked, a
# number with its unit, e.g. 3.29 V, drawn in a stronger colour.
graph-menu-height-heading = At { $height }
# Puts a marker on the reading. $reading is some, with $time the reading's
# time, e.g. 35.0 s; none while no reading is in view (the entry is greyed
# out).
graph-menu-mark = Mark
    .a11y =
        { $reading ->
            [some] Mark the reading at { $time }
           *[none] Mark a reading
        }
# Copies the reading while none is in view (greyed out).
graph-menu-copy = Copy
# Copies the reading: $digits are its digits as the meter showed them and
# its unit, e.g. 3.249 V.
graph-menu-copy-digits = Copy { $digits }
# Deletes the marker under the pointer, or the one whose flag was
# right-clicked. $number is its number.
graph-menu-delete-marker = Delete marker { $number }
# Puts the first cursor on the reading, while none is placed. $reading and
# $time are as for Mark.
graph-menu-put-cursor = Put a cursor
    .a11y =
        { $reading ->
            [some] Put a cursor on the reading at { $time }
           *[none] Put a cursor on a reading
        }
# Puts the cursor not yet placed, $cursor, on the reading. $reading and
# $time are as for Mark.
graph-menu-put-cursor-named = Put cursor { $cursor }
    .a11y =
        { $reading ->
            [some] Put cursor { $cursor } on the reading at { $time }
           *[none] Put cursor { $cursor } on a reading
        }
# Moves cursor $cursor, already placed, to the reading. $reading and $time
# are as for Mark.
graph-menu-move-cursor = Move cursor { $cursor }
    .a11y =
        { $reading ->
            [some] Move cursor { $cursor } to the reading at { $time }
           *[none] Move cursor { $cursor } to a reading
        }
# Removes the cursor under the pointer, $cursor.
graph-menu-remove-cursor = Remove cursor { $cursor }
# Removes both cursors and turns Cursors off.
graph-menu-remove-cursors = Remove cursors
# Draws a reference line at the height clicked, $height (see
# graph-menu-height-heading).
graph-menu-add-ref = Add reference line
    .a11y = Add reference line at { $height }
# Removes the reference line under the pointer, at $height, a number with
# its unit.
graph-menu-remove-ref = Remove reference line { $height }
# Sets one of the alarm's two limits at the height clicked: $side is high
# for the upper limit, low for the lower. The alarm, set up beside the
# reading, warns when a reading goes past a limit. $height as above.
graph-menu-set-alarm =
    { $side ->
        [high] Set alarm high
       *[low] Set alarm low
    }
    .a11y =
        { $side ->
            [high] Set alarm high at { $height }
           *[low] Set alarm low at { $height }
        }
# Removes the alarm limit whose line is under the pointer; $side as above.
graph-menu-remove-alarm =
    { $side ->
        [high] Remove alarm high
       *[low] Remove alarm low
    }
