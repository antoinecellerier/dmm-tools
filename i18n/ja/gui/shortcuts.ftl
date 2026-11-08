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
shortcuts-title = キーボードとマウス
# The × button at the right end of the title row, which closes the dialog:
# its name for a screen reader.
shortcuts-close =
    .a11y = キーボード ショートカットを閉じる

## The General table: the app's own shortcuts, shown in the left column of
## the dialog when it is wide.

# The table's heading.
shortcuts-general = 全般
# Ctrl+O: connects to the meter, or disconnects while connected.
shortcuts-connect = 接続/切断
# Space: pauses reading the meter, or resumes while paused.
shortcuts-pause = 一時停止/再開
# Ctrl+L: clears the graph and the Statistics panel.
shortcuts-clear = グラフと統計のクリア
# Ctrl+R: starts or stops recording.
shortcuts-record = 記録の切り替え
# Ctrl+E: opens the dialog that saves the recording as a CSV file.
shortcuts-export = CSV のエクスポート…
# Ctrl+I: opens the dialog that picks a file to open.
shortcuts-import = ファイルのインポート…
# N: puts a numbered marker on the reading the meter shows now.
shortcuts-add-marker = 表示中の測定値にマーカーを追加
# Ctrl+N: the same, then puts the caret in the marker's note.
shortcuts-add-marker-note = マーカーを追加してメモを入力
# Ctrl+B: steps the big meter (the reading drawn large) through off, full
# window and a minimal strip.
shortcuts-big-meter = 大型表示の切り替え (オフ/全体/最小)
# Ctrl+T: keeps the window above other windows, or stops doing so.
shortcuts-always-on-top = 常に手前に表示の切り替え
# The same row on Wayland, where the window can't do it and the row is
# greyed. Keep the part after the colon the same as
# settings-always-on-top.wayland and shortcuts-always-on-top-toast.
shortcuts-always-on-top-wayland = 常に手前に表示: Wayland では使用できません。ウィンドウを常に手前に表示するには、タイトル バーを右クリックしてください。
# Ctrl+D: hides or shows the window's title bar and border.
shortcuts-decorations = ウィンドウ枠の切り替え
# F11 (Ctrl+⌘+F on macOS): fills the screen with the window, or returns.
shortcuts-fullscreen = 全画面表示の切り替え
# ⌘M, on macOS only: minimises the window.
shortcuts-minimize = ウィンドウの最小化
# Ctrl+Plus/Minus: makes everything in the window larger or smaller.
shortcuts-zoom = 拡大/縮小
# Ctrl+0: returns everything to its normal size.
shortcuts-zoom-reset = ズームのリセット (100%)
# Esc or Ctrl+W: closes this dialog.
shortcuts-close-help = このヘルプを閉じる
# Ctrl+Q or Ctrl+W: closes the app.
shortcuts-quit = 終了

## The Graph (keys) table: keys that act on the graph, while no button or
## field has the keyboard.

# The table's heading.
shortcuts-graph-keys = グラフ (キー)
# [ and ]: pick the next shorter or longer span of time the graph shows.
shortcuts-graph-window = 時間幅の短縮/延長
# The Left and Right arrow keys: move the graph back or forward in time.
shortcuts-graph-scroll = 表示のスクロール
# Home: shows the start of the readings.
shortcuts-graph-start = 先頭へ移動
# End: shows the newest readings and follows them again.
shortcuts-graph-live = ライブへ移動
# M: shows or hides the line drawn at the mean of the readings.
shortcuts-graph-mean = 平均値の線の切り替え
# X: shows or hides the band between the lowest and highest readings.
shortcuts-graph-band = 最小/最大エンベロープの切り替え
# H: shows or hides the histogram beside the graph.
shortcuts-graph-histogram = ヒストグラムの切り替え
# R: shows or hides the reference lines, and puts the caret in the field
# where their values are typed.
shortcuts-graph-reference = 基準線の切り替えと値の入力
# T: shows or hides marks where the readings cross a reference line; Ref is
# the graph's chip that shows the reference lines.
shortcuts-graph-triggers = トリガー マーカーの切り替え ([基準] オン時)
# C: shows or hides the two cursors that measure between points.
shortcuts-graph-cursors = カーソルの切り替え

## The Graph (mouse) table: what the mouse does on the graph. Each message
## is what the gesture does; its .gesture is the gesture itself, in the
## left column. Shift and Ctrl are key names: keep them as they are, the
## way the key column spells them.

# The table's heading.
shortcuts-graph-mouse = グラフ (マウス)
shortcuts-mouse-drag = 履歴を左右にスクロール
    .gesture = ドラッグ
# On a laptop touchpad: two fingers moved left or right together.
shortcuts-mouse-swipe = 履歴を左右にスクロール
    .gesture = 2 本指でスワイプ
# The box dragged out sets both the span of time and the range of values
# shown.
shortcuts-mouse-box-zoom = 境界ボックスにズーム (時間と値)
    .gesture = Shift + ドラッグ
# X axis: the time axis. Zooms around the mouse pointer.
shortcuts-mouse-zoom = マウス ポインターを中心に X 軸をズーム
    .gesture = Ctrl + マウス ホイール
# The wheel alone scrolls the window's panels, not the graph.
shortcuts-mouse-scroll = パネルのスクロール
    .gesture = マウス ホイール
# Follows the newest readings again and fits the value axis to them.
shortcuts-mouse-reset = ライブ追従と Y 軸の自動調整に戻す
    .gesture = ダブルクリック
# While the cursors are shown (C), a click places cursor A or B on the
# reading nearest to it.
shortcuts-mouse-cursor = 最も近い点にカーソル A/B を配置
    .gesture = クリック (カーソル オン時)
# Opens a menu to add a marker, copy the reading, place the cursors, set
# reference lines or set an alarm.
shortcuts-mouse-menu = マーカー、コピー、カーソル、基準線、アラーム
    .gesture = 右クリック
# The minimap is the strip under the graph showing all the readings:
# dragging in it moves the view, dragging its edges resizes it.
shortcuts-mouse-minimap = 時刻へ移動/ビューポートのサイズ変更
    .gesture = ミニマップ上でドラッグ

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
shortcuts-key-left = ←
shortcuts-key-right = →
shortcuts-key-home = Home
shortcuts-key-end = End
# The + and - keys, named together for zooming in and out: Ctrl+Plus/Minus.
shortcuts-key-plus = プラス
shortcuts-key-minus = マイナス

## The note under the tables, in small grey text.

# Space, Escape, Up/Down, PgUp/PgDn and Home/End are key names: write them
# the way the key names above do.
shortcuts-note = いずれかのコントロールにキーボード フォーカスがある間は、グラフと Space キーのショートカットは無効です。フォーカスを解除するには Esc キーを押してください。↑/↓キー、PageUp/PageDown キー、Home/End キーでこのヘルプをスクロールできます。

## Ctrl+T on Wayland, where a window can't put itself above the others.

# A message that pops up briefly over the window. Keep it the same as
# settings-always-on-top.wayland.
shortcuts-always-on-top-toast = Wayland では使用できません。ウィンドウを常に手前に表示するには、タイトル バーを右クリックしてください。
