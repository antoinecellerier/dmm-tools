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
error-hid = erreur HID : { $detail }
# The meter sent something that makes no sense. $detail says what, in
# English, for a bug report.
error-invalid-response = réponse non valide : { $detail }
# The meter on the link is not one the meter chosen can read, worded as
# error-invalid-response. $reason is one of the "Another meter" sentences
# below.
error-wrong-meter = réponse non valide : { $reason }
# A frame from the meter failed its check. $expected and $actual are
# hexadecimal numbers, e.g. 0x01a3.
error-checksum-mismatch = somme de contrôle incorrecte : attendu { $expected }, reçu { $actual }
error-timeout = délai d’attente de la réponse dépassé
# The meter reports a mode this tool does not know. $code is a hexadecimal
# number, e.g. 0x1f.
error-unknown-mode = mode inconnu : { $code }
# $reason is one of the "Refused before it was sent" sentences below.
error-unsupported-command = commande non prise en charge : { $reason }
# $reason is one of the "The meter did not do it" sentences below.
error-command-rejected = commande refusée : { $reason }
# $device is the meter's name or ID, as typed with --device or as the
# settings file holds it, sometimes with an explanation in English.
error-unknown-device = appareil inconnu : { $device }
# A meter on a cable was opened as a simulated one. $device is its ID, e.g.
# ut61eplus.
error-not-simulated = appareil inconnu : { $device } n’est pas un multimètre simulé
# A recording could not be played back. $detail names the file or the line
# and says what is wrong with it, in English.
error-replay = fichier de relecture : { $detail }
# $adapter is the value given with --adapter, sometimes with an explanation
# in English.
error-adapter-not-found = adaptateur introuvable : { $adapter }
# What --adapter named, as error-adapter-not-found's $adapter and the GUI's
# connection-adapter-not-matched quote it, when it is a USB device that is
# no supported cable. $path is the device's path, as given with --adapter.
error-not-an-adapter = { $path } (l’appareil existe mais n’est pas un adaptateur USB pris en charge)
# --adapter named a cable or adapter the chosen meter cannot be reached
# through. $link is bluetooth (a Bluetooth adapter) or cable (a USB cable
# whose name is $cable, e.g. BU-86X). $model is the meter, e.g. UNI-T
# UT61E+.
error-wrong-cable =
    { $link ->
        [bluetooth] --adapter désigne un adaptateur Bluetooth, qui ne convient pas au multimètre { $model } : indiquez avec --adapter le câble propre au multimètre, ou retirez --adapter
       *[cable] --adapter désigne un câble { $cable }, qui ne convient pas au multimètre { $model } : indiquez avec --adapter le câble propre au multimètre, ou retirez --adapter
    }
error-link-lost = Liaison Bluetooth perdue
# $reason is one of the "Bluetooth" sentences below.
error-bluetooth = Bluetooth : { $reason }
# A cable or Bluetooth link is up but no meter answered on it. $link is usb
# (a USB cable), adapter (a Bluetooth adapter plugged into the meter) or
# radio (the meter's own Bluetooth).
error-device-not-identified =
    { $link ->
        [adapter] aucun multimètre n’a répondu via l’adaptateur Bluetooth
        [radio] aucun multimètre n’a répondu via la liaison Bluetooth
       *[usb] aucun multimètre n’a répondu via le câble USB
    }
# No cable was found. $bluetooth is searched when Bluetooth was searched
# too, else not-searched.
error-no-transport-found =
    { $bluetooth ->
        [searched] aucun adaptateur USB pris en charge trouvé, ni aucun appareil Bluetooth à portée
       *[not-searched] aucun adaptateur USB pris en charge trouvé
    }
# As error-no-transport-found, naming the cables looked for; $bluetooth as
# there. $cables lists them with their USB IDs, e.g. "CP2110 0x10c4:0xea80,
# BU-86X 0x0820:0x0001".
error-no-transport-found-looked-for =
    { $bluetooth ->
        [searched] aucun adaptateur USB pris en charge trouvé (câbles recherchés : { $cables }), ni aucun appareil Bluetooth à portée
       *[not-searched] aucun adaptateur USB pris en charge trouvé (câbles recherchés : { $cables })
    }
