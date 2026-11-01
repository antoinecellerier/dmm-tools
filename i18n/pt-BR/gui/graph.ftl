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
    .hover = Mostrar as amostras dos últimos { $window } ({ $keys } para alternar)
# A chip, lit while the graph follows the newest samples; dragging the graph
# turns it off. Keep it short; it is set in small capitals.
graph-live = AO VIVO
    .hover = Acompanhar as amostras mais recentes — desativado ao deslocar a visualização ({ $keys } para voltar)
# The chip for the Y (vertical) axis while its bounds are typed in: Y is the
# axis, Fixed the mode. Clicking it switches to Y:Auto.
graph-y-fixed = Y:fixo
    .hover = Limites fixos no eixo Y: clique para ajustar a escala aos dados visíveis
# The same chip while the Y axis fits the visible readings by itself.
graph-y-auto = Y:auto
    .hover = Escala do eixo Y ajustada aos dados visíveis: clique para inserir limites fixos
# The first of the two small fields after Y:Fixed. The text shows greyed
# inside the field while it is empty; the field is about 50 pixels wide.
graph-y-min = Mín. do eixo Y
    .hover = Limite inferior do eixo Y fixo
# The second field after Y:Fixed, as graph-y-min.
graph-y-max = Máx. do eixo Y
    .hover = Limite superior do eixo Y fixo
# A button that undoes zooming and panning. "Double-click graph" is the
# gesture that does the same. The second hover shows while it is greyed
# out: "live" is the LIVE chip, "auto-Y" the Y:Auto chip.
graph-reset-zoom = Redefinir zoom
    .hover = Voltar ao acompanhamento ao vivo e ao Y automático (clique duas vezes no gráfico)
    .hover-disabled = Já na visualização padrão: ao vivo e Y automático

## The second row, only for meters that send more than one value: Plot:,
## the series the graph plots, then Show:, the traces drawn beside it, each
## in a box of its own. Each box holds one chip per series, labelled with
## the series' name.

# The caption at the start of the first box.
graph-plot = Plotar:
# The Plot: chip for the meter's main reading while the meter has no name
# for it; its text also names that reading's trace in the plot's key, the
# hover and the Show: chip. .a11y is the chip's name for a screen reader,
# which tells it apart from the Show: chip of the same series.
graph-plot-main = Principal
    .hover = Plotar a leitura principal do multímetro
    .a11y = Plotar a leitura principal
# The Plot: chip for the meter's main reading, which the meter calls
# $series, e.g. T1.
graph-plot-main-named =
    .hover = Plotar a leitura { $series } do multímetro
# A Plot: chip for another value the meter sends: $series is its name, e.g.
# Frequency, $unit its unit, e.g. Hz. The .a11y is also that of the main
# reading's chip when the meter names it.
graph-plot-series =
    .hover = Plotar o valor secundário { $series } ({ $unit })
    .a11y = Plotar { $series }
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
            [full] No máximo { $max } unidades: oculte uma para mostrar a curva { $series }
            [narrow] Alargue a janela para mostrar o eixo da curva { $series }, ou clique para ocultá-la
            [shown] Ocultar a curva { $series }
           *[hidden] Desenhar a curva { $series } ao lado da série plotada
        }
    .a11y = Mostrar a curva { $series }

## The last row: what the graph draws over the trace.

# A chip that draws a line at the mean (average) of the readings in view.
graph-mean = Média
    .hover = Desenhar uma linha horizontal na média das amostras visíveis ({ $keys })
# A chip that draws a shaded band between the lowest and highest readings
# over a sliding window of time.
graph-envelope = Mín./Máx.
    .hover = Desenhar uma banda sombreada entre o mínimo e o máximo móveis ({ $keys })
# The small field after Min/Max, about 30 pixels wide: how many seconds the
# band's window spans. The text shows greyed inside the field while it is
# empty; "s" after the field is the symbol for seconds.
graph-envelope-window = Janela Mín./Máx., segundos
    .hover = Tamanho da janela (segundos) usado para calcular o envelope Mín./Máx.
# A chip that shows a histogram, bars of how many readings fell at each
# value, beside or under the graph. $state is hidden while the plotted
# trace itself is switched off, so the histogram has nothing to show;
# shown otherwise.
graph-histogram = Histograma
    .hover =
        { $state ->
            [hidden] Mostre a curva plotada para ver o histograma dela, ou clique para desativá-lo
           *[shown] Mostrar a distribuição das leituras visíveis ({ $keys })
        }
# A chip that draws reference lines: horizontal lines at the values typed
# in the field after it. Short for reference.
graph-ref = Ref.
    .hover = Desenhar linhas de referência horizontais nos valores do campo seguinte ({ $keys })
# The field after Ref, about 80 pixels wide. The text shows greyed inside
# the field while it is empty; the numbers in the hover are an example of
# what to type, kept as they are.
graph-ref-values = Valores
    .hover = Valores de referência, separados por vírgula ou ponto e vírgula (ex.: 3.3, 5, 12)
# A chip, after the reference values, that marks where the trace crosses a
# reference line.
graph-triggers = Gatilhos
    .hover = Marcar os pontos onde o sinal cruza uma linha de referência ({ $keys })
# A chip that lets clicks on the graph place two vertical lines, A and B,
# and measure between them.
graph-cursors = Cursores
    .hover = Clicar no gráfico para colocar dois cursores ({ $keys })
# Small text after Cursors until both are placed.
graph-cursors-hint = clique no gráfico para colocar os cursores
# After Cursors once both are placed, among figures such as ΔT=12.50 s and
# σ=0.0012 V: the mean of the readings from A to B. $value is a number with
# its unit, e.g. 3.2401 V. Keep it short, with no spaces around =. ΔT=,
# Δ= and σ= beside it are notation and stay as they are in every language.
graph-readout-mean = Média={ $value }
# The same row: how many readings lie from A to B. $count is that number.
graph-readout-count = Leituras={ $count }

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
graph-mean-line = Média: { $value }
# A marker's flag (a numbered tag under its line) when hovered, and its name
# for a screen reader; a click opens the note. $number is the marker's
# number.
graph-marker-flag = Escrever a nota do marcador { $number }

## What a screen reader hears of the plot: the sentences below, one after
## the other, in this order. Each ends with its own full stop.

# Between two of the sentences.
graph-plot-a11y-separator = {" "}
# The first sentence.
graph-plot-a11y-title = Gráfico das leituras.
# The first sentence when the plotted series has a name: $series, e.g. T2.
graph-plot-a11y-title-series = Gráfico das leituras de { $series }.
# How much time the plot shows: $seconds is a whole number of seconds.
graph-plot-a11y-window =
    { $seconds ->
        [one] Janela de { $seconds } segundo.
       *[other] Janela de { $seconds } segundos.
    }
# The range of the left axis: $min and $max are numbers, $unit the plotted
# unit, e.g. V; it can be empty.
graph-plot-a11y-y-axis = Eixo Y de { $min } a { $max } { $unit }.
# $count is how many samples the graph holds.
graph-plot-a11y-samples =
    { $count ->
        [0] { $count } amostras.
        [one] { $count } amostra.
       *[other] { $count } amostras.
    }
# Only when other traces are drawn beside the plotted series: $series lists
# them, separated by graph-plot-a11y-also-separator; each is a name, or
# graph-plot-a11y-also-axis.
graph-plot-a11y-also = Mostrando também { $series }.
# Between two traces in graph-plot-a11y-also.
graph-plot-a11y-also-separator = {", "}
# One trace in graph-plot-a11y-also, drawn against an axis of its own on the
# right: $series is its name, $unit its unit, e.g. Hz.
graph-plot-a11y-also-axis = { $series } em { $unit } em um eixo à direita
# Only with markers on the readings in view: $count is how many.
graph-plot-a11y-markers =
    { $count ->
        [one] { $count } marcador visível.
       *[other] { $count } marcadores visíveis.
    }
# Whether the plot follows the newest samples ($state is live) or stays
# where it was panned to (paused). Lowercase in English, as it was written.
graph-plot-a11y-state =
    { $state ->
        [live] ao vivo.
       *[paused] pausado.
    }
# The last sentence: $value is the meter's last reading as it showed it,
# $unit its unit, e.g. V; it can be empty.
graph-plot-a11y-reading = última leitura { $value } { $unit }.
# The last sentence while the meter reads over range (shows OL).
graph-plot-a11y-over-range = atualmente acima da faixa.
# The last sentence before any reading.
graph-plot-a11y-no-data = sem dados.

## The histogram, beside the plot or under the minimap.

# The bar's range in its hover, and in graph-histogram-a11y-most: from
# $low to $high, numbers, in $unit, e.g. V.
graph-histogram-range = { $low } a { $high } { $unit }
# A bar when hovered. $range is the values it spans, e.g. "3.200 to 3.299
# V" (see graph-histogram-range); $count how many readings in view fall
# there; $percent what share of them, a number with one decimal.
graph-histogram-bin =
    .hover =
        { $count ->
            [0] { $range }: { $count } leituras ({ $percent }%)
            [one] { $range }: { $count } leitura ({ $percent }%)
           *[other] { $range }: { $count } leituras ({ $percent }%)
        }
# The line under it while both cursors are placed: the readings from cursor
# A to cursor B (A–B) in that bar. $count and $percent as above.
graph-histogram-bin-interval =
    .hover =
        { $count ->
            [0] A–B: { $count } leituras ({ $percent }%)
            [one] A–B: { $count } leitura ({ $percent }%)
           *[other] A–B: { $count } leituras ({ $percent }%)
        }
# The divider between the plot and the histogram beside it, which is
# dragged sideways to resize it: its name for a screen reader. Left/Right
# are the arrow keys.
graph-histogram-divider-side =
    .a11y = Redimensionar o histograma (Seta para a esquerda ou Seta para a direita para ajustar)
# The divider over the histogram under the minimap, dragged up or down.
# Up/Down are the arrow keys.
graph-histogram-divider-below =
    .a11y = Redimensionar o histograma (Seta para cima ou Seta para baixo para ajustar)

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
        [0] Histograma de { $count } leituras visíveis
        [one] Histograma de { $count } leitura visível
       *[other] Histograma de { $count } leituras visíveis
    }
# After graph-histogram-a11y, and after graph-histogram-a11y-interval: how
# far the readings spread. $value is a number, $unit its unit, e.g. V.
graph-histogram-a11y-sd = desvio padrão { $value } { $unit }
# The bar holding the most readings: $range is its values (see
# graph-histogram-range), $count how many it holds.
graph-histogram-a11y-most = mais frequente de { $range } ({ $count })
# How many readings in view were over range (OL). $count is that number.
graph-histogram-a11y-over-range =
    { $count ->
        [one] { $count } acima da faixa
       *[other] { $count } acima da faixa
    }
# How many readings in view lie above and below the Y axis' ends, out of
# the histogram's reach. $above and $below are those numbers.
graph-histogram-a11y-beyond = { $above } acima e { $below } abaixo do eixo
# While both cursors are placed: $count is how many samples lie between
# them.
graph-histogram-a11y-interval =
    { $count ->
        [0] { $count } leituras entre os cursores
        [one] { $count } leitura entre os cursores
       *[other] { $count } leituras entre os cursores
    }

## The minimap: the strip under the graph showing every sample kept, with
## brackets around the part the graph shows.

# The hover, and its name for a screen reader. Left/Right are the arrow
# keys.
graph-minimap =
    .hover = Minimapa: clique ou arraste para deslocar, arraste as bordas dos colchetes para redimensionar a visualização
    .a11y = Minimapa do gráfico: clique ou arraste para navegar pela linha do tempo (Seta para a esquerda ou Seta para a direita para deslocar), arraste as bordas dos colchetes para redimensionar

## The menu a right-click on the plot, its axes or a marker's flag opens:
## what can be done to the reading nearest the click, under a heading
## naming it, then what can be done at the height clicked, under a heading
## naming that. Each entry's .a11y is its name for a screen reader, which
## does not read the headings. A cursor is one of the two vertical lines
## Cursors places, A and B ($cursor); a reference line is one the Ref chip
## draws; a marker is a numbered flag on a reading, with a note.

# The heading when no reading is in view, and the hover of the entries
# greyed out then.
graph-menu-no-reading = Nenhuma leitura visível
# The heading over the reading's entries. $time is the reading's time on the
# graph, e.g. 35.0 s, drawn in a stronger colour.
graph-menu-reading-heading = Leitura em { $time }
# The heading over the height's entries. $height is the height clicked, a
# number with its unit, e.g. 3.29 V, drawn in a stronger colour.
graph-menu-height-heading = Em { $height }
# Puts a marker on the reading. $reading is some, with $time the reading's
# time, e.g. 35.0 s; none while no reading is in view (the entry is greyed
# out).
graph-menu-mark = Marcar
    .a11y =
        { $reading ->
            [some] Marcar a leitura em { $time }
           *[none] Marcar uma leitura
        }
# Copies the reading while none is in view (greyed out).
graph-menu-copy = Copiar
# Copies the reading: $digits are its digits as the meter showed them, e.g.
# 3.249.
graph-menu-copy-digits = Copiar { $digits }
# Deletes the marker under the pointer, or the one whose flag was
# right-clicked. $number is its number.
graph-menu-delete-marker = Excluir o marcador { $number }
# Puts the first cursor on the reading, while none is placed. $reading and
# $time are as for Mark.
graph-menu-put-cursor = Colocar um cursor
    .a11y =
        { $reading ->
            [some] Colocar um cursor na leitura em { $time }
           *[none] Colocar um cursor em uma leitura
        }
# Puts the cursor not yet placed, $cursor, on the reading. $reading and
# $time are as for Mark.
graph-menu-put-cursor-named = Colocar o cursor { $cursor }
    .a11y =
        { $reading ->
            [some] Colocar o cursor { $cursor } na leitura em { $time }
           *[none] Colocar o cursor { $cursor } em uma leitura
        }
# Moves cursor $cursor, already placed, to the reading. $reading and $time
# are as for Mark.
graph-menu-move-cursor = Mover o cursor { $cursor }
    .a11y =
        { $reading ->
            [some] Mover o cursor { $cursor } para a leitura em { $time }
           *[none] Mover o cursor { $cursor } para uma leitura
        }
# Removes the cursor under the pointer, $cursor.
graph-menu-remove-cursor = Remover o cursor { $cursor }
# Removes both cursors and turns Cursors off.
graph-menu-remove-cursors = Remover os cursores
# Draws a reference line at the height clicked, $height (see
# graph-menu-height-heading).
graph-menu-add-ref = Adicionar linha de referência
    .a11y = Adicionar linha de referência em { $height }
# Removes the reference line under the pointer, at $height, a number with
# its unit.
graph-menu-remove-ref = Remover a linha de referência { $height }
# Sets one of the alarm's two limits at the height clicked: $side is high
# for the upper limit, low for the lower. The alarm, set up beside the
# reading, warns when a reading goes past a limit. $height as above.
graph-menu-set-alarm =
    { $side ->
        [high] Definir o limite superior do alarme
       *[low] Definir o limite inferior do alarme
    }
    .a11y =
        { $side ->
            [high] Definir o limite superior do alarme em { $height }
           *[low] Definir o limite inferior do alarme em { $height }
        }
# Removes the alarm limit whose line is under the pointer; $side as above.
graph-menu-remove-alarm =
    { $side ->
        [high] Remover o limite superior do alarme
       *[low] Remover o limite inferior do alarme
    }
