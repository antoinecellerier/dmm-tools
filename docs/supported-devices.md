# Supported Devices

<!-- Keep this file updated when adding support for new models. -->

Every supported meter talks over a USB HID-to-UART cable. The tool works out
which cable and which meter are attached from the bytes the meter sends
([how](detection-design.md)), so the default `auto` device needs no setup
beyond switching the meter's USB output on, as listed per family below.

**✅ Verified** means the model's protocol tables have been confirmed on real
hardware. **🧪 Experimental** means the protocol was reverse-engineered from
vendor software and manuals; the linked issue collects hardware reports, and
the tool prints a warning on connect. The per-family research is under
[docs/research/](research/); what remains to verify is in the
[verification backlog](verification-backlog.md).

## USB cables

| Cable | Chip | VID:PID | Direction | Meters | Confirmed with |
|---|---|---|---|---|---|
| UT-D09 | CP2110 | `10C4:EA80` | both ways | UT61+/UT161, UT171, UT8802/UT8803, Voltcraft, older UT181A units | UT61E+ |
| UT-D09 | CH9329 | `1A86:E429` | both ways | sold for UT181A, UT171, UT243 | UT181A (two units), UT61B+ |
| UT-D04 | CH9325 / HE2325U | `1A86:E008` | meter to PC only | UT803, UT804, older UNI-T meters | not yet run on a meter |
| UT-D02 | RS232 level converter | — | both ways | serial port, not USB; not supported | — |

## UT61+ / UT161

Handheld. Cable: UT-D09 (either chip). Switch on: insert the USB module,
turn the meter on, long-press USB/Hz until the S icon shows.

| Model | Counts | Status | Notes |
|---|---|---|---|
| UT61E+ | 22000 | ✅ Verified | our reference meter |
| UT61B+ | 6000 | ✅ Verified | 10 A max current; from community captures ([#19](https://github.com/antoinecellerier/dmm-tools/issues/19)) |
| UT61D+ | 6000 | 🧪 Experimental ([#7](https://github.com/antoinecellerier/dmm-tools/issues/7)) | adds temperature and LoZ AC V |
| UT161E | 22000 | 🧪 Experimental ([#7](https://github.com/antoinecellerier/dmm-tools/issues/7)) | same tables as UT61E+ |
| UT161D | 6000 | 🧪 Experimental ([#7](https://github.com/antoinecellerier/dmm-tools/issues/7)) | same tables as UT61D+ |
| UT161B | 6000 | 🧪 Experimental ([#7](https://github.com/antoinecellerier/dmm-tools/issues/7)) | same tables as UT61B+ |

All six share one protocol and differ only in their mode and range tables.
The UT61D+ and UT161 tables come from the manuals; [#7](https://github.com/antoinecellerier/dmm-tools/issues/7)
lists the modes that need a capture.

## UT8802 / UT8803

Bench. Cable: UT-D09 (CP2110). Switch on: connect the cable and turn the
meter on.

| Model | Counts | Status | Notes |
|---|---|---|---|
| UT8802 / UT8802N | — | 🧪 Experimental ([#12](https://github.com/antoinecellerier/dmm-tools/issues/12)) | |
| UT8803 / UT8803E | — | 🧪 Experimental ([#3](https://github.com/antoinecellerier/dmm-tools/issues/3)) | |

Neither has been run on a meter yet.

## UT803 / UT804

Bench. Cable: UT-D04 (CH9325), receive-only, so there are no remote
commands. Switch on: connect the cable and turn the meter on.

| Model | Counts | Status | Notes |
|---|---|---|---|
| UT803 | 6000 | 🧪 Experimental ([#15](https://github.com/antoinecellerier/dmm-tools/issues/15)) | |
| UT804 | 4000 | 🧪 Experimental ([#16](https://github.com/antoinecellerier/dmm-tools/issues/16)) | |

Neither has been run on a meter yet; the CH9325 cable itself is untested.

## UT171 / UT181A

Handheld. Cable: UT-D09 (either chip). Switch on: SETUP → Communication →
ON; the UT181A forgets this at power-off.

| Model | Counts | Status | Notes |
|---|---|---|---|
| UT171A/B/C | — | 🧪 Experimental ([#4](https://github.com/antoinecellerier/dmm-tools/issues/4)) | not yet run on a meter |
| UT181A | — | 🧪 Experimental ([#5](https://github.com/antoinecellerier/dmm-tools/issues/5)) | logging meter |

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

## Not supported yet

Bluetooth meters and adapters (UT60BT, UT-D07B, EEVBlog 121GW, OWON) and
serial meters (Fluke 28x, UT805A) need a transport the tool does not have.
Candidates, their protocols and what each would take are researched in
[new-device-candidates.md](research/new-device-candidates.md). Your model is
missing? [Open an issue](https://github.com/antoinecellerier/dmm-tools/issues)
with its details; captures from an experimental meter go to its issue above
([how to capture](../CONTRIBUTING.md#protocol-captures)).
