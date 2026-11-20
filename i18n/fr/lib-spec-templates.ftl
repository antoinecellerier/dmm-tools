### Specification text made of parts: each message here stands for every
### band, input or note that reads as its English, so lib-specs.ftl lists
### none of them. Written by hand.
###
### $text is another specification text, already translated: put it where
### your language puts it. Any other variable is one value (10kHz, >5A,
### +400 °C), shown as written. Keep the numbers that are written out here
### exactly, as in lib-specs.ftl.

# An accuracy band at the 4000-count display setting: $text is the band.
band-at-4000-counts = { $text } (4000 points)
# A temperature span the accuracy holds over.
# e.g. +40 to +400 °C
band-temperature-from-to = { $from } à { $to }
# A frequency band and the share of the measuring range (MR) the accuracy
# holds over.
# e.g. 10kHz~120kHz, 10 - 15% of MR
band-frequency-share-of-range = { $low }~{ $high }, { $from } - { $to } de la plage

# Overload protection: a fuse's current and voltage ratings.
# e.g. Fuse 10A 1000V
input-protection-fuse = Fusible { $current } { $voltage }
# An approximate input impedance.
# e.g. About 10MΩ
input-impedance-about = Environ { $value }

# Frequency counting on an AC range: $range is the range, $from and $to the
# frequencies counted, $level the smallest signal that counts.
# e.g. AC 600.0mV: 15.00 ~ 50.00kHz, sensitivity 60mV (sine RMS)
note-ac-range-sensitivity = AC { $range } : { $from } ~ { $to }, sensibilité { $level } (valeur efficace sinusoïdale)
# The voltage at the open test leads.
# e.g. Open circuit ≈3V
note-open-circuit-voltage = Circuit ouvert { $voltage }
# The largest value the function shows.
# e.g. Max 600V
note-max-reading = Max. { $value }
# The share of each range the accuracy holds over.
# e.g. Accuracy valid 5%–100% of range
note-accuracy-valid-share-of-range = Précision valable sur { $from }–{ $to } de la plage
# The same within a frequency band, with one range's own share in brackets.
# e.g. Accuracy valid 40Hz–1kHz, 1%–100% of range (60mV: 2%–100%)
note-accuracy-valid-in-frequency-band = Précision valable sur { $low }–{ $high }, { $from }–{ $to } de la plage ({ $range } : { $rangeFrom }–{ $rangeTo })
# The share of each range true-RMS sensing holds over.
# e.g. True RMS valid 10%–100% of range
note-true-rms-valid-share-of-range = True RMS valable sur { $from }–{ $to } de la plage
# A note with the crest factor it holds up to after it: $text is the note.
# e.g. True RMS valid 10%–100% of range; crest factor ≤3.0
note-with-crest-factor = { $text } ; facteur de crête { $factor }
# What the display shows with the test leads shorted.
# e.g. Shorted leads: residual ≤10 digits
note-shorted-leads-residual = Cordons court-circuités : résidu { $count } points
# The signal a frequency measurement needs: $from and $to are the
# frequencies, $low and $high the signal levels.
# e.g. Input 10Hz–40MHz: 200mV–30V rms
note-frequency-input-level = Entrée { $from }–{ $to } : { $low }–{ $high } rms
# The voltage a current range drops per unit of current.
# e.g. Burden voltage 2.1mV/mA
note-burden-voltage = Chute de tension { $value }
# A capacitance range picked by hand, and the value its accuracy starts at.
# e.g. 6.000mF, manual range: not specified below 0.54mF
note-capacitance-manual-range-floor = { $range }, plage manuelle : non spécifié en dessous de { $limit }
# How long a reading takes on a large capacitance range.
# e.g. 60mF range: measurement takes ≈20s
note-capacitance-measurement-time = Plage { $range } : la mesure prend { $time }
# A low resistance range, whose reading includes the test leads' resistance.
# e.g. 200Ω: accuracy + test-lead short-circuit resistance
note-resistance-plus-lead-resistance = { $range } : précision + résistance des cordons de mesure court-circuités

# Overload protection: the largest voltage, DC or AC rms.
# e.g. 1000V DC/AC rms
input-protection-dc-ac-rms = { $voltage } DC/AC rms
# An input impedance: its resistance and the capacitance beside it.
# e.g. 10MΩ, 50pF nominal
input-impedance-nominal = { $resistance }, { $capacitance } nominal
# The same on a low-impedance input, before it rises.
# e.g. Initially approx. 3.0kΩ, 150pF nominal
input-impedance-initially-nominal = Initialement environ { $resistance }, { $capacitance } nominal

# What measuring AC+DC adds to a range's accuracy.
# e.g. AC+DC: add 1%+30 digits to accuracy
note-ac-dc-adds-to-accuracy = AC+DC : ajoutez { $percent }+{ $count } points à la précision
# A low capacitance range, whose reading includes the open test leads'
# capacitance.
# e.g. 20nF: accuracy + open-circuit test-lead capacitance
note-capacitance-plus-lead-capacitance = { $range } : précision + capacité des cordons de mesure en circuit ouvert
# How to take the open test leads' capacitance off a reading.
# e.g. Result = reading − open-lead capacitance; ≤1µF: use REL
note-capacitance-result-use-rel = Résultat = mesure − capacité des cordons en circuit ouvert ; { $limit } : utilisez REL
# The continuity beeper's two thresholds.
# e.g. Beeps continuously ≤10Ω; silent >30Ω
note-continuity-beeps-silent = Bip continu { $below } ; silencieux { $above }
# What a current range shows with nothing connected.
# e.g. Open circuit: residual ≤5 digits
note-open-circuit-residual = Circuit ouvert : résidu { $count } points
# The crest factor true-RMS sensing takes at two display counts.
# e.g. True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts
note-true-rms-crest-factor-at-counts = True RMS ; facteur de crête { $factor } à { $count }, { $fullFactor } à { $fullCount } points
# Two ranges whose accuracy in a frequency band holds over a share of the
# range only.
# e.g. 60.00mV, 600.0mV, 1kHz ~ 20kHz: specified 30% to 100% of range
note-ranges-specified-share-in-frequency-band = { $range }, { $otherRange }, { $low } ~ { $high } : spécifié de { $from } à { $to } de la plage
# A range whose accuracy in a frequency band holds over part of the band
# only.
# e.g. 9.999V, 1kHz ~ 20kHz: specified for 1kHz ~ 15kHz only
note-range-specified-for-frequency-band = { $range }, { $low } ~ { $high } : spécifié pour { $from } ~ { $to } uniquement
# The signal a frequency measurement needs, up to or above one frequency.
# e.g. Hz input ≤100kHz: 200mV–20V rms
note-hz-input-level = Entrée Hz { $limit } : { $low }–{ $high } rms
# A frequency measurement's input, and the frequency its accuracy ends at.
# e.g. Input at zero DC level; >40MHz not specified
note-frequency-input-zero-dc-level = Entrée à niveau DC nul ; { $limit } non spécifié
# How long a high current may be measured: $range is the range, $continuous
# the current it takes without limit, $from and $to the currents it takes
# for $time, with $rest between measurements.
# e.g. 10A: ≤5A continuous; >5A–10A max. 10s, then rest >15min
note-current-time-limit = { $range } : { $continuous } en continu ; { $from }–{ $to } ≤{ $time } à la fois, à { $rest } d’intervalle
