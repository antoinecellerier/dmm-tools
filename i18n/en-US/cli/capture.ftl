### What 'dmm-cli capture' prints around its steps: the guided run a user
### makes to send a report about their meter. It walks the meter through a
### list of steps (set the dial to DC V, press HOLD…), reads what the meter
### sends at each, and writes a report file to attach to an issue on GitHub.
### These lines go to the terminal's error stream.
###
### Each step's own instruction, the readings (5.5687 V [AUTO]) and the
### report file stay as they are, in English: a maintainer reads the report.
### A user may translate the instructions for themselves: see
### --export-instructions below.
### Step ids (dcv, hold_on, extra_0), setting names (range, hold) and words
### in --flag form are typed or read back by those names and stay as they
### are. The keys the user presses to answer (s, q, r, n, y) arrive as
### variables: the run only understands those letters, so they can't be
### changed. "Enter" is the key's name and can be translated.

## Checking that the meter answers, before anything else.

# Dim, while the tool asks the meter for its name.
capture-checking = Checking meter communication...
# Yellow and bold. The cable or Bluetooth link opened, but the meter sent
# nothing. $link is usb (a USB cable), adapter (a Bluetooth adapter plugged
# into the meter), radio (the meter's own Bluetooth) or other.
capture-not-responding =
    { $link ->
        [usb] The USB cable is connected but the meter isn't responding.
        [adapter] The Bluetooth adapter is connected but the meter isn't responding.
        [radio] The Bluetooth link is connected but the meter isn't responding.
       *[other] The link is connected but the meter isn't responding.
    }
# Followed by the meter's own steps for switching its data transmission on.
capture-enable-transmission = To enable data transmission:
capture-run-again = Then run this command again.
# What the meter sent was kept in a file of its own, as evidence. $bytes is
# how many bytes the meter sent, $path the file.
capture-no-response-saved = Bytes received: { $bytes }, saved to { $path }
# $url is the web address of the meter's issue on GitHub.
capture-no-response-attach = If the meter is on and still not answering, attach that file to { $url }
# $error is why the file could not be written.
capture-no-response-not-saved = Could not save the bytes received: { $error }

## The banner once the meter answers, after the lines naming the meter
## (connect.ftl). Each support level is what follows "Status:"; the line
## under the partly verified and experimental ones is indented under it.

# $status is one of the capture-status-* levels below, coloured.
capture-status = Status: { $status }
capture-status-verified = supported model
capture-status-partly-verified = partly verified — connection and main modes confirmed
capture-status-partly-verified-detail = The modes still unconfirmed are what this run is for.
# The protocol was worked out from the meter's traffic, not from its maker.
capture-status-experimental = experimental — the protocol is reverse-engineered, not confirmed
capture-status-experimental-detail = Please complete as many steps as possible and share the report.

## The meter names another model than the one the run opened (a CMS061
## opened as a CMS101): asked before the report file is touched, so the
## operator can run again with the right meter. $named and $opened are
## meter names; $id is the --device id that opens the named one.

# Yellow and bold, after the status banner.
capture-other-model = The meter reports itself as the { $named }, not the { $opened } this run opened.
capture-other-model-detail = Its steps and remote keys would be the { $opened }'s.
# $yes and $no are the keys that answer: y and n. Any other key quits.
capture-other-model-confirm = Capture it as the { $opened } anyway? { $yes }/{ $no }:
capture-other-model-rerun = Run the capture again with --device { $id }.

## A report from an earlier run is already in the file: resume it, start
## again or stop. Each prompt waits for one key, after a space.

# $path is the report file. $captured and $skipped count its steps.
capture-found-existing = Found existing capture: { $path } ({ $captured } captured, { $skipped } skipped)
# $resume, $fresh and $abort are the keys that answer: r, n and q.
capture-resume-prompt = { $resume }=resume, { $fresh }=start fresh, { $abort }=abort:
capture-aborted = Aborted.
# $yes and $no are the keys that answer: y and n.
capture-overwrite-confirm = This will overwrite the existing capture. Are you sure? { $yes }/{ $no }:
capture-resuming = Resuming — already-captured steps will be skipped.
# The file is there but isn't a report this tool can read. $path is the
# file.
capture-unreadable = Found { $path } but couldn't parse it.
# $yes is the key that answers: y.
capture-overwrite-unreadable = Overwrite? { $yes }=start fresh, any other key=abort:
# After "Warning:", at the start and the end of a run whose output goes
# through a pipe or into a file.
capture-not-a-terminal = capture's output is piped or redirected, so it can't wait for you at the meter, have you confirm the readings or check auto-detection. Run it directly in a terminal for a valid report.
# $path is the report file the run writes.
capture-output-file = Output file: { $path }

## What the steps ask for besides the meter and its test leads, listed
## before the first step so the user can gather it all at once. Each item is
## on a line of its own after its number in brackets.

capture-needs-heading = You will need:
# $item is one of the capture-need-* items below; $steps lists the ids of
# the steps that need it, e.g. "dcv_short, ohm_short".
capture-needs-item = { $item }  (steps: { $steps })
# The user types the numbers in brackets of what they don't have, or just
# presses Enter.
capture-needs-prompt = Numbers of anything you don't have (Enter = have everything):
# What the user typed at a numbers prompt (this one or the review's) isn't
# one of the numbers listed. $token is what they typed, in quotes; $max the
# last number listed.
capture-not-a-number = { $token } is not a number between 1 and { $max }
capture-need-shorted-leads = shorted test leads
capture-need-dc-source = battery or other DC source
capture-need-thermocouple = K-type thermocouple
# For the non-contact voltage detector, which senses a live wire nearby.
capture-need-live-wire = live mains wire nearby
# The meter's power-measurement adapter, plugged into a live outlet with a
# load plugged into it.
capture-need-power-adapter = power adapter, live outlet and load
capture-need-transistor = transistor
capture-need-scr = SCR (thyristor)
capture-need-loop-source = 4-20 mA loop and its 15-48 V supply
capture-need-power-module = DC or USB power measurement module, supply and load
# LX-925 is the adapter's name.
capture-need-pv-adapter = LX-925 irradiance adapter

## The steps. Each opens with its id in brackets and its instruction, and
## the tool watches the meter until it shows what the step asks for.

# Bold, between heavy lines.
capture-modes-heading = Measurement Modes
# Dim, under the heading. $skip and $quit are the keys: s and q.
capture-modes-keys = each step captures itself once the meter settles — Enter=capture now, { $skip }=skip one, { $quit }=skip to end and save
# Dim, after a step's instruction: the step the rest of the run relies on.
capture-gate-mark = (gate)
# A resumed run passes over a step the report already has. $step is the
# step's id.
capture-already-captured = { $step } already captured, skipping
# Dim, once the tool has waited a while. $skip and $quit are the keys: s and
# q.
capture-press-enter = Press Enter when the meter is ready ({ $skip }=skip, { $quit }=finish)
# Dim, on a single line the run keeps rewriting while it waits for the meter.
# $reading is the latest reading (5.5687 V [AUTO]); $frame counts the meter's
# replies so far.
capture-progress-watching = watching the meter · { $reading } · frame { $frame }
# Dim, on that same line while a step's readings are taken: $done of $total
# ("reading 4 of 12"), and $reading the latest one.
capture-progress-reading = reading { $done } of { $total } · { $reading }
# Dim: a step that presses a button on the meter by remote control holds it
# until the meter shows the state the instruction asks for. $skip and $quit
# are the keys: s and q.
capture-wait-start = Set the meter up as above: the key goes out once it is (Enter=send now, { $skip }=skip, { $quit }=finish)
# Dim: the meter left the state the step starts from just before the key
# went out, so the step waits for it again.
capture-wait-moved = The meter moved on before the key went out: waiting for it again
# When the terminal can't be read key by key. $skip and $quit are the keys:
# s and q.
capture-any-key = any key=capture, { $skip }=skip, { $quit }=finish:
# Red: the button the step presses on the meter by remote control was
# refused. $error is why.
capture-command-failed = Command failed: { $error }
# Yellow: the tool pressed a button on the meter by remote control and the
# meter didn't change. $command is the button's name as the tool sends it,
# e.g. hold; $reading what the meter shows, e.g. 5.5687 V [AUTO].
capture-did-nothing-shows = { $command } did nothing; the meter still shows { $reading }
# Yellow: the tool pressed a button on the meter by remote control and the
# meter changed, but not to what the step waits for. $command is the button's
# name as the tool sends it, e.g. hold; $reading what the meter shows, e.g.
# 0.00 VA [HOLD].
capture-command-missed = { $command } did not bring the state this step waits for; the meter now shows { $reading }
# $command is the button's name as the tool sends it, e.g. hold.
capture-did-nothing-silent = { $command } did nothing; the meter sent no reading
# Yellow: the meter changed function while the step was reading it, so the
# step starts over. $mode is the function the step is for, $other the one
# the meter went to, e.g. Ω and Continuity.
capture-left-mode = the meter left { $mode } for { $other } while sampling — retaking…
# The reading the step took, in green. $reading is it, e.g. 5.5687 V [AUTO].
capture-we-read = We read: { $reading }
# Something the meter sent couldn't be read; the step goes on. $error is
# why.
capture-frame-error = error: { $error }
# Yellow: the step read nothing from the meter.
capture-no-response = No response from meter.
# Yellow, before capture-few-samples on the same line.
capture-few-samples-label = warning:
# The step got fewer readings than it asks for. $got and $wanted count them.
capture-few-samples = only got { $got }/{ $wanted } samples

## Dim, while a step waits: the meter settled, but not in what the step
## expects. Each says what the reading has and what the step wants.

# $got and $want are functions, in quotes, e.g. "DC V" and "AC V".
capture-mismatch-mode = meter shows: mode is { $got }, want { $want }
# $flag is a status flag the meter shows, e.g. HOLD. $want says whether the
# step wants it on or off; the meter has it the other way.
capture-mismatch-flag =
    { $want ->
        [on] meter shows: { $flag } is off, want on
       *[off] meter shows: { $flag } is on, want off
    }
# $range is the reading's range, e.g. 22V. $shown is none (the reading has
# no range), auto (a range the meter picked itself) or plain. $want is auto
# (the step wants the meter to pick the range itself) or manual (the user
# picks it).
capture-mismatch-range =
    { $want ->
        [auto]
            { $shown ->
                [none] meter shows: range is (none), want auto
                [auto] meter shows: range is { $range } (auto), want auto
               *[plain] meter shows: range is { $range }, want auto
            }
       *[manual]
            { $shown ->
                [none] meter shows: range is (none), want manual
                [auto] meter shows: range is { $range } (auto), want manual
               *[plain] meter shows: range is { $range }, want manual
            }
    }
# $value is what the meter shows, e.g. 12.345 or OL; $shown is absent when
# the reading has no main value, else value. $want is the kind of reading the
# step wants: overload (OL, the meter's over-range sign), negative, finite
# (any number), ncv (the non-contact voltage detector's level 1 or more),
# word (a word the meter shows instead of a number) or zero.
capture-mismatch-value =
    meter shows: value is { $shown ->
        [absent] no main reading
       *[value] { $value }
    }, want { $want ->
        [overload] OL
        [negative] a negative reading
        [ncv] an NCV level of 1 or more
        [word] a word instead of a reading
        [zero] a reading of zero
       *[finite] a numeric reading
    }
# $value and $shown as in capture-mismatch-value, $value with its unit. $min
# is the smallest number the step wants, positive or negative, with the
# mode's base unit, e.g. 1 V.
capture-mismatch-at-least =
    meter shows: value is { $shown ->
        [absent] no main reading
       *[value] { $value }
    }, want at least { $min } either way

## Asking whether the reading matches the meter's screen. Each prompt waits
## for a line, after a space.

# Dim. $retake is yes when the step can be done over. $no and $redo are the
# keys that answer: n and r.
capture-confirm =
    { $retake ->
        [yes] Did the meter show this? (Enter = yes, { $no } = no, { $redo } = retake)
       *[no] Did the meter show this? (Enter = yes, { $no } = no)
    }
# Yellow, when the answer was none of those. $retake, $no and $redo as in
# capture-confirm.
capture-confirm-again =
    { $retake ->
        [yes] Answer { $no }, { $redo }, or press Enter.
       *[no] Answer { $no }, or press Enter.
    }
# The reading didn't match: the user types what the meter showed.
capture-what-shown = What did the meter show?
# Dim: the user typed nothing at capture-what-shown.
capture-recorded-mismatch = recorded: the reading did not match
# Dim. $shown is what the user typed, in quotes.
capture-recorded-shown = recorded: the meter showed { $shown }
# Dim: the step starts over.
capture-retaking = retaking…

## After the first steps, which check that the tool reads the meter right
## ("core semantics": mode, range and flags).

# Green.
capture-core-confirmed = Core semantics confirmed — remaining steps are reviewed at the end.
# Yellow. $steps lists the ids of the steps that failed, e.g. "dcv, ohm".
capture-core-not-confirmed = Core semantics not confirmed ({ $steps }) — every step will ask for confirmation.

## Once the core semantics are confirmed, the tool sets ranges and flags on
## the meter itself by remote control, and prints a dim line for each. Each
## starts with an arrow.

