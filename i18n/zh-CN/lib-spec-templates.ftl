### Specification text made of parts: each message here stands for every
### band, input or note that reads as its English, so lib-specs.ftl lists
### none of them. Written by hand.
###
### $text is another specification text, already translated: put it where
### your language puts it. Any other variable is one value (10kHz, >5A,
### +400 °C), shown as written. Keep the numbers that are written out here
### exactly, as in lib-specs.ftl.

# An accuracy band at the 4000-count display setting: $text is the band.
band-at-4000-counts = { $text } (4000 计数)
# A temperature span the accuracy holds over.
# e.g. +40 to +400 °C
band-temperature-from-to = { $from } 到 { $to }
# A frequency band and the share of the measuring range (MR) the accuracy
# holds over.
# e.g. 10kHz~120kHz, 10 - 15% of MR
band-frequency-share-of-range = { $low }~{ $high }，量程的 { $from } - { $to }

# Overload protection: a fuse's current and voltage ratings.
# e.g. Fuse 10A 1000V
input-protection-fuse = 保险丝 { $current } { $voltage }
# An approximate input impedance.
# e.g. About 10MΩ
input-impedance-about = 约 { $value }

# Frequency counting on an AC range: $range is the range, $from and $to the
# frequencies counted, $level the smallest signal that counts.
# e.g. AC 600.0mV: 15.00 ~ 50.00kHz, sensitivity 60mV (sine RMS)
note-ac-range-sensitivity = AC { $range }: { $from } ~ { $to }，灵敏度 { $level } (正弦波有效值)
# The voltage at the open test leads.
# e.g. Open circuit ≈3V
note-open-circuit-voltage = 开路电压 { $voltage }
# The largest value the function shows.
# e.g. Max 600V
note-max-reading = 最大 { $value }
# The share of each range the accuracy holds over.
# e.g. Accuracy valid 5%–100% of range
note-accuracy-valid-share-of-range = 准确度有效范围: 量程的 { $from }–{ $to }
# The same within a frequency band, with one range's own share in brackets.
# e.g. Accuracy valid 40Hz–1kHz, 1%–100% of range (60mV: 2%–100%)
note-accuracy-valid-in-frequency-band = 准确度有效范围: { $low }–{ $high }，量程的 { $from }–{ $to } ({ $range }: { $rangeFrom }–{ $rangeTo })
# The share of each range true-RMS sensing holds over.
# e.g. True RMS valid 10%–100% of range
note-true-rms-valid-share-of-range = 真有效值有效范围: 量程的 { $from }–{ $to }
# A note with the crest factor it holds up to after it: $text is the note.
# e.g. True RMS valid 10%–100% of range; crest factor ≤3.0
note-with-crest-factor = { $text }；波峰因数 { $factor }
# What the display shows with the test leads shorted.
# e.g. Shorted leads: residual ≤10 digits
note-shorted-leads-residual = 表笔短路: 剩余读数 { $count } 个字
# The signal a frequency measurement needs: $from and $to are the
# frequencies, $low and $high the signal levels.
# e.g. Input 10Hz–40MHz: 200mV–30V rms
note-frequency-input-level = 输入 { $from }–{ $to }: { $low }–{ $high } rms
# The voltage a current range drops per unit of current.
# e.g. Burden voltage 2.1mV/mA
note-burden-voltage = 负载电压 { $value }
# A capacitance range picked by hand, and the value its accuracy starts at.
# e.g. 6.000mF, manual range: not specified below 0.54mF
note-capacitance-manual-range-floor = { $range }，手动量程: { $limit } 以下未指定
# How long a reading takes on a large capacitance range.
# e.g. 60mF range: measurement takes ≈20s
note-capacitance-measurement-time = { $range } 量程: 测量耗时 { $time }
# A low resistance range, whose reading includes the test leads' resistance.
# e.g. 200Ω: accuracy + test-lead short-circuit resistance
note-resistance-plus-lead-resistance = { $range }: 准确度 + 表笔短路电阻

# Overload protection: the largest voltage, DC or AC rms.
# e.g. 1000V DC/AC rms
input-protection-dc-ac-rms = { $voltage } DC/AC rms
# An input impedance: its resistance and the capacitance beside it.
# e.g. 10MΩ, 50pF nominal
input-impedance-nominal = 标称 { $resistance }，{ $capacitance }
# The same on a low-impedance input, before it rises.
# e.g. Initially approx. 3.0kΩ, 150pF nominal
input-impedance-initially-nominal = 初始约 { $resistance }，标称 { $capacitance }

# What measuring AC+DC adds to a range's accuracy.
# e.g. AC+DC: add 1%+30 digits to accuracy
note-ac-dc-adds-to-accuracy = AC+DC: 准确度加 { $percent }+{ $count } 个字
# A low capacitance range, whose reading includes the open test leads'
# capacitance.
# e.g. 20nF: accuracy + open-circuit test-lead capacitance
note-capacitance-plus-lead-capacitance = { $range }: 准确度 + 表笔开路电容
# How to take the open test leads' capacitance off a reading.
# e.g. Result = reading − open-lead capacitance; ≤1µF: use REL
note-capacitance-result-use-rel = 结果 = 读数 − 表笔开路电容；{ $limit }: 使用 REL
# The continuity beeper's two thresholds.
# e.g. Beeps continuously ≤10Ω; silent >30Ω
note-continuity-beeps-silent = { $below } 时连续蜂鸣；{ $above } 时无声
# What a current range shows with nothing connected.
# e.g. Open circuit: residual ≤5 digits
note-open-circuit-residual = 开路: 剩余读数 { $count } 个字
# The crest factor true-RMS sensing takes at two display counts.
# e.g. True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts
note-true-rms-crest-factor-at-counts = 真有效值；波峰因数 { $count } 计数时 { $factor }，{ $fullCount } 计数时 { $fullFactor }
# Two ranges whose accuracy in a frequency band holds over a share of the
# range only.
# e.g. 60.00mV, 600.0mV, 1kHz ~ 20kHz: specified 30% to 100% of range
note-ranges-specified-share-in-frequency-band = { $range }，{ $otherRange }，{ $low } ~ { $high }: 指定范围为量程的 { $from }–{ $to }
# A range whose accuracy in a frequency band holds over part of the band
# only.
# e.g. 9.999V, 1kHz ~ 20kHz: specified for 1kHz ~ 15kHz only
note-range-specified-for-frequency-band = { $range }，{ $low } ~ { $high }: 指定范围仅为 { $from } ~ { $to }
# The signal a frequency measurement needs, up to or above one frequency.
# e.g. Hz input ≤100kHz: 200mV–20V rms
note-hz-input-level = Hz 输入 { $limit }: { $low }–{ $high } rms
# A frequency measurement's input, and the frequency its accuracy ends at.
# e.g. Input at zero DC level; >40MHz not specified
note-frequency-input-zero-dc-level = 输入直流电平为零；{ $limit } 未指定
# How long a high current may be measured: $range is the range, $continuous
# the current it takes without limit, $from and $to the currents it takes
# for $time, with $rest between measurements.
# e.g. 10A: ≤5A continuous; >5A–10A max. 10s, then rest >15min
note-current-time-limit = { $range }: { $continuous } 可连续使用；{ $from }–{ $to } 每次 ≤{ $time }，间隔 { $rest }
