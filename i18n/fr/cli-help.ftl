### dmm-cli's --help: what each command and option does.
###
### Generated from the doc comments in crates/dmm-cli/src/cli.rs by
### `UPDATE_L10N=1 cargo test -p dmm-cli cli_help_ftl`: edit those, not this
### file. The run names each ID whose English changed. When the meaning changed
### too, delete that message from every language's cli-help.ftl, so it shows in
### English until it is translated again.
###
### IDs follow the command tree: read.about is what the read command does,
### read--interval-ms.help what its --interval-ms option does. clap wraps each
### paragraph to the terminal's width: keep a paragraph on one line. Option
### names (--device), values (auto, mock) and command lines ('dmm-cli list')
### stay as they are. clap's own words around these texts (Usage:, Options:,
### [default: …], [possible values: …]) are in shared.ftl.

# dmm-cli
dmm-cli =
    .about = Outil en ligne de commande pour multimètres numériques USB et Bluetooth
# dmm-cli --lang
dmm-cli--lang =
    .help = Langue des messages et de l’aide, ou `system` pour celle du système. Sans cette option, `language` dans le fichier de paramètres (défini par la ligne Langue de dmm-gui) s’applique, ou à défaut la langue du système
# dmm-cli list
list =
    .about = Lister les câbles USB connectés, ainsi que les adaptateurs et multimètres Bluetooth à portée
# dmm-cli info
info =
    .about = Se connecter et afficher les informations de l’appareil
# dmm-cli read
read =
    .about = Lire les mesures en continu
# dmm-cli read --interval-ms
read--interval-ms =
    .help = Garder au plus une mesure par intervalle, en millisecondes (0 = chaque mesure que produit le multimètre)
# dmm-cli read --format
# .value-… says what that value does, in --help's list of values.
read--format =
    .help = Format de sortie [par défaut : text, ou celui qu’indique l’extension du fichier -o]
    .value-replay = Les trames envoyées par le multimètre, que --replay peut relire
# dmm-cli read --output
read--output =
    .help = Fichier de sortie (stdout si non précisé). Si l’option est donnée sans nom, le fichier est nommé d’après le multimètre, son mode et le début de l’exécution
# dmm-cli read --count
read--count =
    .help = Nombre de mesures (0 = illimité : jusqu’à Ctrl+C, ou jusqu’à la fin d’un fichier --replay)
# dmm-cli read --no-reconnect
read--no-reconnect =
    .help = Terminer l’exécution quand la liaison avec le multimètre est perdue, au lieu de se reconnecter
# dmm-cli read --integrate
read--integrate =
    .help = Afficher l’intégrale cumulée dans le temps (charge pour les modes courant, V·s pour la tension)
# dmm-cli read --scale
read--scale =
    .help = Multiplier la mesure, prise en unités de base (V, A, Ω, …), par FACTOR. Une pince de 10 mV/A s’utilise avec --scale 100, une sonde 100:1 avec --scale 100
# dmm-cli read --offset
read--offset =
    .help = Ajouter VALUE après la mise à l’échelle (32 avec --scale 1.8 convertit les °C en °F)
# dmm-cli read --unit
read--unit =
    .help = Étiqueter la mesure mise à l’échelle avec cette unité au lieu de l’unité de base du multimètre
# dmm-cli read --alarm-high
read--alarm-high =
    .help = Avertir quand la mesure dépasse VALUE, en unités de base (V, A, Ω, …), ou dans l’unité --unit d’une mesure mise à l’échelle
# dmm-cli read --alarm-low
read--alarm-low =
    .help = Avertir quand la mesure passe sous VALUE, comme pour --alarm-high
# dmm-cli read --alarm-hysteresis
read--alarm-hysteresis =
    .help = De combien une mesure doit repasser du bon côté de la limite avant que celle-ci déclenche de nouveau l’alarme : une valeur dans l’unité des limites, ou un pourcentage de la limite (`1%`) [par défaut : quelques points du dernier chiffre du multimètre]
# dmm-cli read --alarm-bell
read--alarm-bell =
    .help = Faire sonner le terminal à chaque alarme
# dmm-cli read --replay
read--replay =
    .help = Relire un fichier écrit par --format replay au lieu d’ouvrir un multimètre, jusqu’à la fin du fichier
# dmm-cli read --import
read--import =
    .help = Lire un fichier exporté (CSV, JSON ou relecture) au lieu d’ouvrir un multimètre, sans attendre, afin de le convertir ou d’afficher son résumé
# dmm-cli command
command =
    .about = Envoyer au multimètre une commande qui équivaut à appuyer sur une touche. Lancez la commande sans argument pour lister les commandes disponibles pour l’appareil sélectionné
# dmm-cli command <ACTION>
command--action =
    .help = Nom de la commande (lancez sans argument pour voir les commandes disponibles)
# dmm-cli get
get =
    .about = Lister les choix que proposent les réglages du multimètre depuis sa position actuelle. Lancez la commande sans argument pour voir tous les réglages qui offrent un choix
# dmm-cli get <SETTING>
get--setting =
    .help = Réglage à lister (omettez-le pour les lister tous)
# dmm-cli get --format
get--format =
    .help = Format de sortie
# dmm-cli set
set =
    .about = Changer un réglage du multimètre sans toucher à l’appareil. Lancez la commande sans choix pour lister ce que ce réglage permet d’atteindre depuis la position actuelle
# dmm-cli set <SETTING>
set--setting =
    .help = Réglage à changer
# dmm-cli set <CHOICE>
set--choice =
    .help = Libellé d’une valeur de la liste, ou fragment de libellé qui n’en désigne qu’une seule (lancez la commande sans choix pour les voir)
# dmm-cli debug
debug =
    .about = Mode de vidage hexadécimal brut pour le débogage du protocole
# dmm-cli debug --count
debug--count =
    .help = Nombre de requêtes à envoyer (0 = illimité)
# dmm-cli debug --interval-ms
debug--interval-ms =
    .help = Garder au plus une mesure par intervalle, en millisecondes (0 = chaque mesure)
# dmm-cli completions
completions =
    .about = Générer l’autocomplétion du shell
# dmm-cli completions <SHELL>
completions--shell =
    .help = Shell pour lequel générer l’autocomplétion
# dmm-cli capture
capture =
    .about = Capture guidée du protocole pour les rapports de bug et la vérification
# dmm-cli capture --output
capture--output =
    .help = Fichier de sortie (par défaut : capture-<device>.yaml). Remplace le nommage automatique
# dmm-cli capture --steps
capture--steps =
    .help = N’exécuter que certaines étapes (identifiants séparés par des virgules, par exemple "dcmv,temp,duty")
# dmm-cli capture --unverified
capture--unverified =
    .help = N’exécuter que les étapes pas encore confirmées sur un vrai multimètre, ainsi que les captures supplémentaires
# dmm-cli capture --plan
capture--plan =
    .help = Exécuter les étapes du fichier de plan d’un mainteneur au lieu de la liste propre à l’appareil
# dmm-cli capture --sniff
capture--sniff =
    .help = Ne se fier à rien de ce que dit l’analyseur : détecter les étapes d’après les changements des octets bruts et les confirmer une à une
# dmm-cli capture --no-drive
capture--no-drive =
    .help = Ne pas laisser l’outil régler lui-même les plages et les indicateurs après chaque étape de mode
# dmm-cli capture --settle
capture--settle =
    .help = Attendre MS avant d’échantillonner chaque étape, pour les mesures qui se stabilisent lentement
# dmm-cli capture --list-steps
capture--list-steps =
    .help = Lister tous les identifiants d’étape disponibles et quitter
# dmm-cli capture --format
capture--format =
    .help = Façon dont --list-steps affiche la liste (md est la liste de contrôle du ticket)
# dmm-cli capture --export-instructions
capture--export-instructions =
    .help = Écrire les instructions des étapes dans FILE, à traduire pour --instructions, puis quitter
# dmm-cli capture --instructions
capture--instructions =
    .help = Afficher les instructions des étapes lues dans FILE, traduites à partir de ce qu’a écrit --export-instructions
