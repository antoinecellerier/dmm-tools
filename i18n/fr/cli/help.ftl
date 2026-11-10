### The parts of dmm-cli's --help built as it runs: the first lines of the
### help of --device and --mock-mode, which go on with lists made from the
### meters, and the notes after the options. What each command and option
### does is in cli-help.ftl.
###
### Words in --flag form, setting names (device_family, mock_mode) and
### environment variables (RUST_LOG, LANG) are typed by the user and stay as
### they are. Command lines, file paths and values arrive as variables.

## The first lines of two options' long help (dmm-cli --help, dmm-cli read
## --help).

# Before the list of devices.
help-device-intro = Appareil auquel se connecter.

## After the options of dmm-cli -h.

help-see-long-help = Lancez la commande avec --help pour obtenir la liste complète des appareils pris en charge et le chemin du fichier de paramètres partagé.
# $setting is NO_COLOR=1.
help-no-color = Définissez { $setting } pour désactiver les couleurs.

## After the options of dmm-cli --help: two sections, each a heading in
## capitals and indented lines. clap doesn't wrap these lines: keep them
## short.

help-configuration = CONFIGURATION :
# The settings file's path follows on the next line.
help-settings-file = Fichier de paramètres (partagé avec dmm-gui) :
# Where --device comes from when the command line doesn't give it: three
# numbered lines in order.
help-device-precedence = Priorité de --device :
help-precedence-flag = Option de ligne de commande
help-precedence-settings = device_family du fichier de paramètres ci-dessus
help-precedence-auto = auto : détecter le multimètre connecté
help-environment = ENVIRONNEMENT :
# Beside RUST_LOG; each line of the message is a line of the help. $filter
# is the filter that shows everything the library sends and receives.
help-env-rust-log =
    Filtre du journal. S’il n’est pas défini, les avertissements de dmm_lib et toutes les erreurs s’affichent.
    Utilisez `{ $filter }` pour déboguer au niveau des trames.
# Beside NO_COLOR.
help-env-no-color = Mettre à 1 pour désactiver les couleurs dans le terminal.
# Beside LANGUAGE; each line of the message is a line of the help. The
# variable names stay as they are. On macOS and Windows the system's
# language applies instead.
help-env-language =
    Langue des messages et de l’aide sous Linux, dans l’ordre de lecture de gettext :
    LANGUAGE, puis LC_ALL, LC_MESSAGES, LANG. Les bureaux Debian et Ubuntu
    définissent LANGUAGE, qui l’emporte sur LANG.
# Beside LC_ALL; each line of the message is a line of the help. C and
# POSIX are the locale names that mean English, on every system.
help-env-lc-all =
    C ou POSIX dans la première variable définie parmi LC_ALL, LC_MESSAGES et LANG
    impose l’anglais, quelle que soit la valeur de LANGUAGE.

## After the options of dmm-cli completions --help. The command line for
## each shell follows.

help-completions-install = Installez l’autocomplétion pour votre shell :
