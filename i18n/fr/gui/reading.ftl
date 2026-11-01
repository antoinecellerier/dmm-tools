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
    .hover = Passer le multimètre dans un autre mode de la position actuelle du commutateur
    .a11y = Mode
# The range readout as a dropdown of the meter's ranges in this mode.
reading-range =
    .hover = Passer le multimètre sur une autre plage du mode actuel
    .a11y = Plage de mesure
# The mode readout on a meter that lists its function keys instead of its
# modes: the value is the caption over the open list, which says its entries
# are key presses. A screen reader still calls the dropdown Mode.
reading-keys = Touches du multimètre
    .hover = Appuyer sur une des touches de fonction du multimètre
    .a11y = Mode

## The badges after the meter's own, in capitals like them and as short:
## they sit on the mode line, which a narrow window wraps.

# Shown while a Scale turns the meter's reading into another quantity.
reading-badge-scaled = ÉCHELLE
# Shown, in the error colour, while the reading is above the alarm's high
# limit.
reading-badge-above = LIM. HAUTE
# Shown, in the error colour, while the reading is below the alarm's low
# limit.
reading-badge-below = LIM. BASSE

## What stands where the reading goes when there is none.

# Beside or under dashes (---) while no meter is connected or none has sent
# a reading; also what a screen reader says there.
reading-none = Aucune mesure
# The big meter's second line under the connection problem that takes the
# reading's place: hovering it shows the steps, and $keys (the shortcut that
# turns the big meter off, e.g. Ctrl+B or ⌘B) brings the whole window back.
reading-big-meter-hint = { $keys } pour les détails

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
        [one] { $label } { $value } à { $seconds } seconde
       *[other] { $label } { $value } à { $seconds } secondes
    }
# Said in place of the digits when the input is past the range (OL).
reading-spoken-overload = surcharge
# The non-contact voltage detector's level; $level is a number from 1 up.
reading-spoken-ncv = niveau NCV { $level }
# A word the meter shows in place of the digits, $word (e.g. ---- or Auto),
# which says there is no reading.
reading-spoken-no-reading = pas de mesure ({ $word })
# The units °C and °F spelled out, since screen readers differ on whether
# they read the ° sign. C and F stay as letters, after the number.
reading-spoken-celsius = degrés C
reading-spoken-fahrenheit = degrés F

## The meter's badges spoken, one phrase each.

# HV!: the probes are on a dangerous voltage.
reading-spoken-hv-warning = avertissement de tension dangereuse
# AUTO: the meter picks its range itself.
reading-spoken-auto-range = plage automatique
# HOLD: the display is frozen.
reading-spoken-hold = maintien de l’affichage
# REL: the reading is relative to a stored one.
reading-spoken-rel = relatif
# MIN: the lowest reading since MIN/MAX began.
reading-spoken-min = minimum
# MAX: the highest reading since MIN/MAX began.
reading-spoken-max = maximum
# AVG: the mean of the readings since MIN/MAX began.
reading-spoken-avg = moyenne
# P-MIN: the lowest peak.
reading-spoken-peak-min = crête minimale
# P-MAX: the highest peak.
reading-spoken-peak-max = crête maximale
# LOW BAT: the meter's battery is low.
reading-spoken-low-battery = pile faible
# LEAD ERR: the meter reports a problem with the test leads.
reading-spoken-lead-error = erreur de branchement des cordons
# COMP: the meter's compare mode, which checks the reading against limits.
reading-spoken-comp = comparaison
# REC: the meter is recording readings into its own memory.
reading-spoken-record = enregistrement
# LoZ: the low-impedance voltage mode.
reading-spoken-loz = basse impédance
# VOID: the reading is not valid.
reading-spoken-void = non valide

## The app's own badges spoken, after the meter's.

# reading-badge-scaled.
reading-spoken-scaled = mise à l’échelle logicielle
# reading-badge-above.
reading-spoken-above = au-dessus de la limite haute
# reading-badge-below.
reading-spoken-below = en dessous de la limite basse
