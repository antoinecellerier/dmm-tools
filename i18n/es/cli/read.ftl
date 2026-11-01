### What 'dmm-cli read' prints besides the readings: why it refuses to
### start, the notes as it starts and runs, and the summary when it ends.
### These lines go to the terminal's error stream; the readings themselves,
### and any file they are written to, stay as they are.
###
### Words in --flag form, and their values ('max', replay), are typed by the
### user and stay as they are, as do unit symbols (V, Ω, s). Command lines
### arrive as variables.

## Refusals before anything is opened, after "Error:" (common.ftl).

# An option, or several, then why its value can't be used. $flags is the
# option as typed, e.g. --alarm-hysteresis, or several of them, e.g.
# "--alarm-low, --alarm-high"; $reason is the reason, in lower case.
read-flag-refused = { $flags }: { $reason }
read-alarm-bell-needs-limit = --alarm-bell necesita --alarm-high o --alarm-low
# The hysteresis band says how far back a reading must come before a limit
# alarms again, so it needs a limit.
read-alarm-hysteresis-needs-limit = --alarm-hysteresis necesita --alarm-high o --alarm-low
# --mock-clock-scale and --mock-clock-preseed are contributor options that
# speed a simulated run up: 'max' plays a recording as fast as it decodes, a
# preseed starts a run with readings already behind it. After
# read-flag-refused's options.
read-clock-max-no-preseed = 'max' no admite precarga: cada trama ya se reproduce sin esperar
read-clock-max-needs-replay = 'max' necesita --replay: solo una grabación termina por sí sola
# A recording names the meter it was made with. $command is the command line
# that plays one (dmm-cli read --replay FILE).
read-replay-names-its-device = el archivo de --replay ya indica su multímetro: quita --device ({ $command })
# An exported CSV or JSON file names the meter it was read from.
read-import-names-its-device = el archivo de --import ya indica su multímetro: quita --device
# --format replay writes the frames a meter sent. $device is the simulated
# meter's id, e.g. mock.
read-replay-format-needs-meter = --format replay necesita un multímetro real: no hay nada que grabar con --device { $device }
# $flag is the option given that would change the readings, e.g. --scale or
# --integrate.
read-replay-format-keeps-frames = un archivo de reproducción guarda las tramas del propio multímetro: indica { $flag } al reproducirlo
read-import-no-frames = un archivo CSV o JSON no contiene tramas del multímetro que escribir como reproducción: para eso, usa --import con un archivo de reproducción
read-import-no-interval = --interval-ms selecciona las lecturas a medida que un multímetro las envía: un archivo ya trae todas sus lecturas

## Why --mock-clock-scale's value was refused, at the end of a line from the
## command-line parser, which stays English: "error: invalid value 'abc' for
## '--mock-clock-scale <MOCK_CLOCK_SCALE>': …".

# $value is what was typed.
read-clock-scale-invalid = se esperaba un número o 'max', se recibió '{ $value }'

## Notes as the run starts, each after "Note:" (common.ftl).

# --format named one format and the -o file's name another; the format wins.
# $format is the format given, e.g. csv; $path is the file.
read-format-mismatch = --format { $format } escrito en { $path }
# A recording played into a file takes as long as the recording did. $flag
# is the option that skips the wait (--mock-clock-scale max).
read-paced-replay = reproducción al ritmo de la grabación: usa { $flag } para convertir sin esperar
# --scale, --offset and --unit apply arithmetic to every reading. $transform
# is that arithmetic, e.g. ×2 - 1 → A.
read-scaled = lecturas escaladas por software ({ $transform })
# $limits is what the alarm watches for (alarm-limits in lib-alarm.ftl),
# e.g. "above 5 or below 3, hysteresis 1%".
read-alarm-on = alarma activada: { $limits }

## While the readings arrive.

# After "Note:". The statistics (and with --integrate the integral) start
# again when the readings change what they measure. $change is mode (the
# dial was turned), unit (the range moved), or either of them followed by
# -integral when the integral restarts too. $before and $after are the mode
# names, e.g. DC V and Ω, or the units, e.g. mV and V.
read-series-reset =
    { $change ->
        [mode-integral] Cambio de modo ({ $before } → { $after }), estadísticas e integral restablecidas
        [unit] Cambio de unidad ({ $before } → { $after }), estadísticas restablecidas
        [unit-integral] Cambio de unidad ({ $before } → { $after }), estadísticas e integral restablecidas
       *[mode] Cambio de modo ({ $before } → { $after }), estadísticas restablecidas
    }
# After "Note:". The readings are now of another quantity than the alarm's
# limits, which it stops judging them against. $now is the readings' unit,
# $watched the limits' unit, e.g. Ω and V.
read-alarm-idle = alarma en espera: lecturas en { $now }, límites en { $watched }
# After "Note:". The readings are back in the limits' unit, $watched, e.g. V.
read-alarm-watching = la alarma vuelve a vigilar las lecturas en { $watched }
# Red and bold, opening the line that says a reading crossed a limit; the
# terminal may ring. Keep the colon, or your language's equivalent: the
# sentence follows after a space.
read-alarm-label = Alarma:
# After "Alarm:". $breach says which limit was crossed (alarm-breach in
# lib-alarm.ftl), e.g. "Above high limit 5 V"; $reading is the reading with
# its unit, e.g. 5.012 V.
read-alarm-breach = { $breach } ({ $reading })
# After "Warning:". A frame the meter sent couldn't be read and the run goes
# on without it. $error says why (lib-errors.ftl); $count is how many have
# been skipped so far.
read-unreadable-frame =
    { $count ->
        [one] { $error } ({ $count } trama omitida hasta ahora)
       *[other] { $error } ({ $count } tramas omitidas hasta ahora)
    }
# After "Error:" (common.ftl), ending a --format replay run: a reading came
# with no frame from the meter behind it, which a replay can't hold. $mode is
# the reading's mode name (lib.ftl), e.g. DC V.
read-replay-no-frame = una lectura en modo { $mode } no tiene ninguna trama del multímetro que grabar

## The summary when the run ends, under a blank line.

# After "Note:". $count is how many readings couldn't be read.
read-skipped-frames =
    { $count ->
        [one] { $count } lectura omitida (trama ilegible)
       *[other] { $count } lecturas omitidas (tramas ilegibles)
    }
## After a dim "---": the statistics of the last series of readings, since
## the mode or the unit last changed, one cell per message, joined by " | ":
##
##     --- 3 readings | Min: 0.0264 V | Max: 1.6112 V | Mean: 0.6140 V | σ: 0.8682 V
##
## Min, Max and Mean are the statistics' names in dmm-gui too.

# $count is how many readings.
read-summary-count =
    { $count ->
        [one] { $count } lectura
       *[other] { $count } lecturas
    }
# $min is a number with its unit, e.g. 4.0010 V; the same for the next three.
read-summary-min = Mín.: { $min }
# $max: the largest reading.
read-summary-max = Máx.: { $max }
# $mean: the readings' mean.
read-summary-mean = Media: { $mean }
# $sd: the readings' standard deviation, σ. Left out after a single reading.
read-summary-sd = σ: { $sd }
# Under the summary, with --integrate: the readings summed over time. $value
# is the integral with its unit, e.g. 0.7720 V·s.
read-integral = Integral: { $value }
# The same, with how long the integral ran: $value as there, and $seconds,
# e.g. 1.8, before "s", the symbol for seconds.
read-integral-timed = Integral: { $value } ({ $seconds }s)
# After "Note:", under the integral: gaps in the readings the integral leaves
# out. $count is how many; $max the longest interval it bridges, in seconds
# (s). The rule's other figures, 5 intervals and 2 s, are fixed.
read-skipped-intervals =
    { $count ->
        [one] { $count } intervalo omitido (ninguna lectura durante más de { $max } s, 5 intervalos de muestreo y al menos 2 s: la integral es parcial)
       *[other] { $count } intervalos omitidos (ninguna lectura durante más de { $max } s, 5 intervalos de muestreo y al menos 2 s: la integral es parcial)
    }
# Under the summary: how many times the readings crossed each limit.
# $limits is both, high or low: the limits the alarm had. $above and $below
# are the counts; $high and $low the limits with their unit, e.g. 5 V.
read-alarm-counts =
    { $limits ->
        [both] Alarmas: { $above } por encima de { $high }, { $below } por debajo de { $low }
        [high] Alarmas: { $above } por encima de { $high }
       *[low] Alarmas: { $below } por debajo de { $low }
    }
# -o without a file name: the run named the file after its first reading.
# $path is the file.
read-written-to = Archivo escrito: { $path }
# -o without a file name, and no reading came to name the file after.
read-nothing-written = No llegó ninguna lectura, así que no se escribió ningún archivo
