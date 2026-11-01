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
    .about = Ferramenta de linha de comando para multímetros digitais USB e Bluetooth
# dmm-cli --device
dmm-cli--device =
    .help = Dispositivo a conectar [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...]. Se omitido, usa `device_family` em ~/.config/dmm-tools/settings.json (gravado pelo dmm-gui) e, em seguida, `auto`, que detecta o multímetro pelo cabo USB
# dmm-cli --adapter
dmm-cli--adapter =
    .help = Selecionar um adaptador específico quando houver mais de um ao alcance. Use o número de série ou o caminho HID de um cabo USB, ou o endereço de um adaptador ou multímetro Bluetooth, como 'dmm-cli list' os exibe
# dmm-cli --no-bluetooth
dmm-cli--no-bluetooth =
    .help = Desativar a procura por Bluetooth nesta execução: nenhum adaptador ou multímetro é procurado e 'list' mostra apenas cabos. Tem prioridade sobre a configuração 'bluetooth' salva. Um endereço informado em --adapter ainda é aberto
# dmm-cli --lang
dmm-cli--lang =
    .help = Idioma das mensagens e da ajuda, ou `system` para o idioma do sistema. Se omitido, usa `language` do arquivo de configurações (gravado pela linha Idioma do dmm-gui) e, em seguida, o idioma do sistema
# dmm-cli list
list =
    .about = Listar os cabos USB conectados e os adaptadores e multímetros Bluetooth ao alcance
# dmm-cli info
info =
    .about = Conectar e exibir as informações do dispositivo
# dmm-cli read
read =
    .about = Ler as medições continuamente
# dmm-cli read --interval-ms
read--interval-ms =
    .help = Manter no máximo uma leitura por intervalo, em milissegundos (0 = todas as leituras que o multímetro produz)
# dmm-cli read --format
# .value-… says what that value does, in --help's list of values.
read--format =
    .help = Formato de saída [padrão: text, ou o que a extensão de -o indicar]
    .value-replay = Os próprios quadros do multímetro, para reproduzir com --replay
# dmm-cli read --output
read--output =
    .help = Arquivo de saída (stdout se não especificado). Sem nome, o arquivo recebe o nome do multímetro, do modo e do início da execução
# dmm-cli read --count
read--count =
    .help = Número de leituras (0 = ilimitado: até Ctrl+C ou até o fim de um arquivo --replay)
# dmm-cli read --integrate
read--integrate =
    .help = Exibir a integral acumulada no tempo (carga nos modos de corrente, V·s para tensão)
# dmm-cli read --scale
read--scale =
    .help = Multiplicar a leitura, tomada em unidades de base (V, A, Ω, …), por FACTOR. Um alicate de 10 mV/A usa --scale 100; uma ponta de prova 100:1 usa --scale 100
# dmm-cli read --offset
read--offset =
    .help = Somar VALUE após o ajuste de escala (32 com --scale 1.8 converte °C em °F)
# dmm-cli read --unit
read--unit =
    .help = Rotular a leitura ajustada com esta unidade em vez da unidade de base do multímetro
# dmm-cli read --alarm-high
read--alarm-high =
    .help = Avisar quando a leitura subir acima de VALUE, em unidades de base (V, A, Ω, …), ou na --unit de uma leitura ajustada
# dmm-cli read --alarm-low
read--alarm-low =
    .help = Avisar quando a leitura cair abaixo de VALUE, como em --alarm-high
# dmm-cli read --alarm-hysteresis
read--alarm-hysteresis =
    .help = Quanto uma leitura precisa voltar para dentro do limite antes que o mesmo limite dispare o alarme de novo: um valor na unidade dos limites, ou uma porcentagem do limite (`1%`) [padrão: algumas contagens do último dígito do multímetro]
# dmm-cli read --alarm-bell
read--alarm-bell =
    .help = Tocar o sinal sonoro do terminal a cada alarme
# dmm-cli read --mock-mode
read--mock-mode =
    .help = Fixar o dispositivo simulado em um modo específico (só com --device mock). Sem esta opção, vale o modo simulado do arquivo de configurações; na falta dele, o dispositivo simulado percorre todos os modos
