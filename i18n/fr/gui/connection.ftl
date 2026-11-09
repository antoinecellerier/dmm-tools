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
connection-detecting = Détection du multimètre…
# A recording being played back reached its last reading.
connection-ended = La relecture est terminée
# A stretch of a recording being played back that has no readings in it.
connection-no-frames = Aucune trame à cet endroit du fichier de relecture
# The meter has gone quiet for a moment. $dots is one to four dots, padded
# with spaces to four characters, that grow while the wait lasts: keep it
# at the end, with nothing after it.
connection-waiting = En attente du multimètre{ $dots }
# Under the waiting title, when a meter is picked in Settings.
connection-waiting-check-selection = Vérifiez que le bon appareil est sélectionné dans Paramètres (⚙), ou choisissez-y Détection automatique
# Under the waiting title, when Settings is on Auto-detect.
connection-waiting-switch-on = Activez la transmission des données du multimètre, ou choisissez son modèle dans Paramètres (⚙)

## A failed connection: a title naming the problem, the steps to try, and a
## closing line.

# The closing line under the steps. "Connect" is the top bar's button.
connection-click-connect = Cliquez sur « Connecter » après avoir résolu le problème.
# The closing line after the Bluetooth steps when nothing answered at the
# address given with --adapter, a command-line option kept as it is. "It"
# is the scan command the steps above name.
connection-restart-with-address = Relancez avec --adapter réglé sur l’adresse que cette commande affiche.

# A meter that connects over Bluetooth only, and Bluetooth was not looked at.
connection-bluetooth-not-searched = Recherche Bluetooth non effectuée
# Under that title when Bluetooth is switched off. $error is the sentence
# saying why it was not searched; $hint says how to switch it
# on.
connection-bluetooth-not-searched-off = { $error }. { $hint }
# Under that title otherwise. $error is the sentence saying why.
connection-bluetooth-not-searched-why = { $error }.

# --adapter, a command-line option, named a USB device that isn't there.
connection-adapter-not-found = Adaptateur introuvable
# $selector is the value given with --adapter.
connection-adapter-not-matched = Aucun appareil ne correspond à --adapter '{ $selector }'.
# After the list of what is plugged in.
connection-adapter-restart = Relancez avec la bonne valeur pour --adapter.

# The cable or Bluetooth link is up but no meter answered. $link is usb (a
# USB cable), adapter (a Bluetooth adapter plugged into the meter) or radio
# (the meter's own Bluetooth). A list of meters and their steps follows the
# colon.
connection-not-identified =
    { $link ->
        [adapter] L’adaptateur Bluetooth est connecté, mais aucun multimètre ne s’est identifié.
        [radio] La liaison Bluetooth est établie, mais aucun multimètre ne s’est identifié.
       *[usb] Le câble USB est connecté, mais aucun multimètre ne s’est identifié.
    }

    Activez la transmission des données du multimètre, ou choisissez son modèle dans Paramètres (⚙) :
# After that list.
connection-not-identified-report = Le multimètre transmet déjà mais n’est toujours pas reconnu ? Choisissez son modèle dans Paramètres (⚙) et merci de nous le signaler.

# The meter stopped answering. $link is usb (a USB cable), adapter (a
# Bluetooth adapter plugged into the meter), radio (the meter's own
# Bluetooth) or none (a simulated meter, on no link). $model is the meter
# picked, e.g. UNI-T UT61E+; $steps are its own steps for switching data
# transmission on, several lines.
connection-no-response =
    { $link ->
        [usb]
            Le câble USB est connecté, mais le multimètre
            ne répond pas ({ $model } sélectionné).
        [adapter]
            L’adaptateur Bluetooth est connecté, mais le multimètre
            ne répond pas ({ $model } sélectionné).
        [radio]
            La liaison Bluetooth est établie, mais le multimètre
            ne répond pas ({ $model } sélectionné).
       *[none]
            La liaison est établie, mais le multimètre
            ne répond pas ({ $model } sélectionné).
    }

    Si ce n’est pas le bon appareil, changez-le dans Paramètres (⚙), ou choisissez-y Détection automatique.
    Sinon, activez la transmission des données :
    { $steps }
# The same, on Auto-detect: no meter is picked to name.
connection-no-response-auto =
    Activez la transmission des données du multimètre,
    ou choisissez son modèle dans Paramètres (⚙).

# The meter answered, but with data that could not be read: a corrupt
# frame, or a mode this tool does not know. The error itself follows.
connection-bad-data = Le multimètre a envoyé des données illisibles
# A toast once connected, when other cables were there to open and the session
# took the first, with --adapter a command-line option kept as it is. $count is
# 2 or more.
connection-cables-at-open =
    { $count ->
        [one] { $count } câble USB utilisable par cette session est branché : il a été ouvert. Utilisez --adapter pour en choisir un autre.
       *[other] { $count } câbles USB utilisables par cette session sont branchés : le premier trouvé a été ouvert. Utilisez --adapter pour en choisir un autre.
    }
