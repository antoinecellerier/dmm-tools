### What 'dmm-cli debug' prints about the link before the raw frames. The
### frame lines on the standard output are for protocol work and stay as
### they are; so does the link detail after each label, which the library
### gives in English.

## Labels opening a dim line on the terminal's error stream, the link detail
## following after a space. Keep the colon, or your language's equivalent.

# The kind of link, e.g. CP2110 or Bluetooth.
debug-transport-label = transporte:
# The USB chip's or Bluetooth device's identity, e.g. "CP2110 part=0x0a
# firmware=1".
debug-bridge-label = puente:
# The link's counters, e.g. "TX FIFO: 0 bytes, RX FIFO: 0 bytes".
debug-status-label = estado:

## A frame that didn't arrive or didn't decode, in red, after its number in
## brackets.

debug-timeout = error: tiempo de espera agotado
# $error is why, e.g. "checksum mismatch".
debug-frame-error = error: { $error }
