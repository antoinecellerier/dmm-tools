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
error-hid = HID error: { $detail }
# The meter sent something that makes no sense. $detail says what, in
# English, for a bug report.
error-invalid-response = invalid response: { $detail }
# The meter on the link is not one the meter chosen can read, worded as
# error-invalid-response. $reason is one of the "Another meter" sentences
# below.
error-wrong-meter = invalid response: { $reason }
# A frame from the meter failed its check. $expected and $actual are
# hexadecimal numbers, e.g. 0x01a3.
error-checksum-mismatch = checksum mismatch: expected { $expected }, got { $actual }
error-timeout = timeout waiting for response
# $reason is one of the "Refused before it was sent" sentences below.
error-unsupported-command = unsupported command: { $reason }
# $reason is one of the "The meter did not do it" sentences below.
error-command-rejected = command rejected: { $reason }
# $device is the meter's name or ID, as typed with --device or as the
# settings file holds it, sometimes with an explanation in English.
error-unknown-device = unknown device: { $device }
# A meter on a cable was opened as a simulated one. $device is its ID, e.g.
# ut61eplus.
error-not-simulated = unknown device: { $device } is not simulated
# A recording could not be played back. $detail names the file or the line
# and says what is wrong with it, in English.
error-replay = replay file: { $detail }
# $adapter is the value given with --adapter, sometimes with an explanation
# in English.
error-adapter-not-found = adapter not found: { $adapter }
# What --adapter named, as error-adapter-not-found's $adapter and the GUI's
# connection-adapter-not-matched quote it, when it is a USB device that is
# no supported cable. $path is the device's path, as given with --adapter.
error-not-an-adapter = { $path } (device exists but is not a supported USB adapter)
# --adapter named a cable or adapter the chosen meter cannot be reached
# through. $link is bluetooth (a Bluetooth adapter) or cable (a USB cable
# whose name is $cable, e.g. BU-86X). $model is the meter, e.g. UNI-T
# UT61E+.
error-wrong-cable =
    { $link ->
        [bluetooth] --adapter names a Bluetooth adapter, which cannot carry the { $model }: name the meter's own cable with --adapter, or leave --adapter out
       *[cable] --adapter names a { $cable } cable, which cannot carry the { $model }: name the meter's own cable with --adapter, or leave --adapter out
    }
error-link-lost = Bluetooth link lost
# $reason is one of the "Bluetooth" sentences below.
error-bluetooth = Bluetooth: { $reason }
# A cable or Bluetooth link is up but no meter answered on it. $link is usb
# (a USB cable), adapter (a Bluetooth adapter plugged into the meter) or
# radio (the meter's own Bluetooth).
error-device-not-identified =
    { $link ->
        [adapter] no meter answered over the Bluetooth adapter
        [radio] no meter answered over the Bluetooth link
       *[usb] no meter answered over the USB cable
    }
# No cable was found. $bluetooth is searched when Bluetooth was searched
# too, else not-searched.
error-no-transport-found =
    { $bluetooth ->
        [searched] no supported USB adapter found, nor a Bluetooth device in range
       *[not-searched] no supported USB adapter found
    }
# As error-no-transport-found, naming the cables looked for; $bluetooth as
# there. $cables lists them with their USB IDs, e.g. "CP2110 0x10c4:0xea80,
# BU-86X 0x0820:0x0001".
error-no-transport-found-looked-for =
    { $bluetooth ->
        [searched] no supported USB adapter found (looked for { $cables }), nor a Bluetooth device in range
       *[not-searched] no supported USB adapter found (looked for { $cables })
    }
# A meter with Bluetooth built in, and none was in range. $model is the
# meter, e.g. UNI-T UT60BT.
error-bluetooth-only-not-in-range = no { $model } found in Bluetooth range
# A meter with Bluetooth built in, and Bluetooth was not searched. $model
# is the meter, e.g. UNI-T UT60BT. Each app adds how to switch it on.
error-bluetooth-only-switched-off = { $model } connects over Bluetooth only, and Bluetooth is switched off
# $model is the meter, e.g. UNI-T UT60BT.
error-bluetooth-only-not-built = { $model } connects over Bluetooth only, and this build has no Bluetooth support
# $model is the meter, e.g. UNI-T UT60BT.
error-bluetooth-only-usb-adapter = { $model } connects over Bluetooth only, and --adapter names a USB device: pass the meter's Bluetooth address, or leave --adapter out
# Opening the meter was stopped part-way, by Disconnect or by quitting.
error-cancelled = open called off

## Refused before it was sent: the meter, the link or the dial position
## cannot do it. After "unsupported command:". $setting is a setting's
## name as the CLI takes it (mode, range, rel, minmax…); $word, $target and
## $from_word are a mode's hexadecimal code, e.g. 0x3111.

