### Text both dmm-cli and dmm-gui show: the shared parts of their --help
### and the words clap puts around it, their command-line errors, the
### setup help when no meter is found, and the notices both print.
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
    Mettez entre guillemets les noms qui contiennent des caractères spéciaux : --device 'ut61e+'
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
        [verified] La prise en charge du multimètre { $model } est vérifiée sur du matériel réel.
        [partly-verified] La prise en charge du multimètre { $model } est partiellement vérifiée (certains modes et commandes ne sont pas confirmés sur du matériel réel).
       *[experimental] La prise en charge du multimètre { $model } est expérimentale (non vérifiée sur du matériel réel).
    }

## The help of options both binaries take, after the option in --help, as
## clap shows it: no full stop. Command lines ('dmm-cli list') and options
## (--adapter) stay as they are. Mock mode is the setting's name in
## dmm-gui's Settings, mock_mode its name in the settings file.

flag-adapter = Choisir un adaptateur précis quand plusieurs sont accessibles. Indiquez le numéro de série ou le chemin HID d’un câble USB, ou l’adresse d’un adaptateur ou d’un multimètre Bluetooth, tels que 'dmm-cli list' les affiche
# dmm-cli's, on read, get and set: the mock dmm-gui saved counts too.
flag-mock-mode-cli = Fixer l’appareil simulé (--device mock, ou celui enregistré par dmm-gui) sur un mode de mesure précis au lieu de parcourir tous les modes. Sans cette option, c’est la clé mock_mode du fichier de paramètres (le paramètre « Mode simulé » de dmm-gui) qui fixe ce mode
# dmm-gui's: the mock picked in Settings counts too.
flag-mock-mode-gui = Fixer l’appareil simulé (--device mock, ou celui choisi dans Paramètres) sur un mode de mesure précis au lieu de parcourir tous les modes. Remplace pour cette session le paramètre enregistré « Mode simulé »
# dmm-cli's: its 'list' command shows what is plugged in.
flag-no-bluetooth-cli = Désactiver la recherche Bluetooth pour cette exécution : 'list' n’affiche alors que les câbles. Remplace le paramètre « { bluetooth-setting } » de dmm-gui. Une adresse passée à --adapter est tout de même utilisée
flag-no-bluetooth-gui = Désactiver la recherche Bluetooth pour cette session. Remplace le paramètre « { bluetooth-setting } ». Une adresse passée à --adapter est tout de même utilisée
# dmm-cli's, beside --no-bluetooth.
flag-bluetooth-cli = Rechercher les adaptateurs et multimètres Bluetooth pendant cette exécution, quelle que soit la valeur du paramètre « { bluetooth-setting } » de dmm-gui

## The rest of --mock-mode's long help, after flag-mock-mode-cli or
## flag-mock-mode-gui.

# $modes lists the mock's modes, separated by commas.
mock-mode-list = Modes : { $modes }
# $example is a command line.
mock-mode-example = Exemple : { $example }

## Why a mock-only option was refused, after the option and the value as
## typed.

# The session's device, typed with --device or saved by dmm-gui, is not the
# mock.
mock-mode-mock-only = --mock-mode ne s’applique qu’à l’appareil simulé. Relancez avec --device mock, ou sans --mock-mode.
mock-clock-mock-only = --mock-clock-scale et --mock-clock-preseed ne s’appliquent qu’à un multimètre simulé et à une relecture. Relancez avec --device mock ou --replay, ou sans les options d’horloge.

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
setup-bluetooth-check =
    { $host ->
        [phone] activez le Bluetooth sur ce téléphone et la transmission des données sur le multimètre.
       *[computer] activez le Bluetooth sur cet ordinateur et la transmission des données sur le multimètre.
    }
# After "Bluetooth:", when the search was switched off; how to switch it on
# follows as a step of its own.
setup-bluetooth-off-check = recherche non effectuée.
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
setup-usb-macos-privacy = Si l’appareil est introuvable, vérifiez l’autorisation dans Réglages Système > Confidentialité et sécurité > Surveillance de l’entrée.
setup-bluetooth-adapter =
    { $host ->
        [phone] Si les deux sont activés, désactivez puis réactivez le Bluetooth de ce téléphone.
       *[computer] Si les deux sont activés, éteignez puis rallumez l’adaptateur.
    }
