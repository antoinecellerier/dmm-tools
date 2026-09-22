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
- A `btmon` capture of a session on this machine, 2026-09-22: the HCI trace of
  a connect, the adapter's own parameter update, a host-forced connection
  update and the notification timing under each (§5)
- Sessions on the same machine under Windows 11 Pro (build 26200), its
  Intel Wireless Bluetooth radio through WinRT (btleplug 0.13.2), adapter
  never paired there, 2026-09-22: scans, connects by address, a streamed
  read and a button press (§3-5)
- The archived iDMM2.0 Android app (`references/idmm2/`), read 2026-09-21,
  for the two BLE UART service sets UNI-T's own client knows
  (`../new-device-candidates.md`), and decompiled with jadx on 2026-09-22
  (`references/idmm2/jadx-out/`) for the start command it sends and what it
  makes of the adapter's heartbeat (§3)
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
- **[COMMUNITY]** — from a community source, §7 only; never in §1-6

---

## 1. What the adapter is

The UT-D07B is a transparent bridge [HARDWARE]. Bytes written to its UART RX
characteristic reach the meter's serial line unchanged, and bytes the meter
sends arrive as notifications on its UART TX characteristic — the same stream
the UT-D09 USB cable carries. A host that already speaks a meter's protocol
needs no new framing and no encapsulation. It is not quite silent on the
stream, though: it puts a heartbeat of its own there and it acts on the
UT61+ start command itself (§3).

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
characteristic (`0x2A04`) holds values BlueZ rejects. It costs the adapter
nothing: it asks for the parameters it wants over L2CAP instead, and gets
them (§5).

## 3. Bringing the link up

Subscribe and go [HARDWARE]. With notifications enabled on the UART TX
characteristic, a frame written to the UART RX characteristic reaches the
meter: writing the UT61+ Get Name frame `AB CD 03 5F 01 DA` made the meter beep
and a checksummed `AB CD` frame came back as a notification. Nothing is written
to the ISSC control characteristic first, and no configuration service is
touched.

The adapter takes the UT61+ start command itself [HARDWARE]. Writing
`AB CD 03 5D 01 D8`, the command UNI-T's own client sends once to start
reading, brings readings from about 1-2 s on and keeps them coming with
nothing written again — one 19-byte frame per notification, about 3.2 a
second with our UT61E+ behind it (§5). No ack for the command is ever seen. Behind a USB cable the same
command to the same meter draws the `FF 00` ack and no readings at all
(`../ut61eplus/reverse-engineered-protocol.md` §2.3), so the meter is not the
one acting on it: the adapter polls the meter and forwards what comes back.

A command meant for the meter is acked slowly [HARDWARE]. A button press
(HOLD) written over a fresh link had its `FF 00` ack 1.26 s later, against
about 0.1 s over the cable, and the meter's LCD followed. Presses work while
the readings are streaming.

The adapter puts one frame of its own on the stream [HARDWARE]:
`AB CD 06 AA AA 6E 67 03 A7` — a UT61+-style frame (header, length 6,
big-endian sum checksum) with the payload `AA AA 6E 67`. It comes once when a
link comes up, before the meter's first reply, and about once a second while
the meter is silent: with the meter switched off and the adapter on its own
batteries, the link stayed up and the frame kept coming. It is the adapter's,
not the meter's — over USB the meter never sends it — and what `6E 67` encodes
is not known. A host has to drop it before the meter's parser sees it.
UNI-T's own client reads the frame as "no meter data" [VENDOR]: it blanks the
displayed value and answers with Get Name, then the start command above, once
the name comes back.

Pairing is not required [HARDWARE]: with the bond removed from the host, a
scan found the adapter, the connection and the subscription went through and
the meter answered. A paired adapter is one the host lists without scanning;
that is its only effect. The same holds under Windows 11 [HARDWARE]: never
paired there, the adapter was found, connected, streamed and took a button
press, its ack 1.24 s after the command as on Linux.

Windows does not keep the link [HARDWARE]: in both sessions watched, the
adapter was back to its waiting flash once the program had closed its
connection.

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

Windows 11's advertisement watcher, an active scan, hears it better
[HARDWARE]: six 3 s scans in a row with the adapter waiting all reported it,
the first sighting at most about 1.75 s in, among about ten other devices.
One later 3 s scan did not report it at all, and a connect by address in the
same moment reached it: WinRT connects to an address it has not heard since
the program started, and finds the adapter itself. With the adapter
switched off, that connect gave up after about 8 s.

An adapter just switched back on can refuse its first connect [HARDWARE]:
heard by a Windows scan within seconds of being switched on, it did not
answer the connect that followed (WinRT reported it not connected after about
8 s), and answered the next one, 5 s after that failure. Whether BlueZ sees the same is not known
[UNVERIFIED].

## 5. Link parameters

Read off the air with `btmon` on 2026-09-22, our UT61E+ behind the adapter,
BlueZ 5.87 on kernel HCI [HARDWARE].

| Value | Reading | Confidence |
|---|---|---|
| ATT MTU | 247 bytes, as negotiated with BlueZ 5.87 | [HARDWARE] |
| Connection interval | the adapter sets it. Right after every connect it sends an L2CAP Connection Parameter Update Request for min 224 / max 255 (× 1.25 ms = 280-318.75 ms), latency 0, timeout 500 (5 s); the host accepts and the link runs at 315 ms. BlueZ stores the parameters, so the next connect is created at 315 ms directly | [HARDWARE] |
| The PPCP characteristic (`0x2A04`) | rejected by BlueZ (§2) and it changes nothing — the L2CAP request above is what moves the link | [HARDWARE] |
| A host-forced interval | an LE Connection Update from the host (`hcitool lecup --min 8 --max 16`, root) is accepted and the link moves to 15 ms; the adapter does not ask for its 315 ms back for the rest of the connection | [HARDWARE] |
| Notification cadence | about 315 ms between readings, whatever the radio does: at the forced 15 ms interval they were still 315-330 ms apart | [HARDWARE] |
| Streamed readings (after the start command, §3) | 3.23 Hz — 60 readings in 18.3 s, one 19-byte frame per notification; the same 3.2 Hz over two minutes | [HARDWARE] |
| Polled readings (one 0x5E each) | 1.44 Hz sustained, median gap 0.632 s, with an unacknowledged write; an acknowledged write measured 0.8 s a poll. At the forced 15 ms interval the same poll loop ran at 3.2 Hz, median gap 0.328 s (1.6 Hz before the update in that session) | [HARDWARE] |
| The same poll over USB | about 0.1 s | [HARDWARE] |
| First seconds of a fresh link | polls take up to 2 s while bluetoothd reads the Device Information characteristics (model, serial, firmware strings) | [HARDWARE] |
| Under Windows 11 (WinRT), unpaired | MTU 247. The interval WinRT reported right after the connect was 15 ms in two sessions and 315 ms in a third; whether the adapter's update request had landed when it was read is not known | [HARDWARE]; the timing [UNVERIFIED] |
| Streamed readings under Windows 11 | the same ~315 ms cadence (the gaps that are not doubled average 315 ms), but now and then two notifications arrive back to back: one run received 66 reading notifications while a reader that keeps the newer of two queued readings produced 60 | [HARDWARE] |

A 19-byte UT61+ reading arrives as one notification on this MTU. The
~315 ms cadence is the adapter's own for this family: dropping the
connection interval by a factor of twenty left it where it was. What the
interval costs is the second trip a poll needs — at 315 ms a poll pays two
of them and a streamed reading one, which is why the start command alone
doubles the rate and why at 15 ms a polled link catches up with a streamed
one. The cadence, not the radio, is the ceiling, and the start command
reaches it with nothing asked of the host stack.

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
service set is unknown — it has not been seen. A first capture from one settles
three things: which service and characteristic pair its transparent UART sits
on (§2), whether it sends the same heartbeat frame while the meter is silent,
and whether it acts on the UT61+ start command 0x5D the way this one does (§3)
[UNVERIFIED].

## Implementation Notes

Facts about the adapter that any decoder has to live with:

- The link carries the meter's bytes and nothing else. There is no
  adapter-level framing, length field or checksum wrapped around them, so a
  decoder written for the cable works unchanged.
- The UT61+ start command is the adapter's, not the meter's (§3). A host that
  sends it is fed at the adapter's own cadence with nothing written again;
  one that does not pays a round trip for every reading.
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

## 7. Cross-reference with community sources [COMMUNITY]

Read 2026-09-22, after §1-6 were written from our own unit and UNI-T's own
material; the clean-room boundary was opened that day for the adapter's read
rate. Nothing here was merged into §1-6 and nothing here contradicts them.
Sources and the boundary: `reverse-engineering-approach.md`.

| Finding | Ours (§) | Community | Agree? |
|---|---|---|---|
| Module | PnP vendor `0x005D`, ISSC/Microchip (§1) | "UT-D07 (Bluetooth adapter, **ISSC BL79 BLETR** chip)" (libsigrok `README.devices:351`); a UT-D07A teardown found a PIC18LF25K22 and a BL79BLETRMC2 ([EEVblog, 2017-11-11](https://www.eevblog.com/forum/projects/uni-t-ut-d07a-bluetooth/msg1346931/#msg1346931)) | ✓, and names the part |
| UART service, characteristics | ISSC `…fe7d…`, notify `…1e4d…`, write `…8841…`, control `…aca3…` unused (§2, §3) | Same UUIDs and roles in [webspiderteam/Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows) (`GattMonitor.cs:44`), [libreble/multimeter](https://github.com/libreble/multimeter) (`docs/protocols/uni-t.md`), [olegv142/ut61xpy](https://github.com/olegv142/ut61xpy) (`adapters/ut61xp.py:214-215`) and subsurface (`core/qt-ble.cpp:298-306`), which also avoids `…aca3…` and `…6daa…` | ✓ |
| Pairing not required | §3 | Windows client pairs with `DevicePairingKinds.ConfirmOnly`, no passkey (`PairingHelper.cs:16-18`) | ✓ |
| An unreachable adapter is asleep | §4 | "The BLE adapter will shutdown within a short period of time when it's not being communicated to, needs another power cycle to re-connect. The USB cable does not suffer from such a constraint." (libsigrok `src/hardware/uni-t-ut181a/protocol.c:34-39`) | ✓; adds idle as the trigger |
| Heartbeat `AB CD 06 AA AA 6E 67 03 A7` | The adapter's, dropped before the parser; `6E 67` unknown (§3) | Both BLE clients treat a 9-byte `AB CD … AA AA …` frame as "no data, re-identify" and answer with Get Name (`DecoderUni_T.cs:337-352`; `framing.ts:42-47`). Nobody decodes `6E 67`. libreble's UT60BT captures — same family, no adapter in the path — never saw the frame | ✓; `6E 67` still unknown |
| One UT61+ poll ≈ 0.63 s over BLE, ≈ 0.1 s over USB (§5) | | ut61xpy, which polls the same `0x5E` over a UT-D07B: "minimum achievable data readout interval is around 180 msec for USB adapter and around **800 msec for Bluetooth** adapter" (README) | ✓ on BLE; ✗ on USB (their 180 ms vs our ~100 ms) |
| Connection interval 315 ms, asked for by the adapter; MTU 247; the adapter's ~315 ms cadence bounds the rate (§5) |  | new — nobody else measured them |
| Write without response is the faster write (§5) | | libsigrok writes BLE with ATT Write Request, acknowledged (`bt_bluez.c:1036-1053`); ut61xpy and the Windows client write without response | ✗ with sigrok, by choice not by measurement |
| The adapter carries a stream at near wire rate | not measured | A UT181A in monitor mode over a **UT-D07A** logged "a packet at 10Hz … At 55 Bytes" and, checked against a sine wave, "the ~10 Hz sample rate is real" ([2018-10-04](https://www.eevblog.com/forum/projects/uni-t-ut-d07a-bluetooth/msg1868123/#msg1868123), [2018-10-12](https://www.eevblog.com/forum/projects/uni-t-ut-d07a-bluetooth/msg1896485/#msg1896485)) | new |
| What the ~0.5 s per poll was | the adapter's own cadence, measured (§5) | One unmeasured guess, host→adapter direction: "Perhaps there is a minimum time between packets when sending data to the D07A. I added a 500ms delay and a retry and it has been solid since" ([2018-10-09](https://www.eevblog.com/forum/projects/uni-t-ut-d07a-bluetooth/msg1882682/#msg1882682)) | new, unconfirmed |
| Which meters sit behind which adapter (§6) | UNI-T's two pages disagree | A third position: "Uni-T multimeters with UT-D07A and UT-D07B Bluetooth Adaptor (UT61+ Series, UT161 Series, UT171 Series, and UT181A Only)" (webspiderteam README) | disagrees with UNI-T's global page |
| UT-D07A vs UT-D07B | the A has not been seen (§6) | The A advertises a name starting `UT-D07A`; drivers match on it (`libreble` `ut171.ts:380-385`). Same ISSC module class as ours; its GATT tree is still unenumerated by anyone | partial |

The UT-D07A "sometimes needs START repeated" for the UT171 (`libreble`
`docs/protocols/ut171.md`, untested on their side), which matches the
vendor app re-sending its start frame at +2 s and +3 s.
