### The top bar: the meter's name and the connection buttons on the left,
### the connection status in the middle, and the version, Help & Feedback,
### shortcuts and settings buttons on the right
### (assets/gui-wide-layout.webp).
###
### The bar is one row on a wide window: keep these short. $keys is a
### keyboard shortcut as this computer writes it, e.g. Ctrl+O or ⌘O.

## The meter's name, at the left end.

# Shown in place of a meter's name while Auto-detect hasn't found one yet.
top-bar-auto-detect = Détection auto
# Shown in place of a meter's name for an imported file that names none.
top-bar-imported-file = Fichier importé

## Connection buttons.

top-bar-connect = Connecter
    .hover = Ouvrir la connexion USB au multimètre sélectionné ({ $keys })
# In place of Connect while the meter is being opened: stops the attempt.
top-bar-cancel = Annuler
    .hover = Interrompre la tentative de connexion ({ $keys })
top-bar-disconnect = Déconnecter
    .hover = Fermer la connexion au multimètre ({ $keys })
# A button; ▶ and ⏸ are part of it.
top-bar-resume = ▶ Reprendre
    .hover = Reprendre l’acquisition ({ $keys })
top-bar-pause = ⏸ Pause
    .hover = Suspendre l’acquisition : arrête l’enregistrement et la mise à jour du graphique ({ $keys })
# Clears the graph and the statistics, not the recording.
top-bar-clear = Effacer
    .hover = Effacer l’historique du graphique et les statistiques ({ $keys })
# While the connection is being retried after it dropped.
top-bar-reconnecting = Reconnexion…
# $attempt counts the retries, from 1.
top-bar-reconnecting-attempt = Reconnexion (tentative { $attempt })…
top-bar-reconnecting-hover = Reconnexion automatique en cours : cliquez sur Déconnecter pour arrêter
# $error is the last attempt's error, in the language of the system that raised it.
top-bar-reconnecting-hover-error =
    Reconnexion automatique en cours : cliquez sur Déconnecter pour arrêter.
    Dernière erreur : { $error }
# After shared.ftl's waiting-for-own-cable. "Disconnect" and "Connect" are the
# top bar's button.
top-bar-own-cable-action = Pour utiliser l’autre câble, cliquez sur Déconnecter, puis sur Connecter.
top-bar-stop-retrying = Déconnecter
    .hover = Arrêter les tentatives et fermer la connexion ({ $keys })

## The status beside the coloured dot. $name is the meter's name, e.g.
## UT61E+. $link is how it is connected (USB cable, Bluetooth). $paused is
## yes or no.

# The meter's name when it didn't report one.
top-bar-connected = Connecté
top-bar-status =
    { $paused ->
        [yes] { $name } (en pause)
       *[no] { $name }
    }
top-bar-status-link =
    { $paused ->
        [yes] { $name } · { $link } (en pause)
       *[no] { $name } · { $link }
    }
# An imported file: $name is its meter, $file its file name.
top-bar-status-file = { $name } · { $file }
top-bar-disconnected = Déconnecté
# While the picked meter is being opened.
top-bar-connecting = Connexion…
# While Auto-detect asks the connected meter what it is.
top-bar-detecting = Détection…

## The status's hover: how the meter is connected, spelled out.
## $link is usb (a USB cable), adapter (a Bluetooth adapter plugged into the
## meter), radio (the meter's own Bluetooth) or none.

top-bar-link-live =
    { $link ->
        [usb] Connecté par le câble USB
        [adapter] Connecté par l’adaptateur Bluetooth
        [radio] Connecté par la liaison Bluetooth
       *[none] Multimètre simulé, sans liaison
    }
# A recording being played back: how it was recorded.
top-bar-link-recorded =
    { $link ->
        [usb] Enregistré via le câble USB
        [adapter] Enregistré via l’adaptateur Bluetooth
        [radio] Enregistré via la liaison Bluetooth
       *[none] Enregistré via une liaison non précisée
    }
# $path is the file's full path.
top-bar-replaying = Relecture de { $path }
# $readings is how many readings the file held; $path is the file's full
# path.
top-bar-imported-from =
    { $readings ->
        [one] { $readings } mesure importée de { $path }
       *[other] { $readings } mesures importées de { $path }
    }

## The support badge, after the status, for a meter not yet fully verified.
## A link to report feedback. Capitals, as the meter's own annunciators.

top-bar-badge-verified = VÉRIFIÉ
top-bar-badge-partly-verified = PARTIELLEMENT VÉRIFIÉ
top-bar-badge-experimental = EXPÉRIMENTAL
# $warning is the sentence saying how far the meter's support is verified.
top-bar-badge-hover = { $warning } Cliquez pour envoyer vos commentaires.

## The right end: the version, which opens What's New; Help & Feedback; the
## shortcut list; Settings.

top-bar-version =
    .hover = Afficher les nouveautés : notes de publication de cette version
    .a11y = Afficher les notes de publication
# The version's hover when the bar has no room for the update link.
# $version is the newer release, e.g. v0.10.0.
top-bar-version-update =
    .hover =
        Afficher les nouveautés : notes de publication de cette version
        { $version } est disponible : élargissez la fenêtre pour afficher son lien
    .a11y = Afficher les notes de publication. { $version } est disponible
# The update link; $version is the newer release, e.g. v0.10.0.
top-bar-update =
    .hover = Ouvrir la page de publication de { $version } sur GitHub
    .a11y = { $version } disponible : ouvrir sa page de publication
top-bar-project-link =
    .hover = Ouvrir la page du projet dmm-tools sur GitHub
# The ? button. $keys and $other are its two keys, ? and F1.
top-bar-shortcuts =
    .hover = Afficher les raccourcis clavier et les gestes de la souris ({ $keys } ou { $other })
    .a11y = Raccourcis clavier et gestes de la souris
# The ⚙ button.
top-bar-settings =
    .hover = Afficher ou masquer le panneau des paramètres
    .a11y = Paramètres
