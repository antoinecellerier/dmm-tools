### Text both dmm-cli and dmm-gui show: the shared parts of their --help
### and the words clap puts around it, their command-line errors, the
### setup help when no meter is found, and the notices both print.
###
### Command lines, file paths and web addresses are never part of a message.
### Words in --flag form, and 'dmm-cli list', are typed by the user and stay
### as they are.

## The project page link: the last line of both binaries' --help, and the
## link in the GUI's top bar.

project-link = ヘルプとフィードバック
# $url is the project page's address.
project-link-line = { project-link }: { $url }

## The --device flag's long help, after the binary's own first line. The
## device list between the heading and the aliases is generated.

device-list-heading = デバイス:
# What the "auto" device does. Also its hover in the GUI's device list.
device-auto = 接続されているマルチメーターを検出する
# The "auto" row of the list: what the flag does when it isn't given.
device-auto-default = { device-auto } (既定)
device-aliases =
    別名も使用できる: ut61e+, ut61b, ut171a, ut181 など
    特殊文字を含む名前は引用符で囲む: --device 'ut61e+'
# Beside the simulated meters, in device lists.
device-no-hardware = ハードウェア不要

## How far a meter's support is verified, beside it in device lists.

stability-verified = 検証済み
stability-partly-verified = 一部検証済み
stability-experimental = 試験的
# Both binaries show it for a meter short of verified, each followed by its
# own way to help verify it.
# $model is the meter's name, e.g. "UNI-T UT8803".
# $stability is verified, partly-verified or experimental.
stability-warning =
    { $stability ->
        [verified] マルチメーター { $model } への対応は検証済みです (実機で検証済み)。
        [partly-verified] マルチメーター { $model } への対応は一部検証済みです (一部のモードとコマンドは実機で確認されていません)。
       *[experimental] マルチメーター { $model } への対応は試験的です (実機で検証されていません)。
    }

## The --mock-mode flag's long help, after the binary's own first line.

# $modes lists the mock's modes, separated by commas.
mock-mode-list = モード: { $modes }
# $example is a command line.
mock-mode-example = 例: { $example }
mock-clock-mock-only = --mock-clock-scale と --mock-clock-preseed は、シミュレーションのマルチメーターとリプレイにのみ適用されます。--device mock または --replay を指定するか、クロックのオプションを付けずにもう一度実行してください。

## A link's short name, where the meter is already named: the GUI's
## status line, the setup help's section labels.

link-usb-cable = USB ケーブル
link-bluetooth = Bluetooth

## The help shown when no meter was found: a title, then one section per
## link searched — a label line, then steps. The CLI prints them in a
## terminal, the GUI in a notice.

# No full stop: the CLI adds one, the GUI's notice titles carry none.
not-found-usb = USB ケーブルが見つかりません
not-found-usb-or-bluetooth = USB でも Bluetooth でもマルチメーターが見つかりません
# $address is a Bluetooth address or peripheral id.
not-found-bluetooth-at = { $address } の Bluetooth デバイスが見つかりません
# $model is the meter's name, e.g. "UNI-T UT60BT".
not-found-bluetooth-only = Bluetooth でマルチメーター { $model } が見つかりません
# A section's label line.
# $link is link-usb-cable or link-bluetooth.
# $check is setup-usb-check or setup-bluetooth-check.
setup-label = { $link }: { $check }
setup-usb-check = ケーブルが接続され、マルチメーターの電源が入っていることをご確認ください。
setup-bluetooth-check = このコンピューターの Bluetooth と、マルチメーターのデータ転送をオンにしてください。
# Linux. Two command lines follow.
setup-usb-linux-rule = udev ルールがインストールされていることをご確認ください:
# Linux. The setup guide's address follows.
setup-usb-linux-replug = その後、ケーブルを接続し直してください。ディスプレイのないマシンでは、ルールにグループを指定してください:
# Windows. The quoted names are what Device Manager shows; use the names
# Windows shows in your language.
setup-usb-windows-open = ケーブルを接続した状態でデバイス マネージャーを開いてください:
setup-usb-windows-cp2110 = - [ヒューマン インターフェイス デバイス] の下の 'CP2110 USB to UART Bridge': 操作は必要ありません。
# UNVERIFIED Windows name
setup-usb-windows-input = - [ヒューマン インターフェイス デバイス] の下の 'USB 入力デバイス': 操作は必要ありません。
# The driver's download address follows.
setup-usb-windows-warning = - [ほかのデバイス] の下に黄色の警告アイコンが表示されている場合は、次の場所からドライバーをインストールしてください:
setup-usb-windows-nothing = - 何も表示されない場合は、別の USB ポートをお試しください。
# macOS. Name the settings as macOS shows them in your language.
setup-usb-macos-auto = ケーブルは自動的に認識されます (ドライバーは不要です)。
setup-usb-macos-privacy = デバイスが見つからない場合は、[システム設定] > [プライバシーとセキュリティ] > [入力監視] をご確認ください。
setup-bluetooth-adapter = 両方ともオンになっている場合は、アダプターをオフにしてからもう一度オンにしてください。
setup-bluetooth-macos-privacy = [システム設定] > [プライバシーとセキュリティ] > [Bluetooth] で Bluetooth の使用を許可してください。
# The device catalog's address follows.
setup-bluetooth-catalog = 対応しているアダプターとマルチメーターの一覧は、デバイス カタログをご覧ください:
setup-bluetooth-scan = 'dmm-cli list' を実行すると、通信範囲内のアダプターとマルチメーターをスキャンできます。

