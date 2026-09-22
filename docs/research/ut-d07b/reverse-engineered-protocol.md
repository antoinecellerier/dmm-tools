# UT-D07B Bluetooth adapter: Reverse-Engineered Protocol Specification

What the UNI-T UT-D07B does on the wire. It is not a meter: it is a
battery-powered Bluetooth LE module that plugs into the meter's data
socket and carries that meter's bytes over a BLE transparent-UART service.
Nothing it carries is its own — the frames are the meter's, documented in that
family's spec (`../ut61-family/reverse-engineered-protocol.md` for the UT61+
line).

Based on:
- Live GATT enumeration of our own UT-D07B over BlueZ 5.87, 2026-09-22:
  services, characteristics, flags, advertising data and a first frame
  exchange with our UT61E+ behind it
- The archived iDMM2.0 Android app (`references/idmm2/`), read 2026-09-21,
  for the two BLE UART service sets UNI-T's own client knows
  (`../new-device-candidates.md`)
- UNI-T's UT-D series accessory page,
  https://meters.uni-trend.com/product/ut-d-series/, read 2026-09-22, and the
  Chinese accessory pages https://meters.uni-trend.com.cn/content/4374.html
  and https://meters.uni-trend.com.cn/content/4375.html, for the adapter
  models, their radio versions and the meter series each names
- The UT-D07B's printed instructions, the leaflet shipped with the adapter,
  transcribed by the user 2026-09-22: its LEDs and when it goes to standby
  (§4)

Confidence levels:
- **[HARDWARE]** — seen on our own UT-D07B with our UT61E+ behind it
- **[VENDOR]** — from UNI-T's own app, accessory page or printed instructions
- **[DEDUCED]** — logical inference from the above
- **[UNVERIFIED]** — needs a measurement or another unit to confirm

---

## 1. What the adapter is

The UT-D07B is a transparent bridge [HARDWARE]. Bytes written to its UART RX
characteristic reach the meter's serial line unchanged, and bytes the meter
sends arrive as notifications on its UART TX characteristic — the same stream
the UT-D09 USB cable carries. A host that already speaks a meter's protocol
needs no new framing, no encapsulation and no adapter-level command set.

The adapter cannot say which meter is behind it [HARDWARE]. Its Device
Information service strings are all empty, and its PnP ID names vendor `0x005D`
(ISSC/Microchip), the module maker, not the meter. Identifying the meter is the
host's job, from the frames the meter itself sends.

Power: two 1.5 V R03 (AAA) cells [VENDOR]. Radio: Bluetooth 5.0 low power on
the UT-D07B, 4.0 on the UT-D07A [VENDOR].

## 2. GATT layout

Our unit, 2026-09-22 [HARDWARE]: GAP name
`UT-D07B`, paired and bonded.

| Service / characteristic | Flags | Role |
|---|---|---|
| service `49535343-fe7d-4ae5-8fa9-9fafd205e455` | | ISSC transparent UART |
| `49535343-1e4d-4bd9-ba61-23c647249616` | notify | UART TX, meter → host |
| `49535343-8841-43f4-a8d4-ecbe34729bb3` | write, write-without-response | UART RX, host → meter |
| `49535343-aca3-481c-91ec-d85e28a60318` | write-without-response, notify | ISSC control; not needed (§3) |
| `49535343-6daa-4d02-abf6-19569aca69fe` | read, write | reads `a0 00 40 01`; not needed |
| service `0000d0ff-3c17-d293-8e48-14fe2e4da212`, chars `ffd1`…`ffe0` | | ISSC configuration; not needed |

`0000ff12` appears in the advertising data only — there is no GATT service by
that UUID on this unit [HARDWARE]. It is a scan hint, not a data path. The
`0000ff01`/`ff02`/`ff12` service set the iDMM2.0 app also carries belongs to
UNI-T's native-BLE meters, which have no adapter [VENDOR].

On connect, BlueZ 5.87 logs
`profiles/gap/gas.c:read_ppcp_cb() GAS PPCP: Invalid Connection Parameters
values` [HARDWARE]: the adapter's Peripheral Preferred Connection Parameters
characteristic (`0x2A04`) holds values BlueZ rejects. The link works anyway —
the host's own parameters are used instead.

## 3. Bringing the link up

Subscribe and go [HARDWARE]. With notifications enabled on the UART TX
characteristic, a frame written to the UART RX characteristic reaches the
meter: writing the UT61+ Get Name frame `AB CD 03 5F 01 DA` made the meter beep
and a checksummed `AB CD` frame came back as a notification. Nothing is written
to the ISSC control characteristic first, and no configuration service is
touched.

The adapter puts one frame of its own on the stream [HARDWARE]:
`AB CD 06 AA AA 6E 67 03 A7` — a UT61+-style frame (header, length 6,
big-endian sum checksum) with the payload `AA AA 6E 67`. It comes once when a
link comes up, before the meter's first reply, and about once a second while
the meter is silent: with the meter switched off and the adapter on its own
batteries, the link stayed up and the frame kept coming. It is the adapter's,
not the meter's — over USB the meter never sends it — and what `6E 67` encodes
is not known. A host has to drop it before the meter's parser sees it.

Pairing is not required [HARDWARE]: with the bond removed from the host, a
scan found the adapter, the connection and the subscription went through and
the meter answered. A paired adapter is one the host lists without scanning,
which is what makes a reconnect faster; that is its only effect.

## 4. The adapter sleeps

