# EEVblog 121GW verification

Open checks for the EEVblog 121GW, issue [#32](https://github.com/antoinecellerier/dmm-tools/issues/32).
What real meters have confirmed is tagged `[HARDWARE]` in the [spec](reverse-engineered-protocol.md);
checks that span families are in the [verification backlog](../../verification-backlog.md).
Community sources (spec §15) narrow many of these, firmware 1.02's only for
that ASCII-era firmware; each still wants a current meter.

## First report

- A few seconds of `RUST_LOG=dmm_lib=trace dmm-cli --device 121gw debug` in
  DC V, with the firmware version the meter shows on boot — settles most items
  below at once. Needs any 121GW. (#32)
- A first confirmed report — gates the spec tables, which the manual carries
  (p.17-23). (#32)

## Advertising and GATT

- The advertised name: "121GW", seen on a community meter, or "Bluegiga",
  which EEVblog's app also accepts ([§2](reverse-engineered-protocol.md#2-advertising-and-gatt),
  §15.4) — decides `devices.rs`'s Bluetooth name. Needs a passive scan. (#32)
- Whether the characteristic notifies as well as indicates, and which write
  types it takes (§2, §15.4) — settles §2's Notify and Write type rows. Needs
  GATT discovery and any `key_*` step. (#32)

## Packets

- 19 bytes per indication, or bytes 1-18 without `F2` as community clients
  saw ([§4](reverse-engineered-protocol.md#4-framing), §15.4) — decides the
  shapes `packet.rs` takes. Needs the first-report trace. (#32)
- Packets per second on current firmware against the display's 5 updates:
  about 2 on a 2022 meter, 4 on firmware 1.02 (§3, §15.4) — settles §3's
  rate. Needs the first-report trace. (#32)
- What byte 14 bit 5 means, fixed at 0 by V2 and set on real meters
  ([§15.3](reverse-engineered-protocol.md#153-disagree) D4) — decides
  `packet.rs`'s reserved bits. Needs captures across modes. (#32)

## Main display

- The diode 3 V resolution: 0.1 mV (both apps, firmware 1.02) or 1 mV (manual
  p.20) ([§6.2](reverse-engineered-protocol.md#62-ranges)) — decides
  `tables.rs`'s diode row. Needs a diode across the leads, the LCD noted. (#32)
- The top capacitance range: 9999 µF (p.19, both apps, firmware 1.02) or
  10.00 mF (p.71) (§6.2, §15.3 D2) — decides `tables.rs`'s capacitance row.
  Needs a capacitor above 1000 µF. (#32)
- How far past full scale a range reads before OFL or a range change, and the
  value bytes under OFL, `00 00` in one capture (§6.2 Counts, §6.3, §15.5) —
  settles §6.2's counts note. Needs `ohm_ol` and a reading near full scale. (#32)
- DC+AC V as mode 2 with AC/DC code 3, as firmware 1.02 sends it
  ([§6.1](reverse-engineered-protocol.md#61-mode-codes), §15.4) — decides
  `decode.rs`'s "AC+DC V" name. Needs `acdcv`. (#32)
- Whether byte 15 bit 7 and byte 16 bit 7 repeat byte 6's °C/℉ bits, with no
  community answer ([§6.4](reverse-engineered-protocol.md#64-temperature)) —
  decides whether they stay silent. Needs `temp` and `temp_unit`. (#32)

## VA modes

- How µVA (13/22) is reached, MODE on µA in firmware 1.02, and how mVA/VA
  picks DC or AC (§6.1, §15.4) — decides the `dcuva`, `acuva`, `dcva`, `acva`,
  `dcmva` and `acmva` steps, which ask with a skip. (#32)
- VA ranges 0-3 as current × voltage (§6.2); operand units UEi's V/µA/mA,
  EEVblog's mV (§7.1); fw 1.02's 10 A in ACVA/DCVA ranges 2-3 (§15.4), perhaps
  A, not mA — decides `decode::current_unit`. Needs each VA range. (#32)
- Whether the secondary display switches operand every packet or on a slower
  clock (§15.4) — at a longer sample interval a reading carries whichever one
  its packet showed. Needs a VA capture at `--interval-ms 2000`. (#32)

## Secondary display

- Bytes 9-12 of a blank secondary display, all zero in firmware 1.02
  ([§7.1](reverse-engineered-protocol.md#71-sub-mode-byte-9), §15.4) — decides
  `decode::secondary`'s no sub-value. Needs `setup_blank`. (#32)
- Burden voltage, code 150, in mV (UEi) or V (EEVblog's LCD), and the logging
  interval, code 190, in s or ms (§7.1) — decides `decode::classify`'s 150
  unit. Needs `burden`, and SETUP's `In x` item. (#32)
- What tells the paired and tripled sub codes apart (101 from 100, 136/137
  from 135, …), none sent by firmware 1.02 (§7.1, §15.4) — decides which
  `decode::classify` keeps. Needs the `setup_*` steps. (#32)
- When the secondary display sets Hz and k: seen in AC mV, k from frequency
  range 2 in firmware 1.02 ([§7.2](reverse-engineered-protocol.md#72-sub-range-byte-10),
  §15.4) — decides its frequency unit. Needs `acv` with a signal of a few kHz. (#32)

## Bar graph and annunciators

- The bar's 0-25 scale against the range, its sign on a negative reading
  and the 0~150 bit, set only at scale 1000 in firmware 1.02 ([§8](reverse-engineered-protocol.md#8-bar-graph-bytes-13-14),
  §15.3 D1, §15.4) — settles §8; the driver reads no bar. Needs `dcv_negative`. (#32)
- MIN/MAX beside "1ms" lit: 1 or 2 for the max and min peak, as firmware 1.02
  sends them ([§9](reverse-engineered-protocol.md#9-annunciators-bytes-15-17),
  §15.4) — decides `decode::flags`' `peak_max`/`peak_min`. Needs `peak`. (#32)
- ↙ (byte 16 bit 5), firmware 1.02's danger icon, named by no vendor source
  (§9, §15.4) — decides whether it becomes `hv_warning`. Needs a reading that
  lights the LCD's hazard bolt. (#32)
- TEST, byte 17 bits 1-0 (UEi: secondary AC/DC), the MEM values and MIN/MAX
  5-7, unused by firmware 1.02 (§9, §15.4) — decides which stay silent and
  which MEM values set `record`. Needs `record`, `min_max_all`, `acv`. (#32)

## Keys and commands

- Whether current firmware acts on the `F4` key frames and echoes them, as
  1.02 does ([§11.1](reverse-engineered-protocol.md#111-key-press-f4), §15.4)
  — decides `mod.rs`'s `KEYS` and reading replies. Needs the `key_*` steps. (#32)
- Whether a dial turn brings back auto-ranging; the manual says only that RANGE
  leaves it (p.33) — decides the step's expectation. Needs `range_auto`. (#32)
- Whether the meter acts on the `F8` clock set or code `09`: firmware 1.02
  sets the clock and maps `09` to no key (§11.2, §15.4) — settles §11; the
  driver sends neither, nor does any capture step. Needs a dev build. (#32)

## Firmware and identity

- Which firmware first sent the binary packet ([§1](reverse-engineered-protocol.md#1-model-and-firmware)), and which each ASCII
  format ([§12](reverse-engineered-protocol.md#12-earlier-firmware-formats)) — decides the older-firmware hint's wording.
  Needs meters on older firmware, their versions noted. (#32)
- Bytes 1-4 as a BCD calibration year-month and the Multimeter ID, as in
  firmware 1.02, and months 10-12 ([§10](reverse-engineered-protocol.md#10-identity-bytes-1-4),
  §15.4) — settles §10. Needs the ID changed in SETUP (manual p.63). (#32)
- Whether a Bluetooth connection holds off auto power-off; in firmware 1.02
  it does not ([§3](reverse-engineered-protocol.md#3-bring-up), §15.4) —
  settles §3. Needs the meter left streaming 30 minutes with APO on. (#32)

## Detection

- What the UT61+, UT181A and UT171 probes do to a 121GW, the 121GW detection
  row, and the Bluetooth search and detection it shares with the UT-D07B: in
  the [verification backlog](../../verification-backlog.md).
