### dmm-gui's --help: what the program and each option do.
###
### Generated from the doc comments in crates/dmm-gui/src/main.rs by
### `UPDATE_L10N=1 cargo test -p dmm-gui gui_help_ftl`: edit those, not this
### file. The run names each ID whose English changed. When the meaning changed
### too, delete that message from every language's gui-help.ftl, so it shows in
### English until it is translated again.
###
### IDs follow the options: dmm-gui--replay.help is what --replay does. clap
### wraps each paragraph to the terminal's width: keep a paragraph on one line.
### Option names (--device), values (auto, mock) and command lines ('dmm-cli
### list') stay as they are. The help built as the program runs is in
### gui/command-line.ftl, and clap's own words around these texts (Usage:,
### Options:, [possible values: …]) are in shared.ftl.

# dmm-gui
dmm-gui =
    .about = Aplicativo de interface gráfica para multímetros digitais USB e Bluetooth
# dmm-gui --device
dmm-gui--device =
    .help = Dispositivo a conectar [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...]
# dmm-gui --theme
dmm-gui--theme =
    .help = Usar este tema em vez do salvo nas configurações
# dmm-gui --renderer
dmm-gui--renderer =
    .help = Renderizador gráfico
# dmm-gui --adapter
dmm-gui--adapter =
    .help = Selecionar um adaptador específico quando houver mais de um ao alcance. Use o número de série ou o caminho HID de um cabo USB, ou o endereço de um adaptador ou multímetro Bluetooth, tal como aparecem em 'dmm-cli list'
# dmm-gui --no-bluetooth
dmm-gui--no-bluetooth =
    .help = Desativar a procura por dispositivos Bluetooth nesta sessão. Tem prioridade sobre a configuração “Procurar dispositivos Bluetooth”. Um endereço informado em --adapter é aberto mesmo assim
# dmm-gui --replay
dmm-gui--replay =
    .help = Reproduzir um arquivo gravado por 'dmm-cli read --format replay' ou por Exportar… → Reprodução…, em vez de se conectar a um multímetro
# dmm-gui --import
dmm-gui--import =
    .help = Abrir um arquivo exportado (CSV, JSON ou de reprodução), com os marcadores, em vez de se conectar a um multímetro
# dmm-gui --completions
dmm-gui--completions =
    .help = Gerar um script de preenchimento automático do shell e sair
