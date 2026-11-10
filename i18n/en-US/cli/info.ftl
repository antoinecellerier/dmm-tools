### What 'dmm-cli info' prints: which meter answered, and the link it
### answered on, on the standard output for a person to read or paste into
### a report. The link details come from the library, in English, except
### common-unavailable, which says one could not be read.

# Detection found the meter, which no option named. $meter is its brand and
# model, e.g. UNI-T UT61E+, in bold.
info-detected = Detected: { $meter }
# $transport is the kind of link, e.g. CP2110 or Bluetooth.
info-transport = Transport: { $transport }
# Indented under info-transport, in place of the chip's identity when asking
# for it failed. $bridge is common-unavailable's text.
info-bridge = Bridge: { $bridge }
# Indented under info-transport, after the chip's identity. $status is the
# link's counters, e.g. "TX FIFO: 0 bytes, RX FIFO: 0 bytes".
info-status = Status: { $status }
