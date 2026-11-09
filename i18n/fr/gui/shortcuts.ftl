### The keyboard and mouse help, a dialog opened with ? or F1 or the ?
### button in the top bar (no screenshot in the docs): a title row, then
### three tables (General, Graph (keys), Graph (mouse)) and a note.
###
### Each table row has two columns: the keys or the mouse gesture on the
### left, in a monospace font, and what they do on the right. The key
### column is written by the app as this computer spells the keys, e.g.
### Ctrl+O or ⌘O, and is not translated; these messages are the right-hand
### column, which wraps when the dialog is narrow. Then the message shown
### when Ctrl+T can't keep the window on top.

## The title row.

# The dialog's title.
shortcuts-title = Clavier et souris
# The × button at the right end of the title row, which closes the dialog:
# its name for a screen reader.
shortcuts-close =
    .a11y = Fermer l’aide des raccourcis clavier

## The General table: the app's own shortcuts, shown in the left column of
## the dialog when it is wide.

# The table's heading.
shortcuts-general = Général
# Ctrl+O: connects to the meter, or disconnects while connected.
shortcuts-connect = Connecter / déconnecter
# Space: pauses reading the meter, or resumes while paused.
shortcuts-pause = Mettre en pause / reprendre
# Ctrl+L: clears the graph and the Statistics panel.
shortcuts-clear = Effacer le graphique et les statistiques
# Ctrl+R: starts or stops recording.
shortcuts-record = Démarrer / arrêter l’enregistrement
# Ctrl+E: opens the dialog that saves the recording as a CSV file.
shortcuts-export = Exporter en CSV…
# Ctrl+I: opens the dialog that picks a file to open.
shortcuts-import = Importer un fichier…
# N: puts a numbered marker on the reading the meter shows now.
shortcuts-add-marker = Ajouter un marqueur sur la mesure affichée
# Ctrl+N: the same, then puts the caret in the marker's note.
shortcuts-add-marker-note = Ajouter un marqueur et écrire sa note
# Ctrl+B: steps the big meter (the reading drawn large) through off, full
# window and a minimal strip.
shortcuts-big-meter = Changer le mode grand affichage (désactivé / plein / minimal)
# Ctrl+T: keeps the window above other windows, or stops doing so.
shortcuts-always-on-top = Activer / désactiver Toujours visible
# The same row on Wayland, where the window can't do it and the row is
# greyed. Keep the part after the colon the same as
# settings-always-on-top.wayland and shortcuts-always-on-top-toast.
shortcuts-always-on-top-wayland = Toujours visible : non disponible sous Wayland – faites un clic droit sur la barre de titre pour garder la fenêtre au-dessus des autres
# Ctrl+D: hides or shows the window's title bar and border.
shortcuts-decorations = Afficher / masquer le cadre de la fenêtre
# F11 (Ctrl+⌘+F on macOS): fills the screen with the window, or returns.
shortcuts-fullscreen = Activer / quitter le plein écran
# ⌘M, on macOS only: minimises the window.
shortcuts-minimize = Réduire la fenêtre
# Ctrl+Plus/Minus: makes everything in the window larger or smaller.
shortcuts-zoom = Zoom avant / arrière
# Ctrl+0: returns everything to its normal size.
shortcuts-zoom-reset = Réinitialiser le zoom à 100 %
# Esc or Ctrl+W: closes this dialog.
shortcuts-close-help = Fermer cette aide
# Ctrl+Q or Ctrl+W: closes the app.
shortcuts-quit = Quitter

## The Graph (keys) table: keys that act on the graph, while no button or
## field has the keyboard.

