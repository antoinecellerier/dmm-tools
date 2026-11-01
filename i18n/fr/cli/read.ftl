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
read-flag-refused = { $flags } : { $reason }
read-alarm-bell-needs-limit = --alarm-bell demande --alarm-high ou --alarm-low
# The hysteresis band says how far back a reading must come before a limit
# alarms again, so it needs a limit.
read-alarm-hysteresis-needs-limit = --alarm-hysteresis demande --alarm-high ou --alarm-low
# --mock-clock-scale and --mock-clock-preseed are contributor options that
# speed a simulated run up: 'max' plays a recording as fast as it decodes, a
# preseed starts a run with readings already behind it. After
# read-flag-refused's options.
read-clock-max-no-preseed = 'max' n’accepte pas de préremplissage : chaque trame est déjà jouée sans attendre
read-clock-max-needs-replay = 'max' demande --replay : seul un enregistrement se termine de lui-même
# A recording names the meter it was made with. $command is the command line
# that plays one (dmm-cli read --replay FILE).
read-replay-names-its-device = le fichier donné à --replay indique son propre multimètre : retirez --device ({ $command })
# An exported CSV or JSON file names the meter it was read from.
read-import-names-its-device = le fichier donné à --import indique son propre multimètre : retirez --device
# --format replay writes the frames a meter sent. $device is the simulated
# meter's id, e.g. mock.
read-replay-format-needs-meter = --format replay demande un vrai multimètre : rien à enregistrer depuis --device { $device }
# $flag is the option given that would change the readings, e.g. --scale or
# --integrate.
read-replay-format-keeps-frames = un fichier de relecture contient les propres trames du multimètre : indiquez { $flag } en le relisant
read-import-no-frames = un fichier CSV ou JSON ne contient aucune trame de multimètre à écrire en relecture : pour cela, utilisez --import avec un fichier de relecture
read-import-no-interval = --interval-ms trie les mesures au fil de leur envoi par un multimètre : un fichier contient déjà toutes les siennes

## Why --mock-clock-scale's value was refused, at the end of a line from the
## command-line parser, which stays English: "error: invalid value 'abc' for
## '--mock-clock-scale <MOCK_CLOCK_SCALE>': …".

# $value is what was typed.
read-clock-scale-invalid = nombre ou 'max' attendu, reçu '{ $value }'

## Notes as the run starts, each after "Note:" (common.ftl).

# --format named one format and the -o file's name another; the format wins.
# $format is the format given, e.g. csv; $path is the file.
read-format-mismatch = --format { $format } écrit dans { $path }
# A recording played into a file takes as long as the recording did. $flag
# is the option that skips the wait (--mock-clock-scale max).
read-paced-replay = relecture au rythme de l’enregistrement : ajoutez { $flag } pour convertir sans attendre
# --scale, --offset and --unit apply arithmetic to every reading. $transform
# is that arithmetic, e.g. ×2 - 1 → A.
read-scaled = mesures mises à l’échelle par logiciel ({ $transform })
# $limits is what the alarm watches for (alarm-limits in lib-alarm.ftl),
# e.g. "above 5 or below 3, hysteresis 1%".
read-alarm-on = alarme activée : { $limits }

## While the readings arrive.

# After "Note:". The statistics (and with --integrate the integral) start
# again when the readings change what they measure. $change is mode (the
# dial was turned), unit (the range moved), or either of them followed by
# -integral when the integral restarts too. $before and $after are the mode
# names, e.g. DC V and Ω, or the units, e.g. mV and V.
read-series-reset =
    { $change ->
        [mode-integral] Changement de mode ({ $before } → { $after }), statistiques et intégrale réinitialisées
        [unit] Changement d’unité ({ $before } → { $after }), statistiques réinitialisées
        [unit-integral] Changement d’unité ({ $before } → { $after }), statistiques et intégrale réinitialisées
       *[mode] Changement de mode ({ $before } → { $after }), statistiques réinitialisées
    }
# After "Note:". The readings are now of another quantity than the alarm's
# limits, which it stops judging them against. $now is the readings' unit,
# $watched the limits' unit, e.g. Ω and V.
read-alarm-idle = alarme en attente : mesures en { $now }, limites en { $watched }
# After "Note:". The readings are back in the limits' unit, $watched, e.g. V.
read-alarm-watching = l’alarme surveille de nouveau les mesures en { $watched }
# Red and bold, opening the line that says a reading crossed a limit; the
# terminal may ring. Keep the colon, or your language's equivalent: the
# sentence follows after a space.
read-alarm-label = Alarme :
# After "Alarm:". $breach says which limit was crossed (alarm-breach in
# lib-alarm.ftl), e.g. "Above high limit 5 V"; $reading is the reading with
# its unit, e.g. 5.012 V.
read-alarm-breach = { $breach } ({ $reading })
# After "Warning:". A frame the meter sent couldn't be read and the run goes
# on without it. $error says why (lib-errors.ftl); $count is how many have
# been skipped so far.
read-unreadable-frame =
    { $count ->
        [one] { $error } ({ $count } trame ignorée jusqu’ici)
       *[other] { $error } ({ $count } trames ignorées jusqu’ici)
    }
# After "Error:" (common.ftl), ending a --format replay run: a reading came
# with no frame from the meter behind it, which a replay can't hold. $mode is
# the reading's mode name (lib.ftl), e.g. DC V.
read-replay-no-frame = une mesure en mode { $mode } n’a aucune trame de multimètre à enregistrer

## The summary when the run ends, under a blank line.

# After "Note:". $count is how many readings couldn't be read.
read-skipped-frames =
    { $count ->
        [one] { $count } mesure ignorée (trame illisible)
       *[other] { $count } mesures ignorées (trames illisibles)
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
        [one] { $count } mesure
       *[other] { $count } mesures
    }
# $min is a number with its unit, e.g. 4.0010 V; the same for the next three.
read-summary-min = Min : { $min }
# $max: the largest reading.
read-summary-max = Max : { $max }
# $mean: the readings' mean.
read-summary-mean = Moyenne : { $mean }
# $sd: the readings' standard deviation, σ. Left out after a single reading.
read-summary-sd = σ : { $sd }
# Under the summary, with --integrate: the readings summed over time. $value
# is the integral with its unit, e.g. 0.7720 V·s.
read-integral = Intégrale : { $value }
# The same, with how long the integral ran: $value as there, and $seconds,
# e.g. 1.8, before "s", the symbol for seconds.
read-integral-timed = Intégrale : { $value } ({ $seconds }s)
# After "Note:", under the integral: gaps in the readings the integral leaves
# out. $count is how many; $max the longest interval it bridges, in seconds
# (s). The rule's other figures, 5 intervals and 2 s, are fixed.
read-skipped-intervals =
    { $count ->
        [one] { $count } intervalle ignoré (aucune mesure pendant plus de { $max } s, 5 intervalles d’échantillonnage et au moins 2 s : l’intégrale est partielle)
       *[other] { $count } intervalles ignorés (aucune mesure pendant plus de { $max } s, 5 intervalles d’échantillonnage et au moins 2 s : l’intégrale est partielle)
    }
# Under the summary: how many times the readings crossed each limit.
# $limits is both, high or low: the limits the alarm had. $above and $below
# are the counts; $high and $low the limits with their unit, e.g. 5 V.
read-alarm-counts =
    { $limits ->
        [both] Alarmes : { $above } au-dessus de { $high }, { $below } en dessous de { $low }
        [high] Alarmes : { $above } au-dessus de { $high }
       *[low] Alarmes : { $below } en dessous de { $low }
    }
# -o without a file name: the run named the file after its first reading.
# $path is the file.
read-written-to = Écrit dans { $path }
# -o without a file name, and no reading came to name the file after.
read-nothing-written = Aucune mesure reçue, aucun fichier n’a donc été écrit
