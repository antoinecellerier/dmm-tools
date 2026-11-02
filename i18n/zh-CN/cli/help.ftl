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
help-device-intro = 要连接的设备。
# Before the list of the simulated meter's modes.
help-mock-mode-intro = 将仿真设备固定在指定的测量模式，而不是自动循环。仅在使用 --device mock 时有效。未指定时，由设置文件中的 mock_mode (dmm-gui 的“仿真模式”) 固定。

## The words around every command's help: its headings, what follows an
## option, and the help of the built-in -h, -V and help. They are the same
## on every page of dmm-cli --help and dmm-cli <command> --help.

# The headings, each before its list.
help-usage = 用法:
help-commands = 命令:
help-arguments = 参数:
help-options = 选项:
# After an option's help, in brackets. $value is what the option is when not
# given, e.g. 500.
help-default = [默认值: { $value }]
# After an option's help, in brackets. $values is the values it takes,
# separated by commas.
help-possible-values = [可能的值: { $values }]
# Before the list of values an option takes, each with what it does.
help-possible-values-heading = 可能的值:
# What -h and --help do: on a command with more detail under --help, the two
# notes say how to see more or less.
help-flag = 打印帮助
help-flag-short = 打印帮助 (使用 '--help' 查看更多)
help-flag-long = 打印帮助 (使用 '-h' 查看摘要)
# What -V and --version do.
help-version-flag = 打印版本
# The help command, listed among the commands, and its argument.
help-command = 打印此消息或给定子命令的帮助
help-command-subcommand = 打印子命令的帮助

## After the options of dmm-cli -h.

help-see-long-help = 使用 --help 运行可查看支持的全部设备列表和共享设置文件的路径。
# $setting is NO_COLOR=1.
help-no-color = 设置 { $setting } 可禁用彩色输出。

## After the options of dmm-cli --help: two sections, each a heading in
## capitals and indented lines. clap doesn't wrap these lines: keep them
## short.

help-configuration = 配置:
# The settings file's path follows on the next line.
help-settings-file = 设置文件 (与 dmm-gui 共用):
# Where --device comes from when the command line doesn't give it: three
# numbered lines in order.
help-device-precedence = --device 的优先顺序:
help-precedence-flag = 命令行选项
help-precedence-settings = 上述设置文件中的 device_family
help-precedence-auto = auto: 检测已连接的万用表
help-environment = 环境变量:
# Beside RUST_LOG; each line of the message is a line of the help. $filter
# is the filter that shows everything the library sends and receives.
help-env-rust-log =
    日志过滤器。未设置时显示 dmm_lib 的警告和所有错误。
    使用 `{ $filter }` 进行底层通信调试。
# Beside NO_COLOR.
help-env-no-color = 设为 1 可禁用彩色终端输出。
# Beside LANGUAGE; each line of the message is a line of the help. The
# variable names stay as they are. On macOS and Windows the system's
# language applies instead.
help-env-language =
    Linux 上消息和帮助的语言，读取方式同 gettext:
    依次为 LANGUAGE、LC_ALL、LC_MESSAGES、LANG。Debian 和 Ubuntu
    桌面会设置 LANGUAGE，其优先级高于 LANG。
# Beside LC_ALL; each line of the message is a line of the help. C and
# POSIX are the locale names that mean English, on every system.
help-env-lc-all =
    LC_ALL、LC_MESSAGES、LANG 中第一个已设置的值为 C 或 POSIX 时
    使用英语，无论 LANGUAGE 如何设置。

## After the options of dmm-cli completions --help. The command line for
## each shell follows.

help-completions-install = 为 shell 安装补全:
