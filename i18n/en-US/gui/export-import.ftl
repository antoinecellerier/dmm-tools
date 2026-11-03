### Import… and Export…, the first and last buttons of the row under the
### graph (assets/gui-wide-layout.webp): Import… opens a file saved
### earlier, the Export… split button saves the samples to a file, and its
### arrow drops a menu of the file types. Then the line under the row
### after an import, the prompt shown before something would discard
### samples or markers no file holds, the file types the system's open and
### save dialogs list, and the messages both buttons show when they are
### done.
###
### A sample is one reading the meter sent; a reading, in the import
### messages, is one row of the file. The buttons sit in one row with
### Record and Discard, so keep them short. CSV, JSON and replay are the
### file types; a replay file is one dmm-gui plays back as if the meter were
### attached. $keys is a keyboard shortcut as this computer writes it, e.g.
### Ctrl+E or ⌘E.

## The row of buttons under the graph.

# The first button of the row.
import-button = Import…
    .hover = Open an exported CSV, JSON or replay file ({ $keys })
# The left part of the Export… split button: saves a CSV file at once.
# $buffer is recording when a recording holds the samples, graph when
# nothing is recorded and Export… saves the samples the graph holds.
export-button = Export…
    .hover =
        { $buffer ->
            [recording] Save the recording as a CSV file ({ $keys })
           *[graph] Save the samples from the graph as a CSV file ({ $keys })
        }
# The arrow at the right of the Export… split button, which drops the menu
# of file types. It has no text. $buffer is as for Export….
export-menu =
    .hover =
        { $buffer ->
            [recording] Save the recording as a CSV, JSON or replay file
           *[graph] Save the samples from the graph as a CSV, JSON or replay file
        }
    .a11y = Export file type

## The menu under the Export… arrow: one entry per file type. Each opens
## the save dialog for that type.

# CSV is the file type's name.
export-csv = CSV…
# JSON is the file type's name. dmm-cli read --format json is a command
# line, kept as it is.
export-json = JSON…
    .hover = One object per line, as dmm-cli read --format json prints
# A replay file. dmm-gui --replay is a command line, kept as it is; "as the
# meter" means as if it were the meter attached.
export-replay = Replay…
    .hover = A file dmm-gui --replay plays back as the meter
# The Replay… entry's hover while it is greyed out, and the error shown if
# a replay is asked for anyway. The mock is the simulated meter the app
# offers for trying it out; its samples hold none of the bytes a real meter
# sends (its wire format), which a replay file is made of.
export-no-wire-format = The mock has no wire format to export; connect a real meter

## A small line under the buttons after Import…: the samples are the
## file's.

# $count is how many readings the file gave; $file the file's name, e.g.
# measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv. Export… is the button.
import-hint =
    { $count ->
        [one] Imported { $count } reading from { $file }. "Export…" saves it with your markers and view.
       *[other] Imported { $count } readings from { $file }. "Export…" saves them with your markers and view.
    }

## The prompt shown over the window before something would discard samples
## or markers no file holds: Record over a recording that was never
## exported, Discard on a recording whose samples have left the graph,
## Import… or Connect while the session holds them. Markers are the
## numbered flags the user puts on readings, with a note each; "unsaved
## changes to a marker" means the marker, or its note, was added or changed
## since the last export.

