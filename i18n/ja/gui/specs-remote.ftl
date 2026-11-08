### The Specifications panel in the reading column, under the meter's
### buttons (assets/gui-wide-layout.webp): the figures the meter's manual
### gives for the reading on screen, folded to one line in a narrow window
### (assets/gui-narrow-layout.webp) and on one line under the reading in
### the big meter (assets/gui-big-meter.webp). Then the hovers of the
### meter's buttons under the reading (assets/gui-reading-controls.webp),
### and the title of the What's New window.
###
### The figures (0.1mV, ±(0.05%+5), 45Hz~1kHz), the input impedance and
### the notes under them come from the meter's manual through the library
### and are not in this file; these messages are the words the app puts
### around them. The buttons' own names (HOLD, REL, RANGE, MIN/MAX…) are
### the meter's and stay as the meter prints them.

## The Specifications panel, in a wide window: a heading that folds the
## panel to one line, then a line per figure, then the link to the manual.
## The column is narrow (about 240 pixels): keep the captions short.

# The panel's heading; a click folds the panel to one line or unfolds it.
# Also what a screen reader calls the heading. Its hover depends on the
# panel's state: hover-fold while unfolded, hover-unfold while folded.
specs-heading = 仕様
    .hover-fold = 分解能と確度だけを 1 行で表示する
    .hover-unfold = すべての仕様を表示する
# The reading's resolution: the smallest step the meter shows in this
# range. $resolution is a figure with its unit, e.g. 0.1mV. The two spaces
# set the caption apart from the figure.
specs-resolution = 分解能  { $resolution }
# The reading's accuracy when the manual gives one figure for the range.
# $accuracy is that figure, e.g. ±(0.05%+5): a percentage of the reading
# plus a number of digits; or the manual's own words for it, e.g. Not
# Specified.
specs-accuracy = 確度  { $accuracy }
# Heading over the accuracy figures when the manual gives one per frequency
# band: each line under it is a band and its figure, e.g.
# "45Hz~1kHz ±(0.5%+30)".
specs-accuracy-bands = 確度
# The mode's input impedance: the load the meter puts on the circuit.
# $impedance is the manual's figure, e.g. About 10MΩ. "Input Z" is the
# usual short form on meters; a translation may spell it out if it fits.
specs-input-impedance = 入力インピーダンス  { $impedance }
# A link at the panel's end, and on the summary line, that opens the
# meter's user manual (a PDF on the maker's site) in the web browser. The ↗
# says it leaves the app.
specs-manual = 取扱説明書 ↗
    .hover = メーカーの取扱説明書をブラウザーで開く

## The one-line summary: the folded panel and the narrow window's line,
## e.g. "Res: 0.1mV  Acc: ±(0.05%+5)  |  Manual ↗". Very short captions,
## abbreviated with a colon.

# $resolution as in specs-resolution.
specs-resolution-short = 分解能: { $resolution }
# $accuracy as in specs-accuracy; with several bands, the first band's
# figure and its band, e.g. ±(0.5%+30) 45Hz~1kHz.
specs-accuracy-short = 確度: { $accuracy }

## The big meter's line under the reading, e.g.
## "Resolution 0.1mV  |  Accuracy ±(0.05%+5)  |  Manual ↗".

# $resolution as in specs-resolution.
specs-inline-resolution = 分解能 { $resolution }
# $accuracy as in specs-accuracy-short.
specs-inline-accuracy = 確度 { $accuracy }

## The hovers of the meter's buttons under the reading, in the order they
## show: HOLD, REL, RANGE, AUTO, MAX/MIN, PEAK, SELECT and LIGHT, then the
## meter's own keys. A click sends that button's press to the meter. Each
## describes what multimeters generally do on that button, not one model.

# HOLD: the meter keeps showing the reading it had.
remote-hold =
    .hover = 現在の測定値を固定する
# REL (relative): the meter shows each reading minus the one it had when
# pressed.
remote-rel =
    .hover = 現在の値を基準とした相対値を表示する
# RANGE: the meter leaves auto-range and steps to its next range.
remote-range =
    .hover = RANGE を押す (マニュアルレンジ、1 段階)
# AUTO: the meter picks its range itself again.
remote-auto =
    .hover = マルチメーターをオートレンジに戻す
# MAX/MIN: a click steps between the meter's highest and lowest readings; the
# small × button beside it while it is on leaves the mode.
remote-min-max-exit =
    .hover = 最大値と最小値を記録する (クリックで切り替え、× で終了)
# The same on a meter with no × beside the button.
remote-min-max-no-exit =
    .hover = 最大値と最小値を記録する (クリックで切り替え)
# PEAK: the same for the fastest peaks the meter catches.
remote-peak =
    .hover = ピーク最小値とピーク最大値を取得する (クリックで切り替え、× で終了)
# PEAK on a meter with no × beside the button.
remote-peak-no-exit =
    .hover = ピーク値を取得する
# The small × button after MIN/MAX or PEAK while that mode is on: it leaves
# the mode. $mode is the button's name as the meter prints it, MIN/MAX or
# PEAK. The hover, and its name for a screen reader.
remote-exit =
    .hover = { $mode } モードを終了する
    .a11y = { $mode } モードを終了する
# The hover of a key only some readings have (ZERO in capacitance, 1kHz in
# AC…), when the library gives none of its own. $key is the key's name as
# the meter prints it, e.g. ZERO.
remote-press-key = マルチメーターの { $key } キーを押す
# SELECT: steps through the functions that share the dial position, e.g.
# AC and DC.
remote-select =
    .hover = ファンクションスイッチの現在の位置で使えるサブ機能を順に切り替える
# LIGHT: the display's backlight.
remote-light =
    .hover = マルチメーターのバックライトをオン/オフする

## The What's New window, opened from the version at the top bar's right
## end: the window's title, which the system draws in its title bar. The
## release notes inside it are in English and not in this file.

# In a development build, whose notes are the changes not yet released.
whats-new-title-unreleased = 最新情報 (リリース前)
# In a release. $version is its number, e.g. 0.10.0, after a v.
whats-new-title = v{ $version } の最新情報
