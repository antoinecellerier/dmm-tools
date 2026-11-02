### The parts of dmm-cli's --help built as it runs: the first lines of the
### help of --device and --mock-mode, which go on with lists made from the
### meters, and the notes after the options. What each command and option
### does is in cli-help.ftl.
###
### Words in --flag form, setting names (device_family, mock_mode) and
### environment variables (RUST_LOG, LANG) are typed by the user and stay as
### they are. Command lines, file paths and values arrive as variables.

## The first lines of two options' long help (dmm-cli --help, dmm-cli read
## --help).

# Before the list of devices.
help-device-intro = Dispositivo al que conectarse.
# Before the list of the simulated meter's modes.
help-mock-mode-intro = Fijar el dispositivo simulado en un modo de medición concreto en lugar de recorrer los modos automáticamente. Solo tiene efecto con --device mock. Sin esta opción, lo fija el mock_mode del archivo de configuración (el modo simulado de dmm-gui).

## The words around every command's help: its headings, what follows an
## option, and the help of the built-in -h, -V and help. They are the same
## on every page of dmm-cli --help and dmm-cli <command> --help.

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

## After the options of dmm-cli -h.

help-see-long-help = Usa --help para ver la lista completa de dispositivos compatibles y la ruta del archivo de configuración compartido.
# $setting is NO_COLOR=1.
help-no-color = Establece { $setting } para desactivar la salida en color.

## After the options of dmm-cli --help: two sections, each a heading in
## capitals and indented lines. clap doesn't wrap these lines: keep them
## short.

help-configuration = CONFIGURACIÓN:
# The settings file's path follows on the next line.
help-settings-file = Archivo de configuración (compartido con dmm-gui):
# Where --device comes from when the command line doesn't give it: three
# numbered lines in order.
help-device-precedence = Prioridad de --device:
help-precedence-flag = Opción de la línea de comandos
help-precedence-settings = device_family del archivo de configuración indicado arriba
help-precedence-auto = auto: detectar el multímetro conectado
help-environment = ENTORNO:
# Beside RUST_LOG; each line of the message is a line of the help. $filter
# is the filter that shows everything the library sends and receives.
help-env-rust-log =
    Filtro del registro. Si no se define, se muestran las advertencias de dmm_lib y todos los errores.
    Usa `{ $filter }` para depurar a nivel de trama.
# Beside NO_COLOR.
help-env-no-color = Establecer en 1 para desactivar la salida en color.
# Beside LANGUAGE; each line of the message is a line of the help. The
# variable names stay as they are. On macOS and Windows the system's
# language applies instead.
help-env-language =
    Idioma de los mensajes y de la ayuda en Linux, en el orden en que lo lee gettext:
    LANGUAGE, después LC_ALL, LC_MESSAGES, LANG. Los escritorios Debian y Ubuntu
    definen LANGUAGE, que tiene prioridad sobre LANG.
# Beside LC_ALL; each line of the message is a line of the help. C and
# POSIX are the locale names that mean English, on every system.
help-env-lc-all =
    C o POSIX en la primera variable definida de LC_ALL, LC_MESSAGES y LANG
    significa inglés, diga lo que diga LANGUAGE.

## After the options of dmm-cli completions --help. The command line for
## each shell follows.

help-completions-install = Instala el autocompletado para tu shell:
