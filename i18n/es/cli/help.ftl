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
