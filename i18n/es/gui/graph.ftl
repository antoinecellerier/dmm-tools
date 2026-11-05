### The graph (assets/gui-wide-layout.webp, assets/gui-graph-series.webp,
### assets/gui-graph-histogram.webp): the rows of chips over it, the
### plot with its hover readout and the labels it draws on its lines, the
### histogram beside or under it, the minimap under it, and the menu its
### right-click opens.
###
### A sample is one reading the meter sent; the plotted series is the
### reading the graph draws, the meter's main reading or one of the other
### values it sends (a sub-value, e.g. Frequency or T2); a trace is a series'
### line. The chips sit in tight rows, so keep their labels short. $keys is
### a key that does the same as the chip, as this computer writes it, e.g.
### M or End; keep it as it is. Series names ($series) are the meter's own
### labels, e.g. T2 or Frequency, and units ($unit) are symbols, e.g. V or
### Hz: phrase around them.

## The first row of chips over the graph.

# The chips 5s, 10s, 30s, 1m, 5m and 10m, which pick how much time the graph
# shows; they have no text of their own. $window is the chip's own label,
# e.g. 30s; $keys the two keys that step through them, e.g. [ / ].
graph-window =
    .hover = Mostrar una ventana de { $window } con las muestras más recientes ({ $keys } para cambiar)
# A chip, lit while the graph follows the newest samples; dragging the graph
# turns it off. Keep it short; it is set in small capitals.
graph-live = EN VIVO
    .hover = Seguir las muestras más recientes: se desactiva al desplazar la vista ({ $keys } para volver)
# The chip for the Y (vertical) axis while its bounds are typed in: Y is the
# axis, Fixed the mode. Clicking it switches to Y:Auto.
graph-y-fixed = Y:fijo
    .hover = Eje Y con límites fijos: haz clic para ajustarlo a los datos visibles
# The same chip while the Y axis fits the visible readings by itself.
graph-y-auto = Y:auto
    .hover = Eje Y ajustado a los datos visibles: haz clic para escribir límites fijos
# The first of the two small fields after Y:Fixed. The text shows greyed
# inside the field while it is empty; the field is about 50 pixels wide.
graph-y-min = Mín. del eje Y
    .hover = Límite inferior del eje Y fijo
# The second field after Y:Fixed, as graph-y-min.
graph-y-max = Máx. del eje Y
    .hover = Límite superior del eje Y fijo
# A button that undoes zooming and panning. "Double-click graph" is the
# gesture that does the same. The second hover shows while it is greyed
# out: "live" is the LIVE chip, "auto-Y" the Y:Auto chip.
graph-reset-zoom = Restablecer zoom
    .hover = Volver al seguimiento en vivo y al eje Y automático (doble clic en el gráfico)
    .hover-disabled = Ya está en la vista predeterminada: en vivo y eje Y automático

## The second row, only for meters that send more than one value: Plot:,
## the series the graph plots, then Show:, the traces drawn beside it, each
## in a box of its own. Each box holds one chip per series, labelled with
## the series' name.

# The caption at the start of the first box.
graph-plot = Trazar:
# The Plot: chip for the meter's main reading while the meter has no name
# for it; its text also names that reading's trace in the plot's key, the
# hover and the Show: chip. .a11y is the chip's name for a screen reader,
# which tells it apart from the Show: chip of the same series.
graph-plot-main = Principal
    .hover = Trazar la lectura principal del multímetro
    .a11y = Trazar la lectura principal
# The Plot: chip for the meter's main reading, which the meter calls
# $series, e.g. T1.
graph-plot-main-named =
    .hover = Trazar la lectura { $series } del multímetro
# A Plot: chip for another value the meter sends: $series is its name, e.g.
# Frequency, $unit its unit, e.g. Hz. The .a11y is also that of the main
# reading's chip when the meter names it.
graph-plot-series =
    .hover = Trazar el valor secundario { $series } ({ $unit })
    .a11y = Trazar { $series }
# The caption at the start of the second box.
graph-show = Mostrar:
# A Show: chip, which draws or hides a trace beside the plotted series:
# $series is its name. $state is full when the graph already has $max axes
# in other units (the chip is greyed out; $max is 4), narrow when the window
# is too narrow for the trace's own axis, shown while it is drawn, hidden
# while it is not. .a11y is its name for a screen reader.
graph-show-chip =
    .hover =
        { $state ->
            [full] { $max } unidades como máximo: oculta una para mostrar la curva { $series }
            [narrow] Amplía la ventana para mostrar el eje de la curva { $series }, o haz clic para ocultarla
            [shown] Ocultar la curva { $series }
           *[hidden] Dibujar la curva { $series } junto a la serie trazada
        }
    .a11y = Mostrar la curva { $series }

