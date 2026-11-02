### What 'dmm-cli command' prints: the commands a meter takes, or that one
### was sent. Command words (hold, range, select2…) are typed by the user and
### stay as they are.

# Heads the list of command words, one per line under it, on the standard
# output. $meter is the meter's brand and model, e.g. UNI-T UT181A, in bold.
command-available = { $meter } で使用できるコマンド:
# After "Note:" (common.ftl): the meter takes no commands from dmm-cli.
# $meter is its brand and model, e.g. UNI-T UT803.
command-none = { $meter } のリモート コマンドはまだ実装されていません。
# Green, opening the line that confirms a command went to the meter; the
# command word sent follows after a space.
command-sent-label = 送信:
