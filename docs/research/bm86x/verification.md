# Brymen BU-86X verification

Open checks for the BM869s and BM867s, the BM829s, BM827s, BM822s and
BM821s, and the BM525s and BM521s on Brymen's BU-86X cable, issues
[#34](https://github.com/antoinecellerier/dmm-tools/issues/34),
[#35](https://github.com/antoinecellerier/dmm-tools/issues/35) and
[#36](https://github.com/antoinecellerier/dmm-tools/issues/36).
What real meters have confirmed is tagged `[HARDWARE]` in the
[spec](reverse-engineered-protocol.md); checks that span families are in the
[verification backlog](../../verification-backlog.md). Community code and
captures (spec §13) narrow many of these; each still wants a reporter's meter.

## First report

- A few seconds of `RUST_LOG=dmm_lib=trace dmm-cli --device <id> debug`, in
  DC V reading negative (leads reversed on a battery), then Ω with leads open
  — settles most items below, the report descriptor included. (#34–#36)
- A first confirmed report per series — gates its spec tables, which the two
  manuals carry. (#34–#36)

## Cable

- The VID:PID the cable enumerates with, 0x0820:0x0001 in Brymen's programs,
  0x82/0x01 in the sheets (§2, §13.2) — decides `transport::bu86x`'s match.
  Needs `lsusb` with the cable plugged in. (#34–#36)
- The report descriptor: 8-byte input and 3-byte output reports, unnumbered
  (§2, §13.2) — decides whether `input_payload` drops a leading `00`. Needs
  the first report's trace. (#34–#36)
- The cable's strings, whether its serial is printable, and whether its
  release number reads as a firmware version (§2, §13.2) — decides the
  transport info. Needs `lsusb -v -d 0820:0001`. (#34–#36)
- Which cable a BM820s or BM520s needs: its manual names the BU-82X on p.13
  and the BU-86X on p.20 (§1.2, §13.3) — decides the entries' links. Needs a
  BM82x or BM52x read on a BU-86X. (#35, #36)

## Requests and model bytes

- Whether a BM52x answers `82 66` too, as Brymen's programs expect (§3.1, D1
  in §13.3) — settles §3.1; a named BM52x is asked `52 66` alone. Needs an
  `auto` trace with a BM52x: detection sends `86 66`, `82 66`, `52 66`. (#36)
- What a meter does with another series' request (§3.1, §13.3 D1, §13.6) —
  decides what detection's order costs. Needs `auto` traces on a BM82x and a
  BM52x, and `--device bm82x` on a BM86x. (#34–#36)
- Bytes 20-22 on a BM86x, "don't care" on the BM860 sheet, `86` in community
  captures (§4.2, §13.3) — detection needs four `86`, a named read byte 23
  alone. Needs any BM86x trace. (#34)
- Whether any byte tells the models of a series apart (§4.2) — decides one
  entry per series. Needs traces from two models of one series. (#34–#36)
- The "don't care" bytes: BM860s 2 and 24-27; BM820s/BM520s 2, 18, 24 bits
  3-0 and 25-27 (§4.1, §5.1, §6.1, §13.3) — decides whether they stay
  silent. Needs traces across functions, Ω included. (#34–#36)

## Timing and replies

- Whether a reply waits for a fresh measurement or comes from a frame the
  cable holds, and its rate against the 5, 1.25 and 20/s displays (§1.2, §3.3,
  §13.4) — settles §3.3. Needs DC V, `counts_500000` and `rec`. (#34–#36)
- Whether one open handle can be polled back to back, as Brymen's programs do,
  or must be reopened per reading, as the sheets' flowchart does (§3.2, §13.4)
  — settles §3.2. Needs the first report's trace. (#34–#36)
- A BM86x at 500000 counts (1.25/s) against detection's three 600 ms windows,
  and whether a new request cancels a pending one (§3.3) — decides whether
  `auto` finds it. Needs `auto` after `counts_500000`. (#34)
- Capacitance replies slower than the 4 s wait (§3.3) — decide `REPLY_WAIT`
  and whether a late reply is taken for the next one's: the drain drops only
  what has arrived. Needs `cap` on a large capacitor, then `auto`. (#34–#36)
- What the cable sends with no meter and with the meter off, and the byte 23
  values the programs map to `82` (§3.4, §13.4) — read as a timeout after 4 s.
  Needs the cable plugged in without a meter, then with it off. (#34–#36)
- Whether linking disables APO: the READMEs say so, a community report
  disagrees (§11.6, D2 in §13.4) — settles §11.6. Needs a meter read past its
  APO time, 17 or 30 minutes. (#34–#36)
- What a reading request gets during logging's 50 % power-down (§11.6) —
  settles §11.6. Needs a BM52x logging at 30 s or more, read after 4.2
  minutes. (#36)

## Display

- The minus segments, 4.7 and 12.4 on the BM860s, 4.7 and 9.5 on the
  BM820s/BM520s (§5.3, §6.3, §13.5) — decides the sign. Needs
  `dcv_negative`. (#34–#36)
- The BM860s secondary point: the sheet's hex sets 7p, its figure 8p (§9.1,
  §13.5) — decides where the sub-value's point goes. Needs `hz_acv`, noting
  the LCD. (#34)
- Whether a BM860s sends AC V without the main V, as the sheet's example does
  (§9.1) — such a reply reads as an unknown function. Needs `acv`. (#34)
- The 500000-count digits and points (§5.2) — decides the six-digit read.
  Needs `counts_500000`, noting the LCD. (#34)
- Report II's ID at byte 10 and the small display at 11-14, as BM820 Table 1
  has it, not the sheet's example (§4.3, §13.2) — decides the BM820 map.
  Needs `hz_acv` on a BM82x or BM52x. (#35, #36)
- The small display's own C or F on a temperature, and DC② and T1② there,
  read as sub-values though no manual figure shows them (§6.2, §6.5) — decides
  `secondary`. Needs `t1_t2` and the V steps. (#35, #36)
- T1 + T2 against T1 − T2: both T bits read as T1-T2, the dash silent (§8.2;
  the community disagrees, §13.5) — decides the mode. Needs `t1_t2` and
  `t1_minus_t2`. (#34–#36)
- What an open thermocouple shows, in neither manual (§11.6) — dashes with
  the unit letter read as no reading. Needs `t1_open`. (#34–#36)
- △ read as REL (§5.5, §6.5, §13.5) — confirms the `rel` flag. Needs `rel`.
  (#34–#36)
- Silent segments: BM860s bar scale, ③, ④, the "1" by 1g; BM820s Hi, Lo, LPF,
  @, "%" by △, small D%, MAX-MIN dash (§5.3-§5.5, §6.5) — which become flags.
  Needs `acv`, `dcv_negative`, `rel`, `t1_minus_t2`, `rec_max_min`. (#34–#36)
- Byte 18 bit 3 on the BM820s/BM520s, the programs' "mV" variant, "don't
  care" in the sheet (§8.1) — decides whether it stays silent. Needs `dcmv`
  and `acmv`. (#35, #36)
- Where the bar graph is: no byte of either map carries it (§5.4, §6.4;
  bytes 2 and 24 candidates, §13.5) — decides a bar graph. Needs a reading
  near full scale. (#34–#36)
- Small-display bits no BM820s manual function lights (§6.5): 14.4
  read as the loop percentage; Ω②, F②, S② and a number beside D%② reported
  and dropped — decides `secondary`. Needs captures across functions. (#35, #36)

## Words

- How the meter draws InEr's I and C_Er's `_` (§7.3, §13.5) — `?nEr` and
  `1nEr` both read as InEr. Needs `iner`, and the self-test at power-on.
  (#34–#36)
- "E.F.": which digits and points light, and how many marks each field
  strength lights (§7.3) — the points go unread, the minus and dashes are
  counted. Needs `ef`, `ncv`. (#35)
- What dBm shows with the leads open, in no manual (§11.6) — on a BM82x
  a row of dashes there reads as the EF field. Needs `dbm`. (#34, #35)
- AutoCheck's Ω with the continuity symbol, read as LoZ Ω, and any other
  function with LoZ, reported (§6.5, §11.3) — decides `low_impedance`. Needs
  the `autocheck` steps. (#35, #36)

## Logging models

- The logging words (LEFt, Strt, PAUS, Cont, StoP, `t0.05`) as no reading,
  `P.001`, and R and C lit together, as Recall whatever the digits (§6.5,
  §7.3) — decides the logging modes. Needs the `log_*` steps and `recall`. (#36)
- What is lit with the memory points left and the logged item number, digits
  with no unit in the figures (§7.3) — reported as an unknown function.
  Needs `log_left`, and SELECT while logging. (#36)
- The logged-memory download's unknowns: checksum order, D3-D0, MRAD's slips,
  Model_Id, 60.00A, AutoCheck, capacitance, power-down (§10.2-§10.6, §13.7) —
  decide its decoder. Needs a BM52x and the download, not yet built. (#36)

## Manuals

- The BM867s, 821s, 822s and 827s dials, DC+AC current on the 821s/822s, the
  VFD SELECT order and the "1V range" Hz sensitivity (§11.1-§11.3) — decides
  the capture steps. Needs those meters, or a later manual. (#34, #35)

## Models

- Whether TestController's Elma BM525s, BM821s, BM829s and BM869s (§13.8)
  are these meters — decides Elma aliases. Needs an Elma unit's trace.
  (#34–#36)

## Detection

- `auto` with the BU-86X beside other cables, and the Brymen detection rows:
  in the [verification backlog](../../verification-backlog.md).
