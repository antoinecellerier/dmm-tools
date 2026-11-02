### The reading display: the meter's reading at the top of the reading
### column (assets/gui-wide-layout.webp, assets/gui-narrow-layout.webp), and
### the same reading scaled up to fill the window in the big meter
### (assets/gui-big-meter.webp, assets/gui-minimal-meter-wide.webp).
###
### The reading is the digits and the unit, then the sub-values in rows
### (Max, Min, T2, Frequency…), then the mode and range readouts and the
### meter's badges (HOLD, AUTO…). The digits, units, mode and range names,
### sub-value names and the meter's badges come from the meter or the
### library and are not in this file; these messages are what the app adds.

## The readouts under the reading: the mode, and the range beside it. Where
## the meter can be switched from the app, each is a dropdown drawn as its
## label (V DC ▾, 6V ▾) (assets/gui-reading-controls.webp).

# The mode readout as a dropdown of the modes the meter can switch to at
# its dial position.
reading-mode =
    .hover = マルチメーターを、ファンクションスイッチの現在の位置にある別のモードに切り替える
    .a11y = モード
# The range readout as a dropdown of the meter's ranges in this mode.
reading-range =
    .hover = マルチメーターを現在のモードの別のレンジに切り替える
    .a11y = レンジ
# The mode readout on a meter that lists its function keys instead of its
# modes: the value is the caption over the open list, which says its entries
# are key presses. A screen reader still calls the dropdown Mode.
reading-keys = マルチメーターのキー
    .hover = マルチメーターの機能キーのいずれかを押す
    .a11y = モード

## The badges after the meter's own, in capitals like them and as short:
## they sit on the mode line, which a narrow window wraps.

# Shown while a Scale turns the meter's reading into another quantity.
reading-badge-scaled = スケール
# Shown, in the error colour, while the reading is above the alarm's high
# limit.
reading-badge-above = 上限超過
# Shown, in the error colour, while the reading is below the alarm's low
# limit.
reading-badge-below = 下限未満

## What stands where the reading goes when there is none.

# Beside or under dashes (---) while no meter is connected or none has sent
# a reading; also what a screen reader says there.
reading-none = 測定値なし
# The big meter's second line under the connection problem that takes the
# reading's place: hovering it shows the steps, and $keys (the shortcut that
# turns the big meter off, e.g. Ctrl+B or ⌘B) brings the whole window back.
reading-big-meter-hint = { $keys } で詳細を表示

## What a screen reader says for the reading: never drawn, read aloud each
## time the reading changes. It is one list of phrases, in this order: the
## value, the mode, each sub-value, the meter's badges, then the app's;
## reading-spoken-separator goes between two phrases. For example:
## "DC 1.6112 V, AC+DC V, AC 0.0123 V, hold, software scaled".

# Between two phrases of the list.
reading-spoken-separator = {"、"}
# The reading, or a sub-value, with the name drawn before it. $label is the
# name (DC, Max, T2, Frequency…); $value is the value with its unit, e.g.
# "1.6112 V", "overload Ω" or "24.10 degrees C".
reading-spoken-labelled = { $label } { $value }
# A sub-value the meter caught $seconds seconds after MIN/MAX began,
# drawn as "@12s". $label and $value as in reading-spoken-labelled.
reading-spoken-labelled-at =
    { $seconds ->
       *[other] { $seconds } 秒時点の { $label } { $value }
    }
# Said in place of the digits when the input is past the range (OL).
reading-spoken-overload = オーバーレンジ
# The non-contact voltage detector's level; $level is a number from 1 up.
reading-spoken-ncv = NCV レベル { $level }
# A word the meter shows in place of the digits, $word (e.g. ---- or Auto),
# which says there is no reading.
reading-spoken-no-reading = 測定値なし ({ $word })
# The units °C and °F spelled out, since screen readers differ on whether
# they read the ° sign. C and F stay as letters, after the number.
reading-spoken-celsius = 摂氏度
reading-spoken-fahrenheit = 華氏度

