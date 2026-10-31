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
settings-device = Device:
    .a11y = Device
# The Device dropdown, and its first entry, when no particular meter is
# picked: each kind of meter is tried in turn.
settings-auto-detect = Auto-detect
# The Device dropdown while --device picked the meter for this session.
# $name is the meter, e.g. UNI-T UT61E+, or Auto-detect.
settings-device-pinned = { $name } (--device)
# Beside the greyed Device dropdown while a recording is played back:
# --replay is the command-line option that opened it.
settings-device-replay = (restart without --replay to pick a meter)
# Beside the greyed Device dropdown while the session runs on a simulated
# clock, set by command-line options for testing.
settings-device-clock = (restart without the clock flags to pick a meter)

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
        [yes] Auto (cycle) (--mock-mode)
       *[no] Auto (cycle)
    }
    .hover = Cycle through all synthetic modes to exercise the GUI
# The chip for the mode --mock-mode picked for this session. $mode is its
# name, e.g. dcv.
settings-mock-mode-pinned = { $mode } (--mock-mode)
settings-mock-mode = Mock mode:

## How a connection is made: checkboxes on one row.

settings-auto-connect = Auto-connect on start
    .hover = Open the USB connection automatically when the app launches
# Asking the meter its name makes it beep.
settings-device-name = Show device name on connect (beeps)
    .hover = Query the meter's name after connecting — the meter will beep once
# The Bluetooth checkbox while --no-bluetooth switched the search off for
# this session. $setting is the checkbox's own label, e.g. Look for
# Bluetooth devices.
settings-bluetooth-pinned = { $setting } (--no-bluetooth)
settings-bluetooth =
    .hover = Look for a Bluetooth adapter or meter in range when no USB cable answers. Takes effect on the next connect.

## What is kept of the readings.

settings-sample-interval = Sample interval:
# The first Sample interval chip; the others are durations, e.g. 300ms or 2s.
settings-interval-every-reading = Every reading
    .hover = Keep every reading the meter produces, at its own pace
# The other Sample interval chips' hover. $unit is second for exactly one
# second, seconds for a whole number of seconds and ms otherwise; $n is how
# many seconds or milliseconds. s and ms are unit symbols.
settings-interval-chip =
    .hover =
        { $unit ->
            [second] At most one reading a second: the one nearest each tick
            [seconds] At most one reading every { $n } s: the one nearest each tick
           *[ms] At most one reading every { $n } ms: the one nearest each tick
        }

settings-buffer-size = Buffer size:
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
                    [one] Keep up to { $samples } sample in the graph and for export — { $memory }, about { $span } at 10 readings a second. A stopped recording kept beside them can take as much again
                   *[other] Keep up to { $samples } samples in the graph and for export — { $memory }, about { $span } at 10 readings a second. A stopped recording kept beside them can take as much again
                }
           *[interval]
                { $count ->
                    [one] Keep up to { $samples } sample in the graph and for export — { $memory }, about { $span } at the current sample interval. A stopped recording kept beside them can take as much again
                   *[other] Keep up to { $samples } samples in the graph and for export — { $memory }, about { $span } at the current sample interval. A stopped recording kept beside them can take as much again
                }
        }
# After the Buffer size chips: what the picked size costs. $memory is the
# memory it takes, e.g. ≈194 MB; $span how long it lasts, e.g. 14 h.
settings-buffer-cost = ({ $memory }, about { $span })

## Which panels show, and what the Specifications panel shows: checkboxes
## on one row.

settings-graph = Graph
    .hover = Show the rolling time-series plot
# σ is the standard deviation, as the Statistics panel writes it.
settings-statistics = Statistics
    .hover = Show Min / Max / Mean / σ / integral for the live session
settings-recording = Recording
    .hover = Show the recording controls and sample log
# The Specifications panel's checkbox. $fields is shown while the panel is
# on and the checkboxes for its fields follow it, hidden otherwise.
settings-specs =
    { $fields ->
        [shown] Specifications:
       *[hidden] Specifications
    }
    .hover = Show accuracy and resolution for the current mode
# The Specifications fields, as that panel names them.
settings-specs-resolution = Resolution
    .hover = Show the resolution in the specifications
settings-specs-accuracy = Accuracy
    .hover = Show the accuracy in the specifications
# Input Z is the input impedance.
settings-specs-input-z = Input Z
    .hover = Show the input impedance in the full Specifications panel
settings-specs-notes = Notes
    .hover = Show the manual's notes in the full Specifications panel

## The window.

# The Zoom chips are percentages, e.g. 150%.
settings-zoom = Zoom:
# A Zoom chip's hover. $level is its scale in percent, e.g. 150; $step the
# keys that step the zoom.
settings-zoom-chip =
    .hover = Scale the GUI to { $level }% ({ $step } to step)
# The 100% chip's hover. $reset is the keys that go back to 100%, $step
# those that step the zoom.
settings-zoom-reset-chip =
    .hover = Scale the GUI to 100% ({ $reset }, or { $step } to step)
# Window decorations are the title bar and border.
settings-hide-decorations = Hide window decorations
    .hover = Borderless window — use { $keys } to toggle back
# On Wayland the checkbox is greyed: .wayland is its hover there, and
# settings-always-on-top-wayland the note beside it. Keep both the same as
# shortcuts-always-on-top-toast, which Ctrl+T shows there.
settings-always-on-top = Always on top
    .hover = Keep the window above other desktop windows ({ $keys })
    .wayland = Not available on Wayland — right-click the title bar to keep the window above others
# The note after the greyed checkbox on Wayland, in small grey text.
settings-always-on-top-wayland = (Not available on Wayland — right-click the title bar to keep the window above others)

## The update check, in builds downloaded from GitHub.

settings-check-updates = Check for new versions
    .hover = Once a day, ask GitHub whether a newer release is out. GitHub sees your IP address; nothing about your meter is sent.
# The link the top bar shows when a newer release is out; ↗ marks a link.
# $version is the release, e.g. v0.10.0.
settings-update-release = { $version } available ↗
# The same for a newer nightly build, which has no version number.
settings-update-dev = Newer dev build ↗
# Its shorter form, when the top bar is short of room.
settings-update-dev-short = Update ↗

## How the graph's lines look, the last row.

settings-graph-lines = Graph lines:
# Sub-value lines are the extra traces a meter that shows several values
# at once draws beside the main one.
settings-graph-lines-patterned = Patterned
    .hover = Dashed and dotted sub-value lines, told apart without colour
settings-graph-lines-solid = Solid
    .hover = Continuous lines, told apart by colour and the key
