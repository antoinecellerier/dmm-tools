### Text both dmm-cli and dmm-gui show: the shared parts of their --help,
### the setup help when no meter is found, and the notices both print.
###
### Command lines, file paths and web addresses are never part of a message.
### Words in --flag form, and 'dmm-cli list', are typed by the user and stay
### as they are.

## The project page link: the last line of both binaries' --help, and the
## link in the GUI's top bar.

project-link = Aide et suggestions
# $url is the project page's address.
project-link-line = { project-link } : { $url }

## The --device flag's long help, after the binary's own first line. The
## device list between the heading and the aliases is generated.

device-list-heading = Appareils :
# What the "auto" device does. Also its hover in the GUI's device list.
device-auto = Détecter le multimètre connecté
# The "auto" row of the list: what the flag does when it isn't given.
device-auto-default = { device-auto } (par défaut)
device-aliases =
    Accepte aussi des alias : ut61e+, ut61b, ut171a, ut181, etc.
    Mettez entre guillemets les noms à caractères spéciaux : --device 'ut61e+'
# Beside the simulated meters, in device lists.
device-no-hardware = aucun matériel nécessaire

## How far a meter's support is verified, beside it in device lists.

stability-verified = vérifié
stability-partly-verified = partiellement vérifié
stability-experimental = expérimental
# Both binaries show it for a meter short of verified, each followed by its
# own way to help verify it.
# $model is the meter's name, e.g. "UNI-T UT8803".
# $stability is verified, partly-verified or experimental.
stability-warning =
    { $stability ->
        [verified] La prise en charge du multimètre { $model } est vérifiée (vérifiée sur du matériel réel).
        [partly-verified] La prise en charge du multimètre { $model } est partiellement vérifiée (certains modes et commandes ne sont pas confirmés sur du matériel réel).
       *[experimental] La prise en charge du multimètre { $model } est expérimentale (non vérifiée sur du matériel réel).
    }

## The --mock-mode flag's long help, after the binary's own first line.

# $modes lists the mock's modes, separated by commas.
mock-mode-list = Modes : { $modes }
# $example is a command line.
mock-mode-example = Exemple : { $example }
mock-clock-mock-only = --mock-clock-scale et --mock-clock-preseed ne s’appliquent qu’au multimètre simulé et à une relecture. Relancez avec --device mock ou --replay, ou sans les options d’horloge.

## A link's short name, where the meter is already named: the GUI's
## status line, the setup help's section labels.

link-usb-cable = Câble USB
link-bluetooth = Bluetooth

## The help shown when no meter was found: a title, then one section per
## link searched — a label line, then steps. The CLI prints them in a
## terminal, the GUI in a notice.

# No full stop: the CLI adds one, the GUI's notice titles carry none.
not-found-usb = Aucun câble USB trouvé
not-found-usb-or-bluetooth = Aucun multimètre trouvé en USB ou en Bluetooth
# $address is a Bluetooth address or peripheral id.
not-found-bluetooth-at = Aucun appareil Bluetooth trouvé à l’adresse { $address }
# $model is the meter's name, e.g. "UNI-T UT60BT".
not-found-bluetooth-only = Aucun multimètre { $model } trouvé en Bluetooth
# A section's label line.
# $link is link-usb-cable or link-bluetooth.
# $check is setup-usb-check or setup-bluetooth-check.
setup-label = { $link } : { $check }
setup-usb-check = vérifiez qu’il est branché et que le multimètre est allumé.
setup-bluetooth-check = activez le Bluetooth sur cet ordinateur et la transmission des données sur le multimètre.
# Linux. Two command lines follow.
setup-usb-linux-rule = Vérifiez que la règle udev est installée :
# Linux. The setup guide's address follows.
setup-usb-linux-replug = Rebranchez ensuite le câble. Sur une machine sans écran, ajoutez un groupe à la règle :
# Windows. The quoted names are what Device Manager shows; use the names
# Windows shows in your language.
setup-usb-windows-open = Ouvrez le Gestionnaire de périphériques avec le câble branché :
setup-usb-windows-cp2110 = - 'CP2110 USB to UART Bridge' sous Périphériques d’interface utilisateur : aucune action nécessaire.
# UNVERIFIED Windows name
setup-usb-windows-input = - 'Périphérique d’entrée USB' sous Périphériques d’interface utilisateur : aucune action nécessaire.
# The driver's download address follows.
setup-usb-windows-warning = - Icône d’avertissement jaune sous 'Autres périphériques' : installez le pilote depuis
setup-usb-windows-nothing = - Rien n’apparaît : essayez un autre port USB.
# macOS. Name the settings as macOS shows them in your language.
setup-usb-macos-auto = Le câble devrait être reconnu automatiquement (aucun pilote nécessaire).
setup-usb-macos-privacy = Si l’appareil est introuvable, vérifiez Réglages Système > Confidentialité et sécurité > Surveillance de l’entrée.
setup-bluetooth-adapter = Si les deux sont activés, éteignez puis rallumez l’adaptateur.
setup-bluetooth-macos-privacy = Autorisez le Bluetooth dans Réglages Système > Confidentialité et sécurité > Bluetooth.
# The device catalog's address follows.
setup-bluetooth-catalog = Les adaptateurs et multimètres pris en charge figurent dans le catalogue des appareils :
setup-bluetooth-scan = Lancez 'dmm-cli list' pour rechercher les adaptateurs et multimètres à portée.

## Notices both binaries show.

# A cable or Bluetooth link is open but no meter on it identified itself.
# A notice title in the GUI; dmm-cli starts a sentence with it, naming the
# link after it (cli/connect.ftl).
no-meter-answered = Aucun multimètre n’a répondu
# The meter picked or found has stopped answering. A notice title in the
# GUI; dmm-cli starts a sentence with it (cli/connect.ftl).
no-response = Pas de réponse du multimètre
# The GUI checkbox that lets a connection search over Bluetooth. The CLI
# reads the same setting, so its help names the checkbox too.
bluetooth-setting = Rechercher les appareils Bluetooth
bluetooth-off-cli-flag = Retirez l’option --no-bluetooth.
bluetooth-off-cli-setting = Cochez « { bluetooth-setting } » dans les paramètres de dmm-gui, que dmm-cli lit aussi.
bluetooth-off-gui = Cochez « { bluetooth-setting } » dans Paramètres (⚙).

## What is connected, when an --adapter value matched nothing.

adapters-none = Aucun appareil connecté pour le moment.
adapters-heading = Appareils connectés :
adapters-unavailable = Lancez 'dmm-cli list' pour voir les appareils connectés.
# $address is the value the user typed, written with colons, e.g. 12:34:56:78:9A:BC.
colonless-address-hint = S’il s’agit d’une adresse Bluetooth, écrivez-la { $address }.

## Why a number typed for a scale, an offset or an alarm was refused: the
## reason alone, in lower case and with no full stop. dmm-gui puts it after
## the field's name in a toast ("Invalid scale: …"); dmm-cli after the
## option and the value, at the end of a line from its command-line parser,
## which stays English ("error: invalid value '0' for '--scale <FACTOR>': …").

# $text is what was typed, as it was typed.
input-not-a-number = « { $text } » n’est pas un nombre
# Infinity or "not a number" (NaN).
input-not-finite = doit être un nombre fini
# A scale factor of 0 would turn every reading into the offset.
input-zero-scale = un facteur d’échelle nul effacerait la mesure
# The alarm's low limit and its high limit.
input-low-not-below-high = la limite basse doit être inférieure à la limite haute
# The alarm's hysteresis band: a number in the limits' unit, or a percentage
# of the limit. $text is what was typed, as it was typed.
input-band-not-a-number = « { $text } » n’est ni un nombre ni un pourcentage tel que 1%
# A hysteresis band below zero.
input-band-negative = doit être positif ou nul
# A percentage band with a limit of 0, of which every percentage is 0.
input-percent-of-zero = un pourcentage d’une limite de 0 vaut toujours 0 : donnez plutôt une valeur

## Why an exported file or a recording did not read: in dmm-gui's toast
## after "Import failed:" and the file, in dmm-cli after "Error:" and the
## file. Lower case, with no full stop. The backquotes (`) set off text
## from the file, or a name it must have, which stays as it is: keep them.
## A parser's own words ($detail) arrive in English.

# Where the file stopped: $line is a line number, $reason one of the
# reasons below.
read-error-at-line = ligne { $line } : { $reason }
read-error-empty = le fichier est vide
# A CSV export without its first row naming the columns.
read-error-no-header-row = pas de ligne d’en-tête ({ $detail })
# $column is the name of a column every CSV export has, e.g. timestamp.
read-error-no-column = pas de colonne `{ $column }`
# A JSON export whose first line is not the one starting {"_metadata":.
read-error-no-metadata = pas de ligne `_metadata` ({ $detail })
# $text is what the file has where a reading's value should be.
read-error-not-a-value = `{ $text }` n’est pas une valeur
# $text is a cell starting NCV: whose level is no number. NCV is
# non-contact voltage detection, as the meter prints it.
read-error-not-an-ncv-level = `{ $text }` n’est pas un niveau NCV
# $text is the part of a flags cell that names no status flag.
read-error-not-a-flag = `{ $text }` n’est pas un indicateur
# $text is what the file has where a marker's number should be.
read-error-not-a-marker-number = `{ $text }` n’est pas un numéro de marqueur
# $text is a timestamp from the file, or a recording's "# recorded:" line;
# RFC 3339 is the standard date format the files use.
read-error-not-a-date = `{ $text }` n’est pas une date RFC 3339 : { $detail }
