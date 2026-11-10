### Text both dmm-cli and dmm-gui show: the shared parts of their --help
### and the words clap puts around it, their command-line errors, the
### setup help when no meter is found, and the notices both print.
###
### Command lines, file paths and web addresses are never part of a message.
### Words in --flag form, and 'dmm-cli list', are typed by the user and stay
### as they are.

## The project page link: the last line of both binaries' --help, and the
## link in the GUI's top bar.

project-link = Ajuda e sugestões
# $url is the project page's address.
project-link-line = { project-link }: { $url }

## The --device flag's long help, after the binary's own first line. The
## device list between the heading and the aliases is generated.

device-list-heading = Dispositivos:
# What the "auto" device does. Also its hover in the GUI's device list.
device-auto = Detectar o multímetro conectado
# The "auto" row of the list: what the flag does when it isn't given.
device-auto-default = { device-auto } (padrão)
device-aliases =
    Também aceita nomes alternativos: ut61e+, ut61b, ut171a, ut181 etc.
    Coloque entre aspas os nomes com caracteres especiais: --device 'ut61e+'
# Beside the simulated meters, in device lists.
device-no-hardware = não requer hardware

## How far a meter's support is verified, beside it in device lists.

stability-verified = verificado
stability-partly-verified = parcialmente verificado
stability-experimental = experimental
# Both binaries show it for a meter short of verified, each followed by its
# own way to help verify it.
# $model is the meter's name, e.g. "UNI-T UT8803".
# $stability is verified, partly-verified or experimental.
stability-warning =
    { $stability ->
        [verified] O suporte ao multímetro { $model } foi verificado em hardware real.
        [partly-verified] O suporte ao multímetro { $model } está parcialmente verificado (alguns modos e comandos não foram confirmados em hardware real).
       *[experimental] O suporte ao multímetro { $model } é experimental (não verificado em hardware real).
    }

## The --mock-mode flag's long help, after the binary's own first line.

# $modes lists the mock's modes, separated by commas.
mock-mode-list = Modos: { $modes }
# $example is a command line.
mock-mode-example = Exemplo: { $example }
mock-clock-mock-only = --mock-clock-scale e --mock-clock-preseed só se aplicam ao multímetro simulado e à reprodução. Execute de novo com --device mock ou --replay, ou sem as opções de relógio.

## A link's short name, where the meter is already named: the GUI's
## status line, the setup help's section labels.

link-usb-cable = Cabo USB
link-bluetooth = Bluetooth

## The help shown when no meter was found: a title, then one section per
## link searched — a label line, then steps. The CLI prints them in a
## terminal, the GUI in a notice.

# No full stop: the CLI adds one, the GUI's notice titles carry none.
not-found-usb = Nenhum cabo USB encontrado
not-found-usb-or-bluetooth = Nenhum multímetro encontrado via USB ou Bluetooth
# $address is a Bluetooth address or peripheral id.
not-found-bluetooth-at = Nenhum dispositivo Bluetooth encontrado no endereço { $address }
# $model is the meter's name, e.g. "UNI-T UT60BT".
not-found-bluetooth-only = Nenhum multímetro { $model } encontrado via Bluetooth
# A section's label line.
# $link is link-usb-cable or link-bluetooth.
# $check is setup-usb-check or setup-bluetooth-check.
setup-label = { $link }: { $check }
setup-usb-check = verifique se está conectado e se o multímetro está ligado.
setup-bluetooth-check = ligue o Bluetooth neste computador e a transmissão de dados no multímetro.
# Linux. Two command lines follow.
setup-usb-linux-rule = Verifique se a regra udev está instalada:
# Linux. The setup guide's address follows.
setup-usb-linux-replug = Depois, reconecte o cabo. Em uma máquina sem interface gráfica, mantenha um grupo na regra:
# Windows. The quoted names are what Device Manager shows; use the names
# Windows shows in your language.
setup-usb-windows-open = Abra o Gerenciador de Dispositivos com o cabo conectado:
setup-usb-windows-cp2110 = - 'CP2110 USB to UART Bridge' em Dispositivos de Interface Humana: nenhuma ação necessária.
# UNVERIFIED Windows name
setup-usb-windows-input = - 'Dispositivo de Entrada USB' em Dispositivos de Interface Humana: nenhuma ação necessária.
# The driver's download address follows.
setup-usb-windows-warning = - Ícone de aviso amarelo em 'Outros dispositivos': instale o driver disponível em
setup-usb-windows-nothing = - Nada aparece: tente outra porta USB.
# macOS. Name the settings as macOS shows them in your language.
setup-usb-macos-auto = O cabo deve ser reconhecido automaticamente (não é preciso instalar driver).
setup-usb-macos-privacy = Se o dispositivo não for encontrado, verifique Ajustes do Sistema > Privacidade e Segurança > Monitoração de Entrada.
setup-bluetooth-adapter = Se ambos estiverem ligados, desligue e ligue de novo o adaptador.
setup-bluetooth-macos-privacy = Permita o Bluetooth em Ajustes do Sistema > Privacidade e Segurança > Bluetooth.
# The device catalog's address follows.
setup-bluetooth-catalog = Os adaptadores e multímetros compatíveis estão listados no catálogo de dispositivos:
setup-bluetooth-scan = Execute 'dmm-cli list' para procurar adaptadores e multímetros ao alcance.

## Notices both binaries show.

# A cable or Bluetooth link is open but no meter on it identified itself.
# A notice title in the GUI; dmm-cli starts a sentence with it, naming the
# link after it (cli/connect.ftl).
no-meter-answered = Nenhum multímetro respondeu
# The meter picked or found has stopped answering. A notice title in the
# GUI; dmm-cli starts a sentence with it (cli/connect.ftl).
no-response = Sem resposta do multímetro
# Why a dropped link is not reconnected. $count (2 or more) USB cables the
# open could take are plugged in, with no serial number of their own to tell
# them apart. Each binary follows it with what to do.
several-cables =
    { $count ->
        [one] Há { $count } cabo USB conectado que esta sessão pode abrir, e não é possível identificá-lo. Uma reconexão poderia abrir o cabo de outro multímetro.
       *[other] Há { $count } cabos USB conectados que esta sessão pode abrir, e não é possível distingui-los. Uma reconexão poderia abrir o cabo de outro multímetro.
    }
# While a dropped link is being reopened: its cable has gone and another is
# plugged in. Each binary follows it with how to use that other cable.
waiting-for-own-cable = Outro cabo USB está conectado, mas esta sessão aguarda o cabo que abriu.
# Under a reading the meter shows on its own screen only, its frames carrying
# none: a CMS or UT217's oscilloscope mode.
screen-only-reading = Veja as leituras do osciloscópio no display do multímetro
# The GUI checkbox that lets a connection search over Bluetooth. The CLI
# reads the same setting, so its help names the checkbox too.
bluetooth-setting = Procurar dispositivos Bluetooth
bluetooth-off-gui = Marque “{ bluetooth-setting }” em Configurações (⚙).

## What is connected, when an --adapter value matched nothing.

adapters-none = Nenhum dispositivo conectado no momento.
adapters-heading = Dispositivos conectados:
adapters-unavailable = Execute 'dmm-cli list' para ver os dispositivos conectados.
# $address is the value the user typed, written with colons, e.g. 12:34:56:78:9A:BC.
colonless-address-hint = Se for um endereço Bluetooth, escreva { $address }.

## Why a number typed for a scale, an offset or an alarm was refused: the
## reason alone, in lower case and with no full stop. dmm-gui puts it after
## the field's name in a toast ("Invalid scale: …"); dmm-cli after the
## option and the value, at the end of a line from its command-line parser,
## which stays English ("error: invalid value '0' for '--scale <FACTOR>': …").

# $text is what was typed, as it was typed.
input-not-a-number = “{ $text }” não é um número
# Infinity or "not a number" (NaN).
input-not-finite = deve ser um número finito
# A scale factor of 0 would turn every reading into the offset.
input-zero-scale = um fator de escala igual a zero apagaria a leitura
# The alarm's low limit and its high limit.
input-low-not-below-high = o limite inferior deve ser menor que o superior
# The alarm's hysteresis band: a number in the limits' unit, or a percentage
# of the limit. $text is what was typed, as it was typed.
input-band-not-a-number = “{ $text }” não é um número nem uma porcentagem como 1%
# A hysteresis band below zero.
input-band-negative = deve ser maior ou igual a zero
# A percentage band with a limit of 0, of which every percentage is 0.
input-percent-of-zero = qualquer porcentagem de um limite igual a 0 é 0; informe um valor

## Why an exported file or a recording did not read: in dmm-gui's toast
## after "Import failed:" and the file, in dmm-cli after "Error:" and the
## file. Lower case, with no full stop. The backquotes (`) set off text
## from the file, or a name it must have, which stays as it is: keep them.
## A parser's own words ($detail) arrive in English.

# Where the file stopped: $line is a line number, $reason one of the
# reasons below.
read-error-at-line = linha { $line }: { $reason }
read-error-empty = o arquivo está vazio
# A CSV export without its first row naming the columns.
read-error-no-header-row = sem linha de cabeçalho ({ $detail })
# $column is the name of a column every CSV export has, e.g. timestamp.
read-error-no-column = sem a coluna `{ $column }`
# A JSON export whose first line is not the one starting {"_metadata":.
read-error-no-metadata = sem a linha `_metadata` ({ $detail })
# $text is what the file has where a reading's value should be.
read-error-not-a-value = `{ $text }` não é um valor
# $text is a cell starting NCV: whose level is no number. NCV is
# non-contact voltage detection, as the meter prints it.
read-error-not-an-ncv-level = `{ $text }` não é um nível de NCV
# $text is the part of a flags cell that names no status flag.
read-error-not-a-flag = `{ $text }` não é um indicador
# $text is what the file has where a marker's number should be.
read-error-not-a-marker-number = `{ $text }` não é um número de marcador
# $text is a timestamp from the file, or a recording's "# recorded:" line;
# RFC 3339 is the standard date format the files use.
read-error-not-a-date = `{ $text }` não é uma data RFC 3339: { $detail }

