### What 'dmm-cli command' prints: the commands a meter takes, or that one
### was sent. Command words (hold, range, select2…) are typed by the user and
### stay as they are.

# Heads the list of command words, one per line under it, on the standard
# output. $meter is the meter's brand and model, e.g. UNI-T UT181A, in bold.
command-available = Comandos disponíveis para o multímetro { $meter }:
# After "Note:" (common.ftl): the meter takes no commands from dmm-cli.
# $meter is its brand and model, e.g. UNI-T UT803.
command-none = Ainda não há comandos remotos implementados para o multímetro { $meter }.
# After "Error:" (common.ftl): the meter has no command by that name, and
# the ones it takes. $meter is its brand and model, e.g. UNI-T UT61E+;
# $command the word as typed; $commands its command words, comma-separated.
command-unknown = o multímetro { $meter } não tem o comando '{ $command }'; os comandos aceitos são { $commands }
# The same for a meter dmm-cli sends no commands to; $meter as above.
command-unknown-none = o multímetro { $meter } ainda não aceita comandos remotos
# Green, opening the line that confirms a command went to the meter; the
# command word sent follows after a space.
command-sent-label = Enviado
