# UT-D07B Bluetooth adapter: Reverse Engineering Approach

The UT-D07B carries a meter's own bytes (`reverse-engineered-protocol.md` §1),
so there was no wire protocol to reconstruct — only the BLE surface the adapter
presents and what it takes to get bytes flowing through it. That was read off
our own unit.

## Sources Used

### Primary (clean-room RE)

1. **Live GATT enumeration of our own UT-D07B**, 2026-09-22, over BlueZ 5.87
   on this machine, already paired and bonded.
   Services, characteristics and their flags, the advertising data, the Device
   Information strings and the PnP ID, then a frame exchange with our UT61E+
   behind the adapter: subscribe to the UART TX characteristic, write the
   UT61+ Get Name frame to the UART RX characteristic, read the notification
   that came back. Also the adapter's sleep behaviour after a dropped link —
   woken each time with the adapter's own power switch; toggling the
   meter's data transmission left a sleeping adapter dark — and
   the `GAS PPCP` line BlueZ logs on connect. Then, through the transport once
   it ran: the negotiated MTU, the poll time with each write type, and what
   the adapter sends with the meter switched off. Tagged [HARDWARE] in the
   spec.

2. **A `btmon` capture**, 2026-09-22, root on this machine: the kernel's HCI
   trace of a connect, the L2CAP Connection Parameter Update Request the
   adapter sends straight after it, a connection update forced from the host
   with `hcitool lecup`, and the timestamps of the notifications under each.
   This is where the connection interval, the adapter's cadence and the
   polled and streamed rates in §5 come from — BlueZ hands none of them to a
   client. Tagged [HARDWARE].

3. **The archived iDMM2.0 Android app** (`references/idmm2/`), read
   2026-09-21 for the device survey in `../new-device-candidates.md`. UNI-T's
   own Bluetooth client carries two BLE UART service sets — the ISSC/Microchip
   `49535343-…` group and `0000ff01`/`ff02`/`ff12` — which is what said in
   advance that the adapters use the ISSC group and the native-BLE meters the
   other. Decompiled with **jadx 1.5.6** on 2026-09-22 to
   `references/idmm2/jadx-out/` (gitignored) and read again for the start
   command `AB CD 03 5D 01 D8`, which the app sends once and never repeats,
   and for its reading of the adapter's heartbeat as "no meter data". Tagged
   [VENDOR].

