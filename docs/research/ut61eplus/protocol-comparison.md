# Protocol Comparison: Vendor RE vs Repo Implementation

Comparison between the clean-room reverse-engineered protocol
(`references/reverse-engineered-protocol.md`) and the existing protocol
documentation (`docs/protocol.md` + `docs/verification-backlog.md`).

Verified against live device on 2026-03-19.

## Full Agreement

The reverse-engineered protocol confirms every core aspect of the
existing implementation. Nothing in the repo needs to change for
correctness.

| Aspect | Repo | Vendor RE | Live Device |
|--------|------|-----------|-------------|
| VID/PID | 0x10C4/0xEA80 | 0x10C4/0xEA80 | ✓ |
| Baud rate | 9600 8N1 | 9600 8N1 | ✓ |
| Init: enable UART | [0x41, 0x01] | Same | ✓ |
| Init: configure | [0x50, ...9600 BE...] | Same | ✓ |
| Init: purge RX | [0x43, 0x02] | Same | ✓ |
| Frame header | AB CD | AB CD | ✓ |
| Length byte | payload + 2 | payload + 2 | ✓ |
| Checksum | 16-bit BE sum | 16-bit BE sum | ✓ |
| GetMeasurement | 0x5E | 0x5E | ✓ |
| Response | 19 bytes total | 19 bytes total | ✓ |
| Mode byte | raw, no prefix | raw, no prefix | 0x02 for DCV ✓ |
| Range byte | 0x30 prefix, & 0x0F | 0x30 prefix | 0x30 for range 0 ✓ |
| Display | 7 ASCII, space-padded | 7 ASCII Latin-1 | " 0.0057" ✓ |
| Bar graph | raw, no prefix | not parsed | 0x00 0x00 ✓ |
| Flag bytes | 0x30 prefix, & 0x0F | bit-level extraction | 0x30 0x30 0x30 ✓ |
| Flag 1: REL/HOLD/MIN/MAX | bits 0/1/2/3 | bits 0/1/2/3 | ✓ |
| Flag 2: !AUTO inverted | bit 2 | bit 2, inverted | ✓ |
| Flag 3: P-MIN/P-MAX/DC | bits 1/2/3 | bits 1/2/3 | ✓ |
| Mode values 0x00-0x14 | match | match | ✓ |
| Polled model | request/response | LoopCommandPool | ✓ |
| Command format | AB CD 03 cmd chk_hi chk_lo | Same | ✓ |

## Discrepancies Found

### 1. Mode byte collisions in quirks section — WRONG

The repo's "Known Quirks" section claims:
- "NCV shares mode byte with Hz" (both 0x04)
- "hFE shares mode byte with DCV" (both 0x02)
- "DC A shares mode byte with ACV" (both 0x00)

**These collisions do not exist.** The repo's own mode table AND the
vendor software both assign unique bytes: NCV=0x14, hFE=0x12, DCA=0x10.
The verification backlog confirms these were individually verified
against the real device.

The quirks section contradicts the mode table and the verification data.
These three quirks should be removed.

**Action:** Delete the three collision quirks from `docs/protocol.md`
and the "Mode byte collisions" section from `docs/verification-backlog.md`.

### 2. Modes 0x15-0x19 — minor naming differences

| Byte | Repo | Vendor RE | Notes |
|------|------|-----------|-------|
| 0x15 | LoZ V | LozV | Same, different spelling |
| 0x16 | AC+DC A | LozV | **Disagreement** — vendor says 2nd LoZ mode |
| 0x17 | AC+DC/DC A | LPF | **Disagreement** — vendor says LPF |
| 0x18 | LPF V | (not in vendor table) | Vendor table ends before 0x18 |
| 0x19 | AC+DC V | AC+DC | Same |

The repo maps 0x16/0x17 to current measurement variants (AC+DC A); the
vendor software maps them to voltage modes (LoZ, LPF). These modes are
unverified on real device (marked "—" in the mode table). Since the vendor
software has the mode name strings embedded, its mapping is likely correct
for these unverified entries.

**Action:** Mark 0x16-0x18 as uncertain in `docs/protocol.md`. Verify
against real device when possible.

### 3. Modes 0x1A-0x1E — not in vendor software

The repo lists 5 additional modes (LPF mV, AC+DC mV, LPF A, AC+DC A,
Inrush) that don't appear in the vendor software's mode table at all.
These may have been added in a later firmware version, or derived from
community implementations.

**Action:** No change needed — they're already marked unverified.

## New Information from Vendor RE

### Findings that could improve the repo

1. **Bar graph full-scale table** — The vendor software stores the
   bar graph full-scale count for each mode/range entry:

   | Mode/Range | Full-scale |
   |------------|-----------|
   | ACV 220mV (0x00/0x30) | 6 |
   | ACV 2.2V (0x00/0x31) | 60 |
   | ACV 22V (0x00/0x32) | 600 |
   | ACV 220V (0x00/0x33) | 1000 |
   | DCV 220mV (0x02/0x30) | 6 |
   | DCV 2.2V (0x02/0x31) | 60 |
   | DCV 22V (0x02/0x32) | 600 |
   | DCV 220V (0x02/0x33) | 1000 |
   | Ohm 220Ω (0x06/0x30) | 600 |
   | Ohm 2.2kΩ (0x06/0x31) | 6 |
   | ... | ... |

   This could be used to scale bar graph display in the GUI.

2. **SI prefix lookup table** — The vendor software builds an explicit
   prefix table: T, G, M, k, K, (space), (empty), m, µ, n, p. This
   maps range entries to unit prefixes for display formatting.

3. **Read/write timeouts** — 100ms each. The repo uses timeouts but
   doesn't document the specific values the vendor chose.

4. **OL detection** — The vendor software checks for "O" and "L"
   substrings in the display string, with separate handling for
   negative OL (display contains "-" as well).

5. **Vendor command set is minimal** — V2.02 only sends 0x5E
   (GetMeasurement), 0x4A (Hold), 0x46 (Range). The 10 other commands
   in the repo were discovered through direct device testing, which is
   more thorough than the vendor software.

### Findings that confirm repo decisions

- The vendor software strips spaces from display before parsing —
  same as the repo's approach.
- The vendor software uses `toDouble` for value parsing — consistent
  with the repo using `f64`.
- The vendor software defaults to "UT61B+" as the model string,
  confirming the protocol is shared across the UT61 B+/D+/E+ family.

## Summary

The repo's protocol implementation is **correct**. The only issue is
three stale quirks about mode byte collisions that contradict the repo's
own verified mode table. The vendor RE provides independent confirmation
of essentially every protocol detail, plus some new information (bar
graph full-scale table, SI prefix mapping, OL detection specifics) that
could enhance the implementation.
