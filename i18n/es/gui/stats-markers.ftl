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
stats-heading = Estadísticas
# The caption of the lowest reading.
stats-min = Mín.
# The caption of the highest reading.
stats-max = Máx.
# The caption of the mean (average) of the readings.
stats-mean = Media
# The caption of the standard deviation; σ is its usual symbol.
stats-sd = σ
# The caption of the integral of the readings over time; ∫ is its symbol.
stats-integral = ∫
# The name of the figures of the samples the graph shows.
stats-visible = Visible
# How many readings a group's figures cover, after how long they span (e.g.
# "1m 49s #1103"); # marks a count, $count is the number. A screen reader
# says both in words; $span is written like "1m 49s" or "59 s".
stats-count = #{ $count }
# Under the table when readings stopped for a while: a warning that the
# integral left those stretches out. $count is how many; $limit the longest
# stretch still bridged, in seconds, e.g. 2; the s after it is the symbol
# for seconds. ⚠ is part of it. The hover explains it: 5 and 2 are fixed
# numbers.
stats-gaps =
    { $count ->
        [one] ⚠ { $count } interrupción omitida (>{ $limit }s)
       *[other] ⚠ { $count } interrupciones omitidas (>{ $limit }s)
    }
    .hover = Los tramos de más de { $limit } s sin lecturas (5 intervalos de muestreo, como mínimo 2 s) no se integran. La integral es parcial.
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
    .hover = Veces que la lectura cruzó cada límite, todas marcadas en el gráfico
# What a screen reader calls the small ↺ button beside Session, which starts
# the session's figures over; its hover says what it clears. σ is the
# standard deviation.
stats-reset = Restablecer
    .hover = Restablecer Mín. / Máx. / Media / σ / integral y los contadores de alarmas

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