setup-bluetooth-macos-privacy = Autorisez le Bluetooth dans Réglages Système > Confidentialité et sécurité > Bluetooth.
# The device catalog's address follows.
setup-bluetooth-catalog = Les adaptateurs et multimètres pris en charge figurent dans le catalogue des appareils :
setup-bluetooth-scan = Lancez 'dmm-cli list' pour rechercher les adaptateurs et multimètres à portée.

## Notices both binaries show.

# The settings file names a meter this version has no entry for: a file a
# newer version wrote, or one edited by hand. $device is the id as the file
# holds it, e.g. ut62; $path is the file. dmm-gui shows it in a toast,
# dmm-cli after "Warning:" and before what to do.
saved-device-unknown = Le fichier de paramètres { $path } désigne l’appareil « { $device } », que cette version ne connaît pas : le multimètre est donc détecté automatiquement.
# The settings file's mock_mode (the setting's name in the file, kept as it
# is) names no scenario of the simulated meter. $given is the name as the
# file holds it. Each app follows it with what to do.
saved-mock-mode-unknown = La valeur « { $given } » de mock_mode dans le fichier de paramètres ne désigne aucun mode simulé : l’appareil simulé parcourt donc tous les modes.
# When the meter opened, other cables were there to open and the session
# took the first, with --adapter a command-line option kept as it is. $count
# is 2 or more. dmm-gui shows it in a toast, dmm-cli after "Warning:".
cables-at-open =
    { $host ->
        [phone]
            { $count ->
                [one] { $count } câble USB utilisable par cette session est branché : il a été ouvert.
               *[other] { $count } câbles USB utilisables par cette session sont branchés : le premier trouvé a été ouvert.
            }
       *[computer]
            { $count ->
                [one] { $count } câble USB utilisable par cette session est branché : il a été ouvert. Utilisez --adapter pour en choisir un autre.
               *[other] { $count } câbles USB utilisables par cette session sont branchés : le premier trouvé a été ouvert. Utilisez --adapter pour en choisir un autre.
            }
    }
# When the meter opened over Bluetooth, other adapters or meters matched as
# well and the session took the first, with --adapter a command-line option
# kept as it is. $count is 2 or more. $host as in cables-at-open.
bluetooth-tied =
    { $host ->
        [phone] { $count } adaptateurs ou multimètres Bluetooth correspondent tous aussi bien à la recherche : le premier trouvé a été ouvert.
       *[computer] { $count } adaptateurs ou multimètres Bluetooth correspondent tous aussi bien à la recherche : le premier trouvé a été ouvert. Pour en choisir un autre, indiquez avec --adapter l’un de ceux qu’affiche 'dmm-cli list'.
    }
# Follows cables-at-open when nothing tells those cables apart.
cables-no-reconnect =
    { $host ->
        [phone] Rien ne permet de distinguer ces câbles : une coupure de la liaison mettra fin à cette session.
       *[computer] Rien ne permet de distinguer ces câbles : sauf si --adapter indique le chemin d’un câble, une coupure de la liaison mettra fin à cette session.
    }
