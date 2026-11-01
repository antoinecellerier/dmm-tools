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
args-error = error:

## What went wrong.

# An option that takes no value after "=" was given without one.
args-equals-needed = se necesita un signo igual para asignar valores a '{ $arg }'
# An option was given without its value.
args-value-required = '{ $arg }' necesita un valor, pero no se indicó ninguno
# A value the option doesn't take. A list of the ones it takes may follow
# on the next line (args-possible-values).
args-invalid-value = valor '{ $value }' no válido para '{ $arg }'
# The same, with the reason the value was refused. $reason may be English.
args-invalid-value-because = valor '{ $value }' no válido para '{ $arg }': { $reason }
# The values an option takes, on a line of their own under the error. $values
# is the list, separated by commas.
args-possible-values = [valores posibles: { $values }]
# A command dmm-cli doesn't have, e.g. 'lst'.
args-unrecognized-subcommand = subcomando '{ $command }' no reconocido
# Options that must be given, listed one per line under this.
args-missing-required = no se indicaron los siguientes argumentos obligatorios:
# dmm-cli was run without a command. $command is the program, dmm-cli. The
# commands follow on the next line (args-subcommands).
args-missing-subcommand = '{ $command }' necesita un subcomando, pero no se indicó ninguno
# The commands, on a line of their own under the error. $values is the list,
# separated by commas.
args-subcommands = [subcomandos: { $values }]
# A value after an option that takes no more.
args-unexpected-value = valor '{ $value }' inesperado para '{ $arg }'; no se esperaban más
# Too few values after an option. $needed and $given are numbers; $count is
# $given, for choosing the verb's form ([0] and [one]: one value or none).
args-too-few-values =
    { $count ->
        [0] '{ $arg }' necesita { $needed } valores; solo se indicaron { $given }
        [one] '{ $arg }' necesita { $needed } valores; solo se indicó { $given }
       *[other] '{ $arg }' necesita { $needed } valores; solo se indicaron { $given }
    }
# The wrong number of values after an option. $needed and $given are
# numbers; $count is $given, for choosing the verb's form.
args-wrong-number-of-values =
    { $count ->
        [0] '{ $arg }' necesita { $needed } valores, pero se indicaron { $given }
        [one] '{ $arg }' necesita { $needed } valores, pero se indicó { $given }
       *[other] '{ $arg }' necesita { $needed } valores, pero se indicaron { $given }
    }
# An option dmm-cli doesn't have, or one this command doesn't take.
args-unexpected-argument = argumento '{ $arg }' inesperado
# An option given twice that can only be given once.
args-repeated = el argumento '{ $arg }' no se puede usar más de una vez
# Two options, or an option and a command, that can't go together. $what is
# argument or subcommand; $name and $other are what the user typed.
args-conflict-with =
    { $what ->
        [subcommand] el subcomando '{ $name }' no se puede usar con '{ $other }'
       *[argument] el argumento '{ $name }' no se puede usar con '{ $other }'
    }
# Two options, or an option and a command, that can't go together, with
# the others listed one per line under this. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-list =
    { $what ->
        [subcommand] el subcomando '{ $name }' no se puede usar con:
       *[argument] el argumento '{ $name }' no se puede usar con:
    }
# The same, when the others aren't named. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-others =
    { $what ->
        [subcommand] el subcomando '{ $name }' no se puede usar con uno o más de los otros argumentos indicados
       *[argument] el argumento '{ $name }' no se puede usar con uno o más de los otros argumentos indicados
    }

## Hints after the error.

# Before a hint, indented on its own line.
args-tip = sugerencia:
# A name close to what was typed. $what is subcommand, argument or value;
# $names is one or more quoted names, separated by commas; $count is how
# many.
args-similar =
    { $what ->
        [subcommand]
            { $count ->
                [one] existe un subcomando parecido: { $names }
               *[other] existen subcomandos parecidos: { $names }
            }
        [argument]
            { $count ->
                [one] existe un argumento parecido: { $names }
               *[other] existen argumentos parecidos: { $names }
            }
       *[value]
            { $count ->
                [one] existe un valor parecido: { $names }
               *[other] existen valores parecidos: { $names }
            }
    }
# The last line. $help is --help.
args-more-info = Para obtener más información, prueba '{ $help }'.