# The prompt's title when samples would be lost (and maybe markers too).
export-unsaved-heading-samples = Discard unexported samples?
# The prompt's title when only markers would be lost.
export-unsaved-heading-markers = Discard unexported markers?
# The prompt after Record, which starts a new recording in place of the
# stopped one. $lost is samples when only samples would be lost, markers
# when only markers, both when both. $samples is how many samples were
# never exported; $markers how many markers have unsaved changes. Markers
# stay on the graph's readings even though the recording goes.
export-unsaved-record =
    { $lost ->
        [samples]
            { $samples ->
                [one] Starting a new recording will discard { $samples } unexported sample.
               *[other] Starting a new recording will discard { $samples } unexported samples.
            }
        [markers]
            { $markers ->
                [one] Starting a new recording will discard unsaved changes to { $markers } marker. Markers stay on the graph while it holds their readings.
               *[other] Starting a new recording will discard unsaved changes to { $markers } markers. Markers stay on the graph while it holds their readings.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Starting a new recording will discard { $samples } unexported sample and unsaved changes to { $markers } marker. Markers stay on the graph while it holds their readings.
                       *[other] Starting a new recording will discard { $samples } unexported sample and unsaved changes to { $markers } markers. Markers stay on the graph while it holds their readings.
                    }
               *[other]
                    { $markers ->
                        [one] Starting a new recording will discard { $samples } unexported samples and unsaved changes to { $markers } marker. Markers stay on the graph while it holds their readings.
                       *[other] Starting a new recording will discard { $samples } unexported samples and unsaved changes to { $markers } markers. Markers stay on the graph while it holds their readings.
                    }
            }
    }
# The prompt's button that goes ahead: discards, then starts recording.
export-unsaved-record-confirm = Discard and record
# The prompt after Discard, which drops a stopped recording; Export… then
# saves the samples the graph still holds, so only the ones that have
# scrolled out of it are lost. $lost, $samples and $markers are as for
# Record; "them" in the last sentences means those samples.
export-unsaved-discard =
    { $lost ->
        [samples]
            { $samples ->
                [one] Discarding the recording will lose { $samples } sample that has left the graph.
               *[other] Discarding the recording will lose { $samples } samples that have left the graph.
            }
        [markers]
            { $markers ->
                [one] Discarding the recording will lose unsaved changes to { $markers } marker on readings that have left the graph.
               *[other] Discarding the recording will lose unsaved changes to { $markers } markers on readings that have left the graph.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Discarding the recording will lose { $samples } sample that has left the graph, and unsaved changes to { $markers } marker on it.
                       *[other] Discarding the recording will lose { $samples } sample that has left the graph, and unsaved changes to { $markers } markers on it.
                    }
               *[other]
                    { $markers ->
                        [one] Discarding the recording will lose { $samples } samples that have left the graph, and unsaved changes to { $markers } marker on them.
                       *[other] Discarding the recording will lose { $samples } samples that have left the graph, and unsaved changes to { $markers } markers on them.
                    }
            }
    }
# The prompt's button that goes ahead: drops the recording.
export-unsaved-discard-confirm = Discard recording
# The prompt after picking a file to import, which replaces everything the
# session holds. $lost, $samples and $markers are as for Record.
export-unsaved-import =
    { $lost ->
        [samples]
            { $samples ->
                [one] Importing replaces this session and will discard { $samples } unexported sample.
               *[other] Importing replaces this session and will discard { $samples } unexported samples.
            }
        [markers]
            { $markers ->
                [one] Importing replaces this session and will discard unsaved changes to { $markers } marker.
               *[other] Importing replaces this session and will discard unsaved changes to { $markers } markers.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Importing replaces this session and will discard { $samples } unexported sample and unsaved changes to { $markers } marker.
                       *[other] Importing replaces this session and will discard { $samples } unexported sample and unsaved changes to { $markers } markers.
                    }
               *[other]
                    { $markers ->
                        [one] Importing replaces this session and will discard { $samples } unexported samples and unsaved changes to { $markers } marker.
                       *[other] Importing replaces this session and will discard { $samples } unexported samples and unsaved changes to { $markers } markers.
                    }
            }
    }
