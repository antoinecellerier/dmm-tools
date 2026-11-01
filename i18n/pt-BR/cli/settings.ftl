### What 'dmm-cli get' and 'dmm-cli set' print. Setting names (mode, range,
### hold, rel, minmax, peak) and the choices a listing shows, with what to
### type beside each, are what the user types back into 'dmm-cli set': they
### stay as they are, as does the JSON output. HOLD, REL and MIN/MAX are the
### meter's button names, printed on the meter, and stay too.

## The line heading a listing on the standard output; a line per setting or
## per choice follows. $meter is the meter's brand and model, e.g. UNI-T
## UT61E+, in bold; $mode what it is measuring, e.g. AC V.

# 'get' with no setting: one row per setting the meter can switch.
settings-all-heading = Configurações do multímetro { $meter } ({ $mode }):
# One setting's choices. $setting is that setting: mode, range, hold, rel,
# minmax or peak. The modes offered don't depend on the mode, so that
# heading names none.
settings-heading =
    { $setting ->
        [mode] Modos do multímetro { $meter }:
        [range] Faixa do multímetro { $meter } no modo { $mode }:
        [hold] HOLD do multímetro { $meter } no modo { $mode }:
        [rel] REL do multímetro { $meter } no modo { $mode }:
        [minmax] MIN/MAX do multímetro { $meter } no modo { $mode }:
       *[peak] Pico do multímetro { $meter } no modo { $mode }:
    }

## Dim, at the end of the range row while the meter autoranges: the range it
## picked, $range, e.g. 22V.

# In one setting's listing, on the Auto row.
settings-auto-now = (agora { $range })
# In the listing of every setting.
settings-auto-in = (faixa automática: { $range })

## Dim, under a listing: a switch to try. $command is the command line that
## makes it, e.g. dmm-cli set mode hz.

# Under the listing of every setting.
settings-tip-by-name = Dica: altere uma configuração pelo nome, por exemplo { $command }
# Under one setting's listing, whose right column holds what to type for
# each choice.
settings-tip-right-column = Dica: digite o valor da coluna da direita para mudar para essa opção, por exemplo { $command }

## After "Note:" (common.ftl): the meter has nothing to switch to from where
## it is, and the command stops. Only the dial changes which modes and ranges
## are offered.

# $meter is the meter's brand and model, e.g. UNI-T UT61E+; $mode what it is
# measuring, e.g. NCV. $setting is the setting asked about: mode, range,
# hold, rel, minmax or peak, or all for every setting.
settings-nothing-to-switch =
    { $setting ->
        [mode] No modo { $mode }, o multímetro { $meter } não tem outros modos selecionáveis: use a chave seletora.
        [range] O multímetro { $meter } não tem faixas para alternar no modo { $mode }: use a chave seletora.
        [hold] O multímetro { $meter } não permite alternar HOLD no modo { $mode }.
        [rel] O multímetro { $meter } não permite alternar REL no modo { $mode }.
        [minmax] O multímetro { $meter } não permite alternar MIN/MAX no modo { $mode }.
        [peak] O multímetro { $meter } não permite alternar a função de pico no modo { $mode }.
       *[all] O multímetro { $meter } não tem configurações para alternar no modo { $mode }: use a chave seletora.
    }

## In green on the standard output, once 'set' sees the meter on the choice.
## $choice is that choice, e.g. AC V Hz, 22V, on or MAX; $range the range
## autoranging picked, e.g. 22V. $case is the setting and how it stands: mode,
## range-auto (the meter picks the range), range (a range picked by hand),
## hold or rel (with $choice on or off), minmax or peak (a state, e.g. MAX or
## P-MIN), minmax-off or peak-off (the meter left that state).

# The meter switched.
settings-switched =
    { $case ->
       *[mode] O multímetro está agora no modo { $choice }
        [range-auto] O multímetro está agora em faixa automática ({ $range })
        [range] O multímetro está agora na faixa { $choice } (faixa manual)
        [hold] O multímetro está agora com HOLD { $choice }
        [rel] O multímetro está agora com REL { $choice }
        [minmax] O multímetro está agora em { $choice }
        [minmax-off] O multímetro saiu do MIN/MAX
        [peak] O multímetro está agora em { $choice }
        [peak-off] O multímetro saiu da função de pico
    }
# The meter was on it already, so nothing was sent.
settings-already =
    { $case ->
       *[mode] O multímetro já está no modo { $choice }
        [range-auto] O multímetro já está em faixa automática ({ $range })
        [range] O multímetro já está na faixa { $choice } (faixa manual)
        [hold] O multímetro já está com HOLD { $choice }
        [rel] O multímetro já está com REL { $choice }
        [minmax] O multímetro já está em { $choice }
        [minmax-off] O multímetro já está fora do MIN/MAX
        [peak] O multímetro já está em { $choice }
        [peak-off] O multímetro já está fora da função de pico
    }

## Why 'set' failed, after "Error:" (common.ftl). $setting is the setting as
## typed: mode, range, hold, rel, minmax or peak.

# $input is the choice as typed, e.g. 2; $choices the choices it matches, as
# the listing spells them, separated by commas, e.g. "2.2V, 22V, 220V".
settings-ambiguous = valor ambíguo para { $setting }: { $input } corresponde a { $choices }
# No choice matches $input, the choice as typed; the listing is printed above.
settings-unknown = valor desconhecido para { $setting }: { $input }
# The meter answered that it wouldn't switch. $choice is the choice, e.g.
# 22V; $reason the meter's reason (lib-errors.ftl). A range is also refused
# when the input is past its end.
settings-refused =
    { $setting ->
        [range] o multímetro recusou { $choice }: { $reason } — verifique a posição da chave seletora e se a entrada está dentro da faixa
       *[other] o multímetro recusou { $choice }: { $reason } — verifique a posição da chave seletora
    }
# The meter took the command, but its readings never showed the choice.
settings-not-switched =
    { $setting ->
        [range] O multímetro não mudou: verifique a posição da chave seletora e se a entrada está dentro da faixa
       *[other] O multímetro não mudou: verifique a posição da chave seletora
    }
# As settings-not-switched, naming $live, the choice the meter is still on,
# e.g. off.
settings-not-switched-still =
    { $setting ->
        [range] O multímetro não mudou (continua em { $live }): verifique a posição da chave seletora e se a entrada está dentro da faixa
       *[other] O multímetro não mudou (continua em { $live }): verifique a posição da chave seletora
    }
