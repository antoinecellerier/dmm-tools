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
shortcuts-title = Keyboard & Mouse
# The × button at the right end of the title row, which closes the dialog:
# its name for a screen reader.
shortcuts-close =
    .a11y = Close keyboard shortcuts

## The General table: the app's own shortcuts, shown in the left column of
## the dialog when it is wide.

# The table's heading.
shortcuts-general = General
# Ctrl+O: connects to the meter, or disconnects while connected.
shortcuts-connect = Connect / Disconnect
# Space: pauses reading the meter, or resumes while paused.
shortcuts-pause = Pause / Resume
# Ctrl+L: clears the graph and the Statistics panel.
shortcuts-clear = Clear graph & statistics
# Ctrl+R: starts or stops recording.
shortcuts-record = Toggle recording
# Ctrl+E: opens the dialog that saves the recording as a CSV file.
shortcuts-export = Export CSV…
# Ctrl+I: opens the dialog that picks a file to open.
shortcuts-import = Import a file…
# N: puts a numbered marker on the reading the meter shows now.
shortcuts-add-marker = Add a marker at the reading on screen
# Ctrl+N: the same, then puts the caret in the marker's note.
shortcuts-add-marker-note = Add a marker and write its note
# Ctrl+B: steps the big meter (the reading drawn large) through off, full
# window and a minimal strip.
shortcuts-big-meter = Cycle big meter (off / full / minimal)
# Ctrl+T: keeps the window above other windows, or stops doing so.
shortcuts-always-on-top = Toggle always on top
# The same row on Wayland, where the window can't do it and the row is
# greyed. Keep the part after the colon the same as
# settings-always-on-top.wayland and shortcuts-always-on-top-toast.
shortcuts-always-on-top-wayland = Always on top: not available on Wayland — right-click the title bar to keep the window above others
# Ctrl+D: hides or shows the window's title bar and border.
shortcuts-decorations = Toggle window decorations
# F11 (Ctrl+⌘+F on macOS): fills the screen with the window, or returns.
shortcuts-fullscreen = Toggle fullscreen
# ⌘M, on macOS only: minimises the window.
shortcuts-minimize = Minimise window
# Ctrl+Plus/Minus: makes everything in the window larger or smaller.
shortcuts-zoom = Zoom in / out
# Ctrl+0: returns everything to its normal size.
shortcuts-zoom-reset = Reset zoom to 100%
# Esc or Ctrl+W: closes this dialog.
shortcuts-close-help = Close this help
# Ctrl+Q or Ctrl+W: closes the app.
shortcuts-quit = Quit

## The Graph (keys) table: keys that act on the graph, while no button or
## field has the keyboard.

# The table's heading.
shortcuts-graph-keys = Graph (keys)
# [ and ]: pick the next shorter or longer span of time the graph shows.
shortcuts-graph-window = Shorter / longer time window
# The Left and Right arrow keys: move the graph back or forward in time.
shortcuts-graph-scroll = Scroll view
# Home: shows the start of the readings.
shortcuts-graph-start = Jump to start
# End: shows the newest readings and follows them again.
shortcuts-graph-live = Jump to live
# M: shows or hides the line drawn at the mean of the readings.
shortcuts-graph-mean = Toggle the mean line
# X: shows or hides the band between the lowest and highest readings.
shortcuts-graph-band = Toggle the min/max band
# H: shows or hides the histogram beside the graph.
shortcuts-graph-histogram = Toggle the histogram
# R: shows or hides the reference lines, and puts the caret in the field
# where their values are typed.
shortcuts-graph-reference = Toggle reference lines, and type the values
# T: shows or hides marks where the readings cross a reference line; Ref is
# the graph's chip that shows the reference lines.
shortcuts-graph-triggers = Toggle trigger markers (with Ref on)
# C: shows or hides the two cursors that measure between points.
shortcuts-graph-cursors = Toggle cursors

## The Graph (mouse) table: what the mouse does on the graph. Each message
## is what the gesture does; its .gesture is the gesture itself, in the
## left column. Shift and Ctrl are key names: keep them as they are, the
## way the key column spells them.

# The table's heading.
shortcuts-graph-mouse = Graph (mouse)
shortcuts-mouse-drag = Pan left / right through history
    .gesture = Drag
# The box dragged out sets both the span of time and the range of values
# shown.
shortcuts-mouse-box-zoom = Zoom to bounding box (time & value)
    .gesture = Shift + drag
# X axis: the time axis. Zooms around the mouse pointer.
shortcuts-mouse-zoom = Zoom X axis centered on cursor
    .gesture = Ctrl + scroll wheel
# The wheel alone scrolls the window's panels, not the graph.
shortcuts-mouse-scroll = Scroll the panel
    .gesture = Scroll wheel
# Follows the newest readings again and fits the value axis to them.
shortcuts-mouse-reset = Reset to live follow + auto Y
    .gesture = Double-click
# While the cursors are shown (C), a click places cursor A or B on the
# reading nearest to it.
shortcuts-mouse-cursor = Place cursor A / B at nearest point
    .gesture = Click (cursors on)
# Opens a menu to add a marker, copy the reading, place the cursors, set
# reference lines or set an alarm.
shortcuts-mouse-menu = Mark, copy, cursors, reference lines, alarm
    .gesture = Right-click
# The minimap is the strip under the graph showing all the readings:
# dragging in it moves the view, dragging its edges resizes it.
shortcuts-mouse-minimap = Jump to time / resize viewport
    .gesture = Minimap drag

## Key names, as the key column above and every hover that names a
## shortcut write them, e.g. Ctrl+O, Esc / Ctrl+W. Use the names printed on
## keyboards in your language. On a Mac the modifiers show as ⌘ ⇧ ⌥ symbols
## where the font has them, and these names aren't used.

shortcuts-key-ctrl = Ctrl
shortcuts-key-shift = Shift
shortcuts-key-alt = Alt
shortcuts-key-space = Space
shortcuts-key-escape = Esc
# The left and right arrow keys.
shortcuts-key-left = Left
shortcuts-key-right = Right
shortcuts-key-home = Home
shortcuts-key-end = End
# The + and - keys, named together for zooming in and out: Ctrl+Plus/Minus.
shortcuts-key-plus = Plus
shortcuts-key-minus = Minus

## The note under the tables, in small grey text.

# Space, Escape, Up/Down, PgUp/PgDn and Home/End are key names: write them
# the way the key names above do.
shortcuts-note = Graph and Space shortcuts are disabled while any widget has keyboard focus — press Escape to release it. Up/Down, PgUp/PgDn and Home/End scroll this help.

## Ctrl+T on Wayland, where a window can't put itself above the others.

# A message that pops up briefly over the window. Keep it the same as
# settings-always-on-top.wayland.
shortcuts-always-on-top-toast = Not available on Wayland — right-click the title bar to keep the window above others
