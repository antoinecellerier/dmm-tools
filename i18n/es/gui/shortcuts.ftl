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
shortcuts-title = Teclado y mouse
# The × button at the right end of the title row, which closes the dialog:
# its name for a screen reader.
shortcuts-close =
    .a11y = Cerrar los métodos abreviados de teclado

## The General table: the app's own shortcuts, shown in the left column of
## the dialog when it is wide.

# The table's heading.
shortcuts-general = General
# Ctrl+O: connects to the meter, or disconnects while connected.
shortcuts-connect = Conectar / desconectar
# Space: pauses reading the meter, or resumes while paused.
shortcuts-pause = Pausar / reanudar
# Ctrl+L: clears the graph and the Statistics panel.
shortcuts-clear = Borrar el gráfico y las estadísticas
# Ctrl+R: starts or stops recording.
shortcuts-record = Activar o desactivar la grabación
# Ctrl+E: opens the dialog that saves the recording as a CSV file.
shortcuts-export = Exportar a CSV…
# Ctrl+I: opens the dialog that picks a file to open.
shortcuts-import = Importar un archivo…
# N: puts a numbered marker on the reading the meter shows now.
shortcuts-add-marker = Agregar un marcador en la lectura mostrada
# Ctrl+N: the same, then puts the caret in the marker's note.
shortcuts-add-marker-note = Agregar un marcador y escribir su nota
# Ctrl+B: steps the big meter (the reading drawn large) through off, full
# window and a minimal strip.
shortcuts-big-meter = Cambiar el modo de lectura grande (desactivado / completo / mínimo)
# Ctrl+T: keeps the window above other windows, or stops doing so.
shortcuts-always-on-top = Activar o desactivar Siempre visible
# The same row on Wayland, where the window can't do it and the row is
# greyed. Keep the part after the colon the same as
# settings-always-on-top.wayland and shortcuts-always-on-top-toast.
shortcuts-always-on-top-wayland = Siempre visible: no disponible en Wayland. Haz clic con el botón derecho en la barra de título para mantener la ventana encima de las demás
# Ctrl+D: hides or shows the window's title bar and border.
shortcuts-decorations = Mostrar u ocultar el marco de la ventana
# F11 (Ctrl+⌘+F on macOS): fills the screen with the window, or returns.
shortcuts-fullscreen = Activar o desactivar la pantalla completa
# ⌘M, on macOS only: minimises the window.
shortcuts-minimize = Minimizar la ventana
# Ctrl+Plus/Minus: makes everything in the window larger or smaller.
shortcuts-zoom = Acercar / alejar
# Ctrl+0: returns everything to its normal size.
shortcuts-zoom-reset = Restablecer el zoom al 100 %
# Esc or Ctrl+W: closes this dialog.
shortcuts-close-help = Cerrar esta ayuda
# Ctrl+Q or Ctrl+W: closes the app.
shortcuts-quit = Salir

## The Graph (keys) table: keys that act on the graph, while no button or
## field has the keyboard.

# The table's heading.
shortcuts-graph-keys = Gráfico (teclas)
# [ and ]: pick the next shorter or longer span of time the graph shows.
shortcuts-graph-window = Acortar / alargar la ventana de tiempo
# The Left and Right arrow keys: move the graph back or forward in time.
shortcuts-graph-scroll = Desplazar la vista
# Home: shows the start of the readings.
shortcuts-graph-start = Ir al inicio
# End: shows the newest readings and follows them again.
shortcuts-graph-live = Volver a la vista en vivo
# M: shows or hides the line drawn at the mean of the readings.
shortcuts-graph-mean = Mostrar u ocultar la línea de la media
# X: shows or hides the band between the lowest and highest readings.
shortcuts-graph-band = Mostrar u ocultar la envolvente mín./máx.
# H: shows or hides the histogram beside the graph.
shortcuts-graph-histogram = Mostrar u ocultar el histograma
# R: shows or hides the reference lines, and puts the caret in the field
# where their values are typed.
shortcuts-graph-reference = Mostrar u ocultar las líneas de referencia y escribir sus valores
# T: shows or hides marks where the readings cross a reference line; Ref is
# the graph's chip that shows the reference lines.
shortcuts-graph-triggers = Mostrar u ocultar las marcas de disparo (con Ref. activado)
# C: shows or hides the two cursors that measure between points.
shortcuts-graph-cursors = Mostrar u ocultar los cursores

## The Graph (mouse) table: what the mouse does on the graph. Each message
## is what the gesture does; its .gesture is the gesture itself, in the
## left column. Shift and Ctrl are key names: keep them as they are, the
## way the key column spells them.

# The table's heading.
shortcuts-graph-mouse = Gráfico (mouse)
shortcuts-mouse-drag = Desplazar el historial a la izquierda / derecha
    .gesture = Arrastrar
# On a laptop touchpad: two fingers moved left or right together.
shortcuts-mouse-swipe = Desplazar el historial a la izquierda / derecha
    .gesture = Deslizar con dos dedos
# The box dragged out sets both the span of time and the range of values
# shown.
shortcuts-mouse-box-zoom = Hacer zoom en el rectángulo delimitador (tiempo y valor)
    .gesture = Mayús + arrastrar
# X axis: the time axis. Zooms around the mouse pointer.
shortcuts-mouse-zoom = Hacer zoom en el eje X centrado en el puntero
    .gesture = Ctrl + rueda del mouse
# The wheel alone scrolls the window's panels, not the graph.
shortcuts-mouse-scroll = Desplazar el panel
    .gesture = Rueda del mouse
# Follows the newest readings again and fits the value axis to them.
shortcuts-mouse-reset = Volver al seguimiento en vivo y al eje Y automático
    .gesture = Doble clic
# While the cursors are shown (C), a click places cursor A or B on the
# reading nearest to it.
shortcuts-mouse-cursor = Colocar el cursor A / B en el punto más cercano
    .gesture = Clic (cursores activados)
# Opens a menu to add a marker, copy the reading, place the cursors, set
# reference lines or set an alarm.
shortcuts-mouse-menu = Marcar, copiar, cursores, líneas de referencia, alarma
    .gesture = Clic derecho
# The minimap is the strip under the graph showing all the readings:
# dragging in it moves the view, dragging its edges resizes it.
shortcuts-mouse-minimap = Ir a un momento / cambiar el tamaño del área visible
    .gesture = Arrastrar en el minimapa

## Key names, as the key column above and every hover that names a
## shortcut write them, e.g. Ctrl+O, Esc / Ctrl+W. Use the names printed on
## keyboards in your language. On a Mac the modifiers show as ⌘ ⇧ ⌥ symbols
## where the font has them, and these names aren't used.

shortcuts-key-ctrl = Ctrl
shortcuts-key-shift = Mayús
shortcuts-key-alt = Alt
shortcuts-key-space = Espacio
shortcuts-key-escape = Esc
# The left and right arrow keys.
shortcuts-key-left = Izquierda
shortcuts-key-right = Derecha
shortcuts-key-home = Inicio
shortcuts-key-end = Fin
# The + and - keys, named together for zooming in and out: Ctrl+Plus/Minus.
shortcuts-key-plus = Más
shortcuts-key-minus = Menos

## The note under the tables, in small grey text.

# Space, Escape, Up/Down, PgUp/PgDn and Home/End are key names: write them
# the way the key names above do.
shortcuts-note = Los métodos abreviados del gráfico y la tecla Espacio se desactivan mientras un control tiene el foco del teclado. Presiona Esc para liberarlo. Flecha arriba/Flecha abajo, Re Pág/Av Pág e Inicio/Fin desplazan esta ayuda.

## Ctrl+T on Wayland, where a window can't put itself above the others.

# A message that pops up briefly over the window. Keep it the same as
# settings-always-on-top.wayland.
shortcuts-always-on-top-toast = No disponible en Wayland. Haz clic con el botón derecho en la barra de título para mantener la ventana encima de las demás