# Follows connection-cables-at-open when nothing tells those cables apart.
connection-cables-no-reconnect = Rien ne permet de distinguer ces câbles : sauf si --adapter indique le chemin d’un câble, une coupure de la liaison mettra fin à cette session.
# A dropped link that was not reconnected. shared.ftl's several-cables
# follows, then connection-several-cables-action. "Connect" is the top bar's
# button.
connection-several-cables = Liaison perdue
connection-several-cables-action = Ne laissez branché que le câble de ce multimètre, puis cliquez sur Connecter.
# A toast, when that notice ends a running recording. What it holds can still
# be exported.
connection-several-cables-recording = Enregistrement arrêté : la liaison a été coupée et le câble USB de ce multimètre ne se distingue pas des autres
# Any other failure to connect or to keep reading. The error itself
# follows, then connection-click-connect.
connection-failed = Échec de la connexion
# A meter that stopped answering in the middle of a session. $title is the
# shared notice title, "No response from meter".
connection-no-response-check = { $title } : vérifiez l’appareil sélectionné et la transmission des données
# The reading thread stopped by itself: it crashed, or gave up connecting.
connection-acquisition-stopped = L’acquisition s’est arrêtée de façon inattendue : reconnectez le multimètre pour reprendre

# The last line of a notice for a meter whose support isn't fully verified;
# a link. $warning is the sentence saying how far it is verified.
connection-feedback = { $warning } Envoyer des commentaires.
    .hover = Ouvrir le suivi des tickets GitHub pour envoyer vos commentaires

## The notice under the reading when the Bluetooth link delivers readings
## two at a time, which can end with a command that fixes it.

# Copies the command to the clipboard.
connection-late-copy = Copier
# A toast, once the command is copied.
connection-late-copied = Commande copiée

## Toasts.

# A meter identified itself while Settings was on Auto-detect. $meter is
# its name, e.g. UNI-T UT61E+. $saved is yes when Settings now names it, no
# for a session that detects every time.
connection-detected =
    { $saved ->
        [yes] Multimètre { $meter } détecté et enregistré comme votre appareil. Choisissez Détection automatique dans Paramètres pour relancer la détection.
       *[no] Multimètre { $meter } détecté
    }
# The same, when the name the meter gave for itself differs; $meter and
# $saved as there. $reported is that name, e.g. UT216XD.
connection-detected-reports =
    { $saved ->
        [yes] Multimètre { $meter } détecté (il indique « { $reported } ») et enregistré comme votre appareil. Choisissez Détection automatique dans Paramètres pour relancer la détection.
       *[no] Multimètre { $meter } détecté (il indique « { $reported } »)
    }
# The meter named another model than the one picked in Settings. $reported
# is the name it gave, e.g. UT161E; $picked the model picked, $named the
# model that name belongs to, e.g. UNI-T UT161E.
connection-other-model = Le multimètre indique « { $reported } », et non le modèle { $picked } sélectionné. Choisissez { $named } ou Détection automatique dans Paramètres.
# Detection settled on a meter, then the meter named another. $reported is
# the name it gave, $named the model it belongs to. $saved is yes when
# Settings now names that model.
connection-named-after-detection =
    { $saved ->
        [yes] Le multimètre indique « { $reported } » et ce modèle est enregistré comme votre appareil : reconnectez-le pour le lire en tant que { $named }.
       *[no] Le multimètre indique « { $reported } » : reconnectez-le pour le lire en tant que { $named }.
    }

# The recording stopped because Buffer size, in Settings, was lowered below
# what it held. $count is how many samples it took, $kept the same number
# written short, e.g. 1.2K. Export… is a button.
connection-buffer-shrunk =
    { $count ->
        [one] Enregistrement arrêté : { $kept } échantillon conservé, à sauvegarder avec « Exporter… »
       *[other] Enregistrement arrêté : { $kept } échantillons conservés, à sauvegarder avec « Exporter… »
    }
# The recording filled the buffer. $count is its size in samples, $kept the
# same number written short, e.g. 500K.
connection-buffer-full =
    { $count ->
        [one] Enregistrement arrêté : tampon plein ({ $kept } échantillon)
       *[other] Enregistrement arrêté : tampon plein ({ $kept } échantillons)
    }

# A button command sent to the meter failed. $command is its name as the
# meter's protocol knows it, e.g. hold; $error says why.
connection-command-failed = Échec de la commande '{ $command }' : { $error }
# Switching a meter setting failed. $setting is mode, range or the
# setting's lowercase name as the command line takes it (hold, rel, minmax,
# peak); $error says why.
connection-switch-failed =
    { $setting ->
        [mode] Échec du changement de mode : { $error }
        [range] Échec du changement de plage : { $error }
       *[other] Échec du changement de { $setting } : { $error }
    }

# The × that closes a toast or a notice.
connection-dismiss =
    .hover = Masquer
    .a11y = Masquer la notification
