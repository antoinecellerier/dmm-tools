### What 'dmm-cli get' and 'dmm-cli set' print. Setting names (mode, range,
### hold, rel, minmax, peak) and the choices a listing shows, with what to
### type beside each, are what the user types back into 'dmm-cli set': they
### stay as they are, as does the JSON output. HOLD, REL and MIN/MAX are the
### meter's button names, printed on the meter, and stay too.

## The line heading a listing on the standard output; a line per setting or
## per choice follows. $meter is the meter's brand and model, e.g. UNI-T
## UT61E+, in bold; $mode what it is measuring, e.g. AC V.

# 'get' with no setting: one row per setting the meter can switch.
settings-all-heading = Ajustes del multímetro { $meter } ({ $mode }):
# One setting's choices. $setting is that setting: mode, range, hold, rel,
# minmax or peak. The modes offered don't depend on the mode, so that
# heading names none.
settings-heading =
    { $setting ->
        [mode] Modos del multímetro { $meter }:
        [range] Rango del multímetro { $meter } en modo { $mode }:
        [hold] HOLD del multímetro { $meter } en modo { $mode }:
        [rel] REL del multímetro { $meter } en modo { $mode }:
        [minmax] MIN/MAX del multímetro { $meter } en modo { $mode }:
       *[peak] Pico del multímetro { $meter } en modo { $mode }:
    }

## Dim, at the end of the range row while the meter autoranges: the range it
## picked, $range, e.g. 22V.

# In one setting's listing, on the Auto row.
settings-auto-now = (ahora { $range })
# In the listing of every setting.
settings-auto-in = (rango automático, ahora { $range })

## Dim, under a listing: a switch to try. $command is the command line that
## makes it, e.g. dmm-cli set mode hz.

# Under the listing of every setting.
settings-tip-by-name = Sugerencia: para cambiar un ajuste, indica su nombre, p. ej. { $command }
# Under one setting's listing, whose right column holds what to type for
# each choice.
settings-tip-right-column = Sugerencia: escribe el valor de la columna derecha para cambiar a esa opción, p. ej. { $command }

## After "Note:" (common.ftl): the meter has nothing to switch to from where
## it is, and the command stops. Only the dial changes which modes and ranges
## are offered.

# $meter is the meter's brand and model, e.g. UNI-T UT61E+; $mode what it is
# measuring, e.g. NCV. $setting is the setting asked about: mode, range,
# hold, rel, minmax or peak, or all for every setting.
settings-nothing-to-switch =
    { $setting ->
        [mode] El multímetro { $meter } no permite cambiar de modo desde el modo { $mode }: usa el selector giratorio.
        [range] El multímetro { $meter } no permite cambiar de rango en modo { $mode }: usa el selector giratorio.
        [hold] El multímetro { $meter } no permite cambiar HOLD en modo { $mode }.
        [rel] El multímetro { $meter } no permite cambiar REL en modo { $mode }.
        [minmax] El multímetro { $meter } no permite cambiar MIN/MAX en modo { $mode }.
        [peak] El multímetro { $meter } no permite cambiar el modo pico en modo { $mode }.
       *[all] El multímetro { $meter } no permite cambiar ningún ajuste en modo { $mode }: usa el selector giratorio.
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
       *[mode] El multímetro está ahora en modo { $choice }
        [range-auto] El multímetro está ahora en rango automático ({ $range })
        [range] El multímetro está ahora en el rango { $choice } (rango manual)
        [hold] El multímetro tiene ahora HOLD { $choice }
        [rel] El multímetro tiene ahora REL { $choice }
        [minmax] El multímetro está ahora en { $choice }
        [minmax-off] El multímetro salió de MIN/MAX
        [peak] El multímetro está ahora en { $choice }
        [peak-off] El multímetro salió del modo pico
    }
# The meter was on it already, so nothing was sent.
settings-already =
    { $case ->
       *[mode] El multímetro ya está en modo { $choice }
        [range-auto] El multímetro ya está en rango automático ({ $range })
        [range] El multímetro ya está en el rango { $choice } (rango manual)
        [hold] El multímetro ya tiene HOLD { $choice }
        [rel] El multímetro ya tiene REL { $choice }
        [minmax] El multímetro ya está en { $choice }
        [minmax-off] El multímetro ya está fuera de MIN/MAX
        [peak] El multímetro ya está en { $choice }
        [peak-off] El multímetro ya está fuera del modo pico
    }

## Why 'set' failed, after "Error:" (common.ftl). $setting is the setting as
## typed: mode, range, hold, rel, minmax or peak.

# $input is the choice as typed, e.g. 2; $choices the choices it matches, as
# the listing spells them, separated by commas, e.g. "2.2V, 22V, 220V".
settings-ambiguous = valor ambiguo para { $setting }: { $input } coincide con { $choices }
# No choice matches $input, the choice as typed; the listing is printed above.
settings-unknown = valor desconocido para { $setting }: { $input }
