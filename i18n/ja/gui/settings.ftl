### The settings panel, which the ⚙ button at the top bar's right end opens
### under the bar (assets/gui-settings.webp): rows of checkboxes and of
### chips, each row led by its caption, in groups a rule apart. The colour
### and theme rows have a file of their own.
###
### Rows wrap on a narrow window, so keep the chips short. $keys and the
### other shortcut variables are keyboard shortcuts as this computer writes
### them, e.g. Ctrl+O or ⌘O. Text in parentheses starting with -- names a
### command-line option, kept as it is, that set the row for this session
### only.

## The meter: which one, how the mock runs, and how a connection is made.

# The Device row's caption; .a11y names its dropdown for a screen reader.
settings-device = デバイス:
    .a11y = デバイス
# The Device dropdown, and its first entry, when no particular meter is
# picked: each kind of meter is tried in turn.
settings-auto-detect = 自動検出
# The Device dropdown while --device picked the meter for this session.
# $name is the meter, e.g. UNI-T UT61E+, or Auto-detect.
settings-device-pinned = { $name } (--device)
# Beside the greyed Device dropdown while a recording is played back:
# --replay is the command-line option that opened it.
settings-device-replay = (マルチメーターを選択するには --replay なしで再起動してください)
# Beside the greyed Device dropdown while the session runs on a simulated
# clock, set by command-line options for testing.
settings-device-clock = (マルチメーターを選択するにはクロックのオプションなしで再起動してください)

## The open Device dropdown: Auto-detect, then every meter under its brand.

# An entry's hover and screen-reader name. $name is the meter, e.g. UNI-T
# UT61E+; $tag how far its support is verified, e.g. partly verified.
settings-device-entry = { $name } ({ $tag })

## The Mock mode row, shown while a simulated meter is picked. Its other
## chips are the simulated modes' names, kept as they are.

# The first chip: the simulated meter steps through every mode. $pinned is
# yes when --mock-mode picked it for this session, no otherwise.
settings-mock-auto =
    { $pinned ->
        [yes] 自動 (巡回) (--mock-mode)
       *[no] 自動 (巡回)
    }
    .hover = GUI の動作確認のため、すべてのシミュレーション モードを順に切り替える
# The chip for the mode --mock-mode picked for this session. $mode is its
# name, e.g. dcv.
settings-mock-mode-pinned = { $mode } (--mock-mode)
settings-mock-mode = シミュレーション モード:

## How a connection is made: checkboxes on one row.

settings-auto-connect = 起動時に自動接続する
    .hover = アプリの起動時に USB 接続を自動的に開く
# Asking the meter its name makes it beep.
settings-device-name = 接続時にデバイス名を表示する (ビープ音あり)
    .hover = 接続後にマルチメーターの名前を問い合わせる (ビープ音が 1 回鳴る)
# The Bluetooth checkbox while --no-bluetooth switched the search off for
# this session. $setting is the checkbox's own label, e.g. Look for
# Bluetooth devices.
settings-bluetooth-pinned = { $setting } (--no-bluetooth)
settings-bluetooth =
    .hover = USB ケーブルが応答しないときに、通信範囲内の Bluetooth アダプターやマルチメーターを探します。次回の接続から有効になります。

## What is kept of the readings.

settings-sample-interval = サンプリング間隔:
# The first Sample interval chip; the others are durations, e.g. 300ms or 2s.
settings-interval-every-reading = すべての測定値
    .hover = マルチメーターが送るすべての測定値を、そのペースのまま保持する
# The other Sample interval chips' hover. $unit is second for exactly one
# second, seconds for a whole number of seconds and ms otherwise; $n is how
# many seconds or milliseconds. s and ms are unit symbols.
settings-interval-chip =
    .hover =
        { $unit ->
            [second] 1 秒に最大 1 件の測定値を保持する (各刻みに最も近い測定値)
            [seconds] { $n } s ごとに最大 1 件の測定値を保持する (各刻みに最も近い測定値)
           *[ms] { $n } ms ごとに最大 1 件の測定値を保持する (各刻みに最も近い測定値)
        }

settings-buffer-size = バッファー サイズ:
# A Buffer size chip's hover; the chips are sample counts written short,
# e.g. 500K. $count is the chip's number of samples and $samples the same
# number written short. $memory is what they take, e.g. ≈194 MB, and $span
# how long they last, e.g. 14 h. $pace is every-reading when the Sample
# interval keeps every reading, whose pace the estimate can't know, and
# interval otherwise.
settings-buffer-size-chip =
    .hover =
        { $pace ->
            [every-reading] グラフとエクスポート用にサンプルを最大 { $samples } 件保持します ({ $memory }、毎秒 10 件の測定値で約 { $span })。停止した記録を併せて保持すると、さらに同じだけ使用する場合があります。
           *[interval] グラフとエクスポート用にサンプルを最大 { $samples } 件保持します ({ $memory }、現在のサンプリング間隔で約 { $span })。停止した記録を併せて保持すると、さらに同じだけ使用する場合があります。
        }
# After the Buffer size chips: what the picked size costs. $memory is the
# memory it takes, e.g. ≈194 MB; $span how long it lasts, e.g. 14 h.
settings-buffer-cost = ({ $memory }、約 { $span })

## Which panels show, and what the Specifications panel shows: checkboxes
## on one row.

settings-graph = グラフ
    .hover = 流れる時系列グラフを表示する
# σ is the standard deviation, as the Statistics panel writes it.
settings-statistics = 統計
    .hover = 現在のセッションの最小値/最大値/平均値/σ/積分を表示する
settings-recording = 記録
    .hover = 記録の操作とサンプル ログを表示する
# The Specifications panel's checkbox. $fields is shown while the panel is
# on and the checkboxes for its fields follow it, hidden otherwise.
settings-specs =
    { $fields ->
        [shown] 仕様:
       *[hidden] 仕様
    }
    .hover = 現在のモードの確度と分解能を表示する
# The Specifications fields, as that panel names them.
settings-specs-resolution = 分解能
    .hover = 仕様に分解能を表示する
settings-specs-accuracy = 確度
    .hover = 仕様に確度を表示する
# Input Z is the input impedance.
settings-specs-input-z = 入力インピーダンス
    .hover = 仕様パネルの全体表示に入力インピーダンスを表示する
settings-specs-notes = 注記
    .hover = 仕様パネルの全体表示に取扱説明書の注記を表示する

## The window.

# The Language row's caption, shown once the app speaks more than one
# language. In every language but English, keep the English word in
# parentheses after yours, e.g. Langue (Language):, so someone who can't
# read the window can still find the row. .a11y names its dropdown for a
# screen reader, in your word alone.
settings-language = 言語 (Language):
    .a11y = 言語
    .hover = ウィンドウの表示言語
# The dropdown's first entry: the window speaks the system's language.
# $language is the language that is, by its name in itself, e.g. Français
# or 简体中文. The other entries are each language's name in itself, kept
# as they are.
settings-language-system = システム ({ $language })
    .hover = システムの言語に従う
# Weak text after your language's name in the dropdown while no native
# speaker has reviewed it yet. Shown only next to your own language's name,
# whatever language the window speaks, so its speakers can read it: e.g.
# Português (Brasil)  tradução automática.
settings-language-machine-translated = 自動翻訳
# A screen reader's name for that entry, also in your language whatever the
# window speaks. $language is your language's name in itself, e.g.
# Português (Brasil).
settings-language-machine-translated-entry = { $language }、自動翻訳
# The weak line under the Language row while such a language is spoken,
# then a link that opens the translators' guide on GitHub in the web
# browser; ↗ marks a link.
settings-language-unreviewed = 自動翻訳です。まだ確認されていません。
settings-language-review-link = 確認に協力する ↗
    .hover = GitHub で翻訳者向けガイドを開く
# A toast when Chinese is picked and the computer has no font for it. It
# is always shown in English, which the app's own fonts draw: no need to
# translate it.
settings-language-no-chinese-font = No font for Chinese characters was found. Install a Simplified Chinese font, such as Noto Sans CJK SC, then restart dmm-gui.
settings-language-no-japanese-font = No font for Japanese characters was found. Install a Japanese font, such as Noto Sans CJK JP, then restart dmm-gui.

# The Zoom chips are percentages, e.g. 150%.
settings-zoom = ズーム:
# A Zoom chip's hover. $level is its scale in percent, e.g. 150; $step the
# keys that step the zoom.
settings-zoom-chip =
    .hover = GUI の表示倍率を { $level }% にする ({ $step } で段階的に変更)
# The 100% chip's hover. $reset is the keys that go back to 100%, $step
# those that step the zoom.
settings-zoom-reset-chip =
    .hover = GUI の表示倍率を 100% にする ({ $reset }、または { $step } で段階的に変更)
# Window decorations are the title bar and border.
settings-hide-decorations = ウィンドウ枠を非表示にする
    .hover = 枠なしウィンドウ ({ $keys } で元に戻す)
# On Wayland the checkbox is greyed: .wayland is its hover there, and
# settings-always-on-top-wayland the note beside it. Keep both the same as
# shortcuts-always-on-top-toast, which Ctrl+T shows there.
settings-always-on-top = 常に手前に表示する
    .hover = ウィンドウをほかのデスクトップ ウィンドウより手前に表示する ({ $keys })
    .wayland = Wayland では使用できません。ウィンドウを常に手前に表示するには、タイトル バーを右クリックしてください。
# The note after the greyed checkbox on Wayland, in small grey text.
settings-always-on-top-wayland = (Wayland では使用できません。ウィンドウを常に手前に表示するには、タイトル バーを右クリックしてください)

## The update check, in builds downloaded from GitHub.

settings-check-updates = 新しいバージョンを確認する
    .hover = 1 日に 1 回、新しいリリースが公開されているかを GitHub に問い合わせます。GitHub には IP アドレスが伝わりますが、マルチメーターに関する情報は送信されません。
# The link the top bar shows when a newer release is out; ↗ marks a link.
# $version is the release, e.g. v0.10.0.
settings-update-release = { $version } 公開中 ↗
# The same for a newer nightly build, which has no version number.
settings-update-dev = 新しい開発ビルド ↗
# Its shorter form, when the top bar is short of room.
settings-update-dev-short = 更新 ↗

## How a screen reader hears the reading change.

# A screen reader speaks the reading each time it changes; this row sets how
# often at most. Its chips are durations like the Sample interval row's,
# Every reading taking that row's first chip's label, then Never.
settings-announce = スクリーン リーダーの読み上げ:
settings-announce-every-reading =
    .hover = 新しい測定値をすべて読み上げます。測定値の更新が読み上げより速いと、スクリーン リーダーが途中で読み上げを打ち切る場合があります。
# The duration chips' hover. $n is a whole number of seconds.
settings-announce-chip =
    .hover = 測定値の読み上げは最大で { $n } 秒に 1 回です。モードの変更はすぐに読み上げられます。
settings-announce-never = 読み上げない
    .hover = 測定値が変わっても読み上げません。接続が切れたときは読み上げられます。
# A checkbox at the end of the same row, or under it when the row wraps.
# Focus is keyboard focus, or where the screen reader's cursor is.
settings-announce-focused = 測定値にフォーカスがあるときだけ読み上げる
    .hover = スクリーン リーダーが測定値の上にあるときだけ、選択したペースで読み上げます。Tab キーで移動できます。

## How the graph's lines look, the last row.

settings-graph-lines = グラフの線:
# Sub-value lines are the extra traces a meter that shows several values
# at once draws beside the main one.
settings-graph-lines-patterned = パターン
    .hover = サブ表示値の線を破線と点線にして、色に頼らずに区別する
settings-graph-lines-solid = 実線
    .hover = 連続した線にして、色と凡例で区別する
