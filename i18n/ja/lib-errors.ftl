### The library's errors: why a connection, a command or a file failed.
### They show in the GUI's connection notice under the reading and in its
### toasts (the short messages under the top bar's right end), and in the
### CLI after "Error:". Hand-written; a test checks each English against
### the library's own. The last group is the library's one notice that is
### not an error.
###
### Mode, range and state names arrive already translated, as the reading
### shows them. Button names (SELECT, RANGE, AUTO, EXIT) are printed on the
### meter: keep them as they are, as command-line options (--adapter,
### --device) and command words (auto, rel). Text the operating system or
### the Bluetooth stack wrote arrives as it is, in their language.

## What went wrong, by the part that failed. Several end in a colon and
## the reason, which comes from one of the groups below.

# The USB layer failed. $detail is what it said, in English.
error-hid = HID エラー: { $detail }
# The meter sent something that makes no sense. $detail says what, in
# English, for a bug report.
error-invalid-response = 無効な応答: { $detail }
# The meter on the link is not one the meter chosen can read, worded as
# error-invalid-response. $reason is one of the "Another meter" sentences
# below.
error-wrong-meter = 無効な応答: { $reason }
# A frame from the meter failed its check. $expected and $actual are
# hexadecimal numbers, e.g. 0x01a3.
error-checksum-mismatch = チェックサムが一致しません: 期待値 { $expected }、受信値 { $actual }
error-timeout = 応答待ちがタイムアウトしました
# $reason is one of the "Refused before it was sent" sentences below.
error-unsupported-command = 対応していないコマンド: { $reason }
# $reason is one of the "The meter did not do it" sentences below.
error-command-rejected = コマンドが拒否されました: { $reason }
# $device is the meter's name or ID, as typed with --device or as the
# settings file holds it, sometimes with an explanation in English.
error-unknown-device = 不明なデバイス: { $device }
# A meter on a cable was opened as a simulated one. $device is its ID, e.g.
# ut61eplus.
error-not-simulated = 不明なデバイス: { $device } はシミュレーションのマルチメーターではありません
# A recording could not be played back. $detail names the file or the line
# and says what is wrong with it, in English.
error-replay = リプレイ ファイル: { $detail }
# $adapter is the value given with --adapter, sometimes with an explanation
# in English.
error-adapter-not-found = アダプターが見つかりません: { $adapter }
# What --adapter named, as error-adapter-not-found's $adapter and the GUI's
# connection-adapter-not-matched quote it, when it is a USB device that is
# no supported cable. $path is the device's path, as given with --adapter.
error-not-an-adapter = { $path } (デバイスは存在しますが、対応している USB アダプターではありません)
# --adapter named a cable or adapter the chosen meter cannot be reached
# through. $link is bluetooth (a Bluetooth adapter) or cable (a USB cable
# whose name is $cable, e.g. BU-86X). $model is the meter, e.g. UNI-T
# UT61E+.
error-wrong-cable =
    { $link ->
        [bluetooth] --adapter で指定された Bluetooth アダプターでは { $model } と通信できません: --adapter でマルチメーター専用のケーブルを指定するか、--adapter を省略してください
       *[cable] --adapter で指定された { $cable } ケーブルでは { $model } と通信できません: --adapter でマルチメーター専用のケーブルを指定するか、--adapter を省略してください
    }
error-link-lost = Bluetooth リンクが切断されました
# $reason is one of the "Bluetooth" sentences below.
error-bluetooth = Bluetooth: { $reason }
# A cable or Bluetooth link is up but no meter answered on it. $link is usb
# (a USB cable), adapter (a Bluetooth adapter plugged into the meter) or
# radio (the meter's own Bluetooth).
error-device-not-identified =
    { $link ->
        [adapter] Bluetooth アダプター経由で応答したマルチメーターはありません
        [radio] Bluetooth リンク経由で応答したマルチメーターはありません
       *[usb] USB ケーブル経由で応答したマルチメーターはありません
    }
# No cable was found. $bluetooth is searched when Bluetooth was searched
# too, else not-searched.
error-no-transport-found =
    { $bluetooth ->
        [searched] 対応している USB アダプターも、通信範囲内の Bluetooth デバイスも見つかりません
       *[not-searched] 対応している USB アダプターが見つかりません
    }
# As error-no-transport-found, naming the cables looked for; $bluetooth as
# there. $cables lists them with their USB IDs, e.g. "CP2110 0x10c4:0xea80,
# BU-86X 0x0820:0x0001".
error-no-transport-found-looked-for =
    { $bluetooth ->
        [searched] 対応している USB アダプター (検索対象: { $cables }) も、通信範囲内の Bluetooth デバイスも見つかりません
       *[not-searched] 対応している USB アダプターが見つかりません (検索対象: { $cables })
    }
