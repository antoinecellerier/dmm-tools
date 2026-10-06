# Protocol Reference Index

Each supported meter family has its own wire protocol documented under
`docs/research/<family>/reverse-engineered-protocol.md`. Those per-family
documents are the authoritative reference for transport, framing, mode
tables, flag bytes, command encoding, and hardware-verified behavior.

The protocols are documented from the vendors' own software and manuals for
interoperability: so that owners can read and control their meters from
other programs.

## UNI-T

- [UT61+ / UT161 family](research/ut61-family/reverse-engineered-protocol.md)
  — UT61B+, UT61D+, UT61E+, UT161B, UT161D, UT161E, and the UT60BT and
  UT202BT with Bluetooth built in. Covers per-model differences; defers to
  the [UT61E+ spec](research/ut61eplus/reverse-engineered-protocol.md) for
  transport, framing, commands and flag bytes.
- [UT8803 / UT8803E (bench DMM)](research/ut8803/reverse-engineered-protocol.md)
- [UCI bench family — UT8802 and transport variants](research/uci-bench-family/reverse-engineered-protocol.md) (extends the UT8803 spec with the UT8802 0xAC wire format and the CP2110/CH9325/serial transport alternatives)
- [UT171 series](research/ut171/reverse-engineered-protocol.md)
- [UT181A](research/ut181/reverse-engineered-protocol.md)
- [UT803 / UT804 — proprietary structured data in 11-byte CR LF packets](research/ut803/reverse-engineered-protocol.md)
- [UT71A–E — the UT804's packets from a handheld; covers the Voltcraft VC920/VC940/VC960](research/ut71/reverse-engineered-protocol.md)
- [UT-D07B — the Bluetooth LE adapter that carries a meter's own frames](research/ut-d07b/reverse-engineered-protocol.md)

Researched, not implemented:

- [UT632 / UT632N (bench AC millivoltmeter) — what UNI-T's software shows; no capture yet](research/ut632/reverse-engineered-protocol.md)
- [UT8805 / UT8806 (bench DMMs) — SCPI over USBTMC, LAN and RS-232](research/ut8805/reverse-engineered-protocol.md)

## Voltcraft

- [VC880 / VC650BT](research/vc880/reverse-engineered-protocol.md)
- [VC890](research/vc890/reverse-engineered-protocol.md)
- VC920 / VC940 / VC960 — UT71 rebrands, in the [UT71 spec](research/ut71/reverse-engineered-protocol.md)
- VC871 / VC891 / VC915 / VC925 PV — OWON's 15-byte frame, in the [OWON spec](research/owon/reverse-engineered-protocol.md)

## ZOTEK (ZOYI / BSIDE / ANENG)

- [ZT-300AB, ZT-5B, ZT-5BQ, ZT-5566 family — Bluetooth LE, one protocol with four packet layouts](research/zotek/reverse-engineered-protocol.md)
  — all four layouts are implemented; the ZT-5B's is verified, the others experimental. The ANENG AN9002, V05B,
  ST207 and AN999S are sold as rebrands (matched by specs and keys; AN999S ≈ ZT-5566S/SE).

## EEVblog

- [121GW — Bluetooth LE, a 19-byte binary packet and ASCII-hex key frames](research/121gw/reverse-engineered-protocol.md)
  — implemented, experimental.

## Brymen

- [BM788BT, BM787BT — Bluetooth LE, a password login and 152-byte notifications of CRC-checked packets](research/bm78xbt/reverse-engineered-protocol.md)
  — implemented, experimental.
- [BM860s, BM820s, BM520s — the BU-86X USB cable, a 3-byte request and a 24-byte LCD segment map](research/bm86x/reverse-engineered-protocol.md)
  — implemented, experimental, for live readings; the BM520s logged-memory
  download is not.

## OWON

- [OWON's Bluetooth meters — Bluetooth LE, 6-byte frames of three little-endian words, or 15-byte frames with a sub-display](research/owon/reverse-engineered-protocol.md)
  — both frames are implemented, experimental. The B33, B35T+, B41T+,
  OW16B, OW18B, OW18E and CM2100B send the 6-byte frame; the CMS101,
  CMS061, OW65B, OW67B, OW69B and the Voltcraft VC871, VC891, VC915 and
  VC925 PV the 15-byte one.

## Framing and cables

How each family delimits a frame, in short; each row links the spec section
that owns it.

| Family | Frame header | Length | Checksum |
|---|---|---|---|
| [UT61+ / UT161](research/ut61eplus/reverse-engineered-protocol.md#21-message-framing) | `AB CD` | 1 byte, counting the bytes after it | 16-bit sum of all bytes before it, big-endian |
| [UT8803](research/ut8803/reverse-engineered-protocol.md#frame-format) | `AB CD`, type `02` in byte 3 | none, fixed 21 bytes | 16-bit sum of bytes 0-18, big-endian |
| [UT8802](research/uci-bench-family/reverse-engineered-protocol.md#31-frame-format) | `AC` | none, fixed 8 bytes | none |
| [UT171](research/ut171/reverse-engineered-protocol.md#31-general-structure) | `AB CD` | 2 bytes little-endian, payload + checksum | 16-bit sum of length and payload, little-endian |
| [UT181A](research/ut181/reverse-engineered-protocol.md#3-frame-format----known) | `AB CD` | 2 bytes little-endian, payload + checksum | 16-bit sum of length and payload, little-endian |
| [UT803 / UT804](research/ut803/reverse-engineered-protocol.md#21-11-byte-packets--vendor) | none; ends `0D 0A` | none, fixed 11 bytes | none |
| [UT71, VC920 / VC940 / VC960](research/ut71/reverse-engineered-protocol.md#2-packet) | none; ends `0D 0A` | none, fixed 11 bytes | none |
| [VC880](research/vc880/reverse-engineered-protocol.md#2-frame-format----vendor), [VC890](research/vc890/reverse-engineered-protocol.md#frame-format----vendor) | `AB CD` | 1 byte, counting the bytes after it | 16-bit sum of all bytes before it, big-endian |
| [ZOTEK readings](research/zotek/reverse-engineered-protocol.md#5-framing) | `5A A5`, after an XOR descramble | none, one packet per notification | none |
| [ZOTEK commands](research/zotek/reverse-engineered-protocol.md#81-frame) | `AB CD`, then XOR-scrambled | none, fixed 10 bytes | 16-bit sum of bytes 0-7, big-endian |
| [121GW readings](research/121gw/reverse-engineered-protocol.md#4-framing) | `F2` | none, fixed 19 bytes | XOR of bytes 0-17 |
| [121GW key frames](research/121gw/reverse-engineered-protocol.md#111-key-press-f4) | `F4`, then ASCII hex | none, fixed 5 bytes | none known; the second hex pair repeats the code |
| [BM78xBT](research/bm78xbt/reverse-engineered-protocol.md#4-framing) | `FF 01` or `FF 02`; ends `FF 03` | byte 2, 32 or 24 | CRC-16/MODBUS of byte 2 up to the CRC |
| [BM86x request](research/bm86x/reverse-engineered-protocol.md#31-requests) | none | fixed 3 bytes: `00`, series code, `66` | none |
| [BM86x live reply](research/bm86x/reverse-engineered-protocol.md#41-reports-and-numbering) | none; three 9-byte HID reports | none, fixed 27 bytes | none |
| [OWON readings](research/owon/reverse-engineered-protocol.md#5-live-frame-framing) | none | none, fixed 6 bytes | none |
| [OWON 15-byte readings](research/owon/reverse-engineered-protocol.md#102-framing) | none | none, fixed 15 bytes | none |

A family's bytes are the same on every cable or adapter that relays them.
Which cable and bridge chip serve which meter is the
[cable table](supported-devices.md#cables-and-adapters) in the device
catalog; how an open picks one is [Opening a meter](architecture.md#opening-a-meter).
Each bridge's HID reports and set-up are in a family spec: the CP2110 and
CH9329 in the [UT61E+ spec](research/ut61eplus/reverse-engineered-protocol.md)
§1, the CH9325 in the [UCI bench spec](research/uci-bench-family/reverse-engineered-protocol.md)
§4 and the [UT803/UT804 spec](research/ut803/reverse-engineered-protocol.md)
§4, the BU-86X in the [BM86x spec](research/bm86x/reverse-engineered-protocol.md)
§2, and the UT-D07B Bluetooth adapter in its
[own spec](research/ut-d07b/reverse-engineered-protocol.md).

For how the library works out which family is on the wire from the bytes
it sends, see [detection-design.md](detection-design.md).

Each family's open hardware checks are in the `verification.md` beside its
spec; the checks that span families are in
[verification-backlog.md](verification-backlog.md).

## External reference implementations

Community implementations are consulted only once a family's
vendor-source analysis is complete, to validate it.
Each family's docs under `research/` cite them in a labelled
Cross-Reference section, with the date the boundary was opened; the
findings above that section come from vendor sources.

USB families:

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) — Python implementation (UT61E+, most complete)
- [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — Go implementation (UT61E+)
- [philpagel/ut8803e](https://github.com/philpagel/ut8803e) and [hskim7639/UNI-T](https://github.com/hskim7639/UNI-T) — UT8803 (its approach doc)
- [gulux/Uni-T-CP2110](https://github.com/gulux/Uni-T-CP2110) — UT171 captures and parser (UT171 spec)
- [antage/ut181a](https://github.com/antage/ut181a) and [libsigrok `uni-t-ut181a`](https://github.com/sigrokproject/libsigrok/tree/master/src/hardware/uni-t-ut181a) — UT181A protocol (its approach doc)
- [pylablib](https://github.com/AlexShkarin/pyLabLib) — Python implementation (VC-880)
- [sigrok libsigrok](https://github.com/sigrokproject/libsigrok) — C; UT71x parser and CH9325 set-up (UT803/UT804 spec §8, UCI bench spec §9)
- [sigrok wiki, WCH CH9325](https://sigrok.org/wiki/WCH_CH9325) — CH9325 configuration bytes and report framing (same sections)
- [tmatejuk/ut804_linux_logger](https://github.com/tmatejuk/ut804_linux_logger) — C, RS232 logger; its `UT804.LOG` lists real UT804 packets (UT803/UT804 spec §8)
- [Lukas Schwarz, UT61B analysis](https://lukasschwarz.de/ut61b) — HE2325U/CH9325 set-up and report format (same sections as sigrok)
- [thomasf/uni-trend-ut61d](https://github.com/thomasf/uni-trend-ut61d) — C++, `he2325u/he2325u.cpp` HE2325U/CH9325 reader (same sections)
- [Silicon Labs AN434](https://www.silabs.com/documents/public/application-notes/an434-cp2110-4-interface-specification.pdf) — CP2110/4 HID-to-UART interface specification
- libsigrok's `bm86x`/`bm52x` drivers, [TheHWcave/BM869S-remote-access](https://github.com/TheHWcave/BM869S-remote-access), [freedaun/Brymen-BM869s](https://github.com/freedaun/Brymen-BM869s), [DawOp/Brymen869s-XmlLib](https://github.com/DawOp/Brymen869s-XmlLib) and two infrared-port boards — BU-86X and BM86x (BM86x spec §13)

Bluetooth families; each spec's section has the full list:

- [webspiderteam/Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows), [libreble/multimeter](https://github.com/libreble/multimeter) and [olegv142/ut61xpy](https://github.com/olegv142/ut61xpy) — the UT-D07B (its spec §7), the UT61+ over it (UT61E+ spec §7) and the UT60BT (UT61+ family spec §10, with [QtDMM](https://github.com/qtdmm/QtDMM))
- [ludwich66/Bluetooth-DMM](https://github.com/ludwich66/Bluetooth-DMM), webspiderteam and libreble — ZOTEK's four packet layouts (ZOTEK spec §11)
- [tpwrules/121gw-re](https://github.com/tpwrules/121gw-re), [zonque/121gw-qt5](https://github.com/zonque/121gw-qt5), [chlordk/121gwcli](https://github.com/chlordk/121gwcli) and libsigrok's `eev121gw` — the 121GW (121GW spec §15)
- [milksplash/brymenble](https://github.com/milksplash/brymenble) and two programs built on it — the BM78xBT (BM78xBT spec §12)
- [DeanCording/owonb35](https://github.com/DeanCording/owonb35), [sercona/Owon-Multimeters](https://github.com/sercona/Owon-Multimeters), [pjpa365/owon-suite](https://github.com/pjpa365/owon-suite), webspiderteam and others — the OWON meters (OWON spec §14)
