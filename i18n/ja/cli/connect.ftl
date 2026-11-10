### What dmm-cli prints while it starts and opens the meter: which meter it
### was told to open, what detection found, and what to try when nothing
### answers. A terminal program: these lines go to the terminal's error
### stream, and stay readable when the readings are piped to a file.
###
### Words in --flag form, setting names (mock_mode), 'capture' and dmm-gui
### are typed or known by those names and stay as they are. Command lines
### and web addresses are never part of a message: they arrive as variables,
### on lines of their own.

## Refusals before anything is opened. Each follows "Error:" or "Warning:"
## (common.ftl) on the same line.

# info and debug talk to a real meter. $device is the simulated meter's id,
# e.g. mock.
connect-needs-hardware = このコマンドには実際のハードウェアが必要です (--device { $device } では使用できません)。
# The settings file dmm-gui saves is there but broken, so the run goes on
# without it. $error is the reason, in English, naming the file, e.g.
# "cannot parse /home/me/.config/dmm-tools/settings.json: EOF while parsing
# a string at line 1 column 22".
connect-settings-unreadable = 設定ファイルを修正または削除するまで、保存された設定は無視されます: { $error }

## Which meter the run opens: under a failed open's first line, and at the
## top of 'info'.

# $meter is the meter's brand and model, e.g. UNI-T UT61E+. $by says where
# it was named: flag (--device on the command line; $device is the id
# typed, e.g. ut61eplus), settings (saved by dmm-gui) or recording (the file
# being played back).
connect-selected =
    { $by ->
       *[flag] 選択: { $meter } (--device { $device })
        [settings] 選択: { $meter } (dmm-gui で保存済み。マルチメーターを検出するには --device auto を指定してください)
        [recording] 選択: { $meter } (リプレイ ファイルで指定)
    }
# The name the meter gave for itself. $name is that name as the meter sent
# it, or connect-reported-name-none.
connect-reported-name = 返された名前: { $name }
# In place of the name when the meter gave none, which many never do.
connect-reported-name-none = (なし)

## Detection found the meter: one dim line before the readings. $meter is
## its brand and model, e.g. UNI-T UT61E+; $device the id that names it on
## the command line, e.g. ut61eplus.

# The meter gave no name, or the model's own.
connect-detected = { $meter } を検出しました (次回から検出を省くには、--device { $device } を指定するか dmm-gui で選択してください)
# The meter gave a name of its own that isn't the model's: $reported is
# that name, e.g. UT216XD. "Tables" are how the readings are decoded.
connect-detected-reports = { $meter } を検出しました (マルチメーターが返した名前は「{ $reported }」です。{ $meter } のテーブルを使用します。このテーブルを使い続けるには --device { $device } を、別のテーブルを使うにはほかの ID を指定してください)

## Once a meter whose support isn't fully verified is open: a warning, then
## the command that makes a report, on a line of its own, then where to send
## it.

# $warning is the sentence saying how far the meter's support is verified.
connect-unverified-warning = 警告: { $warning }
# Followed by the command line to run.
connect-unverified-capture = 検証用のレポートを作成するには 'capture' を実行してください:
# $url is the web address of the meter's issue on GitHub.
connect-report-feedback = フィードバックの送信先: { $url }

## Nothing was found to open. A first line, then the steps for each link
## (shared.ftl), then, for a meter whose support isn't fully verified, a
## closing line.

# The first line. $title says what was not found, e.g. "No USB cable
# found"; this message ends the sentence.
connect-not-found = { $title }。
# The closing line. $warning is the sentence saying how far the meter's
# support is verified; $url the web address of its issue on GitHub.
connect-unverified-note = { $warning }フィードバックの送信先: { $url }

## A cable or Bluetooth link is open, but no meter on it answered. A first
## line, the meters that link can carry with the steps that switch each one's
## data transmission on, then a closing paragraph.

# The first line. $title is "No meter answered" (shared.ftl). $link is usb
# (a USB cable), adapter (a Bluetooth adapter plugged into the meter) or
# radio (the meter's own Bluetooth).
connect-not-identified =
    { $link ->
        [adapter] { $title } (Bluetooth アダプター経由)。
        [radio] { $title } (Bluetooth リンク経由)。
       *[usb] { $title } (USB ケーブル経由)。
    }
# The closing paragraph; keep its line breaks, and the {"  "} that indents
# the two command lines under the prose. $command is the command line that names the meter
# (dmm-cli --device <id> ...); $help the one that lists the ids
# (dmm-cli --help); $logging the setting that turns the detailed log on
# (RUST_LOG=dmm_lib=debug); $url the web address of the issue tracker.
connect-not-identified-name-it =
    マルチメーターの電源が入っていてデータを送信しているのに認識されない場合は、機種を指定してください:
    {"  "}{ $command }      ({ $help } で ID の一覧を表示)
    また、{ $logging } の出力を添えて次の場所に報告してください:
    {"  "}{ $url }

## --adapter, a command-line option, named a USB device that isn't there.
## Under the list of what is plugged in.

# $selector is the option as typed, --adapter <serial-path-or-address>.
connect-adapter-select = { $selector } で 1 つ選択してください。

## The meter stopped answering. After "Warning:" (common.ftl); the meter's
## own steps for switching data transmission on follow.

# $title is "No response from meter" (shared.ftl). $meter is the meter
# picked, e.g. UNI-T UT61E+.
connect-no-response = { $title }。マルチメーターが { $meter } であることと、データ転送が有効になっていることをご確認ください。

## The line that closes a failed open, after "Error:" (common.ftl). The
## lines above it have already said what to try.

connect-error-not-found = デバイスが見つかりません
connect-error-not-identified = デバイスを識別できません
connect-error-adapter-not-found = アダプターが見つかりません
# $device is the meter id given with --device, e.g. auto.
connect-error-not-simulated = --device { $device } はシミュレーションではありません