# A meter with Bluetooth built in, and none was in range. $model is the
# meter, e.g. UNI-T UT60BT.
error-bluetooth-only-not-in-range = Bluetooth の通信範囲内にマルチメーター { $model } が見つかりません
# A meter with Bluetooth built in, and Bluetooth was not searched. $model
# is the meter, e.g. UNI-T UT60BT. Each app adds how to switch it on.
error-bluetooth-only-switched-off = マルチメーター { $model } は Bluetooth でのみ接続できますが、Bluetooth がオフになっています
# $model is the meter, e.g. UNI-T UT60BT.
error-bluetooth-only-not-built = マルチメーター { $model } は Bluetooth でのみ接続できますが、このビルドは Bluetooth に対応していません
# $model is the meter, e.g. UNI-T UT60BT.
error-bluetooth-only-usb-adapter = マルチメーター { $model } は Bluetooth でのみ接続できますが、--adapter で USB デバイスが指定されています: マルチメーターの Bluetooth アドレスを指定するか、--adapter を省略してください
# Opening the meter was stopped part-way, by Disconnect or by quitting.
error-cancelled = 接続を中止しました

## Refused before it was sent: the meter, the link or the dial position
## cannot do it. After "unsupported command:". $setting is a setting's
## name as the CLI takes it (mode, range, rel, minmax…); $word, $target and
## $from_word are a mode's hexadecimal code, e.g. 0x3111.

# $setting is a setting's name.
error-unsupported-setting = このマルチメーターでは { $setting } を設定できません
# $mode is a mode.
error-unsupported-several-positions = { $mode } はファンクションスイッチの複数の位置で報告されます: 先に別のモードで測定値を読み取ってください
# $mode is a mode.
error-unsupported-no-position = { $mode } に対応するファンクションスイッチの位置は不明です
# $target and $from are modes.
error-unsupported-mode-unreachable = { $from } からモード { $target } に切り替えることはできません: 先にファンクションスイッチを回してください
# $target and $from_word are mode codes, $from is the mode $from_word is.
error-unsupported-word-unreachable = { $from } ({ $from_word }) からモード { $target } に切り替えることはできません: 先にファンクションスイッチを回してください
# $mode is a mode.
error-unsupported-no-ranges = このマルチメーターの { $mode } には選択できるレンジがありません
# $id is a range's number, $count how many ranges $mode, a mode, has.
error-unsupported-no-such-range = レンジ { $id } は、{ $mode } の { $count } 個のレンジに含まれていません
# $id is a range's number, $count how many ranges $mode, a mode, has.
error-unsupported-range-not-listed = レンジ { $id } は、{ $mode } の { $count } 個のレンジに含まれていません
# $count is how many ranges $mode, a mode, has.
error-unsupported-auto-range-not-listed = オートレンジは、{ $mode } の { $count } 個のレンジに含まれていません
# $mode is a mode, $word its code.
error-unsupported-fixed-range = { $mode } ({ $word }) のレンジ: レンジ固定のモード
# $setting is a setting's name, $mode a mode.
error-unsupported-setting-not-in-mode = このマルチメーターの { $mode } では { $setting } を設定できません
# $setting is a setting's name, $mode a mode, $word its code.
error-unsupported-setting-not-in-word = このマルチメーターの { $mode } ({ $word }) では { $setting } を設定できません
# $setting is a setting's name, $id a state's number, $mode a mode and
# $offered the states it offers, separated by commas (on, off, MAX…).
error-unsupported-no-such-state = { $setting } に状態 { $id } はありません ({ $mode } で使用できる状態: { $offered })
# $command is a command word, $mode a Peak mode, $word its code.
error-unsupported-refused-in-peak = { $mode } ({ $word }) での { $command }: ピーク モードではマルチメーターが受け付けません
# "rel" is the command word. $mode is a mode, $word its code.
error-unsupported-no-rel = { $mode } ({ $word }) での rel
# $model is the meter's part number, e.g. VC-880; $button a button's name.
error-unsupported-no-button = マルチメーター { $model } には { $button } ボタンがありません
# $cable is a cable's name, e.g. BU-86X; $len and $got are byte counts.
error-unsupported-request-length = { $cable } ケーブルは { $got } バイトではなく { $len } バイトのリクエストを受け付けます
# $link is usb (the USB cable), adapter (the Bluetooth adapter) or none (a
# link that does not say what it is); $baud a serial speed, e.g. 19200.
error-unsupported-baud =
    { $link ->
        [usb] USB ケーブルの通信速度を { $baud } ボーに変更できません
        [adapter] Bluetooth アダプターの通信速度を { $baud } ボーに変更できません
       *[none] リンクの通信速度を { $baud } ボーに変更できません
    }
# A developer's error: the simulated meter has no bytes to decode.
error-unsupported-no-wire-format = parse_payload: シミュレーションのマルチメーターには生のフレームがありません

