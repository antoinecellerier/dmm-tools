### The settings panel, which the ⚙ button at the top bar's right end opens
### under the bar (assets/gui-settings.webp): rows of checkboxes and of
### chips, each row led by its caption, in groups a rule apart. The colour
### and theme rows have a file of their own.
###
### Rows wrap on a narrow window, so keep the chips short. $keys and the
### other shortcut variables are keyboard shortcuts as this computer writes
### them, e.g. Ctrl+O or ⌘O. Text in parentheses starting with -- names a
### command-line option, kept as it is, that set the row for this session
### only.

## The meter: which one, how the mock runs, and how a connection is made.

# The Device row's caption; .a11y names its dropdown for a screen reader.
settings-device = Dispositivo:
    .a11y = Dispositivo
# The Device dropdown, and its first entry, when no particular meter is
# picked: each kind of meter is tried in turn.
settings-auto-detect = Detección automática
# The Device dropdown while --device picked the meter for this session.
# $name is the meter, e.g. UNI-T UT61E+, or Auto-detect.
settings-device-pinned = { $name } (--device)
# Beside the greyed Device dropdown while a recording is played back:
# --replay is the command-line option that opened it.
settings-device-replay = (reinicia sin --replay para elegir un multímetro)
# Beside the greyed Device dropdown while the session runs on a simulated
# clock, set by command-line options for testing.
settings-device-clock = (reinicia sin las opciones de reloj para elegir un multímetro)

## The open Device dropdown: Auto-detect, then every meter under its brand.

# An entry's hover and screen-reader name. $name is the meter, e.g. UNI-T
# UT61E+; $tag how far its support is verified, e.g. partly verified.
settings-device-entry = { $name } ({ $tag })

## The Mock mode row, shown while a simulated meter is picked. Its other
## chips are the simulated modes' names, kept as they are.

# The first chip: the simulated meter steps through every mode. $pinned is
# yes when --mock-mode picked it for this session, no otherwise.
settings-mock-auto =
    { $pinned ->
        [yes] Auto (ciclo) (--mock-mode)
       *[no] Auto (ciclo)
    }
    .hover = Recorrer todos los modos simulados para probar la interfaz
# The chip for the mode --mock-mode picked for this session. $mode is its
# name, e.g. dcv.
settings-mock-mode-pinned = { $mode } (--mock-mode)
settings-mock-mode = Modo simulado:

## How a connection is made: checkboxes on one row.

settings-auto-connect = Conectar al iniciar
    .hover = Abrir la conexión USB automáticamente al iniciar la aplicación
# Asking the meter its name makes it beep.
settings-device-name = Mostrar el nombre del dispositivo al conectar (pitido)
    .hover = Pedir su nombre al multímetro después de conectar: el multímetro emite un pitido
# The Bluetooth checkbox while --no-bluetooth switched the search off for
# this session. $setting is the checkbox's own label, e.g. Look for
# Bluetooth devices.
settings-bluetooth-pinned = { $setting } (--no-bluetooth)
settings-bluetooth =
    .hover = Buscar un adaptador o un multímetro Bluetooth al alcance cuando ningún cable USB responda. Se aplica en la próxima conexión.

## What is kept of the readings.

settings-sample-interval = Intervalo de muestreo:
# The first Sample interval chip; the others are durations, e.g. 300ms or 2s.
settings-interval-every-reading = Cada lectura
    .hover = Conservar cada lectura que envía el multímetro, a su propio ritmo
# The other Sample interval chips' hover. $unit is second for exactly one
# second, seconds for a whole number of seconds and ms otherwise; $n is how
# many seconds or milliseconds. s and ms are unit symbols.
settings-interval-chip =
    .hover =
        { $unit ->
            [second] Como máximo una lectura por segundo: la más cercana a cada instante de muestreo
            [seconds] Como máximo una lectura cada { $n } s: la más cercana a cada instante de muestreo
           *[ms] Como máximo una lectura cada { $n } ms: la más cercana a cada instante de muestreo
        }

settings-buffer-size = Tamaño del búfer:
# A Buffer size chip's hover; the chips are sample counts written short,
# e.g. 500K. $count is the chip's number of samples and $samples the same
# number written short. $memory is what they take, e.g. ≈194 MB, and $span
# how long they last, e.g. 14 h. $pace is every-reading when the Sample
# interval keeps every reading, whose pace the estimate can't know, and
# interval otherwise.
settings-buffer-size-chip =
    .hover =
        { $pace ->
            [every-reading]
                { $count ->
                    [one] Conservar hasta { $samples } muestra en el gráfico y para exportar: { $memory }, aproximadamente { $span } a 10 lecturas por segundo. Una grabación detenida que se conserve aparte puede ocupar otro tanto
                   *[other] Conservar hasta { $samples } muestras en el gráfico y para exportar: { $memory }, aproximadamente { $span } a 10 lecturas por segundo. Una grabación detenida que se conserve aparte puede ocupar otro tanto
                }
           *[interval]
                { $count ->
                    [one] Conservar hasta { $samples } muestra en el gráfico y para exportar: { $memory }, aproximadamente { $span } con el intervalo de muestreo actual. Una grabación detenida que se conserve aparte puede ocupar otro tanto
                   *[other] Conservar hasta { $samples } muestras en el gráfico y para exportar: { $memory }, aproximadamente { $span } con el intervalo de muestreo actual. Una grabación detenida que se conserve aparte puede ocupar otro tanto
                }
        }
# After the Buffer size chips: what the picked size costs. $memory is the
# memory it takes, e.g. ≈194 MB; $span how long it lasts, e.g. 14 h.
settings-buffer-cost = ({ $memory }, aprox. { $span })

## Which panels show, and what the Specifications panel shows: checkboxes
## on one row.

settings-graph = Gráfico
    .hover = Mostrar el gráfico de las lecturas, que avanza con el tiempo
# σ is the standard deviation, as the Statistics panel writes it.
settings-statistics = Estadísticas
    .hover = Mostrar Mín. / Máx. / Media / σ / integral de la sesión en vivo
settings-recording = Grabación
    .hover = Mostrar los controles de grabación y el registro de muestras
# The Specifications panel's checkbox. $fields is shown while the panel is
# on and the checkboxes for its fields follow it, hidden otherwise.
settings-specs =
    { $fields ->
        [shown] Especificaciones:
       *[hidden] Especificaciones
    }
    .hover = Mostrar la exactitud y la resolución del modo actual
# The Specifications fields, as that panel names them.
settings-specs-resolution = Resolución
    .hover = Mostrar la resolución en las especificaciones
settings-specs-accuracy = Exactitud
    .hover = Mostrar la exactitud en las especificaciones
# Input Z is the input impedance.
settings-specs-input-z = Impedancia de entrada
    .hover = Mostrar la impedancia de entrada en el panel completo de especificaciones
settings-specs-notes = Notas
    .hover = Mostrar las notas del manual en el panel completo de especificaciones

## The window.

# The Language row's caption, shown once the app speaks more than one
# language. In every language but English, keep the English word in
# parentheses after yours, e.g. Langue (Language):, so someone who can't
# read the window can still find the row. .a11y names its dropdown for a
# screen reader, in your word alone.
settings-language = Idioma (Language):
    .a11y = Idioma
    .hover = El idioma de la interfaz
# The dropdown's first entry: the window speaks the system's language.
# $language is the language that is, by its name in itself, e.g. Français
# or 简体中文. The other entries are each language's name in itself, kept
# as they are.
settings-language-system = Sistema ({ $language })
    .hover = Usar el idioma del sistema
# Weak text after your language's name in the dropdown while no native
# speaker has reviewed it yet. Shown only next to your own language's name,
# whatever language the window speaks, so its speakers can read it: e.g.
# Português (Brasil)  tradução automática.
settings-language-machine-translated = traducción automática
# A screen reader's name for that entry, also in your language whatever the
# window speaks. $language is your language's name in itself, e.g.
# Português (Brasil).
settings-language-machine-translated-entry = { $language }, traducción automática
# The weak line under the Language row while such a language is spoken,
# then a link that opens the translators' guide on GitHub in the web
# browser; ↗ marks a link.
settings-language-unreviewed = Traducción automática, aún sin revisar.
settings-language-review-link = Ayudar a revisarla ↗
    .hover = Abrir la guía para traductores en GitHub
# A toast when Chinese is picked and the computer has no font for it. It
# is always shown in English, which the app's own fonts draw: no need to
# translate it.
settings-language-no-chinese-font = No font for Chinese characters was found. Install a Simplified Chinese font, such as Noto Sans CJK SC, then restart dmm-gui.
settings-language-no-japanese-font = No font for Japanese characters was found. Install a Japanese font, such as Noto Sans CJK JP, then restart dmm-gui.

# The Zoom chips are percentages, e.g. 150%.
settings-zoom = Zoom:
# A Zoom chip's hover. $level is its scale in percent, e.g. 150; $step the
# keys that step the zoom.
settings-zoom-chip =
    .hover = Escalar la interfaz al { $level } % ({ $step } para cambiarlo por pasos)
# The 100% chip's hover. $reset is the keys that go back to 100%, $step
# those that step the zoom.
settings-zoom-reset-chip =
    .hover = Escalar la interfaz al 100 % ({ $reset }, o { $step } para cambiarlo por pasos)
# Window decorations are the title bar and border.
settings-hide-decorations = Ocultar el marco de la ventana
    .hover = Ventana sin marco: usa { $keys } para volver a mostrarlo
# On Wayland the checkbox is greyed: .wayland is its hover there, and
# settings-always-on-top-wayland the note beside it. Keep both the same as
# shortcuts-always-on-top-toast, which Ctrl+T shows there.
settings-always-on-top = Siempre visible
    .hover = Mantener la ventana encima de las demás ventanas del escritorio ({ $keys })
    .wayland = No disponible en Wayland. Haz clic con el botón derecho en la barra de título para mantener la ventana encima de las demás
# The note after the greyed checkbox on Wayland, in small grey text.
settings-always-on-top-wayland = (No disponible en Wayland. Haz clic con el botón derecho en la barra de título para mantener la ventana encima de las demás)

## The update check, in builds downloaded from GitHub.

settings-check-updates = Buscar nuevas versiones
    .hover = Una vez al día, preguntar a GitHub si hay una versión más reciente. GitHub ve tu dirección IP; no se envía nada sobre tu multímetro.
# The link the top bar shows when a newer release is out; ↗ marks a link.
# $version is the release, e.g. v0.10.0.
settings-update-release = { $version } disponible ↗
# The same for a newer nightly build, which has no version number.
settings-update-dev = Compilación de desarrollo más reciente ↗
# Its shorter form, when the top bar is short of room.
settings-update-dev-short = Actualizar ↗

## How a screen reader hears the reading change.

# A screen reader speaks the reading each time it changes; this row sets how
# often at most. Its chips are durations like the Sample interval row's,
# Every reading taking that row's first chip's label, then Never.
settings-announce = Anuncios del lector de pantalla:
settings-announce-every-reading =
    .hover = Anunciar cada lectura nueva. Un lector de pantalla puede interrumpirse cuando las lecturas llegan más rápido de lo que las lee.
# The duration chips' hover. $n is a whole number of seconds.
settings-announce-chip =
    .hover =
        { $n ->
            [one] Anunciar la lectura como máximo una vez por segundo. Un cambio de modo se anuncia de inmediato.
           *[other] Anunciar la lectura como máximo una vez cada { $n } segundos. Un cambio de modo se anuncia de inmediato.
        }
settings-announce-never = Nunca
    .hover = No anunciar las lecturas cuando cambian. Una pérdida de conexión se sigue anunciando.
# A checkbox at the end of the same row, or under it when the row wraps.
# Focus is keyboard focus, or where the screen reader's cursor is.
settings-announce-focused = Solo cuando la lectura tiene el foco
    .hover = Anunciar la lectura solo cuando el lector de pantalla está en ella, al ritmo elegido. La tecla Tab lleva hasta ella.

## How the graph's lines look, the last row.

settings-graph-lines = Curvas:
# Sub-value lines are the extra traces a meter that shows several values
# at once draws beside the main one.
settings-graph-lines-patterned = Discontinuas
    .hover = Líneas discontinuas y de puntos para los valores secundarios, distinguibles sin color
settings-graph-lines-solid = Continuas
    .hover = Líneas continuas, distinguibles por el color y la leyenda

# The file could not be understood, so a copy of it was kept before it is
# written over. $path is the copy's full path, e.g.
# /home/ana/.config/dmm-tools/settings.json.bad.
settings-load-failed-kept = No se pudo leer el archivo de configuración, así que se usan los valores predeterminados. Se conserva una copia en { $path }.
# The file could not be opened at all. $path is its full path, e.g.
# /home/ana/.config/dmm-tools/settings.json.
settings-load-failed = No se pudo leer el archivo de configuración { $path }, así que se usan los valores predeterminados.
