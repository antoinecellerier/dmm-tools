### The recording panel under the graph (assets/gui-wide-layout.webp): its
### Record, Stop and Discard buttons beside Import… and Export…, the count
### of samples recorded, the line saying what Export… saves while nothing
### is recorded, and the messages its buttons show when they are done; the
### dividers dragged to resize it and the reading panel; and the big meter
### toggle at the right end of the reading's buttons, which hides every
### panel but the reading.
###
### A sample is one reading the meter sent. The buttons sit in one row with
### Import… and Export…, so keep them short. $keys is a keyboard shortcut as
### this computer writes it, e.g. Ctrl+R or ⌘R.

## The row of buttons under the graph, after Import….

# A button; ● is part of it.
recording-record = ● Enregistrer
    .hover = Commencer à écrire les échantillons en direct dans la mémoire tampon d’enregistrement ({ $keys })
# The Record button while recording; ■ is part of it.
recording-stop = ■ Arrêter
    .hover = Arrêter l’enregistrement ({ $keys })
# Shown after Export… once a recording has stopped. Export… is the button
# beside it, which saves the samples to a file.
recording-discard = Abandonner
    .hover = Abandonner l’enregistrement. « Exporter… » sauvegarde alors les échantillons du graphique
# After the buttons: $count is how many samples the recording holds. While
# it runs, followed by " | " and how long it has run, e.g. 152 samples | 12s.
recording-count =
    { $count ->
        [one] { $count } échantillon
       *[other] { $count } échantillons
    }
# After the recording's count and time, the whole line in a warning colour,
# once the recording holds as many samples as the Buffer size setting
# allows and takes no more.
recording-buffer-full = (tampon plein)
# A small line under the buttons while nothing is recorded: Export… then
# saves the samples the graph holds, which a mode change restarts, where a
# recording keeps them. Export and Record are the buttons' names. $count is
# how many samples the graph holds.
recording-history-hint =
    { $count ->
        [one] Aucun enregistrement. « Exporter… » sauvegarde { $count } échantillon du graphique. Cliquez sur « Enregistrer » pour capturer les mesures même quand le mode change.
       *[other] Aucun enregistrement. « Exporter… » sauvegarde { $count } échantillons du graphique. Cliquez sur « Enregistrer » pour capturer les mesures même quand le mode change.
    }

## Messages that show for a few seconds at the top right.

# After Discard, when the graph holds none of the recording's samples.
recording-discarded = Enregistrement abandonné
# After Discard, when the graph still holds samples. Export… is the button
# that saves them. $count is how many it holds.
recording-discarded-graph-keeps =
    { $count ->
        [one] Enregistrement abandonné. « Exporter… » sauvegarde { $count } échantillon du graphique.
       *[other] Enregistrement abandonné. « Exporter… » sauvegarde les { $count } échantillons du graphique.
    }
# After Copy in the graph's right-click menu. $digits is the reading as the
# meter showed it, e.g. 1.6109, which is now on the clipboard.
recording-copied = Copié : { $digits }

## The dividers dragged to resize the panels. Read by screen readers only:
## a divider has no text.

# Between the graph and the recording panel. Up and Down are the arrow
# keys.
recording-resize =
    .a11y = Redimensionner le panneau d’enregistrement (Haut/Bas pour ajuster)
# At the right edge of the reading panel, on a wide window. Left and Right
# are the arrow keys.
recording-reading-resize =
    .a11y = Redimensionner le panneau de mesure (Gauche/Droite pour ajuster)

## The big meter toggle: a ⊞ or ⊟ button at the right end of the reading's
## last row of buttons. Its hover is also what screen readers read.

recording-big-meter-on =
    .hover = Masquer les panneaux latéraux et afficher la mesure en plein écran ({ $keys })
# While the reading is full-screen.
recording-big-meter-off =
    .hover = Revenir à la disposition normale à plusieurs panneaux ({ $keys })
