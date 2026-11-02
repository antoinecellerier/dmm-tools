### What dmm-cli prints while it starts and opens the meter: which meter it
### was told to open, what detection found, and what to try when nothing
### answers. A terminal program: these lines go to the terminal's error
### stream, and stay readable when the readings are piped to a file.
###
### Words in --flag form, setting names (mock_mode), 'capture' and dmm-gui
### are typed or known by those names and stay as they are. Command lines
### and web addresses are never part of a message: they arrive as variables,
### on lines of their own.

## Refusals before anything is opened. Each follows "Error:" or "Warning:"
## (common.ftl) on the same line.

# $device is the meter id as given with --device or saved by dmm-gui, e.g.
# ut61eplus.
connect-unknown-device = appareil inconnu : { $device }
# info and debug talk to a real meter. $device is the simulated meter's id,
# e.g. mock.
connect-needs-hardware = Cette commande demande un appareil réel (impossible avec --device { $device }).
# The settings file names a simulated-meter scenario that doesn't exist.
# mock_mode is the setting's name in the file. $error is the reason, in
# English.
connect-saved-mock-mode = mock_mode dans le fichier de paramètres : { $error }

## Which meter the run opens: under a failed open's first line, and at the
## top of 'info'.

# $meter is the meter's brand and model, e.g. UNI-T UT61E+. $by says where
# it was named: flag (--device on the command line; $device is the id
# typed, e.g. ut61eplus), settings (saved by dmm-gui) or recording (the file
# being played back).
connect-selected =
    { $by ->
       *[flag] Sélectionné : { $meter } (--device { $device })
        [settings] Sélectionné : { $meter } (enregistré par dmm-gui ; indiquez --device auto pour détecter le multimètre)
        [recording] Sélectionné : { $meter } (indiqué par l’enregistrement)
    }
# The name the meter gave for itself. $name is that name as the meter sent
# it, or connect-reported-name-none.
connect-reported-name = Nom annoncé : { $name }
# In place of the name when the meter gave none, which many never do.
connect-reported-name-none = (aucun)

## Detection found the meter: one dim line before the readings. $meter is
## its brand and model, e.g. UNI-T UT61E+; $device the id that names it on
## the command line, e.g. ut61eplus.

# The meter gave no name, or the model's own.
connect-detected = { $meter } détecté (indiquez --device { $device } ou choisissez-le dans dmm-gui pour éviter la détection la prochaine fois)
# The meter gave a name of its own that isn't the model's: $reported is
# that name, e.g. UT216XD. "Tables" are how the readings are decoded.
connect-detected-reports = { $meter } détecté (le multimètre annonce « { $reported } », décodage avec les tables du multimètre { $meter } : indiquez --device { $device } pour les conserver, ou un autre identifiant pour les remplacer)

## Once a meter whose support isn't fully verified is open: a warning, then
## the command that makes a report, on a line of its own, then where to send
## it.

# $warning is the sentence saying how far the meter's support is verified.
connect-unverified-warning = AVERTISSEMENT : { $warning }
# Followed by the command line to run.
connect-unverified-capture = Lancez 'capture' pour générer un rapport de validation :
# $url is the web address of the meter's issue on GitHub.
connect-report-feedback = Envoyez vos commentaires : { $url }

## Nothing was found to open. A first line, then the steps for each link
## (shared.ftl), then, for a meter whose support isn't fully verified, a
## closing line.

# The first line. $title says what was not found, e.g. "No USB cable
# found"; this message ends the sentence.
connect-not-found = { $title }.
# The closing line. $warning is the sentence saying how far the meter's
# support is verified; $url the web address of its issue on GitHub.
connect-unverified-note = { $warning } Envoyez vos commentaires : { $url }

## A cable or Bluetooth link is open, but no meter on it answered. A first
## line, the meters that link can carry with the steps that switch each one's
## data transmission on, then a closing paragraph.

# The first line. $title is "No meter answered" (shared.ftl). $link is usb
# (a USB cable), adapter (a Bluetooth adapter plugged into the meter) or
# radio (the meter's own Bluetooth).
connect-not-identified =
    { $link ->
        [adapter] { $title } via l’adaptateur Bluetooth.
        [radio] { $title } via la liaison Bluetooth.
       *[usb] { $title } via le câble USB.
    }
# The closing paragraph; keep its line breaks, and the {"  "} that indents
# the two command lines under the prose. $command is the command line that names the meter
# (dmm-cli --device <id> ...); $help the one that lists the ids
# (dmm-cli --help); $logging the setting that turns the detailed log on
# (RUST_LOG=dmm_lib=debug); $url the web address of the issue tracker.
connect-not-identified-name-it =
    Si le multimètre est allumé et transmet, mais n’est toujours pas reconnu, précisez son modèle :
    {"  "}{ $command }      ({ $help } liste les identifiants)
    et signalez-le à l’adresse ci-dessous, en joignant la sortie obtenue avec { $logging } :
    {"  "}{ $url }

## --adapter, a command-line option, named a USB device that isn't there.
## Under the list of what is plugged in.

# $selector is the option as typed, --adapter <serial-path-or-address>.
connect-adapter-select = Utilisez { $selector } pour en choisir un.

## The meter stopped answering. After "Warning:" (common.ftl); the meter's
## own steps for switching data transmission on follow.

# $title is "No response from meter" (shared.ftl). $meter is the meter
# picked, e.g. UNI-T UT61E+.
connect-no-response = { $title }. Vérifiez qu’il s’agit bien du multimètre { $meter } et que la transmission des données est activée.

## The line that closes a failed open, after "Error:" (common.ftl). The
## lines above it have already said what to try.

connect-error-not-found = appareil introuvable
connect-error-not-identified = appareil non identifié
connect-error-adapter-not-found = adaptateur introuvable
# $device is the meter id given with --device, e.g. auto.
connect-error-not-simulated = --device { $device } n’est pas simulé
