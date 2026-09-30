# VC-880 verification

Open checks for the Voltcraft VC-880 and VC650BT, issue
[#13](https://github.com/antoinecellerier/dmm-tools/issues/13).
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). No VC-880 or VC650BT
has been read yet: every item waits on a first report.

## First report

- `dmm-cli --device vc880 capture --unverified`, PC pressed first — settles
  most items below; report sub-steps with `status: error` and any `remote
  control unreliable` line. Needs a VC-880 on its CP2110 cable. (#13)

## The VC650BT

- A VC650BT's capture, its installer identical to the VC-880's ([approach doc §1](reverse-engineering-approach.md#1-voltsoft-installer-conrad-item-124609124411))
  — decides whether it stays on the VC-880 parser. Needs a VC650BT and
  `dmm-cli --device vc650bt capture --unverified`. (#13)

## Link and streaming

- Whether it streams unasked, only once PC is pressed ([§6](reverse-engineered-protocol.md#6-communication-model----vendor))
  — decides `init`'s no-trigger path. Needs `dmm-cli --device vc880 debug`
  before and after pressing PC. (#13)
- Frames per second against the manual's 2-3 ([§4](reverse-engineered-protocol.md#4-live-data-payload-type-0x01-39-bytes----vendor))
  — decides the settle budget, which counts reads (`SETTLE` in `vc880.rs`).
  Needs a few seconds of `debug`. (#13)
- The vendor's parity 2, even by [ut171 §2.3](../ut171/reverse-engineered-protocol.md#23-uart-config-report-0x50-layout----vendor), none by [§1](reverse-engineered-protocol.md#1-transport-layer----vendor)
  — decides whether `Cp2110::open`'s 8N1 fits a VC-880. Needs Rev 2.4's
  line settings, or a capture with the bridge set to 8E1. (#13)

## Frames and display

- 39-byte LiveData frames, AB CD and a BE16 sum ([§2](reverse-engineered-protocol.md#2-frame-format----vendor),
  [§3](reverse-engineered-protocol.md#3-message-types----vendor))
  — decides the frame reader. Needs any capture. (#13)
- The function byte per mode, 0x00-0x12 ([§4.1](reverse-engineered-protocol.md#41-function-codes-byte-4----vendor))
  — decides `FUNCTION_TABLE`'s labels against the LCD. Needs every mode
  step. (#13)
- The range byte per function, 0x30 plus an index ([§4.2](reverse-engineered-protocol.md#42-range-tables-byte-5----vendor--known))
  — decides the range tables. Needs the mode steps' range sub-steps. (#13)
- The range byte in ACV low-pass, which the vendor never reads (§4.2) —
  decides whether the voltage table labels it (`LOW_PASS`). Needs `lpf`. (#13)
- The main display against the LCD, and what an overload or a blank reading
  sends: `OL`, dashes, OL1 ([§4](reverse-engineered-protocol.md#4-live-data-payload-type-0x01-39-bytes----vendor))
  — decides `parse_value`'s fallbacks. Needs `ohm` with the leads open. (#13)
- Sub values 1 and 2 and the 3-byte bar (§4): format and content, a guess
  in the [cross-reference](reverse-engineering-approach.md#cross-reference-against-pylablib-phase-3)
  — decides whether they become sub-values. Needs MAX/MIN, REL. (#13)

## Flags

- Every named bit of status bytes 30-36 ([§4.3](reverse-engineered-protocol.md#43-status-flag-bytes----vendor))
  — decides the flags reported. Needs HOLD, REL, MAX/MIN/AVG, manual range
  and low battery. (#13)
- Whether bytes 30-35 carry a 0x30 prefix like the range byte — decides
  `POSSIBLE_ASCII_PREFIX` and which bits report as unrecognised. Needs any
  capture's raw frames. (#13)
- Sign1, byte 30 bit 2, against the display's `-` (§4.3) — decides where the
  sign comes from; a mismatch is reported. Needs `dcv_negative`. (#13)
- AVG, byte 31 bit 1, lit only on the AVG step (§4.3) — decides the AVG
  flag. Needs `set minmax avg`, which reads the bit back. (#13)

## Dial and modes

- Each position's functions ([§4.4](reverse-engineered-protocol.md#44-rotary-positions-and-shiftsetup-sub-functions----manual))
  — decides `DIAL`. Needs `get mode` on every position (V~, Lo and
  capacitance print "no switchable modes"); report any mismatch. (#13)
- The order SHIFT/SETUP walks a position's functions (§4.4) — decides the
  table's order. Needs `set mode` under `RUST_LOG=dmm_lib=debug` (its
  `cycle: pressing` lines), or `command select` then `read --count 3`. (#13)
- Whether SHIFT/SETUP on V~ switches anything, the manual's §8b text
  against its figure (§4.4) — decides whether AC V keeps its own position
  in `DIAL`. Needs the V~ position. (#13)
- Whether V⎓ below 400 mV reports 0x02, DC mV (§4.4) — decides the 0x02
  overlap between two positions. Needs V⎓ with a small DC input and
  `read --count 1` or `debug`. (#13)
- The settle budget, untuned: `SHIFT/SETUP did nothing in <mode>` while the
  display did change means it is too tight — decides `SETTLE`. Needs
  `set mode`; report the mode. (#13)

## Range

- Whether one RANGE press (0x46) steps one rung, 0x47 back to auto ([§5](reverse-engineered-protocol.md#5-commands-host--meter----vendor))
  — decides whether presses could be counted. Needs `set range` per rung
  under `RUST_LOG=dmm_lib=debug`, then `set range auto`. (#13)
- A rung the meter's own RANGE button reaches that `get range` leaves out,
  or a press that changes the function — decides the range tables. Needs
  `get range` and `set range` with a stable input. (#13)

## HOLD, REL and MAX/MIN/AVG

- The order 0x49 walks MAX, MIN and AVG, and 0x43 leaving them (§5) —
  decides `flag_states`' order. Needs `set minmax max`, `min`, `avg` and
  `off`, each with the LCD's badge. (#13)
- Whether every mode takes HOLD and REL — decides which modes offer them.
  Needs `set hold on`/`off` and `set rel on`/`off` on two or three
  positions, under `RUST_LOG=dmm_lib=debug`. (#13)
- Which buttons a held meter drops: a mode or range walk presses HOLD off
  and resends — decides that path. Needs `set hold on`, then `set mode` or
  `set range`. (#13)

## Commands

- Whether `light` (0x4B) toggles the backlight (§5) — decides the command.
  Needs `dmm-cli --device vc880 command light`. (#13)

## Detection and the capture list

- What detection's `0x5F` probe does to a VC-880, and the VC-880 detection
  row: [Device auto-detection](../../verification-backlog.md#device-auto-detection).
- The VC-880 capture list's gate steps, still split by other steps:
  [Capture tool](../../verification-backlog.md#capture-tool).

## Vendor sources

- Conrad's VC880 Protocol Rev 2.4 (`references/vc880/protocol/`), read from
  rendered pages, its text layer broken — into the spec as [VENDOR-DOC];
  decides which Voltsoft-only [VENDOR] and [INFERRED] rows hold, parity too.
