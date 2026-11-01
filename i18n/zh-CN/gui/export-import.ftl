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
import-button = 导入…
    .hover = 打开导出的 CSV、JSON 或回放文件 ({ $keys })
# The left part of the Export… split button: saves a CSV file at once.
# $buffer is recording when a recording holds the samples, graph when
# nothing is recorded and Export… saves the samples the graph holds.
export-button = 导出…
    .hover =
        { $buffer ->
            [recording] 将记录保存为 CSV 文件 ({ $keys })
           *[graph] 将图表中的样本保存为 CSV 文件 ({ $keys })
        }
# The arrow at the right of the Export… split button, which drops the menu
# of file types. It has no text. $buffer is as for Export….
export-menu =
    .hover =
        { $buffer ->
            [recording] 将记录保存为 CSV、JSON 或回放文件
           *[graph] 将图表中的样本保存为 CSV、JSON 或回放文件
        }
    .a11y = 导出文件类型

## The menu under the Export… arrow: one entry per file type. Each opens
## the save dialog for that type.

# CSV is the file type's name.
export-csv = CSV…
# JSON is the file type's name. dmm-cli read --format json is a command
# line, kept as it is.
export-json = JSON…
    .hover = 每行一个对象，与 dmm-cli read --format json 的输出相同
# A replay file. dmm-gui --replay is a command line, kept as it is; "as the
# meter" means as if it were the meter attached.
export-replay = 回放…
    .hover = dmm-gui --replay 可将此文件当作万用表回放
# The Replay… entry's hover while it is greyed out, and the error shown if
# a replay is asked for anyway. The mock is the simulated meter the app
# offers for trying it out; its samples hold none of the bytes a real meter
# sends (its wire format), which a replay file is made of.
export-no-wire-format = 仿真万用表没有可导出的传输格式，请连接真实万用表

## A small line under the buttons after Import…: the samples are the
## file's.

# $count is how many readings the file gave; $file the file's name, e.g.
# measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv. Export… is the button.
import-hint = 已从 { $file } 导入 { $count } 个读数。“导出…”会连同标记和视图一并保存。

## The prompt shown over the window before something would discard samples
## or markers no file holds: Record over a recording that was never
## exported, Discard on a recording whose samples have left the graph,
## Import… or Connect while the session holds them. Markers are the
## numbered flags the user puts on readings, with a note each; "unsaved
## changes to a marker" means the marker, or its note, was added or changed
## since the last export.

# The prompt's title when samples would be lost (and maybe markers too).
export-unsaved-heading-samples = 放弃未导出的样本?
# The prompt's title when only markers would be lost.
export-unsaved-heading-markers = 放弃未导出的标记?
# The prompt after Record, which starts a new recording in place of the
# stopped one. $lost is samples when only samples would be lost, markers
# when only markers, both when both. $samples is how many samples were
# never exported; $markers how many markers have unsaved changes. Markers
# stay on the graph's readings even though the recording goes.
export-unsaved-record =
    { $lost ->
        [samples] 开始新记录将放弃 { $samples } 个未导出的样本。
        [markers] 开始新记录将放弃 { $markers } 个标记的未保存更改。图表仍保留其读数时，标记会留在图表上。
       *[both] 开始新记录将放弃 { $samples } 个未导出的样本，以及 { $markers } 个标记的未保存更改。图表仍保留其读数时，标记会留在图表上。
    }
# The prompt's button that goes ahead: discards, then starts recording.
export-unsaved-record-confirm = 放弃并记录
# The prompt after Discard, which drops a stopped recording; Export… then
# saves the samples the graph still holds, so only the ones that have
# scrolled out of it are lost. $lost, $samples and $markers are as for
# Record; "them" in the last sentences means those samples.
export-unsaved-discard =
    { $lost ->
        [samples] 放弃记录将丢失 { $samples } 个已不在图表中的样本。
        [markers] 放弃记录将丢失 { $markers } 个标记的未保存更改，这些标记所在的读数已不在图表中。
       *[both] 放弃记录将丢失 { $samples } 个已不在图表中的样本，以及这些样本上 { $markers } 个标记的未保存更改。
    }
# The prompt's button that goes ahead: drops the recording.
export-unsaved-discard-confirm = 放弃记录
# The prompt after picking a file to import, which replaces everything the
# session holds. $lost, $samples and $markers are as for Record.
export-unsaved-import =
    { $lost ->
        [samples] 导入将替换当前会话，并放弃 { $samples } 个未导出的样本。
        [markers] 导入将替换当前会话，并放弃 { $markers } 个标记的未保存更改。
       *[both] 导入将替换当前会话，并放弃 { $samples } 个未导出的样本，以及 { $markers } 个标记的未保存更改。
    }
# The prompt's button that goes ahead: discards, then imports the file.
export-unsaved-import-confirm = 放弃并导入
# The prompt after Connect while an imported file is open: the meter's
# session takes its place. $lost, $samples and $markers are as for Record.
export-unsaved-connect =
    { $lost ->
        [samples] 连接将结束已导入的会话，并放弃 { $samples } 个未导出的样本。
        [markers] 连接将结束已导入的会话，并放弃 { $markers } 个标记的未保存更改。
       *[both] 连接将结束已导入的会话，并放弃 { $samples } 个未导出的样本，以及 { $markers } 个标记的未保存更改。
    }
# The prompt's button that goes ahead: discards, then connects the meter.
export-unsaved-connect-confirm = 放弃并连接
# The prompt's other button, which keeps everything; it has the focus.
export-unsaved-cancel = 取消

## The file type the system's open and save dialogs list, next to the
## file's extension (.csv, .json, .replay).

# Import…'s open dialog: every type it reads.
import-file-type = 导出的读数
# Export…'s save dialog after CSV…; CSV is the file type's name.
export-file-type-csv = CSV
# After JSON…; JSON is the file type's name.
export-file-type-json = JSON
# After Replay…: a replay file.
export-file-type-replay = 回放

## Messages that show for a few seconds at the top right.

# While a large file is read in. $file is the file's name; $percent how
# much of it is in so far, a whole number from 0 to 99.
import-progress = 正在导入 { $file }… { $percent }%
# When the file is in. $count is how many readings it gave; $file is its
# name.
import-done = 已从 { $file } 导入 { $count } 个读数
# When the file holds more readings than the Buffer size setting (in
# Settings) lets the app keep: the first ones are in, the rest are not.
# $count is how many came in; $left how many were left out; $file is the
# file's name.
import-done-partial = 已从 { $file } 导入前 { $count } 个读数；超出“设置”中缓冲区大小的 { $left } 个读数未导入
# The file was read but has no readings in it. $file is its name.
import-empty = { $file } 中没有读数
# The file could not be read. $path is where it is; $reason why, in the
# words of the file reader or the system, often in English.
import-failed = 导入失败: { $path }: { $reason }
# Reading the file stopped on an internal error. $path is where it is.
import-failed-crashed = 导入失败: { $path }: 导入过程出错
# The reader stopped without saying why.
import-failed-unexplained = 导入失败
# After Export… saved the file. $count is how many samples it holds; $file
# its name, e.g. measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv.
export-done = 已将 { $count } 个样本导出到 { $file }
# After Export… with nothing to save: the recording is running or stopped
# but empty.
export-nothing-recorded = 没有可导出的内容: 记录中还没有样本
# After Export… with nothing to save and no meter connected.
export-nothing-connect = 没有可导出的内容: 请先连接万用表
# After Export… with nothing to save: a meter is connected but has sent
# nothing yet.
export-nothing-yet = 没有可导出的内容: 还没有读数
# After Export… while an earlier export still waits on its save dialog or
# is being written.
export-in-progress = 已有导出正在进行: 请先等它完成
# The file could not be written. $reason is why, in the words of the
# system or the file writer, often in English.
export-failed = 导出失败: { $reason }
# The save dialog went away without saying which file to write.
export-failed-dialog = 导出失败: 保存对话框关闭时没有给出结果
# The part of the app writing the file stopped without saying how it went.
export-failed-writer = 导出失败: 写入程序停止时没有给出结果
# The samples to export were dropped while the save dialog was open, so
# the file would not hold what was asked for.
export-failed-changed = 导出失败: 保存对话框打开期间样本已更改
