### The settings panel, which the ⚙ button at the top bar's right end opens
### under the bar (assets/gui-settings.webp): rows of checkboxes and of
### chips, each row led by its caption, in groups a rule apart. The colour
### and theme rows have a file of their own.
###
### Rows wrap on a narrow window, so keep the chips short. $keys and the
### other shortcut variables are keyboard shortcuts as this computer writes
### them, e.g. Ctrl+O or ⌘O. Text in parentheses starting with -- names a
### command-line option, kept as it is, that set the row for this session
### only.

## The meter: which one, how the mock runs, and how a connection is made.

# The Device row's caption; .a11y names its dropdown for a screen reader.
settings-device = Dispositivo:
    .a11y = Dispositivo
# The Device dropdown, and its first entry, when no particular meter is
# picked: each kind of meter is tried in turn.
settings-auto-detect = Detecção automática
# The Device dropdown while --device picked the meter for this session.
# $name is the meter, e.g. UNI-T UT61E+, or Auto-detect.
settings-device-pinned = { $name } (--device)
# Beside the greyed Device dropdown while a recording is played back:
# --replay is the command-line option that opened it.
settings-device-replay = (reinicie sem --replay para escolher um multímetro)
# Beside the greyed Device dropdown while the session runs on a simulated
# clock, set by command-line options for testing.
settings-device-clock = (reinicie sem as opções de relógio para escolher um multímetro)

## The open Device dropdown: Auto-detect, then every meter under its brand.

# An entry's hover and screen-reader name. $name is the meter, e.g. UNI-T
# UT61E+; $tag how far its support is verified, e.g. partly verified.
settings-device-entry = { $name } ({ $tag })

## The Mock mode row, shown while a simulated meter is picked. Its other
## chips are the simulated modes' names, kept as they are.

# The first chip: the simulated meter steps through every mode. $pinned is
# yes when --mock-mode picked it for this session, no otherwise.
settings-mock-auto =
    { $pinned ->
        [yes] Auto (em ciclo) (--mock-mode)
       *[no] Auto (em ciclo)
    }
    .hover = Percorrer todos os modos simulados para testar a interface
# The chip for the mode --mock-mode picked for this session. $mode is its
# name, e.g. dcv.
settings-mock-mode-pinned = { $mode } (--mock-mode)
settings-mock-mode = Modo simulado:

## How a connection is made: checkboxes on one row.

settings-auto-connect = Conectar ao iniciar
    .hover = Abrir a conexão USB automaticamente quando o aplicativo iniciar
# Asking the meter its name makes it beep.
settings-device-name = Mostrar o nome do dispositivo ao conectar (com bip)
    .hover = Consultar o nome do multímetro depois de conectar: o multímetro emite um bip
# The Bluetooth checkbox while --no-bluetooth switched the search off for
# this session. $setting is the checkbox's own label, e.g. Look for
# Bluetooth devices.
settings-bluetooth-pinned = { $setting } (--no-bluetooth)
settings-bluetooth =
    .hover = Procurar um adaptador ou multímetro Bluetooth ao alcance quando nenhum cabo USB responder. Vale a partir da próxima conexão.

## What is kept of the readings.

settings-sample-interval = Intervalo de amostragem:
# The first Sample interval chip; the others are durations, e.g. 300ms or 2s.
settings-interval-every-reading = Todas as leituras
    .hover = Manter cada leitura que o multímetro produz, no ritmo dele
# The other Sample interval chips' hover. $unit is second for exactly one
# second, seconds for a whole number of seconds and ms otherwise; $n is how
# many seconds or milliseconds. s and ms are unit symbols.
settings-interval-chip =
    .hover =
        { $unit ->
            [second] No máximo uma leitura por segundo: a mais próxima de cada marca de tempo
            [seconds] No máximo uma leitura a cada { $n } s: a mais próxima de cada marca de tempo
           *[ms] No máximo uma leitura a cada { $n } ms: a mais próxima de cada marca de tempo
        }

settings-buffer-size = Tamanho do buffer:
# A Buffer size chip's hover; the chips are sample counts written short,
# e.g. 500K. $count is the chip's number of samples and $samples the same
# number written short. $memory is what they take, e.g. ≈194 MB, and $span
# how long they last, e.g. 14 h. $pace is every-reading when the Sample
# interval keeps every reading, whose pace the estimate can't know, and
# interval otherwise.
settings-buffer-size-chip =
    .hover =
        { $pace ->
            [every-reading]
                { $count ->
                    [one] Manter até { $samples } amostra no gráfico e para exportação: { $memory }, cerca de { $span } a 10 leituras por segundo. Uma gravação parada mantida junto pode ocupar outro tanto
                   *[other] Manter até { $samples } amostras no gráfico e para exportação: { $memory }, cerca de { $span } a 10 leituras por segundo. Uma gravação parada mantida junto pode ocupar outro tanto
                }
           *[interval]
                { $count ->
                    [one] Manter até { $samples } amostra no gráfico e para exportação: { $memory }, cerca de { $span } no intervalo de amostragem atual. Uma gravação parada mantida junto pode ocupar outro tanto
                   *[other] Manter até { $samples } amostras no gráfico e para exportação: { $memory }, cerca de { $span } no intervalo de amostragem atual. Uma gravação parada mantida junto pode ocupar outro tanto
                }
        }
# After the Buffer size chips: what the picked size costs. $memory is the
# memory it takes, e.g. ≈194 MB; $span how long it lasts, e.g. 14 h.
settings-buffer-cost = ({ $memory }, cerca de { $span })

## Which panels show, and what the Specifications panel shows: checkboxes
## on one row.

settings-graph = Gráfico
    .hover = Mostrar o gráfico contínuo das leituras ao longo do tempo
# σ is the standard deviation, as the Statistics panel writes it.
settings-statistics = Estatísticas
    .hover = Mostrar Mín. / Máx. / Média / σ / integral da sessão ao vivo
settings-recording = Gravação
    .hover = Mostrar os controles de gravação e a lista de amostras
# The Specifications panel's checkbox. $fields is shown while the panel is
# on and the checkboxes for its fields follow it, hidden otherwise.
settings-specs =
    { $fields ->
        [shown] Especificações:
       *[hidden] Especificações
    }
    .hover = Mostrar a precisão e a resolução do modo atual
# The Specifications fields, as that panel names them.
settings-specs-resolution = Resolução
    .hover = Mostrar a resolução nas especificações
settings-specs-accuracy = Precisão
    .hover = Mostrar a precisão nas especificações
# Input Z is the input impedance.
settings-specs-input-z = Z de entrada
    .hover = Mostrar a impedância de entrada no painel Especificações completo
settings-specs-notes = Notas
    .hover = Mostrar as notas do manual no painel Especificações completo

## The window.

# The Language row's caption, shown once the app speaks more than one
# language. In every language but English, keep the English word in
# parentheses after yours, e.g. Langue (Language):, so someone who can't
# read the window can still find the row. .a11y names its dropdown for a
# screen reader, in your word alone.
settings-language = Idioma (Language):
    .a11y = Idioma
    .hover = O idioma da interface
# The dropdown's first entry: the window speaks the system's language.
# $language is the language that is, by its name in itself, e.g. Français
# or 简体中文. The other entries are each language's name in itself, kept
# as they are.
settings-language-system = Sistema ({ $language })
    .hover = Seguir o idioma do sistema
# Weak text after your language's name in the dropdown while no native
# speaker has reviewed it yet. Shown only next to your own language's name,
# whatever language the window speaks, so its speakers can read it: e.g.
# Português (Brasil)  tradução automática.
settings-language-machine-translated = tradução automática
# A screen reader's name for that entry, also in your language whatever the
# window speaks. $language is your language's name in itself, e.g.
# Português (Brasil).
settings-language-machine-translated-entry = { $language }, tradução automática
# The weak line under the Language row while such a language is spoken,
# then a link that opens the translators' guide on GitHub in the web
# browser; ↗ marks a link.
settings-language-unreviewed = Tradução automática, ainda não revisada.
settings-language-review-link = Ajudar a revisar ↗
    .hover = Abrir o guia para tradutores no GitHub
# A toast when Chinese is picked and the computer has no font for it. It
# is always shown in English, which the app's own fonts draw: no need to
# translate it.
settings-language-no-chinese-font = No font for Chinese characters was found. Install a Simplified Chinese font, such as Noto Sans CJK SC, then restart dmm-gui.

# The Zoom chips are percentages, e.g. 150%.
settings-zoom = Zoom:
# A Zoom chip's hover. $level is its scale in percent, e.g. 150; $step the
# keys that step the zoom.
settings-zoom-chip =
    .hover = Exibir a interface em { $level }% ({ $step } para mudar passo a passo)
# The 100% chip's hover. $reset is the keys that go back to 100%, $step
# those that step the zoom.
settings-zoom-reset-chip =
    .hover = Exibir a interface em 100% ({ $reset }, ou { $step } para mudar passo a passo)
# Window decorations are the title bar and border.
settings-hide-decorations = Ocultar a moldura da janela
    .hover = Janela sem moldura: use { $keys } para restaurá-la
# On Wayland the checkbox is greyed: .wayland is its hover there, and
# settings-always-on-top-wayland the note beside it. Keep both the same as
# shortcuts-always-on-top-toast, which Ctrl+T shows there.
settings-always-on-top = Sempre visível
    .hover = Manter a janela acima das outras janelas da área de trabalho ({ $keys })
    .wayland = Não disponível no Wayland: clique com o botão direito na barra de título para manter a janela acima das outras
# The note after the greyed checkbox on Wayland, in small grey text.
settings-always-on-top-wayland = (Não disponível no Wayland: clique com o botão direito na barra de título para manter a janela acima das outras)

## The update check, in builds downloaded from GitHub.

settings-check-updates = Verificar novas versões
    .hover = Uma vez por dia, perguntar ao GitHub se há uma versão mais recente. O GitHub vê seu endereço IP; nada sobre o seu multímetro é enviado.
# The link the top bar shows when a newer release is out; ↗ marks a link.
# $version is the release, e.g. v0.10.0.
settings-update-release = { $version } disponível ↗
# The same for a newer nightly build, which has no version number.
settings-update-dev = Nova versão dev ↗
# Its shorter form, when the top bar is short of room.
settings-update-dev-short = Atualizar ↗

## How a screen reader hears the reading change.

# A screen reader speaks the reading each time it changes; this row sets how
# often at most. Its chips are durations like the Sample interval row's,
# Every reading taking that row's first chip's label, then Never.
settings-announce = Anunciar leituras:
settings-announce-every-reading =
    .hover = Anunciar cada nova leitura. Um leitor de tela pode se interromper quando as leituras chegam mais rápido do que ele as lê.
# The duration chips' hover. $n is a whole number of seconds.
settings-announce-chip =
    .hover =
        { $n ->
            [one] Anunciar a leitura no máximo uma vez por segundo. Uma mudança de modo é anunciada imediatamente.
           *[other] Anunciar a leitura no máximo uma vez a cada { $n } segundos. Uma mudança de modo é anunciada imediatamente.
        }
settings-announce-never = Nunca
    .hover = Não anunciar as leituras quando elas mudam. Uma perda de conexão continua sendo anunciada.
# A checkbox under the Announce readings row. Focus is keyboard focus, or
# where the screen reader's cursor is.
settings-announce-focused = Somente quando a leitura tem o foco
    .hover = Anunciar a leitura somente quando o leitor de tela está nela, no ritmo escolhido acima. A tecla Tab leva até ela.

## How the graph's lines look, the last row.

settings-graph-lines = Linhas do gráfico:
# Sub-value lines are the extra traces a meter that shows several values
# at once draws beside the main one.
settings-graph-lines-patterned = Tracejadas
    .hover = Linhas dos valores secundários tracejadas e pontilhadas, distinguíveis sem cor
settings-graph-lines-solid = Contínuas
    .hover = Linhas contínuas, distinguidas pela cor e pela legenda
