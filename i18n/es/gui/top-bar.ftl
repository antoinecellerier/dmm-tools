### The top bar: the meter's name and the connection buttons on the left,
### the connection status in the middle, and the version, Help & Feedback,
### shortcuts and settings buttons on the right
### (assets/gui-wide-layout.webp).
###
### The bar is one row on a wide window: keep these short. $keys is a
### keyboard shortcut as this computer writes it, e.g. Ctrl+O or ⌘O.

## The meter's name, at the left end.

# Shown in place of a meter's name while Auto-detect hasn't found one yet.
top-bar-auto-detect = Detección automática
# Shown in place of a meter's name for an imported file that names none.
top-bar-imported-file = Archivo importado

## Connection buttons.

top-bar-connect = Conectar
    .hover = Abrir la conexión USB con el multímetro seleccionado ({ $keys })
top-bar-disconnect = Desconectar
    .hover = Cerrar la conexión activa con el multímetro ({ $keys })
# A button; ▶ and ⏸ are part of it.
top-bar-resume = ▶ Reanudar
    .hover = Reanudar la adquisición ({ $keys })
top-bar-pause = ⏸ Pausar
    .hover = Detener la adquisición: se detienen la grabación y la actualización del gráfico ({ $keys })
# Clears the graph and the statistics, not the recording.
top-bar-clear = Borrar
    .hover = Borrar el historial del gráfico y las estadísticas ({ $keys })
# While the connection is being retried after it dropped.
top-bar-reconnecting = Reconectando…
# $attempt counts the retries, from 1.
top-bar-reconnecting-attempt = Reconectando (intento { $attempt })…
top-bar-reconnecting-hover = Reintentando la conexión automáticamente: haz clic en Desconectar para detenerla
# $error is the last attempt's error, in the language of the system that raised it.
top-bar-reconnecting-hover-error =
    Reintentando la conexión automáticamente: haz clic en Desconectar para detenerla.
    Último error: { $error }
top-bar-stop-retrying = Desconectar
    .hover = Dejar de reintentar y cerrar la conexión ({ $keys })

## The status beside the coloured dot. $name is the meter's name, e.g.
## UT61E+. $link is how it is connected (USB cable, Bluetooth). $paused is
## yes or no.

# The meter's name when it didn't report one.
top-bar-connected = Conectado
top-bar-status =
    { $paused ->
        [yes] { $name } (en pausa)
       *[no] { $name }
    }
top-bar-status-link =
    { $paused ->
        [yes] { $name } · { $link } (en pausa)
       *[no] { $name } · { $link }
    }
# An imported file: $name is its meter, $file its file name.
top-bar-status-file = { $name } · { $file }
top-bar-disconnected = Desconectado

## The status's hover: how the meter is connected, spelled out.
## $link is usb (a USB cable), adapter (a Bluetooth adapter plugged into the
## meter), radio (the meter's own Bluetooth) or none.

top-bar-link-live =
    { $link ->
        [usb] Conectado mediante el cable USB
        [adapter] Conectado mediante el adaptador Bluetooth
        [radio] Conectado mediante el enlace Bluetooth
       *[none] Multímetro simulado, sin enlace
    }
# A recording being played back: how it was recorded.
top-bar-link-recorded =
    { $link ->
        [usb] Grabado mediante el cable USB
        [adapter] Grabado mediante el adaptador Bluetooth
        [radio] Grabado mediante el enlace Bluetooth
       *[none] Grabado mediante un enlace sin nombre
    }
# $path is the file's full path.
top-bar-replaying = Reproduciendo { $path }
# $readings is how many readings the file held; $path is the file's full
# path.
top-bar-imported-from =
    { $readings ->
        [one] { $readings } lectura importada de { $path }
       *[other] { $readings } lecturas importadas de { $path }
    }

## The support badge, after the status, for a meter not yet fully verified.
## A link to report feedback. Capitals, as the meter's own annunciators.

top-bar-badge-verified = VERIFICADO
top-bar-badge-partly-verified = VERIF. PARCIAL
top-bar-badge-experimental = EXPERIMENTAL
# $warning is the sentence saying how far the meter's support is verified.
top-bar-badge-hover = { $warning } Haz clic para enviar comentarios.

## The right end: the version, which opens What's New; Help & Feedback; the
## shortcut list; Settings.

top-bar-version =
    .hover = Mostrar las novedades: notas de la versión actual
    .a11y = Mostrar las notas de la versión
# The version's hover when the bar has no room for the update link.
# $version is the newer release, e.g. v0.10.0.
top-bar-version-update =
    .hover =
        Mostrar las novedades: notas de la versión actual
        { $version } está disponible: ensancha la ventana para ver su enlace
    .a11y = Mostrar las notas de la versión. { $version } está disponible
# The update link; $version is the newer release, e.g. v0.10.0.
top-bar-update =
    .hover = Abrir la página de la versión { $version } en GitHub
    .a11y = { $version } disponible: abrir la página de la versión
top-bar-project-link =
    .hover = Abrir la página del proyecto dmm-tools en GitHub
# The ? button. $keys and $other are its two keys, ? and F1.
top-bar-shortcuts =
    .hover = Mostrar los métodos abreviados de teclado y los gestos del mouse ({ $keys } o { $other })
    .a11y = Métodos abreviados de teclado y gestos del mouse
# The ⚙ button.
top-bar-settings =
    .hover = Mostrar u ocultar el panel de configuración
    .a11y = Configuración
