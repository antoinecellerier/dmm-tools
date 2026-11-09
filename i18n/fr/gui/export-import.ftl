### Import… and Export…, the first and last buttons of the row under the
### graph (assets/gui-wide-layout.webp): Import… opens a file saved
### earlier, the Export… split button saves the samples to a file, and its
### arrow drops a menu of the file types. Then the line under the row
### after an import, the prompt shown before something would discard
### samples or markers no file holds, the file types the system's open and
### save dialogs list, and the messages both buttons show when they are
### done.
###
### A sample is one reading the meter sent; a reading, in the import
### messages, is one row of the file. The buttons sit in one row with
### Record and Discard, so keep them short. CSV, JSON and replay are the
### file types; a replay file is one dmm-gui plays back as if the meter were
### attached. $keys is a keyboard shortcut as this computer writes it, e.g.
### Ctrl+E or ⌘E.

## The row of buttons under the graph.

# The first button of the row.
import-button = Importer…
    .hover = Ouvrir un fichier exporté au format CSV, JSON ou relecture ({ $keys })
# The left part of the Export… split button: saves a CSV file at once.
# $buffer is recording when a recording holds the samples, graph when
# nothing is recorded and Export… saves the samples the graph holds.
export-button = Exporter…
    .hover =
        { $buffer ->
            [recording] Exporter l’enregistrement dans un fichier CSV ({ $keys })
           *[graph] Exporter les échantillons du graphique dans un fichier CSV ({ $keys })
        }
# The arrow at the right of the Export… split button, which drops the menu
# of file types. It has no text. $buffer is as for Export….
export-menu =
    .hover =
        { $buffer ->
            [recording] Exporter l’enregistrement dans un fichier CSV, JSON ou de relecture
           *[graph] Exporter les échantillons du graphique dans un fichier CSV, JSON ou de relecture
        }
    .a11y = Type de fichier d’export

## The menu under the Export… arrow: one entry per file type. Each opens
## the save dialog for that type.

# CSV is the file type's name.
export-csv = CSV…
# JSON is the file type's name. dmm-cli read --format json is a command
# line, kept as it is.
export-json = JSON…
    .hover = Un objet par ligne, comme l’affiche dmm-cli read --format json
# A replay file. dmm-gui --replay is a command line, kept as it is; "as the
# meter" means as if it were the meter attached.
export-replay = Relecture…
    .hover = Un fichier que dmm-gui --replay rejoue comme si le multimètre était branché
# The Replay… entry's hover while it is greyed out, and the error shown if
# a replay is asked for anyway. The mock is the simulated meter the app
# offers for trying it out; its samples hold none of the bytes a real meter
# sends (its wire format), which a replay file is made of.
export-no-wire-format = Le multimètre simulé n’a pas de trames brutes à exporter. Connectez un vrai multimètre

## A small line under the buttons after Import…: the samples are the
## file's.

# $count is how many readings the file gave; $file the file's name, e.g.
# measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv. Export… is the button.
import-hint =
    { $count ->
        [one] { $count } mesure importée depuis { $file }. Utilisez « Exporter… » pour l’enregistrer avec vos marqueurs et votre vue.
       *[other] { $count } mesures importées depuis { $file }. Utilisez « Exporter… » pour les enregistrer avec vos marqueurs et votre vue.
    }

## The prompt shown over the window before something would discard samples
## or markers no file holds: Record over a recording that was never
## exported, Discard on a recording whose samples have left the graph,
## Import… or Connect while the session holds them. Markers are the
## numbered flags the user puts on readings, with a note each; "unsaved
## changes to a marker" means the marker, or its note, was added or changed
## since the last export.

