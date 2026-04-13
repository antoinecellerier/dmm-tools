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
- [UT803 / UT804 — proprietary structured data in FS9721 framing](research/ut803/reverse-engineered-protocol.md)

## Voltcraft

- [VC880](research/vc880/reverse-engineered-protocol.md)
- [VC890](research/vc890/reverse-engineered-protocol.md)

## Shared infrastructure

Families that use a `0xAB 0xCD` header (UT61+/UT161, UT8803, UT171,
UT181A, VC880, VC890) share the same framing skeleton: header + length
byte + payload + 16-bit big-endian sum checksum. UT8802 uses a `0xAC`
single-byte header with BCD frames, and UT803/UT804 carry proprietary
structured data inside FS9721-style framing — see the per-family docs
for the exact wire format.

The UART byte stream is transport-agnostic within each family. Three
HID bridge chips appear across the supported devices:

- **CP2110** (Silicon Labs) — bidirectional HID-to-UART, used by
  UT61+/UT161 and the UCI bench DMMs (UT8802/UT8803).
- **CH9329** (WCH) — bidirectional, driverless, found on newer UT-D09
  cables for UT181A / UT171 / UT243.
- **CH9325** (QinHeng / HE2325U) — **receive-only** HID bridge used by
  UT803/UT804 and some UT-D04 cables. Can stream meter data to the
  host but cannot send commands back.

See each per-family doc for the HID report layout and any chip-specific
initialization sequence.

For verification status and the outstanding hardware-testing backlog,
see [verification-backlog.md](verification-backlog.md).

## External reference implementations

These third-party implementations are useful cross-references when
extending the UT61E+ family support, but are **not** consumed by the
clean-room research docs under `research/` — cite them in review
discussion, not in the spec files.

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) — Python implementation (UT61E+, most complete)
- [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — Go implementation (UT61E+)
- [pylablib](https://github.com/AlexShkarin/pyLabLib) — Python implementation (VC-880)
- [Silicon Labs AN434](https://www.silabs.com/documents/public/application-notes/an434-cp2110-4-interface-specification.pdf) — CP2110/4 HID-to-UART interface specification
