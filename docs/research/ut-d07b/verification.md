# UT-D07A / UT-D07B verification

Open checks for UNI-T's UT-D07A and UT-D07B Bluetooth adapters and the meters
behind them, issue [#25](https://github.com/antoinecellerier/dmm-tools/issues/25).
What real adapters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). Meters with the adapter
built in: the UT60BT and UT202BT, in [ut61-family](../ut61-family/verification.md).

## The UT-D07A

- Its GATT tree, its heartbeat and its answer to 0x5D ([§6](reverse-engineered-protocol.md#6-which-meters-sit-behind-it))
  — decide whether the ISSC profile (`issc.rs`) carries it unchanged. Needs a
  UT-D07A, a UT61+ meter for the 0x5D answer, and #25's first-report runs. (#25)
- An open that fails with "no data … characteristic we recognise": the
  services `bluetoothctl info <address>` then lists decide the profile it
  needs. (#25)
- Readings bunched so often in front of a meter faster than its interval that
  the pairs notice, which needs most readings on time, stays silent — decides
  `watch_pairs`. Needs a UT-D07A and a streaming UT171 or UT181A. (#25)

## Meters behind the adapter

- The UT71 over a UT-D07A: `--device ut71ab --adapter <address>` opens it
  there — a reading decides whether the ut80x `LINKS` gains Bluetooth. Needs a
  UT71 and a UT-D07A. (#25, #22)
- The UT61B+, UT61D+, UT161 series, UT171 and UT181A behind either adapter
  ([§6](reverse-engineered-protocol.md#6-which-meters-sit-behind-it)) — a
  reading and its `dmm-cli list` line confirm each. Needs the meter. (#25)
- 0x5D before a UT171 or UT181A: polled by the adapter, or only their own start
  frame ([§3](reverse-engineered-protocol.md#3-bringing-the-link-up), [§7](reverse-engineered-protocol.md#7-cross-reference-with-community-sources-community))
  — decides the rate they reach. Needs a trace over each adapter. (#25)
- The heartbeat in front of a UT171 or UT181A, the same bytes or others —
  decides `ADAPTER_HEARTBEAT`'s exact match. Needs the same trace with the
  meter switched off. (#25)

## Windows

- Windows 10, which takes no Balanced request (`winrt.rs`): whether readings
  come in pairs and both reach the host — decides the late-readings notice
  there. Needs `RUST_LOG=dmm_lib=trace dmm-cli read --count 60 --interval-ms 0`. (#25)
- The adapter's battery life with the link held at 60 ms against its own
  315 ms ([§5](reverse-engineered-protocol.md#5-link-parameters)) — decides
  whether the open keeps holding it. Needs two battery runs on Windows 11. (#25)
- A Windows PC whose `list` finds nothing while Bluetooth LE Explorer sees the
  adapter: `RUST_LOG=dmm_lib=debug dmm-cli list`'s `saw …` lines say missing,
  unnamed or unmatched — decides the fix in `search.rs`. Needs that PC. (#25)

## macOS

- A first run: CoreBluetooth's permission prompt or a silent refusal, whether
  pairing is needed, and `--adapter` taking the UUID `list` prints — decides
  setup.md's macOS lines. Needs a Mac and the adapter. (#25)
- Whether macOS grants the request ([§5](reverse-engineered-protocol.md#5-link-parameters)) Apple's accessory
  guidelines reject (min 280 ms, 5 s timeout) — decides the late-readings
  notice. Needs a PacketLogger trace of a connect. (#25)

## Connect and reconnect

- The first connect after power-on ([§4](reverse-engineered-protocol.md#4-the-adapter-sleeps)): under WinRT the open calls it
  "Bluetooth link lost", and the setup retry skips the connect — decides a
  connect retry. Needs `dmm-cli info` just after power-on. (#25)
- A BlueZ reconnect that once took 60 s, not 20 — decides whether the open
  waits out BlueZ's teardown. Needs 6a69591 and the current build, off and on,
  under `RUST_LOG=dmm_lib=debug`; bluetoothd's "StartNotify is not allowed". (#25)
- The adapter taken out of range mid-session and brought back — checks that a
  reconnect releases the old link first (`connection.rs`), unit-tested only.
  Needs our adapter. (#25)
- `dmm-cli list` and `info` with the adapter in standby — the paired fallback
  should end in "No meter found" after about 15 s. Needs our adapter. (#25)

## Scan and pairing

- How often a freshly powered adapter advertises ([§4](reverse-engineered-protocol.md#4-the-adapter-sleeps))
  — decides whether the 3 s `SCAN_WINDOW` hears it. Needs `btmon` or
  `bluetoothctl scan le` from power-on. (#25)
- The other OS's pairing as the cause of the stale keys in [§4](reverse-engineered-protocol.md#4-the-adapter-sleeps)
  — settles its [UNVERIFIED]. Needs a dual-boot PC: pair under one OS, then
  connect under the other. (#25)

## Standby

- The leaflet's standby rules ([§4](reverse-engineered-protocol.md#4-the-adapter-sleeps)),
  none timed on our unit — settles §4's [UNVERIFIED]. Needs the adapter left
  waiting past 5 minutes, then connected with no data past 5 minutes. (#25)
- Whether the heartbeat counts as data for the 5-minute rule — decides
  whether a switched-off meter costs the link. Connect, switch the meter off,
  and watch the link and the blue LED past 5 minutes. (#25)

## Readings and commands

- AC+DC V at a sample interval ([§5](reverse-engineered-protocol.md#5-link-parameters)): ticks keeping one component
  far more than the other — decides the stream's nearest-frame pick. Needs our
  UT61E+ in AC+DC V, `dmm-cli read --interval-ms 1000 --count 60`. (#25)
- A button's ack on a warm link against the 1.26 s on a fresh one ([§3](reverse-engineered-protocol.md#3-bringing-the-link-up))
  — decides whether `BLUETOOTH_PRESS_ACK_TIMEOUT` (2.5 s) can shrink toward
  the cable's 1 s. Needs presses a few minutes into a session. (#25)

## The heartbeat

- What `6E 67` encodes ([§3](reverse-engineered-protocol.md#3-bringing-the-link-up)): whether it varies, on a UT-D07A or on
  low cells — decides the exact match in `issc.rs`. Needs heartbeat traces with
  the meter off. (#25)

## Link drops

- Drops while a headset streamed on the same BlueZ controller, none without —
  whether shared-controller load explains them decides setup.md's
  troubleshooting. Needs a streaming headset, then a second controller. (#25)

## Vendor sources

- UNI-T's US site (uni-trendus.com), not read for the adapters' meter lists
  — decides [§6](reverse-engineered-protocol.md#6-which-meters-sit-behind-it)'s table and each family's `LINKS`, which
  follow UNI-T's pages, not the adapter being transparent. (#25)
