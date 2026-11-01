### The colour and theme rows near the foot of the settings panel, which
### the ⚙ button at the top bar's right end opens (assets/gui-settings.webp):
### the Theme and Colors rows of chips, the Customize colors section with a
### swatch for every colour of the palette and the editor each swatch opens,
### and Save as theme with the messages it shows when it is done.
###
### Theme names (Midnight, Bubble Gum, a theme the user saved) are kept as
### they are: they are file names and command-line values too. Text in
### parentheses starting with -- names a command-line option, kept as it
### is, that set the row for this session only. Rows wrap on a narrow
### window, so keep the chips and swatch names short.

## The Theme row: the Dark, Light and System modes, then the named themes.

colors-theme = Thème :
# The Dark, Light and System chips. $state is pinned when --theme picked
# this mode for the session, customized when the user changed some of its
# colours (Dark and Light only), plain otherwise.
colors-theme-dark =
    { $state ->
        [pinned] Sombre (--theme)
        [customized] Sombre (personnalisé)
       *[plain] Sombre
    }
    .hover = Utiliser le mode sombre pour toute l’interface
# $state as for Dark.
colors-theme-light =
    { $state ->
        [pinned] Clair (--theme)
        [customized] Clair (personnalisé)
       *[plain] Clair
    }
    .hover = Utiliser le mode clair pour toute l’interface
# $state as for Dark, never customized: System has no colours of its own.
colors-theme-system =
    { $state ->
        [pinned] Système (--theme)
       *[plain] Système
    }
    .hover = Suivre le réglage clair/sombre du bureau (Sombre s’il n’en indique aucun)
# A named theme's chip when --theme picked it for the session; otherwise
# the chip is the theme's name alone. $name is the theme's name, e.g.
# Midnight.
colors-theme-named-pinned = { $name } (--theme)
# A named theme's chip after the user changed some of its colours. $name is
# the theme's name.
colors-theme-named-customized = { $name } (personnalisé)
# A built-in theme's hover. $mode is dark or light: the mode the theme is
# drawn in.
colors-theme-builtin =
    .hover =
        { $mode ->
            [light] Thème clair intégré
           *[dark] Thème sombre intégré
        }
# The hover of a theme the user added to the themes folder. $mode is dark
# or light; $path is the theme's file, in full.
colors-theme-file =
    .hover =
        { $mode ->
            [light] Thème clair, depuis { $path }. Supprimez le fichier pour le retirer.
           *[dark] Thème sombre, depuis { $path }. Supprimez le fichier pour le retirer.
        }
# The same when the themes folder can't be found, with $mode as there.
# $file is the theme's file name.
colors-theme-file-unplaced =
    .hover =
        { $mode ->
            [light] Thème clair, depuis { $file }
           *[dark] Thème sombre, depuis { $file }
        }

## After the Theme row's chips: the theme files in the themes folder that
## could not be offered, with why on hover, one file to a line.

# $count is how many files were skipped.
colors-skipped =
    { $count ->
        [one] { $count } fichier de thème ignoré
       *[other] { $count } fichiers de thème ignorés
    }
# The hover's first line. $folder is the themes folder, in full.
colors-skipped-folder = Dans { $folder } :
# A line of the hover. $file is a file name; $reason one of the reasons
# below, starting lower case.
colors-skipped-file = { $file } : { $reason }
# The line for the files left unread because the folder holds too many.
# $count is how many; $max how many files are read, e.g. 100.
colors-skipped-too-many =
    { $count ->
        [one] { $count } autre : limite de { $max } fichiers dépassée
       *[other] { $count } autres : limite de { $max } fichiers dépassée
    }

## Why a theme file can't be used: after its name in the hover above, or
## after "Not saved:" when Save as theme would write such a file. Each
## starts lower case. Others come from the file system or the file reader
## and are shown in their own words.

colors-issue-not-a-file = pas un fichier
# $size is the largest file read, in KiB (kibibytes), e.g. 64.
colors-issue-too-big = plus de { $size } Kio
colors-issue-name-empty = le nom est vide
# $max is the most characters a name may have, e.g. 32.
colors-issue-name-too-long = le nom dépasse { $max } caractères
# Invisible characters, or $ or `.
colors-issue-name-unusable = le nom contient des caractères interdits dans un nom de thème
# The theme's name is Dark, Light or System, which the Theme row's own
# chips go by. $name is the name as the file has it; put it in your
# language's quotes.
colors-issue-name-reserved = « { $name } » est réservé à la ligne Thème
# $what is text or button: the theme's text on its background, or its
# button captions on the button.
colors-issue-not-opaque =
    { $what ->
        [button] le texte du bouton n’est pas opaque sur le bouton
       *[text] le texte n’est pas opaque sur l’arrière-plan
    }
# The colours can't be read on each other. $what is as above; $ratio is
# their contrast, e.g. 1.8, and $floor the least allowed, e.g. 3.
colors-issue-low-contrast =
    { $what ->
        [button] le texte du bouton sur le bouton est à { $ratio }:1, sous le minimum de { $floor }:1
       *[text] le texte sur l’arrière-plan est à { $ratio }:1, sous le minimum de { $floor }:1
    }
# Another file in the folder gives its theme the same name. $name is the
# name as the file has it, to put in your language's quotes; $file the
# other file's name.
colors-issue-name-taken = « { $name } » est déjà le nom de { $file }
colors-issue-file-name-unprintable = le nom du fichier contient des caractères non imprimables
# The folder holds too many theme files. $max is how many are read, e.g.
# 100.
colors-issue-too-many = limite de { $max } fichiers dépassée

## The Colors row: the palette presets under Dark and Light.

colors-colors = Couleurs :
# The preset chips. $state is customized on the picked chip while the user
# has changed some of its colours, plain otherwise.
colors-preset-default =
    { $state ->
        [customized] Par défaut (personnalisé)
       *[plain] Par défaut
    }
    .hover = Palette équilibrée pour un usage quotidien
# $state as for Default.
colors-preset-high-contrast =
    { $state ->
        [customized] Contraste élevé (personnalisé)
       *[plain] Contraste élevé
    }
    .hover = Palette à contraste maximal pour un éclairage vif ou un vidéoprojecteur
# $state as for Default. Protan and deutan are the red-green colour-vision
# deficiencies.
colors-preset-colorblind =
    { $state ->
        [customized] Daltonien (personnalisé)
       *[plain] Daltonien
    }
    .hover = Palette qui reste distinguable en vision protan ou deutan
# After the chips while there are changes a click would lose.
colors-preset-clears = Choisir un préréglage efface vos personnalisations

## The Customize colors section, folded under its header until opened.

colors-customize = Personnaliser les couleurs
    .hover = Modifier chaque couleur par-dessus le préréglage ou le thème choisi
# Beside the header: whose colours the swatches change. $mode is dark or
# light.
colors-editing =
    { $mode ->
        [light] (édition des couleurs du thème clair)
       *[dark] (édition des couleurs du thème sombre)
    }
# The same while a named theme is on. $name is the theme's name.
colors-editing-named = (édition des couleurs de { $name })

# The captions at the start of the swatch rows.
colors-group-ui = Interface :
colors-group-graph = Graphique :
colors-group-status = État :
# The arrow after a colour that others follow, before those it leads.
# $anchor is the leading colour's name, as its swatch below has it.
colors-follow-arrow =
    .hover = Ces couleurs suivent { $anchor } en gardant leur écart, jusqu’à ce que vous en choisissiez une séparément

## The swatches, each named beside it, in the rows the section shows them.

colors-swatch-background = Arrière-plan
    .hover = Couleur d’arrière-plan des panneaux
colors-swatch-frame = Cadre
    .hover = Arrière-plan de la barre du haut et du panneau de mesure (celui des panneaux s’il n’est pas défini)
colors-swatch-button = Bouton
    .hover = Couleur d’arrière-plan des boutons
# Lines between and around the panels.
colors-swatch-border = Bordure
    .hover = Séparateurs, bords des panneaux, cadres et contour du tracé
# Short for the plot's background.
colors-swatch-plot-background = Fond du tracé
    .hover = Couleur d’arrière-plan de la zone de tracé du graphique
colors-swatch-text = Texte
    .hover = Couleur du texte principal
# The meter's value as the reading panel shows it, in large type.
colors-swatch-reading = Mesure
    .hover = La mesure et les valeurs secondaires (la couleur du texte si elle n’est pas définie)
# Text drawn fainter than the rest.
colors-swatch-weak-text = Texte secondaire
    .hover = Texte secondaire : ligne du mode, libellés des valeurs secondaires, indications et légendes
colors-swatch-heading = Titre
    .hover = Titres des panneaux : Spécifications, Statistiques, Visible (la couleur du texte secondaire si elle n’est pas définie)
# The lines that follow the pointer over the graph.
colors-swatch-crosshair = Réticule
    .hover = Couleur du réticule de survol sur le graphique
colors-swatch-accent = Accentuation
    .hover = Badges de mode, liens, et fond des bascules, puces et textes sélectionnés
# The graph's main line, the readings over time.
colors-swatch-data-line = Courbe
    .hover = Couleur de la courbe des mesures sur le graphique
# The band between the lowest and highest readings.
colors-swatch-envelope = Enveloppe
    .hover = Couleur de remplissage de l’enveloppe Min/Max
# The lines of the other values a meter shows at once beside the main one.
colors-swatch-overlay-1 = Superp. 1
    .hover = Couleur de la courbe de la première valeur secondaire tracée à côté de la série
colors-swatch-overlay-2 = Superp. 2
    .hover = Couleur de la courbe de la deuxième valeur secondaire tracée à côté de la série
colors-swatch-overlay-3 = Superp. 3
    .hover = Couleur de la courbe de la troisième valeur secondaire tracée à côté de la série
# The frame the overview strip under the graph draws around the part on
# show.
colors-swatch-viewport = Fenêtre
    .hover = Couleur du rectangle de la fenêtre d’affichage dans la minimap
# A stretch with no readings.
colors-swatch-gap = Interruption
    .hover = Couleur qui marque les interruptions dans les données enregistrées
# The line at the average of the readings.
colors-swatch-mean = Moyenne
    .hover = Couleur de la ligne de moyenne
# Short for reference: a line the user sets at a value.
colors-swatch-ref = Réf.
    .hover = Couleur de la ligne de référence
# Where the readings cross a set level.
colors-swatch-crossing = Franchissement
    .hover = Couleur des marqueurs de franchissement des déclencheurs
colors-swatch-cursor = Curseur
    .hover = Couleur des curseurs de mesure
# The numbered markers the user drops on the graph.
colors-swatch-marker = Marqueur
    .hover = Lignes, drapeaux et numéros des marqueurs
# The status colours, after the states they mark. Connected is the top
# bar's status word, in quotes in the hover.
colors-swatch-connected = Connecté
    .hover = Couleur de l’état « Connecté »
colors-swatch-warning = Avertissement
    .hover = Couleur de l’état d’avertissement
colors-swatch-error = Erreur
    .hover = Couleur de l’état d’erreur
colors-swatch-inactive = Inactif
    .hover = Couleur de l’état inactif / déconnecté

## Beside a swatch.

# A swatch's name for a screen reader while it follows another colour.
# $color is its name, $anchor the name of the colour it follows.
colors-swatch-linked = { $color }, suit { $anchor }
# A small button beside a colour that was set on its own though it can
# follow another; ↺ is part of it. $color is the colour's name, $anchor
# the name of the one it would follow.
colors-relink = ↺ relier
    .hover = Suivre de nouveau { $anchor }
    .a11y = Relier { $color } à { $anchor }
# A warning after a colour the user made hard to see on another. $ratio is
# their contrast, e.g. 2.1; $ground the other colour's name; $floor the
# least contrast it needs, e.g. 4.5.
colors-low-contrast = { $ratio }:1 sur { $ground }
    .hover = Sous le contraste de { $floor }:1 dont cette couleur a besoin sur { $ground }
# A warning after a graph colour the user made hard to tell from another.
# $other is the other colour's name; $distance how far apart they are and
# $floor how far apart they need to be, as whole numbers. ΔE is the
# measure of colour difference.
colors-too-close = proche de { $other }
    .hover = Seulement { $distance } d’écart (ΔE), alors que les couleurs du graphique doivent en avoir { $floor } pour se distinguer

## The editor a swatch opens: a colour picker, then the colour as text.

# The colour written as # and six hexadecimal digits.
colors-hex = Hex
    .hover = Copier cette couleur, ou en coller une au format #RRGGBB

## Under the swatches, once a colour was changed.

colors-reset = Réinitialiser les couleurs
    .hover = Annuler vos modifications des couleurs de ce mode
# The same while a named theme is on. $name is the theme's name.
colors-reset-named =
    .hover = Annuler vos modifications des couleurs de { $name }
colors-save = Enregistrer comme thème…
    .hover = Enregistrer ces couleurs dans un fichier de thème. Enregistré dans votre dossier de thèmes, il rejoint la ligne Thème
# The file type in the save dialog's list of types.
colors-save-file-type = Thème

## What Save as theme says when it is done, in a toast: a short message that
## floats under the top bar's right end for a few seconds.

# $name is the theme's name, from the file name the user typed.
colors-saved = Thème { $name } enregistré dans votre dossier de thèmes
# Saved outside the themes folder. $file is the file name.
colors-saved-elsewhere = { $file } enregistré. Copiez-le dans votre dossier de thèmes pour l’utiliser comme thème
# The save dialog closed without an answer.
colors-save-dialog-failed = Non enregistré : échec de la boîte de dialogue d’enregistrement
# A name typed without .json gets it added, and the file of that name
# already exists. $path is that file, in full.
colors-save-exists = Non enregistré : { $path } existe déjà. Choisissez-le dans la boîte de dialogue pour le remplacer
# Another file in the themes folder gives its theme the same name. $name is
# the name; $file the other file's name.
colors-save-name-taken = Non enregistré : { $name } est déjà le nom de { $file } dans votre dossier de thèmes. Choisissez un autre nom de fichier
# The theme would be a file the folder can't use. $reason is one of the
# reasons above, starting lower case.
colors-save-invalid = Non enregistré : { $reason }
# Writing the file failed. $path is the file, in full; $error why, in the
# words of the system that raised it.
colors-save-failed = Impossible d’enregistrer { $path } : { $error }
