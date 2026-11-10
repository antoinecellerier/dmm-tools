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

# $device is the meter id dmm-gui saved in the settings file, e.g.
# ut61eplus.
connect-unknown-device = 未知设备: { $device }
# info and debug talk to a real meter. $device is the simulated meter's id,
# e.g. mock.
connect-needs-hardware = 此命令需要真实硬件 (--device { $device } 不支持)。
# The settings file names a simulated-meter scenario that doesn't exist.
# mock_mode is the setting's name in the file. $error is the reason, in
# English.
connect-saved-mock-mode = 设置文件中的 mock_mode: { $error }
# The settings file dmm-gui saves is there but broken, so the run goes on
# without it. $error is the reason, in English, naming the file, e.g.
# "cannot parse /home/me/.config/dmm-tools/settings.json: EOF while parsing
# a string at line 1 column 22".
connect-settings-unreadable = 在修复或删除设置文件之前，将忽略已保存的设置: { $error }

## Which meter the run opens: under a failed open's first line, and at the
## top of 'info'.

# $meter is the meter's brand and model, e.g. UNI-T UT61E+. $by says where
# it was named: flag (--device on the command line; $device is the id
# typed, e.g. ut61eplus), settings (saved by dmm-gui) or recording (the file
# being played back).
connect-selected =
    { $by ->
       *[flag] 已选择: { $meter } (--device { $device })
        [settings] 已选择: { $meter } (由 dmm-gui 保存；传入 --device auto 可检测万用表)
        [recording] 已选择: { $meter } (由回放文件指定)
    }
# The name the meter gave for itself. $name is that name as the meter sent
# it, or connect-reported-name-none.
connect-reported-name = 万用表上报的名称: { $name }
# In place of the name when the meter gave none, which many never do.
connect-reported-name-none = (无)

## Detection found the meter: one dim line before the readings. $meter is
## its brand and model, e.g. UNI-T UT61E+; $device the id that names it on
## the command line, e.g. ut61eplus.

# The meter gave no name, or the model's own.
connect-detected = 已检测到 { $meter } (传入 --device { $device } 或在 dmm-gui 中选择，下次即可跳过检测)
# The meter gave a name of its own that isn't the model's: $reported is
# that name, e.g. UT216XD. "Tables" are how the readings are decoded.
connect-detected-reports = 已检测到 { $meter } (万用表上报的名称为“{ $reported }”，现使用 { $meter } 的解码表；传入 --device { $device } 可保持不变，传入其他 id 可改用别的型号)

## Once a meter whose support isn't fully verified is open: a warning, then
## the command that makes a report, on a line of its own, then where to send
## it.

# $warning is the sentence saying how far the meter's support is verified.
connect-unverified-warning = 警告: { $warning }
# Followed by the command line to run.
connect-unverified-capture = 运行 'capture' 生成用于验证的报告:
# $url is the web address of the meter's issue on GitHub.
connect-report-feedback = 提供反馈: { $url }

## Nothing was found to open. A first line, then the steps for each link
## (shared.ftl), then, for a meter whose support isn't fully verified, a
## closing line.

# The first line. $title says what was not found, e.g. "No USB cable
# found"; this message ends the sentence.
connect-not-found = { $title }。
# The closing line. $warning is the sentence saying how far the meter's
# support is verified; $url the web address of its issue on GitHub.
connect-unverified-note = { $warning } 提供反馈: { $url }

## A cable or Bluetooth link is open, but no meter on it answered. A first
## line, the meters that link can carry with the steps that switch each one's
## data transmission on, then a closing paragraph.

# The first line. $title is "No meter answered" (shared.ftl). $link is usb
# (a USB cable), adapter (a Bluetooth adapter plugged into the meter) or
# radio (the meter's own Bluetooth).
connect-not-identified =
    { $link ->
        [adapter] 蓝牙适配器上{ $title }。
        [radio] 蓝牙链路上{ $title }。
       *[usb] USB 连接线上{ $title }。
    }
# The closing paragraph; keep its line breaks, and the {"  "} that indents
# the two command lines under the prose. $command is the command line that names the meter
# (dmm-cli --device <id> ...); $help the one that lists the ids
# (dmm-cli --help); $logging the setting that turns the detailed log on
# (RUST_LOG=dmm_lib=debug); $url the web address of the issue tracker.
connect-not-identified-name-it =
    如果万用表已开机并在发送数据，但仍未被识别，请指定型号:
    {"  "}{ $command }      ({ $help } 列出所有 id)
    并附上 { $logging } 的输出，反馈到
    {"  "}{ $url }

## --adapter, a command-line option, named a USB device that isn't there.
## Under the list of what is plugged in.

# $selector is the option as typed, --adapter <serial-path-or-address>.
connect-adapter-select = 使用 { $selector } 选择其中一个。

## The meter stopped answering. After "Warning:" (common.ftl); the meter's
## own steps for switching data transmission on follow.

# $title is "No response from meter" (shared.ftl). $meter is the meter
# picked, e.g. UNI-T UT61E+.
connect-no-response = { $title }。请确认设备是 { $meter }，并已开启数据传输。

## The line that closes a failed open, after "Error:" (common.ftl). The
## lines above it have already said what to try.

connect-error-not-found = 找不到设备
connect-error-not-identified = 无法识别设备
connect-error-adapter-not-found = 找不到适配器
# $device is the meter id given with --device, e.g. auto.
connect-error-not-simulated = --device { $device } 不是仿真设备
