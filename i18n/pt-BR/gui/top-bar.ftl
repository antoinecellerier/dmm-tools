### The top bar: the meter's name and the connection buttons on the left,
### the connection status in the middle, and the version, Help & Feedback,
### shortcuts and settings buttons on the right
### (assets/gui-wide-layout.webp).
###
### The bar is one row on a wide window: keep these short. $keys is a
### keyboard shortcut as this computer writes it, e.g. Ctrl+O or ⌘O.

## The meter's name, at the left end.

# Shown in place of a meter's name while Auto-detect hasn't found one yet.
top-bar-auto-detect = Detecção automática
# Shown in place of a meter's name for an imported file that names none.
top-bar-imported-file = Arquivo importado

## Connection buttons.

top-bar-connect = Conectar
    .hover = Abrir a conexão USB com o multímetro selecionado ({ $keys })
top-bar-disconnect = Desconectar
    .hover = Fechar a conexão ativa com o multímetro ({ $keys })
# A button; ▶ and ⏸ are part of it.
top-bar-resume = ▶ Continuar
    .hover = Continuar a aquisição ({ $keys })
top-bar-pause = ⏸ Pausar
    .hover = Suspender a aquisição: interrompe a gravação e as atualizações do gráfico ({ $keys })
# Clears the graph and the statistics, not the recording.
top-bar-clear = Limpar
    .hover = Limpar o histórico do gráfico e as estatísticas ({ $keys })
# While the connection is being retried after it dropped.
top-bar-reconnecting = Reconectando…
# $attempt counts the retries, from 1.
top-bar-reconnecting-attempt = Reconectando (tentativa { $attempt })…
top-bar-reconnecting-hover = Tentando reconectar automaticamente: clique em Desconectar para parar
# $error is the last attempt's error, in the language of the system that raised it.
top-bar-reconnecting-hover-error =
    Tentando reconectar automaticamente: clique em Desconectar para parar.
    Último erro: { $error }
top-bar-stop-retrying = Desconectar
    .hover = Parar as tentativas e fechar a conexão ({ $keys })

## The status beside the coloured dot. $name is the meter's name, e.g.
## UT61E+. $link is how it is connected (USB cable, Bluetooth). $paused is
## yes or no.

# The meter's name when it didn't report one.
top-bar-connected = Conectado
top-bar-status =
    { $paused ->
        [yes] { $name } (em pausa)
       *[no] { $name }
    }
top-bar-status-link =
    { $paused ->
        [yes] { $name } · { $link } (em pausa)
       *[no] { $name } · { $link }
    }
# An imported file: $name is its meter, $file its file name.
top-bar-status-file = { $name } · { $file }
top-bar-disconnected = Desconectado

## The status's hover: how the meter is connected, spelled out.
## $link is usb (a USB cable), adapter (a Bluetooth adapter plugged into the
## meter), radio (the meter's own Bluetooth) or none.

top-bar-link-live =
    { $link ->
        [usb] Conectado pelo cabo USB
        [adapter] Conectado pelo adaptador Bluetooth
        [radio] Conectado pelo Bluetooth do próprio multímetro
       *[none] Multímetro simulado, sem conexão
    }
# A recording being played back: how it was recorded.
top-bar-link-recorded =
    { $link ->
        [usb] Gravado pelo cabo USB
        [adapter] Gravado pelo adaptador Bluetooth
        [radio] Gravado pelo Bluetooth do próprio multímetro
       *[none] Gravado por uma conexão não identificada
    }
# $path is the file's full path.
top-bar-replaying = Reproduzindo { $path }
# $readings is how many readings the file held; $path is the file's full
# path.
top-bar-imported-from =
    { $readings ->
        [0] { $readings } leituras importadas de { $path }
        [one] { $readings } leitura importada de { $path }
       *[other] { $readings } leituras importadas de { $path }
    }

## The support badge, after the status, for a meter not yet fully verified.
## A link to report feedback. Capitals, as the meter's own annunciators.

top-bar-badge-verified = VERIFICADO
top-bar-badge-partly-verified = VERIF. PARCIAL
top-bar-badge-experimental = EXPERIMENTAL
# $warning is the sentence saying how far the meter's support is verified.
top-bar-badge-hover = { $warning } Clique para enviar comentários.

## The right end: the version, which opens What's New; Help & Feedback; the
## shortcut list; Settings.

top-bar-version =
    .hover = Mostrar as novidades: as notas desta versão
    .a11y = Mostrar as notas de versão
# The version's hover when the bar has no room for the update link.
# $version is the newer release, e.g. v0.10.0.
top-bar-version-update =
    .hover =
        Mostrar as novidades: as notas desta versão
        { $version } está disponível: aumente a largura da janela para ver o link
    .a11y = Mostrar as notas de versão. { $version } está disponível
# The update link; $version is the newer release, e.g. v0.10.0.
top-bar-update =
    .hover = Abrir a página da versão { $version } no GitHub
    .a11y = { $version } disponível: abrir a página da versão
top-bar-project-link =
    .hover = Abrir a página do projeto dmm-tools no GitHub
# The ? button. $keys and $other are its two keys, ? and F1.
top-bar-shortcuts =
    .hover = Mostrar os atalhos de teclado e os gestos do mouse ({ $keys } ou { $other })
    .a11y = Atalhos de teclado e gestos do mouse
# The ⚙ button.
top-bar-settings =
    .hover = Mostrar ou ocultar o painel de configurações
    .a11y = Configurações
