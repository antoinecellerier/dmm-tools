# UT8803 verification

Open checks for the UT8803 and UT8803E, issue
[#3](https://github.com/antoinecellerier/dmm-tools/issues/3).
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). No UT8803 has been
read yet: everything below comes from UNI-T's uci.dll and manuals.

## First report

- A few seconds of `RUST_LOG=dmm_lib=trace dmm-cli --device ut8803 debug --count 0`
  in DC V, then `capture --unverified` — settles most items below at once. (#3)

## Framing and stream

- A real frame through `frame::extract_frame_ut8803`: 21 bytes, `AB CD`, type
  `02`, the alternating sum big-endian ([§2.3](reverse-engineered-protocol.md#frame-format))
  — decides the extractor. Needs the first report's trace. (#3)
- That the meter streams with nothing sent (§2.4, §2.5) — decides `init`
  sending nothing. Needs the first report's trace. (#3)
- What frame byte 2 carries: zero, a length, a sequence ([§2.3](reverse-engineered-protocol.md#payload-layout))
  — decides whether the parser reads it. Needs traces in several modes. (#3)
- Frames per second against the manual's 2-3 LCD updates (§2.5) — settles
  §8's sampling-rate row. Needs about 10 s of trace. (#3)
- Whether a stray `0x5A` changes the stream (§1.3) — settles §1.3's last
  sentence; the driver sends none. Needs a hidapi script or a dev build. (#3)

## Modes and ranges

- Each dial position's mode byte against the 23 codes of
  [§4.2](reverse-engineered-protocol.md#42-ut8803ut8803n-position-coding-high-d8-d15)
  — decides `POSITION_TABLE`. Needs every capture step. (#3)
- The range byte, `0x30`-`0x36`, per mode and range (§2.3, §5) — decides
  `display_unit`'s prefixes. Needs readings on several ranges of each
  function, noting the LCD's unit. (#3)

## Display

- What the five display bytes hold: ASCII digits, point, spaces, NULs, `OL`,
  or binary (§2.3, payload layout) — decides the parser's display check and float parse.
  Needs the first report's trace and `ohm` (OL). (#3)
- Whether a negative reading spells `-` in the display field too: if so the
  value double-negates, if not `display_raw` stays unsigned, unlike the
  UT8802's — decides the sign handling. Needs `dcv_negative`. (#3)

## Flags

- HOLD, REL, MIN, MAX, AUTO (inverted), OL and sign against §2.3's bit table —
  decides the bits `parse_measurement` reads. Needs `dcv_negative`, `ohm`, and
  HOLD, REL, MAX/MIN and RANGE pressed: no capture step covers them. (#3)
- Whether flag bytes 14-18 carry a `0x30` prefix, as the range byte does —
  decides `POSSIBLE_ASCII_PREFIX`, which keeps bits 4-5 unreported. Needs any
  trace. (#3)

## Detection

- The UT8803 detection row, and the Linux HID check at `--interval-ms 2000`:
  in the [backlog](../../verification-backlog.md#device-auto-detection) and
  its [streaming section](../../verification-backlog.md#reading-and-streaming).

## Vendor sources

- UT8803E Software V1.1's `UT8803.msi`, inside an InstallShield `Setup.exe`
  we could not unpack (approach doc) — hash-compare any uci.dll in it with SDK
  V2.3's; decides whether the spec covers the shipping app. Needs an
  InstallShield extractor.