## Notices both binaries show.

# A cable or Bluetooth link is open but no meter on it identified itself.
# A notice title in the GUI; dmm-cli starts a sentence with it, naming the
# link after it (cli/connect.ftl).
no-meter-answered = 応答したマルチメーターはありません
# The meter picked or found has stopped answering. A notice title in the
# GUI; dmm-cli starts a sentence with it (cli/connect.ftl).
no-response = マルチメーターから応答がありません
# The GUI checkbox that lets a connection search over Bluetooth. The CLI
# reads the same setting, so its help names the checkbox too.
bluetooth-setting = Bluetooth デバイスを探す
bluetooth-off-cli-flag = --no-bluetooth を付けずに実行してください。
bluetooth-off-cli-setting = dmm-gui の設定で [{ bluetooth-setting }] チェック ボックスをオンにしてください。dmm-cli もこの設定を読み取ります。
bluetooth-off-gui = [設定] (⚙) で [{ bluetooth-setting }] チェック ボックスをオンにしてください。

## What is connected, when an --adapter value matched nothing.

adapters-none = 現在接続されているデバイスはありません。
adapters-heading = 接続されているデバイス:
adapters-unavailable = 接続されているデバイスを確認するには、'dmm-cli list' を実行してください。
# $address is the value the user typed, written with colons, e.g. 12:34:56:78:9A:BC.
colonless-address-hint = Bluetooth アドレスの場合は、{ $address } のように記述してください。

## Why a number typed for a scale, an offset or an alarm was refused: the
## reason alone, in lower case and with no full stop. dmm-gui puts it after
## the field's name in a toast ("Invalid scale: …"); dmm-cli after the
## option and the value, at the end of a line from its command-line parser,
## which stays English ("error: invalid value '0' for '--scale <FACTOR>': …").

# $text is what was typed, as it was typed.
input-not-a-number = 「{ $text }」は数値ではありません
# Infinity or "not a number" (NaN).
input-not-finite = 有限の数値である必要があります
# A scale factor of 0 would turn every reading into the offset.
input-zero-scale = 倍率を 0 にすると測定値が消えます
# The alarm's low limit and its high limit.
input-low-not-below-high = 下限は上限より小さくする必要があります
# The alarm's hysteresis band: a number in the limits' unit, or a percentage
# of the limit. $text is what was typed, as it was typed.
input-band-not-a-number = 「{ $text }」は数値でも、1% のようなパーセンテージでもありません
# A hysteresis band below zero.
input-band-negative = 0 以上である必要があります
# A percentage band with a limit of 0, of which every percentage is 0.
input-percent-of-zero = 制限値が 0 のときはパーセンテージも常に 0 になるため、数値で指定してください

## Why an exported file or a recording did not read: in dmm-gui's toast
## after "Import failed:" and the file, in dmm-cli after "Error:" and the
## file. Lower case, with no full stop. The backquotes (`) set off text
## from the file, or a name it must have, which stays as it is: keep them.
## A parser's own words ($detail) arrive in English.

# Where the file stopped: $line is a line number, $reason one of the
# reasons below.
read-error-at-line = { $line } 行目: { $reason }
read-error-empty = ファイルが空です
# A CSV export without its first row naming the columns.
read-error-no-header-row = ヘッダー行がありません ({ $detail })
# $column is the name of a column every CSV export has, e.g. timestamp.
read-error-no-column = `{ $column }` 列がありません
# A JSON export whose first line is not the one starting {"_metadata":.
read-error-no-metadata = `_metadata` 行がありません ({ $detail })
# $text is what the file has where a reading's value should be.
read-error-not-a-value = `{ $text }` は値ではありません
# $text is a cell starting NCV: whose level is no number. NCV is
# non-contact voltage detection, as the meter prints it.
read-error-not-an-ncv-level = `{ $text }` は NCV レベルではありません
# $text is the part of a flags cell that names no status flag.
read-error-not-a-flag = `{ $text }` はステータス フラグではありません
# $text is what the file has where a marker's number should be.
read-error-not-a-marker-number = `{ $text }` はマーカー番号ではありません
# $text is a timestamp from the file, or a recording's "# recorded:" line;
# RFC 3339 is the standard date format the files use.
read-error-not-a-date = `{ $text }` は RFC 3339 形式の日付ではありません: { $detail }

## The words clap puts around every page of dmm-cli's and dmm-gui's --help:
## its headings, what follows an option, and the help of the built-in -h,
## -V and help. They are the same on every page.

# The headings, each before its list.
help-usage = 使用法:
help-commands = コマンド:
help-arguments = 引数:
help-options = オプション:
# After an option's help, in brackets. $value is what the option is when not
# given, e.g. 500.
help-default = [既定値: { $value }]
# After an option's help, in brackets. $values is the values it takes,
# separated by commas.
help-possible-values = [指定可能な値: { $values }]
# Before the list of values an option takes, each with what it does.
help-possible-values-heading = 指定可能な値:
# What -h and --help do: on a command with more detail under --help, the two
# notes say how to see more or less.
help-flag = ヘルプを表示する
help-flag-short = ヘルプを表示する (詳しくは '--help')
help-flag-long = ヘルプを表示する (概要は '-h')
# What -V and --version do.
help-version-flag = バージョンを表示する
# The help command, listed among the commands, and its argument.
help-command = このメッセージ、または指定したサブコマンドのヘルプを表示する
help-command-subcommand = 指定したサブコマンドのヘルプを表示する

## Mistakes on dmm-cli's and dmm-gui's command line: an unknown option, a
## missing command, a value the program doesn't take. Printed on the
## terminal as
##
##     error: invalid value 'xml' for '--format <FORMAT>'
##       [possible values: csv, json, replay]
##
##     Usage: dmm-cli read [OPTIONS]
##
##     For more information, try '--help'.
##
## The usage line's syntax stays English after its heading. Names in quotes
## ($arg, $value, $command, $names) are what the user typed or must type:
## they arrive as they are, sometimes coloured, and stay in the quotes. $arg
## is an option, often with its value's name, e.g. --format <FORMAT>.

# At the start of every message, as the terminal shows errors.
args-error = エラー:

# An option that takes no value after "=" was given without one.
args-equals-needed = '{ $arg }' に値を指定するには等号が必要です
# An option was given without its value.
args-value-required = '{ $arg }' には値が必要ですが、指定されていません
# A value the option doesn't take. A list of the ones it takes may follow
# on the next line (args-possible-values).
args-invalid-value = '{ $arg }' には値 '{ $value }' を使用できません
# The same, with the reason the value was refused. $reason may be English.
args-invalid-value-because = '{ $arg }' には値 '{ $value }' を使用できません: { $reason }
# The values an option takes, on a line of their own under the error. $values
# is the list, separated by commas.
args-possible-values = [指定可能な値: { $values }]
# A command the program doesn't have, e.g. 'lst'.
args-unrecognized-subcommand = サブコマンド '{ $command }' は認識されません
# Options that must be given, listed one per line under this.
args-missing-required = 次の必須の引数が指定されていません:
# The program was run without a command. $command is the program, e.g.
# dmm-cli. The commands follow on the next line (args-subcommands).
args-missing-subcommand = '{ $command }' にはサブコマンドが必要ですが、指定されていません
# The commands, on a line of their own under the error. $values is the list,
# separated by commas.
args-subcommands = [サブコマンド: { $values }]
# A value after an option that takes no more.
args-unexpected-value = '{ $arg }' に予期しない値 '{ $value }' があります (これ以上の値は指定できません)
# Too few values after an option. $needed and $given are numbers; $count is
# $given, for choosing the verb's form ([0] and [one]: one value or none).
args-too-few-values = '{ $arg }' には { $needed } 個の値が必要ですが、{ $given } 個しか指定されていません
# The wrong number of values after an option. $needed and $given are
# numbers; $count is $given, for choosing the verb's form.
args-wrong-number-of-values = '{ $arg }' には { $needed } 個の値が必要ですが、{ $given } 個が指定されました
# An option the program doesn't have, or one this command doesn't take.
args-unexpected-argument = 予期しない引数 '{ $arg }' があります
# An option given twice that can only be given once.
args-repeated = 引数 '{ $arg }' は複数回指定できません
# Two options, or an option and a command, that can't go together. $what is
# argument or subcommand; $name and $other are what the user typed.
args-conflict-with =
    { $what ->
        [subcommand] サブコマンド '{ $name }' は '{ $other }' と同時に使用できません
       *[argument] 引数 '{ $name }' は '{ $other }' と同時に使用できません
    }
# Two options, or an option and a command, that can't go together, with
# the others listed one per line under this. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-list =
    { $what ->
        [subcommand] サブコマンド '{ $name }' は次のものと同時に使用できません:
       *[argument] 引数 '{ $name }' は次のものと同時に使用できません:
    }
# The same, when the others aren't named. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-others =
    { $what ->
        [subcommand] サブコマンド '{ $name }' は、指定されたほかの引数の 1 つ以上と同時に使用できません
       *[argument] 引数 '{ $name }' は、指定されたほかの引数の 1 つ以上と同時に使用できません
    }

## Hints after the error.

# Before a hint, indented on its own line.
args-tip = ヒント:
# A name close to what was typed. $what is subcommand, argument or value;
# $names is one or more quoted names, separated by commas; $count is how
# many.
args-similar =
    { $what ->
        [subcommand] 似たサブコマンドがあります: { $names }
        [argument] 似た引数があります: { $names }
       *[value] 似た値があります: { $names }
    }
# The last line. $help is --help.
args-more-info = 詳しくは '{ $help }' をお試しください。