# $setting is a setting's name.
error-unsupported-setting = { $setting } cannot be set on this meter
# $mode is a mode.
error-unsupported-several-positions = { $mode } is reported from more than one dial position — take a reading in another mode first
# $mode is a mode.
error-unsupported-no-position = { $mode } is on no known dial position
# $target and $from are modes.
error-unsupported-mode-unreachable = mode { $target } is not reachable from { $from } — turn the dial first
# $target and $from_word are mode codes, $from is the mode $from_word is.
error-unsupported-word-unreachable = mode { $target } is not reachable from { $from } ({ $from_word }) — turn the dial first
# $mode is a mode.
error-unsupported-no-ranges = { $mode } has no range to choose on this meter
# $id is a range's number, $count how many ranges $mode, a mode, has.
error-unsupported-no-such-range = range { $id } is not one of the { $count } { $mode } offers
# $id is a range's number, $count how many ranges $mode, a mode, has.
error-unsupported-range-not-listed = range { $id } is not one of the { $count } ranges { $mode } offers
# $count is how many ranges $mode, a mode, has.
error-unsupported-auto-range-not-listed = auto range is not one of the { $count } ranges { $mode } offers
# $mode is a mode, $word its code.
error-unsupported-fixed-range = range in { $mode } ({ $word }): fixed-range mode
# $setting is a setting's name, $mode a mode.
error-unsupported-setting-not-in-mode = { $setting } cannot be set in { $mode } on this meter
# $setting is a setting's name, $mode a mode, $word its code.
error-unsupported-setting-not-in-word = { $setting } cannot be set in { $mode } ({ $word }) on this meter
# $setting is a setting's name, $id a state's number, $mode a mode and
# $offered the states it offers, separated by commas (on, off, MAX…).
error-unsupported-no-such-state = { $setting } has no state { $id }; { $mode } offers { $offered }
# $command is a command word, $mode a Peak mode, $word its code.
error-unsupported-refused-in-peak = { $command } in { $mode } ({ $word }): the meter refuses it in Peak
# "rel" is the command word. $mode is a mode, $word its code.
error-unsupported-no-rel = rel in { $mode } ({ $word })
# $model is the meter's part number, e.g. VC-880; $button a button's name.
error-unsupported-no-button = the { $model } has no { $button } button
# $cable is a cable's name, e.g. BU-86X; $len and $got are byte counts.
error-unsupported-request-length = the { $cable } cable takes { $len }-byte requests, not { $got } bytes
# $link is usb (the USB cable), adapter (the Bluetooth adapter) or none (a
# link that does not say what it is); $baud a serial speed, e.g. 19200.
error-unsupported-baud =
    { $link ->
        [usb] the USB cable cannot change its rate to { $baud } baud
        [adapter] the Bluetooth adapter cannot change its rate to { $baud } baud
       *[none] the link cannot change its rate to { $baud } baud
    }
# A developer's error: the simulated meter has no bytes to decode.
error-unsupported-no-wire-format = parse_payload: the mock has no wire format

## The meter did not do it: a command it was sent did not take, or the
## meter's state says it cannot. After "command rejected:". $button is a
## button's name; $state, $target and $now are modes, ranges or setting
## states.

# $button is a button's name, $state what the meter stayed in.
error-rejected-did-nothing = { $button } did nothing in { $state }
# $target is what was asked for, $now what the meter is in.
error-rejected-never-appeared = { $target } never appeared; the meter is back in { $now }
# $presses is how many times $button was pressed; $state is what the meter
# is in.
error-rejected-gave-up = gave up after { $presses } presses of { $button }; the meter is in { $state }
# $mode is a mode.
error-rejected-mode-changed = the mode changed to { $mode }; stopped pressing RANGE
# $reason is another sentence of this group, $mode the mode the meter was
# in before.
error-rejected-started-in = { $reason }; started in { $mode }
# $range is a range.
error-rejected-auto-did-nothing = AUTO did nothing; the meter is still in { $range }
# $button is a button's name, $state a setting's state.
error-rejected-exit-did-nothing = EXIT { $button } did nothing; the meter is still in { $state }
# $setting is a setting's name, $target and $now its states.
error-rejected-switch-did-nothing = { $setting } { $target } did nothing; the meter is still in { $now }
# $sent is what was sent: a command word, or one of the error-sent messages
# below, or a setting's name and a state.
error-rejected-by-meter = { $sent } rejected by the meter
error-rejected-zero-needs-capacitance = ZERO works in capacitance only
# $code is a number, $model and $opened are meters (e.g. OWON B41T+), $id a
# meter's ID as --device takes it. "Auto-detect" is the GUI's Device
# choice.
error-rejected-other-model = the meter reports OWON model code { $code } ({ $model }), not the { $opened } it was opened as, so remote keys are off; choose Auto-detect or --device { $id }
# $code is a number.
error-rejected-unknown-model = the meter reports OWON model code { $code }, which this tool does not know yet, so remote keys are off
# $command is a command word.
error-rejected-model-unread = the meter's model code was not read, so { $command } is off: hold REL on the meter to leave REL
error-rejected-min-max-locks-range = MIN/MAX locks the range; leave MIN/MAX before changing it
# $command is a command word or a setting's name.
error-rejected-replay = { $command }: a replay takes no commands

# What a refused command switched to, as error-rejected-by-meter names it.
# $mode is a mode.
error-sent-mode = mode { $mode }
# $range is a range.
error-sent-range = range { $range }

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
error-wrong-meter-other-frame = the meter reports OWON model code { $code } ({ $model }), which sends OWON's { $frame }-byte frame, not the { $opened } it was opened as; choose Auto-detect or --device { $id }
# An OWON meter sends the other kind of data frame and names no model.
# $sent and $expected are frame lengths in bytes, 6 or 15; $opened is the
# meter chosen, e.g. Voltcraft VC871. $remedy is auto-detect, or own-model
# when the user has to pick the meter's model themselves.
error-wrong-meter-other-frame-kind =
    { $remedy ->
        [auto-detect] the meter sends OWON's { $sent }-byte frames, not the { $expected }-byte frames of the { $opened } it was opened as; choose Auto-detect
       *[own-model] the meter sends OWON's { $sent }-byte frames, not the { $expected }-byte frames of the { $opened } it was opened as; choose the meter's own model
    }
# An OWON model this tool does not read yet. $code is OWON's model number,
# e.g. 83. "This message" is the whole error, for the issue.
error-wrong-meter-not-yet = the meter reports OWON model code { $code }, which this tool does not read yet; please open an issue with this message
# An OWON model whose readings carry their sign where this tool does not
# read it yet. $code is OWON's model number, e.g. 55.
error-wrong-meter-sign-in-function-word = the meter reports OWON model code { $code }: it sends OWON's 6-byte frame with the sign in the function word, which this tool does not read yet
# A Brymen meter of another series answered. $found and $opened are series,
# e.g. Brymen BM82x and Brymen BM86x; $id is the --device value for $found.
error-wrong-meter-other-series = the meter on the cable is a { $found }, not a { $opened }; choose Auto-detect or --device { $id }
# An OWON meter chosen by its model sends bytes, but never its 6-byte data
# frame. B35 and B35T are OWON models.
error-wrong-meter-no-six-byte-frames = the meter sends no OWON 6-byte frames; it may be an older B35 or B35T, whose 14-byte frames this tool does not read yet
# The same for a meter whose data frame is 15 bytes long.
error-wrong-meter-no-fifteen-byte-frames = the meter sends no OWON 15-byte frames; choose Auto-detect
# An OWON meter that names no model sends frames of neither length. "This
# message" is the whole error, for the issue.
error-wrong-meter-untold-frame-kind = neither the meter's model code nor its frames show whether it sends OWON's 6- or 15-byte frames; please open an issue with this message

## Bluetooth: what the Bluetooth stack could not do. After "Bluetooth:".

# $detail is what the operating system said.
error-bluetooth-runtime = could not start the Bluetooth runtime: { $detail }
error-bluetooth-no-adapter = not available on this computer
error-bluetooth-powered-off = turned off on this computer
# $role is write (the characteristic commands are written to) or notify
# (the one the meter's data arrives on).
error-bluetooth-no-characteristic =
    { $role ->
        [write] the Bluetooth device has no data write characteristic we recognise — it is not a supported adapter or meter, or its services never resolved
       *[notify] the Bluetooth device has no data notify characteristic we recognise — it is not a supported adapter or meter, or its services never resolved
    }
# A Brymen meter refused the password this tool sends.
error-bluetooth-password-refused = the meter refused the connection password 0000 — to reset it, { error-bluetooth-reset-gesture }
# How a Brymen BM788BT/BM787BT's password and name go back to the factory
# settings. The meter's activation steps in lib.ftl (activation-bm78xbt)
# end with the same sentence: word them alike. "Org" is what the display
# shows.
error-bluetooth-reset-gesture = hold the Hz button while turning the dial from OFF to capacitance within 0.6 s: the meter shows "Org" and its connection password and Bluetooth name are back to the factory settings
# A Brymen meter refused the login with error $code, a number.
error-bluetooth-login-refused =
    { $code ->
        [0] the meter refused the login: checksum error (code { $code })
        [1] the meter refused the login: invalid channel ID (code { $code })
        [2] the meter refused the login: out of setting range (code { $code })
        [5] the meter refused the login: invalid arguments (code { $code })
        [6] the meter refused the login: insufficient permissions (code { $code })
       *[other] the meter refused the login: unknown error (code { $code })
    }

## Not an error: the notice under the reading in the GUI when a Bluetooth
## adapter hands over a reading late, together with the next one.

late-readings-title = Some Bluetooth readings arrive late
# $fix is command when a command follows that fixes it for this connection
# (it ends in a colon, and the command shows on the next line as it is), or
# none.
late-readings-advice =
    { $fix ->
        [command] On this adapter, a reading sometimes arrives grouped with the next one. None are lost. For accurate timestamps until the meter disconnects, run:
       *[none] On this adapter, a reading sometimes arrives grouped with the next one. None are lost.
    }
