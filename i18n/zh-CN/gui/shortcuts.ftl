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
shortcuts-title = 键盘和鼠标
# The × button at the right end of the title row, which closes the dialog:
# its name for a screen reader.
shortcuts-close =
    .a11y = 关闭键盘快捷键

## The General table: the app's own shortcuts, shown in the left column of
## the dialog when it is wide.

# The table's heading.
shortcuts-general = 常规
# Ctrl+O: connects to the meter, or disconnects while connected.
shortcuts-connect = 连接/断开连接
# Space: pauses reading the meter, or resumes while paused.
shortcuts-pause = 暂停/继续
# Ctrl+L: clears the graph and the Statistics panel.
shortcuts-clear = 清除图表和统计
# Ctrl+R: starts or stops recording.
shortcuts-record = 开始/停止记录
# Ctrl+E: opens the dialog that saves the recording as a CSV file.
shortcuts-export = 导出 CSV…
# Ctrl+I: opens the dialog that picks a file to open.
shortcuts-import = 导入文件…
# N: puts a numbered marker on the reading the meter shows now.
shortcuts-add-marker = 在屏幕上的读数处添加标记
# Ctrl+N: the same, then puts the caret in the marker's note.
shortcuts-add-marker-note = 添加标记并写备注
# Ctrl+B: steps the big meter (the reading drawn large) through off, full
# window and a minimal strip.
shortcuts-big-meter = 循环切换大字显示 (关/全窗口/极简)
# Ctrl+T: keeps the window above other windows, or stops doing so.
shortcuts-always-on-top = 切换窗口置顶
# The same row on Wayland, where the window can't do it and the row is
# greyed. Keep the part after the colon the same as
# settings-always-on-top.wayland and shortcuts-always-on-top-toast.
shortcuts-always-on-top-wayland = 窗口置顶: Wayland 下无法使用，右键单击标题栏可让窗口保持在其他窗口之上
# Ctrl+D: hides or shows the window's title bar and border.
shortcuts-decorations = 切换标题栏和边框
# F11 (Ctrl+⌘+F on macOS): fills the screen with the window, or returns.
shortcuts-fullscreen = 切换全屏
# ⌘M, on macOS only: minimises the window.
shortcuts-minimize = 最小化窗口
# Ctrl+Plus/Minus: makes everything in the window larger or smaller.
shortcuts-zoom = 放大/缩小
# Ctrl+0: returns everything to its normal size.
shortcuts-zoom-reset = 重置缩放到 100%
# Esc or Ctrl+W: closes this dialog.
shortcuts-close-help = 关闭此帮助
# Ctrl+Q or Ctrl+W: closes the app.
shortcuts-quit = 退出

## The Graph (keys) table: keys that act on the graph, while no button or
## field has the keyboard.

# The table's heading.
shortcuts-graph-keys = 图表 (键盘)
# [ and ]: pick the next shorter or longer span of time the graph shows.
shortcuts-graph-window = 缩短/延长时间窗口
# The Left and Right arrow keys: move the graph back or forward in time.
shortcuts-graph-scroll = 滚动视图
# Home: shows the start of the readings.
shortcuts-graph-start = 跳到开头
# End: shows the newest readings and follows them again.
shortcuts-graph-live = 跳到实时
# M: shows or hides the line drawn at the mean of the readings.
shortcuts-graph-mean = 显示/隐藏平均值线
# X: shows or hides the band between the lowest and highest readings.
shortcuts-graph-band = 显示/隐藏最小值/最大值带
# H: shows or hides the histogram beside the graph.
shortcuts-graph-histogram = 显示/隐藏直方图
# R: shows or hides the reference lines, and puts the caret in the field
# where their values are typed.
shortcuts-graph-reference = 显示/隐藏参考线，并输入参考值
# T: shows or hides marks where the readings cross a reference line; Ref is
# the graph's chip that shows the reference lines.
shortcuts-graph-triggers = 显示/隐藏触发标记 (需开启“参考”)
# C: shows or hides the two cursors that measure between points.
shortcuts-graph-cursors = 显示/隐藏光标

## The Graph (mouse) table: what the mouse does on the graph. Each message
## is what the gesture does; its .gesture is the gesture itself, in the
## left column. Shift and Ctrl are key names: keep them as they are, the
## way the key column spells them.

# The table's heading.
shortcuts-graph-mouse = 图表 (鼠标)
shortcuts-mouse-drag = 在历史数据中向左/向右平移
    .gesture = 拖动
# The box dragged out sets both the span of time and the range of values
# shown.
shortcuts-mouse-box-zoom = 框选缩放 (时间和数值)
    .gesture = Shift + 拖动
# X axis: the time axis. Zooms around the mouse pointer.
shortcuts-mouse-zoom = 以鼠标指针为中心缩放 X 轴
    .gesture = Ctrl + 滚轮
# The wheel alone scrolls the window's panels, not the graph.
shortcuts-mouse-scroll = 滚动面板
    .gesture = 滚轮
# Follows the newest readings again and fits the value axis to them.
shortcuts-mouse-reset = 重置为实时跟随和自动 Y 轴
    .gesture = 双击
# While the cursors are shown (C), a click places cursor A or B on the
# reading nearest to it.
shortcuts-mouse-cursor = 将光标 A/B 放到最近的点
    .gesture = 单击 (光标开启时)
# Opens a menu to add a marker, copy the reading, place the cursors, set
# reference lines or set an alarm.
shortcuts-mouse-menu = 标记、复制、光标、参考线、报警
    .gesture = 右键单击
# The minimap is the strip under the graph showing all the readings:
# dragging in it moves the view, dragging its edges resizes it.
shortcuts-mouse-minimap = 跳到某个时间/调整视区大小
    .gesture = 拖动缩略图

## Key names, as the key column above and every hover that names a
## shortcut write them, e.g. Ctrl+O, Esc / Ctrl+W. Use the names printed on
## keyboards in your language. On a Mac the modifiers show as ⌘ ⇧ ⌥ symbols
## where the font has them, and these names aren't used.

shortcuts-key-ctrl = Ctrl
shortcuts-key-shift = Shift
shortcuts-key-alt = Alt
shortcuts-key-space = 空格键
shortcuts-key-escape = Esc
# The left and right arrow keys.
shortcuts-key-left = 向左键
shortcuts-key-right = 向右键
shortcuts-key-home = Home
shortcuts-key-end = End
# The + and - keys, named together for zooming in and out: Ctrl+Plus/Minus.
shortcuts-key-plus = 加号
shortcuts-key-minus = 减号

## The note under the tables, in small grey text.

# Space, Escape, Up/Down, PgUp/PgDn and Home/End are key names: write them
# the way the key names above do.
shortcuts-note = 任何控件拥有键盘焦点时，图表快捷键和空格键快捷键会停用，按 Esc 可释放焦点。向上键/向下键、Page Up/Page Down 和 Home/End 可滚动此帮助。

## Ctrl+T on Wayland, where a window can't put itself above the others.

# A message that pops up briefly over the window. Keep it the same as
# settings-always-on-top.wayland.
shortcuts-always-on-top-toast = Wayland 下无法使用，右键单击标题栏可让窗口保持在其他窗口之上
