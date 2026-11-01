### The settings panel, which the ⚙ button at the top bar's right end opens
### under the bar (assets/gui-settings.webp): rows of checkboxes and of
### chips, each row led by its caption, in groups a rule apart. The colour
### and theme rows have a file of their own.
###
### Rows wrap on a narrow window, so keep the chips short. $keys and the
### other shortcut variables are keyboard shortcuts as this computer writes
### them, e.g. Ctrl+O or ⌘O. Text in parentheses starting with -- names a
### command-line option, kept as it is, that set the row for this session
### only.

## The meter: which one, how the mock runs, and how a connection is made.

# The Device row's caption; .a11y names its dropdown for a screen reader.
settings-device = Appareil :
    .a11y = Appareil
# The Device dropdown, and its first entry, when no particular meter is
# picked: each kind of meter is tried in turn.
settings-auto-detect = Détection automatique
# The Device dropdown while --device picked the meter for this session.
# $name is the meter, e.g. UNI-T UT61E+, or Auto-detect.
settings-device-pinned = { $name } (--device)
# Beside the greyed Device dropdown while a recording is played back:
# --replay is the command-line option that opened it.
settings-device-replay = (relancez sans --replay pour choisir un multimètre)
# Beside the greyed Device dropdown while the session runs on a simulated
# clock, set by command-line options for testing.
settings-device-clock = (relancez sans les options d’horloge pour choisir un multimètre)

## The open Device dropdown: Auto-detect, then every meter under its brand.

# An entry's hover and screen-reader name. $name is the meter, e.g. UNI-T
# UT61E+; $tag how far its support is verified, e.g. partly verified.
settings-device-entry = { $name } ({ $tag })

## The Mock mode row, shown while a simulated meter is picked. Its other
## chips are the simulated modes' names, kept as they are.

# The first chip: the simulated meter steps through every mode. $pinned is
# yes when --mock-mode picked it for this session, no otherwise.
settings-mock-auto =
    { $pinned ->
        [yes] Auto (en boucle) (--mock-mode)
       *[no] Auto (en boucle)
    }
    .hover = Parcourir tous les modes simulés pour tester l’interface
# The chip for the mode --mock-mode picked for this session. $mode is its
# name, e.g. dcv.
settings-mock-mode-pinned = { $mode } (--mock-mode)
settings-mock-mode = Mode simulé :

## How a connection is made: checkboxes on one row.

settings-auto-connect = Se connecter au démarrage
    .hover = Ouvrir automatiquement la connexion USB au lancement de l’application
# Asking the meter its name makes it beep.
settings-device-name = Afficher le nom de l’appareil à la connexion (bip sonore)
    .hover = Demander son nom au multimètre après la connexion : le multimètre émet un bip sonore
# The Bluetooth checkbox while --no-bluetooth switched the search off for
# this session. $setting is the checkbox's own label, e.g. Look for
# Bluetooth devices.
settings-bluetooth-pinned = { $setting } (--no-bluetooth)
settings-bluetooth =
    .hover = Rechercher un adaptateur ou un multimètre Bluetooth à portée quand aucun câble USB ne répond. Prend effet à la prochaine connexion.

## What is kept of the readings.

settings-sample-interval = Intervalle d’échantillonnage :
# The first Sample interval chip; the others are durations, e.g. 300ms or 2s.
settings-interval-every-reading = Chaque mesure
    .hover = Conserver chaque mesure du multimètre, à son propre rythme
# The other Sample interval chips' hover. $unit is second for exactly one
# second, seconds for a whole number of seconds and ms otherwise; $n is how
# many seconds or milliseconds. s and ms are unit symbols.
settings-interval-chip =
    .hover =
        { $unit ->
            [second] Au plus une mesure par seconde : celle la plus proche de chaque top
            [seconds] Au plus une mesure toutes les { $n } s : celle la plus proche de chaque top
           *[ms] Au plus une mesure toutes les { $n } ms : celle la plus proche de chaque top
        }

settings-buffer-size = Taille du tampon :
# A Buffer size chip's hover; the chips are sample counts written short,
# e.g. 500K. $count is the chip's number of samples and $samples the same
# number written short. $memory is what they take, e.g. ≈194 MB, and $span
# how long they last, e.g. 14 h. $pace is every-reading when the Sample
# interval keeps every reading, whose pace the estimate can't know, and
# interval otherwise.
settings-buffer-size-chip =
    .hover =
        { $pace ->
            [every-reading]
                { $count ->
                    [one] Conserver jusqu’à { $samples } échantillon dans le graphique et pour l’export ({ $memory }, environ { $span } à 10 mesures par seconde). Un enregistrement arrêté conservé en plus peut en prendre encore autant
                   *[other] Conserver jusqu’à { $samples } échantillons dans le graphique et pour l’export ({ $memory }, environ { $span } à 10 mesures par seconde). Un enregistrement arrêté conservé en plus peut en prendre encore autant
                }
           *[interval]
                { $count ->
                    [one] Conserver jusqu’à { $samples } échantillon dans le graphique et pour l’export ({ $memory }, environ { $span } à l’intervalle d’échantillonnage actuel). Un enregistrement arrêté conservé en plus peut en prendre encore autant
                   *[other] Conserver jusqu’à { $samples } échantillons dans le graphique et pour l’export ({ $memory }, environ { $span } à l’intervalle d’échantillonnage actuel). Un enregistrement arrêté conservé en plus peut en prendre encore autant
                }
        }
# After the Buffer size chips: what the picked size costs. $memory is the
# memory it takes, e.g. ≈194 MB; $span how long it lasts, e.g. 14 h.
settings-buffer-cost = ({ $memory }, environ { $span })

## Which panels show, and what the Specifications panel shows: checkboxes
## on one row.

settings-graph = Graphique
    .hover = Afficher le graphique défilant des mesures dans le temps
# σ is the standard deviation, as the Statistics panel writes it.
settings-statistics = Statistiques
    .hover = Afficher Min / Max / Moyenne / σ / intégrale pour la session en cours
settings-recording = Enregistrement
    .hover = Afficher les commandes d’enregistrement et le journal des échantillons
# The Specifications panel's checkbox. $fields is shown while the panel is
# on and the checkboxes for its fields follow it, hidden otherwise.
settings-specs =
    { $fields ->
        [shown] Spécifications :
       *[hidden] Spécifications
    }
    .hover = Afficher la précision et la résolution du mode actuel
# The Specifications fields, as that panel names them.
settings-specs-resolution = Résolution
    .hover = Afficher la résolution dans les spécifications
settings-specs-accuracy = Précision
    .hover = Afficher la précision dans les spécifications
# Input Z is the input impedance.
settings-specs-input-z = Impédance d’entrée
    .hover = Afficher l’impédance d’entrée dans le panneau Spécifications complet
settings-specs-notes = Notes
    .hover = Afficher les notes du manuel dans le panneau Spécifications complet

## The window.

# The Language row's caption, shown once the app speaks more than one
# language. In every language but English, keep the English word in
# parentheses after yours, e.g. Langue (Language):, so someone who can't
# read the window can still find the row. .a11y names its dropdown for a
# screen reader, in your word alone.
settings-language = Langue (Language) :
    .a11y = Langue
    .hover = La langue de l’interface
# The dropdown's first entry: the window speaks the system's language.
# $language is the language that is, by its name in itself, e.g. Français
# or 简体中文. The other entries are each language's name in itself, kept
# as they are.
settings-language-system = Système ({ $language })
    .hover = Suivre la langue du système
# Weak text after your language's name in the dropdown while no native
# speaker has reviewed it yet. Shown only next to your own language's name,
# whatever language the window speaks, so its speakers can read it: e.g.
# Português (Brasil)  tradução automática.
settings-language-machine-translated = traduction automatique
# A screen reader's name for that entry, also in your language whatever the
# window speaks. $language is your language's name in itself, e.g.
# Português (Brasil).
settings-language-machine-translated-entry = { $language }, traduction automatique
# The weak line under the Language row while such a language is spoken,
# then a link that opens the translators' guide on GitHub in the web
# browser; ↗ marks a link.
settings-language-unreviewed = Traduction automatique, pas encore relue.
settings-language-review-link = Aider à la relire ↗
    .hover = Ouvrir le guide des traducteurs sur GitHub
# A toast when Chinese is picked and the computer has no font for it. It
# is always shown in English, which the app's own fonts draw: no need to
# translate it.
settings-language-no-chinese-font = No font for Chinese characters was found. Install a Simplified Chinese font, such as Noto Sans CJK SC, then restart dmm-gui.

# The Zoom chips are percentages, e.g. 150%.
settings-zoom = Zoom :
# A Zoom chip's hover. $level is its scale in percent, e.g. 150; $step the
# keys that step the zoom.
settings-zoom-chip =
    .hover = Afficher l’interface à { $level } % ({ $step } pour ajuster par paliers)
# The 100% chip's hover. $reset is the keys that go back to 100%, $step
# those that step the zoom.
settings-zoom-reset-chip =
    .hover = Afficher l’interface à 100 % ({ $reset }, ou { $step } pour ajuster par paliers)
# Window decorations are the title bar and border.
settings-hide-decorations = Masquer le cadre de la fenêtre
    .hover = Fenêtre sans cadre : utilisez { $keys } pour le rétablir
# On Wayland the checkbox is greyed: .wayland is its hover there, and
# settings-always-on-top-wayland the note beside it. Keep both the same as
# shortcuts-always-on-top-toast, which Ctrl+T shows there.
settings-always-on-top = Toujours visible
    .hover = Garder la fenêtre au-dessus des autres fenêtres ({ $keys })
    .wayland = Non disponible sous Wayland – faites un clic droit sur la barre de titre pour garder la fenêtre au-dessus des autres
# The note after the greyed checkbox on Wayland, in small grey text.
settings-always-on-top-wayland = (Non disponible sous Wayland – faites un clic droit sur la barre de titre pour garder la fenêtre au-dessus des autres)

## The update check, in builds downloaded from GitHub.

settings-check-updates = Rechercher les nouvelles versions
    .hover = Une fois par jour, demander à GitHub si une nouvelle version est sortie. GitHub voit votre adresse IP, mais aucune information sur votre multimètre n’est envoyée.
# The link the top bar shows when a newer release is out; ↗ marks a link.
# $version is the release, e.g. v0.10.0.
settings-update-release = { $version } disponible ↗
# The same for a newer nightly build, which has no version number.
settings-update-dev = Version de développement plus récente ↗
# Its shorter form, when the top bar is short of room.
settings-update-dev-short = Mise à jour ↗

## How the graph's lines look, the last row.

settings-graph-lines = Courbes :
# Sub-value lines are the extra traces a meter that shows several values
# at once draws beside the main one.
settings-graph-lines-patterned = À motifs
    .hover = Valeurs secondaires en tirets et en pointillés, distinctes sans la couleur
settings-graph-lines-solid = Pleines
    .hover = Lignes continues, distinguées par la couleur et la légende
