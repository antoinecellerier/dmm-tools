### The recording panel under the graph (assets/gui-wide-layout.webp): its
### Record, Stop and Discard buttons beside Import… and Export…, the count
### of samples recorded, the line saying what Export… saves while nothing
### is recorded, and the messages its buttons show when they are done; the
### dividers dragged to resize it and the reading panel; and the big meter
### toggle at the right end of the reading's buttons, which hides every
### panel but the reading.
###
### A sample is one reading the meter sent. The buttons sit in one row with
### Import… and Export…, so keep them short. $keys is a keyboard shortcut as
### this computer writes it, e.g. Ctrl+R or ⌘R.

## The row of buttons under the graph, after Import….

# A button; ● is part of it.
recording-record = ● 记录
    .hover = 开始将实时样本写入记录缓冲区 ({ $keys })
# The Record button while recording; ■ is part of it.
recording-stop = ■ 停止
    .hover = 停止记录 ({ $keys })
# Shown after Export… once a recording has stopped. Export… is the button
# beside it, which saves the samples to a file.
recording-discard = 放弃
    .hover = 放弃记录；之后“导出…”将保存图表中的样本
# After the buttons: $count is how many samples the recording holds. While
# it runs, followed by " | " and how long it has run, e.g. 152 samples | 12s.
recording-count =
    { $count ->
       *[other] { $count } 个样本
    }
# After the recording's count and time, the whole line in a warning colour,
# once the recording holds as many samples as the Buffer size setting
# allows and takes no more.
recording-buffer-full = (缓冲区已满)
# A small line under the buttons while nothing is recorded: Export… then
# saves the samples the graph holds, which a mode change restarts, where a
# recording keeps them. Export and Record are the buttons' names. $count is
# how many samples the graph holds.
recording-history-hint =
    { $count ->
       *[other] 没有记录。“导出…”将保存图表中的 { $count } 个样本。单击“记录”可在切换模式时持续采集。
    }

## Messages that show for a few seconds at the top right.

# After Discard, when the graph holds none of the recording's samples.
recording-discarded = 已放弃记录
# After Discard, when the graph still holds samples. Export… is the button
# that saves them. $count is how many it holds.
recording-discarded-graph-keeps =
    { $count ->
       *[other] 已放弃记录。“导出…”将保存图表中的 { $count } 个样本。
    }
# After Copy in the graph's right-click menu. $digits is the reading as the
# meter showed it, with its unit, e.g. 1.6109 V, which is now on the clipboard.
recording-copied = 已复制 { $digits }

## The dividers dragged to resize the panels. Read by screen readers only:
## a divider has no text.

# Between the graph and the recording panel. Up and Down are the arrow
# keys.
recording-resize =
    .a11y = 调整记录面板大小 (用向上键/向下键调整)
# At the right edge of the reading panel, on a wide window. Left and Right
# are the arrow keys.
recording-reading-resize =
    .a11y = 调整读数面板大小 (用向左键/向右键调整)

## The big meter toggle: a ⊞ or ⊟ button at the right end of the reading's
## last row of buttons. Its hover is also what screen readers read.

recording-big-meter-on =
    .hover = 隐藏侧边面板，全屏显示万用表读数 ({ $keys })
# While the reading is full-screen.
recording-big-meter-off =
    .hover = 返回常规的多面板布局 ({ $keys })
