# Supported Devices

<!-- Keep this file updated when adding support for new models. -->

Most supported meters talk over a USB HID-to-UART cable; the UT-D07
Bluetooth adapters serve the meters listed for them below, and the UT60BT,
UT202BT and ZOTEK meters have Bluetooth built in. The tool works out which
link and which meter are attached from the bytes the meter sends
([how](detection-design.md)), so the default `auto` device needs no setup
beyond switching the meter's data transmission on, as listed per family below.

**✅ Verified** means the model's protocol tables have been confirmed on real
hardware. **🟡 Partly verified** means connection and the main modes are
confirmed and the rest is still to verify. **🧪 Experimental** means the
protocol was reverse-engineered from vendor software and manuals. Short of
verified, the linked issue collects hardware reports and the tool prints a
warning on connect. The per-family research is under
[docs/research/](research/); what remains to verify is in the
[verification backlog](verification-backlog.md).

## Cables and adapters

| Cable | Chip | VID:PID | Direction | Meters | Confirmed with |
|---|---|---|---|---|---|
| UT-D09 | CP2110 | `10C4:EA80` | both ways | UT61+/UT161, UT171, UT8802/UT8803, Voltcraft, older UT181A units | UT61E+ |
| UT-D09 | CH9329 | `1A86:E429` | both ways | sold for UT181A, UT171, UT243 | UT181A (two units), UT61B+ |
| UT-D04 | CH9325 / HE2325U | `1A86:E008` | meter to PC (these meters take no commands) | UT803, UT804, and the UT71A–E per UNI-T's accessory page | UT804 on Linux and Windows ([#16](https://github.com/antoinecellerier/dmm-tools/issues/16)) |
| UT-D02 | RS232 level converter | — | both ways | serial port, not USB; not supported | — |
| UT-D07B | Bluetooth LE | — | both ways | UT61+/UT161, UT171, UT181 series per UNI-T's page | UT61E+ ([#25](https://github.com/antoinecellerier/dmm-tools/issues/25)) |
| UT-D07A | Bluetooth LE | — | both ways | UT171, UT181 series per UNI-T's page (also UT71, which auto-detection does not look for over Bluetooth yet) | — untested ([#25](https://github.com/antoinecellerier/dmm-tools/issues/25)) |

## UT61+ / UT161

