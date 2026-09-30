# VC-890 verification

Open checks for the Voltcraft VC-890, issue
[#14](https://github.com/antoinecellerier/dmm-tools/issues/14).
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). No VC-890 has been read
yet: every item waits on a first report.

## First report

- `dmm-cli --device vc890 capture --unverified` — one polled frame confirms
  the [polled model](reverse-engineered-protocol.md#communication-model----vendor)
  and settles most items below. Needs any VC-890 and the UT-D09 cable. (#14)
- A settings sub-step with `status: error`, or the `remote control unreliable
  on this meter` line, in that run — names a command the meter refused.
  Needs the gate steps passed. (#14)

## Frames

- The 66-byte frame and its seven display fields ([Live Data Frame](reverse-engineered-protocol.md#live-data-frame-66-bytes----vendor))
  — decides the parser's offsets and whether sub-displays become aux
  values. Needs the mode steps, LCD noted. (#14)
- The inbound BE16 checksum, inferred ([Frame Format](reverse-engineered-protocol.md#frame-format----vendor))
  — every frame failing it means another scheme. Needs any capture. (#14)
- The remapped function codes and the 6/60/600 ranges ([Function Codes](reverse-engineered-protocol.md#function-codes----vendor))
  — decides `FUNCTION_TABLE` and `range_table`. Needs every mode step. (#14)
- The range byte in ACV LPF, which the vendor never reads ([Range Tables](reverse-engineered-protocol.md#range-tables----vendor))
  — decides whether `lookup_range` keeps 1000 V fixed. Needs `lpf`. (#14)

## Status bytes

- Every bit of bytes 56-61, and whether 56-58 share 59-61's `0x30` prefix
  ([Live Data Frame](reverse-engineered-protocol.md#live-data-frame-66-bytes----vendor))
  — decides the bits read and reported. Needs the mode steps. (#14)
- Sign1 (byte 56 bit 2) against the display's `-` — decides whether the sign
  comes from the bit or the text. Needs `dcv_negative`. (#14)
- Which battery gear, `0x30`-`0x33` (byte 62), goes with the meter's
  low-battery symbol — decides `low_battery`, set on gear 0 alone. Needs the
  `battery` step with a fresh pack and with one the meter flags as low. (#14)
- Byte 63's misplug values past mA input, and Memory_Overwrite (bit 2), which
  the `0-3` nibble reading counts in — decides the misplug mask. Needs each
  misplug warning the meter gives, leads unconnected. (#14)

## VOID with a live reading

- Whether Void (byte 59 bit 3; the manual's "data memory contains no values")
  ever accompanies a live reading — decides whether `flags.void` stays a
  reading flag and the VOID badge's wording. Needs VIEW on an empty memory. (#14)

## Exchanges

- Whether the meter answers a poll only after PC is pressed, as the VC-880
  streams only then — decides the setup text. Needs any VC-890. (#14)
- Whether the meter needs the ack burst or only takes it ([Communication Model](reverse-engineered-protocol.md#communication-model----vendor))
  — decides whether every exchange keeps it. Needs a hidapi script or dev
  build without it. (#14)
- A frame of the command's own type after a button press, and one ahead of
  the poll's live frame — decides `ECHOED_COMMANDS` and whether
  `write_button` waits. Needs a raw trace around a press. (#14)
- One GetDeviceID exchange returning the name, where the vendor tries up to
  10 times — decides whether `get_name` retries. Needs a connect under
  `RUST_LOG=dmm_lib=debug`, its `device name` line. (#14)

## Mode switching

- `dmm-cli --device vc890 get mode` on every position against the
  [dial table](reverse-engineered-protocol.md#rotary-positions-and-shiftsetup-sub-functions----manual),
  V~'s LPF included — decides `DIAL`. Report symbol and list if they differ. (#14)
- The press order the manual never gives: `set mode "<label>"` per entry
  under `RUST_LOG=dmm_lib=debug` (its `cycle:` lines), or `command select`
  then `read --count 3` round each position. Settles the table's order. (#14)
- `SHIFT/SETUP did nothing` while the display did change — decides `SETTLE`
  (no delay, two reads, untuned). Needs the mode and the meter's delay. (#14)

## Ranges

- Whether 0x46 steps one rung at a time: `set range <label>` per rung under
  `RUST_LOG=dmm_lib=debug`, then `set range auto`; the `cycle:` lines give
  the press count, and `<label> never appeared` means it does not. Decides
  the range walk. (#14)
- A rung the RANGE button reaches that `get range` omits, or `the mode
  changed to <mode>` (0x46 moving the function byte) — decides
  `range_table`. Needs a stable input. (#14)

## HOLD, REL and MAX/MIN/AVG

- The order 0x49 walks MAX, MIN and AVG, and whether AVG (byte 57 bit 1)
  lights on its step alone: `set minmax max`, `min`, `avg`, `off`, each with
  the LCD's badge — decides `flag_states` and the `avg` flag. (#14)
- Whether every mode takes HOLD and REL — decides where they are offered.
  Needs `set hold on`/`off`, `set rel on`/`off` on two or three positions. (#14)
- Which buttons a held meter drops — decides whether a walk needs its
  HOLD-off retry here. Needs `set hold on`, then `set mode` or `set range`. (#14)

## Detection

- What detection's probes do to a VC-890, whether it answers `0x5E` on the
  first attempt, and the VC-890 detection row:
  [Device auto-detection](../../verification-backlog.md#device-auto-detection).

## Vendor sources

- Conrad's VC890 Protocol Rev 1.3: p.6 read 2026-09-30; the frame layouts on
  pp.3-5 and 7-8 still to read, from rendered pages — decides any [VENDOR]
  [Live Data Frame](reverse-engineered-protocol.md#live-data-frame-66-bytes----vendor) row they contradict.
- The VoltSoft GUI (`VoltSoft System.exe` / `DeviceClient`), not decompiled
  — holds which battery gear Voltsoft calls low; decides `low_battery`.
