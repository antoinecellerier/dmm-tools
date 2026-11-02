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
    .hover = Switch the meter to another mode of the current dial position
    .a11y = Mode
# The range readout as a dropdown of the meter's ranges in this mode.
reading-range =
    .hover = Switch the meter to another range of the current mode
    .a11y = Range
# The mode readout on a meter that lists its function keys instead of its
# modes: the value is the caption over the open list, which says its entries
# are key presses. A screen reader still calls the dropdown Mode.
reading-keys = Meter keys
    .hover = Press one of the meter's function keys
    .a11y = Mode

## The badges after the meter's own, in capitals like them and as short:
## they sit on the mode line, which a narrow window wraps.

# Shown while a Scale turns the meter's reading into another quantity.
reading-badge-scaled = SCALE
# Shown, in the error colour, while the reading is above the alarm's high
# limit.
reading-badge-above = HI LIMIT
# Shown, in the error colour, while the reading is below the alarm's low
# limit.
reading-badge-below = LO LIMIT

## What stands where the reading goes when there is none.

# Beside or under dashes (---) while no meter is connected or none has sent
# a reading; also what a screen reader says there.
reading-none = No reading
# The big meter's second line under the connection problem that takes the
# reading's place: hovering it shows the steps, and $keys (the shortcut that
# turns the big meter off, e.g. Ctrl+B or ⌘B) brings the whole window back.
reading-big-meter-hint = { $keys } for details

## What a screen reader says for the reading: never drawn, read aloud each
## time the reading changes. It is one list of phrases, in this order: the
## value, the mode, each sub-value, the meter's badges, then the app's;
## reading-spoken-separator goes between two phrases. For example:
## "DC 1.6112 V, AC+DC V, AC 0.0123 V, hold, software scaled".

# Between two phrases of the list.
reading-spoken-separator = {", "}
# The reading, or a sub-value, with the name drawn before it. $label is the
# name (DC, Max, T2, Frequency…); $value is the value with its unit, e.g.
# "1.6112 V", "overload Ω" or "24.10 degrees C".
reading-spoken-labelled = { $label } { $value }
# A sub-value the meter caught $seconds seconds after MIN/MAX began,
# drawn as "@12s". $label and $value as in reading-spoken-labelled.
reading-spoken-labelled-at =
    { $seconds ->
        [one] { $label } { $value } at { $seconds } second
       *[other] { $label } { $value } at { $seconds } seconds
    }
# Said in place of the digits when the input is past the range (OL).
reading-spoken-overload = overload
# The non-contact voltage detector's level; $level is a number from 1 up.
reading-spoken-ncv = NCV level { $level }
# A word the meter shows in place of the digits, $word (e.g. ---- or Auto),
# which says there is no reading.
reading-spoken-no-reading = no reading ({ $word })
# The units °C and °F spelled out, since screen readers differ on whether
# they read the ° sign. C and F stay as letters, after the number.
reading-spoken-celsius = degrees C
reading-spoken-fahrenheit = degrees F

## Unit names, said after a number in place of the unit's symbol, which a
## screen reader spells out letter by letter (mV as "m V"). The screen keeps
## the symbols; only the spoken reading says these.
##
## $value is the number the name follows, as the screen shows it, e.g.
## 1.000 or -0.52, to choose the plural by; after "overload", or in a mode's
## name (DC volts), it is no number and the last form is taken. $prefix is
## the SI prefix's word below, empty for none, and leads the name. Only
## reading-unit-ohm is also given $scale, which prefix it is (none, pico,
## nano, micro, milli, kilo, mega or giga), for a pair the language writes
## its own way, as English contracts kilohm and megohm. reading-unit-percent
## is given none of these.

# The SI prefixes, as they lead a unit's name (milli in millivolt).
reading-unit-prefix-pico = pico
reading-unit-prefix-nano = nano
reading-unit-prefix-micro = micro
reading-unit-prefix-milli = milli
reading-unit-prefix-kilo = kilo
reading-unit-prefix-mega = mega
reading-unit-prefix-giga = giga
reading-unit-volt =
    { $prefix }{ $value ->
        [one] volt
       *[other] volts
    }
reading-unit-ampere =
    { $prefix }{ $value ->
        [one] ampere
       *[other] amperes
    }
reading-unit-ohm =
    { $scale ->
        [kilo]
            { $value ->
                [one] kilohm
               *[other] kilohms
            }
        [mega]
            { $value ->
                [one] megohm
               *[other] megohms
            }
       *[other]
            { $prefix }{ $value ->
                [one] ohm
               *[other] ohms
            }
    }
reading-unit-hertz =
    { $prefix }{ $value ->
       *[other] hertz
    }
reading-unit-farad =
    { $prefix }{ $value ->
        [one] farad
       *[other] farads
    }
reading-unit-siemens =
    { $prefix }{ $value ->
       *[other] siemens
    }
reading-unit-watt =
    { $prefix }{ $value ->
        [one] watt
       *[other] watts
    }
reading-unit-volt-ampere =
    { $prefix }{ $value ->
        [one] volt-ampere
       *[other] volt-amperes
    }
reading-unit-henry =
    { $prefix }{ $value ->
        [one] henry
       *[other] henries
    }
reading-unit-second =
    { $prefix }{ $value ->
        [one] second
       *[other] seconds
    }
# The % sign; it takes no prefix.
reading-unit-percent = percent
# A number and its unit's name: $value is the number as the screen shows
# it, e.g. 1.6112, or a word such as "overload"; $unit is one of the names
# above, e.g. millivolts. $kind is percent for a number followed by
# reading-unit-percent, for a language that says percent before the
# number, and other otherwise.
reading-spoken-amount =
    { $kind ->
        [percent] { $value } { $unit }
       *[other] { $value } { $unit }
    }

## The meter's badges spoken, one phrase each.

# HV!: the probes are on a dangerous voltage.
reading-spoken-hv-warning = high voltage warning
# AUTO: the meter picks its range itself.
reading-spoken-auto-range = auto range
# HOLD: the display is frozen.
reading-spoken-hold = hold
# REL: the reading is relative to a stored one.
reading-spoken-rel = relative
# MIN: the lowest reading since MIN/MAX began.
reading-spoken-min = minimum
# MAX: the highest reading since MIN/MAX began.
reading-spoken-max = maximum
# AVG: the mean of the readings since MIN/MAX began.
reading-spoken-avg = average
# P-MIN: the lowest peak.
reading-spoken-peak-min = peak minimum
# P-MAX: the highest peak.
reading-spoken-peak-max = peak maximum
# LOW BAT: the meter's battery is low.
reading-spoken-low-battery = low battery
# LEAD ERR: the meter reports a problem with the test leads.
reading-spoken-lead-error = lead error
# COMP: the meter's compare mode, which checks the reading against limits.
reading-spoken-comp = compare
# REC: the meter is recording readings into its own memory.
reading-spoken-record = recording
# LoZ: the low-impedance voltage mode.
reading-spoken-loz = low impedance
# VOID: the reading is not valid.
reading-spoken-void = void

## The app's own badges spoken, after the meter's.

# reading-badge-scaled.
reading-spoken-scaled = software scaled
# reading-badge-above.
reading-spoken-above = above the high limit
# reading-badge-below.
reading-spoken-below = below the low limit