An adapter that cannot be reached is asleep, not broken [HARDWARE]. After a
link dropped, its LEDs went dark and it stopped advertising; connection
attempts then failed with `le-connection-abort-by-local` until the adapter
was switched off and on with its own power switch, which brought it back.
The fix is at the adapter, not at the host: nothing the host sends reaches
an adapter that is not advertising. Toggling the meter's data transmission
does not wake it either: tried on our UT61E+ with the adapter in standby, the
adapter stayed dark (2026-09-22) [HARDWARE].

The printed instructions describe the states and the standby rules
[VENDOR]. The blue LED flashes once every 3 s before a connection and twice
every 1.5 s once connected; the red LED lights for 1 s after power-on and
flashes when the battery is below 2.3 V (±0.2 V). Both LEDs are off in
standby, which the adapter enters when its battery is below 2.0 V (±0.2 V),
when no connection is made within 5 minutes of power-on, when communication
with the phone fails within 5 minutes of connecting, or when data to the
phone is interrupted for more than 5 minutes. None of these timings has been
measured on our unit [UNVERIFIED]. The dark adapter after a dropped link
above fits the standby they describe. The leaflet gives no way out of
standby; the power switch is the one seen to work (above).

Switching the meter off does not drop the link [HARDWARE]: the adapter has its
own batteries, keeps the connection and sends its heartbeat (§3). Switching
the meter back on resumes the meter's replies on the same link. Whether the
heartbeat counts as data for the 5-minute rule, or the link ends in standby
5 minutes after the meter goes quiet, is not measured [UNVERIFIED].

An awake adapter is not always heard by a scan [HARDWARE]. With the adapter
blinking its waiting pattern, paired to this host and after its first
disconnect of the session, BlueZ 5.87's default discovery — BR/EDR and LE
interleaved, `bluetoothctl scan on` — reported it once in 10 s, where an
LE-only discovery (`bluetoothctl scan le`) reported it 15 times. A 3 s
interleaved scan, the only mode btleplug 0.13.2 asks BlueZ for, missed it in
most runs. A connect by address, which BlueZ runs as an LE-only connection
attempt, reached it every time, within 10 s. How often a freshly powered
adapter advertises is not measured.

## 5. Link parameters

| Value | Reading | Confidence |
|---|---|---|
| ATT MTU | 247 bytes, as negotiated with BlueZ 5.87 | [HARDWARE] |
| Connection interval, slave latency, supervision timeout | not measured — BlueZ does not expose them to a client | [UNVERIFIED] |
| One UT61+ poll (write, reply) | about 0.63 s with an unacknowledged write, 0.8 s with an acknowledged one, once the link has settled; about 0.1 s over USB | [HARDWARE] |
| First seconds of a fresh link | polls take up to 2 s while bluetoothd reads the Device Information characteristics (model, serial, firmware strings) | [HARDWARE] |

A 19-byte UT61+ reading arrives as one notification on this MTU. The poll
time is well above the meter's own reply time, so the connection interval or
the adapter's UART turnaround is what bounds the rate; which one is
[UNVERIFIED].

## 6. Which meters sit behind it

From UNI-T's accessory pages, read 2026-09-22 [VENDOR]:

| Adapter | Radio | Application, as UNI-T lists it |
|---|---|---|
| UT-D07A | Bluetooth 4.0 | UT71, UT171, UT181 series ([global](https://meters.uni-trend.com/product/ut-d-series/)); UT513B, UT513C, UT513D, UT512E insulation testers ([content/4374](https://meters.uni-trend.com.cn/content/4374.html), and [content/4375](https://meters.uni-trend.com.cn/content/4375.html), which spells it UT-A07A) |
| UT-D07B | Bluetooth 5.0 low power | UT61+ Series, UT161 Series, UT171 Series, UT181 Series ([global](https://meters.uni-trend.com/product/ut-d-series/)); the UT61+ series ([content/4375](https://meters.uni-trend.com.cn/content/4375.html)) |

The two pages already disagree on how much each adapter covers, the US mirror
may name more again, and the adapter being transparent (§1), any meter with the
matching socket can physically sit behind one. The only meter this document's findings were read
with is our own UT61E+ [HARDWARE]. Whether the UT-D07A carries the same ISSC
service set is unknown — it has not been seen.

## Implementation Notes

Facts about the adapter that any decoder has to live with:

- The link carries the meter's bytes and nothing else. There is no
  adapter-level framing, length field or checksum wrapped around them, so a
  decoder written for the cable works unchanged.
- Notifications are ATT-sized chunks, not frames. One notification may carry
  part of a frame or more than one frame, so the reader has to buffer and let
  the meter's own framing find the boundaries — the same way the HID bridges'
  fixed-size reports are handled.
- Writes are bounded by the negotiated ATT MTU minus three bytes; a longer
  write is rejected by the peer rather than split for the sender.
- Silence is the normal state between a command and its reply, and is also
  what a dropped link looks like: the two are told apart by asking the host
  stack whether the peripheral is still connected, not by the stream.
- An unreachable adapter is a sleeping adapter (§4), so a host that finds
  nothing in range should say so rather than report a missing meter. A scan
  that does not hear it is not that test: an awake adapter is missed by an
  interleaved scan, and a connect by address is what tells (§4).
- The adapter's heartbeat (§3) is on the stream in the meter's framing, so
  a host drops that exact byte sequence before the meter's parser runs; a
  silent meter behind a live adapter looks like a link with heartbeats and
  no replies.
- The adapter identifies itself and not the meter (§1), so which meter is
  behind it has to be settled from the frames, exactly as on a USB cable.