# The prompt's title when samples would be lost (and maybe markers too).
export-unsaved-heading-samples = Abandonner les échantillons non exportés ?
# The prompt's title when only markers would be lost.
export-unsaved-heading-markers = Abandonner les marqueurs non exportés ?
# The prompt after Record, which starts a new recording in place of the
# stopped one. $lost is samples when only samples would be lost, markers
# when only markers, both when both. $samples is how many samples were
# never exported; $markers how many markers have unsaved changes. Markers
# stay on the graph's readings even though the recording goes.
export-unsaved-record =
    { $lost ->
        [samples]
            { $samples ->
                [one] Démarrer un nouvel enregistrement fera perdre { $samples } échantillon non exporté.
               *[other] Démarrer un nouvel enregistrement fera perdre { $samples } échantillons non exportés.
            }
        [markers]
            { $markers ->
                [one] Démarrer un nouvel enregistrement fera perdre les modifications non enregistrées de { $markers } marqueur. Les marqueurs restent sur le graphique tant qu’il contient leurs mesures.
               *[other] Démarrer un nouvel enregistrement fera perdre les modifications non enregistrées de { $markers } marqueurs. Les marqueurs restent sur le graphique tant qu’il contient leurs mesures.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Démarrer un nouvel enregistrement fera perdre { $samples } échantillon non exporté et les modifications non enregistrées de { $markers } marqueur. Les marqueurs restent sur le graphique tant qu’il contient leurs mesures.
                       *[other] Démarrer un nouvel enregistrement fera perdre { $samples } échantillon non exporté et les modifications non enregistrées de { $markers } marqueurs. Les marqueurs restent sur le graphique tant qu’il contient leurs mesures.
                    }
               *[other]
                    { $markers ->
                        [one] Démarrer un nouvel enregistrement fera perdre { $samples } échantillons non exportés et les modifications non enregistrées de { $markers } marqueur. Les marqueurs restent sur le graphique tant qu’il contient leurs mesures.
                       *[other] Démarrer un nouvel enregistrement fera perdre { $samples } échantillons non exportés et les modifications non enregistrées de { $markers } marqueurs. Les marqueurs restent sur le graphique tant qu’il contient leurs mesures.
                    }
            }
    }
# The prompt's button that goes ahead: discards, then starts recording.
export-unsaved-record-confirm = Abandonner et enregistrer
# The prompt after Discard, which drops a stopped recording; Export… then
# saves the samples the graph still holds, so only the ones that have
# scrolled out of it are lost. $lost, $samples and $markers are as for
# Record; "them" in the last sentences means those samples.
export-unsaved-discard =
    { $lost ->
        [samples]
            { $samples ->
                [one] Abandonner l’enregistrement fera perdre { $samples } échantillon sorti du graphique.
               *[other] Abandonner l’enregistrement fera perdre { $samples } échantillons sortis du graphique.
            }
        [markers]
            { $markers ->
                [one] Abandonner l’enregistrement fera perdre les modifications non enregistrées de { $markers } marqueur placé sur des mesures sorties du graphique.
               *[other] Abandonner l’enregistrement fera perdre les modifications non enregistrées de { $markers } marqueurs placés sur des mesures sorties du graphique.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Abandonner l’enregistrement fera perdre { $samples } échantillon sorti du graphique, et les modifications non enregistrées de { $markers } marqueur placé dessus.
                       *[other] Abandonner l’enregistrement fera perdre { $samples } échantillon sorti du graphique, et les modifications non enregistrées de { $markers } marqueurs placés dessus.
                    }
               *[other]
                    { $markers ->
                        [one] Abandonner l’enregistrement fera perdre { $samples } échantillons sortis du graphique, et les modifications non enregistrées de { $markers } marqueur placé dessus.
                       *[other] Abandonner l’enregistrement fera perdre { $samples } échantillons sortis du graphique, et les modifications non enregistrées de { $markers } marqueurs placés dessus.
                    }
            }
    }
# The prompt's button that goes ahead: drops the recording.
export-unsaved-discard-confirm = Abandonner l’enregistrement
# The prompt after picking a file to import, which replaces everything the
# session holds. $lost, $samples and $markers are as for Record.
export-unsaved-import =
    { $lost ->
        [samples]
            { $samples ->
                [one] L’importation remplacera cette session et fera perdre { $samples } échantillon non exporté.
               *[other] L’importation remplacera cette session et fera perdre { $samples } échantillons non exportés.
            }
        [markers]
            { $markers ->
                [one] L’importation remplacera cette session et fera perdre les modifications non enregistrées de { $markers } marqueur.
               *[other] L’importation remplacera cette session et fera perdre les modifications non enregistrées de { $markers } marqueurs.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] L’importation remplacera cette session et fera perdre { $samples } échantillon non exporté et les modifications non enregistrées de { $markers } marqueur.
                       *[other] L’importation remplacera cette session et fera perdre { $samples } échantillon non exporté et les modifications non enregistrées de { $markers } marqueurs.
                    }
               *[other]
                    { $markers ->
                        [one] L’importation remplacera cette session et fera perdre { $samples } échantillons non exportés et les modifications non enregistrées de { $markers } marqueur.
                       *[other] L’importation remplacera cette session et fera perdre { $samples } échantillons non exportés et les modifications non enregistrées de { $markers } marqueurs.
                    }
            }
    }
