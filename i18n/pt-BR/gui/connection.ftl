### The connection notices drawn in the reading column under the reading
### (or, in the big-meter modes, its title in place of the reading and the
### rest in a tooltip), and the toasts: the short messages that float under
### the top bar's right end for a few seconds.
###
### ⚙ is the settings button's icon, which the notices point at. The
### notices are hard-wrapped to fit a narrow column: keep their line breaks
### where a line would otherwise run long.

## While there is nothing to show yet, or no more to come.

# The meter is being identified: each kind of meter gets a turn to answer.
connection-detecting = Detectando o multímetro…
# A recording being played back reached its last reading.
connection-ended = A gravação terminou
# A stretch of a recording being played back that has no readings in it.
connection-no-frames = Nenhum quadro neste trecho da gravação
# The meter has gone quiet for a moment. $dots is one to four dots, padded
# with spaces to four characters, that grow while the wait lasts: keep it
# at the end, with nothing after it.
connection-waiting = Aguardando o multímetro{ $dots }
# Under the waiting title, when a meter is picked in Settings.
connection-waiting-check-selection = Verifique se o dispositivo certo está selecionado em Configurações (⚙) ou escolha Detecção automática
# Under the waiting title, when Settings is on Auto-detect.
connection-waiting-switch-on = Ligue a transmissão de dados do multímetro, ou escolha o modelo em Configurações (⚙)

## A failed connection: a title naming the problem, the steps to try, and a
## closing line.

# The closing line under the steps. "Connect" is the top bar's button.
connection-click-connect = Clique em “Conectar” depois de resolver o problema.
# The closing line after the Bluetooth steps when nothing answered at the
# address given with --adapter, a command-line option kept as it is. "It"
# is the scan command the steps above name.
connection-restart-with-address = Reinicie usando --adapter com o endereço exibido por esse comando.

# A meter that connects over Bluetooth only, and Bluetooth was not looked at.
connection-bluetooth-not-searched = Busca Bluetooth não realizada
# Under that title when Bluetooth is switched off. $error is the sentence
# saying why it was not searched; $hint says how to switch it
# on.
connection-bluetooth-not-searched-off = { $error }. { $hint }
# Under that title otherwise. $error is the sentence saying why.
connection-bluetooth-not-searched-why = { $error }.

# --adapter, a command-line option, named a USB device that isn't there.
connection-adapter-not-found = Adaptador não encontrado
# $selector is the value given with --adapter.
connection-adapter-not-matched = Nenhum dispositivo corresponde a --adapter '{ $selector }'.
# After the list of what is plugged in.
connection-adapter-restart = Reinicie com o valor correto de --adapter.

# The cable or Bluetooth link is up but no meter answered. $link is usb (a
# USB cable), adapter (a Bluetooth adapter plugged into the meter) or radio
# (the meter's own Bluetooth). A list of meters and their steps follows the
# colon.
connection-not-identified =
    { $link ->
        [adapter] O adaptador Bluetooth está conectado, mas nenhum multímetro se identificou.
        [radio] A conexão Bluetooth está estabelecida, mas nenhum multímetro se identificou.
       *[usb] O cabo USB está conectado, mas nenhum multímetro se identificou.
    }

    Ligue a transmissão de dados do multímetro, ou escolha o modelo em Configurações (⚙):
# After that list.
connection-not-identified-report = O multímetro já transmite e ainda não foi reconhecido? Escolha o modelo em Configurações (⚙) e avise-nos.

# The meter stopped answering. $link is usb (a USB cable), adapter (a
# Bluetooth adapter plugged into the meter), radio (the meter's own
# Bluetooth) or none (a simulated meter, on no link). $model is the meter
# picked, e.g. UNI-T UT61E+; $steps are its own steps for switching data
# transmission on, several lines.
connection-no-response =
    { $link ->
        [usb]
            O cabo USB está conectado, mas o multímetro
            não responde (modelo selecionado: { $model }).
        [adapter]
            O adaptador Bluetooth está conectado, mas o multímetro
            não responde (modelo selecionado: { $model }).
        [radio]
            A conexão Bluetooth está estabelecida, mas o multímetro
            não responde (modelo selecionado: { $model }).
       *[none]
            A conexão está estabelecida, mas o multímetro
            não responde (modelo selecionado: { $model }).
    }

    Se este for o dispositivo errado, altere-o em Configurações (⚙) ou escolha Detecção automática.
    Caso contrário, ligue a transmissão de dados:
    { $steps }
# The same, on Auto-detect: no meter is picked to name.
connection-no-response-auto =
    Ligue a transmissão de dados do multímetro,
    ou escolha o modelo em Configurações (⚙).

