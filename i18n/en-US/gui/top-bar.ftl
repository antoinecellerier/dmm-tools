### The top bar: the meter's name and the connection buttons on the left,
### the connection status in the middle, and the version, Help & Feedback,
### shortcuts and settings buttons on the right
### (docs/images/gui-main.png).
###
### The bar is one row on a wide window: keep these short. $keys is a
### keyboard shortcut as this computer writes it, e.g. Ctrl+O or ⌘O.

## The meter's name, at the left end.

# Shown in place of a meter's name while Auto-detect hasn't found one yet.
top-bar-auto-detect = Auto-detect
# Shown in place of a meter's name for an imported file that names none.
top-bar-imported-file = Imported file

## Connection buttons.

top-bar-connect = Connect
    .hover = Open USB connection to the selected meter ({ $keys })
top-bar-disconnect = Disconnect
    .hover = Close the active meter connection ({ $keys })
# A button; ▶ and ⏸ are part of it.
top-bar-resume = ▶ Resume
    .hover = Resume acquisition ({ $keys })
top-bar-pause = ⏸ Pause
    .hover = Halt acquisition — stops recording and graph updates ({ $keys })
# Clears the graph and the statistics, not the recording.
top-bar-clear = Clear
    .hover = Clear graph history and statistics ({ $keys })
# While the connection is being retried after it dropped.
top-bar-reconnecting = Reconnecting...
# $attempt counts the retries, from 1.
top-bar-reconnecting-attempt = Reconnecting (attempt { $attempt })...
top-bar-reconnecting-hover = Retrying the connection automatically — click Disconnect to stop
# $error is the last attempt's error, in the language of the system that raised it.
top-bar-reconnecting-hover-error =
    Retrying the connection automatically — click Disconnect to stop.
    Last error: { $error }
top-bar-stop-retrying = Disconnect
    .hover = Stop retrying and close the connection ({ $keys })

## The status beside the coloured dot. $name is the meter's name, e.g.
## UT61E+. $link is how it is connected (USB cable, Bluetooth). $paused is
## yes or no.

# The meter's name when it didn't report one.
top-bar-connected = Connected
top-bar-status =
    { $paused ->
        [yes] { $name } (paused)
       *[no] { $name }
    }
top-bar-status-link =
    { $paused ->
        [yes] { $name } · { $link } (paused)
       *[no] { $name } · { $link }
    }
# An imported file: $name is its meter, $file its file name.
top-bar-status-file = { $name } · { $file }
top-bar-disconnected = Disconnected

## The status's hover: how the meter is connected, spelled out.
## $link is usb (a USB cable), adapter (a Bluetooth adapter plugged into the
## meter), radio (the meter's own Bluetooth) or none.

top-bar-link-live =
    { $link ->
        [usb] Connected over the USB cable
        [adapter] Connected over the Bluetooth adapter
        [radio] Connected over the Bluetooth link
       *[none] Mock meter, no link
    }
# A recording being played back: how it was recorded.
top-bar-link-recorded =
    { $link ->
        [usb] Recorded over the USB cable
        [adapter] Recorded over the Bluetooth adapter
        [radio] Recorded over the Bluetooth link
       *[none] Recorded over an unnamed link
    }
# $path is the file's full path.
top-bar-replaying = Replaying { $path }
# $readings is how many readings the file held; $path is the file's full
# path.
top-bar-imported-from =
    { $readings ->
        [one] { $readings } reading imported from { $path }
       *[other] { $readings } readings imported from { $path }
    }

## The support badge, after the status, for a meter not yet fully verified.
## A link to report feedback. Capitals, as the meter's own annunciators.

top-bar-badge-verified = VERIFIED
top-bar-badge-partly-verified = PARTLY VERIFIED
top-bar-badge-experimental = EXPERIMENTAL
# $warning is the sentence saying how far the meter's support is verified.
top-bar-badge-hover = { $warning } Click to report feedback.

## The right end: the version, which opens What's New; Help & Feedback; the
## shortcut list; Settings.

top-bar-version =
    .hover = Show What's New — release notes for this version
    .a11y = Show release notes
# The version's hover when the bar has no room for the update link.
# $version is the newer release, e.g. v0.10.0.
top-bar-version-update =
    .hover =
        Show What's New — release notes for this version
        { $version } is available — widen the window for its link
    .a11y = Show release notes. { $version } is available
# The update link; $version is the newer release, e.g. v0.10.0.
top-bar-update =
    .hover = Open the { $version } release page on GitHub
    .a11y = { $version } available — open its release page
top-bar-project-link =
    .hover = Open the dmm-tools project page on GitHub
# The ? button. $keys and $other are its two keys, ? and F1.
top-bar-shortcuts =
    .hover = Show keyboard shortcuts and mouse gestures ({ $keys } or { $other })
    .a11y = Keyboard shortcuts and mouse gestures
# The ⚙ button.
top-bar-settings =
    .hover = Show or hide the settings panel
    .a11y = Settings