4. **UNI-T's accessory pages**, read 2026-09-22: the global UT-D series page
   (https://meters.uni-trend.com/product/ut-d-series/) and the Chinese pages
   for the UT-D07A (content/4374) and UT-D07B (content/4375). Model list,
   radio version, battery type. Tagged [VENDOR]. They disagree with each other
   on coverage, which the spec records rather than resolves.

5. **UT-D07B printed instructions, transcribed by the user 2026-09-22** —
   the leaflet shipped with the adapter: what the blue and red LEDs show,
   and the four conditions that put the adapter in standby. Tagged [VENDOR].

6. **Our UT61+ family work** — `../ut61-family/reverse-engineered-protocol.md`
   for the Get Name frame used as the probe, and for the statement that the
   meter's Bluetooth and USB paths carry one protocol.

### Cross-referenced (clean-room boundary opened 2026-09-22)

Sections 1-6 of `reverse-engineered-protocol.md` were written before any of
this was read: everything in them comes from the unit on the bench, UNI-T's
own app and UNI-T's own pages. The user opened the boundary on **2026-09-22**
for one question — how fast the adapter can be read — and the findings are in
§7 of the spec, marked [COMMUNITY], with the working note in
`references/ut-d07b/analysis/findings/read-rate.md` §8.

Read on 2026-09-22:

1. **[libsigrok](https://github.com/sigrokproject/libsigrok)**, master
   `0bc2487778` (2025-11-20): `src/serial_bt.c` and `src/bt/bt_bluez.c` (the
   BLE serial layer — conn types, what it does at connect, its write type),
   `README.devices` (the UT-D07's ISSC BL79 chip), and
   `src/hardware/uni-t-ut181a/` (the UT-D07A's idle shutdown). Verified
   absences: no UT-D07 backend, no UT61+/UT161/UT60BT driver.
2. **sigrok wiki**: [Bluetooth](https://sigrok.org/wiki/Bluetooth) and
   [UNI-T UT181A](https://sigrok.org/wiki/UNI-T_UT181A). Verified absence: no
   page exists for the UT-D07A, the UT-D07B, the UT61E+ or the UT60BT.
3. **[webspiderteam/Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows)**,
   commit `2b83d9e` (2026-05-01) — C#/WPF BLE client that drives UNI-T meters
   through the UT-D07A/B: `GattMonitor.cs`, `Decoders/DecoderUni_T.cs`,
   `PairingHelper.cs`, `README.md`.
4. **[libreble/multimeter](https://github.com/libreble/multimeter)**, commit
   `e887b0f` (2026-09-14) — Web-Bluetooth PWA: `packages/protocol/src/framing.ts`,
   `docs/protocols/uni-t.md` and `docs/protocols/ut171.md`.
5. **[olegv142/ut61xpy](https://github.com/olegv142/ut61xpy)**, commit
   `ad7b324` (2026-09-20) — Python logger that supports the **UT-D07B**
   directly: `adapters/ut61xp.py`, `README.md` (its 180 ms USB / 800 ms
   Bluetooth figures).
6. **[subsurface](https://github.com/subsurface/subsurface)** `core/qt-ble.cpp`
   — another client of the same Microchip/ISSC UUID set, for how it treats the
   control characteristics.
7. **EEVblog threads**:
   ["Uni-t ut-d07a Bluetooth.."](https://www.eevblog.com/forum/projects/uni-t-ut-d07a-bluetooth/)
   (teardown, the UT181A's ~10 Hz over the adapter, the 500 ms write-gap
   guess), ["Uni-T UT-D07B Bluetooth adapter for DMMs"](https://www.eevblog.com/forum/testgear/uni-t-ut-d07b-bluetooth-adapter-for-dmms/),
   ["New Uni-T UT61 series (UT61e+)"](https://www.eevblog.com/forum/testgear/new-uni-t-ut61-series-(ut61e)/)
   (nothing on the protocol).

### Avoided, also on 2026-09-22

- **Community decompilations of UNI-T's Android app** beyond the code already
  quoted in the projects above, and the vendor files those projects re-host
  (`from_vendor/TestDataModel.java`, `anjian_config.json` in
  [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus)). We have
  our own jadx tree of the APK; a re-hosted copy adds nothing and muddies
  provenance.
- APK mirrors and app stores.
- `btleplug` and BlueZ are host-side Bluetooth stacks, not implementations of
  anything UNI-T does; reading their APIs was never a boundary crossing.

## Methodology

1. **Enumerate before writing anything.** The whole GATT tree was read first —
   every service, every characteristic, every flag — so that the two
   characteristics that matter could be identified by their properties (one
   notify, one write) rather than guessed from a UUID seen elsewhere.
2. **One write, one notification.** The smallest possible exchange settled the
   open question: the meter's Get Name frame, which is six bytes and makes the
   meter beep, so the answer is visible as well as readable. It came back as a
   checksummed frame with nothing written to the control characteristic, which
   is what made "subscribe and go" a fact rather than a hope.
3. **Record what the adapter does when it fails.** The sleep behaviour was met
   by accident, not sought; it is in the spec because a host that cannot see an
   adapter has to tell the user something true about why. Switching the meter
   off under a running read is what showed the `AA AA 6E 67` frame to be the
   adapter's heartbeat rather than a meter frame: it kept coming from a link
   whose meter was off.
4. **Send the start command over both links.** Whether command 0x5D streams
   only means something with the cable's answer beside it: over the adapter it
   brought readings and no ack, over the CP2110 to the same meter the ack and
   nothing else. The pair is what places the command at the adapter rather
   than at the meter; the Bluetooth run on its own would have credited the
   meter's firmware with it.
5. **Move one variable on a live link.** The adapter's parameter request and
   the reading rate were read off the same `btmon` capture, then the host
   forced the interval from 315 ms down to 15 ms on the running link and the
   capture was read again. The readings stayed ~315 ms apart, which is what
   separates the adapter's own cadence from the radio's interval — a rate
   measured at one interval could not have.
6. **Leave unmeasured numbers unmeasured.** MTU, connection interval and
   round-trip latency were marked `[UNVERIFIED]` rather than filled in from
   the BLE defaults, until the transport reported the MTU and the capture
   above read the rest off the air.