# The meter named another model than the one picked. $reported is the name
# it gave, e.g. UT161E; $picked the model picked, e.g. UNI-T UT61E+. Each
# app follows it with what to pick (choose-meter-gui, choose-meter-cli).
other-model = Le multimètre indique « { $reported } », et non le modèle { $picked } sélectionné.
# The same when detection opened the meter, without hearing its name in
# time, as $picked; $reported as above.
other-model-detected = Le multimètre indique « { $reported } », mais la détection l’a ouvert en tant que modèle { $picked }.
# The meter's packets are another model's than the one it was opened as:
# $named is that model, e.g. ZOYI / ZOTEK ZT-5566SE; $opened the one opened,
# e.g. ZOYI / ZOTEK ZT-300AB. Each app follows it with what to pick.
other-data = Le multimètre envoie les données d’un modèle { $named }, et non celles du modèle { $opened } choisi à l’ouverture.
# The meter sent data this tool doesn't read yet, said once per run. $meter
# is the meter, e.g. UNI-T UT61E+; $detail what it sent, in English, e.g.
# mode byte: 0x7f. Each app follows it with how to report it.
unrecognised-data = Le multimètre { $meter } a envoyé des données que cet outil ne connaît pas ({ $detail }).
# A cable or Bluetooth link is open but no meter on it identified itself.
# A notice title in the GUI; dmm-cli starts a sentence with it, naming the
# link after it (cli/connect.ftl).
no-meter-answered = Aucun multimètre n’a répondu
# The meter picked or found has stopped answering. A notice title in the
# GUI; dmm-cli starts a sentence with it (cli/connect.ftl).
no-response = Pas de réponse du multimètre
# Why a dropped link is not reconnected. $count (2 or more) USB cables the
# open could take are plugged in, with no serial number of their own to tell
# them apart. Each binary follows it with what to do.
several-cables =
    { $count ->
        [one] { $count } câble USB utilisable par cette session est branché, et rien ne permet de le distinguer. Une reconnexion risquerait d’ouvrir le câble d’un autre multimètre.
       *[other] { $count } câbles USB utilisables par cette session sont branchés, et rien ne permet de les distinguer. Une reconnexion risquerait d’ouvrir le câble d’un autre multimètre.
    }
# Why a dropped link is no longer being reopened: it stayed down for
# $minutes minutes, and the session has ended. A notice or the error that
# ended the run says what the last attempt found.
reconnect-gave-up =
    { $minutes ->
        [one] Reconnexion abandonnée : la liaison avec le multimètre est restée coupée pendant { $minutes } minute.
       *[other] Reconnexion abandonnée : la liaison avec le multimètre est restée coupée pendant { $minutes } minutes.
    }
# While a dropped link is being reopened: its cable has gone and another is
# plugged in. Each binary follows it with how to use that other cable.
waiting-for-own-cable = Un autre câble USB est branché, mais cette session attend celui qu’elle a ouvert.
# Under a reading the meter shows on its own screen only, its frames carrying
# none: a CMS or UT217's oscilloscope mode.
screen-only-reading = En mode oscilloscope, lisez les mesures sur l’écran du multimètre
# The GUI checkbox that lets a connection search over Bluetooth. The CLI
# reads the same setting, so its help names the checkbox too.
bluetooth-setting = Rechercher les appareils Bluetooth
# How dmm-cli switches the search on: after --no-bluetooth, and when the
# saved setting is off.
bluetooth-off-cli-instead = Indiquez --bluetooth à la place de --no-bluetooth.
bluetooth-off-cli-saved = Indiquez --bluetooth, ou cochez « { bluetooth-setting } » dans les paramètres de dmm-gui : dmm-cli les lit aussi.
bluetooth-off-gui = Cochez « { bluetooth-setting } » dans Paramètres (⚙).
# What to check after the meter refused a mode or range switch, or did not
# take it. $setting is range (the input must be within the range too) or
# another setting.
switch-check-dial =
    { $setting ->
        [range] Vérifiez la position du commutateur et assurez-vous que le signal d’entrée reste dans la plage.
       *[other] Vérifiez la position du commutateur.
    }
# How to open the meter as the model it is, after a message saying it is
# another, in dmm-gui. $choice is entry ($model the meter to pick, e.g.
# OWON CMS061), entry-only (the same, when Auto-detect is what opened it),
# auto-detect, or own-model (nothing says which model: the user has to pick
# it). "Auto-detect" is the Device choice that finds the meter itself.
choose-meter-gui =
    { $choice ->
        [entry] Choisissez le multimètre { $model } ou Détection automatique dans Paramètres (⚙).
        [entry-only] Choisissez le multimètre { $model } dans Paramètres (⚙).
        [auto-detect] Choisissez Détection automatique dans Paramètres (⚙).
       *[own-model] Choisissez le véritable modèle du multimètre dans Paramètres (⚙).
    }
# The same in dmm-cli: $choice as above, $id the --device value of the
# meter to pick, e.g. cms061; --device auto detects the meter.
choose-meter-cli =
    { $choice ->
        [entry] Indiquez --device { $id }, ou --device auto.
        [entry-only] Indiquez --device { $id }.
        [auto-detect] Indiquez --device auto.
       *[own-model] Indiquez avec --device le véritable modèle du multimètre (--help en donne la liste).
    }

## What is connected, when an --adapter value matched nothing.

adapters-none = Aucun appareil connecté pour le moment.
adapters-heading = Appareils connectés :
adapters-unavailable = Lancez 'dmm-cli list' pour voir les appareils connectés.
# $address is the value the user typed, written with colons, e.g. 12:34:56:78:9A:BC.
colonless-address-hint = S’il s’agit d’une adresse Bluetooth, écrivez-la sous la forme { $address }.

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
# An alarm limit: a number, or a number with a unit or a prefix such as
# 4980 mV, 10 kohm or 4.7k (kept as they are). $text is what was typed.
input-not-a-quantity = « { $text } » n’est pas un nombre, avec ou sans unité (par exemple 4980 mV ou 4.7k)
# The alarm's two limits were typed with units of different quantities,
# such as volts and amps.
input-limit-units-differ = les limites basse et haute doivent être exprimées dans la même unité
# The alarm's hysteresis band: a number in the limits' unit, or a percentage
# of the limit. $text is what was typed, as it was typed.
input-band-not-a-number = « { $text } » n’est ni un nombre ni un pourcentage tel que 1%
# A hysteresis band below zero.
input-band-negative = doit être positif ou nul
# A hysteresis band typed with a unit, while the limits were typed in
# another unit or with none.
input-band-not-the-limits-unit = l’écart de retour ne peut avoir d’unité que si les limites sont saisies dans cette unité
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

## The words clap puts around every page of dmm-cli's and dmm-gui's --help:
## its headings, what follows an option, and the help of the built-in -h,
## -V and help. They are the same on every page.

# The headings, each before its list.
help-usage = Utilisation :
help-commands = Commandes :
help-arguments = Arguments :
help-options = Options :
# After an option's help, in brackets. $value is what the option is when not
# given, e.g. 500.
help-default = [par défaut : { $value }]
# After an option's help, in brackets. $values is the values it takes,
# separated by commas.
help-possible-values = [valeurs possibles : { $values }]
# Before the list of values an option takes, each with what it does.
help-possible-values-heading = Valeurs possibles :
# What -h and --help do: on a command with more detail under --help, the two
# notes say how to see more or less.
help-flag = Afficher l’aide
help-flag-short = Afficher l’aide (plus de détails avec '--help')
help-flag-long = Afficher l’aide (résumé avec '-h')
# What -V and --version do.
help-version-flag = Afficher la version
# The help command, listed among the commands, and its argument.
help-command = Afficher ce message ou l’aide des commandes indiquées
help-command-subcommand = Afficher l’aide des commandes indiquées

## Mistakes on dmm-cli's and dmm-gui's command line: an unknown option, a
## missing command, a value the program doesn't take. Printed on the
## terminal as
##
##     error: invalid value 'xml' for '--format <FORMAT>'
##       [possible values: csv, json, replay]
##
##     Usage: dmm-cli read [OPTIONS]
##
##     For more information, try '--help'.
##
## The usage line's syntax stays English after its heading. Names in quotes
## ($arg, $value, $command, $names) are what the user typed or must type:
## they arrive as they are, sometimes coloured, and stay in the quotes. $arg
## is an option, often with its value's name, e.g. --format <FORMAT>.

# At the start of every message, as the terminal shows errors.
args-error = erreur :

# An option that takes no value after "=" was given without one.
args-equals-needed = un signe égal est nécessaire pour donner une valeur à '{ $arg }'
# An option was given without its value.
args-value-required = '{ $arg }' attend une valeur, mais aucune n’a été fournie
# A value the option doesn't take. A list of the ones it takes may follow
# on the next line (args-possible-values).
args-invalid-value = valeur '{ $value }' non valide pour '{ $arg }'
# The same, with the reason the value was refused. $reason may be English.
args-invalid-value-because = valeur '{ $value }' non valide pour '{ $arg }' : { $reason }
# Why a --device value was refused, as $reason above: no meter has that
# name.
device-unknown = appareil inconnu : consultez la liste avec --help
# The values an option takes, on a line of their own under the error. $values
# is the list, separated by commas.
args-possible-values = [valeurs possibles : { $values }]
# A command the program doesn't have, e.g. 'lst'.
args-unrecognized-subcommand = commande '{ $command }' inconnue
# Options that must be given, listed one per line under this.
args-missing-required = les arguments obligatoires suivants n’ont pas été fournis :
# The program was run without a command. $command is the program, e.g.
# dmm-cli. The commands follow on the next line (args-subcommands).
args-missing-subcommand = '{ $command }' attend une commande, mais aucune n’a été fournie
# The commands, on a line of their own under the error. $values is the list,
# separated by commas.
args-subcommands = [commandes : { $values }]
# A value after an option that takes no more.
args-unexpected-value = valeur '{ $value }' inattendue pour '{ $arg }' : aucune autre n’était attendue
# Too few values after an option. $needed and $given are numbers; $count is
# $given, for choosing the verb's form ([0] and [one]: one value or none).
args-too-few-values =
    { $count ->
        [one] '{ $arg }' attend { $needed } valeurs, mais seulement { $given } a été fournie
       *[other] '{ $arg }' attend { $needed } valeurs, mais seulement { $given } ont été fournies
    }
# The wrong number of values after an option. $needed and $given are
# numbers; $count is $given, for choosing the verb's form.
args-wrong-number-of-values =
    { $count ->
        [one] '{ $arg }' attend { $needed } valeurs, mais { $given } a été fournie
       *[other] '{ $arg }' attend { $needed } valeurs, mais { $given } ont été fournies
    }
# An option the program doesn't have, or one this command doesn't take.
args-unexpected-argument = argument '{ $arg }' inattendu
# An option given twice that can only be given once.
args-repeated = l’argument '{ $arg }' ne peut être utilisé qu’une fois
# Two options, or an option and a command, that can't go together. $what is
# argument or subcommand; $name and $other are what the user typed.
args-conflict-with =
    { $what ->
        [subcommand] la commande '{ $name }' ne peut pas être utilisée avec '{ $other }'
       *[argument] l’argument '{ $name }' ne peut pas être utilisé avec '{ $other }'
    }
# Two options, or an option and a command, that can't go together, with
# the others listed one per line under this. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-list =
    { $what ->
        [subcommand] la commande '{ $name }' ne peut pas être utilisée avec :
       *[argument] l’argument '{ $name }' ne peut pas être utilisé avec :
    }
# The same, when the others aren't named. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-others =
    { $what ->
        [subcommand] la commande '{ $name }' ne peut pas être utilisée avec un ou plusieurs des autres arguments indiqués
       *[argument] l’argument '{ $name }' ne peut pas être utilisé avec un ou plusieurs des autres arguments indiqués
    }

## Hints after the error.

# Before a hint, indented on its own line.
args-tip = astuce :
# A name close to what was typed. $what is subcommand, argument or value;
# $names is one or more quoted names, separated by commas; $count is how
# many.
args-similar =
    { $what ->
        [subcommand]
            { $count ->
                [one] une commande proche existe : { $names }
               *[other] des commandes proches existent : { $names }
            }
        [argument]
            { $count ->
                [one] un argument proche existe : { $names }
               *[other] des arguments proches existent : { $names }
            }
       *[value]
            { $count ->
                [one] une valeur proche existe : { $names }
               *[other] des valeurs proches existent : { $names }
            }
    }
# The last line. $help is --help.
args-more-info = Pour en savoir plus, essayez '{ $help }'.
