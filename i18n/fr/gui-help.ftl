### dmm-gui's --help: what the program and each option do.
###
### Generated from the doc comments in crates/dmm-gui/src/main.rs by
### `UPDATE_L10N=1 cargo test -p dmm-gui gui_help_ftl`: edit those, not this
### file. The run names each ID whose English changed. When the meaning changed
### too, delete that message from every language's gui-help.ftl, so it shows in
### English until it is translated again.
###
### IDs follow the options: dmm-gui--replay.help is what --replay does. clap
### wraps each paragraph to the terminal's width: keep a paragraph on one line.
### Option names (--device), values (auto, mock) and command lines ('dmm-cli
### list') stay as they are. The help built as the program runs is in
### gui/command-line.ftl, and clap's own words around these texts (Usage:,
### Options:, [possible values: …]) are in shared.ftl.

# dmm-gui
dmm-gui =
    .about = Application graphique pour multimètres numériques USB et Bluetooth
# dmm-gui --device
dmm-gui--device =
    .help = Appareil auquel se connecter [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...]
# dmm-gui --mock-mode
dmm-gui--mock-mode =
    .help = Fixer l’appareil simulé sur un mode précis (implique --device mock)
# dmm-gui --theme
dmm-gui--theme =
    .help = Thème à utiliser à la place du thème enregistré
# dmm-gui --renderer
dmm-gui--renderer =
    .help = Moteur de rendu graphique
# dmm-gui --adapter
dmm-gui--adapter =
    .help = Choisir un adaptateur précis quand plusieurs sont accessibles. Indiquez le numéro de série ou le chemin HID d’un câble USB, ou l’adresse d’un adaptateur ou d’un multimètre Bluetooth, tels que 'dmm-cli list' les affiche
# dmm-gui --no-bluetooth
dmm-gui--no-bluetooth =
    .help = Désactiver la recherche Bluetooth pour cette session. Remplace le paramètre « Rechercher les appareils Bluetooth ». Une adresse passée à --adapter est tout de même utilisée
# dmm-gui --replay
dmm-gui--replay =
    .help = Relire un fichier écrit par 'dmm-cli read --format replay' ou par Exporter… → Relecture…, au lieu de se connecter à un multimètre
# dmm-gui --import
dmm-gui--import =
    .help = Ouvrir un fichier exporté (CSV, JSON ou relecture), avec ses marqueurs, au lieu de se connecter à un multimètre
# dmm-gui --completions
dmm-gui--completions =
    .help = Afficher un script d’autocomplétion pour le shell, puis quitter
