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
error-hid = erro HID: { $detail }
# The meter sent something that makes no sense. $detail says what, in
# English, for a bug report.
error-invalid-response = resposta inválida: { $detail }
# The meter on the link is not one the meter chosen can read, worded as
# error-invalid-response. $reason is one of the "Another meter" sentences
# below.
error-wrong-meter = resposta inválida: { $reason }
# A frame from the meter failed its check. $expected and $actual are
# hexadecimal numbers, e.g. 0x01a3.
error-checksum-mismatch = soma de verificação incorreta: esperado { $expected }, recebido { $actual }
error-timeout = tempo limite esgotado ao aguardar a resposta
# $reason is one of the "Refused before it was sent" sentences below.
error-unsupported-command = comando não suportado: { $reason }
# $reason is one of the "The meter did not do it" sentences below.
error-command-rejected = comando recusado: { $reason }
# A meter on a cable was opened as a simulated one. $device is its ID, e.g.
# ut61eplus.
error-not-simulated = dispositivo desconhecido: { $device } não é um multímetro simulado
# A recording could not be played back. $detail names the file or the line
# and says what is wrong with it, in English.
error-replay = arquivo de reprodução: { $detail }
# $adapter is the value given with --adapter, sometimes with an explanation
# in English.
error-adapter-not-found = adaptador não encontrado: { $adapter }
# What --adapter named, as error-adapter-not-found's $adapter and the GUI's
# connection-adapter-not-matched quote it, when it is a USB device that is
# no supported cable. $path is the device's path, as given with --adapter.
error-not-an-adapter = { $path } (o dispositivo existe, mas não é um adaptador USB compatível)
# --adapter named a cable or adapter the chosen meter cannot be reached
# through. $link is bluetooth (a Bluetooth adapter) or cable (a USB cable
# whose name is $cable, e.g. BU-86X). $model is the meter, e.g. UNI-T
# UT61E+.
error-wrong-cable =
    { $link ->
        [bluetooth] --adapter indica um adaptador Bluetooth, que não se comunica com o multímetro { $model }: indique com --adapter o cabo do próprio multímetro ou omita --adapter
       *[cable] --adapter indica um cabo { $cable }, que não se comunica com o multímetro { $model }: indique com --adapter o cabo do próprio multímetro ou omita --adapter
    }
error-link-lost = Conexão Bluetooth perdida
# $reason is one of the "Bluetooth" sentences below.
error-bluetooth = Bluetooth: { $reason }
# A cable or Bluetooth link is up but no meter answered on it. $link is usb
# (a USB cable), adapter (a Bluetooth adapter plugged into the meter) or
# radio (the meter's own Bluetooth).
error-device-not-identified =
    { $link ->
        [adapter] nenhum multímetro respondeu pelo adaptador Bluetooth
        [radio] nenhum multímetro respondeu pela conexão Bluetooth
       *[usb] nenhum multímetro respondeu pelo cabo USB
    }
# No cable was found. $bluetooth is searched when Bluetooth was searched
# too, else not-searched.
error-no-transport-found =
    { $bluetooth ->
        [searched] nenhum adaptador USB compatível encontrado, e nenhum dispositivo Bluetooth ao alcance
       *[not-searched] nenhum adaptador USB compatível encontrado
    }
# As error-no-transport-found, naming the cables looked for; $bluetooth as
# there. $cables lists them with their USB IDs, e.g. "CP2110 0x10c4:0xea80,
# BU-86X 0x0820:0x0001".
error-no-transport-found-looked-for =
    { $bluetooth ->
        [searched] nenhum adaptador USB compatível encontrado (cabos procurados: { $cables }), e nenhum dispositivo Bluetooth ao alcance
       *[not-searched] nenhum adaptador USB compatível encontrado (cabos procurados: { $cables })
    }
