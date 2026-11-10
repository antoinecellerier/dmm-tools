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

# After shared.ftl's unrecognised-data, on a warning line: where to report
# it. $url is the issue tracker; a command line to run follows on a line of
# its own.
connect-unrecognised-report = Please report it at { $url } with the meter in this state, attaching the output of:
# The same when the data came from a recording being played back: $url as
# above.
connect-unrecognised-report-file = Please report it at { $url }, attaching the recording.
# $device is the meter id dmm-gui saved in the settings file, e.g.
# ut61eplus.
connect-unknown-device = unknown device: { $device }
# info and debug talk to a real meter. $device is the simulated meter's id,
# e.g. mock.
connect-needs-hardware = This command requires real hardware (not supported with --device { $device }).
# The settings file names a simulated-meter scenario that doesn't exist.
# mock_mode is the setting's name in the file. $error is the reason, in
# English.
connect-saved-mock-mode = mock_mode in the settings file: { $error }
# The settings file dmm-gui saves is there but broken, so the run goes on
# without it. $error is the reason, in English, naming the file, e.g.
# "cannot parse /home/me/.config/dmm-tools/settings.json: EOF while parsing
# a string at line 1 column 22".
connect-settings-unreadable = saved settings ignored until the settings file is fixed or deleted: { $error }

## Which meter the run opens: under a failed open's first line, and at the
## top of 'info'.

# $meter is the meter's brand and model, e.g. UNI-T UT61E+. $by says where
# it was named: flag (--device on the command line; $device is the id
# typed, e.g. ut61eplus), settings (saved by dmm-gui) or recording (the file
# being played back).
connect-selected =
    { $by ->
       *[flag] Selected: { $meter } (--device { $device })
        [settings] Selected: { $meter } (saved by dmm-gui; pass --device auto to detect the meter)
        [recording] Selected: { $meter } (named by the recording)
    }
# The name the meter gave for itself. $name is that name as the meter sent
# it, or connect-reported-name-none.
connect-reported-name = Reported name: { $name }
# In place of the name when the meter gave none, which many never do.
connect-reported-name-none = (none)

## Detection found the meter: one dim line before the readings. $meter is
## its brand and model, e.g. UNI-T UT61E+; $device the id that names it on
## the command line, e.g. ut61eplus.

# The meter gave no name, or the model's own.
connect-detected = Detected { $meter } (pass --device { $device } or pick it in dmm-gui to skip detection next time)
# The meter gave a name of its own that isn't the model's: $reported is
# that name, e.g. UT216XD. "Tables" are how the readings are decoded.
connect-detected-reports = Detected { $meter } (the meter reports "{ $reported }"; using { $meter } tables — pass --device { $device } to keep them, or another id to override)

## Once a meter whose support isn't fully verified is open: a warning, then
## the command that makes a report, on a line of its own, then where to send
## it.

# $warning is the sentence saying how far the meter's support is verified.
connect-unverified-warning = WARNING: { $warning }
# Followed by the command line to run.
connect-unverified-capture = Run 'capture' to generate a report for validation:
# $url is the web address of the meter's issue on GitHub.
connect-report-feedback = Report feedback: { $url }

## Nothing was found to open. A first line, then the steps for each link
## (shared.ftl), then, for a meter whose support isn't fully verified, a
## closing line.

# The first line. $title says what was not found, e.g. "No USB cable
# found"; this message ends the sentence.
connect-not-found = { $title }.
# The closing line. $warning is the sentence saying how far the meter's
# support is verified; $url the web address of its issue on GitHub.
connect-unverified-note = { $warning } Report feedback: { $url }

## A cable or Bluetooth link is open, but no meter on it answered. A first
## line, the meters that link can carry with the steps that switch each one's
## data transmission on, then a closing paragraph.

# The first line. $title is "No meter answered" (shared.ftl). $link is usb
# (a USB cable), adapter (a Bluetooth adapter plugged into the meter) or
# radio (the meter's own Bluetooth).
connect-not-identified =
    { $link ->
        [adapter] { $title } over the Bluetooth adapter.
        [radio] { $title } over the Bluetooth link.
       *[usb] { $title } over the USB cable.
    }
# The closing paragraph; keep its line breaks, and the {"  "} that indents
# the two command lines under the prose. $command is the command line that names the meter
# (dmm-cli --device <id> ...); $help the one that lists the ids
# (dmm-cli --help); $logging the setting that turns the detailed log on
# (RUST_LOG=dmm_lib=debug); $url the web address of the issue tracker.
connect-not-identified-name-it =
    If the meter is on and transmitting but still not recognised, name it:
    {"  "}{ $command }      ({ $help } lists the ids)
    and please report it with { $logging } output at
    {"  "}{ $url }

## --adapter, a command-line option, named a USB device that isn't there.
## Under the list of what is plugged in.

# $selector is the option as typed, --adapter <serial-path-or-address>.
connect-adapter-select = Use { $selector } to select one.

## The meter stopped answering. After "Warning:" (common.ftl); the meter's
## own steps for switching data transmission on follow.

# $title is "No response from meter" (shared.ftl). $meter is the meter
# picked, e.g. UNI-T UT61E+.
connect-no-response = { $title }. Check that it is the { $meter } and that data transmission is enabled.

## The line that closes a failed open, after "Error:" (common.ftl). The
## lines above it have already said what to try.

connect-error-not-found = device not found
connect-error-not-identified = device not identified
connect-error-adapter-not-found = adapter not found
# $device is the meter id given with --device, e.g. auto.
connect-error-not-simulated = --device { $device } is not simulated
