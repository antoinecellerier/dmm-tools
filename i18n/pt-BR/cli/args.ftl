### Mistakes on the command line: an unknown option, a missing command, a
### value dmm-cli doesn't take. Printed on the terminal as
###
###     error: invalid value 'xml' for '--format <FORMAT>'
###       [possible values: csv, json, replay]
###
###     Usage: dmm-cli read [OPTIONS]
###
###     For more information, try '--help'.
###
### The usage line stays English, like the rest of --help's layout. Names
### in quotes ($arg, $value, $command, $names) are what the user typed or
### must type: they arrive as they are, sometimes coloured, and stay in the
### quotes. $arg is an option, often with its value's name, e.g.
### --format <FORMAT>.

## The start of the line.

# Before every message, as the terminal shows errors.
args-error = erro:

## What went wrong.

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
# A command dmm-cli doesn't have, e.g. 'lst'.
args-unrecognized-subcommand = subcomando '{ $command }' não reconhecido
# Options that must be given, listed one per line under this.
args-missing-required = os seguintes argumentos obrigatórios não foram fornecidos:
# dmm-cli was run without a command. $command is the program, dmm-cli. The
# commands follow on the next line (args-subcommands).
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
# An option dmm-cli doesn't have, or one this command doesn't take.
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
