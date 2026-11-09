### What 'dmm-cli get' and 'dmm-cli set' print. Setting names (mode, range,
### hold, rel, minmax, peak) and the choices a listing shows, with what to
### type beside each, are what the user types back into 'dmm-cli set': they
### stay as they are, as does the JSON output. HOLD, REL and MIN/MAX are the
### meter's button names, printed on the meter, and stay too.

## The line heading a listing on the standard output; a line per setting or
## per choice follows. $meter is the meter's brand and model, e.g. UNI-T
## UT61E+, in bold; $mode what it is measuring, e.g. AC V.

# 'get' with no setting: one row per setting the meter can switch.
settings-all-heading = Réglages du multimètre { $meter } ({ $mode }) :
# One setting's choices. $setting is that setting: mode, range, hold, rel,
# minmax or peak. The modes offered don't depend on the mode, so that
# heading names none.
settings-heading =
    { $setting ->
        [mode] Modes du multimètre { $meter } :
        [range] Plage de mesure du multimètre { $meter } en mode { $mode } :
        [hold] HOLD du multimètre { $meter } en mode { $mode } :
        [rel] REL du multimètre { $meter } en mode { $mode } :
        [minmax] MIN/MAX du multimètre { $meter } en mode { $mode } :
       *[peak] Crête du multimètre { $meter } en mode { $mode } :
    }

## Dim, at the end of the range row while the meter autoranges: the range it
## picked, $range, e.g. 22V.

# In one setting's listing, on the Auto row.
settings-auto-now = (actuellement { $range })
# In the listing of every setting.
settings-auto-in = (plage automatique : { $range })

## Dim, under a listing: a switch to try. $command is the command line that
## makes it, e.g. dmm-cli set mode hz.

# Under the listing of every setting.
settings-tip-by-name = Conseil : changez un réglage par son nom, par exemple { $command }
# Under one setting's listing, whose right column holds what to type for
# each choice.
settings-tip-right-column = Conseil : tapez la valeur de la colonne de droite pour y passer, par exemple { $command }

## After "Note:" (common.ftl): the meter has nothing to switch to from where
## it is, and the command stops. Only the dial changes which modes and ranges
## are offered.

# $meter is the meter's brand and model, e.g. UNI-T UT61E+; $mode what it is
# measuring, e.g. NCV. $setting is the setting asked about: mode, range,
# hold, rel, minmax or peak, or all for every setting.
settings-nothing-to-switch =
    { $setting ->
        [mode] Le multimètre { $meter } n’a aucun mode commutable à distance depuis le mode { $mode } : utilisez le commutateur rotatif.
        [range] Le multimètre { $meter } n’a aucune plage commutable à distance en mode { $mode } : utilisez le commutateur rotatif.
        [hold] Le multimètre { $meter } n’a pas de fonction HOLD commutable à distance en mode { $mode }.
        [rel] Le multimètre { $meter } n’a pas de fonction REL commutable à distance en mode { $mode }.
        [minmax] Le multimètre { $meter } n’a pas de fonction MIN/MAX commutable à distance en mode { $mode }.
        [peak] Le multimètre { $meter } n’a pas de fonction crête commutable à distance en mode { $mode }.
       *[all] Le multimètre { $meter } n’a aucun réglage commutable à distance en mode { $mode } : utilisez le commutateur rotatif.
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
       *[mode] Le multimètre est maintenant en mode { $choice }
        [range-auto] Le multimètre est maintenant en plage automatique ({ $range })
        [range] Le multimètre est maintenant sur la plage { $choice } (plage manuelle)
        [hold] HOLD est maintenant { $choice } sur le multimètre
        [rel] REL est maintenant { $choice } sur le multimètre
        [minmax] Le multimètre est maintenant en { $choice }
        [minmax-off] Le multimètre est maintenant hors MIN/MAX
        [peak] Le multimètre est maintenant en { $choice }
        [peak-off] Le multimètre est maintenant hors fonction crête
    }
# The meter was on it already, so nothing was sent.
settings-already =
    { $case ->
       *[mode] Le multimètre est déjà en mode { $choice }
        [range-auto] Le multimètre est déjà en plage automatique ({ $range })
        [range] Le multimètre est déjà sur la plage { $choice } (plage manuelle)
        [hold] HOLD est déjà { $choice } sur le multimètre
        [rel] REL est déjà { $choice } sur le multimètre
        [minmax] Le multimètre est déjà en { $choice }
        [minmax-off] Le multimètre est déjà hors MIN/MAX
        [peak] Le multimètre est déjà en { $choice }
        [peak-off] Le multimètre est déjà hors fonction crête
    }

## Why 'set' failed, after "Error:" (common.ftl). $setting is the setting as
## typed: mode, range, hold, rel, minmax or peak.

# $input is the choice as typed, e.g. 2; $choices the choices it matches, as
# the listing spells them, separated by commas, e.g. "2.2V, 22V, 220V".
settings-ambiguous = choix ambigu pour { $setting } : { $input } correspond à { $choices }
# No choice matches $input, the choice as typed; the listing is printed above.
settings-unknown = choix inconnu pour { $setting } : { $input }
# The meter answered that it wouldn't switch. $choice is the choice, e.g.
# 22V; $reason the meter's reason (lib-errors.ftl). A range is also refused
# when the input is past its end.
settings-refused =
    { $setting ->
        [range] le multimètre a refusé { $choice } ({ $reason }) : vérifiez la position du commutateur et assurez-vous que l’entrée reste dans la plage
       *[other] le multimètre a refusé { $choice } ({ $reason }) : vérifiez la position du commutateur
    }
# The meter took the command, but its readings never showed the choice.
settings-not-switched =
    { $setting ->
        [range] Le multimètre n’a pas changé : vérifiez la position du commutateur et assurez-vous que l’entrée reste dans la plage
       *[other] Le multimètre n’a pas changé : vérifiez la position du commutateur
    }
# As settings-not-switched, naming $live, the choice the meter is still on,
# e.g. off.
settings-not-switched-still =
    { $setting ->
        [range] Le multimètre n’a pas changé (toujours { $live }) : vérifiez la position du commutateur et assurez-vous que l’entrée reste dans la plage
       *[other] Le multimètre n’a pas changé (toujours { $live }) : vérifiez la position du commutateur
    }