# The prompt's button that goes ahead: discards, then imports the file.
export-unsaved-import-confirm = Discard and import
# The prompt after Connect while an imported file is open: the meter's
# session takes its place. $lost, $samples and $markers are as for Record.
export-unsaved-connect =
    { $lost ->
        [samples]
            { $samples ->
                [one] Connecting ends the imported session and will discard { $samples } unexported sample.
               *[other] Connecting ends the imported session and will discard { $samples } unexported samples.
            }
        [markers]
            { $markers ->
                [one] Connecting ends the imported session and will discard unsaved changes to { $markers } marker.
               *[other] Connecting ends the imported session and will discard unsaved changes to { $markers } markers.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Connecting ends the imported session and will discard { $samples } unexported sample and unsaved changes to { $markers } marker.
                       *[other] Connecting ends the imported session and will discard { $samples } unexported sample and unsaved changes to { $markers } markers.
                    }
               *[other]
                    { $markers ->
                        [one] Connecting ends the imported session and will discard { $samples } unexported samples and unsaved changes to { $markers } marker.
                       *[other] Connecting ends the imported session and will discard { $samples } unexported samples and unsaved changes to { $markers } markers.
                    }
            }
    }
# The prompt's button that goes ahead: discards, then connects the meter.
export-unsaved-connect-confirm = Discard and connect
# The prompt's other button, which keeps everything; it has the focus.
export-unsaved-cancel = Cancel

## The file type the system's open and save dialogs list, next to the
## file's extension (.csv, .json, .replay).

# Import…'s open dialog: every type it reads.
import-file-type = Exported readings
# Export…'s save dialog after CSV…; CSV is the file type's name.
export-file-type-csv = CSV
# After JSON…; JSON is the file type's name.
export-file-type-json = JSON
# After Replay…: a replay file.
export-file-type-replay = Replay

## Messages that show for a few seconds at the top right.

# While a large file is read in. $file is the file's name; $percent how
# much of it is in so far, a whole number from 0 to 99.
import-progress = Importing { $file }… { $percent }%
# When the file is in. $count is how many readings it gave; $file is its
# name.
import-done =
    { $count ->
        [one] Imported { $count } reading from { $file }
       *[other] Imported { $count } readings from { $file }
    }
# When the file holds more readings than the Buffer size setting (in
# Settings) lets the app keep: the first ones are in, the rest are not.
# $count is how many came in; $left how many were left out; $file is the
# file's name.
import-done-partial =
    { $count ->
        [one]
            { $left ->
                [one] Imported the first { $count } reading from { $file }; { $left } reading past the Buffer size in Settings was left out
               *[other] Imported the first { $count } reading from { $file }; { $left } readings past the Buffer size in Settings were left out
            }
       *[other]
            { $left ->
                [one] Imported the first { $count } readings from { $file }; { $left } reading past the Buffer size in Settings was left out
               *[other] Imported the first { $count } readings from { $file }; { $left } readings past the Buffer size in Settings were left out
            }
    }
# The file was read but has no readings in it. $file is its name.
import-empty = { $file } holds no readings
# The file could not be read. $path is where it is; $reason why, in the
# words of the file reader or the system, often in English.
import-failed = Import failed: { $path }: { $reason }
# Reading the file stopped on an internal error. $path is where it is.
import-failed-crashed = Import failed: { $path }: the import failed
# The file dialog of Import… failed.
import-dialog-failed = Import failed
# After Export… saved the file. $count is how many samples it holds; $file
# its name, e.g. measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv.
export-done =
    { $count ->
        [one] Exported { $count } sample to { $file }
       *[other] Exported { $count } samples to { $file }
    }
# After Export… with nothing to save: the recording is running or stopped
# but empty.
export-nothing-recorded = Nothing to export — the recording has no samples yet
# After Export… with nothing to save and no meter connected.
export-nothing-connect = Nothing to export — connect a meter first
# After Export… with nothing to save: a meter is connected but has sent
# nothing yet.
export-nothing-yet = Nothing to export — no readings yet
# After Export… while an earlier export still waits on its save dialog or
# is being written.
export-in-progress = An export is already in progress — finish it first
# The file could not be written. $reason is why, in the words of the
# system or the file writer, often in English.
export-failed = Export failed: { $reason }
# The samples to export were dropped while the save dialog was open, so
# the file would not hold what was asked for.
export-failed-changed = Export failed: the samples changed while the save dialog was open
