### The parts of dmm-cli's --help built as it runs: the first lines of the
### help of --device and --mock-mode, which go on with lists made from the
### meters, and the notes after the options. What each command and option
### does is in cli-help.ftl.
###
### Words in --flag form, setting names (device_family, mock_mode) and
### environment variables (RUST_LOG, LANG) are typed by the user and stay as
### they are. Command lines, file paths and values arrive as variables.

## The first lines of two options' long help (dmm-cli --help, dmm-cli read
## --help).

# Before the list of devices.
help-device-intro = 接続するデバイス。
# Before the list of the simulated meter's modes.
help-mock-mode-intro = シミュレーション デバイスの測定モードを、自動で切り替えずに特定のモードに固定する。--device mock を指定したときのみ有効。指定しない場合は、設定ファイルの mock_mode (dmm-gui のシミュレーション モード) で固定される。

## The words around every command's help: its headings, what follows an
## option, and the help of the built-in -h, -V and help. They are the same
## on every page of dmm-cli --help and dmm-cli <command> --help.

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

## After the options of dmm-cli -h.

help-see-long-help = 対応デバイスの一覧と共有設定ファイルのパスを見るには、--help を付けて実行してください。
# $setting is NO_COLOR=1.
help-no-color = 色付きの出力を無効にするには { $setting } を設定してください。

## After the options of dmm-cli --help: two sections, each a heading in
## capitals and indented lines. clap doesn't wrap these lines: keep them
## short.

help-configuration = 構成:
# The settings file's path follows on the next line.
help-settings-file = 設定ファイル (dmm-gui と共有):
# Where --device comes from when the command line doesn't give it: three
# numbered lines in order.
help-device-precedence = --device の優先順位:
help-precedence-flag = コマンド ライン オプション
help-precedence-settings = 上記の設定ファイルの device_family
help-precedence-auto = auto: 接続されたマルチメーターを検出する
help-environment = 環境変数:
# Beside RUST_LOG; each line of the message is a line of the help. $filter
# is the filter that shows everything the library sends and receives.
help-env-rust-log =
    ログ フィルター。未設定時は dmm_lib の警告と
    すべてのエラーを表示する。
    通信内容のデバッグには `{ $filter }` を使用する。
# Beside NO_COLOR.
help-env-no-color = 1 にすると端末出力の色付けを無効にする。
# Beside LANGUAGE; each line of the message is a line of the help. The
# variable names stay as they are. On macOS and Windows the system's
# language applies instead.
help-env-language =
    Linux でのメッセージとヘルプの言語。gettext と同じく
    LANGUAGE、次に LC_ALL, LC_MESSAGES, LANG の順に読む。
    Debian と Ubuntu のデスクトップは LANGUAGE を設定し、
    これが LANG より優先される。
# Beside LC_ALL; each line of the message is a line of the help. C and
# POSIX are the locale names that mean English, on every system.
help-env-lc-all =
    LC_ALL, LC_MESSAGES, LANG のうち最初に設定されたものが C または
    POSIX なら、LANGUAGE に関係なく英語になる。

## After the options of dmm-cli completions --help. The command line for
## each shell follows.

help-completions-install = シェルの補完をインストールしてください:
