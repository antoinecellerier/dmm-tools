### The library's errors: why a connection, a command or a file failed.
### They show in the GUI's connection notice under the reading and in its
### toasts (the short messages under the top bar's right end), and in the
### CLI after "Error:". Hand-written; a test checks each English against
### the library's own. The last group is the library's one notice that is
### not an error.
###
### Mode, range and state names arrive already translated, as the reading
### shows them. Button names (SELECT, RANGE, AUTO, EXIT) are printed on the
### meter: keep them as they are, as command-line options (--adapter,
### --device) and command words (auto, rel). Text the operating system or
### the Bluetooth stack wrote arrives as it is, in their language.

## What went wrong, by the part that failed. Several end in a colon and
## the reason, which comes from one of the groups below.

# The USB layer failed. $detail is what it said, in English.
error-hid = error HID: { $detail }
# The meter sent something that makes no sense. $detail says what, in
# English, for a bug report.
error-invalid-response = respuesta no válida: { $detail }
# The meter on the link is not one the meter chosen can read, worded as
# error-invalid-response. $reason is one of the "Another meter" sentences
# below.
error-wrong-meter = respuesta no válida: { $reason }
# A frame from the meter failed its check. $expected and $actual are
# hexadecimal numbers, e.g. 0x01a3.
error-checksum-mismatch = suma de comprobación incorrecta: se esperaba { $expected }, se recibió { $actual }
error-timeout = tiempo de espera agotado sin respuesta
# The meter reports a mode this tool does not know. $code is a hexadecimal
# number, e.g. 0x1f.
error-unknown-mode = modo desconocido: { $code }
# $reason is one of the "Refused before it was sent" sentences below.
error-unsupported-command = comando no admitido: { $reason }
# $reason is one of the "The meter did not do it" sentences below.
error-command-rejected = comando rechazado: { $reason }
# $device is the meter's name or ID, as typed with --device or as the
# settings file holds it, sometimes with an explanation in English.
error-unknown-device = dispositivo desconocido: { $device }
# A meter on a cable was opened as a simulated one. $device is its ID, e.g.
# ut61eplus.
error-not-simulated = dispositivo desconocido: { $device } no es un dispositivo simulado
# A recording could not be played back. $detail names the file or the line
# and says what is wrong with it, in English.
error-replay = archivo de reproducción: { $detail }
# $adapter is the value given with --adapter, sometimes with an explanation
# in English.
error-adapter-not-found = no se encuentra el adaptador: { $adapter }
# What --adapter named, as error-adapter-not-found's $adapter and the GUI's
# connection-adapter-not-matched quote it, when it is a USB device that is
# no supported cable. $path is the device's path, as given with --adapter.
error-not-an-adapter = { $path } (el dispositivo existe, pero no es un adaptador USB compatible)
# --adapter named a cable or adapter the chosen meter cannot be reached
# through. $link is bluetooth (a Bluetooth adapter) or cable (a USB cable
# whose name is $cable, e.g. BU-86X). $model is the meter, e.g. UNI-T
# UT61E+.
error-wrong-cable =
    { $link ->
        [bluetooth] --adapter indica un adaptador Bluetooth, que no sirve para el multímetro { $model }: indica con --adapter el cable propio del multímetro u omite --adapter
       *[cable] --adapter indica un cable { $cable }, que no sirve para el multímetro { $model }: indica con --adapter el cable propio del multímetro u omite --adapter
    }
error-link-lost = Enlace Bluetooth perdido
# $reason is one of the "Bluetooth" sentences below.
error-bluetooth = Bluetooth: { $reason }
# A cable or Bluetooth link is up but no meter answered on it. $link is usb
# (a USB cable), adapter (a Bluetooth adapter plugged into the meter) or
# radio (the meter's own Bluetooth).
error-device-not-identified =
    { $link ->
        [adapter] ningún multímetro respondió a través del adaptador Bluetooth
        [radio] ningún multímetro respondió a través del enlace Bluetooth
       *[usb] ningún multímetro respondió a través del cable USB
    }
# No cable was found. $bluetooth is searched when Bluetooth was searched
# too, else not-searched.
error-no-transport-found =
    { $bluetooth ->
        [searched] no se encuentra ningún adaptador USB compatible ni ningún dispositivo Bluetooth al alcance
       *[not-searched] no se encuentra ningún adaptador USB compatible
    }
# As error-no-transport-found, naming the cables looked for; $bluetooth as
# there. $cables lists them with their USB IDs, e.g. "CP2110 0x10c4:0xea80,
# BU-86X 0x0820:0x0001".
error-no-transport-found-looked-for =
    { $bluetooth ->
        [searched] no se encuentra ningún adaptador USB compatible (cables buscados: { $cables }) ni ningún dispositivo Bluetooth al alcance
       *[not-searched] no se encuentra ningún adaptador USB compatible (cables buscados: { $cables })
    }
# A meter with Bluetooth built in, and none was in range. $model is the
# meter, e.g. UNI-T UT60BT.
error-bluetooth-only-not-in-range = no se encuentra ningún multímetro { $model } al alcance del Bluetooth
# A meter with Bluetooth built in, and Bluetooth was not searched. $model
# is the meter, e.g. UNI-T UT60BT. Each app adds how to switch it on.
error-bluetooth-only-switched-off = el multímetro { $model } solo se conecta por Bluetooth y el Bluetooth está desactivado
# $model is the meter, e.g. UNI-T UT60BT.
error-bluetooth-only-not-built = el multímetro { $model } solo se conecta por Bluetooth y esta compilación no admite Bluetooth
# $model is the meter, e.g. UNI-T UT60BT.
error-bluetooth-only-usb-adapter = el multímetro { $model } solo se conecta por Bluetooth y --adapter indica un dispositivo USB: indica la dirección Bluetooth del multímetro u omite --adapter

## Refused before it was sent: the meter, the link or the dial position
## cannot do it. After "unsupported command:". $setting is a setting's
## name as the CLI takes it (mode, range, rel, minmax…); $word, $target and
## $from_word are a mode's hexadecimal code, e.g. 0x3111.

# $setting is a setting's name.
error-unsupported-setting = el ajuste { $setting } no se puede cambiar en este multímetro
# $mode is a mode.
error-unsupported-several-positions = el multímetro indica el modo { $mode } en más de una posición del selector: toma primero una lectura en otro modo
# $mode is a mode.
error-unsupported-no-position = el modo { $mode } no está en ninguna posición conocida del selector
# $target and $from are modes.
error-unsupported-mode-unreachable = no se puede llegar al modo { $target } desde el modo { $from }: gira primero el selector
# $target and $from_word are mode codes, $from is the mode $from_word is.
error-unsupported-word-unreachable = no se puede llegar al modo { $target } desde el modo { $from } ({ $from_word }): gira primero el selector
# $mode is a mode.
error-unsupported-no-ranges = el modo { $mode } no tiene ningún rango que elegir en este multímetro
# $id is a range's number, $count how many ranges $mode, a mode, has.
error-unsupported-no-such-range = el rango { $id } no es uno de los { $count } que ofrece el modo { $mode }
# $id is a range's number, $count how many ranges $mode, a mode, has.
error-unsupported-range-not-listed = el rango { $id } no es uno de los { $count } rangos que ofrece el modo { $mode }
# $count is how many ranges $mode, a mode, has.
error-unsupported-auto-range-not-listed = el rango automático no es uno de los { $count } rangos que ofrece el modo { $mode }
# $mode is a mode, $word its code.
error-unsupported-fixed-range = rango en modo { $mode } ({ $word }): modo de rango fijo
# $setting is a setting's name, $mode a mode.
error-unsupported-setting-not-in-mode = el ajuste { $setting } no se puede cambiar en modo { $mode } en este multímetro
# $setting is a setting's name, $mode a mode, $word its code.
error-unsupported-setting-not-in-word = el ajuste { $setting } no se puede cambiar en modo { $mode } ({ $word }) en este multímetro
# $setting is a setting's name, $id a state's number, $mode a mode and
# $offered the states it offers, separated by commas (on, off, MAX…).
error-unsupported-no-such-state = el ajuste { $setting } no tiene el estado { $id }; el modo { $mode } ofrece { $offered }
# $command is a command word, $mode a Peak mode, $word its code.
error-unsupported-refused-in-peak = { $command } en modo { $mode } ({ $word }): el multímetro lo rechaza en modo pico
# "rel" is the command word. $mode is a mode, $word its code.
error-unsupported-no-rel = rel en modo { $mode } ({ $word })
# $model is the meter's part number, e.g. VC-880; $button a button's name.
error-unsupported-no-button = el multímetro { $model } no tiene el botón { $button }
# $cable is a cable's name, e.g. BU-86X; $len and $got are byte counts.
error-unsupported-request-length = el cable { $cable } acepta peticiones de { $len } bytes, no de { $got } bytes
# $link is usb (the USB cable), adapter (the Bluetooth adapter) or none (a
# link that does not say what it is); $baud a serial speed, e.g. 19200.
error-unsupported-baud =
    { $link ->
        [usb] el cable USB no puede cambiar su velocidad a { $baud } baudios
        [adapter] el adaptador Bluetooth no puede cambiar su velocidad a { $baud } baudios
       *[none] el enlace no puede cambiar su velocidad a { $baud } baudios
    }
# A developer's error: the simulated meter has no bytes to decode.
error-unsupported-no-wire-format = parse_payload: el dispositivo simulado no tiene formato de transmisión

## The meter did not do it: a command it was sent did not take, or the
## meter's state says it cannot. After "command rejected:". $button is a
## button's name; $state, $target and $now are modes, ranges or setting
## states.

# $button is a button's name, $state what the meter stayed in.
error-rejected-did-nothing = { $button } no tuvo efecto en { $state }
# $target is what was asked for, $now what the meter is in.
error-rejected-never-appeared = { $target } no llegó a aparecer; el multímetro volvió a { $now }
# $presses is how many times $button was pressed; $state is what the meter
# is in.
error-rejected-gave-up =
    { $presses ->
        [one] se abandonó el intento tras presionar { $presses } vez el botón { $button }; el multímetro está en { $state }
       *[other] se abandonó el intento tras presionar { $presses } veces el botón { $button }; el multímetro está en { $state }
    }
# $mode is a mode.
error-rejected-mode-changed = el multímetro cambió al modo { $mode }; se dejó de presionar RANGE
# $reason is another sentence of this group, $mode the mode the meter was
# in before.
error-rejected-started-in = { $reason } (modo inicial: { $mode })
# $range is a range.
error-rejected-auto-did-nothing = AUTO no tuvo efecto; el multímetro sigue en el rango { $range }
# $button is a button's name, $state a setting's state.
error-rejected-exit-did-nothing = EXIT { $button } no tuvo efecto; el multímetro sigue en { $state }
# $setting is a setting's name, $target and $now its states.
error-rejected-switch-did-nothing = { $setting } { $target } no tuvo efecto; el multímetro sigue en { $now }
# $sent is what was sent: a command word, or one of the error-sent messages
# below, or a setting's name and a state.
error-rejected-by-meter = el multímetro rechazó { $sent }
error-rejected-zero-needs-capacitance = ZERO solo funciona en modo capacitancia
# $code is a number, $model and $opened are meters (e.g. OWON B41T+), $id a
# meter's ID as --device takes it. "Auto-detect" is the GUI's Device
# choice.
error-rejected-other-model = el multímetro indica el código de modelo OWON { $code } ({ $model }) y no es el multímetro { $opened } elegido al abrir la conexión, así que los botones remotos están desactivados; elige Detección automática o --device { $id }
# $code is a number.
error-rejected-unknown-model = el multímetro indica el código de modelo OWON { $code }, que esta herramienta aún no conoce, así que los botones remotos están desactivados
# $command is a command word.
error-rejected-model-unread = no se leyó el código de modelo del multímetro, así que el comando { $command } está desactivado: mantén presionado el botón REL del multímetro para salir de REL
error-rejected-min-max-locks-range = MIN/MAX bloquea el rango; sal de MIN/MAX antes de cambiarlo
# $command is a command word or a setting's name.
error-rejected-replay = { $command }: una reproducción no admite comandos

# What a refused command switched to, as error-rejected-by-meter names it.
# $mode is a mode.
error-sent-mode = el modo { $mode }
# $range is a range.
error-sent-range = el rango { $range }

## Another meter: the one on the link is not one the meter chosen can
## read. After "invalid response:". Model names (OWON B35T+, Brymen BM82x)
## and --device's values (vc871, bm82x) stay as they are. Auto-detect is
## the meter choice that finds the meter itself: the GUI's Device list
## names it so, and the CLI takes it as --device auto.

# An OWON meter names its model, which sends the other kind of data frame.
# $code is OWON's model number, e.g. 87; $model the model it names, e.g.
# Voltcraft VC871; $frame the length of that model's frame in bytes, 6 or
# 15; $opened the meter chosen, e.g. OWON B35T+; $id the --device value
# for $model.
error-wrong-meter-other-frame = el multímetro indica el código de modelo OWON { $code } ({ $model }), que envía la trama OWON de { $frame } bytes, y no es el multímetro { $opened } elegido al abrir la conexión; elige Detección automática o --device { $id }
# An OWON meter sends the other kind of data frame and names no model.
# $sent and $expected are frame lengths in bytes, 6 or 15; $opened is the
# meter chosen, e.g. Voltcraft VC871. $remedy is auto-detect, or own-model
# when the user has to pick the meter's model themselves.
error-wrong-meter-other-frame-kind =
    { $remedy ->
        [auto-detect] el multímetro envía tramas OWON de { $sent } bytes, no las tramas de { $expected } bytes del multímetro { $opened } elegido al abrir la conexión; elige Detección automática
       *[own-model] el multímetro envía tramas OWON de { $sent } bytes, no las tramas de { $expected } bytes del multímetro { $opened } elegido al abrir la conexión; elige el modelo exacto del multímetro
    }
# An OWON model this tool does not read yet. $code is OWON's model number,
# e.g. 83. "This message" is the whole error, for the issue.
error-wrong-meter-not-yet = el multímetro indica el código de modelo OWON { $code }, que esta herramienta aún no lee; abre una incidencia (issue) con este mensaje
# An OWON model whose readings carry their sign where this tool does not
# read it yet. $code is OWON's model number, e.g. 55.
error-wrong-meter-sign-in-function-word = el multímetro indica el código de modelo OWON { $code }: envía la trama OWON de 6 bytes con el signo en la palabra de función, que esta herramienta aún no lee
# A Brymen meter of another series answered. $found and $opened are series,
# e.g. Brymen BM82x and Brymen BM86x; $id is the --device value for $found.
error-wrong-meter-other-series = el multímetro conectado al cable es de la serie { $found }, no de la serie { $opened }; elige Detección automática o --device { $id }
# An OWON meter chosen by its model sends bytes, but never its 6-byte data
# frame. B35 and B35T are OWON models.
error-wrong-meter-no-six-byte-frames = el multímetro no envía tramas OWON de 6 bytes; puede ser un B35 o un B35T antiguo, cuyas tramas de 14 bytes esta herramienta aún no lee
# The same for a meter whose data frame is 15 bytes long.
error-wrong-meter-no-fifteen-byte-frames = el multímetro no envía tramas OWON de 15 bytes; elige Detección automática
# An OWON meter that names no model sends frames of neither length. "This
# message" is the whole error, for the issue.
error-wrong-meter-untold-frame-kind = ni el código de modelo del multímetro ni sus tramas indican si envía tramas OWON de 6 o de 15 bytes; abre una incidencia (issue) con este mensaje

## Bluetooth: what the Bluetooth stack could not do. After "Bluetooth:".

# $detail is what the operating system said.
error-bluetooth-runtime = no se pudo iniciar el entorno de ejecución de Bluetooth: { $detail }
error-bluetooth-no-adapter = no disponible en este equipo
error-bluetooth-powered-off = desactivado en este equipo
# $role is write (the characteristic commands are written to) or notify
# (the one the meter's data arrives on).
error-bluetooth-no-characteristic =
    { $role ->
        [write] el dispositivo Bluetooth no tiene ninguna característica de escritura de datos conocida: no es un adaptador ni un multímetro compatible, o sus servicios nunca se resolvieron
       *[notify] el dispositivo Bluetooth no tiene ninguna característica de notificación de datos conocida: no es un adaptador ni un multímetro compatible, o sus servicios nunca se resolvieron
    }
# A Brymen meter refused the password this tool sends.
error-bluetooth-password-refused = el multímetro rechazó la contraseña de conexión 0000: para restablecerla, { error-bluetooth-reset-gesture }
# How a Brymen BM788BT/BM787BT's password and name go back to the factory
# settings. The meter's activation steps in lib.ftl (activation-bm78xbt)
# end with the same sentence: word them alike. "Org" is what the display
# shows.
error-bluetooth-reset-gesture = mantén presionado el botón Hz mientras giras el selector de OFF a capacitancia en menos de 0.6 s: el multímetro muestra “Org”, y la contraseña de conexión y el nombre Bluetooth vuelven a los valores de fábrica
# A Brymen meter refused the login with error $code, a number.
error-bluetooth-login-refused =
    { $code ->
        [0] el multímetro rechazó el inicio de sesión: error de suma de comprobación (código { $code })
        [1] el multímetro rechazó el inicio de sesión: identificador de canal no válido (código { $code })
        [2] el multímetro rechazó el inicio de sesión: fuera del rango de ajuste (código { $code })
        [5] el multímetro rechazó el inicio de sesión: argumentos no válidos (código { $code })
        [6] el multímetro rechazó el inicio de sesión: permisos insuficientes (código { $code })
        [one] el multímetro rechazó el inicio de sesión: error desconocido (código { $code })
       *[other] el multímetro rechazó el inicio de sesión: error desconocido (código { $code })
    }

## Not an error: the notice under the reading in the GUI when a Bluetooth
## adapter hands over a reading late, together with the next one.

late-readings-title = Algunas lecturas Bluetooth llegan con retraso
# $fix is command when a command follows that fixes it for this connection
# (it ends in a colon, and the command shows on the next line as it is), or
# none.
late-readings-advice =
    { $fix ->
        [command] En este adaptador, a veces una lectura llega agrupada con la siguiente. No se pierde ninguna. Para tener marcas de tiempo exactas hasta que se desconecte el multímetro, ejecuta:
       *[none] En este adaptador, a veces una lectura llega agrupada con la siguiente. No se pierde ninguna.
    }
