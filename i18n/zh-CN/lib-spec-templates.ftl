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
