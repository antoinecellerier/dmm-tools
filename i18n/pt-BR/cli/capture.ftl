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
capture-checking = Verificando a comunicação com o multímetro…
# Yellow and bold. The cable or Bluetooth link opened, but the meter sent
# nothing. $link is usb (a USB cable), adapter (a Bluetooth adapter plugged
# into the meter), radio (the meter's own Bluetooth) or other.
capture-not-responding =
    { $link ->
        [usb] O cabo USB está conectado, mas o multímetro não responde.
        [adapter] O adaptador Bluetooth está conectado, mas o multímetro não responde.
        [radio] A conexão Bluetooth está estabelecida, mas o multímetro não responde.
       *[other] A conexão está estabelecida, mas o multímetro não responde.
    }
# Followed by the meter's own steps for switching its data transmission on.
capture-enable-transmission = Para ativar a transmissão de dados:
capture-run-again = Depois execute este comando novamente.
# What the meter sent was kept in a file of its own, as evidence. $bytes is
# how many bytes the meter sent, $path the file.
capture-no-response-saved = Bytes recebidos: { $bytes }, salvos em { $path }
# $url is the web address of the meter's issue on GitHub.
capture-no-response-attach = Se o multímetro estiver ligado e ainda não responder, anexe esse arquivo em { $url }
# $error is why the file could not be written.
capture-no-response-not-saved = Não foi possível salvar os bytes recebidos: { $error }

## The banner once the meter answers, after the lines naming the meter
## (connect.ftl). Each support level is what follows "Status:"; the line
## under the partly verified and experimental ones is indented under it.

# $status is one of the capture-status-* levels below, coloured.
capture-status = Status: { $status }
capture-status-verified = modelo suportado
capture-status-partly-verified = parcialmente verificado — conexão e modos principais confirmados
capture-status-partly-verified-detail = Esta execução serve para confirmar os modos que ainda faltam.
# The protocol was worked out from the meter's traffic, not from its maker.
capture-status-experimental = experimental — protocolo obtido por engenharia reversa, ainda não confirmado
capture-status-experimental-detail = Conclua o máximo de etapas possível e compartilhe o relatório.

## The meter names another model than the one the run opened (a CMS061
## opened as a CMS101): asked before the report file is touched, so the
## operator can run again with the right meter. $named and $opened are
## meter names; $id is the --device id that opens the named one.

# Yellow and bold, after the status banner.
capture-other-model = O multímetro se identifica como { $named }, mas esta execução abriu o modelo { $opened }.
capture-other-model-detail = As etapas e as teclas remotas seriam as do modelo { $opened }.
# $yes and $no are the keys that answer: y and n. Any other key quits.
capture-other-model-confirm = Capturar como { $opened } mesmo assim? { $yes }/{ $no }:
capture-other-model-rerun = Execute a captura novamente com --device { $id }.

## A report from an earlier run is already in the file: resume it, start
## again or stop. Each prompt waits for one key, after a space.

# $path is the report file. $captured and $skipped count its steps.
capture-found-existing = Captura existente encontrada: { $path } (capturadas: { $captured }, puladas: { $skipped })
# $resume, $fresh and $abort are the keys that answer: r, n and q.
capture-resume-prompt = { $resume }=retomar, { $fresh }=começar do zero, { $abort }=cancelar:
capture-aborted = Operação cancelada.
# $yes and $no are the keys that answer: y and n.
capture-overwrite-confirm = Isso substituirá a captura existente. Tem certeza? { $yes }/{ $no }:
capture-resuming = Retomando: as etapas já capturadas serão puladas.
# The file is there but isn't a report this tool can read. $path is the
# file.
capture-unreadable = O arquivo { $path } existe, mas não foi possível analisá-lo.
# $yes is the key that answers: y.
capture-overwrite-unreadable = Substituir? { $yes }=começar do zero, qualquer outra tecla=cancelar:
# $path is the report file the run writes.
capture-output-file = Arquivo de saída: { $path }

## What the steps ask for besides the meter and its test leads, listed
## before the first step so the user can gather it all at once. Each item is
## on a line of its own after its number in brackets.

