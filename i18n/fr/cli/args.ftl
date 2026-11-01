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
args-error = erreur :

## What went wrong.

# An option that takes no value after "=" was given without one.
args-equals-needed = un signe égal est nécessaire pour donner une valeur à '{ $arg }'
# An option was given without its value.
args-value-required = '{ $arg }' attend une valeur, mais aucune n’a été donnée
# A value the option doesn't take. A list of the ones it takes may follow
# on the next line (args-possible-values).
args-invalid-value = valeur '{ $value }' non valide pour '{ $arg }'
# The same, with the reason the value was refused. $reason may be English.
args-invalid-value-because = valeur '{ $value }' non valide pour '{ $arg }' : { $reason }
# The values an option takes, on a line of their own under the error. $values
# is the list, separated by commas.
args-possible-values = [valeurs possibles : { $values }]
# A command dmm-cli doesn't have, e.g. 'lst'.
args-unrecognized-subcommand = commande '{ $command }' inconnue
# Options that must be given, listed one per line under this.
args-missing-required = les arguments obligatoires suivants n’ont pas été donnés :
# dmm-cli was run without a command. $command is the program, dmm-cli. The
# commands follow on the next line (args-subcommands).
args-missing-subcommand = '{ $command }' attend une commande, mais aucune n’a été donnée
# The commands, on a line of their own under the error. $values is the list,
# separated by commas.
args-subcommands = [commandes : { $values }]
# A value after an option that takes no more.
args-unexpected-value = valeur '{ $value }' inattendue pour '{ $arg }' : aucune autre n’était attendue
# Too few values after an option. $needed and $given are numbers; $count is
# $given, for choosing the verb's form ([0] and [one]: one value or none).
args-too-few-values =
    { $count ->
        [one] '{ $arg }' attend { $needed } valeurs, mais seulement { $given } a été donnée
       *[other] '{ $arg }' attend { $needed } valeurs, mais seulement { $given } ont été données
    }
# The wrong number of values after an option. $needed and $given are
# numbers; $count is $given, for choosing the verb's form.
args-wrong-number-of-values =
    { $count ->
        [one] '{ $arg }' attend { $needed } valeurs, mais { $given } a été donnée
       *[other] '{ $arg }' attend { $needed } valeurs, mais { $given } ont été données
    }
# An option dmm-cli doesn't have, or one this command doesn't take.
args-unexpected-argument = argument '{ $arg }' inattendu
# An option given twice that can only be given once.
args-repeated = l’argument '{ $arg }' ne peut être donné qu’une fois
# Two options, or an option and a command, that can't go together. $what is
# argument or subcommand; $name and $other are what the user typed.
args-conflict-with =
    { $what ->
        [subcommand] la commande '{ $name }' ne peut pas être utilisée avec '{ $other }'
       *[argument] l’argument '{ $name }' ne peut pas être utilisé avec '{ $other }'
    }
# Two options, or an option and a command, that can't go together, with
# the others listed one per line under this. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-list =
    { $what ->
        [subcommand] la commande '{ $name }' ne peut pas être utilisée avec :
       *[argument] l’argument '{ $name }' ne peut pas être utilisé avec :
    }
# The same, when the others aren't named. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-others =
    { $what ->
        [subcommand] la commande '{ $name }' ne peut pas être utilisée avec un ou plusieurs des autres arguments donnés
       *[argument] l’argument '{ $name }' ne peut pas être utilisé avec un ou plusieurs des autres arguments donnés
    }

## Hints after the error.

# Before a hint, indented on its own line.
args-tip = astuce :
# A name close to what was typed. $what is subcommand, argument or value;
# $names is one or more quoted names, separated by commas; $count is how
# many.
args-similar =
    { $what ->
        [subcommand]
            { $count ->
                [one] une commande proche existe : { $names }
               *[other] des commandes proches existent : { $names }
            }
        [argument]
            { $count ->
                [one] un argument proche existe : { $names }
               *[other] des arguments proches existent : { $names }
            }
       *[value]
            { $count ->
                [one] une valeur proche existe : { $names }
               *[other] des valeurs proches existent : { $names }
            }
    }
# The last line. $help is --help.
args-more-info = Pour en savoir plus, essayez '{ $help }'.
