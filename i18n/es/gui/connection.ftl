### The connection notices drawn in the reading column under the reading
### (or, in the big-meter modes, its title in place of the reading and the
### rest in a tooltip), and the toasts: the short messages that float under
### the top bar's right end for a few seconds.
###
### ⚙ is the settings button's icon, which the notices point at. The
### notices are hard-wrapped to fit a narrow column: keep their line breaks
### where a line would otherwise run long.

## While there is nothing to show yet, or no more to come.

# The meter is being identified: each kind of meter gets a turn to answer.
connection-detecting = Detectando el multímetro…
# A recording being played back reached its last reading.
connection-ended = La reproducción terminó
# A stretch of a recording being played back that has no readings in it.
connection-no-frames = No hay tramas en esta parte de la grabación
# The meter has gone quiet for a moment. $dots is one to four dots, padded
# with spaces to four characters, that grow while the wait lasts: keep it
# at the end, with nothing after it.
connection-waiting = Esperando al multímetro{ $dots }
# Under the waiting title, when a meter is picked in Settings.
connection-waiting-check-selection = Comprueba que esté seleccionado el dispositivo correcto en Configuración (⚙) o elige allí Detección automática
# Under the waiting title, when Settings is on Auto-detect.
connection-waiting-switch-on = Activa la transmisión de datos del multímetro o elige el modelo en Configuración (⚙)

## A failed connection: a title naming the problem, the steps to try, and a
## closing line.

# The closing line under the steps. "Connect" is the top bar's button.
connection-click-connect = Haz clic en “Conectar” después de resolver el problema.
# The closing line after the Bluetooth steps when nothing answered at the
# address given with --adapter, a command-line option kept as it is. "It"
# is the scan command the steps above name.
connection-restart-with-address = Reinicia con --adapter igual a la dirección que muestra ese comando.

# A meter that connects over Bluetooth only, and Bluetooth was not looked at.
connection-bluetooth-not-searched = No se buscó por Bluetooth
# Under that title when Bluetooth is switched off. $error is the sentence
# saying why it was not searched; $hint says how to switch it
# on.
connection-bluetooth-not-searched-off = { $error }. { $hint }
# Under that title otherwise. $error is the sentence saying why.
connection-bluetooth-not-searched-why = { $error }.

# --adapter, a command-line option, named a USB device that isn't there.
connection-adapter-not-found = No se encuentra el adaptador
# $selector is the value given with --adapter.
connection-adapter-not-matched = Ningún dispositivo coincide con --adapter '{ $selector }'.
# After the list of what is plugged in.
connection-adapter-restart = Reinicia con el valor correcto de --adapter.

# The cable or Bluetooth link is up but no meter answered. $link is usb (a
# USB cable), adapter (a Bluetooth adapter plugged into the meter) or radio
# (the meter's own Bluetooth). A list of meters and their steps follows the
# colon.
connection-not-identified =
    { $link ->
        [adapter] El adaptador Bluetooth está conectado, pero ningún multímetro se identificó.
        [radio] El enlace Bluetooth está conectado, pero ningún multímetro se identificó.
       *[usb] El cable USB está conectado, pero ningún multímetro se identificó.
    }

    Activa la transmisión de datos del multímetro o elige el modelo en Configuración (⚙):
# After that list.
connection-not-identified-report = ¿Ya transmite y sigue sin reconocerse? Elige su modelo en Configuración (⚙) e infórmanos del problema.

# The meter stopped answering. $link is usb (a USB cable), adapter (a
# Bluetooth adapter plugged into the meter), radio (the meter's own
# Bluetooth) or none (a simulated meter, on no link). $model is the meter
# picked, e.g. UNI-T UT61E+; $steps are its own steps for switching data
# transmission on, several lines.
connection-no-response =
    { $link ->
        [usb]
            El cable USB está conectado, pero el multímetro
            no responde (modelo seleccionado: { $model }).
        [adapter]
            El adaptador Bluetooth está conectado, pero el multímetro
            no responde (modelo seleccionado: { $model }).
        [radio]
            El enlace Bluetooth está conectado, pero el multímetro
            no responde (modelo seleccionado: { $model }).
       *[none]
            El enlace está conectado, pero el multímetro
            no responde (modelo seleccionado: { $model }).
    }

    Si no es el dispositivo correcto, cámbialo en Configuración (⚙) o elige allí Detección automática.
    Si lo es, activa la transmisión de datos:
    { $steps }
# The same, on Auto-detect: no meter is picked to name.
connection-no-response-auto =
    Activa la transmisión de datos del multímetro
    o elige el modelo en Configuración (⚙).

# A meter that stopped answering in the middle of a session. $title is the
# shared notice title, "No response from meter".
connection-no-response-check = { $title }: comprueba el dispositivo seleccionado y la transmisión de datos
# The reading thread stopped by itself: it crashed, or gave up connecting.
connection-acquisition-stopped = La adquisición se detuvo inesperadamente. Reconecta para reanudarla

# The last line of a notice for a meter whose support isn't fully verified;
# a link. $warning is the sentence saying how far it is verified.
connection-feedback = { $warning } Enviar comentarios.
    .hover = Abrir el gestor de incidencias de GitHub para enviar comentarios

## The notice under the reading when the Bluetooth link delivers readings
## two at a time, which can end with a command that fixes it.

# Copies the command to the clipboard.
connection-late-copy = Copiar
# A toast, once the command is copied.
connection-late-copied = Comando copiado

## Toasts.

# A meter identified itself while Settings was on Auto-detect. $meter is
# its name, e.g. UNI-T UT61E+. $saved is yes when Settings now names it, no
# for a session that detects every time.
connection-detected =
    { $saved ->
        [yes] Se detectó el multímetro { $meter } y se guardó como tu dispositivo. Elige Detección automática en Configuración para volver a detectar el multímetro.
       *[no] Se detectó el multímetro { $meter }
    }
# The same, when the name the meter gave for itself differs; $meter and
# $saved as there. $reported is that name, e.g. UT216XD.
connection-detected-reports =
    { $saved ->
        [yes] Se detectó el multímetro { $meter } (se identifica como “{ $reported }”) y se guardó como tu dispositivo. Elige Detección automática en Configuración para volver a detectar el multímetro.
       *[no] Se detectó el multímetro { $meter } (se identifica como “{ $reported }”)
    }
# The meter named another model than the one picked in Settings. $reported
# is the name it gave, e.g. UT161E; $picked the model picked, $named the
# model that name belongs to, e.g. UNI-T UT161E.
connection-other-model = El multímetro se identifica como “{ $reported }”, no como el modelo seleccionado, { $picked }. Elige { $named } o Detección automática en Configuración.
# Detection settled on a meter, then the meter named another. $reported is
# the name it gave, $named the model it belongs to. $saved is yes when
# Settings now names that model.
connection-named-after-detection =
    { $saved ->
        [yes] El multímetro se identifica como “{ $reported }” y se guardó como tu dispositivo: reconecta para leerlo como { $named }.
       *[no] El multímetro se identifica como “{ $reported }”: reconecta para leerlo como { $named }.
    }

# The recording stopped because Buffer size, in Settings, was lowered below
# what it held. $count is how many samples it took, $kept the same number
# written short, e.g. 1.2K. Export… is a button.
connection-buffer-shrunk =
    { $count ->
        [one] Grabación detenida: se conserva { $kept } muestra y “Exportar” la guarda
       *[other] Grabación detenida: se conservan { $kept } muestras y “Exportar” las guarda
    }
# The recording filled the buffer. $count is its size in samples, $kept the
# same number written short, e.g. 500K.
connection-buffer-full =
    { $count ->
        [one] Grabación detenida: búfer lleno ({ $kept } muestra)
       *[other] Grabación detenida: búfer lleno ({ $kept } muestras)
    }

# A button command sent to the meter failed. $command is its name as the
# meter's protocol knows it, e.g. hold; $error says why.
connection-command-failed = Error en el comando '{ $command }': { $error }
# Switching a meter setting failed. $setting is mode, range or the
# setting's lowercase name as the command line takes it (hold, rel, minmax,
# peak); $error says why.
connection-switch-failed =
    { $setting ->
        [mode] Error al cambiar el modo: { $error }
        [range] Error al cambiar el rango: { $error }
       *[other] Error al cambiar el ajuste { $setting }: { $error }
    }

# The × that closes a toast or a notice.
connection-dismiss =
    .hover = Cerrar
    .a11y = Cerrar la notificación