Handheld. Cable: UT-D09 (either chip), or the UT-D07B Bluetooth adapter
([setup](setup.md#bluetooth)). Switch on: insert the USB module, turn the
meter on, long-press USB/Hz until the S icon shows.

| Model | Counts | Status | Notes |
|---|---|---|---|
| UT61E+ | 22000 | ✅ Verified | our reference meter |
| UT61B+ | 6000 | ✅ Verified ([#19](https://github.com/antoinecellerier/dmm-tools/issues/19)) | 10 A max current |
| UT61D+ | 6000 | 🧪 Experimental ([#7](https://github.com/antoinecellerier/dmm-tools/issues/7)) | adds temperature and LoZ AC V |
| UT161E | 22000 | 🧪 Experimental ([#7](https://github.com/antoinecellerier/dmm-tools/issues/7)) | same tables as UT61E+ |
| UT161D | 6000 | 🧪 Experimental ([#7](https://github.com/antoinecellerier/dmm-tools/issues/7)) | same tables as UT61D+ |
| UT161B | 6000 | 🧪 Experimental ([#7](https://github.com/antoinecellerier/dmm-tools/issues/7)) | same tables as UT61B+ |

All six share one protocol and differ only in their mode and range tables.
The UT61D+ and UT161 tables come from the manuals; [#7](https://github.com/antoinecellerier/dmm-tools/issues/7)
lists the modes that need a capture.

## UT60BT / UT202BT

Handheld (UT60BT) and clamp meter (UT202BT). Bluetooth built in, no cable
([setup](setup.md#bluetooth)). Switch on: long-press SEL on the UT60BT, or
short-press the Bluetooth button on the UT202BT, until the Bluetooth symbol
shows.

| Model | Counts | Status | Notes |
|---|---|---|---|
| UT60BT | 9999 | 🧪 Experimental ([#26](https://github.com/antoinecellerier/dmm-tools/issues/26)) | |
| UT202BT | 9999 | 🧪 Experimental ([#27](https://github.com/antoinecellerier/dmm-tools/issues/27)) | clamp: 600 A AC, inrush, LPF; second display as a sub-value |

Neither has been run on a meter yet: the range tables come from UNI-T's
iDMM2.0 app and the manuals ([backlog](verification-backlog.md)).

## UT8802 / UT8803

Bench. Cable: UT-D09 (CP2110). Switch on: connect the cable and turn the
meter on.

| Model | Counts | Status | Notes |
|---|---|---|---|
| UT8802 / UT8802N | — | 🧪 Experimental ([#12](https://github.com/antoinecellerier/dmm-tools/issues/12)) | |
| UT8803 / UT8803E | — | 🧪 Experimental ([#3](https://github.com/antoinecellerier/dmm-tools/issues/3)) | |

Neither has been run on a meter yet.

## UT803 / UT804

Bench. Cable: UT-D04 (CH9325). No remote commands. Switch on: press SEND
(UT804) or RS232 (UT803) so the display shows it; the UT804's EXIT turns SEND
off.

| Model | Counts | Status | Notes |
|---|---|---|---|
| UT803 | 6000 | 🧪 Experimental ([#15](https://github.com/antoinecellerier/dmm-tools/issues/15)) | |
| UT804 | 40000 | ✅ Verified ([#16](https://github.com/antoinecellerier/dmm-tools/issues/16)) | |

A reporter's UT804 has confirmed every dial position and auto-detection;
what MAX MIN and REL send is still to confirm
([backlog](verification-backlog.md)). The UT803 has not been run on a meter
yet, and auto-detection does not find it: select UT803 as the device.

## UT71 / Voltcraft VC920 / VC940 / VC960

Handheld. Cable: UT-D04 (CH9325) for the UT71; the VC920/VC940/VC960 ship an
RS-232 cable (serial, not supported) and take Voltcraft's optional USB
adapter 120317, whose chip is unconfirmed. No remote commands. Switch on:
hold MAX MIN for 1 s (press SEND on a UT71A) so the display shows SEND; EXIT
turns it off.

| Model | Counts | Status | Notes |
|---|---|---|---|
| UT71A | 20000 | 🧪 Experimental ([#22](https://github.com/antoinecellerier/dmm-tools/issues/22)) | no temperature or 4-20 mA %; SEND key |
| UT71B | 20000 | 🧪 Experimental ([#22](https://github.com/antoinecellerier/dmm-tools/issues/22)) | adds °C/°F and 4-20 mA % |
| UT71C / UT71D | 40000 | 🧪 Experimental ([#22](https://github.com/antoinecellerier/dmm-tools/issues/22)) | |
| UT71E | 40000 | 🧪 Experimental ([#22](https://github.com/antoinecellerier/dmm-tools/issues/22)) | adds power (W); one V≂ position |
| VC920 / VC960 | 40000 | 🧪 Experimental ([#23](https://github.com/antoinecellerier/dmm-tools/issues/23)) | resistance 4000 counts; AC V to 750 V |
| VC940 | 40000 | 🧪 Experimental ([#23](https://github.com/antoinecellerier/dmm-tools/issues/23)) | as VC920, adds power (W) |

None has been run on a meter yet: the range tables come from UNI-T's protocol
sheet and the manuals ([backlog](verification-backlog.md)). Auto-detection
reports these meters as a UT804: pick UT71A/B, UT71C/D/E or Voltcraft VC920
as the device once (`--device ut71ab`, `ut71cde` or `vc920`).

A UT71 on a UT-D07A Bluetooth adapter is untested and not found by
auto-detection: name both the model and the adapter
(`--device ut71ab --adapter <address>`) and report the result on
[#25](https://github.com/antoinecellerier/dmm-tools/issues/25).

## UT171 / UT181A

Handheld. Cable: UT-D09 (either chip), or the UT-D07B or UT-D07A (untested)
Bluetooth adapter ([setup](setup.md#bluetooth)). Switch on: SETUP →
Communication → ON; the UT181A forgets this at power-off.

| Model | Counts | Status | Notes |
|---|---|---|---|
| UT171A/B/C | — | 🧪 Experimental ([#4](https://github.com/antoinecellerier/dmm-tools/issues/4)) | not yet run on a meter |
| UT181A | — | 🟡 Partly verified ([#5](https://github.com/antoinecellerier/dmm-tools/issues/5)) | logging meter |

Two reporters have run the UT181A over the CH9329 cable: connection, V DC,
V AC + Hz and dual-probe temperature are confirmed. The MIN/MAX, REL, Peak
and COMP formats, the remote commands and the CP2110 cable are still pending
([backlog](verification-backlog.md)).

## Voltcraft VC-880 / VC650BT / VC-890

Cable: UT-D09 (CP2110). Switch on: press the PC button on the meter.

| Model | Counts | Status | Notes |
|---|---|---|---|
| VC-880 | 40000 | 🧪 Experimental ([#13](https://github.com/antoinecellerier/dmm-tools/issues/13)) | handheld |
| VC650BT | 40000 | 🧪 Experimental ([#13](https://github.com/antoinecellerier/dmm-tools/issues/13)) | bench; same protocol as VC-880 |
| VC-890 | 60000 | 🧪 Experimental ([#14](https://github.com/antoinecellerier/dmm-tools/issues/14)) | handheld, OLED; own protocol |

No Voltcraft meter has been run on hardware yet: the mode tables come from
the manuals and the remote commands from the vendor software.

## ZOTEK / ZOYI / BSIDE / ANENG

Handheld (ZT-300AB, ZT-5B), desktop (ZT-5566SE) and clamp meter (ZT-5BQ).
Bluetooth built in, no cable; the meter shows up as "Bluetooth DMM"
([setup](setup.md#bluetooth)). Switch on: hold Hz% for 2 s on the ZT-300AB,
press POWER on the ZT-5566SE, press Power and Hz together on the ZT-5BQ, or
short-press the red button on the ZT-5B, until the Bluetooth symbol shows.

| Model | Counts | Status | Notes |
|---|---|---|---|
| ZT-300AB / AN9002 | 6000 | 🧪 Experimental | rotary dial; also sold by BSIDE |
| ZT-5566SE / AN999S | 19999 | 🧪 Experimental | Bluetooth speaker; second display as a sub-value |
| ZT-5BQ / ST207 | 6000 | 🧪 Experimental | clamp: 600 A AC, inrush, peak hold |
| ZT-5B / V05B | 6000 | 🧪 Experimental | auto-only pocket meter |

Not run on a meter yet: the decoding comes from ZOTEK's apps and manuals,
whose ZT-5566SE pages document Bluetooth for the speaker only, and the
remote keys from the apps. The meter
names only its packet layout, so auto-detection picks the row above that
sends it ([backlog](verification-backlog.md#zotek-zoyi--aneng--bside-experimental-awaiting-a-hardware-report)).

## Not supported yet

Other meters with a Bluetooth radio built in (EEVBlog 121GW, OWON) and
serial meters (Fluke 28x, UT805A) are not supported yet.
Candidates, their protocols and what each would take are researched in
[new-device-candidates.md](research/new-device-candidates.md). Your model is
missing? [Open an issue](https://github.com/antoinecellerier/dmm-tools/issues)
with its details; captures from an experimental meter go to its issue above
([how to capture](../CONTRIBUTING.md#protocol-captures)).
