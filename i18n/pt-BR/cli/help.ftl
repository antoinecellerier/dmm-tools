### The parts of dmm-cli's --help built as it runs: the first lines of the
### help of --device and --mock-mode, which go on with lists made from the
### meters, and the notes after the options. What each command and option
### does is in cli-help.ftl.
###
### Words in --flag form, setting names (device_family, mock_mode) and
### environment variables (RUST_LOG, LANG) are typed by the user and stay as
### they are. Command lines, file paths and values arrive as variables.

## The first lines of two options' long help (dmm-cli --help, dmm-cli read
## --help).

# Before the list of devices.
help-device-intro = Dispositivo a conectar.
# Before the list of the simulated meter's modes.
help-mock-mode-intro = Fixar o dispositivo simulado em um modo de medição específico em vez de percorrer os modos automaticamente. Só tem efeito com --device mock. Sem esta opção, vale o mock_mode do arquivo de configurações (o modo simulado do dmm-gui).

## The words around every command's help: its headings, what follows an
## option, and the help of the built-in -h, -V and help. They are the same
## on every page of dmm-cli --help and dmm-cli <command> --help.

# The headings, each before its list.
help-usage = Uso:
help-commands = Comandos:
help-arguments = Argumentos:
help-options = Opções:
# After an option's help, in brackets. $value is what the option is when not
# given, e.g. 500.
help-default = [padrão: { $value }]
# After an option's help, in brackets. $values is the values it takes,
# separated by commas.
help-possible-values = [valores possíveis: { $values }]
# Before the list of values an option takes, each with what it does.
help-possible-values-heading = Valores possíveis:
# What -h and --help do: on a command with more detail under --help, the two
# notes say how to see more or less.
help-flag = Exibir a ajuda
help-flag-short = Exibir a ajuda (veja mais com '--help')
help-flag-long = Exibir a ajuda (veja um resumo com '-h')
# What -V and --version do.
help-version-flag = Exibir a versão
# The help command, listed among the commands, and its argument.
help-command = Exibir esta mensagem ou a ajuda dos subcomandos informados
help-command-subcommand = Exibir a ajuda dos subcomandos

## After the options of dmm-cli -h.

help-see-long-help = Execute com --help para ver a lista completa de dispositivos compatíveis e o caminho do arquivo de configurações compartilhado.
# $setting is NO_COLOR=1.
help-no-color = Defina { $setting } para desativar a saída colorida.

## After the options of dmm-cli --help: two sections, each a heading in
## capitals and indented lines. clap doesn't wrap these lines: keep them
## short.

help-configuration = CONFIGURAÇÃO:
# The settings file's path follows on the next line.
help-settings-file = Arquivo de configurações (compartilhado com o dmm-gui):
# Where --device comes from when the command line doesn't give it: three
# numbered lines in order.
help-device-precedence = Precedência de --device:
help-precedence-flag = Opção de linha de comando
help-precedence-settings = device_family do arquivo de configurações acima
help-precedence-auto = auto: detectar o multímetro conectado
help-environment = AMBIENTE:
# Beside RUST_LOG; each line of the message is a line of the help. $filter
# is the filter that shows everything the library sends and receives.
help-env-rust-log =
    Filtro de log. Sem ele, são exibidos os avisos de dmm_lib e todos os erros.
    Use `{ $filter }` para depurar a comunicação byte a byte.
# Beside NO_COLOR.
help-env-no-color = Defina como 1 para desativar as cores no terminal.
# Beside LANGUAGE; each line of the message is a line of the help. The
# variable names stay as they are. On macOS and Windows the system's
# language applies instead.
help-env-language =
    Idioma das mensagens e da ajuda no Linux, lido pelo gettext nesta ordem:
    LANGUAGE, depois LC_ALL, LC_MESSAGES, LANG. Os ambientes de desktop do
    Debian e do Ubuntu definem LANGUAGE, que tem prioridade sobre LANG.
# Beside LC_ALL; each line of the message is a line of the help. C and
# POSIX are the locale names that mean English, on every system.
help-env-lc-all =
    C ou POSIX na primeira variável definida entre LC_ALL, LC_MESSAGES e LANG
    significa inglês, seja qual for o valor de LANGUAGE.

## After the options of dmm-cli completions --help. The command line for
## each shell follows.

help-completions-install = Instale o preenchimento automático para o seu shell:
