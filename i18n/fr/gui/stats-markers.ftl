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
stats-heading = Statistiques
# The caption of the lowest reading.
stats-min = Min :
# The caption of the highest reading.
stats-max = Max :
# The caption of the mean (average) of the readings.
stats-mean = Moyenne :
# The caption of the standard deviation; σ is its usual symbol.
stats-sd = σ :
# How many readings the figures above cover; $count is that number. On a
# row of its own in a wide window; a narrow one shows the number alone, in
# brackets.
stats-count = Mesures : { $count }
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
        [one] ⚠ { $count } interruption >{ $limit }s ignorée
       *[other] ⚠ { $count } interruptions >{ $limit }s ignorées
    }
    .hover = Les périodes de plus de { $limit } s sans mesure (5 intervalles d’échantillonnage, 2 s au minimum) ne sont pas intégrées. L’intégrale est partielle.
# While the Alarm row under the reading has limits set: how many times the
# reading went past each. $limits is both while there is a high and a low
# limit, high or low while there is only that one; $high and $low are the
# counts.
stats-alarm-counts =
    { $limits ->
        [both] Alarmes : { $high } au-dessus, { $low } en dessous
        [high] Alarmes : { $high } au-dessus
       *[low] Alarmes : { $low } en dessous
    }
    .hover = Nombre de franchissements de chaque limite, chacun marqué sur le graphique
# A small button that starts the figures over. σ is the standard deviation.
stats-reset = Réinitialiser
    .hover = Réinitialiser Min / Max / Moyenne / σ / intégrale et compteurs d’alarme
# The heading of the figures of the samples the graph shows, in a wide
# window. They are drawn in a fainter colour, as their own block.
stats-visible = Visible
# The same, as the caption at the start of their line in a narrow window,
# before stats-min.
stats-visible-caption = Visible :

## The marker list under the Record row, and what N, Ctrl+N and a click on
## a marker's flag say at the top right. Notes are written in the list,
## which shows only while the Recording panel does.

# When N is pressed before any reading came in, or on a reading the graph
# and the recording no longer hold.
markers-no-reading = Pas encore de mesure à marquer
# When N is pressed on a reading that already has a marker; $number is its
# number.
markers-already-marked = Le marqueur { $number } est déjà sur cette mesure
# After N or Ctrl+N marks a reading while the list is hidden: $number is the
# new marker's number. $hidden is big-meter while the big meter mode hides
# every panel but the reading, recording while the Recording panel is
# switched off; "Recording" is the name of its checkbox in Settings.
markers-added =
    { $hidden ->
        [big-meter] Marqueur { $number } ajouté. Quittez le mode grand affichage pour ajouter une note.
       *[recording] Marqueur { $number } ajouté. Activez Enregistrement dans Paramètres pour ajouter une note.
    }
# Ctrl+N on a reading that already has a marker while the list is hidden;
# $number and $hidden as in markers-added.
markers-already-marked-hidden =
    { $hidden ->
        [big-meter] Le marqueur { $number } est déjà sur cette mesure. Quittez le mode grand affichage pour ajouter une note.
       *[recording] Le marqueur { $number } est déjà sur cette mesure. Activez Enregistrement dans Paramètres pour ajouter une note.
    }
# A click on a marker's flag on the graph while the list is hidden; $hidden
# as in markers-added.
markers-note-hidden =
    { $hidden ->
        [big-meter] Quittez le mode grand affichage pour ajouter une note.
       *[recording] Activez Enregistrement dans Paramètres pour ajouter une note.
    }
# Over the list when a recording is too long to list in full; Export…
# saves every sample to a file. $time is the time of the first sample
# listed, e.g. 14:02:37.
markers-log-cut = Exportez l’enregistrement pour voir les échantillons antérieurs à { $time }.
# When Ctrl+N cannot show the note it opened because the panel is too short
# for a single row; $number is the marker's number.
markers-log-too-short = Agrandissez le panneau Enregistrement pour écrire la note du marqueur { $number }.
# After a marker's reading, in small text, when the graph no longer holds
# that reading.
markers-off-graph = hors du graphique
# After a marker's reading, in small text, when it was placed on a reading
# from before the recording started.
markers-off-recording = hors de l’enregistrement
# The hover, and the name a screen reader reads, of the + a sample's row
# shows under the pointer; $time is the sample's time, e.g. 14:02:37.512.
markers-add = Ajouter un marqueur à { $time }
# A marker's number, drawn as its flag: the hover and the name a screen
# reader reads. Clicking it scrolls the graph to the marker; $number is its
# number. .hover-disabled shows while the graph no longer holds its reading.
markers-show = Afficher le marqueur { $number } sur le graphique
    .hover-disabled = La mesure du marqueur { $number } n’est plus sur le graphique
# The field a marker's note is typed in. The text shows greyed inside it
# while it is empty; .a11y is its name for a screen reader, $number the
# marker's number.
markers-note = Ajouter une note
    .a11y = Note du marqueur { $number }
# The × button at the end of a marker's row: its hover, and the name a
# screen reader reads; $number is the marker's number.
markers-delete = Supprimer le marqueur { $number }
