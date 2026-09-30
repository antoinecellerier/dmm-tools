# UT171 verification

Open checks for the UT171A, UT171B and UT171C, issue
[#4](https://github.com/antoinecellerier/dmm-tools/issues/4).
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). No UT171 has been read
yet: every item waits on a first report.

## First report

- `dmm-cli --device ut171 capture --unverified`, with Communication ON set on
  the meter — one measurement frame closes the framing ([§3.1](reverse-engineered-protocol.md#31-general-structure))
  and settles most items below. Needs any UT171 and its USB cable. (#4)

## Frames

- The extended frame: which modes send it, its extra bytes and the third
  float ([§5.2](reverse-engineered-protocol.md#52-extended-frame-27-bytes-length--0x17))
  — decides whether the parser reads it. Needs `vac`, `vacdc`. (#4)
- Status2 and the byte after it ([§5.1](reverse-engineered-protocol.md#51-standard-frame-21-bytes-length--0x11)),
  capture-deduced — decides the values the parser stays silent on. Needs the
  DC and AC steps. (#4)

## Flags

- Every flag bit ([§5.3](reverse-engineered-protocol.md#53-flags-byte-offset-5----vendor)):
  HOLD, inverted AUTO, low battery, and bits 0, 1, 3, 4, 5, which no evidence
  backs — decides the flags read and reported. Needs HOLD and RANGE. (#4)

## Modes

- The mode bytes of [§6](reverse-engineered-protocol.md#6-mode-byte-table----vendor),
  13 seen in captures, Duty % (0x10 or 0x0F with a sub-field) and VFC (0x1A)
  deduced — decides `lookup_mode`. Needs every mode step, `duty` and `vfc`. (#4)

## Ranges

- The range byte, raw and 1-based with 0 for auto ([§5.4](reverse-engineered-protocol.md#54-range-byte-offset-8----vendor))
  — decides `lookup_range`. Needs RANGE presses in Hz, capacitance and the
  current modes. (#4)

## Values

- The float's scale ([§5.1](reverse-engineered-protocol.md#51-standard-frame-21-bytes-length--0x11)):
  Ω in kΩ from range 2 and MΩ from 5, capacitance and nS unknown — decides
  `display_unit`. Needs `ohm`, `cap`, `ns` across ranges. (#4)
- The aux float outside V AC and mV AC, where it is the frequency in kHz
  ([§5.1](reverse-engineered-protocol.md#51-standard-frame-21-bytes-length--0x11))
  — decides its label. Needs the capture steps. (#4)

## Commands

- Whether the meter streams without the connect frame once Communication is
  ON ([§4.3](reverse-engineered-protocol.md#43-streaming-commands-and-simple-commands))
  — decides whether `init` must send it. Needs a trace before `connect`. (#4)
- The builder's layout ([§4.1](reverse-engineered-protocol.md#41-command-frame-builder----vendor))
  or §3.1's for its commands — decides every command past connect and pause.
  Needs a USB capture of the vendor app's save, read or delete. (#4)
- The save, stop and query-count IDs (§4.3) and which of 0x51 and 0x52 reads
  what ([§4.5](reverse-engineered-protocol.md#45-read-commands-0x51-0x52----vendor))
  — decides a data-log download. Needs the same capture. (#4)
- A function change's wire form: [§4.7](reverse-engineered-protocol.md#47-mode-transition-commands----vendor)'s
  codes overflow the builder's byte, no cycle key is known — decides `set mode`.
  Needs UT171C.exe's method tables traced, as for the UT181A's SET_MODE. (#4)

## Detection and the adapter

- What detection's probes do to a UT171, and its frames against the UT181A's:
  [Device auto-detection](../../verification-backlog.md#device-auto-detection).
- A UT171 behind the UT-D07A or UT-D07B: in the
  [UT-D07B checks](../ut-d07b/verification.md#meters-behind-the-adapter).

## Vendor sources

- The UT171 series PC software UNI-T uploaded 2023-02-03 ([candidates](../new-device-candidates.md#uni-t-chinese-sites))
  — hash-compare it with the UT171C setup the approach doc traced; decides
  whether the spec covers the current app.
- What still rests on the functions §3.3 retracts (`FUN_00654bf5`,
  `FUN_0065492d`, `FUN_0065478e`): the byte-6 frame type, HOLD bit 7 and the
  header check — decides which rows stay [VENDOR]. Needs the decompile. (#4)