# dmm-cli read --replay
read--replay =
    .help = Reproduzir um arquivo gravado por --format replay em vez de abrir um multímetro; termina com o arquivo
# dmm-cli read --import
read--import =
    .help = Ler um arquivo CSV, JSON ou de reprodução exportado em vez de abrir um multímetro, sem esperar: para convertê-lo ou exibir o resumo
# dmm-cli command
command =
    .about = Enviar ao multímetro um comando que simula o pressionamento de uma tecla. Execute sem argumentos para listar os comandos disponíveis para o dispositivo selecionado
# dmm-cli command <ACTION>
command--action =
    .help = Nome do comando (execute sem argumentos para ver os comandos disponíveis)
# dmm-cli get
get =
    .about = Listar as opções que as configurações do multímetro oferecem a partir do estado atual. Execute sem argumento para ver todas as configurações que oferecem opções
# dmm-cli get <SETTING>
get--setting =
    .help = Configuração a listar (omita para todas)
# dmm-cli get --format
get--format =
    .help = Formato de saída
# dmm-cli get --mock-mode
get--mock-mode =
    .help = Fixar o dispositivo simulado em um modo específico (só com --device mock). Sem esta opção, vale o modo simulado do arquivo de configurações; na falta dele, o dispositivo simulado percorre todos os modos
# dmm-cli set
set =
    .about = Alterar uma das configurações do multímetro sem tocar nele. Execute sem valor para listar os valores que essa configuração aceita a partir do estado atual
# dmm-cli set <SETTING>
set--setting =
    .help = Configuração a alterar
# dmm-cli set <CHOICE>
set--choice =
    .help = Rótulo de um valor, ou um trecho único de um rótulo, da listagem (execute sem ele para vê-los)
# dmm-cli set --mock-mode
set--mock-mode =
    .help = Fixar o dispositivo simulado em um modo específico (só com --device mock). Sem esta opção, vale o modo simulado do arquivo de configurações; na falta dele, o dispositivo simulado percorre todos os modos
# dmm-cli debug
debug =
    .about = Modo de despejo hexadecimal bruto para depuração do protocolo
# dmm-cli debug --count
debug--count =
    .help = Número de solicitações a enviar (0 = ilimitado)
# dmm-cli debug --interval-ms
debug--interval-ms =
    .help = Manter no máximo uma leitura por intervalo, em milissegundos (0 = todas as leituras)
# dmm-cli completions
completions =
    .about = Gerar o preenchimento automático do shell
# dmm-cli completions <SHELL>
completions--shell =
    .help = Shell para o qual gerar o preenchimento automático
# dmm-cli capture
capture =
    .about = Captura guiada do protocolo para relatórios de bugs e verificação
# dmm-cli capture --output
capture--output =
    .help = Arquivo de saída (padrão: capture-<device>.yaml). Substitui a nomeação automática
# dmm-cli capture --steps
capture--steps =
    .help = Executar apenas etapas específicas (IDs separados por vírgula, por exemplo "dcmv,temp,duty")
# dmm-cli capture --unverified
capture--unverified =
    .help = Executar apenas as etapas ainda não confirmadas em um multímetro real, mais a passagem livre
# dmm-cli capture --plan
capture--plan =
    .help = Executar as etapas do arquivo de plano de um mantenedor em vez da lista do próprio dispositivo
# dmm-cli capture --sniff
capture--sniff =
    .help = Não confiar em nada que o analisador diga: detectar as etapas pelas mudanças nos bytes brutos e confirmar cada uma
# dmm-cli capture --no-drive
capture--no-drive =
    .help = Não deixar o aplicativo ajustar faixas e indicadores por conta própria após cada etapa de modo
# dmm-cli capture --settle
capture--settle =
    .help = Esperar MS antes de amostrar cada etapa, para leituras que se estabilizam devagar
# dmm-cli capture --list-steps
capture--list-steps =
    .help = Listar todos os IDs de etapa disponíveis e sair
# dmm-cli capture --format
capture--format =
    .help = Como --list-steps exibe a lista (md é a lista de verificação da issue)
