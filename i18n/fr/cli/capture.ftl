### What 'dmm-cli capture' prints around its steps: the guided run a user
### makes to send a report about their meter. It walks the meter through a
### list of steps (set the dial to DC V, press HOLD…), reads what the meter
### sends at each, and writes a report file to attach to an issue on GitHub.
### These lines go to the terminal's error stream.
###
### Each step's own instruction, the readings (5.5687 V [AUTO]) and the
### report file stay as they are, in English: a maintainer reads the report.
### A user may translate the instructions for themselves: see
### --export-instructions below.
### Step ids (dcv, hold_on, extra_0), setting names (range, hold) and words
### in --flag form are typed or read back by those names and stay as they
### are. The keys the user presses to answer (s, q, r, n, y) arrive as
### variables: the run only understands those letters, so they can't be
### changed. "Enter" is the key's name and can be translated.

## Checking that the meter answers, before anything else.

# Dim, while the tool asks the meter for its name.
capture-checking = Vérification de la communication avec le multimètre…
# Yellow and bold. The cable or Bluetooth link opened, but the meter sent
# nothing. $link is usb (a USB cable), adapter (a Bluetooth adapter plugged
# into the meter), radio (the meter's own Bluetooth) or other.
capture-not-responding =
    { $link ->
        [usb] Le câble USB est connecté, mais le multimètre ne répond pas.
        [adapter] L’adaptateur Bluetooth est connecté, mais le multimètre ne répond pas.
        [radio] La liaison Bluetooth est établie, mais le multimètre ne répond pas.
       *[other] La liaison est établie, mais le multimètre ne répond pas.
    }
# Followed by the meter's own steps for switching its data transmission on.
capture-enable-transmission = Pour activer la transmission des données :
capture-run-again = Relancez ensuite cette commande.
# What the meter sent was kept in a file of its own, as evidence. $bytes is
# how many bytes the meter sent, $path the file.
capture-no-response-saved = Octets reçus : { $bytes }, enregistrés dans { $path }
# $url is the web address of the meter's issue on GitHub.
capture-no-response-attach = Si le multimètre est allumé et ne répond toujours pas, joignez ce fichier au ticket { $url }
# $error is why the file could not be written.
capture-no-response-not-saved = Impossible d’enregistrer les octets reçus : { $error }

## The banner once the meter answers, after the lines naming the meter
## (connect.ftl). Each support level is what follows "Status:"; the line
## under the partly verified and experimental ones is indented under it.

# $status is one of the capture-status-* levels below, coloured.
capture-status = État : { $status }
capture-status-verified = modèle pris en charge
capture-status-partly-verified = partiellement vérifié : connexion et modes principaux confirmés
capture-status-partly-verified-detail = Cette exécution sert à confirmer les modes qui ne le sont pas encore.
# The protocol was worked out from the meter's traffic, not from its maker.
capture-status-experimental = expérimental : protocole obtenu par rétro-ingénierie, non confirmé
capture-status-experimental-detail = Effectuez autant d’étapes que possible et partagez le rapport.

## The meter names another model than the one the run opened (a CMS061
## opened as a CMS101): asked before the report file is touched, so the
## operator can run again with the right meter. $named and $opened are
## meter names; $id is the --device id that opens the named one.

# Yellow and bold, after the status banner.
capture-other-model = Le multimètre s’identifie comme le modèle { $named }, alors que cette exécution a ouvert le modèle { $opened }.
capture-other-model-detail = Les étapes et les touches à distance utilisées seraient celles du modèle { $opened }.
# $yes and $no are the keys that answer: y and n. Any other key quits.
capture-other-model-confirm = Capturer quand même en tant que modèle { $opened } ? { $yes }/{ $no } :
capture-other-model-rerun = Relancez la capture avec --device { $id }.

## A report from an earlier run is already in the file: resume it, start
## again or stop. Each prompt waits for one key, after a space.

