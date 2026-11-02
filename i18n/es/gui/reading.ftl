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
    .hover = Pasar el multímetro a otro modo de la posición actual del selector
    .a11y = Modo
# The range readout as a dropdown of the meter's ranges in this mode.
reading-range =
    .hover = Pasar el multímetro a otro rango del modo actual
    .a11y = Rango
# The mode readout on a meter that lists its function keys instead of its
# modes: the value is the caption over the open list, which says its entries
# are key presses. A screen reader still calls the dropdown Mode.
reading-keys = Botones del multímetro
    .hover = Presionar uno de los botones de función del multímetro
    .a11y = Modo

## The badges after the meter's own, in capitals like them and as short:
## they sit on the mode line, which a narrow window wraps.

# Shown while a Scale turns the meter's reading into another quantity.
reading-badge-scaled = ESCALA
# Shown, in the error colour, while the reading is above the alarm's high
# limit.
reading-badge-above = LÍM. SUP.
# Shown, in the error colour, while the reading is below the alarm's low
# limit.
reading-badge-below = LÍM. INF.

## What stands where the reading goes when there is none.

# Beside or under dashes (---) while no meter is connected or none has sent
# a reading; also what a screen reader says there.
reading-none = Sin lectura
# The big meter's second line under the connection problem that takes the
# reading's place: hovering it shows the steps, and $keys (the shortcut that
# turns the big meter off, e.g. Ctrl+B or ⌘B) brings the whole window back.
reading-big-meter-hint = { $keys } para más detalles

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
        [one] { $label } { $value } al cabo de { $seconds } segundo
       *[other] { $label } { $value } al cabo de { $seconds } segundos
    }
# Said in place of the digits when the input is past the range (OL).
reading-spoken-overload = sobrecarga
# The non-contact voltage detector's level; $level is a number from 1 up.
reading-spoken-ncv = nivel NCV { $level }
# A word the meter shows in place of the digits, $word (e.g. ---- or Auto),
# which says there is no reading.
reading-spoken-no-reading = sin lectura ({ $word })
# The units °C and °F spelled out, since screen readers differ on whether
# they read the ° sign. C and F stay as letters, after the number.
reading-spoken-celsius = grados C
reading-spoken-fahrenheit = grados F

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
reading-unit-prefix-milli = mili
reading-unit-prefix-kilo = kilo
reading-unit-prefix-mega = mega
reading-unit-prefix-giga = giga
reading-unit-volt =
    { $prefix }{ $value ->
        [one] voltio
       *[other] voltios
    }
reading-unit-ampere =
    { $prefix }{ $value ->
        [one] amperio
       *[other] amperios
    }
reading-unit-ohm =
    { $prefix }{ $value ->
        [one] ohmio
       *[other] ohmios
    }
reading-unit-hertz =
    { $prefix }{ $value ->
        [one] hercio
       *[other] hercios
    }
reading-unit-farad =
    { $prefix }{ $value ->
        [one] faradio
       *[other] faradios
    }
reading-unit-siemens = { $prefix }siemens
reading-unit-watt =
    { $prefix }{ $value ->
        [one] vatio
       *[other] vatios
    }
reading-unit-volt-ampere =
    { $prefix }{ $value ->
        [one] voltamperio
       *[other] voltamperios
    }
reading-unit-henry =
    { $prefix }{ $value ->
        [one] henrio
       *[other] henrios
    }
reading-unit-second =
    { $prefix }{ $value ->
        [one] segundo
       *[other] segundos
    }
# The % sign; it takes no prefix.
reading-unit-percent = por ciento
# A number and its unit's name: $value is the number as the screen shows
# it, e.g. 1.6112, or a word such as "overload"; $unit is one of the names
# above, e.g. millivolts. $kind is percent for a number followed by
# reading-unit-percent, for a language that says percent before the
# number, and other otherwise.
reading-spoken-amount = { $value } { $unit }

## The meter's badges spoken, one phrase each.

# HV!: the probes are on a dangerous voltage.
reading-spoken-hv-warning = advertencia de tensión peligrosa
# AUTO: the meter picks its range itself.
reading-spoken-auto-range = rango automático
# HOLD: the display is frozen.
reading-spoken-hold = retención
# REL: the reading is relative to a stored one.
reading-spoken-rel = relativo
# MIN: the lowest reading since MIN/MAX began.
reading-spoken-min = mínimo
# MAX: the highest reading since MIN/MAX began.
reading-spoken-max = máximo
# AVG: the mean of the readings since MIN/MAX began.
reading-spoken-avg = promedio
# P-MIN: the lowest peak.
reading-spoken-peak-min = pico mínimo
# P-MAX: the highest peak.
reading-spoken-peak-max = pico máximo
# LOW BAT: the meter's battery is low.
reading-spoken-low-battery = batería baja
# LEAD ERR: the meter reports a problem with the test leads.
reading-spoken-lead-error = error de conexión de los cables de prueba
# COMP: the meter's compare mode, which checks the reading against limits.
reading-spoken-comp = comparación
# REC: the meter is recording readings into its own memory.
reading-spoken-record = grabación
# LoZ: the low-impedance voltage mode.
reading-spoken-loz = baja impedancia
# VOID: the reading is not valid.
reading-spoken-void = lectura no válida

## The app's own badges spoken, after the meter's.

# reading-badge-scaled.
reading-spoken-scaled = escalado por software
# reading-badge-above.
reading-spoken-above = por encima del límite superior
# reading-badge-below.
reading-spoken-below = por debajo del límite inferior