# The table's heading.
shortcuts-graph-keys = Graphique (touches)
# [ and ]: pick the next shorter or longer span of time the graph shows.
shortcuts-graph-window = Raccourcir / allonger la fenêtre de temps
# The Left and Right arrow keys: move the graph back or forward in time.
shortcuts-graph-scroll = Faire défiler la vue
# Home: shows the start of the readings.
shortcuts-graph-start = Aller au début
# End: shows the newest readings and follows them again.
shortcuts-graph-live = Revenir au direct
# M: shows or hides the line drawn at the mean of the readings.
shortcuts-graph-mean = Afficher / masquer la moyenne
# X: shows or hides the band between the lowest and highest readings.
shortcuts-graph-band = Afficher / masquer l’enveloppe min/max
# H: shows or hides the histogram beside the graph.
shortcuts-graph-histogram = Afficher / masquer l’histogramme
# R: shows or hides the reference lines, and puts the caret in the field
# where their values are typed.
shortcuts-graph-reference = Afficher / masquer les lignes de référence et saisir leurs valeurs
# T: shows or hides marks where the readings cross a reference line; Ref is
# the graph's chip that shows the reference lines.
shortcuts-graph-triggers = Afficher / masquer les déclencheurs (avec Réf. activé)
# C: shows or hides the two cursors that measure between points.
shortcuts-graph-cursors = Afficher / masquer les curseurs

## The Graph (mouse) table: what the mouse does on the graph. Each message
## is what the gesture does; its .gesture is the gesture itself, in the
## left column. Shift and Ctrl are key names: keep them as they are, the
## way the key column spells them.

# The table's heading.
shortcuts-graph-mouse = Graphique (souris)
shortcuts-mouse-drag = Faire défiler l’historique vers la gauche / la droite
    .gesture = Glisser
# On a laptop touchpad: two fingers moved left or right together.
shortcuts-mouse-swipe = Faire défiler l’historique vers la gauche / la droite
    .gesture = Balayage à deux doigts
# The box dragged out sets both the span of time and the range of values
# shown.
shortcuts-mouse-box-zoom = Zoomer sur le rectangle tracé (temps et valeur)
    .gesture = Maj + glisser
# X axis: the time axis. Zooms around the mouse pointer.
shortcuts-mouse-zoom = Zoomer sur l’axe X autour du pointeur
    .gesture = Ctrl + molette
# The wheel alone scrolls the window's panels, not the graph.
shortcuts-mouse-scroll = Faire défiler le panneau
    .gesture = Molette
# Follows the newest readings again and fits the value axis to them.
shortcuts-mouse-reset = Revenir au suivi en direct et à l’axe Y auto
    .gesture = Double-clic
# While the cursors are shown (C), a click places cursor A or B on the
# reading nearest to it.
shortcuts-mouse-cursor = Placer le curseur A / B sur le point le plus proche
    .gesture = Clic (curseurs activés)
# Opens a menu to add a marker, copy the reading, place the cursors, set
# reference lines or set an alarm.
shortcuts-mouse-menu = Marqueur, copie, curseurs, lignes de référence, alarme
    .gesture = Clic droit
# The minimap is the strip under the graph showing all the readings:
# dragging in it moves the view, dragging its edges resizes it.
shortcuts-mouse-minimap = Aller à un instant / redimensionner la fenêtre d’affichage
    .gesture = Glisser (minimap)

## Key names, as the key column above and every hover that names a
## shortcut write them, e.g. Ctrl+O, Esc / Ctrl+W. Use the names printed on
## keyboards in your language. On a Mac the modifiers show as ⌘ ⇧ ⌥ symbols
## where the font has them, and these names aren't used.

shortcuts-key-ctrl = Ctrl
shortcuts-key-shift = Maj
shortcuts-key-alt = Alt
shortcuts-key-space = Espace
shortcuts-key-escape = Échap
# The left and right arrow keys.
shortcuts-key-left = Gauche
shortcuts-key-right = Droite
shortcuts-key-home = Origine
shortcuts-key-end = Fin
# The + and - keys, named together for zooming in and out: Ctrl+Plus/Minus.
shortcuts-key-plus = Plus
shortcuts-key-minus = Moins

## The note under the tables, in small grey text.

# Space, Escape, Up/Down, PgUp/PgDn and Home/End are key names: write them
# the way the key names above do.
shortcuts-note = Les raccourcis du graphique et la touche Espace sont désactivés tant qu’un élément a le focus clavier : appuyez sur Échap pour le libérer. Haut/Bas, Pg préc/Pg suiv et Origine/Fin font défiler cette aide.

## Ctrl+T on Wayland, where a window can't put itself above the others.

# A message that pops up briefly over the window. Keep it the same as
# settings-always-on-top.wayland.
shortcuts-always-on-top-toast = Non disponible sous Wayland – faites un clic droit sur la barre de titre pour garder la fenêtre au-dessus des autres
