### What 'dmm-cli get' and 'dmm-cli set' print. Setting names (mode, range,
### hold, rel, minmax, peak) and the choices a listing shows, with what to
### type beside each, are what the user types back into 'dmm-cli set': they
### stay as they are, as does the JSON output. HOLD, REL and MIN/MAX are the
### meter's button names, printed on the meter, and stay too.

## The line heading a listing on the standard output; a line per setting or
## per choice follows. $meter is the meter's brand and model, e.g. UNI-T
## UT61E+, in bold; $mode what it is measuring, e.g. AC V.

# 'get' with no setting: one row per setting the meter can switch.
settings-all-heading = { $meter } の設定 ({ $mode }):
# One setting's choices. $setting is that setting: mode, range, hold, rel,
# minmax or peak. The modes offered don't depend on the mode, so that
# heading names none.
settings-heading =
    { $setting ->
        [mode] { $meter } のモード:
        [range] { $meter } のレンジ ({ $mode }):
        [hold] { $meter } の HOLD ({ $mode }):
        [rel] { $meter } の REL ({ $mode }):
        [minmax] { $meter } の MIN/MAX ({ $mode }):
       *[peak] { $meter } のピーク ({ $mode }):
    }

## Dim, at the end of the range row while the meter autoranges: the range it
## picked, $range, e.g. 22V.

# In one setting's listing, on the Auto row.
settings-auto-now = (現在 { $range })
# In the listing of every setting.
settings-auto-in = (オートレンジで { $range })

## Dim, under a listing: a switch to try. $command is the command line that
## makes it, e.g. dmm-cli set mode hz.

# Under the listing of every setting.
settings-tip-by-name = ヒント: 名前で切り替えられます。例: { $command }
# Under one setting's listing, whose right column holds what to type for
# each choice.
settings-tip-right-column = ヒント: 右の列の値で切り替えられます。例: { $command }

## After "Note:" (common.ftl): the meter has nothing to switch to from where
## it is, and the command stops. Only the dial changes which modes and ranges
## are offered.

# $meter is the meter's brand and model, e.g. UNI-T UT61E+; $mode what it is
# measuring, e.g. NCV. $setting is the setting asked about: mode, range,
# hold, rel, minmax or peak, or all for every setting.
settings-nothing-to-switch =
    { $setting ->
        [mode] { $meter } では、{ $mode } のとき切り替えられるモードはありません。ファンクションスイッチで切り替えてください。
        [range] { $meter } では、{ $mode } のとき切り替えられるレンジはありません。ファンクションスイッチで切り替えてください。
        [hold] { $meter } では、{ $mode } のとき HOLD を切り替えられません。
        [rel] { $meter } では、{ $mode } のとき REL を切り替えられません。
        [minmax] { $meter } では、{ $mode } のとき MIN/MAX を切り替えられません。
        [peak] { $meter } では、{ $mode } のときピークを切り替えられません。
       *[all] { $meter } では、{ $mode } のとき切り替えられる設定はありません。ファンクションスイッチで切り替えてください。
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
       *[mode] マルチメーターが { $choice } になりました
        [range-auto] マルチメーターがオートレンジになりました ({ $range })
        [range] マルチメーターが { $choice } (マニュアルレンジ) になりました
        [hold] マルチメーターの HOLD が { $choice } になりました
        [rel] マルチメーターの REL が { $choice } になりました
        [minmax] マルチメーターが { $choice } になりました
        [minmax-off] マルチメーターの MIN/MAX が解除されました
        [peak] マルチメーターが { $choice } になりました
        [peak-off] マルチメーターのピークが解除されました
    }
# The meter was on it already, so nothing was sent.
settings-already =
    { $case ->
       *[mode] マルチメーターはすでに { $choice } です
        [range-auto] マルチメーターはすでにオートレンジです ({ $range })
        [range] マルチメーターはすでに { $choice } (マニュアルレンジ) です
        [hold] マルチメーターの HOLD はすでに { $choice } です
        [rel] マルチメーターの REL はすでに { $choice } です
        [minmax] マルチメーターはすでに { $choice } です
        [minmax-off] マルチメーターの MIN/MAX はすでに解除されています
        [peak] マルチメーターはすでに { $choice } です
        [peak-off] マルチメーターのピークはすでに解除されています
    }

## Why 'set' failed, after "Error:" (common.ftl). $setting is the setting as
## typed: mode, range, hold, rel, minmax or peak.

# $input is the choice as typed, e.g. 2; $choices the choices it matches, as
# the listing spells them, separated by commas, e.g. "2.2V, 22V, 220V".
settings-ambiguous = { $setting } の値があいまいです: { $input } は { $choices } のいずれにも一致します
# No choice matches $input, the choice as typed; the listing is printed above.
settings-unknown = { $setting } の値が不明です: { $input }
# The meter answered that it wouldn't switch. $choice is the choice, e.g.
# 22V; $reason the meter's reason (lib-errors.ftl). A range is also refused
# when the input is past its end.
settings-refused =
    { $setting ->
        [range] マルチメーターが { $choice } を拒否しました: { $reason }。ファンクションスイッチの位置と、入力がレンジ内であることをご確認ください。
       *[other] マルチメーターが { $choice } を拒否しました: { $reason }。ファンクションスイッチの位置をご確認ください。
    }
# The meter took the command, but its readings never showed the choice.
settings-not-switched =
    { $setting ->
        [range] マルチメーターが切り替わりませんでした。ファンクションスイッチの位置と、入力がレンジ内であることをご確認ください。
       *[other] マルチメーターが切り替わりませんでした。ファンクションスイッチの位置をご確認ください。
    }
# As settings-not-switched, naming $live, the choice the meter is still on,
# e.g. off.
settings-not-switched-still =
    { $setting ->
        [range] マルチメーターが切り替わりませんでした (現在も { $live })。ファンクションスイッチの位置と、入力がレンジ内であることをご確認ください。
       *[other] マルチメーターが切り替わりませんでした (現在も { $live })。ファンクションスイッチの位置をご確認ください。
    }
