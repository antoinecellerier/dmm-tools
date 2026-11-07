# VC-890 Reverse-Engineering Approach

The VC-890 spec was derived from the same primary source as the VC-880:
the ILSpy decompilation of Voltsoft's `DMSShare.dll`
(`references/vc880/vendor-software/DMSShare_decompiled.cs`), which
contains both device implementations (`VC890Obj` / `VC890Reading`
alongside `VC880Obj` / `VC880Reading`).

See [../vc880/reverse-engineering-approach.md](../vc880/reverse-engineering-approach.md)
for how that decompile was obtained and validated. The VC-890-specific
work consisted of diffing the two class pairs to catch the remapped
function codes, the polled (0x5E) communication model, the 66-byte
frame layout, the ack protocol, and the relocated status bits — see
the [protocol spec](reverse-engineered-protocol.md) for results and
confidence markers.

Conrad's VC890 Protocol Rev 1.3 (2013-1-4, `references/vc890/protocol/`)
was read 2026-09-28, from rendered pages, for its handshake only: the
Result message and whether any timing is given (pp. 1-2, 9-13). Its
[VENDOR-DOC] lines are in the spec's Communication Model. Page 6 (bytes
59-63) was read 2026-09-30, from a rendered page, into the spec's Live Data
Frame. The remaining pages (3-5, 7-8, 10-12) were read 2026-11-06 and
2026-11-07, from rendered pages, in two independent reads that also took in
the VC-890 manual's range tables (printed pages 84-85, rendered) and
Voltsoft's `DeviceClient.exe` and `DeviceServer.exe` (decompiled with
`ilspycmd` from the same installer) and its web pages (`Web Server/DeviceWeb/`,
shipped as source): the display fields, status-byte names, range-table
conflicts, full command list, command confirmation and battery icons in the
spec come from them. `VoltSoft System.exe` holds no VC model strings and was
not decompiled. Every Voltsoft source here comes from the VC-880 installer;
whether the VC-890's own installer (item 124600) ships a different
`DMSShare.dll` is unknown, as it was not unpacked.

No VC-890 hardware has been available; everything else is
decompile-derived, and its open checks are in
[verification.md](verification.md).