## The meter did not do it: a command it was sent did not take, or the
## meter's state says it cannot. After "command rejected:". $button is a
## button's name; $state, $target and $now are modes, ranges or setting
## states.

# $button is a button's name, $state what the meter stayed in.
error-rejected-did-nothing = { $button } を押しても変化せず、{ $state } のままです
# $target is what was asked for, $now what the meter is in.
error-rejected-never-appeared = { $target } にならず、マルチメーターは { $now } に戻りました
# $presses is how many times $button was pressed; $state is what the meter
# is in.
error-rejected-gave-up = { $button } を { $presses } 回押したところで中止しました (現在の状態: { $state })
# $mode is a mode.
error-rejected-mode-changed = モードが { $mode } に変わったため、RANGE を押すのを中止しました
# $reason is another sentence of this group, $mode the mode the meter was
# in before.
error-rejected-started-in = { $reason } (開始時のモード: { $mode })
# $range is a range.
error-rejected-auto-did-nothing = AUTO を押しても変化せず、マルチメーターは { $range } のままです
# $button is a button's name, $state a setting's state.
error-rejected-exit-did-nothing = EXIT { $button } を押しても変化せず、マルチメーターは { $state } のままです
# $setting is a setting's name, $target and $now its states.
error-rejected-switch-did-nothing = { $setting } { $target } を実行しても変化せず、マルチメーターは { $now } のままです
# $sent is what was sent: a command word, or one of the error-sent messages
# below, or a setting's name and a state.
error-rejected-by-meter = マルチメーターが { $sent } を拒否しました
error-rejected-zero-needs-capacitance = ZERO は静電容量の測定でのみ機能します
error-rejected-rel-needs-reading = ディスプレイに数値が表示されていないと、REL は使用できません
error-rejected-rel-is-on = REL はすでにオンになっているため、先に解除してください
# $code is a number, $model and $opened are meters (e.g. OWON B41T+), $id a
# meter's ID as --device takes it. "Auto-detect" is the GUI's Device
# choice.
error-rejected-other-model = マルチメーターが報告した OWON のモデル コードは { $code } ({ $model }) で、接続時に指定した { $opened } と異なるため、リモート キーは無効です: [自動検出] または --device { $id } を選択してください
# $code is a number.
error-rejected-unknown-model = マルチメーターが報告した OWON のモデル コード { $code } はこのツールではまだ認識できないため、リモート キーは無効です
# $command is a command word.
error-rejected-model-unread = マルチメーターのモデル コードを読み取れなかったため、{ $command } は無効です: REL を解除するには、マルチメーターの REL を長押ししてください
error-rejected-min-max-locks-range = MIN/MAX ではレンジが固定されます: レンジを変更する前に MIN/MAX を解除してください
# $command is a command word or a setting's name.
error-rejected-replay = { $command }: リプレイではコマンドを使用できません

# What a refused command switched to, as error-rejected-by-meter names it.
# $mode is a mode.
error-sent-mode = モード { $mode }
# $range is a range.
error-sent-range = レンジ { $range }

## Another meter: the one on the link is not one the meter chosen can
## read. After "invalid response:". Model names (OWON B35T+, Brymen BM82x)
## and --device's values (vc871, bm82x) stay as they are. Auto-detect is
## the meter choice that finds the meter itself: the GUI's Device list
## names it so, and the CLI takes it as --device auto.

# An OWON meter names its model, which sends the other kind of data frame.
# $code is OWON's model number, e.g. 87; $model the model it names, e.g.
# Voltcraft VC871; $frame the length of that model's frame in bytes, 6 or
# 15; $opened the meter chosen, e.g. OWON B35T+; $id the --device value
# for $model.
error-wrong-meter-other-frame = マルチメーターが報告した OWON のモデル コードは { $code } ({ $model }) で、接続時に指定した { $opened } ではなく、OWON の { $frame } バイト フレームを送信するモデルです: [自動検出] または --device { $id } を選択してください
# An OWON meter sends the other kind of data frame and names no model.
# $sent and $expected are frame lengths in bytes, 6 or 15; $opened is the
# meter chosen, e.g. Voltcraft VC871. $remedy is auto-detect, or own-model
# when the user has to pick the meter's model themselves.
error-wrong-meter-other-frame-kind =
    { $remedy ->
        [auto-detect] マルチメーターが送信しているのは OWON の { $sent } バイト フレームで、接続時に指定した { $opened } の { $expected } バイト フレームではありません: [自動検出] を選択してください
       *[own-model] マルチメーターが送信しているのは OWON の { $sent } バイト フレームで、接続時に指定した { $opened } の { $expected } バイト フレームではありません: マルチメーターの実際の機種を選択してください
    }