# A meter with Bluetooth built in, and none was in range. $model is the
# meter, e.g. UNI-T UT60BT.
error-bluetooth-only-not-in-range = nenhum multímetro { $model } encontrado ao alcance do Bluetooth
# A meter with Bluetooth built in, and Bluetooth was not searched. $model
# is the meter, e.g. UNI-T UT60BT. Each app adds how to switch it on.
error-bluetooth-only-switched-off = o multímetro { $model } só se conecta via Bluetooth, e o Bluetooth está desativado
# $model is the meter, e.g. UNI-T UT60BT.
error-bluetooth-only-not-built = o multímetro { $model } só se conecta via Bluetooth, e esta versão não tem suporte a Bluetooth
# $model is the meter, e.g. UNI-T UT60BT.
error-bluetooth-only-usb-adapter = o multímetro { $model } só se conecta via Bluetooth, e --adapter indica um dispositivo USB: informe o endereço Bluetooth do multímetro ou omita --adapter
# Opening the meter was stopped part-way, by Disconnect or by quitting.
error-cancelled = abertura da conexão cancelada

## Refused before it was sent: the meter, the link or the dial position
## cannot do it. After "unsupported command:". $setting is a setting's
## name as the CLI takes it (mode, range, rel, minmax…); $word, $target and
## $from_word are a mode's hexadecimal code, e.g. 0x3111.

# $setting is a setting's name.
error-unsupported-setting = a configuração { $setting } não pode ser alterada neste multímetro
# $mode is a mode.
error-unsupported-several-positions = o modo { $mode } aparece em mais de uma posição da chave seletora: faça primeiro uma leitura em outro modo
# $mode is a mode.
error-unsupported-no-position = o modo { $mode } não corresponde a nenhuma posição conhecida da chave seletora
# $target and $from are modes.
error-unsupported-mode-unreachable = não é possível passar do modo { $from } para o modo { $target }: gire primeiro a chave seletora
# $target and $from_word are mode codes, $from is the mode $from_word is.
error-unsupported-word-unreachable = não é possível passar do modo { $from } ({ $from_word }) para o modo { $target }: gire primeiro a chave seletora
# $mode is a mode.
error-unsupported-no-ranges = o modo { $mode } não tem faixas selecionáveis neste multímetro
# $id is a range's number, $count how many ranges $mode, a mode, has.
error-unsupported-no-such-range = a faixa { $id } não está entre as { $count } que o modo { $mode } oferece
# $id is a range's number, $count how many ranges $mode, a mode, has.
error-unsupported-range-not-listed = a faixa { $id } não está entre as { $count } faixas que o modo { $mode } oferece
# $count is how many ranges $mode, a mode, has.
error-unsupported-auto-range-not-listed = a faixa automática não está entre as { $count } faixas que o modo { $mode } oferece
# $mode is a mode, $word its code.
error-unsupported-fixed-range = faixa no modo { $mode } ({ $word }): modo de faixa fixa
# $setting is a setting's name, $mode a mode.
error-unsupported-setting-not-in-mode = a configuração { $setting } não pode ser alterada no modo { $mode } neste multímetro
# $setting is a setting's name, $mode a mode, $word its code.
error-unsupported-setting-not-in-word = a configuração { $setting } não pode ser alterada no modo { $mode } ({ $word }) neste multímetro
# $setting is a setting's name, $id a state's number, $mode a mode and
# $offered the states it offers, separated by commas (on, off, MAX…).
error-unsupported-no-such-state = a configuração { $setting } não tem o estado { $id }; o modo { $mode } oferece { $offered }
# $command is a command word, $mode a Peak mode, $word its code.
error-unsupported-refused-in-peak = { $command } no modo { $mode } ({ $word }): o multímetro recusa este comando no modo de pico
# "rel" is the command word. $mode is a mode, $word its code.
error-unsupported-no-rel = rel no modo { $mode } ({ $word })
# $model is the meter's part number, e.g. VC-880; $button a button's name.
error-unsupported-no-button = o multímetro { $model } não tem a tecla { $button }
# $cable is a cable's name, e.g. BU-86X; $len and $got are byte counts.
error-unsupported-request-length = o cabo { $cable } aceita solicitações de { $len } bytes, não de { $got } bytes
# $link is usb (the USB cable), adapter (the Bluetooth adapter) or none (a
# link that does not say what it is); $baud a serial speed, e.g. 19200.
error-unsupported-baud =
    { $link ->
        [usb] o cabo USB não pode mudar a velocidade para { $baud } baud
        [adapter] o adaptador Bluetooth não pode mudar a velocidade para { $baud } baud
       *[none] a conexão não pode mudar a velocidade para { $baud } baud
    }