# The prompt's button that goes ahead: discards, then imports the file.
export-unsaved-import-confirm = Abandonner et importer
# The prompt after Connect while an imported file is open: the meter's
# session takes its place. $lost, $samples and $markers are as for Record.
export-unsaved-connect =
    { $lost ->
        [samples]
            { $samples ->
                [one] La connexion mettra fin à la session importée et fera perdre { $samples } échantillon non exporté.
               *[other] La connexion mettra fin à la session importée et fera perdre { $samples } échantillons non exportés.
            }
        [markers]
            { $markers ->
                [one] La connexion mettra fin à la session importée et fera perdre les modifications non enregistrées de { $markers } marqueur.
               *[other] La connexion mettra fin à la session importée et fera perdre les modifications non enregistrées de { $markers } marqueurs.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] La connexion mettra fin à la session importée et fera perdre { $samples } échantillon non exporté et les modifications non enregistrées de { $markers } marqueur.
                       *[other] La connexion mettra fin à la session importée et fera perdre { $samples } échantillon non exporté et les modifications non enregistrées de { $markers } marqueurs.
                    }
               *[other]
                    { $markers ->
                        [one] La connexion mettra fin à la session importée et fera perdre { $samples } échantillons non exportés et les modifications non enregistrées de { $markers } marqueur.
                       *[other] La connexion mettra fin à la session importée et fera perdre { $samples } échantillons non exportés et les modifications non enregistrées de { $markers } marqueurs.
                    }
            }
    }
# The prompt's button that goes ahead: discards, then connects the meter.
export-unsaved-connect-confirm = Abandonner et connecter
# The prompt's other button, which keeps everything; it has the focus.
export-unsaved-cancel = Annuler

## The file type the system's open and save dialogs list, next to the
## file's extension (.csv, .json, .replay).

# Import…'s open dialog: every type it reads.
import-file-type = Mesures exportées
# Export…'s save dialog after CSV…; CSV is the file type's name.
export-file-type-csv = CSV
# After JSON…; JSON is the file type's name.
export-file-type-json = JSON
# After Replay…: a replay file.
export-file-type-replay = Relecture

## Messages that show for a few seconds at the top right.

# While a large file is read in. $file is the file's name; $percent how
# much of it is in so far, a whole number from 0 to 99.
import-progress = Importation de { $file }… { $percent } %
# When the file is in. $count is how many readings it gave; $file is its
# name.
import-done =
    { $count ->
        [one] { $count } mesure importée depuis { $file }
       *[other] { $count } mesures importées depuis { $file }
    }
# When the file holds more readings than the Buffer size setting (in
# Settings) lets the app keep: the first ones are in, the rest are not.
# $count is how many came in; $left how many were left out; $file is the
# file's name.
import-done-partial =
    { $count ->
        [one]
            { $left ->
                [one] La première mesure de { $file } a été importée ; { $left } mesure a été ignorée, car la taille du tampon définie dans Paramètres est atteinte
               *[other] La première mesure de { $file } a été importée ; { $left } mesures ont été ignorées, car la taille du tampon définie dans Paramètres est atteinte
            }
       *[other]
            { $left ->
                [one] Les { $count } premières mesures de { $file } ont été importées ; { $left } mesure a été ignorée, car la taille du tampon définie dans Paramètres est atteinte
               *[other] Les { $count } premières mesures de { $file } ont été importées ; { $left } mesures ont été ignorées, car la taille du tampon définie dans Paramètres est atteinte
            }
    }
# The file was read but has no readings in it. $file is its name.
import-empty = { $file } ne contient aucune mesure
# The file could not be read. $path is where it is; $reason why, in the
# words of the file reader or the system, often in English.
import-failed = Échec de l’importation : { $path } : { $reason }
# Reading the file stopped on an internal error. $path is where it is.
import-failed-crashed = Échec de l’importation : { $path } : erreur interne
# The file dialog of Import… failed.
import-dialog-failed = Échec de l’importation
# After Export… saved the file. $count is how many samples it holds; $file
# its name, e.g. measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv.
export-done =
    { $count ->
        [one] { $count } échantillon exporté vers { $file }
       *[other] { $count } échantillons exportés vers { $file }
    }
# After Export… with nothing to save: the recording is running or stopped
# but empty.
export-nothing-recorded = Rien à exporter : l’enregistrement ne contient encore aucun échantillon
# After Export… with nothing to save and no meter connected.
export-nothing-connect = Rien à exporter : connectez d’abord un multimètre
# After Export… with nothing to save: a meter is connected but has sent
# nothing yet.
export-nothing-yet = Rien à exporter : aucune mesure pour l’instant
# After Export… while an earlier export still waits on its save dialog or
# is being written.
export-in-progress = Un export est déjà en cours : terminez-le d’abord
# The file could not be written. $reason is why, in the words of the
# system or the file writer, often in English.
export-failed = Échec de l’export : { $reason }
# The samples to export were dropped while the save dialog was open, so
# the file would not hold what was asked for.
export-failed-changed = Échec de l’export : les échantillons ont changé pendant que la boîte de dialogue d’enregistrement était ouverte
