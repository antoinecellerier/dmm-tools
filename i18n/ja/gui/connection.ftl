### The connection notices drawn in the reading column under the reading
### (or, in the big-meter modes, its title in place of the reading and the
### rest in a tooltip), and the toasts: the short messages that float under
### the top bar's right end for a few seconds.
###
### ⚙ is the settings button's icon, which the notices point at. The
### notices are hard-wrapped to fit a narrow column: keep their line breaks
### where a line would otherwise run long.

## While there is nothing to show yet, or no more to come.

# The meter is being identified: each kind of meter gets a turn to answer.
connection-detecting = マルチメーターを検出中…
# A recording being played back reached its last reading.
connection-ended = リプレイが終了しました
# A stretch of a recording being played back that has no readings in it.
connection-no-frames = 記録のこの区間にはフレームがありません
# The meter has gone quiet for a moment. $dots is one to four dots, padded
# with spaces to four characters, that grow while the wait lasts: keep it
# at the end, with nothing after it.
connection-waiting = マルチメーターを待機中{ $dots }
# Under the waiting title, when a meter is picked in Settings.
connection-waiting-check-selection = [設定] (⚙) で正しいデバイスが選択されているかを確認するか、[自動検出] を選択してください。
# Under the waiting title, when Settings is on Auto-detect.
connection-waiting-switch-on = マルチメーターのデータ転送をオンにするか、[設定] (⚙) で機種を選択してください。

## A failed connection: a title naming the problem, the steps to try, and a
## closing line.

# Under a failed connection's steps when a meter is picked in Settings,
# which a detected connect may have saved. $model is that meter, e.g.
# UNI-T UT61E+.
connection-selected-meter = [設定] (⚙) で { $model } が選択されています。使用するマルチメーターが別の機種の場合は、[設定] で選択し直すか、[自動検出] を選択してください。
# The same when the title above already names the meter picked.
connection-selected-meter-named = 使用するマルチメーターが別の機種の場合は、[設定] (⚙) で選択し直すか、[自動検出] を選択してください。
# After shared.ftl's unrecognised-data, in a toast: where to report it.
# { project-link } is the top bar's Help & Feedback link.
connection-unrecognised-report = トップ バーの [{ project-link }] から報告してください。
# The closing line under the steps. "Connect" is the top bar's button.
connection-click-connect = 問題を解決してから、もう一度 [接続] を選択してください。
# The closing line after the Bluetooth steps when nothing answered at the
# address given with --adapter, a command-line option kept as it is. "It"
# is the scan command the steps above name.
connection-restart-with-address = そのコマンドが表示するアドレスを --adapter に指定して、もう一度起動してください。

# A meter that connects over Bluetooth only, and Bluetooth was not looked at.
connection-bluetooth-not-searched = Bluetooth は検索されませんでした
# Under that title when Bluetooth is switched off. $error is the sentence
# saying why it was not searched; $hint says how to switch it
# on.
connection-bluetooth-not-searched-off = { $error }。{ $hint }
# Under that title otherwise. $error is the sentence saying why.
connection-bluetooth-not-searched-why = { $error }。

# --adapter, a command-line option, named a USB device that isn't there.
connection-adapter-not-found = アダプターが見つかりません
# $selector is the value given with --adapter.
connection-adapter-not-matched = --adapter '{ $selector }' に一致するデバイスはありません。
# After the list of what is plugged in.
connection-adapter-restart = 正しい --adapter の値を指定して、もう一度起動してください。

# The cable or Bluetooth link is up but no meter answered. $link is usb (a
# USB cable), adapter (a Bluetooth adapter plugged into the meter) or radio
# (the meter's own Bluetooth). A list of meters and their steps follows the
# colon.
connection-not-identified =
    { $link ->
        [adapter] Bluetooth アダプターは接続されていますが、マルチメーターを識別できませんでした。
        [radio] Bluetooth リンクは確立されていますが、マルチメーターを識別できませんでした。
       *[usb] USB ケーブルは接続されていますが、マルチメーターを識別できませんでした。
    }

    マルチメーターのデータ転送をオンにするか、[設定] (⚙) で機種を選択してください:
# After that list.
connection-not-identified-report = すでにデータを転送しているのに認識されない場合は、[設定] (⚙) で機種を選択し、報告してください。

# The meter stopped answering. $link is usb (a USB cable), adapter (a
# Bluetooth adapter plugged into the meter), radio (the meter's own
# Bluetooth) or none (a simulated meter, on no link). $model is the meter
# picked, e.g. UNI-T UT61E+; $steps are its own steps for switching data
# transmission on, several lines.
connection-no-response =
    { $link ->
        [usb]
            USB ケーブルは接続されていますが、選択中の { $model } が
            応答しません。
        [adapter]
            Bluetooth アダプターは接続されていますが、選択中の { $model } が
            応答しません。
        [radio]
            Bluetooth リンクは確立されていますが、選択中の { $model } が
            応答しません。
       *[none]
            リンクは確立されていますが、マルチメーターが
            応答しません ({ $model } を選択中)。
    }

    デバイスが違う場合は、[設定] (⚙) で変更するか、[自動検出] を選択してください。
    それ以外の場合は、データ転送をオンにしてください:
    { $steps }
# The same, on Auto-detect: no meter is picked to name.
connection-no-response-auto =
    マルチメーターのデータ転送をオンにするか、[設定] (⚙) で
    機種を選択してください。
# The meter answered, but with data that could not be read: a corrupt
# frame, or a mode this tool does not know. The error itself follows.
connection-bad-data = マルチメーターから読み取れないデータが届きました
# A dropped link that was not reconnected. shared.ftl's several-cables
# follows, then connection-several-cables-action. "Connect" is the top bar's
# button.
connection-several-cables = リンクが切断されました
connection-several-cables-action = このマルチメーターのケーブルだけを接続した状態で、[接続] をクリックしてください。
# A toast, when that notice ends a running recording. What it holds can still
# be exported.
connection-several-cables-recording = リンクが切断され、このマルチメーターの USB ケーブルをほかのケーブルと区別できないため、記録を停止しました。
# A toast, when a different meter answers the session during a recording,
# as after a reconnect under Auto-detect or a device picked in Settings.
# $meter is the meter that answered, a model name. What was recorded can
# still be exported.
connection-other-meter-recording = 記録の開始時とは別のマルチメーター { $meter } が応答したため、記録を停止しました。記録済みのデータは引き続きエクスポートできます。
# Any other failure to connect or to keep reading. The error itself
# follows, then connection-click-connect.
connection-failed = 接続できませんでした
# The reading thread crashed, or the system refused to start it. $detail is
# the crash's or the system's own text, which stays English. Followed by
# connection-click-connect.
connection-internal-error = 内部エラー: { $detail }

# A meter that stopped answering in the middle of a session. $title is the
# shared notice title, "No response from meter".
connection-no-response-check = { $title }。デバイスの選択とデータ転送をご確認ください。
# The reading thread stopped by itself: it crashed, or gave up connecting.
connection-acquisition-stopped = データ取得が予期せず停止しました。再開するには再接続してください。

# The last line of a notice for a meter whose support isn't fully verified;
# a link. $warning is the sentence saying how far it is verified.
connection-feedback = { $warning }フィードバックを送る
    .hover = GitHub の Issues を開く

## The notice under the reading when the Bluetooth link delivers readings
## two at a time, which can end with a command that fixes it.

# Copies the command to the clipboard.
connection-late-copy = コピー
# A toast, once the command is copied.
connection-late-copied = コマンドをコピーしました
# The notice's last line, on a link the tool has no fix for yet.
# { project-link } is the top bar's Help & Feedback link.
connection-late-report = マルチメーターの機種とオペレーティング システムを明記して、トップ バーの [{ project-link }] から報告してください。

## The note under the reading that asks, once, for a capture of a meter whose
## support is experimental. A capture is a guided run of the dmm-cli tool
## that records what the meter sends in each of its modes.

connection-capture-ask-title = 検証にご協力ください
# $warning is the sentence saying how far the meter's support is verified,
# which names the meter. "yours" is the reader's own meter.
connection-capture-ask-body = { $warning }お使いのマルチメーターでガイドに沿ったキャプチャを行うことが、このプロジェクトへの最も価値のある貢献になります。どの機能が動作するかが確認でき、問題があれば修正につながります。
# The same on a phone, which has no dmm-cli to run the capture with;
# $warning as above.
connection-capture-ask-body-computer = { $warning }お使いのマルチメーターでガイドに沿ったキャプチャを行うことが、このプロジェクトへの最も価値のある貢献になります。どの機能が動作するかが確認でき、問題があれば修正につながります。キャプチャは、このアプリでマルチメーターを切断したあと、パソコンで dmm-cli を使って行います。
# Above the command that runs the capture: this window has the meter, and
# the command needs it. Copy is the button after the command, and the note
# goes when the top bar's Disconnect is pressed, so the copy comes first.
connection-capture-ask-disconnect = このコマンドをコピーし、マルチメーターを切断してから実行してください:
# A link to the contributors' guide on GitHub; ↗ marks a link.
connection-capture-ask-link = キャプチャの方法 ↗
    .hover = GitHub でキャプチャのガイドを開く

## Toasts.

# After shared.ftl's saved-mock-mode-unknown. Mock mode is the setting's
# name in Settings.
connection-saved-mock-mode-unknown-action = [設定] (⚙) でシミュレーション モードを選択してください。

# A meter identified itself while Settings was on Auto-detect. $meter is
# its name, e.g. UNI-T UT61E+. $saved is yes when Settings now names it, no
# for a session that detects every time.
connection-detected =
    { $saved ->
        [yes] { $meter } を検出し、使用するデバイスとして保存しました。もう一度検出するには、[設定] で [自動検出] を選択してください。
       *[no] { $meter } を検出しました。
    }
# The same, when the name the meter gave for itself differs; $meter and
# $saved as there. $reported is that name, e.g. UT216XD.
connection-detected-reports =
    { $saved ->
        [yes] 「{ $reported }」と報告するマルチメーターを { $meter } として検出し、使用するデバイスとして保存しました。もう一度検出するには、[設定] で [自動検出] を選択してください。
       *[no] 「{ $reported }」と報告するマルチメーターを { $meter } として検出しました。
    }
# Detection settled on a meter, then the meter named another. $reported is
# the name it gave, $named the model it belongs to. $saved is yes when
# Settings now names that model.
connection-named-after-detection =
    { $saved ->
        [yes] マルチメーターは「{ $reported }」と報告しています。使用するデバイスとして保存しました。{ $named } として読み取るには再接続してください。
       *[no] マルチメーターは「{ $reported }」と報告しています。{ $named } として読み取るには再接続してください。
    }

# The recording stopped because Buffer size, in Settings, was lowered below
# what it held. $count is how many samples it took, $kept the same number
# written short, e.g. 1.2K. Export… is a button.
connection-buffer-shrunk = 記録を停止しました。サンプル { $kept } 件は保持されています。[エクスポート…] で保存できます。
# The recording filled the buffer. $count is its size in samples, $kept the
# same number written short, e.g. 500K.
connection-buffer-full = 記録を停止しました。バッファーがいっぱいです (サンプル { $kept } 件)。

# A button command sent to the meter failed. $command is its name as the
# meter's protocol knows it, e.g. hold; $error says why.
connection-command-failed = コマンド '{ $command }' を実行できませんでした: { $error }
# Switching a meter setting failed. $setting is mode, range or the
# setting's lowercase name as the command line takes it (hold, rel, minmax,
# peak); $error says why.
connection-switch-failed =
    { $setting ->
        [mode] モードを切り替えられませんでした: { $error }
        [range] レンジを切り替えられませんでした: { $error }
       *[other] { $setting } を切り替えられませんでした: { $error }
    }
# One of the two messages above ($failed), then what to do about it, a
# sentence of its own ($remedy): what to pick when the meter is another
# model (shared.ftl's choose-meter-gui), or what to check after a refused
# switch (switch-check-dial).
connection-failed-then = { $failed }。{ $remedy }

# The × that closes a toast or a notice.
connection-dismiss =
    .hover = 閉じる
    .a11y = 通知を閉じる