# $path is the report file. $captured and $skipped count its steps.
capture-found-existing = Capture existante trouvée : { $path } (étapes capturées : { $captured }, ignorées : { $skipped })
# $resume, $fresh and $abort are the keys that answer: r, n and q.
capture-resume-prompt = { $resume }=reprendre, { $fresh }=recommencer, { $abort }=abandonner :
capture-aborted = Abandonné.
# $yes and $no are the keys that answer: y and n.
capture-overwrite-confirm = La capture existante sera écrasée. Êtes-vous sûr ? { $yes }/{ $no } :
capture-resuming = Reprise : les étapes déjà capturées seront ignorées.
# The file is there but isn't a report this tool can read. $path is the
# file.
capture-unreadable = Fichier { $path } trouvé, mais impossible de l’analyser.
# $yes is the key that answers: y.
capture-overwrite-unreadable = Écraser ? { $yes }=recommencer, toute autre touche=abandonner :
# $path is the report file the run writes.
capture-output-file = Fichier de sortie : { $path }

## What the steps ask for besides the meter and its test leads, listed
## before the first step so the user can gather it all at once. Each item is
## on a line of its own after its number in brackets.

capture-needs-heading = Vous aurez besoin de :
# $item is one of the capture-need-* items below; $steps lists the ids of
# the steps that need it, e.g. "dcv_short, ohm_short".
capture-needs-item = { $item }  (étapes : { $steps })
# The user types the numbers in brackets of what they don't have, or just
# presses Enter.
capture-needs-prompt = Numéros de ce qui vous manque (Entrée = rien ne manque) :
# What the user typed at a numbers prompt (this one or the review's) isn't
# one of the numbers listed. $token is what they typed, in quotes; $max the
# last number listed.
capture-not-a-number = { $token } n’est pas un nombre entre 1 et { $max }
capture-need-shorted-leads = cordons court-circuités
capture-need-dc-source = pile ou autre source de tension continue
capture-need-thermocouple = thermocouple de type K
# For the non-contact voltage detector, which senses a live wire nearby.
capture-need-live-wire = fil secteur sous tension à proximité
# The meter's power-measurement adapter, plugged into a live outlet with a
# load plugged into it.
capture-need-power-adapter = adaptateur de mesure de puissance, prise sous tension et charge
capture-need-transistor = transistor
capture-need-scr = SCR (thyristor)
capture-need-loop-source = boucle 4-20 mA et son alimentation 15-48 V
capture-need-power-module = module de mesure de puissance en courant continu ou USB, alimentation et charge
# LX-925 is the adapter's name.
capture-need-pv-adapter = adaptateur d’irradiance LX-925

## The steps. Each opens with its id in brackets and its instruction, and
## the tool watches the meter until it shows what the step asks for.

# Bold, between heavy lines.
capture-modes-heading = Modes de mesure
# Dim, under the heading. $skip and $quit are the keys: s and q.
capture-modes-keys = chaque étape est capturée automatiquement une fois le multimètre stabilisé : Entrée=capturer maintenant, { $skip }=passer une étape, { $quit }=passer à la fin et enregistrer
# Dim, after a step's instruction: the step the rest of the run relies on.
capture-gate-mark = (étape clé)
# A resumed run passes over a step the report already has. $step is the
# step's id.
capture-already-captured = étape { $step } déjà capturée, ignorée
# Dim, once the tool has waited a while. $skip and $quit are the keys: s and
# q.
capture-press-enter = Appuyez sur Entrée quand le multimètre est prêt ({ $skip }=passer, { $quit }=terminer)
# Dim, on a single line the run keeps rewriting while it waits for the meter.
# $reading is the latest reading (5.5687 V [AUTO]); $frame counts the meter's
# replies so far.
capture-progress-watching = surveillance du multimètre · { $reading } · trame { $frame }
# Dim, on that same line while a step's readings are taken: $done of $total
# ("reading 4 of 12"), and $reading the latest one.
capture-progress-reading = mesure { $done } sur { $total } · { $reading }
# Dim: a step that presses a button on the meter by remote control holds it
# until the meter shows the state the instruction asks for. $skip and $quit
# are the keys: s and q.
capture-wait-start = Réglez le multimètre comme indiqué ci-dessus : la commande sera envoyée dès qu’il sera prêt (Entrée=envoyer maintenant, { $skip }=passer, { $quit }=terminer)
# Dim: the meter left the state the step starts from just before the key
# went out, so the step waits for it again.
capture-wait-moved = Le multimètre a quitté l’état attendu avant l’envoi de la commande : attente du retour à cet état
# When the terminal can't be read key by key. $skip and $quit are the keys:
# s and q.
capture-any-key = toute touche=capturer, { $skip }=passer, { $quit }=terminer :
# Red: the button the step presses on the meter by remote control was
# refused. $error is why.
capture-command-failed = Échec de la commande : { $error }
# Yellow: the tool pressed a button on the meter by remote control and the
# meter didn't change. $command is the button's name as the tool sends it,
# e.g. hold; $reading what the meter shows, e.g. 5.5687 V [AUTO].
capture-did-nothing-shows = la commande { $command } n’a eu aucun effet : le multimètre affiche toujours { $reading }
# Yellow: the tool pressed a button on the meter by remote control and the
# meter changed, but not to what the step waits for. $command is the button's
# name as the tool sends it, e.g. hold; $reading what the meter shows, e.g.
# 0.00 VA [HOLD].
capture-command-missed = la commande { $command } n’a pas mis le multimètre dans l’état attendu par cette étape : il affiche maintenant { $reading }
# $command is the button's name as the tool sends it, e.g. hold.
capture-did-nothing-silent = la commande { $command } n’a eu aucun effet : le multimètre n’a envoyé aucune mesure
# Yellow: the meter changed function while the step was reading it, so the
# step starts over. $mode is the function the step is for, $other the one
# the meter went to, e.g. Ω and Continuity.
capture-left-mode = le multimètre est passé du mode { $mode } au mode { $other } pendant l’échantillonnage : nouvel essai…
# The reading the step took, in green. $reading is it, e.g. 5.5687 V [AUTO].
capture-we-read = Mesure lue : { $reading }
# Something the meter sent couldn't be read; the step goes on. $error is
# why.
capture-frame-error = erreur : { $error }
# Yellow: the step read nothing from the meter.
capture-no-response = Pas de réponse du multimètre.
# Yellow, before capture-few-samples on the same line.
capture-few-samples-label = avertissement :
# The step got fewer readings than it asks for. $got and $wanted count them.
capture-few-samples = seulement { $got }/{ $wanted } échantillons reçus

## Dim, while a step waits: the meter settled, but not in what the step
## expects. Each says what the reading has and what the step wants.

# $got and $want are functions, in quotes, e.g. "DC V" and "AC V".
capture-mismatch-mode = le multimètre affiche le mode { $got }, l’étape attend le mode { $want }
# $flag is a status flag the meter shows, e.g. HOLD. $want says whether the
# step wants it on or off; the meter has it the other way.
capture-mismatch-flag =
    { $want ->
        [on] { $flag } est désactivé sur le multimètre, l’étape l’attend activé
       *[off] { $flag } est activé sur le multimètre, l’étape l’attend désactivé
    }
# $range is the reading's range, e.g. 22V. $shown is none (the reading has
# no range), auto (a range the meter picked itself) or plain. $want is auto
# (the step wants the meter to pick the range itself) or manual (the user
# picks it).
capture-mismatch-range =
    { $want ->
        [auto]
            { $shown ->
                [none] le multimètre n’affiche aucune plage, l’étape attend la plage automatique
                [auto] le multimètre affiche la plage { $range } (automatique), l’étape attend la plage automatique
               *[plain] le multimètre affiche la plage { $range }, l’étape attend la plage automatique
            }
       *[manual]
            { $shown ->
                [none] le multimètre n’affiche aucune plage, l’étape attend une plage manuelle
                [auto] le multimètre affiche la plage { $range } (automatique), l’étape attend une plage manuelle
               *[plain] le multimètre affiche la plage { $range }, l’étape attend une plage manuelle
            }
    }
# $value is what the meter shows, e.g. 12.345 or OL; $shown is absent when
# the reading has no main value, else value. $want is the kind of reading the
# step wants: overload (OL, the meter's over-range sign), negative, finite
# (any number), ncv (the non-contact voltage detector's level 1 or more),
# word (a word the meter shows instead of a number) or zero.
capture-mismatch-value =
    le multimètre affiche { $shown ->
        [absent] aucune mesure principale
       *[value] la valeur { $value }
    }, l’étape attend { $want ->
        [overload] OL
        [negative] une mesure négative
        [ncv] un niveau NCV de 1 ou plus
        [word] un mot au lieu d’une mesure
        [zero] une mesure nulle
       *[finite] une mesure numérique
    }
# $value and $shown as in capture-mismatch-value. $min is the smallest
# number the step wants, positive or negative, e.g. 1.
capture-mismatch-at-least =
    le multimètre affiche { $shown ->
        [absent] aucune mesure principale
       *[value] la valeur { $value }
    }, l’étape attend au moins { $min } en valeur absolue

## Asking whether the reading matches the meter's screen. Each prompt waits
## for a line, after a space.

# Dim. $retake is yes when the step can be done over. $no and $redo are the
# keys that answer: n and r.
capture-confirm =
    { $retake ->
        [yes] Le multimètre affichait-il cette valeur ? (Entrée = oui, { $no } = non, { $redo } = refaire)
       *[no] Le multimètre affichait-il cette valeur ? (Entrée = oui, { $no } = non)
    }
# Yellow, when the answer was none of those. $retake, $no and $redo as in
# capture-confirm.
capture-confirm-again =
    { $retake ->
        [yes] Répondez { $no } ou { $redo }, ou appuyez sur Entrée.
       *[no] Répondez { $no }, ou appuyez sur Entrée.
    }
# The reading didn't match: the user types what the meter showed.
capture-what-shown = Qu’affichait le multimètre ?
# Dim: the user typed nothing at capture-what-shown.
capture-recorded-mismatch = enregistré : la mesure ne correspondait pas
# Dim. $shown is what the user typed, in quotes.
capture-recorded-shown = enregistré : le multimètre affichait { $shown }
# Dim: the step starts over.
capture-retaking = nouvel essai…

## After the first steps, which check that the tool reads the meter right
## ("core semantics": mode, range and flags).

# Green.
capture-core-confirmed = Lecture du mode, de la plage et des indicateurs confirmée : les étapes restantes seront vérifiées à la fin.
# Yellow. $steps lists the ids of the steps that failed, e.g. "dcv, ohm".
capture-core-not-confirmed = Lecture du mode, de la plage et des indicateurs non confirmée ({ $steps }) : chaque étape demandera une confirmation.

## Once the core semantics are confirmed, the tool sets ranges and flags on
## the meter itself by remote control, and prints a dim line for each. Each
## starts with an arrow.

# $mode is the function, e.g. Continuity.
capture-drive-switching = passage en mode { $mode } (envoyé par l’outil)
# $mode is the function, e.g. Continuity; $error why the meter refused.
capture-drive-switch-failed = impossible de passer en mode { $mode } ({ $error }) : faites-le à la main
# $setting is the setting's name, rel (relative mode): the meter refuses it
# while it shows OL, its over-range sign.
capture-drive-rel-skipped-ol = réglage { $setting } ignoré : la mesure affiche OL
# $setting is the setting's name, rel (relative mode).
capture-drive-rel-skipped-blank = réglage { $setting } ignoré : aucune mesure
# In place of a reading, after the setting and value tried, e.g.
# "range:22V".
capture-drive-no-reading = aucune mesure
# Yellow. $setting is the setting's name, e.g. range or hold.
capture-drive-reset-failed = impossible de réinitialiser le réglage { $setting } : appuyez sur la touche correspondante du multimètre
# Yellow: the meter refused too many of the tool's commands.
capture-drive-unreliable = commande à distance peu fiable sur ce multimètre : plus aucun balayage automatique

## The end-of-run review: the readings the run took without asking, each
## with its number, to check against what the meter showed.

# Bold, between heavy lines.
capture-review-heading = Vérification des mesures
# The user types the numbers of the wrong readings, or just presses Enter.
capture-review-prompt = Numéros des mesures qui ne correspondaient PAS à l’écran du multimètre (Entrée = toutes correctes) :

## The freeform pass: the user sets the meter to anything the steps didn't
## cover and describes it.

# Bold, between heavy lines; "Part 4" is the run's last part.
capture-freeform-heading = Partie 4 : captures supplémentaires (facultative)
capture-freeform-intro = Réglez le multimètre sur tout mode ou état non couvert ci-dessus.
# After the capture's id in brackets, e.g. [extra_0]; waits for a line.
# $quit is the word that ends the pass: q.
capture-freeform-prompt = Décrivez le réglage du multimètre (ou '{ $quit }' pour terminer) :

## The last check: does the tool's automatic detection recognise the
## meter? The user switches it off and on, and the tool detects it afresh.

# Dim. $bridge is the chip in the cable or the Bluetooth link, e.g. CP2110.
capture-detect-confirmed = La détection automatique est déjà confirmée via { $bridge } : sa vérification est ignorée.
# Bold, between heavy lines.
capture-detect-heading = Dernière vérification : détection automatique
capture-detect-off-cable = Éteignez le multimètre et débranchez son câble USB.
capture-detect-off = Éteignez le multimètre.
# Waits for one key. $skip is the key that skips: s.
capture-detect-off-prompt = Appuyez sur Entrée une fois qu’il est éteint, ou sur { $skip } pour passer :
# $mode is the function to turn the dial to, DC V.
capture-detect-on-cable = Rebranchez le câble et rallumez le multimètre en mode { $mode }.
# $mode is the function to turn the dial to, DC V.
capture-detect-on = Rallumez-le en mode { $mode }.
# Followed by the meter's own steps for switching its data transmission on.
capture-detect-transmission = Si le multimètre a désactivé la transmission des données, réactivez-la :
# Waits for one key. $skip is the key that skips: s.
capture-detect-ready-prompt = Appuyez sur Entrée quand il est prêt, ou sur { $skip } pour passer :
# Dim.
capture-detect-checking = Vérification de la détection automatique…
# Detection found the meter; the tool reads it once more. $instruction is
# what to do, the step's own instruction, in English.
capture-detect-now = Maintenant : { $instruction }
# Dim. $skip is the key that skips: s.
capture-detect-read-keys = La mesure est prise une fois le multimètre stabilisé : Entrée pour la prendre tout de suite, { $skip } pour passer.

## The line the check ends on: dim when detection found the meter, yellow
## with advice when it didn't. $found is the model detection picked, e.g.
## UNI-T UT61E+; $reading what the meter then read, e.g. 5.5687 V [AUTO].
## The advice names the meter the run was for: $device is its id on the
## command line, e.g. ut61eplus, and $meter its name, e.g. UNI-T UT61E+.

# $reason is why the link didn't open again, in English, or
# capture-detect-no-reason.
capture-detect-not-reopened = La détection automatique n’a pas été vérifiée : la liaison ne s’est pas rouverte ({ $reason }).
capture-detect-no-reason = aucune raison indiquée
capture-detect-found = La détection automatique a trouvé le multimètre { $found }.
capture-detect-found-read = La détection automatique a trouvé le multimètre { $found } et a lu { $reading }.
capture-detect-found-read-other = La détection automatique a trouvé le multimètre { $found }, mais a ensuite lu { $reading } : continuez à utiliser --device { $device }, ou choisissez le multimètre { $meter } dans dmm-gui.
capture-detect-found-read-nothing = La détection automatique a trouvé le multimètre { $found }, mais n’a ensuite rien lu : continuez à utiliser --device { $device }, ou choisissez le multimètre { $meter } dans dmm-gui.
capture-detect-picked-other = La détection automatique a pris ce multimètre pour le modèle { $found } : continuez à utiliser --device { $device }, ou choisissez le multimètre { $meter } dans dmm-gui.
capture-detect-not-found = La détection automatique n’a pas trouvé ce multimètre : continuez à utiliser --device { $device }, ou choisissez le multimètre { $meter } dans dmm-gui.

## The end of the run.

# After "Warning:" (common.ftl): the detection check stopped. $error is why.
capture-detect-failed = vérification de la détection automatique : { $error }
# Bold and green, between equals signs.
capture-complete = Capture terminée !
# $path is the report file, in bold.
capture-report-saved = Rapport enregistré : { $path }
# The run followed a list of steps a maintainer wrote (a "plan"). $plan is
# its file; $covered and $total count its steps.
capture-plan-covered = Plan { $plan } : { $covered } des { $total } étapes capturées
capture-nothing-new = Aucune étape non vérifiée n’a été capturée : rien de nouveau à signaler.
# $covered and $total count the meter's steps that hardware hasn't confirmed
# yet; $meter is its name, e.g. UNI-T UT61E+.
capture-covered = Étapes non vérifiées couvertes pour le multimètre { $meter } : { $covered } sur { $total }.
# $resume is the key that resumes, r (capture-resume-prompt).
capture-not-finished = Capture inachevée : relancez la même commande et répondez { $resume } pour reprendre.
# $flag is the option as typed, --steps <ids>.
capture-steps-hint = { $flag } n’exécute que les étapes indiquées.
# $url is the web address of the meter's issue on GitHub.
capture-attach = Joignez le rapport au ticket { $url }

