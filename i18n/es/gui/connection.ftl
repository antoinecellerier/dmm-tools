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

# Under a failed connection's steps when a meter is picked in Settings,
# which a detected connect may have saved. $model is that meter, e.g.
# UNI-T UT61E+.
connection-selected-meter = El multímetro { $model } está seleccionado en Configuración (⚙). Si el tuyo es otro, selecciónalo allí o elige Detección automática.
# The same when the title above already names the meter picked.
connection-selected-meter-named = Si tu multímetro es otro, selecciónalo en Configuración (⚙) o elige allí Detección automática.
# After shared.ftl's unrecognised-data, in a toast: where to report it.
# { project-link } is the top bar's Help & Feedback link.
connection-unrecognised-report = Comunícalo desde { project-link }, en la barra superior.
# The closing line under the steps. "Connect" is the top bar's button.
connection-click-connect = Resuelve el problema y vuelve a presionar “Conectar”.
# The closing line after the Bluetooth steps when nothing answered at the
# address given with --adapter, a command-line option kept as it is. "It"
# is the scan command the steps above name.
connection-restart-with-address = Reinicia con --adapter seguido de la dirección que muestra ese comando.

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

# The meter answered, but with data that could not be read: a corrupt
# frame, or a mode this tool does not know. The error itself follows.
connection-bad-data = El multímetro envió datos ilegibles
# A dropped link that was not reconnected. shared.ftl's several-cables
# follows, then connection-several-cables-action. "Connect" is the top bar's
# button.
connection-several-cables = Enlace perdido
connection-several-cables-action = Deja conectado solo el cable de este multímetro y haz clic en Conectar.
# A toast, when that notice ends a running recording. What it holds can still
# be exported.
connection-several-cables-recording = Grabación detenida: se perdió el enlace y el cable USB de este multímetro no se puede distinguir de los demás
# A toast, when a different meter answers the session during a recording,
# as after a reconnect under Auto-detect or a device picked in Settings.
# $meter is the meter that answered, a model name. What was recorded can
# still be exported.
connection-other-meter-recording = Grabación detenida: respondió el multímetro { $meter } en lugar del que se estaba grabando. Lo grabado aún se puede exportar.
# Any other failure to connect or to keep reading. The error itself
# follows, then connection-click-connect.
connection-failed = Error de conexión
# The reading thread crashed, or the system refused to start it. $detail is
# the crash's or the system's own text, which stays English. Followed by
# connection-click-connect.
connection-internal-error = Error interno: { $detail }

# A meter that stopped answering in the middle of a session. $title is the
# shared notice title, "No response from meter".
connection-no-response-check = { $title }: comprueba el dispositivo seleccionado y la transmisión de datos
# The reading thread stopped by itself: it crashed, or gave up connecting.
connection-acquisition-stopped = La adquisición se detuvo inesperadamente. Reconecta para reanudarla

# The last line of a notice for a meter whose support isn't fully verified;
# a link. $warning is the sentence saying how far it is verified.
connection-feedback = { $warning } Enviar comentarios.
    .hover = Abrir el gestor de incidencias de GitHub

## The notice under the reading when the Bluetooth link delivers readings
## two at a time, which can end with a command that fixes it.

# Copies the command to the clipboard.
connection-late-copy = Copiar
# A toast, once the command is copied.
connection-late-copied = Comando copiado
# The notice's last line, on a link the tool has no fix for yet.
# { project-link } is the top bar's Help & Feedback link.
connection-late-report = Comunícalo desde { project-link }, en la barra superior, e indica el multímetro y el sistema operativo.

## The note under the reading that asks, once, for a capture of a meter whose
## support is experimental. A capture is a guided run of the dmm-cli tool
## that records what the meter sends in each of its modes.

connection-capture-ask-title = Ayuda a verificar este multímetro
# $warning is the sentence saying how far the meter's support is verified,
# which names the meter. "yours" is the reader's own meter.
connection-capture-ask-body = { $warning } Una captura guiada hecha con el tuyo es la contribución más valiosa que puedes hacer al proyecto: confirma lo que funciona y permite corregir cualquier problema.
# The same on a phone, which has no dmm-cli to run the capture with;
# $warning as above.
connection-capture-ask-body-computer = { $warning } Una captura guiada hecha con el tuyo, que se ejecuta con dmm-cli en un equipo después de desconectar el multímetro de esta aplicación, es la contribución más valiosa que puedes hacer al proyecto: confirma lo que funciona y permite corregir cualquier problema.
# Above the command that runs the capture: this window has the meter, and
# the command needs it. Copy is the button after the command, and the note
# goes when the top bar's Disconnect is pressed, so the copy comes first.
connection-capture-ask-disconnect = Copia este comando, desconecta el multímetro y luego ejecuta el comando:
# A link to the contributors' guide on GitHub; ↗ marks a link.
connection-capture-ask-link = Cómo hacer una captura ↗
    .hover = Abrir la guía de captura en GitHub

## Toasts.

# After shared.ftl's saved-mock-mode-unknown. Mock mode is the setting's
# name in Settings.
connection-saved-mock-mode-unknown-action = Elige un modo simulado en Configuración (⚙).
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
        [one] Grabación detenida: se conserva { $kept } muestra. Usa “Exportar” para guardarla
       *[other] Grabación detenida: se conservan { $kept } muestras. Usa “Exportar” para guardarlas
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
# One of the two messages above ($failed), then what to do about it, a
# sentence of its own ($remedy): what to pick when the meter is another
# model (shared.ftl's choose-meter-gui), or what to check after a refused
# switch (switch-check-dial).
connection-failed-then = { $failed }. { $remedy }

# The × that closes a toast or a notice.
connection-dismiss =
    .hover = Cerrar
    .a11y = Cerrar la notificación
