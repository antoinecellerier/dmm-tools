### What the library says about a threshold alarm: the limits it watches
### and the limit a reading went past. dmm-gui shows them in its toasts (the
### short messages under the top bar's right end), dmm-cli on the lines it
### prints while it reads. Hand-written; a test checks the English against
### the library's own.
###
### Numbers arrive formatted, and unit symbols (V, Ω, °C) stay as they are.

## The limits an alarm watches, after "Alarm on:" in dmm-gui and "alarm on:"
## in dmm-cli (cli/read.ftl).

# Which limits are set: $limits is both, high or low. $high and $low are
# the limits as numbers, without their unit, e.g. 5 or -0.25.
alarm-limits =
    { $limits ->
        [both] por encima de { $high } o por debajo de { $low }
        [high] por encima de { $high }
       *[low] por debajo de { $low }
    }
# With a hysteresis band given: how far back a reading must come before the
# same limit alarms again. $limits is the text of alarm-limits; $band is a
# number, or a percentage of the limit such as 1%.
alarm-hysteresis = { $limits }, histéresis de { $band }

## A reading past a limit: dmm-gui's toast, and dmm-cli's line after
## "Alarm:" or "Note:" (cli/read.ftl). alarm-breach is also the note of
## the marker on that reading, kept in recordings and exports. $side is
## high (the high limit) or low; $limit is the limit with its unit, e.g.
## 5 V.

# The reading crossed the limit: the alarm goes off.
alarm-breach =
    { $side ->
        [low] Por debajo del límite inferior de { $limit }
       *[high] Por encima del límite superior de { $limit }
    }
# The reading was already past the limit when the alarm started watching
# it: said, but not counted, and the alarm waits for the reading to come
# back and cross again.
alarm-breach-already =
    { $side ->
        [low] La lectura ya está por debajo del límite inferior de { $limit }. Las alarmas se activan a partir del próximo cruce
       *[high] La lectura ya está por encima del límite superior de { $limit }. Las alarmas se activan a partir del próximo cruce
    }
