### The Statistics panel (assets/gui-wide-layout.webp): the figures of the
### meter's main reading since the last Reset (Session), and of the samples
### the graph shows (Visible), as one table. Then the marker list under the
### Record row: each marker's reading with its number, its note and a
### button deleting it, among the recorded samples while recording.
###
### A sample is one reading the meter sent. A marker is a numbered flag on
### a reading, with a note, drawn on the graph and listed under the Record
### row.

## The Statistics panel. Its table has a column per group in the reading
## column (a row per group where the window is wide enough), each group
## under its name, how long its readings span and how many there are; a
## narrow window writes each group as a line, "Min 0.00 mA · Max …". The
## captions name a column or a row of figures: keep them short, without a
## colon.

# The panel's heading.
stats-heading = Statistics
# The caption of the lowest reading.
stats-min = Min
# The caption of the highest reading.
stats-max = Max
# The caption of the mean (average) of the readings.
stats-mean = Mean
# The caption of the standard deviation; σ is its usual symbol.
stats-sd = σ
# The caption of the integral of the readings over time; ∫ is its symbol.
stats-integral = ∫
# The name of the figures since the last Reset.
stats-session = Session
# The name of the figures of the samples the graph shows.
stats-visible = Visible
# The name of the figures of the samples between the graph's two cursors,
# which the graph labels A and B; the dash is an en dash. The screen
# reader's name says it in words.
stats-cursors = A–B
    .a11y = Between cursors A and B
# The caption of how far apart the readings at the two cursors are; Δ is
# its usual symbol.
stats-delta = Δ
# How many readings a group's figures cover, after how long they span (e.g.
# "1m 49s #1103"); # marks a count, $count is the number. A screen reader
# says both in words; $span is written like "1m 49s" or "59 s".
stats-count = #{ $count }
    .a11y =
        { $count ->
            [one] { $count } reading over { $span }
           *[other] { $count } readings over { $span }
        }
# Under the table when readings stopped for a while: a warning that the
# integral left those stretches out. $count is how many; $limit the longest
# stretch still bridged, in seconds, e.g. 2; the s after it is the symbol
# for seconds. ⚠ is part of it. The hover explains it: 5 and 2 are fixed
# numbers.
stats-gaps =
    { $count ->
        [one] ⚠ { $count } gap >{ $limit }s skipped
       *[other] ⚠ { $count } gaps >{ $limit }s skipped
    }
    .hover = Stretches of more than { $limit } s without a reading are not integrated: 5 sample intervals, and at least 2 s. The integral is partial.
# While the Alarm row under the reading has limits set: how many times the
# reading went past each. $limits is both while there is a high and a low
# limit, high or low while there is only that one; $high and $low are the
# counts.
stats-alarm-counts =
    { $limits ->
        [both] Alarms: { $high } high, { $low } low
        [high] Alarms: { $high } high
       *[low] Alarms: { $low } low
    }
    .hover = Times the reading crossed each limit, each marked on the graph
# What a screen reader calls the small ↺ button beside Session, which starts
# the session's figures over; its hover says what it clears. σ is the
# standard deviation.
stats-reset = Reset
    .hover = Reset Min / Max / Mean / σ / integral and alarm counters

## The marker list under the Record row, and what N, Ctrl+N and a click on
## a marker's flag say at the top right. Notes are written in the list,
## which shows only while the Recording panel does.

# When N is pressed before any reading came in, or on a reading the graph
# and the recording no longer hold.
markers-no-reading = No reading to mark yet
# When N is pressed on a reading that already has a marker; $number is its
# number.
markers-already-marked = Marker { $number } is already on this reading
# After N or Ctrl+N marks a reading while the list is hidden: $number is the
# new marker's number. $hidden is big-meter while the big meter mode hides
# every panel but the reading, recording while the Recording panel is
# switched off; "Recording" is the name of its checkbox in Settings.
markers-added =
    { $hidden ->
        [big-meter] Marker { $number } added. Leave big meter mode to add a note.
       *[recording] Marker { $number } added. Turn on Recording in Settings to add a note.
    }
# Ctrl+N on a reading that already has a marker while the list is hidden;
# $number and $hidden as in markers-added.
markers-already-marked-hidden =
    { $hidden ->
        [big-meter] Marker { $number } is already on this reading. Leave big meter mode to add a note.
       *[recording] Marker { $number } is already on this reading. Turn on Recording in Settings to add a note.
    }
# A click on a marker's flag on the graph while the list is hidden; $hidden
# as in markers-added.
markers-note-hidden =
    { $hidden ->
        [big-meter] Leave big meter mode to add a note.
       *[recording] Turn on Recording in Settings to add a note.
    }
# Over the list when a recording is too long to list in full; Export…
# saves every sample to a file. $time is the time of the first sample
# listed, e.g. 14:02:37.
markers-log-cut = Export to see the samples before { $time }.
# When Ctrl+N cannot show the note it opened because the panel is too short
# for a single row; $number is the marker's number.
markers-log-too-short = Make the Recording panel taller to write marker { $number }'s note.
# After a marker's reading, in small text, when the graph no longer holds
# that reading.
markers-off-graph = not on the graph
# After a marker's reading, in small text, when it was placed on a reading
# from before the recording started.
markers-off-recording = not in the recording
# The hover, and the name a screen reader reads, of the + a sample's row
# shows under the pointer; $time is the sample's time, e.g. 14:02:37.512.
markers-add = Add a marker at { $time }
# A marker's number, drawn as its flag: the hover and the name a screen
# reader reads. Clicking it scrolls the graph to the marker; $number is its
# number. .hover-disabled shows while the graph no longer holds its reading.
markers-show = Show marker { $number } on the graph
    .hover-disabled = Marker { $number }'s reading is no longer on the graph
# The field a marker's note is typed in. The text shows greyed inside it
# while it is empty; .a11y is its name for a screen reader, $number the
# marker's number.
markers-note = Add a note
    .a11y = Note for marker { $number }
# The × button at the end of a marker's row: its hover, and the name a
# screen reader reads; $number is the marker's number.
markers-delete = Delete marker { $number }
