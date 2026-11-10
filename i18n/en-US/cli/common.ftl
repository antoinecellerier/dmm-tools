### Words every dmm-cli command uses. dmm-cli is a terminal program: its
### lines go to a terminal, or to a file or another program when piped, and
### are read as plain text, with colour where the terminal shows it.

## The word that opens a line saying what went wrong, before the sentence
## that says it. Keep the colon, or your language's equivalent, and
## nothing after it: the sentence follows after a space.

# Red and bold: the command failed.
common-error-label = Error:
# Yellow: the command goes on, but something is off.
common-warning-label = Warning:
# Yellow: something worth knowing about how the command runs.
common-note-label = Note:

## A link detail ('dmm-cli info' and 'dmm-cli debug') the cable or Bluetooth
## device failed to give, in place of the detail after its label.

# $error is why, e.g. "HID error: …".
common-unavailable = unavailable ({ $error })