# $mode is the function, e.g. Continuity.
capture-drive-switching = switching to { $mode } (sent by the tool)
# $mode is the function, e.g. Continuity; $error why the meter refused.
capture-drive-switch-failed = could not switch to { $mode }: { $error } — do it by hand
# $setting is the setting's name, rel (relative mode): the meter refuses it
# while it shows OL, its over-range sign.
capture-drive-rel-skipped-ol = { $setting } skipped: reading is OL
# $setting is the setting's name, rel (relative mode).
capture-drive-rel-skipped-blank = { $setting } skipped: no reading
# In place of a reading, after the setting and value tried, e.g.
# "range:22V".
capture-drive-no-reading = no reading
# Yellow. $setting is the setting's name, e.g. range or hold.
capture-drive-reset-failed = { $setting } could not be reset — press the meter's button
# Yellow: the meter refused too many of the tool's commands.
capture-drive-unreliable = remote control unreliable on this meter — no more automatic sweeps

## The end-of-run review: the readings the run took without asking, each
## with its number, to check against what the meter showed.

# Bold, between heavy lines.
capture-review-heading = Review the readings
# The user types the numbers of the wrong readings, or just presses Enter.
capture-review-prompt = Numbers of the readings that did NOT match the meter's screen (Enter = all correct):

## The freeform pass: the user sets the meter to anything the steps didn't
## cover and describes it.

# Bold, between heavy lines; "Part 4" is the run's last part.
capture-freeform-heading = Part 4: Additional Captures (optional)
capture-freeform-intro = Set the meter to any mode/state not covered above.
# After the capture's id in brackets, e.g. [extra_0]; waits for a line.
# $quit is the word that ends the pass: q.
capture-freeform-prompt = Describe what you set the meter to (or '{ $quit }' to finish):

## The last check: does the tool's automatic detection recognise the
## meter? The user switches it off and on, and the tool detects it afresh.

# Dim. $bridge is the chip in the cable or the Bluetooth link, e.g. CP2110.
capture-detect-confirmed = Auto-detection is already confirmed over { $bridge }, so its check is skipped.
# Bold, between heavy lines.
capture-detect-heading = Last check: auto-detection
capture-detect-off-cable = Turn the meter off and unplug its USB cable.
capture-detect-off = Turn the meter off.
# Waits for one key. $skip is the key that skips: s.
capture-detect-off-prompt = Press Enter once it is off, or { $skip } to skip:
# $mode is the function to turn the dial to, DC V.
capture-detect-on-cable = Plug the cable back in and turn the meter on at { $mode }.
# $mode is the function to turn the dial to, DC V.
capture-detect-on = Turn it back on at { $mode }.
# Followed by the meter's own steps for switching its data transmission on.
capture-detect-transmission = If the meter switched data transmission off, turn it back on:
# Waits for one key. $skip is the key that skips: s.
capture-detect-ready-prompt = Press Enter when it is ready, or { $skip } to skip:
# Dim.
capture-detect-checking = Checking auto-detection…
# Detection found the meter; the tool reads it once more. $instruction is
# what to do, the step's own instruction, in English.
capture-detect-now = Now: { $instruction }
# Dim. $skip is the key that skips: s.
capture-detect-read-keys = The reading is taken once the meter settles — Enter takes it now, { $skip } skips.

## The line the check ends on: dim when detection found the meter, yellow
## with advice when it didn't. $found is the model detection picked, e.g.
## UNI-T UT61E+; $reading what the meter then read, e.g. 5.5687 V [AUTO].
## The advice names the meter the run was for: $device is its id on the
## command line, e.g. ut61eplus, and $meter its name, e.g. UNI-T UT61E+.

# $reason is why the link didn't open again, in English, or
# capture-detect-no-reason.
capture-detect-not-reopened = Auto-detection was not checked: the link did not reopen ({ $reason }).
capture-detect-no-reason = no reason given
capture-detect-found = Auto-detection found { $found }.
capture-detect-found-read = Auto-detection found { $found } and read { $reading }.
capture-detect-found-read-other = Auto-detection found { $found }, but then read { $reading } — keep using --device { $device }, or pick { $meter } in dmm-gui.
capture-detect-found-read-nothing = Auto-detection found { $found }, but then read nothing — keep using --device { $device }, or pick { $meter } in dmm-gui.
capture-detect-picked-other = Auto-detection picked { $found } for this meter — keep using --device { $device }, or pick { $meter } in dmm-gui.
capture-detect-not-found = Auto-detection did not find this meter — keep using --device { $device }, or pick { $meter } in dmm-gui.

## The end of the run.

