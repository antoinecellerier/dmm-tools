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

# The closing line under the steps. "Connect" is the top bar's button.
connection-click-connect = 問題を解決したら、[接続] をクリックしてください。
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
# A toast once connected, when other cables were there to open and the session
# took the first, with --adapter a command-line option kept as it is. $count is
# 2 or more.
connection-cables-at-open = 使用できる USB ケーブルが { $count } 本接続されています。最初に見つかったケーブルを使用しています。別のケーブルを選択するには、--adapter を指定してください。
# Follows connection-cables-at-open when nothing tells those cables apart.
connection-cables-no-reconnect = これらのケーブルは区別できません。--adapter でケーブルのパスを指定しない限り、リンクが切断されるとそのまま接続が終了します。
# A dropped link that was not reconnected. shared.ftl's several-cables
# follows, then connection-several-cables-action. "Connect" is the top bar's
# button.
connection-several-cables = リンクが切断されました
connection-several-cables-action = このマルチメーターのケーブルだけを接続した状態で、[接続] をクリックしてください。
# A toast, when that notice ends a running recording. What it holds can still
# be exported.
connection-several-cables-recording = リンクが切断され、このマルチメーターの USB ケーブルをほかのケーブルと区別できないため、記録を停止しました。
# Any other failure to connect or to keep reading. The error itself
# follows, then connection-click-connect.
connection-failed = 接続できませんでした

# A meter that stopped answering in the middle of a session. $title is the
# shared notice title, "No response from meter".
connection-no-response-check = { $title }。デバイスの選択とデータ転送をご確認ください。
# The reading thread stopped by itself: it crashed, or gave up connecting.
connection-acquisition-stopped = データ取得が予期せず停止しました。再開するには再接続してください。

# The last line of a notice for a meter whose support isn't fully verified;
# a link. $warning is the sentence saying how far it is verified.
connection-feedback = { $warning }フィードバックを送る
    .hover = GitHub の Issues を開いてフィードバックを送る

## The notice under the reading when the Bluetooth link delivers readings
## two at a time, which can end with a command that fixes it.

# Copies the command to the clipboard.
connection-late-copy = コピー
# A toast, once the command is copied.
connection-late-copied = コマンドをコピーしました

## Toasts.

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
# The meter named another model than the one picked in Settings. $reported
# is the name it gave, e.g. UT161E; $picked the model picked, $named the
# model that name belongs to, e.g. UNI-T UT161E.
connection-other-model = マルチメーターは選択した { $picked } ではなく「{ $reported }」と報告しています。[設定] で { $named } または [自動検出] を選択してください。
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

# The × that closes a toast or a notice.
connection-dismiss =
    .hover = 閉じる
    .a11y = 通知を閉じる
