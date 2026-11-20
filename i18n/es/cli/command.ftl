### What 'dmm-cli command' prints: the commands a meter takes, or that one
### was sent. Command words (hold, range, select2…) are typed by the user and
### stay as they are.

# Heads the list of command words, one per line under it, on the standard
# output. $meter is the meter's brand and model, e.g. UNI-T UT181A, in bold.
command-available = Comandos disponibles para el multímetro { $meter }:
# After "Note:" (common.ftl): the meter takes no commands from dmm-cli.
# $meter is its brand and model, e.g. UNI-T UT803.
command-none = Todavía no hay comandos remotos implementados para el multímetro { $meter }.
# After "Error:" (common.ftl): the meter has no command by that name, and
# the ones it takes. $meter is its brand and model, e.g. UNI-T UT61E+;
# $command the word as typed; $commands its command words, comma-separated.
command-unknown = el multímetro { $meter } no tiene el comando '{ $command }'; sus comandos son { $commands }
# The same for a meter dmm-cli sends no commands to; $meter as above.
command-unknown-none = todavía no hay comandos remotos para el multímetro { $meter }
# Green, opening the line that confirms a command went to the meter; the
# command word sent follows after a space.
command-sent-label = Enviado