# The meter answered, but with data that could not be read: a corrupt
# frame, or a mode this tool does not know. The error itself follows.
connection-bad-data = O multímetro enviou dados ilegíveis
# A dropped link that was not reconnected. shared.ftl's several-cables
# follows, then connection-several-cables-action. "Connect" is the top bar's
# button.
connection-several-cables = Conexão perdida
connection-several-cables-action = Deixe conectado apenas o cabo deste multímetro e depois clique em Conectar.
# A toast, when that notice ends a running recording. What it holds can still
# be exported.
connection-several-cables-recording = Gravação interrompida: a conexão caiu e não é possível distinguir o cabo USB deste multímetro dos demais
# Any other failure to connect or to keep reading. The error itself
# follows, then connection-click-connect.
connection-failed = Falha na conexão

# A meter that stopped answering in the middle of a session. $title is the
# shared notice title, "No response from meter".
connection-no-response-check = { $title }: verifique o dispositivo selecionado e a transmissão de dados
# The reading thread stopped by itself: it crashed, or gave up connecting.
connection-acquisition-stopped = A aquisição parou inesperadamente: reconecte para continuar

# The last line of a notice for a meter whose support isn't fully verified;
# a link. $warning is the sentence saying how far it is verified.
connection-feedback = { $warning } Enviar comentários.
    .hover = Abrir o rastreador de problemas do projeto no GitHub para enviar comentários

## The notice under the reading when the Bluetooth link delivers readings
## two at a time, which can end with a command that fixes it.

# Copies the command to the clipboard.
connection-late-copy = Copiar
# A toast, once the command is copied.
connection-late-copied = Comando copiado

## Toasts.

# A meter identified itself while Settings was on Auto-detect. $meter is
# its name, e.g. UNI-T UT61E+. $saved is yes when Settings now names it, no
# for a session that detects every time.
connection-detected =
    { $saved ->
        [yes] Multímetro { $meter } detectado e salvo como seu dispositivo. Escolha Detecção automática em Configurações para detectar de novo.
       *[no] Multímetro { $meter } detectado
    }
# The same, when the name the meter gave for itself differs; $meter and
# $saved as there. $reported is that name, e.g. UT216XD.
connection-detected-reports =
    { $saved ->
        [yes] Multímetro { $meter } detectado (o multímetro se identifica como “{ $reported }”) e salvo como seu dispositivo. Escolha Detecção automática em Configurações para detectar de novo.
       *[no] Multímetro { $meter } detectado (o multímetro se identifica como “{ $reported }”)
    }
# The meter named another model than the one picked in Settings. $reported
# is the name it gave, e.g. UT161E; $picked the model picked, $named the
# model that name belongs to, e.g. UNI-T UT161E.
connection-other-model = O multímetro se identifica como “{ $reported }”, não como o modelo { $picked } selecionado. Escolha { $named } ou Detecção automática em Configurações.
# Detection settled on a meter, then the meter named another. $reported is
# the name it gave, $named the model it belongs to. $saved is yes when
# Settings now names that model.
connection-named-after-detection =
    { $saved ->
        [yes] O multímetro se identifica como “{ $reported }”, e esse modelo foi salvo como seu dispositivo: reconecte para tratá-lo como { $named }.
       *[no] O multímetro se identifica como “{ $reported }”: reconecte para tratá-lo como { $named }.
    }

# The recording stopped because Buffer size, in Settings, was lowered below
# what it held. $count is how many samples it took, $kept the same number
# written short, e.g. 1.2K. Export… is a button.
connection-buffer-shrunk =
    { $count ->
        [0] Gravação interrompida: { $kept } amostras mantidas; salve-as com “Exportar…”
        [one] Gravação interrompida: { $kept } amostra mantida; salve-a com “Exportar…”
       *[other] Gravação interrompida: { $kept } amostras mantidas; salve-as com “Exportar…”
    }
# The recording filled the buffer. $count is its size in samples, $kept the
# same number written short, e.g. 500K.
connection-buffer-full =
    { $count ->
        [one] Gravação interrompida: buffer cheio ({ $kept } amostra)
       *[other] Gravação interrompida: buffer cheio ({ $kept } amostras)
    }

# A button command sent to the meter failed. $command is its name as the
# meter's protocol knows it, e.g. hold; $error says why.
connection-command-failed = Falha no comando '{ $command }': { $error }
# Switching a meter setting failed. $setting is mode, range or the
# setting's lowercase name as the command line takes it (hold, rel, minmax,
# peak); $error says why.
connection-switch-failed =
    { $setting ->
        [mode] Falha ao alterar o modo: { $error }
        [range] Falha ao alterar a faixa: { $error }
       *[other] Falha ao alterar a configuração { $setting }: { $error }
    }

# The × that closes a toast or a notice.
connection-dismiss =
    .hover = Ignorar
    .a11y = Ignorar a notificação
