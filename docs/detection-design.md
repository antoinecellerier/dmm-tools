# Device Detection Design

`dmm-lib` identifies the meter on an already-open transport from the bytes it sends, so the
user no longer has to name the family before connecting (issue #9). Nothing in USB says which
meter is on the cable — every CP2110 adapter enumerates as `10C4:EA80` — and the wrong family
fails as a parse error rather than a hint: PR #8 recorded `checksum mismatch: expected 0x0075,
got 0x07ed` from a UT181A read as a UT61+.

Detection lives in `crates/dmm-lib/src/detect.rs`. `detect_device(transport, bridge)` walks a
probe cascade and returns the registry entry it settled on plus the name the meter reported,
if it gave one; `Dmm::from_detected` opens the session with both, so the name is not asked
again. What each family sends and what it recognises is the family's own
`Fingerprint`, declared in the family module next to the constants it already puts on the wire;
`detect.rs` is the engine that runs them. This document is the algorithm and the reasons behind
its shape; the CLI and GUI surface (`--device auto`, the Auto-detect chip) is described in
[cli-reference.md](cli-reference.md) and [gui-reference.md](gui-reference.md). Per-family
verification status is not repeated here — it lives in the backlog's
[Device auto-detection](verification-backlog.md#device-auto-detection) section. Where the
design departs from what [issue #9](https://github.com/antoinecellerier/dmm-tools/issues/9)
assumed, the sections below state what holds today.

## Evidence

- A UT61+ never sends a byte unsolicited. `0x5F` gets the ack `AB CD 04 FF 00 02 7B` in
  36–83 ms and the ASCII name frame in 144–191 ms, on both bridges
  (`references/ut61eplus/captures-2026-09-07/`, `references/ut61b-plus/`). The name picks the
  exact registry entry: `UT61E+` and `UT61B+` are the entries' `display_name`s.
- CP2110 can deliver one UART byte per HID report
  (`references/ut61eplus/captures-2026-09-07/ut61eplus-verify.yaml`, taken before read
  coalescing): a probe must accumulate bytes rather than expect a whole frame per read.
- A UT181A is silent until SET_MONITOR, then streams 2-byte-LE frames of type `0x02`, and
  beeps once on the command (PR #8).
- The UT171 connect frame `AB CD 04 00 0A 01 0F 00` is opcode `0x0A` — UT181A *start
  recording* ([ut181 §10.1](research/ut181/reverse-engineered-protocol.md)). Sending it is
  safe only because a UT181A with Communication ON is identified in the step before; with
  Communication OFF it ignores everything anyway.
- UT181A and UT171 frame lengths overlap (UT181A normal format 19 bytes without aux or
  bargraph, UT171 16 or 22): length alone cannot split them.

## The cascade

Each step is an optional transmission followed by a listen window. The whole buffer is
re-classified after every read, so a meter that streams on its own is identified in whichever
window it first speaks — the unprompted families need no step of their own. The CP2110 carries
every AB CD family, so the table is its cascade:

| Step | TX | Identifies |
|---|---|---|
| 1 | `AB CD 03 5F 01 DA` (UT61+ Get Name) | UT61+/UT161 model, by name |
| 2 | `AB CD 04 00 05 01 0A 00` (UT181A SET_MONITOR) | UT181A |
| 3 | `AB CD 04 00 0A 01 0F 00` (UT171 connect) | UT171 |
| 4 | 3× `AB CD 04 FF 00 02 7B` then `AB CD 03 5E 01 D9` (VC-890 poll) | VC-890 |
| any | — (nothing sent) | UT8803, UT8802, VC-880; on Bluetooth, the ZOTEK meters, the 121GW, the BM78xBT and the OWON meters |

The CH9329 and the UT-D07B carry only the UT61+, UT171 and UT181A, so they run steps 1–3; on
Bluetooth, the meters with the radio built in are heard in those windows. The BU-86X gets three
steps of its own: `00 86 66`, `00 82 66` and `00 52 66`, the reading requests of the BM86x,
BM82x and BM52x in registry order, whose reply names the series in its model bytes.

The order is load-bearing, and it is derived rather than written down. Registry order is the
preference — `DEVICES` lists the most common meters first, which is what puts `0x5F` at the head,
the best-verified probe and the fastest to answer. The one hard constraint is the UT171
fingerprint's own `send_after: &[Ut181a]`, which holds the connect frame back until the UT181A
trigger has gone out, so that a UT181A with Communication ON is already identified when the `0x0A`
opcode that would start a recording on it arrives. A family named in `send_after` that the bridge
does not carry is ignored — there is nothing there to wait for — and constraints that contradict
each other fall back to registry order with a WARN, since sending every probe in a suspect order
beats sending none.

Each step logs at DEBUG — the probe sent, and what a rule identified — and the result is
one INFO line, so `RUST_LOG=dmm_lib=debug` is the whole story when a reporter's meter is not
recognised.

## Evidence strength

A family's `Fingerprint` (`protocol/mod.rs`) is a log label, an optional trigger — bytes the family
already sends in normal use (its init, name query or reading request) — the families that trigger has to follow (`send_after`),
whether its extractor validates a checksum, and a rule that classifies the whole receive buffer
with that family's constants. Which of them run is the registry's call: every `DEVICES` entry
points at its family's fingerprint, and detection runs the ones the bridge carries, in table order.

A rule answers `Evidence::Model` (a registry id, plus the name where the meter sent one) or
`Evidence::FamilyOnly` (the family is settled, no model named). Every carried rule runs after
every read, against every candidate offset in the buffer, and the strongest answer wins:

| Rank | Evidence | Where it comes from |
|---|---|---|
| 4 | a model the meter named itself | the UT61+ name frame; an OWON meter's model code, read from FFF2 |
| 3 | a model from a frame whose 16-bit checksum held | UT8803, UT171, UT181A, VC-880, VC-890, BM78xBT |
| 2 | `FamilyOnly` — a checksummed frame naming no model | a bare 14-byte UT61+ reading |
| 1 | a model from a rule with no checksum, or an 8-bit XOR | UT8802, UT804, ZOTEK, 121GW, BM86x, BM82x, BM52x, OWON frames without a model code |

A `FamilyOnly` at the top is remembered rather than acted on: the window keeps listening, and a
name frame arriving in it outranks the fallback (see
[Names and the registry](#names-and-the-registry)). When the window ends with nothing stronger,
that fallback is what detection returns rather than the next probe going out — the family is
settled, and every trigger left in the cascade belongs to another one. Two *different* families
tied at the top identify nothing: a WARN names both and the window keeps listening, because a
tie is a 2^-16 checksum collision or a rule claiming too much, not a meter.
Extractor errors are ignored throughout: to a classifier "this is not that format here" is the
answer, not a failure, and a checksum mismatch at one offset says nothing about the next.

The overlaps the ranking arbitrates, each rule declining what is not its own:

- `ut8803` — a measurement frame whose checksum holds. A UT61+ DC V frame has the UT8803's type
  byte in the same place, but it is shorter, so the UT8803 checksum fails on it.
- `ut181a` and `ut171` — the 2-byte-LE pair: identical framing and measurement type byte, and
  payload lengths that overlap. The UT181A takes a payload only it can send and anything its own
  trigger elicited. The UT171 declines exactly those two: such a payload is longer than its own
  extended frame ([ut171 §3.4/§5.2](research/ut171/reverse-engineered-protocol.md)), and a frame
  arriving right after SET_MONITOR is the one that command asked for. It takes the rest, with a
  WARN when the frame came before its connect frame went out. OK/ER replies are ignored. Neither
  rule can fire on the other families: a UT61+ name frame reads a length of `0x5508`, a VC-880
  frame `0x0124`.
- `vc880`, `vc890` and `ut61eplus` — the 1-byte BE16 families. A UT8803 frame's mode byte reads
  as a plausible length here, which is what the checksum settles. The two Voltcraft meters share
  a type byte and differ in payload length. Acks are skipped. A name frame picks a UT61+ model; a
  bare reading settles the family only (`FamilyOnly`, fallback `ut61eplus`).
- `ut8802` — two consecutive `0xAC` frames. That format's validation passes roughly 1% of random
  bytes and UT181A frames carry arbitrary float32 payload, which is why an unchecksummed claim
  ranks below even a settled family.
- `121gw` — one packet whose 8-bit XOR holds, whose mode and range are in the tables and whose
  reserved bits are clear. The XOR passes one window in 256, so the rule ranks with the
  unchecksummed ones; the tables and reserved bits keep a chance match rare.
- `bm78xbt` — one reading packet whose CRC holds and that says it is a meter's. None of the AB CD,
  121GW or ZOTEK frames starts the way it does.
- `bm86x`, `bm82x`, `bm52x` — four model bytes of the series in a row, whichever request drew
  them: Brymen's programs expect a BM52x to answer the BM82x's request. One meter sends one
  series' code, so no two of the rules match.
- `owon` — a 6-byte frame whose function word carries OWON's marker. With a model code read at
  connect, one frame names the code's entry, or the B35T+ entry for an unknown code; a code
  OWON's programs read with another decoder (the 15-byte frame, series 55) gives the B35T+ entry
  on any bytes, and its `init` refuses it naming the format. Without a code, two frames 6 bytes
  apart, no unknown status bit set, and the marker at every later 6-byte step to the buffer's end,
  in whole steps, give the B35T+ entry at rank 1; a 15-byte meter's frames fail that.

The bytes each rule expects are in the backlog's
[Device auto-detection](verification-backlog.md#device-auto-detection) table and in each family's
`recognise`.

`ut80x` is the CH9325's rule and is the only one consulted there; the AB CD rules are the other
bridges' (see [Bridges and adapters](#bridges-and-adapters)).

## Names and the registry

A name frame resolves through `protocol/ut61eplus`'s `device_for_reported_name(name)`, which
matches `display_name` then aliases, case-insensitively, across the UT61+/UT161 entries only. An
unrecognised name falls back to `ut61eplus`, keeps the reported name on `Detected`, and warns:
a UT61D+ or a UT161x still reads, with UT61E+ tables, and the user is told which name to
report so an alias can absorb it.

A meter with Bluetooth built in names itself before any frame: `Ble` reports the name the peer
advertised (`Transport::advertised_name()`). `built_in_meters()` in `lib.rs` looks that name up
in each entry's `bluetooth_names`, with the prefix rule the search takes the peer by
(`registry::advertising()`).

When entries match, detection runs only their fingerprints: a UT60BT gets Get Name and never the
UT181A's or the UT171's probes. An adapter's name, or a peer opened by address with no name heard,
matches none and gets the bridge's whole cascade. A link that read a characteristic at bring-up
runs only the fingerprints that declare `claims_info_links` (OWON's, whose profile is the only one
that reads one) whatever its name, so a renamed OWON meter gets no UNI-T probe on its key
characteristic.

When exactly one entry matches, a `FamilyOnly` fallback opens that entry rather than
`ut61eplus`, and so does an unrecognised name frame. The tables differ: a UT60BT's V range starts
at 999.9mV, a UT61E+'s at 2.2V. The WARN says the tables came from the advertised name. A name
several entries share narrows the probes to theirs but picks no model: the frames decide, and a frame
naming none gets the fingerprint's own fallback.

A name frame that resolves still outranks the advertised name. If the two disagree, a WARN names
both and the name frame's entry opens.

With nothing identified, `detect_device` returns `Error::DeviceNotIdentified`, whose
`kind()` is `ErrorKind::Timeout` — reconnecting after enabling transmission on the meter does
help. Its `Display` names the link the probe ran over — the USB cable, the Bluetooth adapter or
the meter's own radio — never the bridge chip.

## Bridges and adapters

Which rules a bridge gets is the registry's call, not `detect.rs`'s. Each entry's `links` list
the cables its meter is found on, and detection runs only the fingerprints of the families whose
entries list the bridge it opened — the same list the "no meter answered" help draws on. A family
seen on a new cable joins detection there by being listed on it.

- **CP2110 and CH9329** share the AB CD probes, but the CH9329 carries only the UT61+, UT171 and
  UT181A, so it runs steps 1–3 of [the cascade](#the-cascade). The shortcut issue #9 assumed
  (CH9329 means UT181A) does not hold: a UT61B+ is verified over CH9329 and older UT181A units ship the CP2110.
- **CH9325** carries the UT80x family alone: the UT803/UT804, and the UT71 and Voltcraft
  VC920/VC940/VC960, which send the UT804's packets. It is receive-only past its init, which
  already sends `0x5A`, so detection there is a single listen window that takes any whole CR LF
  packet as a UT804. A packet does not name its model, so a UT71 or VC9x0 is claimed as a UT804
  and has to be named (the GUI's device chip, saved once, or `--device ut71ab` / `ut71cde` /
  `vc920`). The CH9325 starts at 2400 baud, where only the UT804 is heard: a UT803 talks at
  19200, so it is not detected and has to be named (`--device ut803`).
- **UT-D07B** is a transparent UART bridge like the cables, so the UT61+, UT171 and UT181A
  fingerprints listed on it run unchanged. It is opened after the USB bus, so a cable with a
  silent meter on it wins over a live meter on the adapter.
- **ZOTEK meters** have the radio built in and stream on their own, so their rule sends nothing
  and reads whatever window is open. It takes one whole packet, found by its scrambled header, cut
  at its type's length and made only of listed glyphs. The packet's type byte picks the registry
  entry for that layout; the meter never names its model.
- **121GW** streams on its own too, and its rule takes one packet found by its XOR as above. With
  "121GW" advertised, it is the only rule that runs.
- **BM78xBT** streams once the transport has logged in, before detection starts. Its rule takes
  one CRC-valid reading packet, and with "BM78xBT" advertised it is the only rule that runs.
  Behind an unnamed link it rides the UNI-T probe windows and sends nothing.
- **OWON meters** stream on their own too. Their Bluetooth profile reads the model code once at
  connect, before detection starts, and the rule takes it with one frame; with no code read, it
  needs two. With "BDM" or "LILLIPUT" as the peer's name, or FFF2 read, it is the only rule that
  runs; behind an unnamed link that read no FFF2 it rides the UNI-T probe windows and sends
  nothing.
- **BU-86X** carries Brymen's three series alone, so detection there sends each series' reading
  request, the one a named meter sends for every reading. A meter in capacitance, or a BM86x at
  500000 counts, can answer after the windows close; the activation steps say to set a voltage
  function. `auto` tries the BU-86X after every other cable, so with any other cable plugged in
  too it is never probed. A BU-86X whose meter is off stops `auto` there, before Bluetooth: name
  the meter, or pass `--adapter`.

Only the first adapter found is probed. With several plugged in, the existing
multiple-adapter warning applies and `--adapter` selects one; probing every adapter is a
follow-up, not part of this design.

## Timing

A listen window is bounded twice: a deadline (`WINDOW` in `detect.rs`, long enough for the UT61+
name reply and one frame of a UT8803's 2–3 Hz stream) **and** an empty-read cap
(`MAX_EMPTY_READS`, as in `framing::read_uart_bytes`), because a drained `MockTransport` returns
`Ok(0)` instantly and would otherwise spin until the deadline. The walk to "not identified" takes
one window per trigger: four on the CP2110, three on the CH9329, the UT-D07B and the BU-86X. A
UT61+ answers inside the first one, so an auto open normally costs one name reply.

A window that follows no probe — the CH9325's only one — lasts `LISTEN_ONLY_WINDOW` instead,
because the meter sets the pace there (issue #16's UT804). The empty-read cap outlasts it even
though the CH9325 answers every idle poll with an empty report.

The buffer persists across steps, capped at `MAX_RX_BUF` with the oldest bytes dropped. That is
what lets bytes arriving one per HID report assemble into a frame across a window boundary, and
what gives a slow streamer (the UT8803) the whole walk instead of one window.

## Failure modes and mitigations

| Failure mode | Cause | What the user sees | Mitigation |
|---|---|---|---|
| Nothing answers | Meter off or not transmitting; wrong cable or baud | `DeviceNotIdentified` after the last window | Activation help for the bridge; `--device <id>` skips probing |
| Misidentification from junk | Random bytes passing a lax extractor | Wrong parser, later parse errors | The [evidence ranking](#evidence-strength); the "Detected X" notice |
| Stale frame from an earlier session | CH9329 does not purge RX on open | Family evidence before the probe reply | A name frame outranks a reading; a lone reading falls back |
| UT181A vs UT171 ambiguity | Same framing and type byte, overlapping lengths | Wrong one of the two | Each rule declines the other's frames ([overlaps](#evidence-strength)) |
| VC650BT reported as VC-880 | Byte-identical protocol | The wrong name, readings unaffected | Pick the VC650BT chip, or `--device vc650bt` |
| UT71 or VC9x0 reported as UT804 | They send the UT804's packets | The wrong name, specs and range labels | Pick the model chip once, or `--device` |
| Unknown UT61+ name | A name not seen yet, or a future model | Reading works, tables may be off | Fallback tables, `reported_name` kept; aliases absorb variants |
| Probe side effect on the wrong meter | The UT171 connect is UT181A start recording | A recording started, a beep, or nothing | [Probe order](#the-cascade) (`send_after`); `--device` skips probing |
| Probe changes meter state | SET_MONITOR left on; the VC-890 ack burst | None expected | Both are sent in normal use anyway |
| Slow or sparse replies | Slow streamers; late Brymen answers | A missed frame in a short window | `WINDOW`, `LISTEN_ONLY_WINDOW`; the buffer persists across steps |
| Byte-at-a-time delivery | CP2110 sends one UART byte per HID report | Partial frames | Re-classify after every read; keep the buffer across steps |
| Garbage flood | Wrong baud, noisy line | Unbounded buffer | `MAX_RX_BUF` cap, oldest bytes dropped |
| Several adapters plugged in | Only the first bridge found is probed | The other meter is never seen | The existing warning; `--adapter` selects one |
| Reconnect churn in the GUI | Every reconnect attempt re-probes | Repeated beeps, slower recovery | The GUI saves the meter it detected and opens it pinned |
| Older binary reads `"auto"` | A newer settings file opened by an older binary | `unknown device: auto` | Pass `--device`, or pick any model in the GUI |
| Name probe beeps the UT61+ | `0x5F` beeps (confirmed on our UT61E+) | A beep per connect | The GUI saves the meter it detected; `--device` pins from the start |

In the GUI, a first failure shows the help and waits for Connect, while a drop mid-session
reconnects and re-probes on its own. The session that detected keeps re-probing on every
reconnect, since its opener was built before the meter answered; retrying the entry it detected
before falling back to the full cascade is a follow-up. After an unknown name, the GUI saves the
fallback entry, and its toast names the model the meter reported, which is what the user has to
quote. Auto-detect sends `0x5F`, and so beeps, whatever **Show device name on connect (beeps)**
says. A settings file needs no change on upgrade.

The probe order protects a UT181A only when its reply arrives inside its own window: one that
finishes arriving after the deadline is classified once `0x0A` has gone out. The backlog carries
that gap, and every other probe's effect on the wrong meter, for reporters to confirm.

## Adding a family

The steps are in [adding-devices.md](adding-devices.md#phase-4-implementation). Two things are
the family's own to answer. A family that needs a trigger owns the question of what that trigger
does to every other meter on the same bridge — the `0x0A` collision between the UT171 connect and
UT181A start recording is why `send_after` is a field on the fingerprint rather than a comment. A
rule that another family's frames could satisfy is the rule's own problem too: it declines what
its spec cannot produce rather than relying on being asked second.
