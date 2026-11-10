### The parts of dmm-cli's --help built as it runs: the first line of the
### help of --device, which goes on with a list made from the meters, and
### the notes after the options. What each command and option
### does is in cli-help.ftl.
###
### Words in --flag form, setting names (device_family, mock_mode) and
### environment variables (RUST_LOG, LANG) are typed by the user and stay as
### they are. Command lines, file paths and values arrive as variables.

## The first line of --device's long help (dmm-cli --help).

# Before the list of devices.
help-device-intro = Device to connect to.

## After the options of dmm-cli -h.

help-see-long-help = Run with --help for the full list of supported devices and the shared-settings file path.
# $setting is NO_COLOR=1.
help-no-color = Set { $setting } to disable colored output.

## After the options of dmm-cli --help: two sections, each a heading in
## capitals and indented lines. clap doesn't wrap these lines: keep them
## short.

help-configuration = CONFIGURATION:
# The settings file's path follows on the next line.
help-settings-file = Settings file (shared with dmm-gui):
# Where --device comes from when the command line doesn't give it: three
# numbered lines in order.
help-device-precedence = --device precedence:
help-precedence-flag = Command-line flag
help-precedence-settings = device_family from the settings file above
help-precedence-auto = auto — detect the connected meter
help-environment = ENVIRONMENT:
# Beside RUST_LOG; each line of the message is a line of the help. $filter
# is the filter that shows everything the library sends and receives.
help-env-rust-log =
    Log filter. Unset, dmm_lib warnings and all errors are shown.
    Use `{ $filter }` for wire-level debugging.
# Beside NO_COLOR.
help-env-no-color = Set to 1 to disable colored terminal output.
# Beside LANGUAGE; each line of the message is a line of the help. The
# variable names stay as they are. On macOS and Windows the system's
# language applies instead.
help-env-language =
    Language of messages and help on Linux, as gettext reads it:
    LANGUAGE, then LC_ALL, LC_MESSAGES, LANG. Debian and Ubuntu
    desktops set LANGUAGE, which wins over LANG.
# Beside LC_ALL; each line of the message is a line of the help. C and
# POSIX are the locale names that mean English, on every system.
help-env-lc-all =
    C or POSIX in the first set of LC_ALL, LC_MESSAGES and LANG
    means English, whatever LANGUAGE says.

## After the options of dmm-cli completions --help. The command line for
## each shell follows.

help-completions-install = Install completions for your shell:
