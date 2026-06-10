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

No VC-890 hardware has been available; everything is decompile-derived
and tracked in the verification backlog.
