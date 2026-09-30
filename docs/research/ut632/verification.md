# UT632 verification

Open checks for the UT632 and UT632N, researched from UNI-T's software but not
implemented; no UT632 has been captured and no issue is open yet.
Checks that span families are in the [verification backlog](../../verification-backlog.md).

## Wire format

Each capture: the UT632 on its CH9325 cable, running
`RUST_LOG=dmm_lib=trace dmm-cli --device ut804 debug --count 0` for a few
seconds with a reading on the LCD; attach that log. The UT804 path leaves the
bridge at 2400 baud and logs every UART byte as `CH9325 RX` before parsing;
its start-up sends one 0x5A trigger byte, whose effect on a UT632 is unknown.

- The payload encoding — LCD segments, as the [UT804.exe twin](reverse-engineered-protocol.md#3-the-ut804exe-twin--vendor)
  decodes for the UT60A/B/C, or not; UT803.exe holds no decoder. Decides the
  whole parser. Needs that capture.
- The frame length — 14 bytes with high nibbles 1-E is [DEDUCED](reverse-engineered-protocol.md#4-wire-format-as-far-as-it-follows);
  UT803.exe checks none. Decides the extractor (a 14-byte one,
  `extract_frame_fs9721`, is recoverable from 1693093 —
  [candidates](../new-device-candidates.md#uni-t-ut632--ut632n)). Needs that capture.
- The line format, data bits and parity ([§1.2](reverse-engineered-protocol.md#12-uart-parameters--vendor))
  — the app's 8N1 port settings do not show it. Decides the transport's UART
  setup. Needs that capture.
- Whether the meter sends unprompted or needs a button press first — decides
  whether connecting needs a step from the user. Needs that capture, with and
  without the send button.
- Whether the UT632N sends what the UT632 does — decides one entry or two.
  Needs that capture on a UT632N beside a UT632.

## Vendor sources

- Whether UNI-T's general-purpose PC software ([backlog](../../verification-backlog.md#vendor-sources-not-yet-read))
  drives a UT632 and how it decodes it — could settle the payload before a
  capture.
