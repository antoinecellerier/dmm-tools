### The top bar: the meter's name and the connection buttons on the left,
### the connection status in the middle, and the version, Help & Feedback,
### shortcuts and settings buttons on the right
### (assets/gui-wide-layout.webp).
###
### The bar is one row on a wide window: keep these short. $keys is a
### keyboard shortcut as this computer writes it, e.g. Ctrl+O or ⌘O.

## The meter's name, at the left end.

# Shown in place of a meter's name while Auto-detect hasn't found one yet.
top-bar-auto-detect = 自動検出
# Shown in place of a meter's name for an imported file that names none.
top-bar-imported-file = インポートしたファイル

## Connection buttons.

top-bar-connect = 接続
    .hover = 選択したマルチメーターへの USB 接続を開く ({ $keys })
# In place of Connect while the meter is being opened: stops the attempt.
top-bar-cancel = キャンセル
    .hover = 接続を中止する ({ $keys })
top-bar-disconnect = 切断
    .hover = マルチメーターとの接続を閉じる ({ $keys })
# A button; ▶ and ⏸ are part of it.
top-bar-resume = ▶ 再開
    .hover = データ取得を再開する ({ $keys })
top-bar-pause = ⏸ 一時停止
    .hover = データ取得を一時停止し、記録とグラフの更新を止める ({ $keys })
# Clears the graph and the statistics, not the recording.
top-bar-clear = クリア
    .hover = グラフの履歴と統計をクリアする ({ $keys })
# While the connection is being retried after it dropped.
top-bar-reconnecting = 再接続中…
# $attempt counts the retries, from 1.
top-bar-reconnecting-attempt = 再接続中 ({ $attempt } 回目)…
top-bar-reconnecting-hover = 自動的に再接続しています。停止するには [切断] をクリックしてください。
# $error is the last attempt's error, in the language of the system that raised it.
top-bar-reconnecting-hover-error =
    自動的に再接続しています。停止するには [切断] をクリックしてください。
    最後のエラー: { $error }
# After shared.ftl's waiting-for-own-cable. "Disconnect" and "Connect" are the
# top bar's button.
top-bar-own-cable-action = 別のケーブルを使用するには、[切断] をクリックしてから [接続] をクリックしてください。
top-bar-stop-retrying = 切断
    .hover = 再試行をやめて接続を閉じる ({ $keys })

## The status beside the coloured dot. $name is the meter's name, e.g.
## UT61E+. $link is how it is connected (USB cable, Bluetooth). $paused is
## yes or no.

# The meter's name when it didn't report one.
top-bar-connected = 接続済み
top-bar-status =
    { $paused ->
        [yes] { $name } (一時停止中)
       *[no] { $name }
    }
top-bar-status-link =
    { $paused ->
        [yes] { $name } · { $link } (一時停止中)
       *[no] { $name } · { $link }
    }
# An imported file: $name is its meter, $file its file name.
top-bar-status-file = { $name } · { $file }
top-bar-disconnected = 切断済み
# While the picked meter is being opened.
top-bar-connecting = 接続中…
# While Auto-detect asks the connected meter what it is.
top-bar-detecting = 検出中…

## The status's hover: how the meter is connected, spelled out.
## $link is usb (a USB cable), adapter (a Bluetooth adapter plugged into the
## meter), radio (the meter's own Bluetooth) or none.

top-bar-link-live =
    { $link ->
        [usb] USB ケーブルで接続中
        [adapter] Bluetooth アダプターで接続中
        [radio] Bluetooth リンクで接続中
       *[none] シミュレーションのマルチメーター (リンクなし)
    }
# A recording being played back: how it was recorded.
top-bar-link-recorded =
    { $link ->
        [usb] USB ケーブルで記録
        [adapter] Bluetooth アダプターで記録
        [radio] Bluetooth リンクで記録
       *[none] 名前のないリンクで記録
    }
# $path is the file's full path.
top-bar-replaying = { $path } をリプレイ中
# $readings is how many readings the file held; $path is the file's full
# path.
top-bar-imported-from = { $path } からインポートした測定値 { $readings } 件

## The support badge, after the status, for a meter not yet fully verified.
## A link to report feedback. Capitals, as the meter's own annunciators.

top-bar-badge-verified = 検証済み
top-bar-badge-partly-verified = 一部検証済み
top-bar-badge-experimental = 試験的
# $warning is the sentence saying how far the meter's support is verified.
top-bar-badge-hover = { $warning }クリックしてフィードバックを送ってください。

## The right end: the version, which opens What's New; Help & Feedback; the
## shortcut list; Settings.

top-bar-version =
    .hover = 最新情報を表示する (このバージョンのリリース ノート)
    .a11y = リリース ノートを表示する
# The version's hover when the bar has no room for the update link.
# $version is the newer release, e.g. v0.10.0.
top-bar-version-update =
    .hover =
        最新情報を表示する (このバージョンのリリース ノート)
        { $version } が公開されています。リンクを表示するにはウィンドウを広げてください。
    .a11y = リリース ノートを表示する。{ $version } が公開されています
# The update link; $version is the newer release, e.g. v0.10.0.
top-bar-update =
    .hover = GitHub で { $version } のリリース ページを開く
    .a11y = { $version } が公開されています: リリース ページを開く
top-bar-project-link =
    .hover = GitHub で dmm-tools のプロジェクト ページを開く
# The ? button. $keys and $other are its two keys, ? and F1.
top-bar-shortcuts =
    .hover = キーボード ショートカットとマウス操作を表示する ({ $keys } または { $other })
    .a11y = キーボード ショートカットとマウス操作
# The ⚙ button.
top-bar-settings =
    .hover = 設定パネルの表示/非表示を切り替える
    .a11y = 設定