# A meter with Bluetooth built in, and none was in range. $model is the
# meter, e.g. UNI-T UT60BT.
error-bluetooth-only-not-in-range = aucun multimètre { $model } trouvé à portée Bluetooth
# A meter with Bluetooth built in, and Bluetooth was not searched. $model
# is the meter, e.g. UNI-T UT60BT. Each app adds how to switch it on.
error-bluetooth-only-switched-off = le multimètre { $model } se connecte uniquement en Bluetooth, et le Bluetooth est désactivé
# $model is the meter, e.g. UNI-T UT60BT.
error-bluetooth-only-not-built = le multimètre { $model } se connecte uniquement en Bluetooth, et cette version n’intègre pas la prise en charge du Bluetooth
# $model is the meter, e.g. UNI-T UT60BT.
error-bluetooth-only-usb-adapter = le multimètre { $model } se connecte uniquement en Bluetooth, et --adapter désigne un appareil USB : indiquez l’adresse Bluetooth du multimètre, ou retirez --adapter

## Refused before it was sent: the meter, the link or the dial position
## cannot do it. After "unsupported command:". $setting is a setting's
## name as the CLI takes it (mode, range, rel, minmax…); $word, $target and
## $from_word are a mode's hexadecimal code, e.g. 0x3111.

# $setting is a setting's name.
error-unsupported-setting = le réglage { $setting } ne peut pas être modifié sur ce multimètre
# $mode is a mode.
error-unsupported-several-positions = le mode { $mode } est signalé depuis plusieurs positions du commutateur : faites d’abord une mesure dans un autre mode
# $mode is a mode.
error-unsupported-no-position = le mode { $mode } ne correspond à aucune position connue du commutateur
# $target and $from are modes.
error-unsupported-mode-unreachable = le mode { $target } n’est pas accessible depuis le mode { $from } : tournez d’abord le commutateur
# $target and $from_word are mode codes, $from is the mode $from_word is.
error-unsupported-word-unreachable = le mode { $target } n’est pas accessible depuis le mode { $from } ({ $from_word }) : tournez d’abord le commutateur
# $mode is a mode.
error-unsupported-no-ranges = le mode { $mode } n’a aucune plage à choisir sur ce multimètre
# $id is a range's number, $count how many ranges $mode, a mode, has.
error-unsupported-no-such-range = la plage { $id } ne fait pas partie des { $count } plages que propose le mode { $mode }
# $id is a range's number, $count how many ranges $mode, a mode, has.
error-unsupported-range-not-listed = la plage { $id } ne fait pas partie des { $count } plages que propose le mode { $mode }
# $count is how many ranges $mode, a mode, has.
error-unsupported-auto-range-not-listed = la plage automatique ne fait pas partie des { $count } plages que propose le mode { $mode }
# $mode is a mode, $word its code.
error-unsupported-fixed-range = plage en mode { $mode } ({ $word }) : mode à plage fixe
# $setting is a setting's name, $mode a mode.
error-unsupported-setting-not-in-mode = le réglage { $setting } ne peut pas être modifié en mode { $mode } sur ce multimètre
# $setting is a setting's name, $mode a mode, $word its code.
error-unsupported-setting-not-in-word = le réglage { $setting } ne peut pas être modifié en mode { $mode } ({ $word }) sur ce multimètre
# $setting is a setting's name, $id a state's number, $mode a mode and
# $offered the states it offers, separated by commas (on, off, MAX…).
error-unsupported-no-such-state = le réglage { $setting } n’a pas d’état { $id } : le mode { $mode } propose { $offered }
# $command is a command word, $mode a Peak mode, $word its code.
error-unsupported-refused-in-peak = { $command } en mode { $mode } ({ $word }) : le multimètre le refuse en mode crête
# "rel" is the command word. $mode is a mode, $word its code.
error-unsupported-no-rel = rel en mode { $mode } ({ $word })
# $model is the meter's part number, e.g. VC-880; $button a button's name.
error-unsupported-no-button = le multimètre { $model } n’a pas de touche { $button }
# $cable is a cable's name, e.g. BU-86X; $len and $got are byte counts.
error-unsupported-request-length = le câble { $cable } accepte des requêtes de { $len } octets, pas de { $got } octets
# $link is usb (the USB cable), adapter (the Bluetooth adapter) or none (a
# link that does not say what it is); $baud a serial speed, e.g. 19200.
error-unsupported-baud =
    { $link ->
        [usb] le câble USB ne peut pas régler son débit sur { $baud } bauds
        [adapter] l’adaptateur Bluetooth ne peut pas régler son débit sur { $baud } bauds
       *[none] la liaison ne peut pas régler son débit sur { $baud } bauds
    }
# A developer's error: the simulated meter has no bytes to decode.
error-unsupported-no-wire-format = parse_payload : le multimètre simulé n’a pas de trames brutes