# An OWON model this tool does not read yet. $code is OWON's model number,
# e.g. 83. "This message" is the whole error, for the issue.
error-wrong-meter-not-yet = マルチメーターが報告した OWON のモデル コードは { $code } で、この機種はこのツールではまだ読み取れません: このメッセージを添えて Issue を作成してください
# An OWON model whose readings carry their sign where this tool does not
# read it yet. $code is OWON's model number, e.g. 55.
error-wrong-meter-sign-in-function-word = マルチメーターが報告した OWON のモデル コードは { $code } で、この機種はファンクション ワードに符号を含む OWON の 6 バイト フレームを送信しますが、このツールではまだ読み取れません
# A Brymen meter of another series answered. $found and $opened are series,
# e.g. Brymen BM82x and Brymen BM86x; $id is the --device value for $found.
error-wrong-meter-other-series = ケーブルに接続されているマルチメーターは { $found } で、{ $opened } ではありません: [自動検出] または --device { $id } を選択してください
# An OWON meter chosen by its model sends bytes, but never its 6-byte data
# frame. B35 and B35T are OWON models.
error-wrong-meter-no-six-byte-frames = マルチメーターが OWON の 6 バイト フレームを送信しません: 古い B35 または B35T で、このツールではまだ読み取れない 14 バイト フレームを送信している可能性があります
# The same for a meter whose data frame is 15 bytes long.
error-wrong-meter-no-fifteen-byte-frames = マルチメーターが OWON の 15 バイト フレームを送信しません: [自動検出] を選択してください
# An OWON meter that names no model sends frames of neither length. "This
# message" is the whole error, for the issue.
error-wrong-meter-untold-frame-kind = マルチメーターのモデル コードからもフレームからも、OWON の 6 バイト フレームと 15 バイト フレームのどちらを送信するのか判別できません: このメッセージを添えて Issue を作成してください

## Bluetooth: what the Bluetooth stack could not do. After "Bluetooth:".

# $detail is what the operating system said.
error-bluetooth-runtime = Bluetooth ランタイムを起動できませんでした: { $detail }
error-bluetooth-no-adapter = このコンピューターでは使用できません
error-bluetooth-powered-off = このコンピューターでオフになっています
# $role is write (the characteristic commands are written to) or notify
# (the one the meter's data arrives on).
error-bluetooth-no-characteristic =
    { $role ->
        [write] Bluetooth デバイスにデータ書き込み用のキャラクタリスティックが見つかりません: 対応しているアダプターやマルチメーターではないか、サービスの検出が完了しませんでした
       *[notify] Bluetooth デバイスにデータ通知用のキャラクタリスティックが見つかりません: 対応しているアダプターやマルチメーターではないか、サービスの検出が完了しませんでした
    }
# A Brymen meter refused the password this tool sends.
error-bluetooth-password-refused = マルチメーターが接続パスワード 0000 を拒否しました: リセットするには、{ error-bluetooth-reset-gesture }
# How a Brymen BM788BT/BM787BT's password and name go back to the factory
# settings. The meter's activation steps in lib.ftl (activation-bm78xbt)
# end with the same sentence: word them alike. "Org" is what the display
# shows.
error-bluetooth-reset-gesture = Hz ボタンを押しながら、ファンクションスイッチを 0.6 s 以内に OFF から静電容量の位置まで回してください: マルチメーターに「Org」と表示され、接続パスワードと Bluetooth 名が工場出荷時の設定に戻ります
# A Brymen meter refused the login with error $code, a number.
error-bluetooth-login-refused =
    { $code ->
        [0] マルチメーターがログインを拒否しました: チェックサム エラー (コード { $code })
        [1] マルチメーターがログインを拒否しました: 無効なチャネル ID (コード { $code })
        [2] マルチメーターがログインを拒否しました: 設定範囲外 (コード { $code })
        [5] マルチメーターがログインを拒否しました: 無効な引数 (コード { $code })
        [6] マルチメーターがログインを拒否しました: 権限不足 (コード { $code })
       *[other] マルチメーターがログインを拒否しました: 不明なエラー (コード { $code })
    }

## Not an error: the notice under the reading in the GUI when a Bluetooth
## adapter hands over a reading late, together with the next one.

late-readings-title = 一部の Bluetooth 測定値が遅れて届きます
# $fix is command when a command follows that fixes it for this connection
# (it ends in a colon, and the command shows on the next line as it is), or
# none.
late-readings-advice =
    { $fix ->
        [command] このアダプターでは、測定値が次の測定値とまとめて届くことがあります。測定値が失われることはありません。マルチメーターが切断されるまでの間、正確なタイムスタンプを得るには、次のコマンドを実行してください:
       *[none] このアダプターでは、測定値が次の測定値とまとめて届くことがあります。測定値が失われることはありません。
    }
