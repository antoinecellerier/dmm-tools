### The colour and theme rows near the foot of the settings panel, which
### the ⚙ button at the top bar's right end opens (assets/gui-settings.webp):
### the Theme and Colors rows of chips, the Customize colors section with a
### swatch for every colour of the palette and the editor each swatch opens,
### and Save as theme with the messages it shows when it is done.
###
### Theme names (Midnight, Bubble Gum, a theme the user saved) are kept as
### they are: they are file names and command-line values too. Text in
### parentheses starting with -- names a command-line option, kept as it
### is, that set the row for this session only. Rows wrap on a narrow
### window, so keep the chips and swatch names short.

## The Theme row: the Dark, Light and System modes, then the named themes.

colors-theme = 主题:
# The Dark, Light and System chips. $state is pinned when --theme picked
# this mode for the session, customized when the user changed some of its
# colours (Dark and Light only), plain otherwise.
colors-theme-dark =
    { $state ->
        [pinned] 深色 (--theme)
        [customized] 深色 (自定义)
       *[plain] 深色
    }
    .hover = 整个 GUI 使用深色模式
# $state as for Dark.
colors-theme-light =
    { $state ->
        [pinned] 浅色 (--theme)
        [customized] 浅色 (自定义)
       *[plain] 浅色
    }
    .hover = 整个 GUI 使用浅色模式
# $state as for Dark, never customized: System has no colours of its own.
colors-theme-system =
    { $state ->
        [pinned] 系统 (--theme)
       *[plain] 系统
    }
    .hover = 跟随桌面的浅色/深色设置 (桌面未报告时用深色)
# A named theme's chip when --theme picked it for the session; otherwise
# the chip is the theme's name alone. $name is the theme's name, e.g.
# Midnight.
colors-theme-named-pinned = { $name } (--theme)
# A named theme's chip after the user changed some of its colours. $name is
# the theme's name.
colors-theme-named-customized = { $name } (自定义)
# A built-in theme's hover. $mode is dark or light: the mode the theme is
# drawn in.
colors-theme-builtin =
    .hover =
        { $mode ->
            [light] 内置浅色主题
           *[dark] 内置深色主题
        }
# The hover of a theme the user added to the themes folder. $mode is dark
# or light; $path is the theme's file, in full.
colors-theme-file =
    .hover =
        { $mode ->
            [light] 浅色主题，来自 { $path }。删除该文件即可移除。
           *[dark] 深色主题，来自 { $path }。删除该文件即可移除。
        }
# The same when the themes folder can't be found, with $mode as there.
# $file is the theme's file name.
colors-theme-file-unplaced =
    .hover =
        { $mode ->
            [light] 浅色主题，来自 { $file }
           *[dark] 深色主题，来自 { $file }
        }

## After the Theme row's chips: the theme files in the themes folder that
## could not be offered, with why on hover, one file to a line.

# $count is how many files were skipped.
colors-skipped =
    { $count ->
       *[other] 已跳过 { $count } 个主题文件
    }
# The hover's first line. $folder is the themes folder, in full.
colors-skipped-folder = 在 { $folder } 中:
# A line of the hover. $file is a file name; $reason one of the reasons
# below, starting lower case.
colors-skipped-file = { $file }: { $reason }
# The line for the files left unread because the folder holds too many.
# $count is how many; $max how many files are read, e.g. 100.
colors-skipped-too-many =
    { $count ->
       *[other] 另有 { $count } 个: 超过 { $max } 个文件的上限
    }

## Why a theme file can't be used: after its name in the hover above, or
## after "Not saved:" when Save as theme would write such a file. Each
## starts lower case. Others come from the file system or the file reader
## and are shown in their own words.

colors-issue-not-a-file = 不是文件
# $size is the largest file read, in KiB (kibibytes), e.g. 64.
colors-issue-too-big = 超过 { $size } KiB
colors-issue-name-empty = 名称为空
# $max is the most characters a name may have, e.g. 32.
colors-issue-name-too-long = 名称超过 { $max } 个字符
# Invisible characters, or $ or `.
colors-issue-name-unusable = 名称含有不允许使用的字符
# The theme's name is Dark, Light or System, which the Theme row's own
# chips go by. $name is the name as the file has it; put it in your
# language's quotes.
colors-issue-name-reserved = “{ $name }”已被“主题”行占用
# $what is text or button: the theme's text on its background, or its
# button captions on the button.
colors-issue-not-opaque =
    { $what ->
        [button] 按钮文字颜色不是完全不透明
       *[text] 背景上的文字颜色不是完全不透明
    }
# The colours can't be read on each other. $what is as above; $ratio is
# their contrast, e.g. 1.8, and $floor the least allowed, e.g. 3.
colors-issue-low-contrast =
    { $what ->
        [button] 按钮文字的对比度为 { $ratio }:1，低于 { $floor }:1 的下限
       *[text] 背景上的文字对比度为 { $ratio }:1，低于 { $floor }:1 的下限
    }
# The theme's text on its frame, the surface of the top bar and Settings.
colors-issue-frame-not-opaque = 框架上的文字颜色不是完全不透明
# As above, on the frame. $ratio and $floor are as above.
colors-issue-frame-low-contrast = 框架上的文字对比度为 { $ratio }:1，低于 { $floor }:1 的下限
# Another file in the folder gives its theme the same name. $name is the
# name as the file has it, to put in your language's quotes; $file the
# other file's name.
colors-issue-name-taken = “{ $name }”已是 { $file } 的名称
colors-issue-file-name-unprintable = 文件名含有无法显示的字符
# The folder holds too many theme files. $max is how many are read, e.g.
# 100.
colors-issue-too-many = 超过 { $max } 个文件的上限

## The Colors row: the palette presets under Dark and Light.

colors-colors = 颜色:
# The preset chips. $state is customized on the picked chip while the user
# has changed some of its colours, plain otherwise.
colors-preset-default =
    { $state ->
        [customized] 默认 (自定义)
       *[plain] 默认
    }
    .hover = 适合日常使用的均衡配色
# $state as for Default.
colors-preset-high-contrast =
    { $state ->
        [customized] 高对比度 (自定义)
       *[plain] 高对比度
    }
    .hover = 适合明亮环境或投影仪的最大对比度配色
# $state as for Default. Protan and deutan are the red-green colour-vision
# deficiencies.
colors-preset-colorblind =
    { $state ->
        [customized] 色盲友好 (自定义)
       *[plain] 色盲友好
    }
    .hover = 红色盲和绿色盲用户也能分辨的配色
# After the chips while there are changes a click would lose.
colors-preset-clears = 选择预设将清除自定义

## The Customize colors section, folded under its header until opened.

colors-customize = 自定义颜色
    .hover = 在所选预设或主题之上逐个覆盖颜色
# Beside the header: whose colours the swatches change. $mode is dark or
# light.
colors-editing =
    { $mode ->
        [light] (正在编辑浅色主题的颜色)
       *[dark] (正在编辑深色主题的颜色)
    }
# The same while a named theme is on. $name is the theme's name.
colors-editing-named = (正在编辑 { $name } 的颜色)

# The captions at the start of the swatch rows.
colors-group-ui = 界面:
colors-group-graph = 图表:
colors-group-status = 状态:
# The arrow after a colour that others follow, before those it leads.
# $anchor is the leading colour's name, as its swatch below has it.
colors-follow-arrow =
    .hover = 这些颜色跟随“{ $anchor }”变化并保持相对偏移，直到你单独设置其中一个

## The swatches, each named beside it, in the rows the section shows them.

colors-swatch-background = 背景
    .hover = 面板背景色
colors-swatch-frame = 框架
    .hover = 顶栏和读数面板的背景 (未设置时同面板背景)
colors-swatch-button = 按钮
    .hover = 按钮背景色
# Lines between and around the panels.
colors-swatch-border = 边框
    .hover = 分隔线、面板边缘、框线和绘图轮廓
# Short for the plot's background.
colors-swatch-plot-background = 绘图背景
    .hover = 图表绘图区的背景色
colors-swatch-text = 文本
    .hover = 主要文本颜色
# The meter's value as the reading panel shows it, in large type.
colors-swatch-reading = 读数
    .hover = 读数和副读数 (未设置时同文本颜色)
# Text drawn fainter than the rest.
colors-swatch-weak-text = 次要文本
    .hover = 次要文本: 模式行、副读数标签、提示和说明
colors-swatch-heading = 标题
    .hover = 面板标题: 技术指标、统计、可见部分 (未设置时同次要文本颜色)
# The lines that follow the pointer over the graph.
colors-swatch-crosshair = 十字线
    .hover = 图表上悬停十字线的颜色
colors-swatch-accent = 主题色
    .hover = 模式徽标、链接，以及选中的开关、标签和文字的底色
# The graph's main line, the readings over time.
colors-swatch-data-line = 数据线
    .hover = 图表上数据线的颜色
# The band between the lowest and highest readings.
colors-swatch-envelope = 包络线
    .hover = 最小值/最大值包络的填充色
# The lines of the other values a meter shows at once beside the main one.
colors-swatch-overlay-1 = 叠加 1
    .hover = 绘制在主曲线旁的第一个副读数的线条颜色