# A developer's error: the simulated meter has no bytes to decode.
error-unsupported-no-wire-format = parse_payload: o multímetro simulado não tem formato de dados brutos

## The meter did not do it: a command it was sent did not take, or the
## meter's state says it cannot. After "command rejected:". $button is a
## button's name; $state, $target and $now are modes, ranges or setting
## states.

# $button is a button's name, $state what the meter stayed in.
error-rejected-did-nothing = a tecla { $button } não teve efeito no estado { $state }
# $target is what was asked for, $now what the meter is in.
error-rejected-never-appeared = o estado pedido ({ $target }) nunca apareceu; o multímetro voltou ao estado { $now }
# $presses is how many times $button was pressed; $state is what the meter
# is in.
error-rejected-gave-up =
    { $presses ->
        [one] a tecla { $button } foi pressionada { $presses } vez sem resultado; o multímetro está no estado { $state }
       *[other] a tecla { $button } foi pressionada { $presses } vezes sem resultado; o multímetro está no estado { $state }
    }
# $mode is a mode.
error-rejected-mode-changed = o multímetro mudou para o modo { $mode }; o aplicativo parou de pressionar RANGE
# $reason is another sentence of this group, $mode the mode the meter was
# in before.
error-rejected-started-in = { $reason }; modo inicial: { $mode }
# $range is a range.
error-rejected-auto-did-nothing = a tecla AUTO não teve efeito; o multímetro continua na faixa { $range }
# $button is a button's name, $state a setting's state.
error-rejected-exit-did-nothing = EXIT { $button } não teve efeito; o multímetro continua no estado { $state }
# $setting is a setting's name, $target and $now its states.
error-rejected-switch-did-nothing = { $setting } { $target } não teve efeito; o multímetro continua no estado { $now }
# $sent is what was sent: a command word, or one of the error-sent messages
# below, or a setting's name and a state.
error-rejected-by-meter = o multímetro recusou { $sent }
error-rejected-zero-needs-capacitance = ZERO só funciona no modo de capacitância
error-rejected-rel-needs-reading = REL só funciona com um número no display
error-rejected-rel-is-on = REL já está ativado; saia de REL primeiro
# $code is a number.
error-rejected-unknown-model = o multímetro informa o código de modelo OWON { $code }, que o aplicativo ainda não conhece, por isso as teclas remotas estão desativadas
# $command is a command word.
error-rejected-model-unread = o código de modelo do multímetro não foi lido, por isso o comando { $command } está desativado: mantenha a tecla REL pressionada no multímetro para sair de REL
error-rejected-min-max-locks-range = MIN/MAX trava a faixa; saia de MIN/MAX antes de alterá-la
# $command is a command word or a setting's name.
error-rejected-replay = { $command }: a reprodução não aceita comandos

# What a refused command switched to, as error-rejected-by-meter names it.
# $mode is a mode.
error-sent-mode = o modo { $mode }
# $range is a range.
error-sent-range = a faixa { $range }

## Another meter: the one on the link is not one the meter chosen can
## read. After "invalid response:". Model names (OWON B35T+, Brymen BM82x)
## stay as they are. Each app follows the sentence with what to pick.

