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
args-equals-needed = equal sign is needed when assigning values to '{ $arg }'
# An option was given without its value.
args-value-required = a value is required for '{ $arg }' but none was supplied
# A value the option doesn't take. A list of the ones it takes may follow
# on the next line (args-possible-values).
args-invalid-value = invalid value '{ $value }' for '{ $arg }'
# The same, with the reason the value was refused. $reason may be English.
args-invalid-value-because = invalid value '{ $value }' for '{ $arg }': { $reason }
# The values an option takes, on a line of their own under the error. $values
# is the list, separated by commas.
args-possible-values = [possible values: { $values }]
# A command dmm-cli doesn't have, e.g. 'lst'.
args-unrecognized-subcommand = unrecognized subcommand '{ $command }'
# Options that must be given, listed one per line under this.
args-missing-required = the following required arguments were not provided:
# dmm-cli was run without a command. $command is the program, dmm-cli. The
# commands follow on the next line (args-subcommands).
args-missing-subcommand = '{ $command }' requires a subcommand but one was not provided
# The commands, on a line of their own under the error. $values is the list,
# separated by commas.
args-subcommands = [subcommands: { $values }]
# A value after an option that takes no more.
args-unexpected-value = unexpected value '{ $value }' for '{ $arg }' found; no more were expected
# Too few values after an option. $needed and $given are numbers; $count is
# $given, for choosing the verb's form ([0] and [one]: one value or none).
args-too-few-values =
    { $count ->
        [0] { $needed } values required by '{ $arg }'; only { $given } was provided
        [one] { $needed } values required by '{ $arg }'; only { $given } was provided
       *[other] { $needed } values required by '{ $arg }'; only { $given } were provided
    }
# The wrong number of values after an option. $needed and $given are
# numbers; $count is $given, for choosing the verb's form.
args-wrong-number-of-values =
    { $count ->
        [0] { $needed } values required for '{ $arg }' but { $given } was provided
        [one] { $needed } values required for '{ $arg }' but { $given } was provided
       *[other] { $needed } values required for '{ $arg }' but { $given } were provided
    }
# An option dmm-cli doesn't have, or one this command doesn't take.
args-unexpected-argument = unexpected argument '{ $arg }' found
# An option given twice that can only be given once.
args-repeated = the argument '{ $arg }' cannot be used multiple times
# Two options, or an option and a command, that can't go together. $what is
# argument or subcommand; $name and $other are what the user typed.
args-conflict-with =
    { $what ->
        [subcommand] the subcommand '{ $name }' cannot be used with '{ $other }'
       *[argument] the argument '{ $name }' cannot be used with '{ $other }'
    }
# Two options, or an option and a command, that can't go together, with
# the others listed one per line under this. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-list =
    { $what ->
        [subcommand] the subcommand '{ $name }' cannot be used with:
       *[argument] the argument '{ $name }' cannot be used with:
    }
# The same, when the others aren't named. $what is argument or
# subcommand; $name is what the user typed.
args-conflict-with-others =
    { $what ->
        [subcommand] the subcommand '{ $name }' cannot be used with one or more of the other specified arguments
       *[argument] the argument '{ $name }' cannot be used with one or more of the other specified arguments
    }

## Hints after the error.

# Before a hint, indented on its own line.
args-tip = tip:
# A name close to what was typed. $what is subcommand, argument or value;
# $names is one or more quoted names, separated by commas; $count is how
# many.
args-similar =
    { $what ->
        [subcommand]
            { $count ->
                [one] a similar subcommand exists: { $names }
               *[other] some similar subcommands exist: { $names }
            }
        [argument]
            { $count ->
                [one] a similar argument exists: { $names }
               *[other] some similar arguments exist: { $names }
            }
       *[value]
            { $count ->
                [one] a similar value exists: { $names }
               *[other] some similar values exist: { $names }
            }
    }
# The last line. $help is --help.
args-more-info = For more information, try '{ $help }'.
