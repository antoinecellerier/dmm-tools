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
stats-heading = Estatísticas
# The caption of the lowest reading.
stats-min = Mín.
# The caption of the highest reading.
stats-max = Máx.
# The caption of the mean (average) of the readings.
stats-mean = Média
# The caption of the standard deviation; σ is its usual symbol.
stats-sd = σ
# The caption of the integral of the readings over time; ∫ is its symbol.
stats-integral = ∫
# The name of the figures of the samples the graph shows.
stats-visible = Visível
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
        [one] ⚠ { $count } lacuna >{ $limit }s ignorada
       *[other] ⚠ { $count } lacunas >{ $limit }s ignoradas
    }
    .hover = Trechos de mais de { $limit } s sem leitura não são integrados: 5 intervalos de amostragem e no mínimo 2 s. A integral é parcial.
# While the Alarm row under the reading has limits set: how many times the
# reading went past each. $limits is both while there is a high and a low
# limit, high or low while there is only that one; $high and $low are the
# counts.
stats-alarm-counts =
    { $limits ->
        [both] Alarmes: { $high } acima, { $low } abaixo
        [high] Alarmes: { $high } acima
       *[low] Alarmes: { $low } abaixo
    }
    .hover = Quantas vezes a leitura ultrapassou cada limite; todas ficam marcadas no gráfico
# What a screen reader calls the small ↺ button beside Session, which starts
# the session's figures over; its hover says what it clears. σ is the
# standard deviation.
stats-reset = Redefinir
    .hover = Redefinir Mín. / Máx. / Média / σ / integral e os contadores de alarme

## The marker list under the Record row, and what N, Ctrl+N and a click on
## a marker's flag say at the top right. Notes are written in the list,
## which shows only while the Recording panel does.

# When N is pressed before any reading came in, or on a reading the graph
# and the recording no longer hold.
markers-no-reading = Ainda não há leitura para marcar
# When N is pressed on a reading that already has a marker; $number is its
# number.
markers-already-marked = O marcador { $number } já está nesta leitura
# After N or Ctrl+N marks a reading while the list is hidden: $number is the
# new marker's number. $hidden is big-meter while the big meter mode hides
# every panel but the reading, recording while the Recording panel is
# switched off; "Recording" is the name of its checkbox in Settings.
markers-added =
    { $hidden ->
        [big-meter] Marcador { $number } adicionado. Saia do modo de leitura ampliada para adicionar uma nota.
       *[recording] Marcador { $number } adicionado. Ative Gravação em Configurações para adicionar uma nota.
    }
# Ctrl+N on a reading that already has a marker while the list is hidden;
# $number and $hidden as in markers-added.
markers-already-marked-hidden =
    { $hidden ->
        [big-meter] O marcador { $number } já está nesta leitura. Saia do modo de leitura ampliada para adicionar uma nota.
       *[recording] O marcador { $number } já está nesta leitura. Ative Gravação em Configurações para adicionar uma nota.
    }
# A click on a marker's flag on the graph while the list is hidden; $hidden
# as in markers-added.
markers-note-hidden =
    { $hidden ->
        [big-meter] Saia do modo de leitura ampliada para adicionar uma nota.
       *[recording] Ative Gravação em Configurações para adicionar uma nota.
    }
# Over the list when a recording is too long to list in full; Export…
# saves every sample to a file. $time is the time of the first sample
# listed, e.g. 14:02:37.
markers-log-cut = Exporte para ver as amostras anteriores a { $time }.
# When Ctrl+N cannot show the note it opened because the panel is too short
# for a single row; $number is the marker's number.
markers-log-too-short = Aumente a altura do painel Gravação para escrever a nota do marcador { $number }.
# After a marker's reading, in small text, when the graph no longer holds
# that reading.
markers-off-graph = fora do gráfico
# After a marker's reading, in small text, when it was placed on a reading
# from before the recording started.
markers-off-recording = fora da gravação
# The hover, and the name a screen reader reads, of the + a sample's row
# shows under the pointer; $time is the sample's time, e.g. 14:02:37.512.
markers-add = Adicionar um marcador em { $time }
# A marker's number, drawn as its flag: the hover and the name a screen
# reader reads. Clicking it scrolls the graph to the marker; $number is its
# number. .hover-disabled shows while the graph no longer holds its reading.
markers-show = Mostrar o marcador { $number } no gráfico
    .hover-disabled = A leitura do marcador { $number } não está mais no gráfico
# The field a marker's note is typed in. The text shows greyed inside it
# while it is empty; .a11y is its name for a screen reader, $number the
# marker's number.
markers-note = Adicionar nota
    .a11y = Nota do marcador { $number }
# The × button at the end of a marker's row: its hover, and the name a
# screen reader reads; $number is the marker's number.
markers-delete = Excluir o marcador { $number }
