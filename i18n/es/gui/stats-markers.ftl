### The Statistics panel (assets/gui-wide-layout.webp): in a wide window, the
### figures of the meter's main reading since the last Reset, one per row
### under a heading, then those of the samples the graph shows, under
### Visible; in a narrow one, each block on a line of its own over the
### graph. Then the marker list under the Record row: each marker's reading
### with its number, its note and a button deleting it, among the recorded
### samples while recording.
###
### A sample is one reading the meter sent. A marker is a numbered flag on
### a reading, with a note, drawn on the graph and listed under the Record
### row. Figures ($value) come written with their padding, e.g.
### "    4.4957"; units ($unit) are symbols, e.g. V or V·s: keep both as
### they are and phrase around them.

## The Statistics panel. The captions sit in a monospace column, right
## aligned to the widest of stats-min, stats-max, stats-mean and stats-sd,
## so the figures after them line up: keep them short.

# The panel's heading in a wide window.
stats-heading = Estadísticas
# The caption of the lowest reading.
stats-min = Mín.:
# The caption of the highest reading.
stats-max = Máx.:
# The caption of the mean (average) of the readings.
stats-mean = Media:
# The caption of the standard deviation; σ is its usual symbol.
stats-sd = σ:
# How many readings the figures above cover; $count is that number. On a
# row of its own in a wide window; a narrow one shows the number alone, in
# brackets.
stats-count = Lecturas: { $count }
# The integral of the readings over time: ∫ is its symbol, $value the
# figure, $unit its unit (e.g. V·s or mAh), $seconds how long it covers in
# whole seconds; the s after it is the symbol for seconds.
stats-integral = ∫:{ $value } { $unit } ({ $seconds }s)
# The same, while the time it covers is unknown; $value and $unit as in
# stats-integral.
stats-integral-untimed = ∫:{ $value } { $unit }
# Under the integral when readings stopped for a while: a warning that the
# integral left those stretches out. $count is how many; $limit the longest
# stretch still bridged, in seconds, e.g. 2; the s after it is the symbol
# for seconds. ⚠ is part of it. The hover explains it: 5 and 2 are fixed
# numbers.
stats-gaps =
    { $count ->
        [one] ⚠ { $count } interrupción >{ $limit }s omitida
       *[other] ⚠ { $count } interrupciones >{ $limit }s omitidas
    }
    .hover = Los tramos sin lecturas de más de { $limit } s (5 intervalos de muestreo, como mínimo 2 s) no se integran. La integral es parcial.
# While the Alarm row under the reading has limits set: how many times the
# reading went past each. $limits is both while there is a high and a low
# limit, high or low while there is only that one; $high and $low are the
# counts.
stats-alarm-counts =
    { $limits ->
        [both] Alarmas: { $high } por encima, { $low } por debajo
        [high] Alarmas: { $high } por encima
       *[low] Alarmas: { $low } por debajo
    }
    .hover = Veces que la lectura cruzó cada límite, cada una marcada en el gráfico
# A small button that starts the figures over. σ is the standard deviation.
stats-reset = Restablecer
    .hover = Restablecer Mín. / Máx. / Media / σ / integral y los contadores de alarmas
# The heading of the figures of the samples the graph shows, in a wide
# window. They are drawn in a fainter colour, as their own block.
stats-visible = Visible
# The same, as the caption at the start of their line in a narrow window,
# before stats-min.
stats-visible-caption = Visible:

## The marker list under the Record row, and what N, Ctrl+N and a click on
## a marker's flag say at the top right. Notes are written in the list,
## which shows only while the Recording panel does.

# When N is pressed before any reading came in, or on a reading the graph
# and the recording no longer hold.
markers-no-reading = Aún no hay lecturas que marcar
# When N is pressed on a reading that already has a marker; $number is its
# number.
markers-already-marked = El marcador { $number } ya está en esta lectura
# After N or Ctrl+N marks a reading while the list is hidden: $number is the
# new marker's number. $hidden is big-meter while the big meter mode hides
# every panel but the reading, recording while the Recording panel is
# switched off; "Recording" is the name of its checkbox in Settings.
markers-added =
    { $hidden ->
        [big-meter] Marcador { $number } agregado. Sal del modo de lectura grande para agregar una nota.
       *[recording] Marcador { $number } agregado. Activa Grabación en Configuración para agregar una nota.
    }
# Ctrl+N on a reading that already has a marker while the list is hidden;
# $number and $hidden as in markers-added.
markers-already-marked-hidden =
    { $hidden ->
        [big-meter] El marcador { $number } ya está en esta lectura. Sal del modo de lectura grande para agregar una nota.
       *[recording] El marcador { $number } ya está en esta lectura. Activa Grabación en Configuración para agregar una nota.
    }
# A click on a marker's flag on the graph while the list is hidden; $hidden
# as in markers-added.
markers-note-hidden =
    { $hidden ->
        [big-meter] Sal del modo de lectura grande para agregar una nota.
       *[recording] Activa Grabación en Configuración para agregar una nota.
    }
# Over the list when a recording is too long to list in full; Export…
# saves every sample to a file. $time is the time of the first sample
# listed, e.g. 14:02:37.
markers-log-cut = Exporta para ver las muestras anteriores a las { $time }.
# When Ctrl+N cannot show the note it opened because the panel is too short
# for a single row; $number is the marker's number.
markers-log-too-short = Aumenta la altura del panel Grabación para escribir la nota del marcador { $number }.
# After a marker's reading, in small text, when the graph no longer holds
# that reading.
markers-off-graph = fuera del gráfico
# After a marker's reading, in small text, when it was placed on a reading
# from before the recording started.
markers-off-recording = fuera de la grabación
# The hover, and the name a screen reader reads, of the + a sample's row
# shows under the pointer; $time is the sample's time, e.g. 14:02:37.512.
markers-add = Agregar marcador a las { $time }
# A marker's number, drawn as its flag: the hover and the name a screen
# reader reads. Clicking it scrolls the graph to the marker; $number is its
# number. .hover-disabled shows while the graph no longer holds its reading.
markers-show = Mostrar el marcador { $number } en el gráfico
    .hover-disabled = La lectura del marcador { $number } ya no está en el gráfico
# The field a marker's note is typed in. The text shows greyed inside it
# while it is empty; .a11y is its name for a screen reader, $number the
# marker's number.
markers-note = Agregar nota
    .a11y = Nota del marcador { $number }
# The × button at the end of a marker's row: its hover, and the name a
# screen reader reads; $number is the marker's number.
markers-delete = Eliminar el marcador { $number }
