### dmm-cli's --help: what each command and option does.
###
### Generated from the doc comments in crates/dmm-cli/src/cli.rs by
### `UPDATE_L10N=1 cargo test -p dmm-cli cli_help_ftl`: edit those, not this
### file. The run names each ID whose English changed. When the meaning changed
### too, delete that message from every language's cli-help.ftl, so it shows in
### English until it is translated again.
###
### IDs follow the command tree: read.about is what the read command does,
### read--interval-ms.help what its --interval-ms option does. clap wraps each
### paragraph to the terminal's width: keep a paragraph on one line. Option
### names (--device), values (auto, mock) and command lines ('dmm-cli list')
### stay as they are. clap's own words around these texts (Usage:, Options:,
### [default: …], [possible values: …]) are in cli/help.ftl.

# dmm-cli
dmm-cli =
    .about = Herramienta de línea de comandos para multímetros digitales USB y Bluetooth
# dmm-cli --device
dmm-cli--device =
    .help = Dispositivo al que conectarse [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...]. Si se omite, se usa `device_family` de ~/.config/dmm-tools/settings.json (escrito por dmm-gui) y, si no, `auto`, que detecta el multímetro por el cable USB
# dmm-cli --adapter
dmm-cli--adapter =
    .help = Seleccionar un adaptador concreto cuando hay más de uno accesible. Indica el número de serie o la ruta HID de un cable USB, o la dirección de un adaptador o multímetro Bluetooth, tal como los muestra 'dmm-cli list'
# dmm-cli --no-bluetooth
dmm-cli--no-bluetooth =
    .help = Desactivar la búsqueda Bluetooth en esta ejecución: no se buscan adaptadores ni multímetros y 'list' solo muestra cables. Anula el ajuste 'bluetooth' guardado. Una dirección indicada en --adapter se abre igualmente
# dmm-cli --lang
dmm-cli--lang =
    .help = Idioma de los mensajes y de la ayuda, o `system` para el del sistema. Si se omite, se usa `language` del archivo de configuración (escrito por la fila Idioma de dmm-gui) y, si no, el idioma del sistema
# dmm-cli list
list =
    .about = Mostrar los cables USB conectados y los adaptadores y multímetros Bluetooth al alcance
# dmm-cli info
info =
    .about = Conectar y mostrar la información del dispositivo
# dmm-cli read
read =
    .about = Leer mediciones de forma continua
# dmm-cli read --interval-ms
read--interval-ms =
    .help = Conservar como máximo una lectura por intervalo, en milisegundos (0 = cada lectura que produce el multímetro)
# dmm-cli read --format
# .value-… says what that value does, in --help's list of values.
read--format =
    .help = Formato de salida [predeterminado: text, o el que indique la extensión de -o]
    .value-replay = Las tramas del propio multímetro, para que --replay las reproduzca
# dmm-cli read --output
read--output =
    .help = Archivo de salida (stdout si no se especifica). Sin nombre, el archivo recibe el nombre del multímetro, su modo y el inicio de la ejecución
# dmm-cli read --count
read--count =
    .help = Número de lecturas (0 = ilimitado: hasta Ctrl+C o el final de un archivo de --replay)
# dmm-cli read --integrate
read--integrate =
    .help = Mostrar la integral acumulada en el tiempo (carga para los modos de corriente, V·s para la tensión)
# dmm-cli read --scale
read--scale =
    .help = Multiplicar la lectura, tomada en unidades base (V, A, Ω, …), por FACTOR. Una pinza de 10 mV/A es --scale 100; una sonda 100:1 es --scale 100
# dmm-cli read --offset
read--offset =
    .help = Sumar VALUE después de escalar (32 con --scale 1.8 convierte °C en °F)
# dmm-cli read --unit
read--unit =
    .help = Etiquetar la lectura escalada con esta unidad en lugar de la unidad base del multímetro
# dmm-cli read --alarm-high
read--alarm-high =
    .help = Avisar cuando la lectura supere VALUE, en unidades base (V, A, Ω, …), o en la unidad de --unit de una lectura escalada
# dmm-cli read --alarm-low
read--alarm-low =
    .help = Avisar cuando la lectura baje de VALUE, igual que --alarm-high
# dmm-cli read --alarm-hysteresis
read--alarm-hysteresis =
    .help = Cuánto debe volver hacia dentro una lectura antes de que el mismo límite vuelva a activar la alarma: un valor en la unidad de los límites, o un porcentaje del límite (`1%`) [predeterminado: unas pocas cuentas del último dígito del multímetro]
# dmm-cli read --alarm-bell
read--alarm-bell =
    .help = Hacer sonar la campana de la consola en cada alarma
# dmm-cli read --mock-mode
read--mock-mode =
    .help = Fijar el dispositivo simulado en un modo concreto (solo con --device mock). Sin esta opción, se aplica el modo simulado del archivo de configuración y, si no, el simulador recorre todos los modos
# dmm-cli read --replay
read--replay =
    .help = Reproducir un archivo escrito por --format replay en lugar de abrir un multímetro; termina con el archivo
# dmm-cli read --import
read--import =
    .help = Leer un archivo CSV, JSON o de reproducción exportado en lugar de abrir un multímetro, sin esperar: para convertirlo o mostrar su resumen
# dmm-cli command
command =
    .about = Enviar al multímetro un comando que presiona un botón. Ejecútalo sin argumentos para ver los comandos disponibles del dispositivo seleccionado
# dmm-cli command <ACTION>
command--action =
    .help = Nombre del comando (ejecútalo sin argumentos para ver los comandos disponibles)
# dmm-cli get
get =
    .about = Mostrar a qué se pueden cambiar los ajustes del multímetro desde donde está ahora. Ejecútalo sin argumento para ver todos los ajustes que ofrecen opciones
# dmm-cli get <SETTING>
get--setting =
    .help = Ajuste que mostrar (omítelo para verlos todos)
# dmm-cli get --format
get--format =
    .help = Formato de salida
# dmm-cli get --mock-mode
get--mock-mode =
    .help = Fijar el dispositivo simulado en un modo concreto (solo con --device mock). Sin esta opción, se aplica el modo simulado del archivo de configuración y, si no, el simulador recorre todos los modos
# dmm-cli set
set =
    .about = Cambiar un ajuste del multímetro sin tocar el multímetro. Ejecútalo sin valor para ver a qué valores llega ese ajuste desde aquí
# dmm-cli set <SETTING>
set--setting =
    .help = Ajuste que cambiar
# dmm-cli set <CHOICE>
set--choice =
    .help = Etiqueta del valor, o un fragmento único de ella, tal como aparece en la lista (ejecútalo sin él para verlas)
# dmm-cli set --mock-mode
set--mock-mode =
    .help = Fijar el dispositivo simulado en un modo concreto (solo con --device mock). Sin esta opción, se aplica el modo simulado del archivo de configuración y, si no, el simulador recorre todos los modos
# dmm-cli debug
debug =
    .about = Modo de volcado hexadecimal sin procesar para depurar protocolos
# dmm-cli debug --count
debug--count =
    .help = Número de peticiones que enviar (0 = ilimitado)
# dmm-cli debug --interval-ms
debug--interval-ms =
    .help = Conservar como máximo una lectura por intervalo, en milisegundos (0 = cada lectura)
# dmm-cli completions
completions =
    .about = Generar el autocompletado para el shell
# dmm-cli completions <SHELL>
completions--shell =
    .help = Shell para el que generar el autocompletado
# dmm-cli capture
capture =
    .about = Captura guiada del protocolo para informes de errores y verificación
# dmm-cli capture --output
capture--output =
    .help = Archivo de salida (predeterminado: capture-<device>.yaml). Anula el nombre automático
# dmm-cli capture --steps
capture--steps =
    .help = Ejecutar solo pasos concretos (identificadores separados por comas, p. ej. "dcmv,temp,duty")
# dmm-cli capture --unverified
capture--unverified =
    .help = Ejecutar solo los pasos aún no confirmados con hardware real, más la pasada libre
# dmm-cli capture --plan
capture--plan =
    .help = Ejecutar los pasos del archivo de plan de un mantenedor en lugar de la lista propia del dispositivo
# dmm-cli capture --sniff
capture--sniff =
    .help = No confiar en nada de lo que dice el analizador: detectar los pasos por cambios en los bytes sin procesar y confirmar cada uno
# dmm-cli capture --no-drive
capture--no-drive =
    .help = No dejar que la herramienta ajuste por sí misma los rangos y los indicadores después de cada paso de modo
# dmm-cli capture --settle
capture--settle =
    .help = Esperar MS antes de muestrear cada paso, para lecturas que tardan en estabilizarse
# dmm-cli capture --list-steps
capture--list-steps =
    .help = Mostrar todos los identificadores de paso disponibles y salir
# dmm-cli capture --format
capture--format =
    .help = Cómo muestra --list-steps la lista (md es la lista de comprobación de la incidencia)
