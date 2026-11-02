### Mistakes on the command line: an unknown option, a missing command, a
### value dmm-cli doesn't take. Printed on the terminal as
###
###     error: invalid value 'xml' for '--format <FORMAT>'
###       [possible values: csv, json, replay]
###
###     Usage: dmm-cli read [OPTIONS]
###
###     For more information, try '--help'.
###
### The usage line stays English, like the rest of --help's layout. Names
### in quotes ($arg, $value, $command, $names) are what the user typed or
### must type: they arrive as they are, sometimes coloured, and stay in the
### quotes. $arg is an option, often with its value's name, e.g.
### --format <FORMAT>.

## The start of the line.

# Before every message, as the terminal shows errors.
args-error = エラー:

## What went wrong.

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
# A command dmm-cli doesn't have, e.g. 'lst'.
args-unrecognized-subcommand = サブコマンド '{ $command }' は認識されません
# Options that must be given, listed one per line under this.
args-missing-required = 次の必須の引数が指定されていません:
# dmm-cli was run without a command. $command is the program, dmm-cli. The
# commands follow on the next line (args-subcommands).
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
# An option dmm-cli doesn't have, or one this command doesn't take.
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
