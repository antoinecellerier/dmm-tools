### Text both dmm-cli and dmm-gui show: the shared parts of their --help,
### the setup help when no meter is found, and the notices both print.
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
        [verified] La compatibilidad con el multímetro { $model } está verificada (verificada en hardware real).
        [partly-verified] La compatibilidad con el multímetro { $model } está parcialmente verificada (algunos modos y comandos aún no están confirmados en hardware real).
       *[experimental] La compatibilidad con el multímetro { $model } es experimental (no verificada en hardware real).
    }

## The --mock-mode flag's long help, after the binary's own first line.

# $modes lists the mock's modes, separated by commas.
mock-mode-list = Modos: { $modes }
# $example is a command line.
mock-mode-example = Ejemplo: { $example }
mock-clock-mock-only = --mock-clock-scale y --mock-clock-preseed solo se aplican al dispositivo simulado y a una reproducción. Vuelve a ejecutar con --device mock o --replay, o sin las opciones de reloj.

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
setup-bluetooth-check = activa el Bluetooth en este equipo y la transmisión de datos en el multímetro.
# Linux. Two command lines follow.
setup-usb-linux-rule = Asegúrate de que la regla udev esté instalada:
# Linux. The setup guide's address follows.
setup-usb-linux-replug = Después, vuelve a conectar el cable. En un equipo sin pantalla, incluye un grupo en la regla:
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
setup-bluetooth-adapter = Si ambos están activados, apaga el adaptador y vuelve a encenderlo.
setup-bluetooth-macos-privacy = Permite el acceso al Bluetooth en Ajustes del Sistema / Configuración del Sistema > Privacidad y seguridad > Bluetooth.
# The device catalog's address follows.
setup-bluetooth-catalog = Los adaptadores y multímetros compatibles figuran en el catálogo de dispositivos:
setup-bluetooth-scan = Ejecuta 'dmm-cli list' para buscar adaptadores y multímetros al alcance.

## Notices both binaries show.

# A cable or Bluetooth link is open but no meter on it identified itself.
# A notice title in the GUI; dmm-cli starts a sentence with it, naming the
# link after it (cli/connect.ftl).
no-meter-answered = Ningún multímetro respondió
# The meter picked or found has stopped answering. A notice title in the
# GUI; dmm-cli starts a sentence with it (cli/connect.ftl).
no-response = Sin respuesta del multímetro
# The GUI checkbox that lets a connection search over Bluetooth. The CLI
# reads the same setting, so its help names the checkbox too.
bluetooth-setting = Buscar dispositivos Bluetooth
bluetooth-off-cli-flag = Omite la opción --no-bluetooth.
bluetooth-off-cli-setting = Activa “{ bluetooth-setting }” en la configuración de dmm-gui, que dmm-cli también lee.
bluetooth-off-gui = Activa “{ bluetooth-setting }” en Configuración (⚙).

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
input-zero-scale = un factor de escala de cero borraría la lectura
# The alarm's low limit and its high limit.
input-low-not-below-high = el límite inferior debe ser menor que el superior
# The alarm's hysteresis band: a number in the limits' unit, or a percentage
# of the limit. $text is what was typed, as it was typed.
input-band-not-a-number = “{ $text }” no es un número ni un porcentaje como 1%
# A hysteresis band below zero.
input-band-negative = debe ser mayor o igual que cero
# A percentage band with a limit of 0, of which every percentage is 0.
input-percent-of-zero = un porcentaje de un límite de 0 siempre es 0; indica un valor en su lugar

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
read-error-no-header-row = sin fila de encabezado ({ $detail })
# $column is the name of a column every CSV export has, e.g. timestamp.
read-error-no-column = sin columna `{ $column }`
# A JSON export whose first line is not the one starting {"_metadata":.
read-error-no-metadata = sin línea `_metadata` ({ $detail })
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
