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
  — settles most items below, the report descriptor included. (#34, #36)
- A first confirmed report for the BM86x — gates its spec tables, which
  its manual carries; a BM827s (#35) and a BM525s (#36) have run the BM82x
  and BM52x, so their tables can be transcribed. (#34)

## Cable

- The report descriptor: 8-byte input and 3-byte output reports, unnumbered
  (§2, §13.2) — decides whether `input_payload` drops a leading `00` on
  Linux and macOS; Windows reads a BM827s (#35). Needs the first report's
  trace there. (#34–#36)
- Whether the cable's serial is printable (§2, §13.2) — decides the
  transport info. Needs `lsusb -v -d 0820:0001`. (#34–#36)

## Requests and model bytes

- Whether a BM52x answers `82 66` too, as Brymen's programs expect and the
  #36 reporter's own tool saw (§3.1, D1 in §13.3) — settles §3.1; a named
  BM52x is asked `52 66` alone. Needs an `auto` trace with a BM52x:
  detection sends `86 66`, `82 66`, `52 66`. (#36)
- What a BM82x or BM52x does with `86 66`, and a BM86x with `82 66` (§3.1,
  §13.3 D1, §13.6) — decides what detection's order costs. Needs `auto`
  traces on a BM82x and a BM52x, and `--device bm82x` on a BM86x. (#34–#36)
- Bytes 20-22 on a BM86x, "don't care" on the BM860 sheet, `86` in community
  captures (§4.2, §13.3) — detection needs four `86`, a named read byte 23
  alone. Needs any BM86x trace. (#34)
- Whether any byte tells the models of a series apart (§4.2) — decides one
  entry per series. Needs traces from two models of one series. (#34–#36)
- The "don't care" bytes: BM860s 2 and 24-27; a BM827s's and a BM525s's
  byte 18 and byte 24's low half vary (§4.1, §5.1, §6.1, §13.3) — decides
  whether they stay silent or carry the bar. Needs traces across functions,
  Ω included, and a reading swept up a range. (#34–#36)

## Timing and replies

- Whether a BM86x reply waits for a fresh measurement, as a BM827s's and a
  BM525s's do, and its rate against the 5, 1.25 and 20/s displays (§1.2,
  §3.3, §13.4) — settles §3.3. Needs DC V, `counts_500000` and `rec`. (#34)
- A BM86x at 500000 counts (1.25/s), or a BM82x or BM52x answering after
  about 780 ms with AC on the small display, against detection's three
  600 ms windows, and whether a new request cancels a pending one (§3.3) —
  decides whether `auto` finds it. Needs `auto` after `counts_500000`, or
  after `dcv_acv` on a BM82x or BM52x. (#34–#36)
- Requests dropped after an idle gap: a BM827s drops up to half after
  500 ms, each costing `reply::read`'s 4 s wait (§3.3) — decides whether a
  read re-sends, which `read --interval-ms` and the GUI's interval need. Needs
  `read --interval-ms 1000` on a BM82x, and any BM86x or BM52x. (#34–#36)
- What a BM525s shows while it stops answering: 12 s to 100 s at the
  `iner`, `log_left`, `log_start` and `recall` steps (§3.3) — decides whether
  those steps expect no reply. Needs those steps run at a terminal. (#36)
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
  minutes. A main display blank but for its point beside a unit now reads
  as no reading without a warning, so a trace shows it, the warnings won't.
  (#36)

## Display

- The minus segments, 4.7 and 12.4 on the BM860s, and the small display's
  9.5 on the BM820s/BM520s; a BM827s's 4.7 is confirmed (§5.3, §6.3, §13.5)
  — decides the sign. Needs `dcv_negative` on a BM86x, and a negative
  small-display reading. (#34–#36)
- The BM860s secondary point: the sheet's hex sets 7p, its figure 8p (§9.1,
  §13.5) — decides where the sub-value's point goes. Needs `hz_acv`, noting
  the LCD. (#34)
- Whether a BM860s sends AC V without the main V, as the sheet's example does
  (§9.1) — such a reply reads as an unknown function. Needs `acv`. (#34)
- The 500000-count digits and points (§5.2) — decides the six-digit read.
  Needs `counts_500000`, noting the LCD. (#34)
- The small display's own C or F on a temperature, and DC② and T1② there,
  read as sub-values though no manual figure shows them (§6.2, §6.5) — decides
  `secondary`. Needs `t1_t2` and the V steps. (#35, #36)
- T1 + T2 against T1 − T2: both T bits read as T1-T2, the dash silent (§8.2;
  the community disagrees, §13.5) — decides the mode. Needs `t1_t2` and
  `t1_minus_t2`. (#34–#36)
- What an open thermocouple shows on a BM86x, in neither manual; a BM827s
  and a BM525s show `---C` (§11.6) — dashes with the unit letter read as no
  reading. Needs `t1_open`. (#34)
- △ read as REL on a BM86x, as a BM827s lights it (§5.5, §6.5, §13.5) —
  confirms the `rel` flag. Needs `rel`. (#34)
- Silent segments: BM860s bar scale, ③, ④, the "1" by 1g; BM820s Hi, Lo, LPF,
  @, "%" by △, small D% (§5.3-§5.5, §6.5) — which become flags. Needs `acv`,
  `dcv_negative`, `rel`, `t1_minus_t2`. (#34–#36)
- Where the bar graph is: no byte of either map carries it (§5.4, §6.4;
  bytes 2 and 24 candidates, §13.5); on a BM827s byte 24's low half and
  byte 18's high half fill in steps (§6.1) — decides a bar graph. Needs a
  reading swept up a range, the bar noted. (#34–#36)
- Small-display bits no BM820s manual function lights (§6.5): 14.4
  read as the loop percentage; Ω②, F②, S② and a number beside D%② reported
  and dropped — decides `secondary`. Needs captures across functions. (#35, #36)

## Words

- How the meter draws InEr's I and C_Er's `_` (§7.3, §13.5) — `?nEr` and
  `1nEr` both read as InEr. Needs `iner`, and the self-test at power-on.
  (#34–#36)
- "E.F.": whether the meter answers at all while it shows (the #35
  reporter's own tool got nothing), which digits and points light, how many
  marks each field strength lights, and whether any annunciator lights with
  them (§7.3) — dashes read as the field only with nothing else lit. Needs
  `ef`, `ncv`. (#35)
- What dBm shows with the leads open, in no manual (§11.6) — dashes beside
  dBm read as no reading, not EF. Needs `dbm`. (#34, #35)
- Whether a BM86x also shows dashes beside a unit after RANGE or SELECT
  (§7.3) — decides whether those stay silent there. Needs `hz_acv` and
  `manual_range`. (#34)
- AutoCheck's Ω with the continuity symbol on a BM829s (a BM525s lights Ω
  alone, §6.5), read as LoZ Ω, and any other function with LoZ, reported
  (§11.3) — decides `low_impedance`. Needs the `autocheck` steps. (#35)

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

- The BM867s, 821s, 822s and 827s dials, DC+AC current on the 821s/822s and
  827s (a BM827s's SELECT never showed it, §11.3), the VFD SELECT order and
  the "1V range" Hz sensitivity (§11.1-§11.3) — decides the capture steps.
  Needs those meters, or a later manual. (#34, #35)
- Which unit the A mA position shows with the leads open and the red lead in
  A, mA or A; a BM827s and a BM525s sent mA with the dial left at A mA
  (§11.3, §11.5) — decides the `dca` steps' expectations. Needs `dca`, the
  dial turned after the lead moves, the LCD noted. (#34–#36)

## Models

- Whether TestController's Elma BM525s, BM821s, BM829s and BM869s (§13.8)
  are these meters — decides Elma aliases. Needs an Elma unit's trace.
  (#34–#36)

## Detection

- The Brymen detection rows: [Device auto-detection](../../verification-backlog.md#device-auto-detection);
  `auto` with the BU-86X beside other cables: [Known defects](../../verification-backlog.md#known-defects).
