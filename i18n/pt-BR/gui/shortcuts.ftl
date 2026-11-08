### The keyboard and mouse help, a dialog opened with ? or F1 or the ?
### button in the top bar (no screenshot in the docs): a title row, then
### three tables (General, Graph (keys), Graph (mouse)) and a note.
###
### Each table row has two columns: the keys or the mouse gesture on the
### left, in a monospace font, and what they do on the right. The key
### column is written by the app as this computer spells the keys, e.g.
### Ctrl+O or ⌘O, and is not translated; these messages are the right-hand
### column, which wraps when the dialog is narrow. Then the message shown
### when Ctrl+T can't keep the window on top.

## The title row.

# The dialog's title.
shortcuts-title = Teclado e mouse
# The × button at the right end of the title row, which closes the dialog:
# its name for a screen reader.
shortcuts-close =
    .a11y = Fechar os atalhos de teclado

## The General table: the app's own shortcuts, shown in the left column of
## the dialog when it is wide.

# The table's heading.
shortcuts-general = Geral
# Ctrl+O: connects to the meter, or disconnects while connected.
shortcuts-connect = Conectar / desconectar
# Space: pauses reading the meter, or resumes while paused.
shortcuts-pause = Pausar / continuar
# Ctrl+L: clears the graph and the Statistics panel.
shortcuts-clear = Limpar o gráfico e as estatísticas
# Ctrl+R: starts or stops recording.
shortcuts-record = Alternar a gravação
# Ctrl+E: opens the dialog that saves the recording as a CSV file.
shortcuts-export = Exportar CSV…
# Ctrl+I: opens the dialog that picks a file to open.
shortcuts-import = Importar um arquivo…
# N: puts a numbered marker on the reading the meter shows now.
shortcuts-add-marker = Adicionar um marcador na leitura exibida
# Ctrl+N: the same, then puts the caret in the marker's note.
shortcuts-add-marker-note = Adicionar um marcador e escrever a nota dele
# Ctrl+B: steps the big meter (the reading drawn large) through off, full
# window and a minimal strip.
shortcuts-big-meter = Alternar a leitura ampliada (desativada / janela inteira / mínima)
# Ctrl+T: keeps the window above other windows, or stops doing so.
shortcuts-always-on-top = Ativar / desativar Sempre visível
# The same row on Wayland, where the window can't do it and the row is
# greyed. Keep the part after the colon the same as
# settings-always-on-top.wayland and shortcuts-always-on-top-toast.
shortcuts-always-on-top-wayland = Sempre visível: não disponível no Wayland: clique com o botão direito na barra de título para manter a janela acima das outras
# Ctrl+D: hides or shows the window's title bar and border.
shortcuts-decorations = Mostrar / ocultar a moldura da janela
# F11 (Ctrl+⌘+F on macOS): fills the screen with the window, or returns.
shortcuts-fullscreen = Alternar a tela inteira
# ⌘M, on macOS only: minimises the window.
shortcuts-minimize = Minimizar a janela
# Ctrl+Plus/Minus: makes everything in the window larger or smaller.
shortcuts-zoom = Ampliar / reduzir
# Ctrl+0: returns everything to its normal size.
shortcuts-zoom-reset = Redefinir o zoom para 100%
# Esc or Ctrl+W: closes this dialog.
shortcuts-close-help = Fechar esta ajuda
# Ctrl+Q or Ctrl+W: closes the app.
shortcuts-quit = Sair

## The Graph (keys) table: keys that act on the graph, while no button or
## field has the keyboard.

# The table's heading.
shortcuts-graph-keys = Gráfico (teclas)
# [ and ]: pick the next shorter or longer span of time the graph shows.
shortcuts-graph-window = Janela de tempo mais curta / mais longa
# The Left and Right arrow keys: move the graph back or forward in time.
shortcuts-graph-scroll = Rolar o gráfico
# Home: shows the start of the readings.
shortcuts-graph-start = Ir para o início
# End: shows the newest readings and follows them again.
shortcuts-graph-live = Voltar ao vivo
# M: shows or hides the line drawn at the mean of the readings.
shortcuts-graph-mean = Mostrar / ocultar a linha da média
# X: shows or hides the band between the lowest and highest readings.
shortcuts-graph-band = Mostrar / ocultar o envelope mín./máx.
# H: shows or hides the histogram beside the graph.
shortcuts-graph-histogram = Mostrar / ocultar o histograma
# R: shows or hides the reference lines, and puts the caret in the field
# where their values are typed.
shortcuts-graph-reference = Mostrar / ocultar as linhas de referência e digitar os valores
# T: shows or hides marks where the readings cross a reference line; Ref is
# the graph's chip that shows the reference lines.
shortcuts-graph-triggers = Mostrar / ocultar os marcadores de gatilho (com Ref. ativado)
# C: shows or hides the two cursors that measure between points.
shortcuts-graph-cursors = Mostrar / ocultar os cursores

## The Graph (mouse) table: what the mouse does on the graph. Each message
## is what the gesture does; its .gesture is the gesture itself, in the
## left column. Shift and Ctrl are key names: keep them as they are, the
## way the key column spells them.

# The table's heading.
shortcuts-graph-mouse = Gráfico (mouse)
shortcuts-mouse-drag = Deslocar o histórico para a esquerda / direita
    .gesture = Arrastar
# On a laptop touchpad: two fingers moved left or right together.
shortcuts-mouse-swipe = Deslocar o histórico para a esquerda / direita
    .gesture = Deslizar com dois dedos
# The box dragged out sets both the span of time and the range of values
# shown.
shortcuts-mouse-box-zoom = Aplicar zoom ao retângulo delimitador (tempo e valor)
    .gesture = Shift + arrastar
# X axis: the time axis. Zooms around the mouse pointer.
shortcuts-mouse-zoom = Aplicar zoom ao eixo X em torno do ponteiro
    .gesture = Ctrl + roda de rolagem
# The wheel alone scrolls the window's panels, not the graph.
shortcuts-mouse-scroll = Rolar o painel
    .gesture = Roda de rolagem
# Follows the newest readings again and fits the value axis to them.
shortcuts-mouse-reset = Voltar ao acompanhamento ao vivo e ao Y automático
    .gesture = Clicar duas vezes
# While the cursors are shown (C), a click places cursor A or B on the
# reading nearest to it.
shortcuts-mouse-cursor = Colocar o cursor A / B no ponto mais próximo
    .gesture = Clicar (cursores ativados)
# Opens a menu to add a marker, copy the reading, place the cursors, set
# reference lines or set an alarm.
shortcuts-mouse-menu = Marcar, copiar, cursores, linhas de referência, alarme
    .gesture = Clicar com o botão direito
# The minimap is the strip under the graph showing all the readings:
# dragging in it moves the view, dragging its edges resizes it.
shortcuts-mouse-minimap = Ir para um instante / redimensionar a área visível
    .gesture = Arrastar no minimapa

## Key names, as the key column above and every hover that names a
## shortcut write them, e.g. Ctrl+O, Esc / Ctrl+W. Use the names printed on
## keyboards in your language. On a Mac the modifiers show as ⌘ ⇧ ⌥ symbols
## where the font has them, and these names aren't used.

shortcuts-key-ctrl = Ctrl
shortcuts-key-shift = Shift
shortcuts-key-alt = Alt
shortcuts-key-space = Espaço
shortcuts-key-escape = Esc
# The left and right arrow keys.
shortcuts-key-left = Esquerda
shortcuts-key-right = Direita
shortcuts-key-home = Home
shortcuts-key-end = End
# The + and - keys, named together for zooming in and out: Ctrl+Plus/Minus.
shortcuts-key-plus = Mais
shortcuts-key-minus = Menos

## The note under the tables, in small grey text.

# Space, Escape, Up/Down, PgUp/PgDn and Home/End are key names: write them
# the way the key names above do.
shortcuts-note = Os atalhos do gráfico e a tecla Espaço ficam desativados enquanto um controle estiver com o foco do teclado: pressione Esc para liberá-lo. As setas para cima/para baixo, Page Up/Page Down e Home/End rolam esta ajuda.

## Ctrl+T on Wayland, where a window can't put itself above the others.

# A message that pops up briefly over the window. Keep it the same as
# settings-always-on-top.wayland.
shortcuts-always-on-top-toast = Não disponível no Wayland: clique com o botão direito na barra de título para manter a janela acima das outras
