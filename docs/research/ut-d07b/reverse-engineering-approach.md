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

2. **The archived iDMM2.0 Android app** (`references/idmm2/`), read
   2026-09-21 for the device survey in `../new-device-candidates.md`. UNI-T's
   own Bluetooth client carries two BLE UART service sets — the ISSC/Microchip
   `49535343-…` group and `0000ff01`/`ff02`/`ff12` — which is what said in
   advance that the adapters use the ISSC group and the native-BLE meters the
   other. No further analysis of the app was needed once the adapter was
   enumerated. Tagged [VENDOR].

3. **UNI-T's accessory pages**, read 2026-09-22: the global UT-D series page
   (https://meters.uni-trend.com/product/ut-d-series/) and the Chinese pages
   for the UT-D07A (content/4374) and UT-D07B (content/4375). Model list,
   radio version, battery type. Tagged [VENDOR]. They disagree with each other
   on coverage, which the spec records rather than resolves.

4. **Our UT61+ family work** — `../ut61-family/reverse-engineered-protocol.md`
   for the Get Name frame used as the probe, and for the statement that the
   meter's Bluetooth and USB paths carry one protocol.

### Avoided (clean-room boundary)

- **No community BLE implementation was read**: no sigrok decoder, no phone-app
  teardown by a third party, no forum thread, no GitHub project for these
  adapters or for the ISSC transparent-UART profile. Everything in
  `reverse-engineered-protocol.md` comes from the unit on the bench, UNI-T's
  own app and UNI-T's own pages.
- The boundary has not been opened for this adapter. If it is, the finding goes
  in a labelled cross-reference section at the end of the spec, with the date,
  the way the other families' docs do it.
- `btleplug` and BlueZ are host-side Bluetooth stacks, not implementations of
  anything UNI-T does; reading their APIs is not a boundary crossing.

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
4. **Leave unmeasured numbers unmeasured.** MTU, connection interval and
   round-trip latency are marked `[UNVERIFIED]` rather than filled in from the
   BLE defaults — the transport exposes them at runtime, so they will be read
   off a real session instead.
