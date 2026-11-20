### Text both dmm-cli and dmm-gui show: the shared parts of their --help
### and the words clap puts around it, their command-line errors, the
### setup help when no meter is found, and the notices both print.
###
### Command lines, file paths and web addresses are never part of a message.
### Words in --flag form, and 'dmm-cli list', are typed by the user and stay
### as they are.

## The project page link: the last line of both binaries' --help, and the
## link in the GUI's top bar.

project-link = Ayuda y comentarios
# $url is the project page's address.
project-link-line = { project-link }: { $url }

## The --device flag's long help, after the binary's own first line. The
## device list between the heading and the aliases is generated.

device-list-heading = Dispositivos:
# What the "auto" device does. Also its hover in the GUI's device list.
device-auto = Detectar el multímetro conectado
# The "auto" row of the list: what the flag does when it isn't given.
device-auto-default = { device-auto } (predeterminado)
device-aliases =
    También acepta alias: ut61e+, ut61b, ut171a, ut181, etc.
    Pon entre comillas los nombres con caracteres especiales: --device 'ut61e+'
# Beside the simulated meters, in device lists.
device-no-hardware = no requiere hardware

## How far a meter's support is verified, beside it in device lists.

stability-verified = verificado
stability-partly-verified = parcialmente verificado
stability-experimental = experimental
# Both binaries show it for a meter short of verified, each followed by its
# own way to help verify it.
# $model is the meter's name, e.g. "UNI-T UT8803".
# $stability is verified, partly-verified or experimental.
stability-warning =
    { $stability ->
        [verified] La compatibilidad con el multímetro { $model } está verificada en hardware real.
        [partly-verified] La compatibilidad con el multímetro { $model } está parcialmente verificada (algunos modos y comandos aún no están confirmados en hardware real).
       *[experimental] La compatibilidad con el multímetro { $model } es experimental (no verificada en hardware real).
    }

## The help of options both binaries take, after the option in --help, as
## clap shows it: no full stop. Command lines ('dmm-cli list') and options
## (--adapter) stay as they are. Mock mode is the setting's name in
## dmm-gui's Settings, mock_mode its name in the settings file.

flag-adapter = Seleccionar un adaptador concreto cuando hay más de uno disponible. Indica el número de serie o la ruta HID de un cable USB, o la dirección de un adaptador o multímetro Bluetooth, tal como los muestra 'dmm-cli list'
# dmm-cli's, on read, get and set: the mock dmm-gui saved counts too.
flag-mock-mode-cli = Fijar el dispositivo simulado (--device mock o el que guardó dmm-gui) en un único modo de medición en lugar de recorrerlos todos. Si se omite, lo fija el valor mock_mode del archivo de configuración (Modo simulado en dmm-gui)
# dmm-gui's: the mock picked in Settings counts too.
flag-mock-mode-gui = Fijar el dispositivo simulado (--device mock o el elegido en Configuración) en un único modo de medición en lugar de recorrerlos todos. En esta sesión, anula el ajuste guardado Modo simulado
# dmm-cli's: its 'list' command shows what is plugged in.
flag-no-bluetooth-cli = Desactivar la búsqueda por Bluetooth en esta ejecución: 'list' solo muestra cables. Anula el ajuste “{ bluetooth-setting }” de dmm-gui. Una dirección indicada con --adapter se abre igualmente
flag-no-bluetooth-gui = Desactivar la búsqueda por Bluetooth en esta sesión. Anula el ajuste “{ bluetooth-setting }”. Una dirección indicada con --adapter se abre igualmente
# dmm-cli's, beside --no-bluetooth.
flag-bluetooth-cli = Buscar adaptadores y multímetros Bluetooth en esta ejecución, independientemente del ajuste “{ bluetooth-setting }” de dmm-gui

## The rest of --mock-mode's long help, after flag-mock-mode-cli or
## flag-mock-mode-gui.

# $modes lists the mock's modes, separated by commas.
mock-mode-list = Modos: { $modes }
# $example is a command line.
mock-mode-example = Ejemplo: { $example }

## Why a mock-only option was refused, after the option and the value as
## typed.

# The session's device, typed with --device or saved by dmm-gui, is not the
# mock.
mock-mode-mock-only = --mock-mode solo se aplica al dispositivo simulado. Vuelve a ejecutar con --device mock o sin --mock-mode.
mock-clock-mock-only = --mock-clock-scale y --mock-clock-preseed solo se aplican a un multímetro simulado y a una reproducción. Vuelve a ejecutar con --device mock o --replay, o sin las opciones de reloj.

## A link's short name, where the meter is already named: the GUI's
## status line, the setup help's section labels.

link-usb-cable = Cable USB
link-bluetooth = Bluetooth

## The help shown when no meter was found: a title, then one section per
## link searched — a label line, then steps. The CLI prints them in a
## terminal, the GUI in a notice.

# No full stop: the CLI adds one, the GUI's notice titles carry none.
not-found-usb = No se encuentra ningún cable USB
not-found-usb-or-bluetooth = No se encuentra ningún multímetro por USB ni por Bluetooth
# $address is a Bluetooth address or peripheral id.
not-found-bluetooth-at = No se encuentra ningún dispositivo Bluetooth en la dirección { $address }
# $model is the meter's name, e.g. "UNI-T UT60BT".
not-found-bluetooth-only = No se encuentra ningún multímetro { $model } por Bluetooth
# A section's label line.
# $link is link-usb-cable or link-bluetooth.
# $check is setup-usb-check or setup-bluetooth-check.
setup-label = { $link }: { $check }
setup-usb-check = comprueba que esté conectado y que el multímetro esté encendido.
setup-bluetooth-check =
    { $host ->
        [phone] activa el Bluetooth en este teléfono y la transmisión de datos en el multímetro.
       *[computer] activa el Bluetooth en este equipo y la transmisión de datos en el multímetro.
    }
# After "Bluetooth:", when the search was switched off; how to switch it on
# follows as a step of its own.
setup-bluetooth-off-check = no se buscaron dispositivos.
# Linux. Two command lines follow.
setup-usb-linux-rule = Asegúrate de que la regla udev esté instalada:
# Linux. The setup guide's address follows.
setup-usb-linux-replug = Después, vuelve a conectar el cable. En un equipo sin pantalla, añade un grupo a la regla:
# Windows. The quoted names are what Device Manager shows; use the names
# Windows shows in your language.
setup-usb-windows-open = Abre el Administrador de dispositivos con el cable conectado:
setup-usb-windows-cp2110 = - 'CP2110 USB to UART Bridge' en Dispositivos de interfaz humana: no requiere ninguna acción.
# UNVERIFIED Windows name
setup-usb-windows-input = - 'Dispositivo de entrada USB' en Dispositivos de interfaz humana: no requiere ninguna acción.
# The driver's download address follows.
setup-usb-windows-warning = - Icono de advertencia amarillo en 'Otros dispositivos': instala el controlador desde
setup-usb-windows-nothing = - No aparece nada: prueba otro puerto USB.
# macOS. Name the settings as macOS shows them in your language.
setup-usb-macos-auto = El cable debería reconocerse automáticamente (no requiere controlador).
setup-usb-macos-privacy = Si no se encuentra el dispositivo, comprueba Ajustes del Sistema / Configuración del Sistema > Privacidad y seguridad > Monitorización de entrada.
setup-bluetooth-adapter =
    { $host ->
        [phone] Si ambos están activados, desactiva el Bluetooth de este teléfono y vuelve a activarlo.
       *[computer] Si ambos están activados, apaga el adaptador y vuelve a encenderlo.
    }
setup-bluetooth-macos-privacy = Permite el acceso al Bluetooth en Ajustes del Sistema / Configuración del Sistema > Privacidad y seguridad > Bluetooth.
# The device catalog's address follows.
setup-bluetooth-catalog = Los adaptadores y multímetros compatibles figuran en el catálogo de dispositivos:
setup-bluetooth-scan = Ejecuta 'dmm-cli list' para buscar adaptadores y multímetros al alcance.

## Notices both binaries show.

# The settings file names a meter this version has no entry for: a file a
# newer version wrote, or one edited by hand. $device is the id as the file
# holds it, e.g. ut62; $path is the file. dmm-gui shows it in a toast,
# dmm-cli after "Warning:" and before what to do.
saved-device-unknown = El archivo de configuración { $path } indica el dispositivo “{ $device }”, que esta versión no conoce, así que el multímetro se detecta automáticamente.
# The settings file's mock_mode (the setting's name in the file, kept as it
# is) names no scenario of the simulated meter. $given is the name as the
# file holds it. Each app follows it with what to do.
saved-mock-mode-unknown = El valor “{ $given }” de mock_mode en el archivo de configuración no es un modo simulado, así que el dispositivo simulado recorre todos los modos.
# When the meter opened, other cables were there to open and the session
# took the first, with --adapter a command-line option kept as it is. $count
# is 2 or more. dmm-gui shows it in a toast, dmm-cli after "Warning:".
cables-at-open =
    { $host ->
        [phone]
            { $count ->
                [one] Hay { $count } cable USB conectado que esta sesión podría abrir. Se abrió ese.
               *[other] Hay { $count } cables USB conectados que esta sesión podría abrir. Abrió el primero que encontró.
            }
       *[computer]
            { $count ->
                [one] Hay { $count } cable USB conectado que esta sesión podría abrir. Se abrió ese. Usa --adapter para elegir otro.
               *[other] Hay { $count } cables USB conectados que esta sesión podría abrir. Abrió el primero que encontró. Usa --adapter para elegir otro.
            }
    }
# When the meter opened over Bluetooth, other adapters or meters matched as
# well and the session took the first, with --adapter a command-line option
# kept as it is. $count is 2 or more. $host as in cables-at-open.
bluetooth-tied =
    { $host ->
        [phone] Hay { $count } adaptadores o multímetros Bluetooth que coinciden igual de bien. Esta sesión abrió el primero que encontró.
       *[computer] Hay { $count } adaptadores o multímetros Bluetooth que coinciden igual de bien. Esta sesión abrió el primero que encontró. Para elegir otro, usa --adapter con uno de los que muestra 'dmm-cli list'.
    }
# Follows cables-at-open when nothing tells those cables apart.
cables-no-reconnect =
    { $host ->
        [phone] Estos cables no se pueden distinguir entre sí: si se pierde el enlace, la sesión terminará.
       *[computer] Estos cables no se pueden distinguir entre sí: si se pierde el enlace, la sesión terminará, a menos que indiques la ruta de un cable con --adapter.
    }
# The meter named another model than the one picked. $reported is the name
# it gave, e.g. UT161E; $picked the model picked, e.g. UNI-T UT61E+. Each
# app follows it with what to pick (choose-meter-gui, choose-meter-cli).
other-model = El multímetro se identifica como “{ $reported }”, no como el modelo seleccionado, { $picked }.
# The same when detection opened the meter, without hearing its name in
# time, as $picked; $reported as above.
other-model-detected = El multímetro se identifica como “{ $reported }”; la detección lo abrió como el modelo { $picked }.
# The meter's packets are another model's than the one it was opened as:
# $named is that model, e.g. ZOYI / ZOTEK ZT-5566SE; $opened the one opened,
# e.g. ZOYI / ZOTEK ZT-300AB. Each app follows it with what to pick.
other-data = El multímetro envía datos del modelo { $named }, no del modelo { $opened } elegido al abrir la conexión.
# The meter sent data this tool doesn't read yet, said once per run. $meter
# is the meter, e.g. UNI-T UT61E+; $detail what it sent, in English, e.g.
# mode byte: 0x7f. Each app follows it with how to report it.
unrecognised-data = El multímetro { $meter } envió datos que esta herramienta no reconoce ({ $detail }).
# A cable or Bluetooth link is open but no meter on it identified itself.
# A notice title in the GUI; dmm-cli starts a sentence with it, naming the
# link after it (cli/connect.ftl).
no-meter-answered = Ningún multímetro respondió
# The meter picked or found has stopped answering. A notice title in the
# GUI; dmm-cli starts a sentence with it (cli/connect.ftl).
no-response = El multímetro no responde
# Why a dropped link is not reconnected. $count (2 or more) USB cables the
# open could take are plugged in, with no serial number of their own to tell
# them apart. Each binary follows it with what to do.
several-cables =
    { $count ->
        [one] Hay { $count } cable USB conectado que esta sesión podría abrir y que no se puede distinguir de los demás. Al reconectar podría abrirse el de otro multímetro.
       *[other] Hay { $count } cables USB conectados que esta sesión podría abrir y que no se pueden distinguir entre sí. Al reconectar podría abrirse el de otro multímetro.
    }
# Why a dropped link is no longer being reopened: it stayed down for
# $minutes minutes, and the session has ended. A notice or the error that
# ended the run says what the last attempt found.
reconnect-gave-up =
    { $minutes ->
        [one] Reconexión abandonada: el enlace con el multímetro estuvo interrumpido durante { $minutes } minuto.
       *[other] Reconexión abandonada: el enlace con el multímetro estuvo interrumpido durante { $minutes } minutos.
    }
# While a dropped link is being reopened: its cable has gone and another is
# plugged in. Each binary follows it with how to use that other cable.
waiting-for-own-cable = Hay otro cable USB conectado, pero esta sesión espera el que abrió.
# Under a reading the meter shows on its own screen only, its frames carrying
# none: a CMS or UT217's oscilloscope mode.
screen-only-reading = Consulta la pantalla del multímetro para ver las lecturas del modo osciloscopio
# The GUI checkbox that lets a connection search over Bluetooth. The CLI
# reads the same setting, so its help names the checkbox too.
bluetooth-setting = Buscar dispositivos Bluetooth
# How dmm-cli switches the search on: after --no-bluetooth, and when the
# saved setting is off.
bluetooth-off-cli-instead = Usa --bluetooth en lugar de --no-bluetooth.
bluetooth-off-cli-saved = Usa --bluetooth o activa “{ bluetooth-setting }” en la configuración de dmm-gui, que dmm-cli también lee.
bluetooth-off-gui = Activa “{ bluetooth-setting }” en Configuración (⚙).
# What to check after the meter refused a mode or range switch, or did not
# take it. $setting is range (the input must be within the range too) or
# another setting.
switch-check-dial =
    { $setting ->
        [range] Comprueba la posición del selector y que la entrada esté dentro del rango.
       *[other] Comprueba la posición del selector.
    }
# How to open the meter as the model it is, after a message saying it is
# another, in dmm-gui. $choice is entry ($model the meter to pick, e.g.
# OWON CMS061), entry-only (the same, when Auto-detect is what opened it),
# auto-detect, or own-model (nothing says which model: the user has to pick
# it). "Auto-detect" is the Device choice that finds the meter itself.
choose-meter-gui =
    { $choice ->
        [entry] Elige el multímetro { $model } o Detección automática en Configuración (⚙).
        [entry-only] Elige el multímetro { $model } en Configuración (⚙).
        [auto-detect] Elige Detección automática en Configuración (⚙).
       *[own-model] Elige en Configuración (⚙) el modelo real del multímetro.
    }
# The same in dmm-cli: $choice as above, $id the --device value of the
# meter to pick, e.g. cms061; --device auto detects the meter.
choose-meter-cli =
    { $choice ->
        [entry] Usa --device { $id } o --device auto.
        [entry-only] Usa --device { $id }.
        [auto-detect] Usa --device auto.
       *[own-model] Usa --device con el modelo real del multímetro; --help muestra la lista.
    }

## What is connected, when an --adapter value matched nothing.

adapters-none = No hay dispositivos conectados en este momento.
adapters-heading = Dispositivos conectados:
adapters-unavailable = Ejecuta 'dmm-cli list' para ver los dispositivos conectados.
# $address is the value the user typed, written with colons, e.g. 12:34:56:78:9A:BC.
colonless-address-hint = Si es una dirección Bluetooth, escríbela como { $address }.

## Why a number typed for a scale, an offset or an alarm was refused: the
## reason alone, in lower case and with no full stop. dmm-gui puts it after
## the field's name in a toast ("Invalid scale: …"); dmm-cli after the
## option and the value, at the end of a line from its command-line parser,
## which stays English ("error: invalid value '0' for '--scale <FACTOR>': …").

# $text is what was typed, as it was typed.
input-not-a-number = “{ $text }” no es un número
# Infinity or "not a number" (NaN).
input-not-finite = debe ser un número finito
# A scale factor of 0 would turn every reading into the offset.
input-zero-scale = un factor de escala igual a cero borraría la lectura
# The alarm's low limit and its high limit.
input-low-not-below-high = el límite inferior debe ser menor que el superior
# An alarm limit: a number, or a number with a unit or a prefix such as
# 4980 mV, 10 kohm or 4.7k (kept as they are). $text is what was typed.
input-not-a-quantity = “{ $text }” no es un número ni un número con unidad como 4980 mV o 4.7k
# The alarm's two limits were typed with units of different quantities,
# such as volts and amps.
input-limit-units-differ = los límites inferior y superior deben estar en la misma unidad
# The alarm's hysteresis band: a number in the limits' unit, or a percentage
# of the limit. $text is what was typed, as it was typed.
input-band-not-a-number = “{ $text }” no es un número ni un porcentaje como 1%
# A hysteresis band below zero.
input-band-negative = debe ser mayor o igual que cero
# A hysteresis band typed with a unit, while the limits were typed in
# another unit or with none.
input-band-not-the-limits-unit = la banda solo admite una unidad si los límites se escriben en esa misma unidad
# A percentage band with a limit of 0, of which every percentage is 0.
input-percent-of-zero = con un límite de 0, cualquier porcentaje da 0; indica un valor en su lugar

## Why an exported file or a recording did not read: in dmm-gui's toast
## after "Import failed:" and the file, in dmm-cli after "Error:" and the
## file. Lower case, with no full stop. The backquotes (`) set off text
## from the file, or a name it must have, which stays as it is: keep them.
## A parser's own words ($detail) arrive in English.

# Where the file stopped: $line is a line number, $reason one of the
# reasons below.
read-error-at-line = línea { $line }: { $reason }
read-error-empty = el archivo está vacío
# A CSV export without its first row naming the columns.
read-error-no-header-row = falta la fila de encabezado ({ $detail })
# $column is the name of a column every CSV export has, e.g. timestamp.
read-error-no-column = falta la columna `{ $column }`
# A JSON export whose first line is not the one starting {"_metadata":.
read-error-no-metadata = falta la línea `_metadata` ({ $detail })
# $text is what the file has where a reading's value should be.
read-error-not-a-value = `{ $text }` no es un valor
# $text is a cell starting NCV: whose level is no number. NCV is
# non-contact voltage detection, as the meter prints it.
read-error-not-an-ncv-level = `{ $text }` no es un nivel NCV
# $text is the part of a flags cell that names no status flag.
read-error-not-a-flag = `{ $text }` no es un indicador
# $text is what the file has where a marker's number should be.
read-error-not-a-marker-number = `{ $text }` no es un número de marcador
# $text is a timestamp from the file, or a recording's "# recorded:" line;
# RFC 3339 is the standard date format the files use.
read-error-not-a-date = `{ $text }` no es una fecha RFC 3339: { $detail }

## The words clap puts around every page of dmm-cli's and dmm-gui's --help:
## its headings, what follows an option, and the help of the built-in -h,
## -V and help. They are the same on every page.

# The headings, each before its list.
help-usage = Uso:
help-commands = Comandos:
help-arguments = Argumentos:
help-options = Opciones:
# After an option's help, in brackets. $value is what the option is when not
# given, e.g. 500.
help-default = [predeterminado: { $value }]
# After an option's help, in brackets. $values is the values it takes,
# separated by commas.
help-possible-values = [valores posibles: { $values }]
# Before the list of values an option takes, each with what it does.
help-possible-values-heading = Valores posibles:
# What -h and --help do: on a command with more detail under --help, the two
# notes say how to see more or less.
help-flag = Mostrar la ayuda
help-flag-short = Mostrar la ayuda (más detalles con '--help')
help-flag-long = Mostrar la ayuda (resumen con '-h')
# What -V and --version do.
help-version-flag = Mostrar la versión
# The help command, listed among the commands, and its argument.
help-command = Mostrar este mensaje o la ayuda de los subcomandos indicados
help-command-subcommand = Mostrar la ayuda de los subcomandos indicados

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
args-error = error:

# An option that takes no value after "=" was given without one.
args-equals-needed = se necesita un signo igual para asignar valores a '{ $arg }'
# An option was given without its value.
args-value-required = '{ $arg }' necesita un valor, pero no se indicó ninguno
# A value the option doesn't take. A list of the ones it takes may follow
# on the next line (args-possible-values).
args-invalid-value = valor '{ $value }' no válido para '{ $arg }'
# The same, with the reason the value was refused. $reason may be English.
args-invalid-value-because = valor '{ $value }' no válido para '{ $arg }': { $reason }
# Why a --device value was refused, as $reason above: no meter has that
# name.
device-unknown = no existe ese dispositivo; usa --help para ver la lista
# The values an option takes, on a line of their own under the error. $values
# is the list, separated by commas.
args-possible-values = [valores posibles: { $values }]
# A command the program doesn't have, e.g. 'lst'.
args-unrecognized-subcommand = subcomando '{ $command }' no reconocido
# Options that must be given, listed one per line under this.
args-missing-required = no se indicaron los siguientes argumentos obligatorios:
# The program was run without a command. $command is the program, e.g.
# dmm-cli. The commands follow on the next line (args-subcommands).
args-missing-subcommand = '{ $command }' necesita un subcomando, pero no se indicó ninguno
# The commands, on a line of their own under the error. $values is the list,
# separated by commas.
args-subcommands = [subcomandos: { $values }]
# A value after an option that takes no more.
args-unexpected-value = valor '{ $value }' inesperado para '{ $arg }'; no se esperaban más
# Too few values after an option. $needed and $given are numbers; $count is
# $given, for choosing the verb's form ([0] and [one]: one value or none).
args-too-few-values =
    { $count ->
        [0] '{ $arg }' necesita { $needed } valores; solo se indicaron { $given }
        [one] '{ $arg }' necesita { $needed } valores; solo se indicó { $given }
       *[other] '{ $arg }' necesita { $needed } valores; solo se indicaron { $given }
    }
# The wrong number of values after an option. $needed and $given are
# numbers; $count is $given, for choosing the verb's form.
args-wrong-number-of-values =
    { $count ->
        [0] '{ $arg }' necesita { $needed } valores, pero se indicaron { $given }
        [one] '{ $arg }' necesita { $needed } valores, pero se indicó { $given }
       *[other] '{ $arg }' necesita { $needed } valores, pero se indicaron { $given }
    }
# An option the program doesn't have, or one this command doesn't take.
args-unexpected-argument = argumento '{ $arg }' inesperado
# An option given twice that can only be given once.
args-repeated = el argumento '{ $arg }' no se puede usar más de una vez
# Two options, or an option and a command, that can't go together. $what is
# argument or subcommand; $name and $other are what the user typed.
args-conflict-with =
    { $what ->
        [subcommand] el subcomando '{ $name }' no se puede usar con '{ $other }'
       *[argument] el argumento '{ $name }' no se puede usar con '{ $other }'
    }
# Two options, or an option and a command, that can't go together, with
# the others listed one per line under this. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-list =
    { $what ->
        [subcommand] el subcomando '{ $name }' no se puede usar con:
       *[argument] el argumento '{ $name }' no se puede usar con:
    }
# The same, when the others aren't named. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-others =
    { $what ->
        [subcommand] el subcomando '{ $name }' no se puede usar con uno o más de los otros argumentos indicados
       *[argument] el argumento '{ $name }' no se puede usar con uno o más de los otros argumentos indicados
    }

## Hints after the error.

# Before a hint, indented on its own line.
args-tip = sugerencia:
# A name close to what was typed. $what is subcommand, argument or value;
# $names is one or more quoted names, separated by commas; $count is how
# many.
args-similar =
    { $what ->
        [subcommand]
            { $count ->
                [one] existe un subcomando parecido: { $names }
               *[other] existen subcomandos parecidos: { $names }
            }
        [argument]
            { $count ->
                [one] existe un argumento parecido: { $names }
               *[other] existen argumentos parecidos: { $names }
            }
       *[value]
            { $count ->
                [one] existe un valor parecido: { $names }
               *[other] existen valores parecidos: { $names }
            }
    }
# The last line. $help is --help.
args-more-info = Para obtener más información, prueba '{ $help }'.
