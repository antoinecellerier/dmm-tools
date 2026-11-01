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
args-error = 错误:

## What went wrong.

# An option that takes no value after "=" was given without one.
args-equals-needed = 为 '{ $arg }' 赋值时需要等号
# An option was given without its value.
args-value-required = '{ $arg }' 需要一个值，但没有提供
# A value the option doesn't take. A list of the ones it takes may follow
# on the next line (args-possible-values).
args-invalid-value = 为 '{ $arg }' 提供的值 '{ $value }' 无效
# The same, with the reason the value was refused. $reason may be English.
args-invalid-value-because = 为 '{ $arg }' 提供的值 '{ $value }' 无效: { $reason }
# The values an option takes, on a line of their own under the error. $values
# is the list, separated by commas.
args-possible-values = [可能的值: { $values }]
# A command dmm-cli doesn't have, e.g. 'lst'.
args-unrecognized-subcommand = 无法识别的子命令 '{ $command }'
# Options that must be given, listed one per line under this.
args-missing-required = 未提供以下必需的参数:
# dmm-cli was run without a command. $command is the program, dmm-cli. The
# commands follow on the next line (args-subcommands).
args-missing-subcommand = '{ $command }' 需要一个子命令，但没有提供
# The commands, on a line of their own under the error. $values is the list,
# separated by commas.
args-subcommands = [子命令: { $values }]
# A value after an option that takes no more.
args-unexpected-value = 为 '{ $arg }' 找到了多余的值 '{ $value }'，它不再接受更多的值
# Too few values after an option. $needed and $given are numbers; $count is
# $given, for choosing the verb's form ([0] and [one]: one value or none).
args-too-few-values =
    { $count ->
       *[other] '{ $arg }' 需要 { $needed } 个值，但只提供了 { $given } 个
    }
# The wrong number of values after an option. $needed and $given are
# numbers; $count is $given, for choosing the verb's form.
args-wrong-number-of-values =
    { $count ->
       *[other] '{ $arg }' 需要 { $needed } 个值，但提供了 { $given } 个
    }
# An option dmm-cli doesn't have, or one this command doesn't take.
args-unexpected-argument = 发现意外的参数 '{ $arg }'
# An option given twice that can only be given once.
args-repeated = 参数 '{ $arg }' 不能重复使用
# Two options, or an option and a command, that can't go together. $what is
# argument or subcommand; $name and $other are what the user typed.
args-conflict-with =
    { $what ->
        [subcommand] 子命令 '{ $name }' 不能与 '{ $other }' 一起使用
       *[argument] 参数 '{ $name }' 不能与 '{ $other }' 一起使用
    }
# Two options, or an option and a command, that can't go together, with
# the others listed one per line under this. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-list =
    { $what ->
        [subcommand] 子命令 '{ $name }' 不能与以下内容一起使用:
       *[argument] 参数 '{ $name }' 不能与以下内容一起使用:
    }
# The same, when the others aren't named. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-others =
    { $what ->
        [subcommand] 子命令 '{ $name }' 不能与指定的其他一个或多个参数一起使用
       *[argument] 参数 '{ $name }' 不能与指定的其他一个或多个参数一起使用
    }

## Hints after the error.

# Before a hint, indented on its own line.
args-tip = 提示:
# A name close to what was typed. $what is subcommand, argument or value;
# $names is one or more quoted names, separated by commas; $count is how
# many.
args-similar =
    { $what ->
        [subcommand]
            { $count ->
               *[other] 存在类似的子命令: { $names }
            }
        [argument]
            { $count ->
               *[other] 存在类似的参数: { $names }
            }
       *[value]
            { $count ->
               *[other] 存在类似的值: { $names }
            }
    }
# The last line. $help is --help.
args-more-info = 如需更多信息，请尝试 '{ $help }'。