## The words clap puts around every page of dmm-cli's and dmm-gui's --help:
## its headings, what follows an option, and the help of the built-in -h,
## -V and help. They are the same on every page.

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

## Mistakes on dmm-cli's and dmm-gui's command line: an unknown option, a
## missing command, a value the program doesn't take. Printed on the
## terminal as
##
##     error: invalid value 'xml' for '--format <FORMAT>'
##       [possible values: csv, json, replay]
##
##     Usage: dmm-cli read [OPTIONS]
##
##     For more information, try '--help'.
##
## The usage line's syntax stays English after its heading. Names in quotes
## ($arg, $value, $command, $names) are what the user typed or must type:
## they arrive as they are, sometimes coloured, and stay in the quotes. $arg
## is an option, often with its value's name, e.g. --format <FORMAT>.

# At the start of every message, as the terminal shows errors.
args-error = erro:

# An option that takes no value after "=" was given without one.
args-equals-needed = é necessário um sinal de igual para atribuir valores a '{ $arg }'
# An option was given without its value.
args-value-required = '{ $arg }' requer um valor, mas nenhum foi fornecido
# A value the option doesn't take. A list of the ones it takes may follow
# on the next line (args-possible-values).
args-invalid-value = valor '{ $value }' inválido para '{ $arg }'
# The same, with the reason the value was refused. $reason may be English.
args-invalid-value-because = valor '{ $value }' inválido para '{ $arg }': { $reason }
# The values an option takes, on a line of their own under the error. $values
# is the list, separated by commas.
args-possible-values = [valores possíveis: { $values }]
# A command the program doesn't have, e.g. 'lst'.
args-unrecognized-subcommand = subcomando '{ $command }' não reconhecido
# Options that must be given, listed one per line under this.
args-missing-required = os seguintes argumentos obrigatórios não foram fornecidos:
# The program was run without a command. $command is the program, e.g.
# dmm-cli. The commands follow on the next line (args-subcommands).
args-missing-subcommand = '{ $command }' requer um subcomando, mas nenhum foi fornecido
# The commands, on a line of their own under the error. $values is the list,
# separated by commas.
args-subcommands = [subcomandos: { $values }]
# A value after an option that takes no more.
args-unexpected-value = valor '{ $value }' inesperado para '{ $arg }'; nenhum outro era esperado
# Too few values after an option. $needed and $given are numbers; $count is
# $given, for choosing the verb's form ([0] and [one]: one value or none).
args-too-few-values =
    { $count ->
        [0] '{ $arg }' requer { $needed } valores; nenhum foi fornecido
        [one] '{ $arg }' requer { $needed } valores; apenas { $given } foi fornecido
       *[other] '{ $arg }' requer { $needed } valores; apenas { $given } foram fornecidos
    }
# The wrong number of values after an option. $needed and $given are
# numbers; $count is $given, for choosing the verb's form.
args-wrong-number-of-values =
    { $count ->
        [0] '{ $arg }' requer { $needed } valores, mas nenhum foi fornecido
        [one] '{ $arg }' requer { $needed } valores, mas { $given } foi fornecido
       *[other] '{ $arg }' requer { $needed } valores, mas { $given } foram fornecidos
    }
# An option the program doesn't have, or one this command doesn't take.
args-unexpected-argument = argumento '{ $arg }' inesperado
# An option given twice that can only be given once.
args-repeated = o argumento '{ $arg }' não pode ser usado mais de uma vez
# Two options, or an option and a command, that can't go together. $what is
# argument or subcommand; $name and $other are what the user typed.
args-conflict-with =
    { $what ->
        [subcommand] o subcomando '{ $name }' não pode ser usado com '{ $other }'
       *[argument] o argumento '{ $name }' não pode ser usado com '{ $other }'
    }
# Two options, or an option and a command, that can't go together, with
# the others listed one per line under this. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-list =
    { $what ->
        [subcommand] o subcomando '{ $name }' não pode ser usado com:
       *[argument] o argumento '{ $name }' não pode ser usado com:
    }
# The same, when the others aren't named. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-others =
    { $what ->
        [subcommand] o subcomando '{ $name }' não pode ser usado com um ou mais dos outros argumentos informados
       *[argument] o argumento '{ $name }' não pode ser usado com um ou mais dos outros argumentos informados
    }

## Hints after the error.

# Before a hint, indented on its own line.
args-tip = dica:
# A name close to what was typed. $what is subcommand, argument or value;
# $names is one or more quoted names, separated by commas; $count is how
# many.
args-similar =
    { $what ->
        [subcommand]
            { $count ->
                [one] existe um subcomando parecido: { $names }
               *[other] existem subcomandos parecidos: { $names }
            }
        [argument]
            { $count ->
                [one] existe um argumento parecido: { $names }
               *[other] existem argumentos parecidos: { $names }
            }
       *[value]
            { $count ->
                [one] existe um valor parecido: { $names }
               *[other] existem valores parecidos: { $names }
            }
    }
# The last line. $help is --help.
args-more-info = Para mais informações, use '{ $help }'.