colors-swatch-overlay-2 = 叠加 2
    .hover = 绘制在主曲线旁的第二个副读数的线条颜色
colors-swatch-overlay-3 = 叠加 3
    .hover = 绘制在主曲线旁的第三个副读数的线条颜色
# The frame the overview strip under the graph draws around the part on
# show.
colors-swatch-viewport = 视区
    .hover = 缩略图中视区矩形的颜色
# A stretch with no readings.
colors-swatch-gap = 间断
    .hover = 标记记录数据中间断的颜色
# The line at the average of the readings.
colors-swatch-mean = 平均值
    .hover = 平均值叠加线的颜色
# Short for reference: a line the user sets at a value.
colors-swatch-ref = 参考
    .hover = 参考线的颜色
# Where the readings cross a set level.
colors-swatch-crossing = 穿越
    .hover = 触发穿越标记的颜色
colors-swatch-cursor = 光标
    .hover = 测量光标的颜色
# The numbered markers the user drops on the graph.
colors-swatch-marker = 标记
    .hover = 标记线、旗标和编号
# The status colours, after the states they mark. Connected is the top
# bar's status word, in quotes in the hover.
colors-swatch-connected = 已连接
    .hover = “已连接”状态的颜色
colors-swatch-warning = 警告
    .hover = 警告状态的颜色
colors-swatch-error = 错误
    .hover = 错误状态的颜色
colors-swatch-inactive = 未激活
    .hover = 未激活/已断开连接状态的颜色

## Beside a swatch.

# A swatch's name for a screen reader while it follows another colour.
# $color is its name, $anchor the name of the colour it follows.
colors-swatch-linked = { $color }，跟随“{ $anchor }”
# A small button beside a colour that was set on its own though it can
# follow another; ↺ is part of it. $color is the colour's name, $anchor
# the name of the one it would follow.
colors-relink = ↺ 重新跟随
    .hover = 重新跟随“{ $anchor }”
    .a11y = 让“{ $color }”重新跟随“{ $anchor }”
# A warning after a colour the user made hard to see on another. $ratio is
# their contrast, e.g. 2.1; $ground the other colour's name; $floor the
# least contrast it needs, e.g. 4.5.
colors-low-contrast = 在“{ $ground }”上为 { $ratio }:1
    .hover = 此颜色在“{ $ground }”上需要至少 { $floor }:1 的对比度
# A warning after a graph colour the user made hard to tell from another.
# $other is the other colour's name; $distance how far apart they are and
# $floor how far apart they need to be, as whole numbers. ΔE is the
# measure of colour difference.
colors-too-close = 与“{ $other }”相近
    .hover = 仅相差 { $distance } (ΔE)，图表颜色需要相差 { $floor } 才能区分

## The editor a swatch opens: a colour picker, then the colour as text.

# The colour written as # and six hexadecimal digits.
colors-hex = 十六进制
    .hover = 复制此颜色，或粘贴 #RRGGBB 格式的颜色

## Under the swatches, once a colour was changed.

colors-reset = 重置颜色
    .hover = 放弃对此模式颜色所做的更改
# The same while a named theme is on. $name is the theme's name.
colors-reset-named =
    .hover = 放弃对 { $name } 颜色所做的更改
colors-save = 另存为主题…
    .hover = 将这些颜色保存为主题文件；保存在主题文件夹中的文件会加入“主题”行
# The file type in the save dialog's list of types.
colors-save-file-type = 主题

## What Save as theme says when it is done, in a toast: a short message that
## floats under the top bar's right end for a few seconds.

# $name is the theme's name, from the file name the user typed.
colors-saved = 已将 { $name } 保存到主题文件夹
# Saved outside the themes folder. $file is the file name.
colors-saved-elsewhere = 已保存 { $file }。将它复制到主题文件夹即可作为主题使用
# The save dialog closed without an answer.
colors-save-dialog-failed = 未保存: 保存对话框出错
# A name typed without .json gets it added, and the file of that name
# already exists. $path is that file, in full.
colors-save-exists = 未保存: { $path } 已存在；在对话框中选择该文件即可替换
# Another file in the themes folder gives its theme the same name. $name is
# the name; $file the other file's name.
colors-save-name-taken = 未保存: { $name } 已是主题文件夹中 { $file } 的名称；请换一个文件名
# The theme would be a file the folder can't use. $reason is one of the
# reasons above, starting lower case.
colors-save-invalid = 未保存: { $reason }
# Writing the file failed. $path is the file, in full; $error why, in the
# words of the system that raised it.
colors-save-failed = 无法保存 { $path }: { $error }
