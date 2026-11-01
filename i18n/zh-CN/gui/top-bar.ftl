### The top bar: the meter's name and the connection buttons on the left,
### the connection status in the middle, and the version, Help & Feedback,
### shortcuts and settings buttons on the right
### (assets/gui-wide-layout.webp).
###
### The bar is one row on a wide window: keep these short. $keys is a
### keyboard shortcut as this computer writes it, e.g. Ctrl+O or ⌘O.

## The meter's name, at the left end.

# Shown in place of a meter's name while Auto-detect hasn't found one yet.
top-bar-auto-detect = 自动检测
# Shown in place of a meter's name for an imported file that names none.
top-bar-imported-file = 导入的文件

## Connection buttons.

top-bar-connect = 连接
    .hover = 打开到所选万用表的 USB 连接 ({ $keys })
top-bar-disconnect = 断开连接
    .hover = 关闭当前的万用表连接 ({ $keys })
# A button; ▶ and ⏸ are part of it.
top-bar-resume = ▶ 继续
    .hover = 继续采集 ({ $keys })
top-bar-pause = ⏸ 暂停
    .hover = 暂停采集: 记录和图表都停止更新 ({ $keys })
# Clears the graph and the statistics, not the recording.
top-bar-clear = 清除
    .hover = 清除图表历史和统计 ({ $keys })
# While the connection is being retried after it dropped.
top-bar-reconnecting = 正在重新连接…
# $attempt counts the retries, from 1.
top-bar-reconnecting-attempt = 正在重新连接 (第 { $attempt } 次尝试)…
top-bar-reconnecting-hover = 正在自动重试连接，单击“断开连接”可停止
# $error is the last attempt's error, in the language of the system that raised it.
top-bar-reconnecting-hover-error =
    正在自动重试连接，单击“断开连接”可停止。
    上次错误: { $error }
top-bar-stop-retrying = 断开连接
    .hover = 停止重试并关闭连接 ({ $keys })

## The status beside the coloured dot. $name is the meter's name, e.g.
## UT61E+. $link is how it is connected (USB cable, Bluetooth). $paused is
## yes or no.

# The meter's name when it didn't report one.
top-bar-connected = 已连接
top-bar-status =
    { $paused ->
        [yes] { $name } (已暂停)
       *[no] { $name }
    }
top-bar-status-link =
    { $paused ->
        [yes] { $name } · { $link } (已暂停)
       *[no] { $name } · { $link }
    }
# An imported file: $name is its meter, $file its file name.
top-bar-status-file = { $name } · { $file }
top-bar-disconnected = 已断开连接

## The status's hover: how the meter is connected, spelled out.
## $link is usb (a USB cable), adapter (a Bluetooth adapter plugged into the
## meter), radio (the meter's own Bluetooth) or none.

top-bar-link-live =
    { $link ->
        [usb] 已通过 USB 连接线连接
        [adapter] 已通过蓝牙适配器连接
        [radio] 已通过蓝牙链路连接
       *[none] 仿真万用表，无连接
    }
# A recording being played back: how it was recorded.
top-bar-link-recorded =
    { $link ->
        [usb] 通过 USB 连接线采集
        [adapter] 通过蓝牙适配器采集
        [radio] 通过蓝牙链路采集
       *[none] 通过未命名的连接采集
    }
# $path is the file's full path.
top-bar-replaying = 正在回放 { $path }
# $readings is how many readings the file held; $path is the file's full
# path.
top-bar-imported-from =
    { $readings ->
       *[other] 已从 { $path } 导入 { $readings } 个读数
    }

## The support badge, after the status, for a meter not yet fully verified.
## A link to report feedback. Capitals, as the meter's own annunciators.

top-bar-badge-verified = 已验证
top-bar-badge-partly-verified = 部分验证
top-bar-badge-experimental = 实验性
# $warning is the sentence saying how far the meter's support is verified.
top-bar-badge-hover = { $warning }单击以提供反馈。

## The right end: the version, which opens What's New; Help & Feedback; the
## shortcut list; Settings.

top-bar-version =
    .hover = 显示新增功能: 此版本的发行说明
    .a11y = 显示发行说明
# The version's hover when the bar has no room for the update link.
# $version is the newer release, e.g. v0.10.0.
top-bar-version-update =
    .hover =
        显示新增功能: 此版本的发行说明
        { $version } 已发布，加宽窗口可看到其链接
    .a11y = 显示发行说明。{ $version } 已发布
# The update link; $version is the newer release, e.g. v0.10.0.
top-bar-update =
    .hover = 在 GitHub 上打开 { $version } 的发布页面
    .a11y = { $version } 已发布，打开其发布页面
top-bar-project-link =
    .hover = 在 GitHub 上打开 dmm-tools 项目页面
# The ? button. $keys and $other are its two keys, ? and F1.
top-bar-shortcuts =
    .hover = 显示键盘快捷键和鼠标手势 ({ $keys } 或 { $other })
    .a11y = 键盘快捷键和鼠标手势
# The ⚙ button.
top-bar-settings =
    .hover = 显示或隐藏设置面板
    .a11y = 设置
