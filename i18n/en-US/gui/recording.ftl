### The recording panel under the graph (docs/images/gui-main.png): its
### Record, Stop and Discard buttons beside Import… and Export…, the count
### of samples recorded, the line saying what Export… saves while nothing
### is recorded, and the messages its buttons show when they are done; the
### dividers dragged to resize it and the reading panel; and the big meter
### toggle at the right end of the reading's buttons, which hides every
### panel but the reading.
###
### A sample is one reading the meter sent. The buttons sit in one row with
### Import… and Export…, so keep them short. $keys is a keyboard shortcut as
### this computer writes it, e.g. Ctrl+R or ⌘R.

## The row of buttons under the graph, after Import….

# A button; ● is part of it.
recording-record = ● Record
    .hover = Start writing live samples to the recording buffer ({ $keys })
# The Record button while recording; ■ is part of it.
recording-stop = ■ Stop
    .hover = Stop recording ({ $keys })
# Shown after Export… once a recording has stopped. Export… is the button
# beside it, which saves the samples to a file.
recording-discard = Discard
    .hover = Discard the recording; Export then saves samples from the graph
# After the buttons while recording. $count is how many samples the
# recording holds; $seconds how long it has run, in whole seconds, e.g. 12.
# The s after it is the symbol for seconds.
recording-status =
    { $count ->
        [one] { $count } sample | { $seconds }s
       *[other] { $count } samples | { $seconds }s
    }
# The same, in a warning colour, once the recording holds as many samples
# as the Buffer size setting allows and takes no more; $count and $seconds
# as there.
recording-status-full =
    { $count ->
        [one] { $count } sample | { $seconds }s (buffer full)
       *[other] { $count } samples | { $seconds }s (buffer full)
    }
# After the buttons once a recording has stopped. $count is how many
# samples it holds.
recording-count =
    { $count ->
        [one] { $count } sample
       *[other] { $count } samples
    }
# A small line under the buttons while nothing is recorded: Export… then
# saves the samples the graph holds, which a mode change restarts, where a
# recording keeps them. Export and Record are the buttons' names. $count is
# how many samples the graph holds.
recording-history-hint =
    { $count ->
        [one] No recording. Export saves { $count } sample from the graph. Record to capture across mode changes.
       *[other] No recording. Export saves { $count } samples from the graph. Record to capture across mode changes.
    }

## Messages that show for a few seconds at the top right.

# After Discard, when the graph holds none of the recording's samples.
recording-discarded = Recording discarded
# After Discard, when the graph still holds samples. Export… is the button
# that saves them. $count is how many it holds.
recording-discarded-graph-keeps =
    { $count ->
        [one] Recording discarded. Export… saves the graph's { $count } sample.
       *[other] Recording discarded. Export… saves the graph's { $count } samples.
    }
# After Copy in the graph's right-click menu. $digits is the reading as the
# meter showed it, e.g. 1.6109, which is now on the clipboard.
recording-copied = Copied { $digits }

## The dividers dragged to resize the panels. Read by screen readers only:
## a divider has no text.

# Between the graph and the recording panel. Up and Down are the arrow
# keys.
recording-resize =
    .a11y = Resize recording panel (Up/Down to adjust)
# At the right edge of the reading panel, on a wide window. Left and Right
# are the arrow keys.
recording-reading-resize =
    .a11y = Resize reading panel (Left/Right to adjust)

## The big meter toggle: a ⊞ or ⊟ button at the right end of the reading's
## last row of buttons. Its hover is also what screen readers read.

recording-big-meter-on =
    .hover = Hide side panels and show the meter reading full-screen ({ $keys })
# While the reading is full-screen.
recording-big-meter-off =
    .hover = Return to the normal multi-panel layout ({ $keys })