# After "Warning:" (common.ftl): the detection check stopped. $error is why.
capture-detect-failed = auto-detection check: { $error }
# Bold and green, between equals signs.
capture-complete = Capture complete!
# $path is the report file, in bold.
capture-report-saved = Report saved to: { $path }
# The run followed a list of steps a maintainer wrote (a "plan"). $plan is
# its file; $covered and $total count its steps.
capture-plan-covered = Plan { $plan }: { $covered } of { $total } steps captured
capture-nothing-new = No unverified step was captured, so there is nothing new to report.
# $covered and $total count the meter's steps that hardware hasn't confirmed
# yet; $meter is its name, e.g. UNI-T UT61E+.
capture-covered = Covered { $covered } of { $total } unverified steps for { $meter }.
# $resume is the key that resumes, r (capture-resume-prompt).
capture-not-finished = Not finished — run the same command again and answer { $resume } to resume.
# $flag is the option as typed, --steps <ids>.
capture-steps-hint = { $flag } runs named steps on their own.
# $url is the web address of the meter's issue on GitHub.
capture-attach = Attach the report to { $url }

## --export-instructions: the step instructions written to a file the user
## translates for themselves, then passes back with --instructions. The
## file is Fluent source, like this one.

# The comment at the top of the exported file, a line of it per line here.
# $meter is the meter's name, e.g. Brymen BM829s/BM827s/BM822s/BM821s.
# Keep "=", "-device" and "#" as they are: they are the file's
# syntax.
capture-instructions-header =
    Capture step instructions for { $meter }, to translate.
    Replace the text after each "=" with your translation, on the same line.
    Keep the step id before the "=", the "-device" line and the lines
    starting with "#" as they are. A step you delete shows in English.
# A line of the header, when the steps quote words from the meter: its
# key labels and what its screen shows. $marks lists them, e.g. "AC, HOLD,
# MIN/MAX, SELECT". A translation, by a person or a machine, must leave them
# as they are, so say so plainly.
capture-instructions-keep = Keep these as they are, as the meter prints them on its keys and screen: { $marks }
# The header's last line. $command is the command that runs the capture
# with the translated file.
capture-instructions-run = Then run: { $command }
# In place of writing a file, for a meter with no steps of its own (a
# simulated one). $meter is its name, e.g. Mock UT61E+.
capture-instructions-none = { $meter } has no capture steps of its own: there is nothing to translate.
# After writing the file. $count is the number of steps; $path the file.
capture-instructions-written =
    { $count ->
        [one] Wrote { $count } step instruction to { $path }.
       *[other] Wrote { $count } step instructions to { $path }.
    }
# $command is the command that runs the capture with the translated file.
capture-instructions-next = Translate them, then run: { $command }
# Yellow, before the first step: steps whose English changed after the
# file was exported, so they show a translation of an older wording.
# $path is the file; $steps lists the step ids, e.g. "dcv, hold".
capture-instructions-changed = These steps changed after { $path } was exported, and show your translation of their old text: { $steps }. Export a new file to see the new English.
# One problem in the file, on a line of its own under
# capture-error-instructions-unreadable. $line is a line number.
capture-instructions-problem-syntax = line { $line }: not a step id, "=" and its text
# Another problem. $id is a step id, e.g. dcv.
capture-instructions-problem-not-text = { $id }: the text holds curly brackets; remove them
# Another problem. $id is a step id, e.g. dcv.
capture-instructions-problem-twice = { $id }: the step is there twice
# Another problem: the file couldn't be read at all.
capture-instructions-problem-unparsable = the file can't be read; check the curly brackets and quotes in it

## The line that closes a failed run, after "Error:" (common.ftl).

capture-error-not-responding = meter not responding
capture-error-input-gone = keyboard input stopped working; finish the capture and rerun
# --steps named steps this meter doesn't have. $steps lists them, e.g.
# "dcmv, tmep"; $command is the command line that lists the meter's steps.
capture-error-unknown-steps =
    unknown capture step(s): { $steps }
    Run { $command } to see the steps this device supports.
# The --instructions file can't be read. $path is the file; $problems the
# problems, one per line (capture-instructions-problem-*).
capture-error-instructions-unreadable =
    { $path } can't be read as capture instructions:
    { $problems }
# The --instructions file was exported for another meter. $path is the
# file; $named the meter id in it, e.g. ut61eplus; $device this run's, e.g.
# bm82x; $command the command that exports a file for this meter.
capture-error-instructions-device = { $path } holds the instructions for { $named }, not { $device }. Export them for this meter with { $command }.
# The --instructions file names steps this run doesn't have, most likely a
# step id that was translated. $path is the file; $ids the ids not found,
# e.g. "vcc, retencao"; $known the ids the run has, e.g. "dcv, acv, hold".
capture-error-instructions-ids = { $path } names steps this run doesn't have: { $ids }. Keep each step id as it was exported; the steps are: { $known }.
