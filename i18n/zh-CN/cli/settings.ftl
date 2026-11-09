### What 'dmm-cli get' and 'dmm-cli set' print. Setting names (mode, range,
### hold, rel, minmax, peak) and the choices a listing shows, with what to
### type beside each, are what the user types back into 'dmm-cli set': they
### stay as they are, as does the JSON output. HOLD, REL and MIN/MAX are the
### meter's button names, printed on the meter, and stay too.

## The line heading a listing on the standard output; a line per setting or
## per choice follows. $meter is the meter's brand and model, e.g. UNI-T
## UT61E+, in bold; $mode what it is measuring, e.g. AC V.

# 'get' with no setting: one row per setting the meter can switch.
settings-all-heading = { $meter } ({ $mode }) 的设置:
# One setting's choices. $setting is that setting: mode, range, hold, rel,
# minmax or peak. The modes offered don't depend on the mode, so that
# heading names none.
settings-heading =
    { $setting ->
        [mode] { $meter } 的模式:
        [range] { $meter } 在“{ $mode }”下的量程:
        [hold] { $meter } 在“{ $mode }”下的 HOLD:
        [rel] { $meter } 在“{ $mode }”下的 REL:
        [minmax] { $meter } 在“{ $mode }”下的 MIN/MAX:
       *[peak] { $meter } 在“{ $mode }”下的峰值:
    }

## Dim, at the end of the range row while the meter autoranges: the range it
## picked, $range, e.g. 22V.

# In one setting's listing, on the Auto row.
settings-auto-now = (当前 { $range })
# In the listing of every setting.
settings-auto-in = (自动量程: { $range })

## Dim, under a listing: a switch to try. $command is the command line that
## makes it, e.g. dmm-cli set mode hz.

# Under the listing of every setting.
settings-tip-by-name = 提示: 按名称切换其中一项，例如 { $command }
# Under one setting's listing, whose right column holds what to type for
# each choice.
settings-tip-right-column = 提示: 右列的值用于切换到该项，例如 { $command }

## After "Note:" (common.ftl): the meter has nothing to switch to from where
## it is, and the command stops. Only the dial changes which modes and ranges
## are offered.

# $meter is the meter's brand and model, e.g. UNI-T UT61E+; $mode what it is
# measuring, e.g. NCV. $setting is the setting asked about: mode, range,
# hold, rel, minmax or peak, or all for every setting.
settings-nothing-to-switch =
    { $setting ->
        [mode] { $meter } 在“{ $mode }”下没有可切换的模式，请使用旋钮。
        [range] { $meter } 在“{ $mode }”下没有可切换的量程，请使用旋钮。
        [hold] { $meter } 在“{ $mode }”下没有可切换的 HOLD。
        [rel] { $meter } 在“{ $mode }”下没有可切换的 REL。
        [minmax] { $meter } 在“{ $mode }”下没有可切换的 MIN/MAX。
        [peak] { $meter } 在“{ $mode }”下没有可切换的峰值。
       *[all] { $meter } 在“{ $mode }”下没有可切换的设置，请使用旋钮。
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
       *[mode] 万用表已切换到 { $choice }
        [range-auto] 万用表已切换到自动量程 ({ $range })
        [range] 万用表已切换到 { $choice } (手动量程)
        [hold] 万用表 HOLD 已设为{ $choice }
        [rel] 万用表 REL 已设为{ $choice }
        [minmax] 万用表已进入 { $choice } 状态
        [minmax-off] 万用表已退出 MIN/MAX 状态
        [peak] 万用表已进入 { $choice } 状态
        [peak-off] 万用表已退出峰值模式
    }
# The meter was on it already, so nothing was sent.
settings-already =
    { $case ->
       *[mode] 万用表已处于 { $choice } 模式
        [range-auto] 万用表已处于自动量程 ({ $range })
        [range] 万用表已处于 { $choice } (手动量程)
        [hold] 万用表 HOLD 已是{ $choice }
        [rel] 万用表 REL 已是{ $choice }
        [minmax] 万用表已处于 { $choice } 状态
        [minmax-off] 万用表已不在 MIN/MAX 状态
        [peak] 万用表已处于 { $choice } 状态
        [peak-off] 万用表已不在峰值状态
    }

## Why 'set' failed, after "Error:" (common.ftl). $setting is the setting as
## typed: mode, range, hold, rel, minmax or peak.

# $input is the choice as typed, e.g. 2; $choices the choices it matches, as
# the listing spells them, separated by commas, e.g. "2.2V, 22V, 220V".
settings-ambiguous = { $setting } 不明确: { $input } 匹配 { $choices }
# No choice matches $input, the choice as typed; the listing is printed above.
settings-unknown = 未知的 { $setting }: { $input }
# The meter answered that it wouldn't switch. $choice is the choice, e.g.
# 22V; $reason the meter's reason (lib-errors.ftl). A range is also refused
# when the input is past its end.
settings-refused =
    { $setting ->
        [range] 万用表拒绝了 { $choice }: { $reason }，请检查旋钮位置，并确认输入在该量程内
       *[other] 万用表拒绝了 { $choice }: { $reason }，请检查旋钮位置
    }
# The meter took the command, but its readings never showed the choice.
settings-not-switched =
    { $setting ->
        [range] 万用表未切换，请检查旋钮位置，并确认输入在该量程内
       *[other] 万用表未切换，请检查旋钮位置
    }
# As settings-not-switched, naming $live, the choice the meter is still on,
# e.g. off.
settings-not-switched-still =
    { $setting ->
        [range] 万用表未切换 (仍为 { $live })，请检查旋钮位置，并确认输入在该量程内
       *[other] 万用表未切换 (仍为 { $live })，请检查旋钮位置
    }
