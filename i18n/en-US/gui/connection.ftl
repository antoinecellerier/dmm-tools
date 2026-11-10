### The connection notices drawn in the reading column under the reading
### (or, in the big-meter modes, its title in place of the reading and the
### rest in a tooltip), and the toasts: the short messages that float under
### the top bar's right end for a few seconds.
###
### ⚙ is the settings button's icon, which the notices point at. The
### notices are hard-wrapped to fit a narrow column: keep their line breaks
### where a line would otherwise run long.

## While there is nothing to show yet, or no more to come.

# The meter is being identified: each kind of meter gets a turn to answer.
connection-detecting = Detecting the meter…
# A recording being played back reached its last reading.
connection-ended = The recording has ended
# A stretch of a recording being played back that has no readings in it.
connection-no-frames = No frames in the recording here
# The meter has gone quiet for a moment. $dots is one to four dots, padded
# with spaces to four characters, that grow while the wait lasts: keep it
# at the end, with nothing after it.
connection-waiting = Waiting for meter{ $dots }
# Under the waiting title, when a meter is picked in Settings.
connection-waiting-check-selection = Check that the correct device is selected in Settings (⚙), or pick Auto-detect there
# Under the waiting title, when Settings is on Auto-detect.
connection-waiting-switch-on = Switch on the meter's data transmission, or pick the model in Settings (⚙)

## A failed connection: a title naming the problem, the steps to try, and a
## closing line.

# The closing line under the steps. "Connect" is the top bar's button.
connection-click-connect = Click "Connect" after resolving the issue.
# The closing line after the Bluetooth steps when nothing answered at the
# address given with --adapter, a command-line option kept as it is. "It"
# is the scan command the steps above name.
connection-restart-with-address = Restart with --adapter set to the address it prints.

# A meter that connects over Bluetooth only, and Bluetooth was not looked at.
connection-bluetooth-not-searched = Bluetooth not searched
# Under that title when Bluetooth is switched off. $error is the sentence
# saying why it was not searched; $hint says how to switch it
# on.
connection-bluetooth-not-searched-off = { $error }. { $hint }
# Under that title otherwise. $error is the sentence saying why.
connection-bluetooth-not-searched-why = { $error }.

# --adapter, a command-line option, named a USB device that isn't there.
connection-adapter-not-found = Adapter not found
# $selector is the value given with --adapter.
connection-adapter-not-matched = No device matched --adapter '{ $selector }'.
# After the list of what is plugged in.
connection-adapter-restart = Restart with the correct --adapter value.

# The cable or Bluetooth link is up but no meter answered. $link is usb (a
# USB cable), adapter (a Bluetooth adapter plugged into the meter) or radio
# (the meter's own Bluetooth). A list of meters and their steps follows the
# colon.
connection-not-identified =
    { $link ->
        [adapter] The Bluetooth adapter is connected but no meter identified itself.
        [radio] The Bluetooth link is connected but no meter identified itself.
       *[usb] The USB cable is connected but no meter identified itself.
    }

    Switch on the meter's data transmission, or pick the model in Settings (⚙):
# After that list.
connection-not-identified-report = Already transmitting and still not recognised? Pick its model in Settings (⚙) and please report it.

# The meter stopped answering. $link is usb (a USB cable), adapter (a
# Bluetooth adapter plugged into the meter), radio (the meter's own
# Bluetooth) or none (a simulated meter, on no link). $model is the meter
# picked, e.g. UNI-T UT61E+; $steps are its own steps for switching data
# transmission on, several lines.
connection-no-response =
    { $link ->
        [usb]
            The USB cable is connected but the meter
            isn't responding ({ $model } selected).
        [adapter]
            The Bluetooth adapter is connected but the meter
            isn't responding ({ $model } selected).
        [radio]
            The Bluetooth link is connected but the meter
            isn't responding ({ $model } selected).
       *[none]
            The link is connected but the meter
            isn't responding ({ $model } selected).
    }

    If this is the wrong device, change it in Settings (⚙), or pick Auto-detect there.
    Otherwise, enable data transmission:
    { $steps }
# The same, on Auto-detect: no meter is picked to name.
connection-no-response-auto =
    Switch on the meter's data transmission,
    or pick the model in Settings (⚙).

# The meter answered, but with data that could not be read: a corrupt
# frame, or a mode this tool does not know. The error itself follows.
connection-bad-data = Meter sent data that could not be read
# A toast once connected, when other cables were there to open and the session
# took the first, with --adapter a command-line option kept as it is. $count is
# 2 or more.
connection-cables-at-open =
    { $count ->
       *[other] { $count } USB cables this session could open are plugged in. It took the first it found. Pass --adapter to pick another.
    }