## Unit names, said after a number in place of the unit's symbol, which a
## screen reader spells out letter by letter (mV as "m V"). The screen keeps
## the symbols; only the spoken reading says these.
##
## $value is the number the name follows, as the screen shows it, e.g.
## 1.000 or -0.52, to choose the plural by; after "overload", or in a mode's
## name (DC volts), it is no number and the last form is taken. $prefix is
## the SI prefix's word below, empty for none, and leads the name. Only
## reading-unit-ohm is also given $scale, which prefix it is (none, pico,
## nano, micro, milli, kilo, mega or giga), for a pair the language writes
## its own way, as English contracts kilohm and megohm. reading-unit-percent
## is given none of these.

# The SI prefixes, as they lead a unit's name (milli in millivolt).
reading-unit-prefix-pico = ピコ
reading-unit-prefix-nano = ナノ
reading-unit-prefix-micro = マイクロ
reading-unit-prefix-milli = ミリ
reading-unit-prefix-kilo = キロ
reading-unit-prefix-mega = メガ
reading-unit-prefix-giga = ギガ
reading-unit-volt =
    { $prefix }{ $value ->
       *[other] ボルト
    }
reading-unit-ampere =
    { $prefix }{ $value ->
       *[other] アンペア
    }
reading-unit-ohm =
    { $scale ->
        [kilo]
            { $value ->
               *[other] キロオーム
            }
        [mega]
            { $value ->
               *[other] メガオーム
            }
       *[other]
            { $prefix }{ $value ->
               *[other] オーム
            }
    }
reading-unit-hertz =
    { $prefix }{ $value ->
       *[other] ヘルツ
    }
reading-unit-farad =
    { $prefix }{ $value ->
       *[other] ファラド
    }
reading-unit-siemens =
    { $prefix }{ $value ->
       *[other] ジーメンス
    }
reading-unit-watt =
    { $prefix }{ $value ->
       *[other] ワット
    }
reading-unit-volt-ampere =
    { $prefix }{ $value ->
       *[other] ボルトアンペア
    }
reading-unit-henry =
    { $prefix }{ $value ->
       *[other] ヘンリー
    }
reading-unit-second =
    { $prefix }{ $value ->
       *[other] 秒
    }
# The % sign; it takes no prefix.
reading-unit-percent = パーセント
# A number and its unit's name: $value is the number as the screen shows
# it, e.g. 1.6112, or a word such as "overload"; $unit is one of the names
# above, e.g. millivolts. $kind is percent for a number followed by
# reading-unit-percent, for a language that says percent before the
# number, and other otherwise.
reading-spoken-amount =
    { $kind ->
        [percent] { $value } { $unit }
       *[other] { $value } { $unit }
    }

## The meter's badges spoken, one phrase each.

# HV!: the probes are on a dangerous voltage.
reading-spoken-hv-warning = 高電圧警告
# AUTO: the meter picks its range itself.
reading-spoken-auto-range = オートレンジ
# HOLD: the display is frozen.
reading-spoken-hold = データホールド
# REL: the reading is relative to a stored one.
reading-spoken-rel = 相対値
# MIN: the lowest reading since MIN/MAX began.
reading-spoken-min = 最小値
# MAX: the highest reading since MIN/MAX began.
reading-spoken-max = 最大値
# AVG: the mean of the readings since MIN/MAX began.
reading-spoken-avg = 平均値
# P-MIN: the lowest peak.
reading-spoken-peak-min = ピーク最小値
# P-MAX: the highest peak.
reading-spoken-peak-max = ピーク最大値
# LOW BAT: the meter's battery is low.
reading-spoken-low-battery = 電池消耗
# LEAD ERR: the meter reports a problem with the test leads.
reading-spoken-lead-error = テストリード誤挿入
# COMP: the meter's compare mode, which checks the reading against limits.
reading-spoken-comp = 比較
# REC: the meter is recording readings into its own memory.
reading-spoken-record = 記録中
# LoZ: the low-impedance voltage mode.
reading-spoken-loz = 低インピーダンス
# VOID: the reading is not valid.
reading-spoken-void = 無効

## The app's own badges spoken, after the meter's.

# reading-badge-scaled.
reading-spoken-scaled = ソフトウェアでスケーリング済み
# reading-badge-above.
reading-spoken-above = 上限超過
# reading-badge-below.
reading-spoken-below = 下限未満
