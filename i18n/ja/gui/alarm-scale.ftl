### The Scale and Alarm chips at the end of the meter's buttons under the
### reading (assets/gui-reading-controls.webp), the row of fields each chip
### opens under them, the toasts they raise (the short messages under the
### top bar's right end) and the alarm's limit lines on the graph. No doc
### image shows a row open.
###
### Scale is a software transform: it turns the meter's reading into what
### it stands for, such as a current clamp's 10 mV/A into amps. Nothing is
### sent to the meter. Alarm watches the main reading against a low and a
### high limit, and marks each reading that crosses one.
###
### Numbers and units arrive formatted and stay as they are.

## The chips, at the end of the meter's buttons (HOLD, REL…), which a
## narrow window wraps: keep the names short. Each chip is a body that
## switches its feature on or off, joined to a caret (▸) that opens its row
## of fields; the body is filled while the feature is on.

# The Scale chip's body.
scale-chip = スケール
    .hover = スケーリングのオン/オフを切り替える
# The caret beside Scale: opens the row of fields below.
scale-settings =
    .hover = スケールを設定する
    .a11y = スケールの設定
# The Alarm chip's body.
alarm-chip = アラーム
    .hover = アラームのオン/オフを切り替える
# The caret beside Alarm: opens the row of fields below.
alarm-settings =
    .hover = 上限と下限を設定する
    .a11y = アラームの設定
# The hover of a greyed-out Scale or Alarm body while the window shows the
# meter alone with nothing typed in its fields yet. With the graph and the
# recording both hidden in Settings, there is no room for the row of
# fields.
alarm-scale-setup-in-settings = 先に [設定] でグラフまたは記録を表示してください。
# The same in the big meter, which has no room for the row. $keys is the
# shortcut that leaves the big meter, e.g. Ctrl+B or ⌘B.
alarm-scale-setup-in-normal-layout = 先に通常のレイアウトで設定してください ({ $keys })。

## The Scale row, under the chips while Scale's caret is open:
## "× [Scale factor] + [Offset] → [Unit label] [Apply] [Off]". The ×, +
## and → before each field are symbols, not text. The fields are narrow:
## their placeholders are cut short with … when longer than the field.

# Placeholder in the empty field after ×: the number each reading is
# multiplied by.
scale-factor = 倍率
# Placeholder in the empty field after +: the number added after
# multiplying.
scale-offset = 加算
# Placeholder in the empty field after →: the unit the scaled reading is
# shown in, e.g. A or °F.
scale-unit = 単位
# Applies the three fields to the reading.
scale-apply = 適用
    .hover = これ以降のすべての測定値にこの値を適用する
# Turns scaling off; the fields keep their values for the next Apply.
scale-off = オフ
    .hover = スケーリングを停止し、マルチメーターの元の測定値を表示する

## The Scale row's toasts.

# A field could not be read; nothing changes. $reason is why, lower-case
# with no full stop, e.g. “abc” is not a number.
scale-invalid-factor = 倍率を使用できません: { $reason }
# The same for the Offset field, with $reason as there.
scale-invalid-offset = オフセットを使用できません: { $reason }
# Scaling turned on, or changed. $transform is the scale in force, written
# in symbols and numbers, e.g. ×100 → A or ×2 + 0.5. $also says what else
# happened: recording, a recording is under way and goes on with the scaled
# readings; alarm, the alarm was on and was turned off, since its limits
# were in the old unit; both; or none.
scale-on =
    { $also ->
        [recording] 測定値をスケーリングしています: { $transform }。記録はスケーリング後の値で続行されます。
        [alarm] 測定値をスケーリングしています: { $transform }。アラームはオフになりました。新しい単位で上限と下限を設定し直してください。
        [both] 測定値をスケーリングしています: { $transform }。記録はスケーリング後の値で続行されます。アラームはオフになりました。新しい単位で上限と下限を設定し直してください。
       *[none] 測定値をスケーリングしています: { $transform }
    }
# Scaling turned off. $also as in scale-on.
scale-off-toast =
    { $also ->
        [recording] スケーリングはオフになりました。記録はスケーリング後の値で続行されます。
        [alarm] スケーリングはオフになりました。アラームもオフになりました。新しい単位で上限と下限を設定し直してください。
        [both] スケーリングはオフになりました。記録はスケーリング後の値で続行されます。アラームもオフになりました。新しい単位で上限と下限を設定し直してください。
       *[none] スケーリング オフ
    }

## The Alarm row, under the chips while Alarm's caret is open:
## "Low [none] High [none] V ± [auto] [Apply] [Off]". The fields are narrow.

# Caption before the low limit's field.
alarm-low = 下限
# Caption before the high limit's field.
alarm-high = 上限
# Placeholder in an empty Low or High field: that side has no limit.
alarm-no-limit = なし
# The field after ±, the hysteresis band: how far back inside a limit the
# reading must come before that limit can alarm again. Its placeholder says
# that blank picks the band automatically.
alarm-band = 自動
    .hover = 測定値がどこまで戻れば、その制限値でもう一度アラームが鳴るかを指定します。値、または 1% のようなパーセントを入力します。空欄の場合は自動です。
# After the limits, the unit they are in, e.g. V; this is its hover.
alarm-unit =
    .hover = 上限と下限の単位
# The same before an alarm is on: the unit taken from the reading on
# screen.
alarm-unit-from-reading =
    .hover = 上限と下限の単位 (画面の測定値から取得)
# Shown in place of the unit, in the warning colour, while the reading is
# of another quantity than the limits: the dial was turned. $unit is the
# unit of the readings now arriving, e.g. Ω; on hover, $limits is the unit
# the limits are in, e.g. V. Short: it sits in the row.
alarm-idle = 待機中: { $unit }
    .hover = 待機中: 測定値の単位は { $unit }、上限と下限の単位は { $limits } です。
# Turns the alarm on with the fields' limits.
alarm-apply = 適用
    .hover = この上限と下限でアラームをオンにする
# Turns the alarm off; the fields keep their values for the next Apply.
alarm-off = オフ
    .hover = アラームをオフにする

## The Alarm row's toasts. $reason is why a value was refused, lower-case
## with no full stop, e.g. “abc” is not a number.

# The Low field could not be read.
alarm-invalid-low = 下限を使用できません: { $reason }
# The High field could not be read.
alarm-invalid-high = 上限を使用できません: { $reason }
# A limit is not a finite number.
alarm-invalid-limit = 制限値を使用できません: { $reason }
# The two limits don't make a pair: the low one is not below the high one.
alarm-invalid-limits = 上限と下限を使用できません: { $reason }
# The ± field could not be read, or does not fit the limits.
alarm-invalid-band = ヒステリシスを使用できません: { $reason }
# The alarm turned on. $limits is the limits it watches, from
# alarm-limits/alarm-hysteresis in lib-alarm.ftl, e.g. "above 5 or below 3,
# hysteresis 1%".
alarm-on-toast = アラーム オン: { $limits }
# The alarm turned off.
alarm-off-toast = アラーム オフ

## The graph: a dotted line at each limit, labelled at its right end.

# $side is high (the high limit) or low; $value is the limit with its unit,
# e.g. 3.0000 V.
alarm-limit-line =
    { $side ->
        [low] 下限: { $value }
       *[high] 上限: { $value }
    }
