# UT8805 / UT8806 verification

Open questions before a SCPI implementation for the UT8805N, UT8805A,
UT8805E, UT8806, UT8806A and UT8806E. The family is specified, not
implemented, and has no issue: nobody on the project owns one, so these are
for whoever implements it or owns a meter.
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md) (nothing yet); checks that span
families are in the [verification backlog](../../verification-backlog.md).

## First report

- `lsusb -v` (the VID, the FE/03/01 interface and the string descriptors of
  [§2.1](reverse-engineered-protocol.md#21-device-identity--vendor)), a raw `*IDN?` over USB and a port scan
  (80, 111, 5025, 49152) — settle most items below. Needs any model, noting its version.

## USB

- The VID as enumerated: 0483 in every descriptor ([§2.1](reverse-engineered-protocol.md#21-device-identity--vendor)),
  0486 on a UT8805E's web page (§14) — decides the UT8805N/E ID. Needs
  `lsusb -v` on a UT8805E.
- The UT8806A's firmware line and VID:PID, no image fetched (§1, §11) —
  decides its ID and command map. Needs `lsusb -v` and `*IDN?` on a UT8806A.
- Whether any unit enumerates as the manuals' `0x5345:0x1234`, which no image
  carries (§2.1) — decides whether that ID is ever looked for. Needs `lsusb`.
- GET_CAPABILITIES bytes as sent ([§2.2](reverse-engineered-protocol.md#22-usbtmc-handling--vendor))
  — confirms no USB488 capability and no TermChar. Needs the request on an N
  and an H7 unit.

## LAN

- TCP 80 on the N line: a V1.87.001 unit serves a page, the V1.87.014 image
  has no web strings (§14, [§3.1](reverse-engineered-protocol.md#31-services-per-firmware-line--vendor-firmware-summary-table))
  — settles §3.1's N column. Needs a port scan per UT8805N/E firmware version.
- 5025 and 80 on the H7 line, as its images say (§3.1) — decides whether the
  raw socket reaches the UT8805A and UT8806s. Needs a port scan on one.
- Which UT8805E versions send VXI-11 replies unpadded to 4 bytes: V1.87.014
  did, V1.87.017 reportedly pads (§14) — decides whether a VXI-11 client must
  tolerate it. Needs `*IDN?` over VXI-11, noting the version.
- The GPIB address setting, in no user manual, and the E models' GPIB option
  ([§1](reverse-engineered-protocol.md#1-models-and-interfaces)) — settles
  §1's port row. Needs a unit with the option.

## Message layer

- The `*IDN?` bytes: separators, the `SW ` prefix, the SN field and
  `FACTORY:MIS:MODEL/SN` as their source ([§5.3](reverse-engineered-protocol.md#53-idn), §14)
  — decides identification. Needs each line.
- The reply terminator, `\r\n` on the N and `\n` on the H7 line from firmware
  ([§5.1](reverse-engineered-protocol.md#51-terminators)) — decides the reply
  split. Needs any query on each line.
- The RS-232 reply terminator and handshake; SCPI itself works there on a
  UT8805E ([§4](reverse-engineered-protocol.md#4-rs-232), §14) — decides the
  serial link settings. Needs `*IDN?` over the DB9.
- `CONF?` and `FUNC?` bytes: firmware `VOLT:DC +2.0…E+01` and `"VOLT:DC"`, the
  manuals otherwise ([§6.2](reverse-engineered-protocol.md#62-configure-and-function-replies);
  `FUNC?` leans quoted, §14) — decides the function parser. Needs each line.

## Remote and local

- What enters remote on each line, and whether the keys lock
  ([§9](reverse-engineered-protocol.md#9-remote-and-local)) — decides what a
  session does to the panel. Needs queries per line, watching the panel.
- Whether `*UNREMOTE` releases the N line (sent without a reported error,
  §14) and `SYST:LOC` the UT8806 (§9) — decides how a session hands the meter
  back. Needs each line.

## Readings

- `DATA:LAST?` on the free-running panel without `INIT`, and its unit token
  per function ([§7.1](reverse-engineered-protocol.md#71-documented-queries--known), §8)
  — decides the poll query. Needs `DATA:LAST?` across the functions.
- The undocumented commands in [§7.2](reverse-engineered-protocol.md#72-registered-but-undocumented--vendor-normtxt-scpitxt)
  and §3.1's `SYST:COMM:TCPIP:CONTROL?`: what each returns or changes —
  settles §7.2 and the poll query. Needs an N and an H7 unit.
- The reading-memory size: 1,000 or 10,000, 50k on the UT8806A (§7.1) —
  settles §7.1's figure. Needs `DATA:POINts?` with the memory full.
- Overload by sign on each line, the UT8805A's `READ?` sign quirk, the `*`
  invalid marker and NaN ([§8](reverse-engineered-protocol.md#8-reply-formats-and-special-values))
  — decides the overload decoding. Needs OL in both polarities per line.
- Continuity and diode replies with the leads open (§8) — decides how those
  read as OL. Needs `MEAS:CONT?` and `MEAS:DIOD?` on any model.
- Whether dB/dBm scaling, limits or statistics change `READ?` (§8) — decides
  whether a reading needs the math state. Needs each turned on.

## Configuration and ranges

- Range ladders per model ([§6.3](reverse-engineered-protocol.md#63-ranges-and-terminals--known)):
  the UT8806 user manual against S6E, the UT8805 capacitance top (leans 2 mF,
  §14), the UT8806A's "2→1" figures (§11) — gate the spec tables. Needs each.
- The UT8806A's `FRES:ZERO:AUTO` and AC bandwidth set, "2→1" too (§11) —
  settles §11's UT8806A row. Needs a UT8806A.
- The UT8806A NPLC list and the N/E thermocouple types, EP p.21 against
  p.27 ([§6.4](reverse-engineered-protocol.md#64-rate-resolution-impedance-autozero-temperature);
  leans eight, §14) — gate the spec tables. Needs a UT8806A and a UT8805N/E.
- The UT8805 continuity threshold default, 0 or 30 Ω (§8) — settles §8's
  figure. Needs `CONT:THR:VAL?` after `*RST` on a UT8805.
- The `TRIGger:DELay` range on both families, and `OUTPut:TRIGger:SLOPe` on a
  UT8806, which no image registers ([§7.3](reverse-engineered-protocol.md#73-trigger-model--known))
  — settles §7.3. Needs each family.
- What `ROUTe:TERMinals?` returns, read as the front/rear switch (§6.3) —
  settles its meaning. Needs a UT8806 with each input selected.
- `SYSTem:BEEPer:STATe`, reportedly without effect on a UT8805E (§14) —
  decides whether the beeper can be set remotely. Needs continuity beeping.

## Vendor sources

- UT8805E firmware V1.88.001, listed but not fetched (approach doc, Sources
  Used) — would confirm from the image the N-line placement §14 reports
  (§11). Needs a fetch from UNI-T HQ's SharePoint.
- UT8806E firmware V1.01.0101, B101's version string (§5.3) — a hash-compare
  with B101 decides whether the UT8806 images cover the E's current release.
  Needs a fetch from UNI-T HQ's SharePoint.
- UNI-T's general-purpose PC software, which may drive several bench meters:
  in the [verification backlog](../../verification-backlog.md#vendor-sources).
