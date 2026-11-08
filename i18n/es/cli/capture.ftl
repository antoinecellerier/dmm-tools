### What 'dmm-cli capture' prints around its steps: the guided run a user
### makes to send a report about their meter. It walks the meter through a
### list of steps (set the dial to DC V, press HOLD…), reads what the meter
### sends at each, and writes a report file to attach to an issue on GitHub.
### These lines go to the terminal's error stream.
###
### Each step's own instruction, the readings (5.5687 V [AUTO]) and the
### report file stay as they are, in English: a maintainer reads the report.
### A user may translate the instructions for themselves: see
### --export-instructions below.
### Step ids (dcv, hold_on, extra_0), setting names (range, hold) and words
### in --flag form are typed or read back by those names and stay as they
### are. The keys the user presses to answer (s, q, r, n, y) arrive as
### variables: the run only understands those letters, so they can't be
### changed. "Enter" is the key's name and can be translated.

## Checking that the meter answers, before anything else.

# Dim, while the tool asks the meter for its name.
capture-checking = Comprobando la comunicación con el multímetro…
# Yellow and bold. The cable or Bluetooth link opened, but the meter sent
# nothing. $link is usb (a USB cable), adapter (a Bluetooth adapter plugged
# into the meter), radio (the meter's own Bluetooth) or other.
capture-not-responding =
    { $link ->
        [usb] El cable USB está conectado, pero el multímetro no responde.
        [adapter] El adaptador Bluetooth está conectado, pero el multímetro no responde.
        [radio] El enlace Bluetooth está conectado, pero el multímetro no responde.
       *[other] El enlace está conectado, pero el multímetro no responde.
    }
# Followed by the meter's own steps for switching its data transmission on.
capture-enable-transmission = Para activar la transmisión de datos:
capture-run-again = Después, vuelve a ejecutar este comando.
# What the meter sent was kept in a file of its own, as evidence. $bytes is
# how many bytes the meter sent, $path the file.
capture-no-response-saved = Bytes recibidos: { $bytes }, guardados en { $path }
# $url is the web address of the meter's issue on GitHub.
capture-no-response-attach = Si el multímetro está encendido y sigue sin responder, adjunta ese archivo en { $url }
# $error is why the file could not be written.
capture-no-response-not-saved = No se pudieron guardar los bytes recibidos: { $error }

## The banner once the meter answers, after the lines naming the meter
## (connect.ftl). Each support level is what follows "Status:"; the line
## under the partly verified and experimental ones is indented under it.

# $status is one of the capture-status-* levels below, coloured.
capture-status = Estado: { $status }
capture-status-verified = modelo compatible
capture-status-partly-verified = parcialmente verificado: conexión y modos principales confirmados
capture-status-partly-verified-detail = Esta ejecución es para los modos aún no confirmados.
# The protocol was worked out from the meter's traffic, not from its maker.
capture-status-experimental = experimental: protocolo obtenido por ingeniería inversa, no confirmado
capture-status-experimental-detail = Completa todos los pasos que puedas y comparte el informe.

# Yellow and bold, after the status banner.
capture-other-model = El multímetro se identifica como { $named }, no como el modelo { $opened } elegido al abrir la conexión.
capture-other-model-detail = Los pasos y los botones remotos serían los del multímetro { $opened }.
# $yes and $no are the keys that answer: y and n. Any other key quits.
capture-other-model-confirm = ¿Capturarlo de todos modos como { $opened }? { $yes }/{ $no }:
capture-other-model-rerun = Vuelve a ejecutar la captura con --device { $id }.

## A report from an earlier run is already in the file: resume it, start
## again or stop. Each prompt waits for one key, after a space.

# $path is the report file. $captured and $skipped count its steps.
capture-found-existing = Se encontró una captura existente: { $path } (pasos capturados: { $captured }, omitidos: { $skipped })
# $resume, $fresh and $abort are the keys that answer: r, n and q.
capture-resume-prompt = { $resume }=reanudar, { $fresh }=empezar de nuevo, { $abort }=cancelar:
capture-aborted = Captura cancelada.
# $yes and $no are the keys that answer: y and n.
capture-overwrite-confirm = Se sobrescribirá la captura existente. ¿Continuar? { $yes }/{ $no }:
capture-resuming = Reanudando: se omitirán los pasos ya capturados.
# The file is there but isn't a report this tool can read. $path is the
# file.
capture-unreadable = Se encontró el archivo { $path }, pero no se pudo analizar.
# $yes is the key that answers: y.
capture-overwrite-unreadable = ¿Sobrescribir? { $yes }=empezar de nuevo, cualquier otra tecla=cancelar:
# $path is the report file the run writes.
capture-output-file = Archivo de salida: { $path }

## What the steps ask for besides the meter and its test leads, listed
## before the first step so the user can gather it all at once. Each item is
## on a line of its own after its number in brackets.

capture-needs-heading = Necesitarás:
# $item is one of the capture-need-* items below; $steps lists the ids of
# the steps that need it, e.g. "dcv_short, ohm_short".
capture-needs-item = { $item }  (pasos: { $steps })
# The user types the numbers in brackets of what they don't have, or just
# presses Enter.
capture-needs-prompt = Números de lo que no tengas (Intro = lo tengo todo):
# What the user typed at a numbers prompt (this one or the review's) isn't
# one of the numbers listed. $token is what they typed, in quotes; $max the
# last number listed.
capture-not-a-number = { $token } no es un número entre 1 y { $max }
capture-need-shorted-leads = cables de prueba en cortocircuito
capture-need-dc-source = pila u otra fuente de tensión continua
capture-need-thermocouple = termopar tipo K
# For the non-contact voltage detector, which senses a live wire nearby.
capture-need-live-wire = cable con tensión de la red eléctrica, a mano
# The meter's power-measurement adapter, plugged into a live outlet with a
# load plugged into it.
capture-need-power-adapter = adaptador de medición de potencia, toma de corriente con tensión y una carga
capture-need-transistor = transistor
capture-need-scr = SCR (tiristor)
capture-need-loop-source = lazo de 4-20 mA y su fuente de alimentación de 15-48 V
capture-need-power-module = módulo de medición de potencia para corriente continua o USB, fuente de alimentación y carga
# LX-925 is the adapter's name.
capture-need-pv-adapter = adaptador de irradiancia LX-925

## The steps. Each opens with its id in brackets and its instruction, and
## the tool watches the meter until it shows what the step asks for.

# Bold, between heavy lines.
capture-modes-heading = Modos de medición
# Dim, under the heading. $skip and $quit are the keys: s and q.
capture-modes-keys = cada paso se captura automáticamente en cuanto el multímetro se estabiliza: Intro=capturar ahora, { $skip }=omitir uno, { $quit }=saltar al final y guardar
# Dim, after a step's instruction: the step the rest of the run relies on.
capture-gate-mark = (paso clave)
# A resumed run passes over a step the report already has. $step is the
# step's id.
capture-already-captured = paso { $step } ya capturado, se omite
# Dim, once the tool has waited a while. $skip and $quit are the keys: s and
# q.
capture-press-enter = Presiona Intro cuando el multímetro esté listo ({ $skip }=omitir, { $quit }=terminar)
# Dim, on a single line the run keeps rewriting while it waits for the meter.
# $reading is the latest reading (5.5687 V [AUTO]); $frame counts the meter's
# replies so far.
capture-progress-watching = observando el multímetro · { $reading } · trama { $frame }
# Dim, on that same line while a step's readings are taken: $done of $total
# ("reading 4 of 12"), and $reading the latest one.
capture-progress-reading = lectura { $done } de { $total } · { $reading }
# Dim: a step that presses a button on the meter by remote control holds it
# until the meter shows the state the instruction asks for. $skip and $quit
# are the keys: s and q.
capture-wait-start = Prepara el multímetro como se indica arriba: el comando se enviará en cuanto el multímetro esté listo (Intro=enviar ahora, { $skip }=omitir, { $quit }=terminar)
# Dim: the meter left the state the step starts from just before the key
# went out, so the step waits for it again.
capture-wait-moved = El multímetro cambió de estado antes de que se enviara el comando: esperando a que vuelva a ese estado
# When the terminal can't be read key by key. $skip and $quit are the keys:
# s and q.
capture-any-key = cualquier tecla=capturar, { $skip }=omitir, { $quit }=terminar:
# Red: the button the step presses on the meter by remote control was
# refused. $error is why.
capture-command-failed = Error en el comando: { $error }
# Yellow: the tool pressed a button on the meter by remote control and the
# meter didn't change. $command is the button's name as the tool sends it,
# e.g. hold; $reading what the meter shows, e.g. 5.5687 V [AUTO].
capture-did-nothing-shows = El comando { $command } no tuvo efecto; el multímetro sigue mostrando { $reading }
# Yellow: the tool pressed a button on the meter by remote control and the
# meter changed, but not to what the step waits for. $command is the button's
# name as the tool sends it, e.g. hold; $reading what the meter shows, e.g.
# 0.00 VA [HOLD].
capture-command-missed = El comando { $command } no llevó al estado que espera este paso; el multímetro muestra ahora { $reading }
# $command is the button's name as the tool sends it, e.g. hold.
capture-did-nothing-silent = El comando { $command } no tuvo efecto; el multímetro no envió ninguna lectura
# Yellow: the meter changed function while the step was reading it, so the
# step starts over. $mode is the function the step is for, $other the one
# the meter went to, e.g. Ω and Continuity.
capture-left-mode = el multímetro pasó del modo { $mode } al modo { $other } durante el muestreo: repitiendo…
# The reading the step took, in green. $reading is it, e.g. 5.5687 V [AUTO].
capture-we-read = Se leyó: { $reading }
# Something the meter sent couldn't be read; the step goes on. $error is
# why.
capture-frame-error = error: { $error }
# Yellow: the step read nothing from the meter.
capture-no-response = Sin respuesta del multímetro.
# Yellow, before capture-few-samples on the same line.
capture-few-samples-label = advertencia:
# The step got fewer readings than it asks for. $got and $wanted count them.
capture-few-samples = solo se obtuvieron { $got }/{ $wanted } muestras

## Dim, while a step waits: the meter settled, but not in what the step
## expects. Each says what the reading has and what the step wants.

# $got and $want are functions, in quotes, e.g. "DC V" and "AC V".
capture-mismatch-mode = el multímetro muestra: modo { $got }, se espera { $want }
# $flag is a status flag the meter shows, e.g. HOLD. $want says whether the
# step wants it on or off; the meter has it the other way.
capture-mismatch-flag =
    { $want ->
        [on] el multímetro muestra: { $flag } desactivado, se espera activado
       *[off] el multímetro muestra: { $flag } activado, se espera desactivado
    }
# $range is the reading's range, e.g. 22V. $shown is none (the reading has
# no range), auto (a range the meter picked itself) or plain. $want is auto
# (the step wants the meter to pick the range itself) or manual (the user
# picks it).
capture-mismatch-range =
    { $want ->
        [auto]
            { $shown ->
                [none] el multímetro muestra: rango (ninguno), se espera automático
                [auto] el multímetro muestra: rango { $range } (automático), se espera automático
               *[plain] el multímetro muestra: rango { $range }, se espera automático
            }
       *[manual]
            { $shown ->
                [none] el multímetro muestra: rango (ninguno), se espera manual
                [auto] el multímetro muestra: rango { $range } (automático), se espera manual
               *[plain] el multímetro muestra: rango { $range }, se espera manual
            }
    }
# $value is what the meter shows, e.g. 12.345 or OL; $shown is absent when
# the reading has no main value, else value. $want is the kind of reading the
# step wants: overload (OL, the meter's over-range sign), negative, finite
# (any number), ncv (the non-contact voltage detector's level 1 or more),
# word (a word the meter shows instead of a number) or zero.
capture-mismatch-value =
    el multímetro muestra: { $shown ->
        [absent] sin lectura principal
       *[value] valor { $value }
    }, se espera { $want ->
        [overload] OL
        [negative] una lectura negativa
        [ncv] un nivel NCV de 1 o más
        [word] una palabra en lugar de una lectura
        [zero] una lectura de cero
       *[finite] una lectura numérica
    }
# $value and $shown as in capture-mismatch-value. $min is the smallest
# number the step wants, positive or negative, e.g. 1.
capture-mismatch-at-least =
    el multímetro muestra: { $shown ->
        [absent] sin lectura principal
       *[value] valor { $value }
    }, se espera al menos { $min } en valor absoluto

## Asking whether the reading matches the meter's screen. Each prompt waits
## for a line, after a space.

# Dim. $retake is yes when the step can be done over. $no and $redo are the
# keys that answer: n and r.
capture-confirm =
    { $retake ->
        [yes] ¿El multímetro mostró esto? (Intro = sí, { $no } = no, { $redo } = repetir)
       *[no] ¿El multímetro mostró esto? (Intro = sí, { $no } = no)
    }
# Yellow, when the answer was none of those. $retake, $no and $redo as in
# capture-confirm.
capture-confirm-again =
    { $retake ->
        [yes] Responde { $no } o { $redo }, o presiona Intro.
       *[no] Responde { $no } o presiona Intro.
    }
# The reading didn't match: the user types what the meter showed.
capture-what-shown = ¿Qué mostró el multímetro?
# Dim: the user typed nothing at capture-what-shown.
capture-recorded-mismatch = registrado: la lectura no coincidió
# Dim. $shown is what the user typed, in quotes.
capture-recorded-shown = registrado: el multímetro mostró { $shown }
# Dim: the step starts over.
capture-retaking = repitiendo…

## After the first steps, which check that the tool reads the meter right
## ("core semantics": mode, range and flags).

# Green.
capture-core-confirmed = Semántica básica confirmada: los pasos restantes se revisan al final.
# Yellow. $steps lists the ids of the steps that failed, e.g. "dcv, ohm".
capture-core-not-confirmed = Semántica básica no confirmada ({ $steps }): cada paso pedirá confirmación.

## Once the core semantics are confirmed, the tool sets ranges and flags on
## the meter itself by remote control, and prints a dim line for each. Each
## starts with an arrow.

# $mode is the function, e.g. Continuity.
capture-drive-switching = cambiando al modo { $mode } (comando enviado por la herramienta)
# $mode is the function, e.g. Continuity; $error why the meter refused.
capture-drive-switch-failed = no se pudo cambiar al modo { $mode }: { $error }. Hazlo a mano
# $setting is the setting's name, rel (relative mode): the meter refuses it
# while it shows OL, its over-range sign.
capture-drive-rel-skipped-ol = ajuste { $setting } omitido: la lectura es OL
# $setting is the setting's name, rel (relative mode).
capture-drive-rel-skipped-blank = ajuste { $setting } omitido: no hay lectura
# In place of a reading, after the setting and value tried, e.g.
# "range:22V".
capture-drive-no-reading = sin lectura
# Yellow. $setting is the setting's name, e.g. range or hold.
capture-drive-reset-failed = no se pudo restablecer el ajuste { $setting }. Presiona el botón del multímetro
# Yellow: the meter refused too many of the tool's commands.
capture-drive-unreliable = control remoto poco fiable en este multímetro: no habrá más barridos automáticos

## The end-of-run review: the readings the run took without asking, each
## with its number, to check against what the meter showed.

# Bold, between heavy lines.
capture-review-heading = Revisión de las lecturas
# The user types the numbers of the wrong readings, or just presses Enter.
capture-review-prompt = Números de las lecturas que NO coincidieron con la pantalla del multímetro (Intro = todas correctas):

## The freeform pass: the user sets the meter to anything the steps didn't
## cover and describes it.

# Bold, between heavy lines; "Part 4" is the run's last part.
capture-freeform-heading = Parte 4: capturas adicionales (opcional)
capture-freeform-intro = Pon el multímetro en cualquier modo o estado que no se haya cubierto arriba.
# After the capture's id in brackets, e.g. [extra_0]; waits for a line.
# $quit is the word that ends the pass: q.
capture-freeform-prompt = Describe cómo configuraste el multímetro (o '{ $quit }' para terminar):

## The last check: does the tool's automatic detection recognise the
## meter? The user switches it off and on, and the tool detects it afresh.

# Dim. $bridge is the chip in the cable or the Bluetooth link, e.g. CP2110.
capture-detect-confirmed = La detección automática ya está confirmada a través de { $bridge }, así que se omite su comprobación.
# Bold, between heavy lines.
capture-detect-heading = Última comprobación: detección automática
capture-detect-off-cable = Apaga el multímetro y desconecta su cable USB.
capture-detect-off = Apaga el multímetro.
# Waits for one key. $skip is the key that skips: s.
capture-detect-off-prompt = Presiona Intro cuando esté apagado, o { $skip } para omitir:
# $mode is the function to turn the dial to, DC V.
capture-detect-on-cable = Vuelve a conectar el cable y enciende el multímetro con el selector en { $mode }.
# $mode is the function to turn the dial to, DC V.
capture-detect-on = Vuelve a encenderlo con el selector en { $mode }.
# Followed by the meter's own steps for switching its data transmission on.
capture-detect-transmission = Si el multímetro desactivó la transmisión de datos, vuelve a activarla:
# Waits for one key. $skip is the key that skips: s.
capture-detect-ready-prompt = Presiona Intro cuando esté listo, o { $skip } para omitir:
# Dim.
capture-detect-checking = Comprobando la detección automática…
# Detection found the meter; the tool reads it once more. $instruction is
# what to do, the step's own instruction, in English.
capture-detect-now = Ahora: { $instruction }
# Dim. $skip is the key that skips: s.
capture-detect-read-keys = La lectura se toma cuando el multímetro se estabiliza: Intro la toma ahora, { $skip } la omite.

## The line the check ends on: dim when detection found the meter, yellow
## with advice when it didn't. $found is the model detection picked, e.g.
## UNI-T UT61E+; $reading what the meter then read, e.g. 5.5687 V [AUTO].
## The advice names the meter the run was for: $device is its id on the
## command line, e.g. ut61eplus, and $meter its name, e.g. UNI-T UT61E+.

# $reason is why the link didn't open again, in English, or
# capture-detect-no-reason.
capture-detect-not-reopened = No se comprobó la detección automática: el enlace no se volvió a abrir ({ $reason }).
capture-detect-no-reason = sin motivo indicado
capture-detect-found = La detección automática encontró el multímetro { $found }.
capture-detect-found-read = La detección automática encontró el multímetro { $found } y leyó { $reading }.
capture-detect-found-read-other = La detección automática encontró el multímetro { $found }, pero después leyó { $reading }. Sigue usando --device { $device } o elige el multímetro { $meter } en dmm-gui.
capture-detect-found-read-nothing = La detección automática encontró el multímetro { $found }, pero después no leyó nada. Sigue usando --device { $device } o elige el multímetro { $meter } en dmm-gui.
capture-detect-picked-other = La detección automática identificó este multímetro como { $found }. Sigue usando --device { $device } o elige el multímetro { $meter } en dmm-gui.
capture-detect-not-found = La detección automática no encontró este multímetro. Sigue usando --device { $device } o elige el multímetro { $meter } en dmm-gui.

## The end of the run.

# After "Warning:" (common.ftl): the detection check stopped. $error is why.
capture-detect-failed = comprobación de la detección automática: { $error }
# Bold and green, between equals signs.
capture-complete = ¡Captura completada!
# $path is the report file, in bold.
capture-report-saved = Informe guardado en: { $path }
# The run followed a list of steps a maintainer wrote (a "plan"). $plan is
# its file; $covered and $total count its steps.
capture-plan-covered = Plan { $plan }: { $covered } de { $total } pasos capturados
capture-nothing-new = No se capturó ningún paso sin verificar, así que no hay nada nuevo que informar.
# $covered and $total count the meter's steps that hardware hasn't confirmed
# yet; $meter is its name, e.g. UNI-T UT61E+.
capture-covered = Se cubrieron { $covered } de los { $total } pasos sin verificar del multímetro { $meter }.
# $resume is the key that resumes, r (capture-resume-prompt).
capture-not-finished = Sin terminar: ejecuta el mismo comando otra vez y responde { $resume } para reanudar.
# $flag is the option as typed, --steps <ids>.
capture-steps-hint = { $flag } ejecuta solo los pasos indicados.
# $url is the web address of the meter's issue on GitHub.
capture-attach = Adjunta el informe en { $url }

## --export-instructions: the step instructions written to a file the user
## translates for themselves, then passes back with --instructions. The
## file is Fluent source, like this one.

# The comment at the top of the exported file, a line of it per line here.
# $meter is the meter's name, e.g. Brymen BM829s/BM827s/BM822s/BM821s.
# Keep "=", "-device" and "#" as they are: they are the file's
# syntax.
capture-instructions-header =
    Instrucciones de los pasos de captura del multímetro { $meter }, para traducir.
    Sustituye el texto después de cada "=" por tu traducción, en la misma línea.
    Deja como están el identificador de paso antes del "=", la línea "-device"
    y las líneas que empiezan por "#". Un paso que borres se muestra en inglés.
# A line of the header, when the steps quote words from the meter: its
# key labels and what its screen shows. $marks lists them, e.g. "AC, HOLD,
# MIN/MAX, SELECT". A translation, by a person or a machine, must leave them
# as they are, so say so plainly.
capture-instructions-keep = Deja estos términos tal como el multímetro los muestra en sus teclas y en su pantalla: { $marks }
# The header's last line. $command is the command that runs the capture
# with the translated file.
capture-instructions-run = Después ejecuta: { $command }
# In place of writing a file, for a meter with no steps of its own (a
# simulated one). $meter is its name, e.g. Mock UT61E+.
capture-instructions-none = El multímetro { $meter } no tiene pasos de captura propios: no hay nada que traducir.
# After writing the file. $count is the number of steps; $path the file.
capture-instructions-written =
    { $count ->
        [one] Se escribió { $count } instrucción de paso en { $path }.
       *[other] Se escribieron { $count } instrucciones de pasos en { $path }.
    }
# $command is the command that runs the capture with the translated file.
capture-instructions-next = Tradúcelas y después ejecuta: { $command }
# Yellow, before the first step: steps whose English changed after the
# file was exported, so they show a translation of an older wording.
# $path is the file; $steps lists the step ids, e.g. "dcv, hold".
capture-instructions-changed = Estos pasos cambiaron después de exportar el archivo { $path } y muestran tu traducción de su texto anterior: { $steps }. Exporta un archivo nuevo para ver el nuevo texto en inglés.
# One problem in the file, on a line of its own under
# capture-error-instructions-unreadable. $line is a line number.
capture-instructions-problem-syntax = línea { $line }: no es un identificador de paso, un "=" y su texto
# Another problem. $id is a step id, e.g. dcv.
capture-instructions-problem-not-text = { $id }: el texto contiene llaves; quítalas
# Another problem. $id is a step id, e.g. dcv.
capture-instructions-problem-twice = { $id }: el paso aparece dos veces
# Another problem: the file couldn't be read at all.
capture-instructions-problem-unparsable = no se puede leer el archivo; revisa las llaves y las comillas que contiene

## The line that closes a failed run, after "Error:" (common.ftl).

capture-error-not-responding = el multímetro no responde
capture-error-input-gone = la entrada del teclado dejó de funcionar; termina la captura y vuelve a ejecutarla
# --steps named steps this meter doesn't have. $steps lists them, e.g.
# "dcmv, tmep"; $command is the command line that lists the meter's steps.
capture-error-unknown-steps =
    pasos de captura desconocidos: { $steps }
    Ejecuta { $command } para ver los pasos que admite este dispositivo.
# The --instructions file can't be read. $path is the file; $problems the
# problems, one per line (capture-instructions-problem-*).
capture-error-instructions-unreadable =
    el archivo { $path } no se puede leer como instrucciones de captura:
    { $problems }
# The --instructions file was exported for another meter. $path is the
# file; $named the meter id in it, e.g. ut61eplus; $device this run's, e.g.
# bm82x; $command the command that exports a file for this meter.
capture-error-instructions-device = el archivo { $path } contiene las instrucciones para { $named }, no para { $device }. Expórtalas para este multímetro con { $command }.
# The --instructions file names steps this run doesn't have, most likely a
# step id that was translated. $path is the file; $ids the ids not found,
# e.g. "vcc, retencao"; $known the ids the run has, e.g. "dcv, acv, hold".
capture-error-instructions-ids = el archivo { $path } incluye pasos que esta ejecución no tiene: { $ids }. Deja cada identificador de paso tal como se exportó; los pasos son: { $known }.