capture-needs-heading = Você vai precisar de:
# $item is one of the capture-need-* items below; $steps lists the ids of
# the steps that need it, e.g. "dcv_short, ohm_short".
capture-needs-item = { $item }  (etapas: { $steps })
# The user types the numbers in brackets of what they don't have, or just
# presses Enter.
capture-needs-prompt = Números dos itens que você não tem (Enter = não falta nada):
# What the user typed at a numbers prompt (this one or the review's) isn't
# one of the numbers listed. $token is what they typed, in quotes; $max the
# last number listed.
capture-not-a-number = { $token } não é um número entre 1 e { $max }
capture-need-shorted-leads = pontas de prova em curto
capture-need-dc-source = bateria ou outra fonte de tensão contínua
capture-need-thermocouple = termopar tipo K
# For the non-contact voltage detector, which senses a live wire nearby.
capture-need-live-wire = fio energizado da rede elétrica por perto
# The meter's power-measurement adapter, plugged into a live outlet with a
# load plugged into it.
capture-need-power-adapter = adaptador de medição de potência, tomada energizada e carga
capture-need-transistor = transistor
capture-need-scr = SCR (tiristor)
capture-need-loop-source = loop 4-20 mA com fonte de 15-48 V
capture-need-power-module = módulo de medição de potência em corrente contínua ou USB, fonte e carga
# LX-925 is the adapter's name.
capture-need-pv-adapter = adaptador de irradiância LX-925

## The steps. Each opens with its id in brackets and its instruction, and
## the tool watches the meter until it shows what the step asks for.

# Bold, between heavy lines.
capture-modes-heading = Modos de medição
# Dim, under the heading. $skip and $quit are the keys: s and q.
capture-modes-keys = cada etapa é capturada automaticamente quando o multímetro se estabiliza: Enter=capturar agora, { $skip }=pular uma, { $quit }=pular até o fim e salvar
# Dim, after a step's instruction: the step the rest of the run relies on.
capture-gate-mark = (pré-requisito)
# A resumed run passes over a step the report already has. $step is the
# step's id.
capture-already-captured = etapa { $step } já capturada, pulando
# Dim, once the tool has waited a while. $skip and $quit are the keys: s and
# q.
capture-press-enter = Pressione Enter quando o multímetro estiver pronto ({ $skip }=pular, { $quit }=concluir)
# Dim, on a single line the run keeps rewriting while it waits for the meter.
# $reading is the latest reading (5.5687 V [AUTO]); $frame counts the meter's
# replies so far.
capture-progress-watching = monitorando o multímetro · { $reading } · quadro { $frame }
# Dim, on that same line while a step's readings are taken: $done of $total
# ("reading 4 of 12"), and $reading the latest one.
capture-progress-reading = leitura { $done } de { $total } · { $reading }
# Dim: a step that presses a button on the meter by remote control holds it
# until the meter shows the state the instruction asks for. $skip and $quit
# are the keys: s and q.
capture-wait-start = Prepare o multímetro como indicado acima: o comando será enviado assim que ele estiver pronto (Enter=enviar agora, { $skip }=pular, { $quit }=concluir)
# Dim: the meter left the state the step starts from just before the key
# went out, so the step waits for it again.
capture-wait-moved = O multímetro saiu do estado necessário antes do envio do comando: aguardando que ele volte a esse estado
# When the terminal can't be read key by key. $skip and $quit are the keys:
# s and q.
capture-any-key = qualquer tecla=capturar, { $skip }=pular, { $quit }=concluir:
# Red: the button the step presses on the meter by remote control was
# refused. $error is why.
capture-command-failed = Falha no comando: { $error }
# Yellow: the tool pressed a button on the meter by remote control and the
# meter didn't change. $command is the button's name as the tool sends it,
# e.g. hold; $reading what the meter shows, e.g. 5.5687 V [AUTO].
capture-did-nothing-shows = O comando { $command } não teve efeito; o multímetro ainda mostra { $reading }
# Yellow: the tool pressed a button on the meter by remote control and the
# meter changed, but not to what the step waits for. $command is the button's
# name as the tool sends it, e.g. hold; $reading what the meter shows, e.g.
# 0.00 VA [HOLD].
capture-command-missed = O comando { $command } não levou ao estado que esta etapa aguarda; o multímetro agora mostra { $reading }
# $command is the button's name as the tool sends it, e.g. hold.
capture-did-nothing-silent = O comando { $command } não teve efeito; o multímetro não enviou nenhuma leitura
# Yellow: the meter changed function while the step was reading it, so the
# step starts over. $mode is the function the step is for, $other the one
# the meter went to, e.g. Ω and Continuity.
capture-left-mode = o multímetro passou do modo { $mode } para o modo { $other } durante a amostragem — refazendo…
# The reading the step took, in green. $reading is it, e.g. 5.5687 V [AUTO].
capture-we-read = Leitura obtida: { $reading }
# Something the meter sent couldn't be read; the step goes on. $error is
# why.
capture-frame-error = erro: { $error }
# Yellow: the step read nothing from the meter.
capture-no-response = Sem resposta do multímetro.
# Yellow, before capture-few-samples on the same line.
capture-few-samples-label = aviso:
# The step got fewer readings than it asks for. $got and $wanted count them.
capture-few-samples = apenas { $got }/{ $wanted } amostras obtidas

## Dim, while a step waits: the meter settled, but not in what the step
## expects. Each says what the reading has and what the step wants.

# $got and $want are functions, in quotes, e.g. "DC V" and "AC V".
capture-mismatch-mode = o multímetro mostra: modo { $got }, a etapa espera { $want }
# $flag is a status flag the meter shows, e.g. HOLD. $want says whether the
# step wants it on or off; the meter has it the other way.
capture-mismatch-flag =
    { $want ->
        [on] o multímetro mostra: { $flag } desativado, a etapa espera ativado
       *[off] o multímetro mostra: { $flag } ativado, a etapa espera desativado
    }
# $range is the reading's range, e.g. 22V. $shown is none (the reading has
# no range), auto (a range the meter picked itself) or plain. $want is auto
# (the step wants the meter to pick the range itself) or manual (the user
# picks it).
capture-mismatch-range =
    { $want ->
        [auto]
            { $shown ->
                [none] o multímetro mostra: faixa (nenhuma), a etapa espera faixa automática
                [auto] o multímetro mostra: faixa { $range } (automática), a etapa espera faixa automática
               *[plain] o multímetro mostra: faixa { $range }, a etapa espera faixa automática
            }
       *[manual]
            { $shown ->
                [none] o multímetro mostra: faixa (nenhuma), a etapa espera faixa manual
                [auto] o multímetro mostra: faixa { $range } (automática), a etapa espera faixa manual
               *[plain] o multímetro mostra: faixa { $range }, a etapa espera faixa manual
            }
    }
# $value is what the meter shows, e.g. 12.345 or OL; $shown is absent when
# the reading has no main value, else value. $want is the kind of reading the
# step wants: overload (OL, the meter's over-range sign), negative, finite
# (any number), ncv (the non-contact voltage detector's level 1 or more),
# word (a word the meter shows instead of a number) or zero.
capture-mismatch-value =
    o multímetro mostra: { $shown ->
        [absent] nenhuma leitura principal
       *[value] valor { $value }
    }, a etapa espera { $want ->
        [overload] OL
        [negative] uma leitura negativa
        [ncv] um nível NCV de 1 ou mais
        [word] uma palavra em vez de uma leitura
        [zero] uma leitura igual a zero
       *[finite] uma leitura numérica
    }
# $value and $shown as in capture-mismatch-value. $min is the smallest
# number the step wants, positive or negative, e.g. 1.
capture-mismatch-at-least =
    o multímetro mostra: { $shown ->
        [absent] nenhuma leitura principal
       *[value] valor { $value }
    }, a etapa espera pelo menos { $min } em valor absoluto

## Asking whether the reading matches the meter's screen. Each prompt waits
## for a line, after a space.

# Dim. $retake is yes when the step can be done over. $no and $redo are the
# keys that answer: n and r.
capture-confirm =
    { $retake ->
        [yes] O multímetro mostrou isto? (Enter = sim, { $no } = não, { $redo } = refazer)
       *[no] O multímetro mostrou isto? (Enter = sim, { $no } = não)
    }
# Yellow, when the answer was none of those. $retake, $no and $redo as in
# capture-confirm.
capture-confirm-again =
    { $retake ->
        [yes] Responda { $no }, { $redo } ou pressione Enter.
       *[no] Responda { $no } ou pressione Enter.
    }
# The reading didn't match: the user types what the meter showed.
capture-what-shown = O que o multímetro mostrou?
# Dim: the user typed nothing at capture-what-shown.
capture-recorded-mismatch = registrado: a leitura não correspondia
# Dim. $shown is what the user typed, in quotes.
capture-recorded-shown = registrado: o multímetro mostrou { $shown }
# Dim: the step starts over.
capture-retaking = refazendo…

## After the first steps, which check that the tool reads the meter right
## ("core semantics": mode, range and flags).

# Green.
capture-core-confirmed = Semântica básica confirmada: as etapas restantes serão revisadas no final.
# Yellow. $steps lists the ids of the steps that failed, e.g. "dcv, ohm".
capture-core-not-confirmed = Semântica básica não confirmada ({ $steps }): cada etapa pedirá confirmação.

## Once the core semantics are confirmed, the tool sets ranges and flags on
## the meter itself by remote control, and prints a dim line for each. Each
## starts with an arrow.

# $mode is the function, e.g. Continuity.
capture-drive-switching = mudando para o modo { $mode } (comando enviado pelo aplicativo)
# $mode is the function, e.g. Continuity; $error why the meter refused.
capture-drive-switch-failed = não foi possível mudar para o modo { $mode }: { $error } — faça isso manualmente
# $setting is the setting's name, rel (relative mode): the meter refuses it
# while it shows OL, its over-range sign.
capture-drive-rel-skipped-ol = configuração { $setting } pulada: a leitura é OL
# $setting is the setting's name, rel (relative mode).
capture-drive-rel-skipped-blank = configuração { $setting } pulada: sem leitura
# In place of a reading, after the setting and value tried, e.g.
# "range:22V".
capture-drive-no-reading = sem leitura
# Yellow. $setting is the setting's name, e.g. range or hold.
capture-drive-reset-failed = não foi possível redefinir a configuração { $setting } — pressione a tecla do multímetro
# Yellow: the meter refused too many of the tool's commands.
capture-drive-unreliable = controle remoto não confiável neste multímetro — sem mais ajustes automáticos

## The end-of-run review: the readings the run took without asking, each
## with its number, to check against what the meter showed.

# Bold, between heavy lines.
capture-review-heading = Revise as leituras
# The user types the numbers of the wrong readings, or just presses Enter.
capture-review-prompt = Números das leituras que NÃO corresponderam ao display do multímetro (Enter = todas corretas):

## The freeform pass: the user sets the meter to anything the steps didn't
## cover and describes it.

# Bold, between heavy lines; "Part 4" is the run's last part.
capture-freeform-heading = Parte 4: capturas adicionais (opcional)
capture-freeform-intro = Coloque o multímetro em qualquer modo ou estado que as etapas acima não tenham coberto.
# After the capture's id in brackets, e.g. [extra_0]; waits for a line.
# $quit is the word that ends the pass: q.
capture-freeform-prompt = Descreva como você configurou o multímetro (ou '{ $quit }' para concluir):

## The last check: does the tool's automatic detection recognise the
## meter? The user switches it off and on, and the tool detects it afresh.

# Dim. $bridge is the chip in the cable or the Bluetooth link, e.g. CP2110.
capture-detect-confirmed = A detecção automática já está confirmada via { $bridge }, então esta verificação será pulada.
# Bold, between heavy lines.
capture-detect-heading = Última verificação: detecção automática
capture-detect-off-cable = Desligue o multímetro e desconecte o cabo USB.
capture-detect-off = Desligue o multímetro.
# Waits for one key. $skip is the key that skips: s.
capture-detect-off-prompt = Pressione Enter depois de desligá-lo ou { $skip } para pular:
# $mode is the function to turn the dial to, DC V.
capture-detect-on-cable = Reconecte o cabo e ligue o multímetro com a chave seletora em { $mode }.
# $mode is the function to turn the dial to, DC V.
capture-detect-on = Ligue-o novamente com a chave seletora em { $mode }.
# Followed by the meter's own steps for switching its data transmission on.
capture-detect-transmission = Se o multímetro desativou a transmissão de dados, ative-a novamente:
# Waits for one key. $skip is the key that skips: s.
capture-detect-ready-prompt = Pressione Enter quando ele estiver pronto ou { $skip } para pular:
# Dim.
capture-detect-checking = Verificando a detecção automática…
# Detection found the meter; the tool reads it once more. $instruction is
# what to do, the step's own instruction, in English.
capture-detect-now = Agora: { $instruction }
# Dim. $skip is the key that skips: s.
capture-detect-read-keys = A leitura é feita assim que o multímetro se estabiliza: Enter faz a leitura agora, { $skip } pula.

## The line the check ends on: dim when detection found the meter, yellow
## with advice when it didn't. $found is the model detection picked, e.g.
## UNI-T UT61E+; $reading what the meter then read, e.g. 5.5687 V [AUTO].
## The advice names the meter the run was for: $device is its id on the
## command line, e.g. ut61eplus, and $meter its name, e.g. UNI-T UT61E+.

# $reason is why the link didn't open again, in English, or
# capture-detect-no-reason.
capture-detect-not-reopened = A detecção automática não foi verificada: a conexão não foi reaberta ({ $reason }).
capture-detect-no-reason = nenhum motivo informado
capture-detect-found = A detecção automática encontrou o multímetro { $found }.
capture-detect-found-read = A detecção automática encontrou o multímetro { $found } e leu { $reading }.
capture-detect-found-read-other = A detecção automática encontrou o multímetro { $found }, mas depois leu { $reading } — continue usando --device { $device } ou escolha o multímetro { $meter } no dmm-gui.
capture-detect-found-read-nothing = A detecção automática encontrou o multímetro { $found }, mas depois não leu nada — continue usando --device { $device } ou escolha o multímetro { $meter } no dmm-gui.
capture-detect-picked-other = A detecção automática identificou este multímetro como { $found } — continue usando --device { $device } ou escolha o multímetro { $meter } no dmm-gui.
capture-detect-not-found = A detecção automática não encontrou este multímetro — continue usando --device { $device } ou escolha o multímetro { $meter } no dmm-gui.

## The end of the run.

# After "Warning:" (common.ftl): the detection check stopped. $error is why.
capture-detect-failed = verificação da detecção automática: { $error }
# Bold and green, between equals signs.
capture-complete = Captura concluída!
# $path is the report file, in bold.
capture-report-saved = Relatório salvo em: { $path }
# The run followed a list of steps a maintainer wrote (a "plan"). $plan is
# its file; $covered and $total count its steps.
capture-plan-covered =
    { $total ->
        [0] Plano { $plan }: { $covered } de { $total } etapas capturadas
        [one] Plano { $plan }: { $covered } de { $total } etapa capturada
       *[other] Plano { $plan }: { $covered } de { $total } etapas capturadas
    }
capture-nothing-new = Nenhuma etapa não verificada foi capturada, então não há nada novo a relatar.
# $covered and $total count the meter's steps that hardware hasn't confirmed
# yet; $meter is its name, e.g. UNI-T UT61E+.
capture-covered = Foram capturadas { $covered } de { $total } etapas não verificadas do multímetro { $meter }.
# $resume is the key that resumes, r (capture-resume-prompt).
capture-not-finished = Captura não concluída: execute o mesmo comando novamente e responda { $resume } para retomar.
# $flag is the option as typed, --steps <ids>.
capture-steps-hint = { $flag } executa somente as etapas indicadas.
# $url is the web address of the meter's issue on GitHub.
capture-attach = Anexe o relatório em { $url }

## --export-instructions: the step instructions written to a file the user
## translates for themselves, then passes back with --instructions. The
## file is Fluent source, like this one.

# The comment at the top of the exported file, a line of it per line here.
# $meter is the meter's name, e.g. Brymen BM829s/BM827s/BM822s/BM821s.
# Keep "=", "-device" and "#" as they are: they are the file's
# syntax.
capture-instructions-header =
    Instruções das etapas de captura do multímetro { $meter }, para traduzir.
    Substitua o texto após cada “=” pela sua tradução, na mesma linha.
    Mantenha como estão o ID da etapa antes do “=”, a linha “-device” e as
    linhas que começam com “#”. Uma etapa que você apagar aparece em inglês.
# A line of the header, when the steps quote words from the meter: its
# key labels and what its screen shows. $marks lists them, e.g. "AC, HOLD,
# MIN/MAX, SELECT". A translation, by a person or a machine, must leave them
# as they are, so say so plainly.
capture-instructions-keep = Mantenha estes termos exatamente como o multímetro os mostra nas teclas e na tela: { $marks }
# The header's last line. $command is the command that runs the capture
# with the translated file.
capture-instructions-run = Depois execute: { $command }
# In place of writing a file, for a meter with no steps of its own (a
# simulated one). $meter is its name, e.g. Mock UT61E+.
capture-instructions-none = O multímetro { $meter } não tem etapas de captura próprias: não há nada para traduzir.
# After writing the file. $count is the number of steps; $path the file.
capture-instructions-written =
    { $count ->
        [0] { $count } instruções de etapas gravadas em { $path }.
        [one] { $count } instrução de etapa gravada em { $path }.
       *[other] { $count } instruções de etapas gravadas em { $path }.
    }
# $command is the command that runs the capture with the translated file.
capture-instructions-next = Traduza-as e depois execute: { $command }
# Yellow, before the first step: steps whose English changed after the
# file was exported, so they show a translation of an older wording.
# $path is the file; $steps lists the step ids, e.g. "dcv, hold".
capture-instructions-changed = Estas etapas mudaram depois que o arquivo { $path } foi exportado e mostram a sua tradução do texto antigo: { $steps }. Exporte um novo arquivo para ver o novo texto em inglês.
# One problem in the file, on a line of its own under
# capture-error-instructions-unreadable. $line is a line number.
capture-instructions-problem-syntax = linha { $line }: não está no formato ID da etapa, “=” e texto
# Another problem. $id is a step id, e.g. dcv.
capture-instructions-problem-not-text = { $id }: o texto contém chaves; remova-as
# Another problem. $id is a step id, e.g. dcv.
capture-instructions-problem-twice = { $id }: a etapa aparece duas vezes
# Another problem: the file couldn't be read at all.
capture-instructions-problem-unparsable = não foi possível ler o arquivo; verifique as chaves e as aspas nele

## The line that closes a failed run, after "Error:" (common.ftl).

capture-error-not-responding = o multímetro não responde
capture-error-input-gone = a entrada do teclado parou de funcionar; conclua a captura e execute-a novamente
# --steps named steps this meter doesn't have. $steps lists them, e.g.
# "dcmv, tmep"; $command is the command line that lists the meter's steps.
capture-error-unknown-steps =
    etapas de captura desconhecidas: { $steps }
    Execute { $command } para ver as etapas que este dispositivo suporta.
# The --instructions file can't be read. $path is the file; $problems the
# problems, one per line (capture-instructions-problem-*).
capture-error-instructions-unreadable =
    não é possível ler o arquivo { $path } como instruções de captura:
    { $problems }
# The --instructions file was exported for another meter. $path is the
# file; $named the meter id in it, e.g. ut61eplus; $device this run's, e.g.
# bm82x; $command the command that exports a file for this meter.
capture-error-instructions-device = o arquivo { $path } contém as instruções do multímetro { $named }, e não do multímetro { $device }. Exporte-as para este multímetro com { $command }.
# The --instructions file names steps this run doesn't have, most likely a
# step id that was translated. $path is the file; $ids the ids not found,
# e.g. "vcc, retencao"; $known the ids the run has, e.g. "dcv, acv, hold".
capture-error-instructions-ids = o arquivo { $path } indica etapas que esta execução não tem: { $ids }. Mantenha cada ID de etapa como foi exportado; as etapas são: { $known }.
