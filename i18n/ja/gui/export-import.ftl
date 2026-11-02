### Import… and Export…, the first and last buttons of the row under the
### graph (assets/gui-wide-layout.webp): Import… opens a file saved
### earlier, the Export… split button saves the samples to a file, and its
### arrow drops a menu of the file types. Then the line under the row
### after an import, the prompt shown before something would discard
### samples or markers no file holds, the file types the system's open and
### save dialogs list, and the messages both buttons show when they are
### done.
###
### A sample is one reading the meter sent; a reading, in the import
### messages, is one row of the file. The buttons sit in one row with
### Record and Discard, so keep them short. CSV, JSON and replay are the
### file types; a replay file is one dmm-gui plays back as if the meter were
### attached. $keys is a keyboard shortcut as this computer writes it, e.g.
### Ctrl+E or ⌘E.

## The row of buttons under the graph.

# The first button of the row.
import-button = インポート…
    .hover = エクスポートした CSV、JSON、またはリプレイ ファイルを開く ({ $keys })
# The left part of the Export… split button: saves a CSV file at once.
# $buffer is recording when a recording holds the samples, graph when
# nothing is recorded and Export… saves the samples the graph holds.
export-button = エクスポート…
    .hover =
        { $buffer ->
            [recording] 記録を CSV ファイルとして保存する ({ $keys })
           *[graph] グラフのサンプルを CSV ファイルとして保存する ({ $keys })
        }
# The arrow at the right of the Export… split button, which drops the menu
# of file types. It has no text. $buffer is as for Export….
export-menu =
    .hover =
        { $buffer ->
            [recording] 記録を CSV、JSON、またはリプレイ ファイルとして保存する
           *[graph] グラフのサンプルを CSV、JSON、またはリプレイ ファイルとして保存する
        }
    .a11y = エクスポート ファイルの種類

## The menu under the Export… arrow: one entry per file type. Each opens
## the save dialog for that type.

# CSV is the file type's name.
export-csv = CSV…
# JSON is the file type's name. dmm-cli read --format json is a command
# line, kept as it is.
export-json = JSON…
    .hover = 1 行に 1 つのオブジェクト (dmm-cli read --format json の出力と同じ形式)
# A replay file. dmm-gui --replay is a command line, kept as it is; "as the
# meter" means as if it were the meter attached.
export-replay = リプレイ…
    .hover = dmm-gui --replay で、マルチメーターが接続されているかのように再生されるファイル
# The Replay… entry's hover while it is greyed out, and the error shown if
# a replay is asked for anyway. The mock is the simulated meter the app
# offers for trying it out; its samples hold none of the bytes a real meter
# sends (its wire format), which a replay file is made of.
export-no-wire-format = シミュレーションのマルチメーターには、エクスポートできる通信データがありません。実際のマルチメーターを接続してください。

## A small line under the buttons after Import…: the samples are the
## file's.

# $count is how many readings the file gave; $file the file's name, e.g.
# measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv. Export… is the button.
import-hint = { $file } から { $count } 件の測定値をインポートしました。[エクスポート…] で、マーカーと表示状態も含めて保存できます。

## The prompt shown over the window before something would discard samples
## or markers no file holds: Record over a recording that was never
## exported, Discard on a recording whose samples have left the graph,
## Import… or Connect while the session holds them. Markers are the
## numbered flags the user puts on readings, with a note each; "unsaved
## changes to a marker" means the marker, or its note, was added or changed
## since the last export.

# The prompt's title when samples would be lost (and maybe markers too).
export-unsaved-heading-samples = エクスポートされていないサンプルを破棄しますか?
# The prompt's title when only markers would be lost.
export-unsaved-heading-markers = エクスポートされていないマーカーを破棄しますか?
# The prompt after Record, which starts a new recording in place of the
# stopped one. $lost is samples when only samples would be lost, markers
# when only markers, both when both. $samples is how many samples were
# never exported; $markers how many markers have unsaved changes. Markers
# stay on the graph's readings even though the recording goes.
export-unsaved-record =
    { $lost ->
        [samples] 新しい記録を開始すると、エクスポートされていないサンプル { $samples } 件が破棄されます。
        [markers] 新しい記録を開始すると、マーカー { $markers } 個の保存されていない変更が破棄されます。グラフに測定値が残っている間は、マーカーもグラフに残ります。
       *[both] 新しい記録を開始すると、エクスポートされていないサンプル { $samples } 件と、マーカー { $markers } 個の保存されていない変更が破棄されます。グラフに測定値が残っている間は、マーカーもグラフに残ります。
    }
# The prompt's button that goes ahead: discards, then starts recording.
export-unsaved-record-confirm = 破棄して記録
# The prompt after Discard, which drops a stopped recording; Export… then
# saves the samples the graph still holds, so only the ones that have
# scrolled out of it are lost. $lost, $samples and $markers are as for
# Record; "them" in the last sentences means those samples.
export-unsaved-discard =
    { $lost ->
        [samples] 記録を破棄すると、グラフに残っていないサンプル { $samples } 件が失われます。
        [markers] 記録を破棄すると、グラフに残っていない測定値に付いたマーカー { $markers } 個について、保存されていない変更が失われます。
       *[both] 記録を破棄すると、グラフに残っていないサンプル { $samples } 件と、それらに付いたマーカー { $markers } 個の保存されていない変更が失われます。
    }
# The prompt's button that goes ahead: drops the recording.
export-unsaved-discard-confirm = 記録の破棄
# The prompt after picking a file to import, which replaces everything the
# session holds. $lost, $samples and $markers are as for Record.
export-unsaved-import =
    { $lost ->
        [samples] インポートすると現在のセッションが置き換えられ、エクスポートされていないサンプル { $samples } 件が破棄されます。
        [markers] インポートすると現在のセッションが置き換えられ、マーカー { $markers } 個の保存されていない変更が破棄されます。
       *[both] インポートすると現在のセッションが置き換えられ、エクスポートされていないサンプル { $samples } 件と、マーカー { $markers } 個の保存されていない変更が破棄されます。
    }
# The prompt's button that goes ahead: discards, then imports the file.
export-unsaved-import-confirm = 破棄してインポート
# The prompt after Connect while an imported file is open: the meter's
# session takes its place. $lost, $samples and $markers are as for Record.
export-unsaved-connect =
    { $lost ->
        [samples] 接続するとインポートしたセッションが終了し、エクスポートされていないサンプル { $samples } 件が破棄されます。
        [markers] 接続するとインポートしたセッションが終了し、マーカー { $markers } 個の保存されていない変更が破棄されます。
       *[both] 接続するとインポートしたセッションが終了し、エクスポートされていないサンプル { $samples } 件と、マーカー { $markers } 個の保存されていない変更が破棄されます。
    }
# The prompt's button that goes ahead: discards, then connects the meter.
export-unsaved-connect-confirm = 破棄して接続
# The prompt's other button, which keeps everything; it has the focus.
export-unsaved-cancel = キャンセル

## The file type the system's open and save dialogs list, next to the
## file's extension (.csv, .json, .replay).

# Import…'s open dialog: every type it reads.
import-file-type = エクスポートした測定値
# Export…'s save dialog after CSV…; CSV is the file type's name.
export-file-type-csv = CSV
# After JSON…; JSON is the file type's name.
export-file-type-json = JSON
# After Replay…: a replay file.
export-file-type-replay = リプレイ

## Messages that show for a few seconds at the top right.

# While a large file is read in. $file is the file's name; $percent how
# much of it is in so far, a whole number from 0 to 99.
import-progress = { $file } をインポート中… { $percent }%
# When the file is in. $count is how many readings it gave; $file is its
# name.
import-done = { $file } から { $count } 件の測定値をインポートしました。
# When the file holds more readings than the Buffer size setting (in
# Settings) lets the app keep: the first ones are in, the rest are not.
# $count is how many came in; $left how many were left out; $file is the
# file's name.
import-done-partial = { $file } から最初の { $count } 件の測定値をインポートしました。[設定] の [バッファー サイズ] を超える { $left } 件の測定値は除外されました。
# The file was read but has no readings in it. $file is its name.
import-empty = { $file } には測定値がありません。
# The file could not be read. $path is where it is; $reason why, in the
# words of the file reader or the system, often in English.
import-failed = インポートできませんでした: { $path }: { $reason }
# Reading the file stopped on an internal error. $path is where it is.
import-failed-crashed = インポートできませんでした: { $path }: インポート処理が失敗しました。
# The reader stopped without saying why.
import-failed-unexplained = インポートできませんでした。
# After Export… saved the file. $count is how many samples it holds; $file
# its name, e.g. measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv.
export-done = { $count } 件のサンプルを { $file } にエクスポートしました。
# After Export… with nothing to save: the recording is running or stopped
# but empty.
export-nothing-recorded = エクスポートするものがありません。記録にはまだサンプルがありません。
# After Export… with nothing to save and no meter connected.
export-nothing-connect = エクスポートするものがありません。先にマルチメーターを接続してください。
# After Export… with nothing to save: a meter is connected but has sent
# nothing yet.
export-nothing-yet = エクスポートするものがありません。まだ測定値がありません。
# After Export… while an earlier export still waits on its save dialog or
# is being written.
export-in-progress = エクスポートはすでに実行中です。先にそちらを完了してください。
# The file could not be written. $reason is why, in the words of the
# system or the file writer, often in English.
export-failed = エクスポートできませんでした: { $reason }
# The save dialog went away without saying which file to write.
export-failed-dialog = エクスポートできませんでした: 保存先が選択されないまま保存ダイアログが閉じました。
# The part of the app writing the file stopped without saying how it went.
export-failed-writer = エクスポートできませんでした: 書き込み処理が結果を返さずに停止しました。
# The samples to export were dropped while the save dialog was open, so
# the file would not hold what was asked for.
export-failed-changed = エクスポートできませんでした: 保存ダイアログが開いている間にサンプルが変更されました。