## The meter did not do it: a command it was sent did not take, or the
## meter's state says it cannot. After "command rejected:". $button is a
## button's name; $state, $target and $now are modes, ranges or setting
## states.

# $button is a button's name, $state what the meter stayed in.
error-rejected-did-nothing = la touche { $button } n’a eu aucun effet (état : { $state })
# $target is what was asked for, $now what the meter is in.
error-rejected-never-appeared = l’état demandé ({ $target }) n’est jamais apparu : le multimètre est revenu sur { $now }
# $presses is how many times $button was pressed; $state is what the meter
# is in.
error-rejected-gave-up =
    { $presses ->
        [one] abandon après { $presses } appui sur la touche { $button } : le multimètre est sur { $state }
       *[other] abandon après { $presses } appuis sur la touche { $button } : le multimètre est sur { $state }
    }
# $mode is a mode.
error-rejected-mode-changed = le multimètre est passé en mode { $mode } : arrêt des appuis sur RANGE
# $reason is another sentence of this group, $mode the mode the meter was
# in before.
error-rejected-started-in = { $reason } (mode de départ : { $mode })
# $range is a range.
error-rejected-auto-did-nothing = la touche AUTO n’a eu aucun effet : le multimètre est toujours sur la plage { $range }
# $button is a button's name, $state a setting's state.
error-rejected-exit-did-nothing = EXIT { $button } n’a eu aucun effet : le multimètre est toujours sur { $state }
# $setting is a setting's name, $target and $now its states.
error-rejected-switch-did-nothing = { $setting } { $target } n’a eu aucun effet : le multimètre est toujours sur { $now }
# $sent is what was sent: a command word, or one of the error-sent messages
# below, or a setting's name and a state.
error-rejected-by-meter = le multimètre a refusé { $sent }
error-rejected-zero-needs-capacitance = ZERO ne fonctionne qu’en mode capacité
# $code is a number, $model and $opened are meters (e.g. OWON B41T+), $id a
# meter's ID as --device takes it. "Auto-detect" is the GUI's Device
# choice.
error-rejected-other-model = le multimètre indique le code de modèle OWON { $code } ({ $model }) et n’est donc pas le multimètre { $opened } choisi à l’ouverture, si bien que les touches à distance sont désactivées : choisissez Détection automatique ou --device { $id }
# $code is a number.
error-rejected-unknown-model = le multimètre indique le code de modèle OWON { $code }, que cet outil ne connaît pas encore, donc les touches à distance sont désactivées
# $command is a command word.
error-rejected-model-unread = le code de modèle du multimètre n’a pas été lu, donc la commande { $command } est désactivée : maintenez la touche REL enfoncée sur le multimètre pour quitter REL
error-rejected-min-max-locks-range = MIN/MAX verrouille la plage : quittez MIN/MAX avant de la changer
# $command is a command word or a setting's name.
error-rejected-replay = { $command } : une relecture n’accepte aucune commande

# What a refused command switched to, as error-rejected-by-meter names it.
# $mode is a mode.
error-sent-mode = le mode { $mode }
# $range is a range.
error-sent-range = la plage { $range }

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
error-wrong-meter-other-frame = le multimètre indique le code de modèle OWON { $code } ({ $model }), qui envoie la trame OWON de { $frame } octets, et n’est donc pas le multimètre { $opened } choisi à l’ouverture : choisissez Détection automatique ou --device { $id }
# An OWON meter sends the other kind of data frame and names no model.
# $sent and $expected are frame lengths in bytes, 6 or 15; $opened is the
# meter chosen, e.g. Voltcraft VC871. $remedy is auto-detect, or own-model
# when the user has to pick the meter's model themselves.
error-wrong-meter-other-frame-kind =
    { $remedy ->
        [auto-detect] le multimètre envoie des trames OWON de { $sent } octets, et non les trames de { $expected } octets du multimètre { $opened } choisi à l’ouverture : choisissez Détection automatique
       *[own-model] le multimètre envoie des trames OWON de { $sent } octets, et non les trames de { $expected } octets du multimètre { $opened } choisi à l’ouverture : choisissez le modèle exact du multimètre
    }
# An OWON model this tool does not read yet. $code is OWON's model number,
# e.g. 83. "This message" is the whole error, for the issue.
error-wrong-meter-not-yet = le multimètre indique le code de modèle OWON { $code }, que cet outil ne lit pas encore : ouvrez un ticket (issue) avec ce message
# An OWON model whose readings carry their sign where this tool does not
# read it yet. $code is OWON's model number, e.g. 55.
error-wrong-meter-sign-in-function-word = le multimètre indique le code de modèle OWON { $code } : il envoie la trame OWON de 6 octets avec le signe dans le mot de fonction, que cet outil ne lit pas encore
# A Brymen meter of another series answered. $found and $opened are series,
# e.g. Brymen BM82x and Brymen BM86x; $id is the --device value for $found.
error-wrong-meter-other-series = le multimètre branché sur le câble est un multimètre { $found }, et non un multimètre { $opened } : choisissez Détection automatique ou --device { $id }
# An OWON meter chosen by its model sends bytes, but never its 6-byte data
# frame. B35 and B35T are OWON models.
error-wrong-meter-no-six-byte-frames = le multimètre n’envoie aucune trame OWON de 6 octets : il s’agit peut-être d’un ancien B35 ou B35T, dont cet outil ne lit pas encore les trames de 14 octets
# The same for a meter whose data frame is 15 bytes long.
error-wrong-meter-no-fifteen-byte-frames = le multimètre n’envoie aucune trame OWON de 15 octets : choisissez Détection automatique
# An OWON meter that names no model sends frames of neither length. "This
# message" is the whole error, for the issue.
error-wrong-meter-untold-frame-kind = ni le code de modèle du multimètre ni ses trames n’indiquent s’il envoie des trames OWON de 6 ou de 15 octets : ouvrez un ticket (issue) avec ce message

## Bluetooth: what the Bluetooth stack could not do. After "Bluetooth:".

# $detail is what the operating system said.
error-bluetooth-runtime = impossible de démarrer l’environnement d’exécution Bluetooth : { $detail }
error-bluetooth-no-adapter = non disponible sur cet ordinateur
error-bluetooth-powered-off = désactivé sur cet ordinateur
# $role is write (the characteristic commands are written to) or notify
# (the one the meter's data arrives on).
error-bluetooth-no-characteristic =
    { $role ->
        [write] l’appareil Bluetooth n’a aucune caractéristique d’écriture de données reconnue : ce n’est pas un adaptateur ou un multimètre pris en charge, ou ses services n’ont jamais été résolus
       *[notify] l’appareil Bluetooth n’a aucune caractéristique de notification de données reconnue : ce n’est pas un adaptateur ou un multimètre pris en charge, ou ses services n’ont jamais été résolus
    }
# A Brymen meter refused the password this tool sends.
error-bluetooth-password-refused = le multimètre a refusé le mot de passe de connexion 0000 : pour le réinitialiser, { error-bluetooth-reset-gesture }
# How a Brymen BM788BT/BM787BT's password and name go back to the factory
# settings. The meter's activation steps in lib.ftl (activation-bm78xbt)
# end with the same sentence: word them alike. "Org" is what the display
# shows.
error-bluetooth-reset-gesture = maintenez la touche Hz enfoncée en tournant le commutateur de OFF à la position capacité en moins de 0.6 s : le multimètre affiche « Org », et son mot de passe de connexion et son nom Bluetooth reviennent aux réglages d’usine
# A Brymen meter refused the login with error $code, a number.
error-bluetooth-login-refused =
    { $code ->
        [0] le multimètre a refusé l’authentification : erreur de somme de contrôle (code { $code })
        [1] le multimètre a refusé l’authentification : identifiant de canal non valide (code { $code })
        [2] le multimètre a refusé l’authentification : hors de la plage de réglage (code { $code })
        [5] le multimètre a refusé l’authentification : arguments non valides (code { $code })
        [6] le multimètre a refusé l’authentification : autorisations insuffisantes (code { $code })
        [one] le multimètre a refusé l’authentification : erreur inconnue (code { $code })
       *[other] le multimètre a refusé l’authentification : erreur inconnue (code { $code })
    }

## Not an error: the notice under the reading in the GUI when a Bluetooth
## adapter hands over a reading late, together with the next one.

late-readings-title = Certaines mesures Bluetooth arrivent en retard
# $fix is command when a command follows that fixes it for this connection
# (it ends in a colon, and the command shows on the next line as it is), or
# none.
late-readings-advice =
    { $fix ->
        [command] Sur cet adaptateur, une mesure arrive parfois groupée avec la suivante. Aucune n’est perdue. Pour des horodatages exacts jusqu’à la déconnexion du multimètre, lancez :
       *[none] Sur cet adaptateur, une mesure arrive parfois groupée avec la suivante. Aucune n’est perdue.
    }
