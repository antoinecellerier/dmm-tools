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
recording-record = ● Grabar
    .hover = Empezar a escribir las muestras en vivo en el búfer de grabación ({ $keys })
# The Record button while recording; ■ is part of it.
recording-stop = ■ Detener
    .hover = Detener la grabación ({ $keys })
# Shown after Export… once a recording has stopped. Export… is the button
# beside it, which saves the samples to a file.
recording-discard = Descartar
    .hover = Descartar la grabación. Después, “Exportar” guarda las muestras del gráfico
# After the buttons: $count is how many samples the recording holds. While
# it runs, followed by " | " and how long it has run, e.g. 152 samples | 12s.
recording-count =
    { $count ->
        [one] { $count } muestra
       *[other] { $count } muestras
    }
# After the recording's count and time, the whole line in a warning colour,
# once the recording holds as many samples as the Buffer size setting
# allows and takes no more.
recording-buffer-full = (búfer lleno)
# A small line under the buttons while nothing is recorded: Export… then
# saves the samples the graph holds, which a mode change restarts, where a
# recording keeps them. Export and Record are the buttons' names. $count is
# how many samples the graph holds.
recording-history-hint =
    { $count ->
        [one] Sin grabación. “Exportar” guarda { $count } muestra del gráfico. Haz clic en “Grabar” para seguir capturando aunque cambie el modo.
       *[other] Sin grabación. “Exportar” guarda { $count } muestras del gráfico. Haz clic en “Grabar” para seguir capturando aunque cambie el modo.
    }

## Messages that show for a few seconds at the top right.

# After Discard, when the graph holds none of the recording's samples.
recording-discarded = Grabación descartada
# After Discard, when the graph still holds samples. Export… is the button
# that saves them. $count is how many it holds.
recording-discarded-graph-keeps =
    { $count ->
        [one] Grabación descartada. “Exportar” guarda { $count } muestra del gráfico.
       *[other] Grabación descartada. “Exportar” guarda las { $count } muestras del gráfico.
    }
# After Copy in the graph's right-click menu. $digits is the reading as the
# meter showed it, with its unit, e.g. 1.6109 V, which is now on the clipboard.
recording-copied = Copiado: { $digits }

## The dividers dragged to resize the panels. Read by screen readers only:
## a divider has no text.

# Between the graph and the recording panel. Up and Down are the arrow
# keys.
recording-resize =
    .a11y = Cambiar el tamaño del panel de grabación (Flecha arriba o Flecha abajo para ajustarlo)
# At the right edge of the reading panel, on a wide window. Left and Right
# are the arrow keys.
recording-reading-resize =
    .a11y = Cambiar el tamaño del panel de lectura (Flecha izquierda o Flecha derecha para ajustarlo)

## The big meter toggle: a ⊞ or ⊟ button at the right end of the reading's
## last row of buttons. Its hover is also what screen readers read.

recording-big-meter-on =
    .hover = Ocultar los paneles laterales y mostrar la lectura del multímetro en pantalla completa ({ $keys })
# While the reading is full-screen.
recording-big-meter-off =
    .hover = Volver a la vista normal de varios paneles ({ $keys })
