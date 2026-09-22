# Protocol Reference Index

Each supported meter family has its own wire protocol documented under
`docs/research/<family>/reverse-engineered-protocol.md`. Those per-family
documents are the authoritative reference for transport, framing, mode
tables, flag bytes, command encoding, and hardware-verified behavior.

## UNI-T

- [UT61+ / UT161 family](research/ut61-family/reverse-engineered-protocol.md)
  — UT61B+, UT61D+, UT61E+, UT161B, UT161D, UT161E. Covers per-model
  differences; defers to [research/ut61eplus/reverse-engineered-protocol.md](research/ut61eplus/reverse-engineered-protocol.md)
  for the full CP2110/CH9329 transport, `AB CD` framing, request/response
  format, flag bytes, sampling rate, and implementation quirks.
- [UT8803 / UT8803E (bench DMM)](research/ut8803/reverse-engineered-protocol.md)
- [UCI bench family — UT8802 and transport variants](research/uci-bench-family/reverse-engineered-protocol.md) (extends the UT8803 spec with the UT8802 0xAC wire format and the CP2110/CH9325/serial transport alternatives)
- [UT171 series](research/ut171/reverse-engineered-protocol.md)
- [UT181A](research/ut181/reverse-engineered-protocol.md)
- [UT803 / UT804 — proprietary structured data in 11-byte CR LF packets](research/ut803/reverse-engineered-protocol.md)
- [UT71A–E — the UT804's packets from a handheld; covers the Voltcraft VC920/VC940/VC960](research/ut71/reverse-engineered-protocol.md)

## Voltcraft

- [VC880](research/vc880/reverse-engineered-protocol.md)
- [VC890](research/vc890/reverse-engineered-protocol.md)
- VC920 / VC940 / VC960 — UT71 rebrands, in the [UT71 spec](research/ut71/reverse-engineered-protocol.md)

## Shared infrastructure

Families that use a `0xAB 0xCD` header share a framing skeleton but
differ in the details: UT61+/UT161, UT8803, VC880, and VC890 use a
1-byte length plus a 16-bit **big-endian** sum checksum, while UT171
and UT181A use a 2-byte **little-endian** length (counting payload +
checksum) plus a 16-bit **little-endian** sum. UT8802 uses a `0xAC`
single-byte header with BCD frames and no checksum, and UT803/UT804 —
and the UT71 and VC920/VC940/VC960 with them — send proprietary
structured data in 11-byte packets ending CR LF — see the per-family
docs for the exact wire format.

The UART byte stream is transport-agnostic within each family. Three
HID bridge chips and one Bluetooth adapter appear across the supported
devices:

- **CP2110** (Silicon Labs) — bidirectional HID-to-UART, used by
  UT61+/UT161 and the UCI bench DMMs (UT8802/UT8803).
- **CH9329** (WCH) — bidirectional, driverless, found on newer UT-D09
  cables for UT181A / UT171 / UT243, and reported on a UT61B+.
- **CH9325** (QinHeng / HE2325U) — HID-to-UART, used by UT803/UT804
  and some UT-D04 cables; the UT71 apps set their cable up as one, and
  which chip the UT71 cable and Voltcraft's VC9x0 USB adapter carry is
  unverified ([UT71 spec](research/ut71/reverse-engineered-protocol.md)
  §1.1). These meters stream without a host request
  and no command for them is known; the UNI-T SDK writes one `0x5A`
  byte at init, and the host-to-meter report framing is
  community-sourced ([UCI bench spec](research/uci-bench-family/reverse-engineered-protocol.md) §4.2, §9).
- **UT-D07B** (ISSC/Microchip) — a Bluetooth LE transparent-UART
  adapter rather than a chip in a cable, carrying the same bytes the
  cable does for the meter series UNI-T lists on it
  ([UT-D07B spec](research/ut-d07b/reverse-engineered-protocol.md)).

See each per-family doc for the HID report layout and any chip-specific
initialization sequence.

For how the library works out which family is on the wire from the bytes
it sends, see [detection-design.md](detection-design.md).

For verification status and the outstanding hardware-testing backlog,
see [verification-backlog.md](verification-backlog.md).

## External reference implementations

Community implementations are consulted only after a family's
vendor-source analysis, with the maintainer's approval, for validation.
Each family's docs under `research/` cite them in a labelled
Cross-Reference section, with the date the boundary was opened; the
findings above that section come from vendor sources.

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) — Python implementation (UT61E+, most complete)
- [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — Go implementation (UT61E+)
- [pylablib](https://github.com/AlexShkarin/pyLabLib) — Python implementation (VC-880)
- [sigrok libsigrok](https://github.com/sigrokproject/libsigrok) — C; UT71x parser and CH9325 set-up (UT803/UT804 spec §8, UCI bench spec §9)
- [sigrok wiki, WCH CH9325](https://sigrok.org/wiki/WCH_CH9325) — CH9325 configuration bytes and report framing (same sections)
- [tmatejuk/ut804_linux_logger](https://github.com/tmatejuk/ut804_linux_logger) — C, RS232 logger; its `UT804.LOG` lists real UT804 packets (UT803/UT804 spec §8)
- [Lukas Schwarz, UT61B analysis](https://lukasschwarz.de/ut61b) — HE2325U/CH9325 set-up and report format (same sections as sigrok)
- [thomasf/uni-trend-ut61d](https://github.com/thomasf/uni-trend-ut61d) — C++, `he2325u/he2325u.cpp` HE2325U/CH9325 reader (same sections)
- [Silicon Labs AN434](https://www.silabs.com/documents/public/application-notes/an434-cp2110-4-interface-specification.pdf) — CP2110/4 HID-to-UART interface specification
