### What 'dmm-cli get' and 'dmm-cli set' print. Setting names (mode, range,
### hold, rel, minmax, peak) and the choices a listing shows, with what to
### type beside each, are what the user types back into 'dmm-cli set': they
### stay as they are, as does the JSON output. HOLD, REL and MIN/MAX are the
### meter's button names, printed on the meter, and stay too.

## The line heading a listing on the standard output; a line per setting or
## per choice follows. $meter is the meter's brand and model, e.g. UNI-T
## UT61E+, in bold; $mode what it is measuring, e.g. AC V.

# 'get' with no setting: one row per setting the meter can switch.
settings-all-heading = Settings for { $meter } ({ $mode }):
# One setting's choices. $setting is that setting: mode, range, hold, rel,
# minmax or peak. The modes offered don't depend on the mode, so that
# heading names none.
settings-heading =
    { $setting ->
        [mode] Modes for { $meter }:
        [range] Range for { $meter } in { $mode }:
        [hold] HOLD for { $meter } in { $mode }:
        [rel] REL for { $meter } in { $mode }:
        [minmax] MIN/MAX for { $meter } in { $mode }:
       *[peak] Peak for { $meter } in { $mode }:
    }

## Dim, at the end of the range row while the meter autoranges: the range it
## picked, $range, e.g. 22V.

# In one setting's listing, on the Auto row.
settings-auto-now = (now { $range })
# In the listing of every setting.
settings-auto-in = (auto-ranging in { $range })

## Dim, under a listing: a switch to try. $command is the command line that
## makes it, e.g. dmm-cli set mode hz.

# Under the listing of every setting.
settings-tip-by-name = Tip: switch one by name, e.g. { $command }
# Under one setting's listing, whose right column holds what to type for
# each choice.
settings-tip-right-column = Tip: the right column switches to it, e.g. { $command }

## After "Note:" (common.ftl): the meter has nothing to switch to from where
## it is, and the command stops. Only the dial changes which modes and ranges
## are offered.

# $meter is the meter's brand and model, e.g. UNI-T UT61E+; $mode what it is
# measuring, e.g. NCV. $setting is the setting asked about: mode, range,
# hold, rel, minmax or peak, or all for every setting.
settings-nothing-to-switch =
    { $setting ->
        [mode] { $meter } has no switchable modes in { $mode } — use the dial.
        [range] { $meter } has no switchable ranges in { $mode } — use the dial.
        [hold] { $meter } has no switchable HOLD in { $mode }.
        [rel] { $meter } has no switchable REL in { $mode }.
        [minmax] { $meter } has no switchable MIN/MAX in { $mode }.
        [peak] { $meter } has no switchable Peak in { $mode }.
       *[all] { $meter } has no switchable settings in { $mode } — use the dial.
    }

## In green on the standard output, once 'set' sees the meter on the choice.
## $choice is that choice, e.g. AC V Hz, 22V, on or MAX; $range the range
## autoranging picked, e.g. 22V. $case is the setting and how it stands: mode,
## range-auto (the meter picks the range), range (a range picked by hand),
## hold or rel (with $choice on or off), minmax or peak (a state, e.g. MAX or
## P-MIN), minmax-off or peak-off (the meter left that state).

# The meter switched.
settings-switched =
    { $case ->
       *[mode] Meter now in { $choice }
        [range-auto] Meter now auto-ranging ({ $range })
        [range] Meter now in { $choice } (manual range)
        [hold] Meter now HOLD { $choice }
        [rel] Meter now REL { $choice }
        [minmax] Meter now in { $choice }
        [minmax-off] Meter now out of MIN/MAX
        [peak] Meter now in { $choice }
        [peak-off] Meter now out of Peak
    }
# The meter was on it already, so nothing was sent.
settings-already =
    { $case ->
       *[mode] Meter is already in { $choice }
        [range-auto] Meter is already auto-ranging ({ $range })
        [range] Meter is already in { $choice } (manual range)
        [hold] Meter is already HOLD { $choice }
        [rel] Meter is already REL { $choice }
        [minmax] Meter is already in { $choice }
        [minmax-off] Meter is already out of MIN/MAX
        [peak] Meter is already in { $choice }
        [peak-off] Meter is already out of Peak
    }

## Why 'set' failed, after "Error:" (common.ftl). $setting is the setting as
## typed: mode, range, hold, rel, minmax or peak.

# $input is the choice as typed, e.g. 2; $choices the choices it matches, as
# the listing spells them, separated by commas, e.g. "2.2V, 22V, 220V".
settings-ambiguous = ambiguous { $setting }: { $input } matches { $choices }
# No choice matches $input, the choice as typed; the listing is printed above.
settings-unknown = unknown { $setting }: { $input }
# The meter answered that it wouldn't switch. $choice is the choice, e.g.
# 22V; $reason the meter's reason (lib-errors.ftl); $remedy what to check, a
# sentence of its own (shared.ftl's switch-check-dial).
settings-refused-check = the meter refused { $choice }: { $reason }. { $remedy }
# The same when the reason says what to do itself, e.g. "leave MIN/MAX
# first": $choice and $reason as above.
settings-refused-because = the meter refused { $choice }: { $reason }
# The meter refused because it is another model than the one chosen: $choice
# and $reason as above; $remedy says what to pick, a sentence of its own
# (shared.ftl's choose-meter-cli).
settings-refused-other-model = the meter refused { $choice }: { $reason }. { $remedy }
# The meter took the command, but its readings never showed the choice.
# $remedy is what to check, as above.
settings-not-taken = Meter did not switch. { $remedy }
# As settings-not-taken, naming $live, the choice the meter is still on,
# e.g. off; $remedy as there.
settings-not-taken-still = Meter did not switch (still { $live }). { $remedy }
