### The parts of dmm-cli's --help built as it runs: the first line of the
### help of --device, which goes on with a list made from the meters, and
### the notes after the options. What each command and option
### does is in cli-help.ftl.
###
### Words in --flag form, setting names (device_family, mock_mode) and
### environment variables (RUST_LOG, LANG) are typed by the user and stay as
### they are. Command lines, file paths and values arrive as variables.

## --device's help: what -h shows, and the first line of what --help
## shows, before the list of devices.

# Before the list of devices.
help-device = Dispositivo al que conectarse [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...]. Si se omite, se usa `device_family` de { $path } (escrito por dmm-gui) y, si no, `auto`, que detecta el multímetro por el cable USB
help-device-intro = Dispositivo al que conectarse.

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
    LANGUAGE, después LC_ALL, LC_MESSAGES, LANG. Los escritorios de Debian y Ubuntu
    definen LANGUAGE, que tiene prioridad sobre LANG.
# Beside LC_ALL; each line of the message is a line of the help. C and
# POSIX are the locale names that mean English, on every system.
help-env-lc-all =
    C o POSIX en la primera variable definida de LC_ALL, LC_MESSAGES y LANG
    significa inglés, diga lo que diga LANGUAGE.

## After the options of dmm-cli completions --help. The command line for
## each shell follows.

help-completions-install = Instala el autocompletado para tu shell:
