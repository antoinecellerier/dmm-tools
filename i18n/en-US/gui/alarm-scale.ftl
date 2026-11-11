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
scale-chip = Scale
    .hover = Turn scaling on or off
# The caret beside Scale: opens the row of fields below.
scale-settings =
    .hover = Set the scale
    .a11y = Scale settings
# The Alarm chip's body.
alarm-chip = Alarm
    .hover = Turn the alarm on or off
# The caret beside Alarm: opens the row of fields below.
alarm-settings =
    .hover = Set the limits
    .a11y = Alarm settings
# The hover of a greyed-out Scale or Alarm body while the window shows the
# meter alone with nothing typed in its fields yet. With the graph and the
# recording both hidden in Settings, there is no room for the row of
# fields.
alarm-scale-setup-in-settings = Show the graph or recording in Settings first
# The same in the big meter, which has no room for the row. $keys is the
# shortcut that leaves the big meter, e.g. Ctrl+B or ⌘B.
alarm-scale-setup-in-normal-layout = Set up in the normal layout first ({ $keys })

## The Scale row, under the chips while Scale's caret is open:
## "× [Scale factor] + [Offset] → [Unit label] [Apply] [Off]". The ×, +
## and → before each field are symbols, not text. The fields are narrow:
## their placeholders are cut short with … when longer than the field.

# Placeholder in the empty field after ×: the number each reading is
# multiplied by.
scale-factor = Scale factor
# Placeholder in the empty field after +: the number added after
# multiplying.
scale-offset = Offset
# Placeholder in the empty field after →: the unit the scaled reading is
# shown in, e.g. A or °F.
scale-unit = Unit label
# Applies the three fields to the reading.
scale-apply = Apply
    .hover = Apply these values to every reading from now on
# Turns scaling off; the fields keep their values for the next Apply.
scale-off = Off
    .hover = Stop scaling and show the meter's own reading again

## The Scale row's toasts.

# A field could not be read; nothing changes. $reason is why, lower-case
# with no full stop, e.g. “abc” is not a number.
scale-invalid-factor = Invalid scale: { $reason }
# The same for the Offset field, with $reason as there.
scale-invalid-offset = Invalid offset: { $reason }
# Scaling turned on, or changed. $transform is the scale in force, written
# in symbols and numbers, e.g. ×100 → A or ×2 + 0.5. $also says what else
# happened: recording, a recording is under way and goes on with the scaled
# readings; alarm, the alarm was on and was turned off, since its limits
# were in the old unit; both; or none.
scale-on =
    { $also ->
        [recording] Scaling readings: { $transform } — recording continues with scaled values
        [alarm] Scaling readings: { $transform } — alarm off; set its limits again in the new unit
        [both] Scaling readings: { $transform } — recording continues with scaled values — alarm off; set its limits again in the new unit
       *[none] Scaling readings: { $transform }
    }
# Scaling turned off. $also as in scale-on, except that recording means
# the recording goes on with the meter's own, unscaled readings.
scale-cleared =
    { $also ->
        [recording] Scaling off — recording continues with unscaled values
        [alarm] Scaling off — alarm off; set its limits again in the new unit
        [both] Scaling off — recording continues with unscaled values — alarm off; set its limits again in the new unit
       *[none] Scaling off
    }

## The Alarm row, under the chips while Alarm's caret is open:
## "Low [none] High [none] V ± [auto] [Apply] [Off]". The fields are narrow.

# Caption before the low limit's field.
alarm-low = Low
# Caption before the high limit's field.
alarm-high = High
# Placeholder in an empty Low or High field: that side has no limit.
alarm-no-limit = none
# The field after ±, the hysteresis band: how far back inside a limit the
# reading must come before that limit can alarm again. Its placeholder says
# that blank picks the band automatically.
alarm-band = auto
    .hover = How far back the reading must come before a limit alarms again: a value, or a percentage such as 1%. Blank: automatic
# After the limits, the unit they are in, e.g. V; this is its hover.
# A line added to the Low or High field's hover when its limit is typed in a
# unit of another quantity than the reading on screen: it is kept, and the
# alarm watches only readings of that quantity. $entry is the limit as typed
# ("5 kΩ"); $unit is its quantity's unit ("Ω").
alarm-limit-other-quantity = { $entry }: alarms on { $unit } readings
alarm-unit =
    .hover = The unit the limits are in
# The same before an alarm is on: the unit taken from the reading on
# screen.
alarm-unit-from-reading =
    .hover = The unit the limits are in, from the reading on screen
# Shown in place of the unit, in the warning colour, while the reading is
# of another quantity than the limits: the dial was turned. $unit is the
# unit of the readings now arriving, e.g. Ω; on hover, $limits is the unit
# the limits are in, e.g. V. Short: it sits in the row.
alarm-idle = idle: { $unit }
    .hover = Waiting: the readings are in { $unit }, the limits in { $limits }
# Turns the alarm on with the fields' limits.
alarm-apply = Apply
    .hover = Turn the alarm on with these limits
# Turns the alarm off; the fields keep their values for the next Apply.
alarm-off = Off
    .hover = Turn the alarm off

## The Alarm row's toasts. $reason is why a value was refused, lower-case
## with no full stop, e.g. “abc” is not a number.

# The Low field could not be read.
alarm-invalid-low = Invalid low limit: { $reason }
# The High field could not be read.
alarm-invalid-high = Invalid high limit: { $reason }
# A limit is not a finite number.
alarm-invalid-limit = Invalid limit: { $reason }
# The two limits don't make a pair: the low one is not below the high one.
alarm-invalid-limits = Invalid limits: { $reason }
# The ± field could not be read, or does not fit the limits.
alarm-invalid-band = Invalid hysteresis: { $reason }
# The alarm turned on. $limits is the limits it watches, from
# alarm-limits/alarm-hysteresis in lib-alarm.ftl, e.g. "above 5 or below 3,
# hysteresis 1%".
alarm-on-toast = Alarm on: { $limits }
# The alarm turned off.
alarm-off-toast = Alarm off
# The alarm carried over to an import or another meter turned off: the new
# readings put its limits in another unit than before. $unit is the new
# one, e.g. V or kV.
alarm-off-other-unit = Readings in { $unit } — alarm off; set its limits again in the new unit

## The graph: a dotted line at each limit, labelled at its right end.

# $side is high (the high limit) or low; $value is the limit with its unit,
# e.g. 3.0000 V.
alarm-limit-line =
    { $side ->
        [low] Low: { $value }
       *[high] High: { $value }
    }