## The last row: what the graph draws over the trace.

# A chip that draws a line at the mean (average) of the readings in view.
graph-mean = Media
    .hover = Dibujar una línea horizontal en la media de las muestras visibles ({ $keys })
# A chip that draws a shaded band between the lowest and highest readings
# over a sliding window of time.
graph-envelope = Mín./Máx.
    .hover = Dibujar una banda sombreada entre el mínimo y el máximo móviles ({ $keys })
# The small field after Min/Max, about 30 pixels wide: how many seconds the
# band's window spans. The text shows greyed inside the field while it is
# empty; "s" after the field is the symbol for seconds.
graph-envelope-window = Ventana Mín./Máx. en segundos
    .hover = Tamaño de la ventana (segundos) para calcular la envolvente Mín./Máx.
# A chip that shows a histogram, bars of how many readings fell at each
# value, beside or under the graph. $state is hidden while the plotted
# trace itself is switched off, so the histogram has nothing to show;
# shown otherwise.
graph-histogram = Histograma
    .hover =
        { $state ->
            [hidden] Activa la curva trazada para ver su histograma o haz clic para desactivarlo
           *[shown] Mostrar la distribución de las lecturas visibles ({ $keys })
        }
# A chip that draws reference lines: horizontal lines at the values typed
# in the field after it. Short for reference.
graph-ref = Ref.
    .hover = Dibujar líneas de referencia horizontales en los valores del campo siguiente ({ $keys })
# The field after Ref, about 80 pixels wide. The text shows greyed inside
# the field while it is empty; the numbers in the hover are an example of
# what to type, kept as they are.
graph-ref-values = Valores
    .hover = Valores de referencia separados por comas o punto y coma (p. ej. 3.3, 5, 12)
# A chip, after the reference values, that marks where the trace crosses a
# reference line.
graph-triggers = Disparos
    .hover = Marcar los puntos donde la señal cruza una línea de referencia ({ $keys })
# A chip that lets clicks on the graph place two vertical lines, A and B,
# and measure between them.
graph-cursors = Cursores
    .hover = Haz clic en el gráfico para colocar dos cursores ({ $keys })
# Small text after Cursors until both are placed.
graph-cursors-hint = haz clic en el gráfico para colocar los cursores
# After Cursors once both are placed, among figures such as ΔT=12.50 s and
# σ=0.0012 V: the mean of the readings from A to B. $value is a number with
# its unit, e.g. 3.2401 V. Keep it short, with no spaces around =. ΔT=,
# Δ= and σ= beside it are notation and stay as they are in every language.
graph-readout-mean = Media={ $value }
# The same row: how many readings lie from A to B. $count is that number.
graph-readout-count = Lecturas={ $count }

## The plot: the readout by the pointer, the labels on its lines, and the
## flags of markers along its bottom edge.

# The readout by the pointer, under the time, inside a stretch where the
# meter read over range (showed OL).
graph-hover-overload = sobrecarga
# The same, with several series listed: $series is the plotted series'
# name.
graph-hover-overload-series = { $series }: sobrecarga
# One series' row in the readout when several are listed: $series is its
# name, e.g. T2; $value a number; $unit its unit, e.g. V.
graph-hover-series = { $series }: { $value } { $unit }
# The label at the right end of the Mean line. $value is the mean with its
# unit, e.g. 4.5045 V.
graph-mean-line = Media: { $value }
# A marker's flag (a numbered tag under its line) when hovered, and its name
# for a screen reader; a click opens the note. $number is the marker's
# number.
graph-marker-flag = Escribir la nota del marcador { $number }

## What a screen reader hears of the plot: the sentences below, one after
## the other, in this order. Each ends with its own full stop.

# Between two of the sentences.
graph-plot-a11y-separator = {" "}
# The first sentence.
graph-plot-a11y-title = Gráfico de mediciones.
# The first sentence when the plotted series has a name: $series, e.g. T2.
graph-plot-a11y-title-series = Gráfico de mediciones de { $series }.
# How much time the plot shows: $seconds is a whole number of seconds.
graph-plot-a11y-window =
    { $seconds ->
        [one] Ventana de { $seconds } segundo.
       *[other] Ventana de { $seconds } segundos.
    }
# The range of the left axis: $min is a number, $max a number with the
# plotted unit (reading-spoken-amount), e.g. 5.000 volts, or a bare number.
graph-plot-a11y-y-axis = Eje Y de { $min } a { $max }.
# $count is how many samples the graph holds.
graph-plot-a11y-samples =
    { $count ->
        [one] { $count } muestra.
       *[other] { $count } muestras.
    }
# Only when other traces are drawn beside the plotted series: $series lists
# them, separated by graph-plot-a11y-also-separator; each is a name, or
# graph-plot-a11y-also-axis.
graph-plot-a11y-also = También muestra { $series }.
# Between two traces in graph-plot-a11y-also.
graph-plot-a11y-also-separator = {", "}
# One trace in graph-plot-a11y-also, drawn against an axis of its own on the
# right: $series is its name, $unit its unit, e.g. Hz.
graph-plot-a11y-also-axis = { $series } en { $unit }, con su propio eje a la derecha
# Only with markers on the readings in view: $count is how many.
graph-plot-a11y-markers =
    { $count ->
        [one] { $count } marcador visible.
       *[other] { $count } marcadores visibles.
    }
# Whether the plot follows the newest samples ($state is live) or stays
# where it was panned to (paused). Lowercase in English, as it was written.
graph-plot-a11y-state =
    { $state ->
        [live] en vivo.
       *[paused] en pausa.
    }
# The last sentence: $value is the meter's last reading as it showed it,
# with its unit (reading-spoken-amount), e.g. 1.6112 volts.
graph-plot-a11y-reading = última lectura { $value }.
# The last sentence while the meter reads over range (shows OL).
graph-plot-a11y-over-range = actualmente fuera de rango.
# The last sentence before any reading.
graph-plot-a11y-no-data = sin datos.

## The histogram, beside the plot or under the minimap.

# The bar's range in its hover, and in graph-histogram-a11y-most: from
# $low, a number, to $high, a number with its unit: 3.299 V in the hover,
# 3.299 volts spoken (reading-spoken-amount).
graph-histogram-range = { $low } a { $high }
# A bar when hovered. $range is the values it spans, e.g. "3.200 to 3.299
# V" (see graph-histogram-range); $count how many readings in view fall
# there; $percent what share of them, a number with one decimal.
graph-histogram-bin =
    .hover =
        { $count ->
            [one] { $range }: { $count } lectura ({ $percent }%)
           *[other] { $range }: { $count } lecturas ({ $percent }%)
        }
# The line under it while both cursors are placed: the readings from cursor
# A to cursor B (A–B) in that bar. $count and $percent as above.
graph-histogram-bin-interval =
    .hover =
        { $count ->
            [one] A–B: { $count } lectura ({ $percent }%)
           *[other] A–B: { $count } lecturas ({ $percent }%)
        }
# The divider between the plot and the histogram beside it, which is
# dragged sideways to resize it: its name for a screen reader. Left/Right
# are the arrow keys.
graph-histogram-divider-side =
    .a11y = Cambiar el tamaño del histograma (Flecha izquierda o Flecha derecha para ajustarlo)
# The divider over the histogram under the minimap, dragged up or down.
# Up/Down are the arrow keys.
graph-histogram-divider-below =
    .a11y = Cambiar el tamaño del histograma (Flecha arriba o Flecha abajo para ajustarlo)

## What a screen reader hears of the histogram, one sentence among the
## plot's: graph-histogram-a11y, then the phrases after it, each after
## graph-histogram-a11y-separator (graph-histogram-a11y-interval after
## graph-histogram-a11y-interval-separator), in graph-histogram-a11y-sentence.

# The sentence: $clauses is graph-histogram-a11y and the phrases after it.
graph-histogram-a11y-sentence = { $clauses }.
# Between two phrases.
graph-histogram-a11y-separator = {", "}
# Before graph-histogram-a11y-interval.
graph-histogram-a11y-interval-separator = {"; "}
# $count is how many readings are in view.
graph-histogram-a11y =
    { $count ->
        [one] Histograma de { $count } lectura visible
       *[other] Histograma de { $count } lecturas visibles
    }
# After graph-histogram-a11y, and after graph-histogram-a11y-interval: how
# far the readings spread. $value is a number with its unit
# (reading-spoken-amount), e.g. 0.002 volts.
graph-histogram-a11y-sd = desviación estándar { $value }
# The bar holding the most readings: $range is its values (see
# graph-histogram-range), $count how many it holds.
graph-histogram-a11y-most = la mayoría en el intervalo de { $range } ({ $count })
# How many readings in view were over range (OL). $count is that number.
graph-histogram-a11y-over-range =
    { $count ->
        [one] { $count } fuera de rango
       *[other] { $count } fuera de rango
    }
# How many readings in view lie above and below the Y axis' ends, out of
# the histogram's reach. $above and $below are those numbers.
graph-histogram-a11y-beyond = { $above } por encima y { $below } por debajo del eje
# While both cursors are placed: $count is how many samples lie between
# them.
graph-histogram-a11y-interval =
    { $count ->
        [one] { $count } lectura entre los cursores
       *[other] { $count } lecturas entre los cursores
    }

## The minimap: the strip under the graph showing every sample kept, with
## brackets around the part the graph shows.

# The hover, and its name for a screen reader. Left/Right are the arrow
# keys.
graph-minimap =
    .hover = Minimapa: haz clic o arrastra para desplazarte, arrastra los bordes de los corchetes para cambiar el tamaño de la vista
    .a11y = Minimapa del gráfico: haz clic o arrastra para recorrer la línea de tiempo (Flecha izquierda o Flecha derecha para desplazarte), arrastra los bordes de los corchetes para cambiar el tamaño

## The menu a right-click on the plot, its axes or a marker's flag opens:
## what can be done to the reading nearest the click, under a heading
## naming it, then what can be done at the height clicked, under a heading
## naming that. Each entry's .a11y is its name for a screen reader, which
## does not read the headings. A cursor is one of the two vertical lines
## Cursors places, A and B ($cursor); a reference line is one the Ref chip
## draws; a marker is a numbered flag on a reading, with a note.

# The heading when no reading is in view, and the hover of the entries
# greyed out then.
graph-menu-no-reading = Ninguna lectura visible
# The heading over the reading's entries. $time is the reading's time on the
# graph, e.g. 35.0 s, drawn in a stronger colour.
graph-menu-reading-heading = Lectura a los { $time }
# The heading over the height's entries. $height is the height clicked, a
# number with its unit, e.g. 3.29 V, drawn in a stronger colour.
graph-menu-height-heading = En { $height }
# Puts a marker on the reading. $reading is some, with $time the reading's
# time, e.g. 35.0 s; none while no reading is in view (the entry is greyed
# out).
graph-menu-mark = Marcar
    .a11y =
        { $reading ->
            [some] Marcar la lectura a los { $time }
           *[none] Marcar una lectura
        }
# Copies the reading while none is in view (greyed out).
graph-menu-copy = Copiar
# Copies the reading: $digits are its digits as the meter showed them and
# its unit, e.g. 3.249 V.
graph-menu-copy-digits = Copiar { $digits }
# Deletes the marker under the pointer, or the one whose flag was
# right-clicked. $number is its number.
graph-menu-delete-marker = Eliminar marcador { $number }
# Puts the first cursor on the reading, while none is placed. $reading and
# $time are as for Mark.
graph-menu-put-cursor = Colocar un cursor
    .a11y =
        { $reading ->
            [some] Colocar un cursor sobre la lectura a los { $time }
           *[none] Colocar un cursor sobre una lectura
        }
# Puts the cursor not yet placed, $cursor, on the reading. $reading and
# $time are as for Mark.
graph-menu-put-cursor-named = Colocar cursor { $cursor }
    .a11y =
        { $reading ->
            [some] Colocar el cursor { $cursor } sobre la lectura a los { $time }
           *[none] Colocar el cursor { $cursor } sobre una lectura
        }
# Moves cursor $cursor, already placed, to the reading. $reading and $time
# are as for Mark.
graph-menu-move-cursor = Mover cursor { $cursor }
    .a11y =
        { $reading ->
            [some] Mover el cursor { $cursor } a la lectura a los { $time }
           *[none] Mover el cursor { $cursor } a una lectura
        }
# Removes the cursor under the pointer, $cursor.
graph-menu-remove-cursor = Quitar cursor { $cursor }
# Removes both cursors and turns Cursors off.
graph-menu-remove-cursors = Quitar cursores
# Draws a reference line at the height clicked, $height (see
# graph-menu-height-heading).
graph-menu-add-ref = Agregar línea de referencia
    .a11y = Agregar línea de referencia en { $height }
# Removes the reference line under the pointer, at $height, a number with
# its unit.
graph-menu-remove-ref = Quitar línea de referencia en { $height }
# Sets one of the alarm's two limits at the height clicked: $side is high
# for the upper limit, low for the lower. The alarm, set up beside the
# reading, warns when a reading goes past a limit. $height as above.
graph-menu-set-alarm =
    { $side ->
        [high] Fijar alarma superior
       *[low] Fijar alarma inferior
    }
    .a11y =
        { $side ->
            [high] Fijar el límite superior de la alarma en { $height }
           *[low] Fijar el límite inferior de la alarma en { $height }
        }
# Removes the alarm limit whose line is under the pointer; $side as above.
graph-menu-remove-alarm =
    { $side ->
        [high] Quitar alarma superior
       *[low] Quitar alarma inferior
    }