# An OWON model this tool does not read yet. $code is OWON's model number,
# e.g. 83. "This message" is the whole error, for the issue.
error-wrong-meter-not-yet = o multímetro informa o código de modelo OWON { $code }, de um modelo que o aplicativo ainda não lê; abra uma issue com esta mensagem
# An OWON model whose readings carry their sign where this tool does not
# read it yet. $code is OWON's model number, e.g. 55.
error-wrong-meter-sign-in-function-word = o multímetro informa o código de modelo OWON { $code }: envia o quadro OWON de 6 bytes com o sinal na palavra de função, que o aplicativo ainda não lê
# An OWON meter chosen by its model sends bytes, but never its 6-byte data
# frame. B35 and B35T are OWON models.
error-wrong-meter-no-six-byte-frames = o multímetro não envia quadros OWON de 6 bytes; pode ser um B35 ou B35T mais antigo, cujos quadros de 14 bytes o aplicativo ainda não lê
# An OWON meter that names no model sends frames of neither length. "This
# message" is the whole error, for the issue.
error-wrong-meter-untold-frame-kind = nem o código de modelo do multímetro nem os quadros recebidos mostram se são quadros OWON de 6 ou de 15 bytes; abra uma issue com esta mensagem

## Bluetooth: what the Bluetooth stack could not do. After "Bluetooth:".

# $detail is what the operating system said.
error-bluetooth-runtime = não foi possível iniciar o ambiente de execução do Bluetooth: { $detail }
error-bluetooth-no-adapter = não disponível neste computador
error-bluetooth-powered-off = desligado neste computador
# $role is write (the characteristic commands are written to) or notify
# (the one the meter's data arrives on).
error-bluetooth-no-characteristic =
    { $role ->
        [write] o dispositivo Bluetooth não tem nenhuma característica reconhecida para escrita de dados: não é um adaptador ou multímetro compatível, ou os serviços nunca foram resolvidos
       *[notify] o dispositivo Bluetooth não tem nenhuma característica reconhecida para notificação de dados: não é um adaptador ou multímetro compatível, ou os serviços nunca foram resolvidos
    }
# A Brymen meter refused the password this tool sends.
error-bluetooth-password-refused = o multímetro recusou a senha de conexão 0000; para redefini-la, { error-bluetooth-reset-gesture }
# How a Brymen BM788BT/BM787BT's password and name go back to the factory
# settings. The meter's activation steps in lib.ftl (activation-bm78xbt)
# end with the same sentence: word them alike. "Org" is what the display
# shows.
error-bluetooth-reset-gesture = mantenha a tecla Hz pressionada enquanto gira a chave seletora de OFF para capacitância em menos de 0.6 s: o multímetro exibe “Org”, e a senha de conexão e o nome Bluetooth voltam às configurações de fábrica
# A Brymen meter refused the login with error $code, a number.
error-bluetooth-login-refused =
    { $code ->
        [0] o multímetro recusou o login: erro de soma de verificação (código { $code })
        [1] o multímetro recusou o login: ID de canal inválido (código { $code })
        [2] o multímetro recusou o login: fora da faixa de configuração (código { $code })
        [5] o multímetro recusou o login: argumentos inválidos (código { $code })
        [6] o multímetro recusou o login: permissões insuficientes (código { $code })
        [one] o multímetro recusou o login: erro desconhecido (código { $code })
       *[other] o multímetro recusou o login: erro desconhecido (código { $code })
    }

## Not an error: the notice under the reading in the GUI when a Bluetooth
## adapter hands over a reading late, together with the next one.

late-readings-title = Algumas leituras Bluetooth chegam atrasadas
# $fix is command when a command follows that fixes it for this connection
# (it ends in a colon, and the command shows on the next line as it is), or
# none.
late-readings-advice =
    { $fix ->
        [command] Neste adaptador, uma leitura às vezes chega agrupada com a seguinte. Nenhuma se perde. Para obter carimbos de data/hora precisos até o multímetro se desconectar, execute:
       *[none] Neste adaptador, uma leitura às vezes chega agrupada com a seguinte. Nenhuma se perde.
    }