# Follows connection-cables-at-open when nothing tells those cables apart.
connection-cables-no-reconnect = These cables can't be told apart, so a dropped link will end this session unless --adapter names a cable's path.
# A dropped link that was not reconnected. shared.ftl's several-cables
# follows, then connection-several-cables-action. "Connect" is the top bar's
# button.
connection-several-cables = Link lost
connection-several-cables-action = Leave only this meter's cable plugged in, then Connect.
# A toast, when that notice ends a running recording. What it holds can still
# be exported.
connection-several-cables-recording = Recording stopped: the link dropped and this meter's USB cable can't be told from the others
# Any other failure to connect or to keep reading. The error itself
# follows, then connection-click-connect.
connection-failed = Connection failed
# The reading thread crashed. $detail is the crash's own text, which stays
# English. Followed by connection-click-connect.
connection-internal-error = Internal error: { $detail }

# A meter that stopped answering in the middle of a session. $title is the
# shared notice title, "No response from meter".
connection-no-response-check = { $title } — check device selection and data transmission
# The reading thread stopped without saying why.
connection-acquisition-stopped = Acquisition stopped unexpectedly — reconnect to resume

# The last line of a notice for a meter whose support isn't fully verified;
# a link. $warning is the sentence saying how far it is verified.
connection-feedback = { $warning } Report feedback.
    .hover = Opens the GitHub issue tracker to report feedback

## The notice under the reading when the Bluetooth link delivers readings
## two at a time, which can end with a command that fixes it.

# Copies the command to the clipboard.
connection-late-copy = Copy
# A toast, once the command is copied.
connection-late-copied = Command copied

## Toasts.

# A meter identified itself while Settings was on Auto-detect. $meter is
# its name, e.g. UNI-T UT61E+. $saved is yes when Settings now names it, no
# for a session that detects every time.
connection-detected =
    { $saved ->
        [yes] Detected { $meter }, saved as your device. Pick Auto-detect in Settings to probe again.
       *[no] Detected { $meter }
    }
# The same, when the name the meter gave for itself differs; $meter and
# $saved as there. $reported is that name, e.g. UT216XD.
connection-detected-reports =
    { $saved ->
        [yes] Detected { $meter } (the meter reports "{ $reported }"), saved as your device. Pick Auto-detect in Settings to probe again.
       *[no] Detected { $meter } (the meter reports "{ $reported }")
    }
# The meter named another model than the one picked in Settings. $reported
# is the name it gave, e.g. UT161E; $picked the model picked, $named the
# model that name belongs to, e.g. UNI-T UT161E.
connection-other-model = The meter reports "{ $reported }", not the { $picked } selected. Choose { $named } or Auto-detect in Settings.
# Detection settled on a meter, then the meter named another. $reported is
# the name it gave, $named the model it belongs to. $saved is yes when
# Settings now names that model.
connection-named-after-detection =
    { $saved ->
        [yes] The meter reports "{ $reported }", saved as your device: reconnect to read it as a { $named }.
       *[no] The meter reports "{ $reported }": reconnect to read it as a { $named }.
    }

# The recording stopped because Buffer size, in Settings, was lowered below
# what it held. $count is how many samples it took, $kept the same number
# written short, e.g. 1.2K. Export… is a button.
connection-buffer-shrunk =
    { $count ->
        [one] Recording stopped — its { $kept } sample is kept, "Export…" saves it
       *[other] Recording stopped — its { $kept } samples are kept, "Export…" saves them
    }
# The recording filled the buffer. $count is its size in samples, $kept the
# same number written short, e.g. 500K.
connection-buffer-full =
    { $count ->
        [one] Recording stopped — buffer full ({ $kept } sample)
       *[other] Recording stopped — buffer full ({ $kept } samples)
    }

# A button command sent to the meter failed. $command is its name as the
# meter's protocol knows it, e.g. hold; $error says why.
connection-command-failed = Command '{ $command }' failed: { $error }
# Switching a meter setting failed. $setting is mode, range or the
# setting's lowercase name as the command line takes it (hold, rel, minmax,
# peak); $error says why.
connection-switch-failed =
    { $setting ->
        [mode] Mode switch failed: { $error }
        [range] Range switch failed: { $error }
       *[other] { $setting } switch failed: { $error }
    }

# The × that closes a toast or a notice.
connection-dismiss =
    .hover = Dismiss
    .a11y = Dismiss notification