## --export-instructions: the step instructions written to a file the user
## translates for themselves, then passes back with --instructions. The
## file is Fluent source, like this one.

# The comment at the top of the exported file, a line of it per line here.
# $meter is the meter's name, e.g. Brymen BM829s/BM827s/BM822s/BM821s.
# Keep "=", "-device" and "#" as they are: they are the file's
# syntax.
capture-instructions-header =
    Instructions des étapes de capture du multimètre { $meter }, à traduire.
    Remplacez le texte après chaque « = » par votre traduction, sur la même ligne.
    Gardez tels quels l’identifiant d’étape avant le « = », la ligne « -device »
    et les lignes qui commencent par « # ». Une étape supprimée s’affiche en anglais.
# A line of the header, when the steps quote words from the meter: its
# key labels and what its screen shows. $marks lists them, e.g. "AC, HOLD,
# MIN/MAX, SELECT". A translation, by a person or a machine, must leave them
# as they are, so say so plainly.
capture-instructions-keep = Gardez ces termes tels quels, comme le multimètre les affiche sur ses touches et son écran : { $marks }
# The header's last line. $command is the command that runs the capture
# with the translated file.
capture-instructions-run = Lancez ensuite : { $command }
# In place of writing a file, for a meter with no steps of its own (a
# simulated one). $meter is its name, e.g. Mock UT61E+.
capture-instructions-none = Le multimètre { $meter } n’a pas ses propres étapes de capture : rien à traduire.
# After writing the file. $count is the number of steps; $path the file.
capture-instructions-written =
    { $count ->
        [one] { $count } instruction d’étape écrite dans { $path }.
       *[other] { $count } instructions d’étape écrites dans { $path }.
    }
# $command is the command that runs the capture with the translated file.
capture-instructions-next = Traduisez-les, puis lancez : { $command }
# Yellow, before the first step: steps whose English changed after the
# file was exported, so they show a translation of an older wording.
# $path is the file; $steps lists the step ids, e.g. "dcv, hold".
capture-instructions-changed = Ces étapes ont changé depuis l’export de { $path } et affichent votre traduction de leur ancien texte : { $steps }. Exportez un nouveau fichier pour voir le nouveau texte anglais.
# One problem in the file, on a line of its own under
# capture-error-instructions-unreadable. $line is a line number.
capture-instructions-problem-syntax = ligne { $line } : un identifiant d’étape suivi de « = » et de son texte est attendu
# Another problem. $id is a step id, e.g. dcv.
capture-instructions-problem-not-text = { $id } : le texte contient des accolades, supprimez-les
# Another problem. $id is a step id, e.g. dcv.
capture-instructions-problem-twice = { $id } : l’étape figure deux fois
# Another problem: the file couldn't be read at all.
capture-instructions-problem-unparsable = le fichier est illisible, vérifiez ses accolades et ses guillemets

## The line that closes a failed run, after "Error:" (common.ftl).

capture-error-not-responding = le multimètre ne répond pas
capture-error-input-gone = la saisie au clavier ne fonctionne plus : terminez la capture et relancez-la
# --steps named steps this meter doesn't have. $steps lists them, e.g.
# "dcmv, tmep"; $command is the command line that lists the meter's steps.
capture-error-unknown-steps =
    étape(s) de capture inconnue(s) : { $steps }
    Lancez { $command } pour voir les étapes que cet appareil prend en charge.
# The --instructions file can't be read. $path is the file; $problems the
# problems, one per line (capture-instructions-problem-*).
capture-error-instructions-unreadable =
    impossible de lire { $path } comme instructions de capture :
    { $problems }
# The --instructions file was exported for another meter. $path is the
# file; $named the meter id in it, e.g. ut61eplus; $device this run's, e.g.
# bm82x; $command the command that exports a file for this meter.
capture-error-instructions-device = { $path } contient les instructions de l’appareil { $named }, et non celles de l’appareil { $device }. Exportez-les pour ce multimètre avec { $command }.
# The --instructions file names steps this run doesn't have, most likely a
# step id that was translated. $path is the file; $ids the ids not found,
# e.g. "vcc, retencao"; $known the ids the run has, e.g. "dcv, acv, hold".
capture-error-instructions-ids = { $path } mentionne des étapes absentes de cette exécution : { $ids }. Gardez chaque identifiant d’étape tel qu’il a été exporté. Étapes disponibles : { $known }.
