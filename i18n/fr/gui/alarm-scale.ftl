### The Scale and Alarm chips at the end of the meter's buttons under the
### reading (assets/gui-reading-controls.webp), the row of fields each chip
### opens under them, the toasts they raise (the short messages under the
### top bar's right end) and the alarm's limit lines on the graph. No doc
### image shows a row open.
###
### Scale is a software transform: it turns the meter's reading into what
### it stands for, such as a current clamp's 10 mV/A into amps. Nothing is
### sent to the meter. Alarm watches the main reading against a low and a
### high limit, and marks each reading that crosses one.
###
### Numbers and units arrive formatted and stay as they are.

## The chips, at the end of the meter's buttons (HOLD, REL…), which a
## narrow window wraps: keep the names short. Each chip is a body that
## switches its feature on or off, joined to a caret (▸) that opens its row
## of fields; the body is filled while the feature is on.

# The Scale chip's body.
scale-chip = Échelle
    .hover = Activer ou désactiver la mise à l’échelle
# The caret beside Scale: opens the row of fields below.
scale-settings =
    .hover = Régler l’échelle
    .a11y = Paramètres de l’échelle
# The Alarm chip's body.
alarm-chip = Alarme
    .hover = Activer ou désactiver l’alarme
# The caret beside Alarm: opens the row of fields below.
alarm-settings =
    .hover = Régler les limites
    .a11y = Paramètres de l’alarme
# The hover of a greyed-out Scale or Alarm body while the window shows the
# meter alone with nothing typed in its fields yet. With the graph and the
# recording both hidden in Settings, there is no room for the row of
# fields.
alarm-scale-setup-in-settings = Affichez d’abord le graphique ou l’enregistrement dans Paramètres
# The same in the big meter, which has no room for the row. $keys is the
# shortcut that leaves the big meter, e.g. Ctrl+B or ⌘B.
alarm-scale-setup-in-normal-layout = Effectuez d’abord le réglage dans la disposition normale ({ $keys })

## The Scale row, under the chips while Scale's caret is open:
## "× [Scale factor] + [Offset] → [Unit label] [Apply] [Off]". The ×, +
## and → before each field are symbols, not text. The fields are narrow:
## their placeholders are cut short with … when longer than the field.

# Placeholder in the empty field after ×: the number each reading is
# multiplied by.
scale-factor = Facteur
# Placeholder in the empty field after +: the number added after
# multiplying.
scale-offset = Décal.
# Placeholder in the empty field after →: the unit the scaled reading is
# shown in, e.g. A or °F.
scale-unit = Unité
# Applies the three fields to the reading.
scale-apply = Appliquer
    .hover = Appliquer ces valeurs à chaque mesure à partir de maintenant
# Turns scaling off; the fields keep their values for the next Apply.
scale-off = Désactiver
    .hover = Arrêter la mise à l’échelle et réafficher la mesure du multimètre

## The Scale row's toasts.

# A field could not be read; nothing changes. $reason is why, lower-case
# with no full stop, e.g. “abc” is not a number.
scale-invalid-factor = Échelle non valide : { $reason }
# The same for the Offset field, with $reason as there.
scale-invalid-offset = Décalage non valide : { $reason }
# Scaling turned on, or changed. $transform is the scale in force, written
# in symbols and numbers, e.g. ×100 → A or ×2 + 0.5. $also says what else
# happened: recording, a recording is under way and goes on with the scaled
# readings; alarm, the alarm was on and was turned off, since its limits
# were in the old unit; both; or none.
scale-on =
    { $also ->
        [recording] Mise à l’échelle des mesures : { $transform } – l’enregistrement continue avec les valeurs mises à l’échelle
        [alarm] Mise à l’échelle des mesures : { $transform } – alarme désactivée, redéfinissez ses limites dans la nouvelle unité
        [both] Mise à l’échelle des mesures : { $transform } – l’enregistrement continue avec les valeurs mises à l’échelle – alarme désactivée, redéfinissez ses limites dans la nouvelle unité
       *[none] Mise à l’échelle des mesures : { $transform }
    }
# Scaling turned off. $also as in scale-on.
scale-off-toast =
    { $also ->
        [recording] Mise à l’échelle désactivée – l’enregistrement continue avec les valeurs mises à l’échelle
        [alarm] Mise à l’échelle désactivée – alarme désactivée, redéfinissez ses limites dans la nouvelle unité
        [both] Mise à l’échelle désactivée – l’enregistrement continue avec les valeurs mises à l’échelle – alarme désactivée, redéfinissez ses limites dans la nouvelle unité
       *[none] Mise à l’échelle désactivée
    }

## The Alarm row, under the chips while Alarm's caret is open:
## "Low [none] High [none] V ± [auto] [Apply] [Off]". The fields are narrow.

# Caption before the low limit's field.
alarm-low = Basse
# Caption before the high limit's field.
alarm-high = Haute
# Placeholder in an empty Low or High field: that side has no limit.
alarm-no-limit = aucune
# The field after ±, the hysteresis band: how far back inside a limit the
# reading must come before that limit can alarm again. Its placeholder says
# that blank picks the band automatically.
alarm-band = auto
    .hover = Écart de retour exigé avant qu’une limite puisse redéclencher l’alarme : une valeur ou un pourcentage, par exemple 1 %. Vide : automatique
# After the limits, the unit they are in, e.g. V; this is its hover.
alarm-unit =
    .hover = L’unité des limites
# The same before an alarm is on: the unit taken from the reading on
# screen.
alarm-unit-from-reading =
    .hover = L’unité des limites, d’après la mesure affichée
# Shown in place of the unit, in the warning colour, while the reading is
# of another quantity than the limits: the dial was turned. $unit is the
# unit of the readings now arriving, e.g. Ω; on hover, $limits is the unit
# the limits are in, e.g. V. Short: it sits in the row.
alarm-idle = en attente : { $unit }
    .hover = En attente : les mesures sont en { $unit }, les limites en { $limits }
# Turns the alarm on with the fields' limits.
alarm-apply = Appliquer
    .hover = Activer l’alarme avec ces limites
# Turns the alarm off; the fields keep their values for the next Apply.
alarm-off = Désactiver
    .hover = Désactiver l’alarme

## The Alarm row's toasts. $reason is why a value was refused, lower-case
## with no full stop, e.g. “abc” is not a number.

# The Low field could not be read.
alarm-invalid-low = Limite basse non valide : { $reason }
# The High field could not be read.
alarm-invalid-high = Limite haute non valide : { $reason }
# A limit is not a finite number.
alarm-invalid-limit = Limite non valide : { $reason }
# The two limits don't make a pair: the low one is not below the high one.
alarm-invalid-limits = Limites non valides : { $reason }
# The ± field could not be read, or does not fit the limits.
alarm-invalid-band = Hystérésis non valide : { $reason }
# The alarm turned on. $limits is the limits it watches, from
# alarm-limits/alarm-hysteresis in lib-alarm.ftl, e.g. "above 5 or below 3,
# hysteresis 1%".
alarm-on-toast = Alarme activée : { $limits }
# The alarm turned off.
alarm-off-toast = Alarme désactivée

## The graph: a dotted line at each limit, labelled at its right end.

# $side is high (the high limit) or low; $value is the limit with its unit,
# e.g. 3.0000 V.
alarm-limit-line =
    { $side ->
        [low] Basse : { $value }
       *[high] Haute : { $value }
    }
