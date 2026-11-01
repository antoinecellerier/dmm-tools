### The library's names that the apps translate by what they name rather
### than by their English: status flags, link names, the names of a main
### reading and the simulated meters' brand. Hand-written; a test checks the
### English against the library's own.

## Status flags: the meter's annunciators, beside the reading and in the
## statistics. They are what the meter prints on its display, so you may
## keep them as they are; keep them as short.

flag-hold = HOLD
flag-rel = REL
flag-auto-range = AUTO
flag-min = MIN
flag-max = MAX
flag-avg = AVG
flag-low-battery = LOW BAT
# The meter warns of a hazardous voltage.
flag-hv-warning = HV!
# The highest peak held.
flag-peak-max = P-MAX
# The lowest peak held.
flag-peak-min = P-MIN
# A test lead is in the wrong socket for the mode.
flag-lead-error = LEAD ERR
# Comparator mode: the reading is checked against limits.
flag-comp = COMP
flag-record = REC
# Low-impedance voltage mode.
flag-loz = LoZ
# The reading is not valid.
flag-void = VOID

## A link's full name, for text with room to spell it out: an error, or a
## hover where the status line shortened it.

link-name-usb-cable = câble USB
# A meter with Bluetooth built in: there is no adapter to name.
link-name-bluetooth-link = liaison Bluetooth
# The Bluetooth adapter plugged into the meter.
link-name-bluetooth-adapter = adaptateur Bluetooth

## The name of a main reading that is one part of what the meter measures,
## beside sub-values that are the others: in the graph's legend and the
## statistics.

# The DC component, beside an AC one.
main-label-dc = DC
# The first thermocouple, beside a T2 one.
main-label-t1 = T1
# The second thermocouple, beside a T1 one.
main-label-t2 = T2
# The difference from a reference, beside Reference and Absolute.
main-label-relative = Relatif
# The highest peak, beside a Peak Min one.
main-label-peak-max = Crête max.

## The sub-value under a scaled reading: the meter's own reading before
## the scale. Below the reading, in the graph's series chips and its
## legend.

raw-label = Brut

## The device lists' heading for the simulated meters. Real brands are
## names and stay as they are.

brand-simulated = Simulés
