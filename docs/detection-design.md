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
[Device auto-detection](verification-backlog.md#device-auto-detection) section.

## What changed since the issue was filed

| Issue #9 assumption | Now |
|---|---|
| CH9329 (`1A86:E429`) → UT181A shortcut | Wrong. A UT61B+ was verified over CH9329 (issue #19, commit `1d078a6`). CH9329 and CP2110 need the same cascade. |
| UT8803 needs a `0x5A` trigger | Stale. Removed in the 2026-06 review; UT8803/UT8802 stream unprompted (`ut8803/mod.rs`). |
| Get Name `0x5F` unverified beyond UT61E+ | Verified on UT61B+ too, over CH9329 ([verification-backlog.md](verification-backlog.md)). UT61D+/UT161x still unverified. |
| 4 families | 8 hardware families plus the CH9325 bridge (UT803/UT804). VC-880/VC-890 share the UT61E+'s `AB CD` BE16 framing; VC-890 even polls with `0x5E`. |
| No captures | 12 UT61E+ (CP2110) and 2 UT61B+ (CH9329) capture reports under `references/`, all with `init_frames`; UT181A traces in PR #8 (full CH9329 trace) and issue #5 (capture samples); 3 real UT181A frames in `ut181a/parse.rs` and `tests/golden/ut181a/`. UT804 packets in issue #16. Nothing for UT171, UT8802, UT8803, UT803, VC-880, VC-890. |

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

On CP2110 and CH9329 each step is an optional transmission followed by a listen window. The
whole buffer is re-classified after every read, so a meter that streams on its own is
identified in whichever window it first speaks — the unprompted families need no step of their
own.

| Step | TX | Identifies |
|---|---|---|
| 1 | `AB CD 03 5F 01 DA` (UT61+ Get Name) | UT61+/UT161 model, by name |
| 2 | `AB CD 04 00 05 01 0A 00` (UT181A SET_MONITOR) | UT181A |
| 3 | `AB CD 04 00 0A 01 0F 00` (UT171 connect) | UT171 |
| 4 | 3× `AB CD 04 FF 00 02 7B` then `AB CD 03 5E 01 D9` (VC-890 poll) | VC-890 |
| any | — (nothing sent) | UT8803, UT8802, VC-880; on Bluetooth, the ZOTEK meters |

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

A family's `Fingerprint` (`protocol/mod.rs`) is a log label, an optional trigger — byte-identical
to what the family's own `init` sends — the families that trigger has to follow (`send_after`),
whether its extractor validates a checksum, and a rule that classifies the whole receive buffer
with that family's constants. Which of them run is the registry's call: every `DEVICES` entry
points at its family's fingerprint, and detection runs the ones the bridge carries, in table order.

A rule answers `Evidence::Model` (a registry id, plus the name where the meter sent one) or
`Evidence::FamilyOnly` (the family is settled, no model named). Every carried rule runs after
every read, against every candidate offset in the buffer, and the strongest answer wins:

| Rank | Evidence | Where it comes from |
|---|---|---|
| 4 | a model the meter named itself | the UT61+ name frame, the only one that picks an exact sibling |
| 3 | a model from a frame whose checksum held | UT8803, UT171, UT181A, VC-880, VC-890 |
| 2 | `FamilyOnly` — a checksummed frame naming no model | a bare 14-byte UT61+ reading |
| 1 | a model from a rule that validates no checksum | UT8802 (`0xAC`), UT804 (any CR LF packet), ZOTEK (a whole descrambled packet) |

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

- `ut8803` — byte 3 is `0x02` and the 21-byte checksum holds. A UT61+ DC V frame also carries
  `0x02` at byte 3, but it is 19 bytes long, so its checksum fails there.
- `ut181a` and `ut171` — the 2-byte-LE pair, whose framing and measurement type byte (`0x02`) are
  identical and whose payload lengths overlap. The UT181A takes a payload ≥ 31 bytes (only it can
  send one) and anything its own trigger elicited. The UT171 declines exactly those two: a payload
  past 21 bytes is longer than its own extended frame
  ([ut171 §3.4/§5.2](research/ut171/reverse-engineered-protocol.md)), and a frame arriving right
  after SET_MONITOR is the one that command asked for. It takes the rest, with a WARN when nothing
  had been sent yet. Payload `[0] == 0x01` is an OK/ER reply and is ignored. Neither can fire on
  the other families: a UT61+ name frame reads a length of `0x5508`, a VC-880 frame `0x0124`.
- `vc880`, `vc890` and `ut61eplus` — the 1-byte BE16 families. A UT8803 frame's byte 2 is a mode
  byte that reads as a plausible length here, which is what the checksum settles. Payload
  `[0] == 0x01` with length 34 is `vc880`, length 61 `vc890`. `FF 00` is an ack — skipped.
  Printable ASCII of length 3..=20 is a UT61+ name; a 14-byte payload is a UT61+ reading, which
  settles the family only (`FamilyOnly`, fallback `ut61eplus`).
- `ut8802` — `0xAC` offsets, two consecutive `extract_frame_ut8802` hits exactly 8 bytes apart.
  That format's validation passes roughly 1% of random bytes and UT181A frames carry arbitrary
  float32 payload, which is why an unchecksummed claim ranks below even a settled family.

`ut80x` is the CH9325's rule and is the only one consulted there; the AB CD rules are the other
bridges' (see [Bridges and adapters](#bridges-and-adapters)).

## Names and the registry

A name frame resolves through `registry::device_for_reported_name(name)`, which matches
`display_name` then aliases, case-insensitively, across the UT61+/UT161 entries only. An
unrecognised name falls back to `ut61eplus`, keeps the reported name on `Detected`, and warns:
a UT61D+ or a UT161x still reads, with UT61E+ tables, and the user is told which name to
report so an alias can absorb it.

A meter with Bluetooth built in names itself before any frame. `Ble` reports the name the peer
advertised (`Transport::advertised_name()`), and `built_in_meters()` in `lib.rs` looks it up in
each entry's `bluetooth_names` (`registry::advertising()`, the prefix rule the search takes the
peer by). When entries match, detection runs only their fingerprints: a UT60BT gets Get Name and
never the UT181A's or the UT171's probes. An adapter's name, or a peer opened by address with no
name heard, matches none and gets the bridge's whole cascade. When exactly one entry matches, a
`FamilyOnly` fallback, and an unrecognised name frame too, opens that entry rather than
`ut61eplus`, since a UT60BT's V position starts at 999.9mV where the UT61E+'s starts at 2.2V;
the WARN says the tables came from the advertised name. A name frame that resolves still
outranks the advertised name; if the two disagree, a WARN names both and the name frame's entry
opens. A name several entries share narrows the probes to theirs but picks no model: the frames
decide, and a frame naming none gets the fingerprint's own fallback.

With nothing identified, `detect_device` returns `Error::DeviceNotIdentified { bridge }`, whose
`kind()` is `ErrorKind::Timeout` — reconnecting after enabling transmission on the meter does
help. Its `Display` names the link the probe ran over — the USB cable or the Bluetooth
adapter — never the bridge chip.

## Bridges and adapters

Which rules a bridge gets is the registry's call, not `detect.rs`'s: `preferred_transports()` in
`lib.rs` lists the cables each family is found on, and detection runs only the fingerprints of the
families listed on the bridge it opened — the same list the "no meter answered" help draws on. The
shortcut issue #9 assumed (CH9329 means UT181A) does not hold: a UT61B+ is verified over CH9329
and older UT181A units ship the CP2110, so CP2110 and CH9329 run the same cascade. The CH9325
carries the UT80x family alone — the UT803/UT804, and the UT71 and Voltcraft VC920/VC940/VC960,
which send the UT804's packets; it is receive-only past its init, which already sends `0x5A`,
so detection there is a single listen window that takes any whole CR LF packet as a UT804. A
packet does not name its model, so a UT71 or VC9x0 is claimed as a UT804 and has to be named
(the GUI's device chip, saved once, or `--device ut71ab` / `ut71cde` / `vc920`); and the CH9325
starts at 2400 baud, where only the UT804 is heard: a UT803 talks at 19200, so it is not detected
and has to be named (`--device ut803`). A family seen on a new cable joins detection there by
being listed on it. The UT-D07B Bluetooth adapter is a transparent UART bridge like the cables, so the
UT61+, UT171 and UT181A fingerprints listed on it run unchanged. It is
opened after the USB bus, so a cable with a silent meter on it wins over a live meter on the
adapter. The ZOTEK meters have the radio built in and stream on their own, so their rule sends
nothing and reads whatever window is open: it takes one whole packet, found by its scrambled
header, cut at its type's length and made only of listed glyphs, and the packet's type byte picks
the registry entry for that layout — the meter never names its model.

Only the first adapter found is probed. With several plugged in, the existing
multiple-adapter warning applies and `--adapter` selects one; probing every adapter is a
follow-up, not part of this design.

## Timing

A listen window is bounded twice: an `Instant` deadline of ~600 ms (hardware pacing, per
[protocol.md](protocol.md)) **and** an empty-read cap (`MAX_EMPTY_READS` = 256 reads in a row without data, as in
`framing::read_uart_bytes`), because a drained `MockTransport` returns `Ok(0)` instantly and
would otherwise spin until the deadline. Four windows put the walk to "not identified" at
≈2.6 s, while a UT61+ answers inside the first one — ack in 36–83 ms, name in 144–191 ms — so
an auto open normally costs about 200 ms.

A window that follows no probe — the CH9325's only one — lasts 1.5 s instead. The meter sets
the pace there, and issue #16's UT804 sends a packet every 656 ms: a 600 ms window holds a whole
one in about five runs in six, 1.5 s gives two chances. The cap still outlasts it, since the
CH9325's idle report comes every ~12 ms and 256 of them take ~3 s.

The buffer persists across steps, capped at 4096 bytes with the oldest dropped. That is what
lets bytes arriving one per HID report assemble into a frame across a window boundary, and
what gives a slow streamer (UT8803 at 2–3 Hz) the whole ≈2.6 s budget instead of one window's
worth. Extractor errors are ignored, never propagated.

## Failure modes and mitigations

| Failure mode | Cause | What the user sees | Mitigation |
|---|---|---|---|
| Nothing answers | Meter off; Communication OFF (UT171/UT181A); PC button not pressed (VC-880); wrong cable; CH9325 at the wrong baud; a meter whose arm is wrong | `DeviceNotIdentified` after ~2.6 s (1.5 s on the CH9325) | Help lists the activation instructions of every family on that bridge and ends with how to name the meter yourself and where to report it; `--device <id>` pins a model and skips probing; in the GUI a first failure shows the help and waits for Connect, while a drop mid-session reconnects and re-probes on its own |
| Misidentification from junk | Random bytes passing a lax extractor (the `0xAC` 8-byte UT8802 format passes ~1% of random input); garbage from a wrong CH9325 baud | Wrong parser, later checksum or parse errors | Checksummed evidence outranks a pattern match, and a named model outranks both; UT8802 needs two consecutive frames 8 bytes apart, so its claim only stands when no `AB CD` rule made a stronger one; the "Detected X" notice tells the user what was picked and that `--device` overrides it |
| Stale frame from an earlier session | CH9329 does not purge RX on open; a UT61+ mid-poll or a UT181A left streaming | Family evidence arriving before the probe reply | A name frame outranks a measurement frame within the window; a lone 14-byte UT61+ frame falls back to `ut61eplus`, or to the entry a meter with Bluetooth built in advertises, with `reported_name: None` and a WARN |
| UT181A vs UT171 ambiguity | Same framing and type byte; payload lengths overlap (UT181A 19 bytes without aux or bargraph, UT171 16/22) | Wrong one of the two | Payload ≥ 31 → UT181A; a payload past 21 bytes is past the UT171's extended frame, and a frame right after SET_MONITOR is the UT181A's, so the UT171 rule declines both; what is left before any LE16 trigger → UT171 with a WARN; recorded in the backlog; parse-based arbitration once UT171 hardware exists |
| VC650BT reported as VC-880 | The two Voltcraft meters speak a byte-identical protocol, so no frame tells them apart | The wrong name on screen, readings unaffected | Pick the VC650BT chip or `--device vc650bt` to carry the right name; same tables either way |
| UT71 or VC9x0 reported as UT804 | The handhelds send the UT804's packets, so no frame tells them apart | The wrong name, the UT804's specs and range labels (a UT71A/B's 2 V range reads 4V), readings unaffected | Pick the UT71A/B, UT71C/D/E or Voltcraft VC920 chip once — the GUI saves it — or `--device ut71ab` / `ut71cde` / `vc920`; same decoding either way |
| Unknown UT61+ name | UT61D+/UT161x/UT60BT/UT202BT names never seen; a future model | Reading works, tables may be off | Fall back to `ut61eplus` tables (on a meter with Bluetooth built in, to the entry its advertised name picked), keep `reported_name`, notice "meter reports X, using UT61E+ tables"; ask the user to report the name; registry aliases absorb spelling variants. What the GUI saves is that fallback entry, and its toast names the model the meter reported, which is what the user has to quote |
| Probe side effect on the wrong meter | The UT171 connect is UT181A opcode `0x0A` (start recording); SET_MONITOR `0x05` meaning on a UT171 unknown; `0x5F` on VC-8x0/UT171/UT181A unknown | A recording started, a beep, or nothing | Order: `0x5F` first (registry order puts the most common meter first, and it is the most verified probe, replying within 200 ms), the UT171's `send_after` putting the UT181A trigger before the connect, so a UT181A with Communication ON that answers inside its own window is identified before `0x0A` goes out — a reply that only finishes arriving after that window's deadline is not, and the backlog carries the gap; one with Communication OFF ignores everything; each exposure listed in the backlog for reporters to confirm; `--device` avoids probing entirely |
| Probe changes meter state | SET_MONITOR left on; the VC-890 ack burst | None expected | SET_MONITOR is what the UT181A init sends anyway; the acks are what the vendor software sends before every command |
| Slow or sparse replies | UT8803 streams at 2–3 Hz; UT804 a packet every 656 ms; VC-880 rate unknown; UT61+ name seen within 191 ms | A missed frame in a short window | Windows ≥ 600 ms, 1.5 s where nothing is sent; the buffer persists across steps, so a streamer gets the whole ≈2.6 s budget and a frame split across a window boundary survives |
| Byte-at-a-time delivery | CP2110 delivers one UART byte per HID report | Partial frames | Accumulate and re-classify after every read; never clear the buffer on a step boundary |
| Garbage flood | Wrong baud, noisy line | Unbounded buffer | 4096-byte cap, oldest bytes dropped; extractor errors ignored, never propagated |
| Several adapters plugged in | Only the first bridge found is probed | The other meter is never seen | The existing warning; `--adapter` selects one; probing every adapter is a listed follow-up, not in scope |
| Reconnect churn in the GUI | Every reconnect attempt re-probes: a UT181A beeps each time; ~2.6 s per attempt while nothing answers | Repeated beeps, slower recovery | The GUI saves the meter it detected, so every later session — reconnects included — opens it pinned; the session that detected keeps re-probing, since its opener was built before the meter answered. Follow-up: retry the entry detected earlier in the session before falling back to the full cascade |
| Older binary reads `"auto"` | A settings file written by this version opened by an older `dmm-cli`/`dmm-gui` | `unknown device: auto` | Upgrading needs no step, so no migration line; on a downgrade, pass `--device` or pick any model in the GUI, which rewrites the file |
| Name probe beeps the UT61+ | `0x5F` beeps (confirmed on our UT61E+), so every auto connect beeps regardless of the GUI's name-query toggle | A beep per connect | The GUI saves the meter it detected as the device, so later sessions open pinned and the beep goes back under the GUI's name-query toggle — which the probe overrides while detecting; `--device` pins the model the same way from the start; documented in `docs/gui-reference.md` and `docs/cli-reference.md` |

## Adding a family

A family joins detection with four things: a `Fingerprint` exported from its own module, built
from the constants it already sends and parses with, declaring whether its extractor checks a
checksum and which families its trigger has to follow, and pointed at from every one of that
family's `DEVICES` entries — nothing is added to `detect.rs`; a test beside that rule, over real
bytes where hardware exists and the vendor trace otherwise; a row in the cascade table above, or
in its unprompted line if the meter streams by itself; and a line in the backlog's
[Device auto-detection](verification-backlog.md#device-auto-detection) section saying what is
sent, what is expected back, and whether hardware has confirmed it. A family that needs a trigger
also owns the question of what that trigger does to every other meter on the same bridge — the
`0x0A` collision between the UT171 connect and UT181A start recording is why `send_after` is a
field on the fingerprint rather than a comment. A rule that another family's frames could satisfy
is the rule's own problem too: it declines what its spec cannot produce rather than relying on
being asked second.
