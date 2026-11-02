### The Scale and Alarm chips at the end of the meter's buttons under the
### reading (assets/gui-reading-controls.webp), the row of fields each chip
### opens under them, the toasts they raise (the short messages under the
### top bar's right end) and the alarm's limit lines on the graph. No doc
### image shows a row open.
###
### Scale is a software transform: it turns the meter's reading into what
### it stands for, such as a current clamp's 10 mV/A into amps. Nothing is
### sent to the meter. Alarm watches the main reading against a low and a
### high limit, and marks each reading that crosses one.
###
### Numbers and units arrive formatted and stay as they are.

## The chips, at the end of the meter's buttons (HOLD, REL…), which a
## narrow window wraps: keep the names short. Each chip is a body that
## switches its feature on or off, joined to a caret (▸) that opens its row
## of fields; the body is filled while the feature is on.

# The Scale chip's body.
scale-chip = Escala
    .hover = Ativar ou desativar o ajuste de escala
# The caret beside Scale: opens the row of fields below.
scale-settings =
    .hover = Definir a escala
    .a11y = Configurações da escala
# The Alarm chip's body.
alarm-chip = Alarme
    .hover = Ativar ou desativar o alarme
# The caret beside Alarm: opens the row of fields below.
alarm-settings =
    .hover = Definir os limites
    .a11y = Configurações do alarme
# The hover of a greyed-out Scale or Alarm body while the window shows the
# meter alone with nothing typed in its fields yet. With the graph and the
# recording both hidden in Settings, there is no room for the row of
# fields.
alarm-scale-setup-in-settings = Mostre primeiro o gráfico ou a gravação em Configurações
# The same in the big meter, which has no room for the row. $keys is the
# shortcut that leaves the big meter, e.g. Ctrl+B or ⌘B.
alarm-scale-setup-in-normal-layout = Configure primeiro no layout normal ({ $keys })

## The Scale row, under the chips while Scale's caret is open:
## "× [Scale factor] + [Offset] → [Unit label] [Apply] [Off]". The ×, +
## and → before each field are symbols, not text. The fields are narrow:
## their placeholders are cut short with … when longer than the field.

# Placeholder in the empty field after ×: the number each reading is
# multiplied by.
scale-factor = Fator de escala
# Placeholder in the empty field after +: the number added after
# multiplying.
scale-offset = Desloc.
# Placeholder in the empty field after →: the unit the scaled reading is
# shown in, e.g. A or °F.
scale-unit = Unid.
# Applies the three fields to the reading.
scale-apply = Aplicar
    .hover = Aplicar estes valores a todas as leituras a partir de agora
# Turns scaling off; the fields keep their values for the next Apply.
scale-off = Desativar
    .hover = Parar o ajuste de escala e voltar a mostrar a leitura do próprio multímetro

## The Scale row's toasts.

# A field could not be read; nothing changes. $reason is why, lower-case
# with no full stop, e.g. “abc” is not a number.
scale-invalid-factor = Escala inválida: { $reason }
# The same for the Offset field, with $reason as there.
scale-invalid-offset = Deslocamento inválido: { $reason }
# Scaling turned on, or changed. $transform is the scale in force, written
# in symbols and numbers, e.g. ×100 → A or ×2 + 0.5. $also says what else
# happened: recording, a recording is under way and goes on with the scaled
# readings; alarm, the alarm was on and was turned off, since its limits
# were in the old unit; both; or none.
scale-on =
    { $also ->
        [recording] Escala aplicada às leituras: { $transform } — a gravação continua com os valores em escala
        [alarm] Escala aplicada às leituras: { $transform } — alarme desativado; defina os limites novamente na nova unidade
        [both] Escala aplicada às leituras: { $transform } — a gravação continua com os valores em escala — alarme desativado; defina os limites novamente na nova unidade
       *[none] Escala aplicada às leituras: { $transform }
    }
# Scaling turned off. $also as in scale-on.
scale-off-toast =
    { $also ->
        [recording] Escala desativada — a gravação continua com os valores em escala
        [alarm] Escala desativada — alarme desativado; defina os limites novamente na nova unidade
        [both] Escala desativada — a gravação continua com os valores em escala — alarme desativado; defina os limites novamente na nova unidade
       *[none] Escala desativada
    }

## The Alarm row, under the chips while Alarm's caret is open:
## "Low [none] High [none] V ± [auto] [Apply] [Off]". The fields are narrow.

# Caption before the low limit's field.
alarm-low = Inferior
# Caption before the high limit's field.
alarm-high = Superior
# Placeholder in an empty Low or High field: that side has no limit.
alarm-no-limit = nenhum
# The field after ±, the hysteresis band: how far back inside a limit the
# reading must come before that limit can alarm again. Its placeholder says
# that blank picks the band automatically.
alarm-band = auto
    .hover = Quanto a leitura deve voltar antes que um limite dispare o alarme de novo: um valor ou uma porcentagem, como 1%. Em branco: automático
# After the limits, the unit they are in, e.g. V; this is its hover.
alarm-unit =
    .hover = A unidade dos limites
# The same before an alarm is on: the unit taken from the reading on
# screen.
alarm-unit-from-reading =
    .hover = A unidade dos limites, tirada da leitura exibida
# Shown in place of the unit, in the warning colour, while the reading is
# of another quantity than the limits: the dial was turned. $unit is the
# unit of the readings now arriving, e.g. Ω; on hover, $limits is the unit
# the limits are in, e.g. V. Short: it sits in the row.
alarm-idle = em espera: { $unit }
    .hover = Em espera: as leituras estão em { $unit }, os limites em { $limits }
# Turns the alarm on with the fields' limits.
alarm-apply = Aplicar
    .hover = Ativar o alarme com estes limites
# Turns the alarm off; the fields keep their values for the next Apply.
alarm-off = Desativar
    .hover = Desativar o alarme

## The Alarm row's toasts. $reason is why a value was refused, lower-case
## with no full stop, e.g. “abc” is not a number.

# The Low field could not be read.
alarm-invalid-low = Limite inferior inválido: { $reason }
# The High field could not be read.
alarm-invalid-high = Limite superior inválido: { $reason }
# A limit is not a finite number.
alarm-invalid-limit = Limite inválido: { $reason }
# The two limits don't make a pair: the low one is not below the high one.
alarm-invalid-limits = Limites inválidos: { $reason }
# The ± field could not be read, or does not fit the limits.
alarm-invalid-band = Histerese inválida: { $reason }
# The alarm turned on. $limits is the limits it watches, from
# alarm-limits/alarm-hysteresis in lib-alarm.ftl, e.g. "above 5 or below 3,
# hysteresis 1%".
alarm-on-toast = Alarme ativado: { $limits }
# The alarm turned off.
alarm-off-toast = Alarme desativado

## The graph: a dotted line at each limit, labelled at its right end.

# $side is high (the high limit) or low; $value is the limit with its unit,
# e.g. 3.0000 V.
alarm-limit-line =
    { $side ->
        [low] Inferior: { $value }
       *[high] Superior: { $value }
    }
