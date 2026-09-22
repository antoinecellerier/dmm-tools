# UT8805 / UT8806: Reverse-Engineered Protocol Specification

Protocol specification for the UNI-T UT8805N, UT8805A and UT8805E
(5½-digit) and UT8806, UT8806A and UT8806E (6½-digit) bench multimeters.
They are SCPI instruments: USBTMC on the rear USB port, VXI-11 (and, on
one firmware line, a raw socket and a web page) on LAN, and a DB9 RS-232
port. Nothing here is implemented; the document records what the meters
do on the wire. The approach doc beside it records the sources and
methods, and why the E models are read as the base models' export
builds.

Based on:
- The six SCPI programming manuals, user manuals, datasheets and quick
  guides (`references/ut8805/manuals/`; findings in
  `references/ut8805/analysis/findings/manuals-8805.md`, `manuals-8806.md`)
- Five firmware images: UT8805N SW V1.87.014, UT8805A V1.01.0010, UT8806
  V0.01.0101, UT8806E V0.01.0100 and V0.01.0085 (`firmware.md`; tables
  and decompiles in `references/ut8805/firmware/analysis/`)
- UNI-T's UT8805 PC software (V1.10, UT8805N V2.0, UT8805E V1.09), the
  Instrument Application V3.0 with its UT8806 plugin, the UT8806 IVI-C
  and LabVIEW drivers (`software.md`), the UNI-T SDK V2.3 (`uci-sdk.md`)
- An independent fact-check of the first draft (`factcheck-spec-v1.md`)

Confidence levels:
- **[KNOWN]** — stated in a UNI-T manual, cited by key and page
- **[VENDOR]** — read from a firmware image or UNI-T's software, with
  the image or binary and a function address, file or line
- **[INFERRED]** — logical inference from the above
- **[UNVERIFIED]** — requires real device testing; all collected in §13
- **[HARDWARE]** — seen on a real meter: none yet for this family

Manual keys: NP/AP/EP = UT8805N/A/E programming manuals; NU/AU/EU =
UT8805N/A/E user manuals, EG = the US-site UT8805E user guide; ND/AD/ED
= UT8805 datasheets; S6/S6A/S6E = UT8806/A/E programming manuals;
U6/U6A = UT8806/A user manuals, U6E = `UT8806E_user_manual_EN_V1.3.pdf`;
D6/D6A/D6E = UT8806 datasheets. Pages are PDF pages. Firmware keys:
**N** = UT8805N V1.87.014, **A** = UT8805A V1.01.0010, **E85**/**E100** =
UT8806E V0.01.0085/0100, **B101** = UT8806 V0.01.0101; "H7 images" = A,
E85, E100, B101.

---

## 1. Models and Interfaces

| | UT8805N / UT8805E | UT8805A | UT8806 / UT8806E | UT8806A |
|---|---|---|---|---|
| Rear ports [KNOWN] | USB Device, LAN, RS-232C (DB9), Ext Trig, VM Comp (NU p.15); GPIB option on the N (NU p.15) and in EG p.10, absent from EU p.10 and ED p.3, p.10 | same, GPIB option (AU p.14) | same, GPIB option (U6 p19-20; U6E p20-22) | same |
| Firmware line [VENDOR] | N; the E assumed on the N line [INFERRED] | H7 | H7 | none fetched |

Stated support: "USB-TMC, IEEE 488.2, VXI11 and SCPI" (NU p.8; AU p.6;
EU p.6; U6 p8; U6E p8). The I/O menu holds only a LAN and a UART page;
no interface selector and no GPIB address setting appears in any user
manual (NU p.42 fig. 2-32; EU p.29; U6 p62; U6E p71) [KNOWN]. All ports
live at once [INFERRED]. NI-VISA is the stated driver (AP p.56; S6E p76).

---

## 2. USB

### 2.1 Device Identity — [VENDOR]

| | N (RAM 0x20000080) | H7 images (A RAM 0x24004E58; B101 RAM 0x240056C8) |
|---|---|---|
| Device descriptor | `12 01 00 02 00 00 00 40 83 04 40 75 00 02 01 02 03 01` | `12 01 00 02 00 00 00 40 83 04 40 57 00 01 01 02 03 01` |
| VID:PID | **0483:7540** | **0483:5740** |
| bcdDevice | 0x0200 | 0x0100 |
| Configuration | 1 interface, self-powered (0xC0), 100 mA; iConfiguration 0, iInterface 0 (A RAM 0x24004E90: `09 02 20 00 01 01 00 c0 32 09 04 00 00 02 fe 03 01 00`) | same on every image |
| Interface class | FE/03/01 (USBTMC, USB488 protocol) | same |
| Endpoints | 0x02 bulk OUT 64 B, 0x81 bulk IN 64 B; **no interrupt IN** | same |

The port is the MCU's own full-speed peripheral (N: OTG_FS via ST's
STM32F4 library, `USBD_Init` 0x0807DFA2; H7: USB2_OTG_FS at 0x40080000,
B101 0x900B653E); no bridge chip [INFERRED from the firmware].

String descriptors (built from ASCII at run time):

| | Manufacturer | Product | Serial |
|---|---|---|---|
| N | `STMicroelectronics` | `dm-8805` | the instrument's serial-number string (RAM 0x20015207, also served by `:SYSTem:INFO?`) |
| A (0x9017D990-0x9017DA42) | `Uni-Trend` | `UT8805-ATE` | `DMM` + 12 hex digits from the MCU UID (15 chars) |
| E85 | `Uni-Trend` | `UT8806` | `DMM` + UID, as A |
| E100 (pool 0x90120D44), B101 (0x90120DB8-0x90120E96) | `Uni-Trend` | `UT8806` | the instrument SN (RAM 0x24037336), 13 chars max; factory placeholder `UT1A134600512` |

The H7 images also hold `DMM Configuration` / `DMM Interface`, the N
`VCP Config` / `VCP Interface`; no descriptor references them.

**VID:PID in UNI-T's documents.** The manuals' only USB resource is the
example `USB0::0x5345::0x1234::SN20220718::INSTR` (AP p.69, p.71; S6
p84-85; S6E p89-90; S6A p86, p88), the same in every manual; 0x5345:0x1234
matches no image — uci.dll's device table gives it to UNI-T's
P330XC/P330XM/PDP-B power supplies and the SDK's `libusb0.inf` lists it
as a generic "USB Device" [VENDOR, `uci-sdk.md`]. The IVI-C examples use
`USB0::0x0483::0x5740::UT1A134600512::INSTR` (IVI
`Examples/Cpp/CppExample1/Program.cpp:31`), the H7 line's ID with its
placeholder serial; the UT8805 apps' hot-plug filter accepts 0483:7540
(V1.10 `FUN_00414a10`), the N line's ID [VENDOR]. Both are right for
their line.

### 2.2 USBTMC Handling — [VENDOR]

Class-request handler: B101 0x900A9472, N 0x0803E514 (same code base;
decompiled for these two images, descriptors checked on all five).

| Request | Handled |
|---|---|
| 1, 2 INITIATE_ABORT_BULK_OUT, CHECK_ABORT_BULK_OUT_STATUS | yes |
| 3, 4 INITIATE_ABORT_BULK_IN, CHECK_ABORT_BULK_IN_STATUS | yes |
| 5, 6 INITIATE_CLEAR, CHECK_CLEAR_STATUS | yes |
| 7 GET_CAPABILITIES (wLength 0x18) | yes |
| 64 INDICATOR_PULSE | yes |
| 128 READ_STATUS_BYTE, 160 REN_CONTROL, 161 GO_TO_LOCAL, 162 LOCAL_LOCKOUT (USB488) | **no** — fall through to a stall [INFERRED] |

GET_CAPABILITIES (B101 0x900A9292, N 0x0802880C): status 1, bcdUSBTMC
0x0100, interface capabilities bit 2 set (indicator pulse), TermChar bit
clear, bcdUSB488 0x0100, all USB488 capability bits clear; expected
bytes `01 00 00 01 04 00 00×6 00 01 00 00 00×8` [INFERRED; the buffer
is BSS]. The interface claims USB488 in bInterfaceProtocol but
advertises no USB488 capability and no TermChar support.

Bulk-OUT (B101 0x900A8FF8 / 0x900A9156): MsgID 1 DEV_DEP_MSG_OUT (EOM
from bmTransferAttributes bit 0) and MsgID 2 REQUEST_DEV_DEP_MSG_IN are
handled; other MsgIDs, 126/127 included, go to the error/stall path.
bTag must equal ~bTagInverse; a bTag equal to the previous transfer's is
rejected; after a transfer without EOM the continuation header
(0x900A9156) must carry bTag = previous + 1.

---

## 3. LAN

### 3.1 Services per Firmware Line — [VENDOR, firmware summary table]

| | N | H7 images |
|---|---|---|
| Stack | lwIP 1.4.1, uC/OS-II; DHCP | FreeRTOS/CMSIS-RTOS2; threads listed at B101 0x9012C4F8, A 0x90188648 |
| TCP 111 | portmapper-style listener | portmapper, TCP and UDP |
| TCP 49152 | VXI-11-core-style RPC server | VXI-11 core |
| TCP 5025 | none | raw SCPI socket |
| TCP 80 | none | HTTP: LXI web page "Web-Enable UT8805" (A 0x150783) / "Web-Enable UT8806", LXI identification XML |
| UDP 5353 | none | mDNS |

The N also registers `SYSTem:COMMunication:TCPIP:CONTROL?` (handler
0x080321B3); its reply is unknown [UNVERIFIED].

### 3.2 What the Manuals Say — [KNOWN]

VXI-11 is the stated LAN protocol (U6 p20; U6E p21; ND p.3; AD p.2; ED
p.3); the UT8806 datasheets add "LXI (version 1.5): Sockets, VXI-11, Web
user interface" (D6 p11; D6A p10). **No manual gives a socket port.**
Resource string: `TCPIP0::<ip>::inst0::INSTR` (AP p.61, p.63, p.71; S6
p76, p85; S6E p81, p90; S6A p88); UNI-T's Instrument Application also
builds `TCPIP0::<ip>::5025::SOCKET` (libcxp2_measure.dll,
`VisaLan::defaultPort()` @1d2baaf50 = 0x13A1) [VENDOR]. The boilerplate
example `SYST:COMM:LAN:IPAD` is in no command table or image. The UT8806
web UI has a login (U6 p65; U6E p76).

---

## 4. RS-232

[KNOWN] unless marked. DB9 male on the rear; pins 2 RXD, 3 TXD, 5 GND
(U6 p20; U6E p21); the supplied cable is a straight-through DB9
female-female (NU p.77; U6 p100).

| | UT8805 (NU p.43-44; EU p.30; AU p.35) | UT8806 (U6 p63-64; U6E p73) |
|---|---|---|
| Baud | 9600 (default), 14400, 19200, 38400, 56000, 57600, 115200, 128000, 256000 | 2400, 4800, 9600 (default), 19200, 38400, 56000, 57600, 115200, 128000, 256000 |
| Parity | none (default), odd, even | same |
| Data bits | 8 with no parity; 7 with odd or even parity | same |
| Stop bits | 1 (default), 1.5, 2 | same |
| Handshake | not mentioned | not mentioned |

Settings persist. The programming chapters cover USB and LAN only; that
the same SCPI runs over RS-232 is [INFERRED], supported by "supports
operation over LAN, USB, RS-232C and GPIB" (U6 p8), the UT8806E
V0.01.0084 release note "Optimize serial port communication reception"
(`Release Notes.txt`) [VENDOR], and UNI-T's apps opening `ASRL<n>::INSTR`
with the UI's baud, parity and stop bits, always 8 data bits (UT8805
V1.10 `FUN_00433c30`) [VENDOR]. The IVI-C driver sets 8N1, LF termchar,
no flow control on ASRL sessions (`UT8806.cpp:1819-1852`) [VENDOR]. The
RS-232 reply terminator and handshake are [UNVERIFIED].

---

## 5. SCPI Message Layer

### 5.1 Terminators

Commands end with NL (AP p.3; S6E p3; S6 p3) [KNOWN]; every UNI-T tool
writes LF — the IVI-C driver a literal `\n` in each command string
(`UT8806.cpp`), IA (`VisaIO::write`) and the UT8805 apps (V1.10
`FUN_00420540`) append `\n` when it is missing, LabVIEW its LF constant
[VENDOR]. Replies: **`\r\n` on the N (0x0808E224), `\n` on the H7
images** [VENDOR]. The manuals state a terminator only for block data
(`\n`, AP p.4; S6E p4). No tool sets a USB read termchar; USBTMC ends the
read on EOM [VENDOR].

### 5.2 Syntax — [KNOWN]

Standard SCPI: case-insensitive, short forms by the upper-case letters
("VolTaGe, volt and Volt are all acceptable", NP p.1; EP p.2), `;`
chaining with implied paths (EP p.3, p.49), Booleans returned as `0`/`1`
(NP p.2; EP p.3; S6E p64 shows `ON`), suffixes M, k, m, u (NP p.2).

### 5.3 `*IDN?`

Firmware [VENDOR, summary table; strings `UNI-T`, `UT8805`,
`SW V1.87.014` adjacent at N 0xEA768-0xEA780]:

| Image | Reply |
|---|---|
| N | `UNI-T,UT8805,<SN>,SW V1.87.014` |
| A | `UNI-T,<model>,<SN>,SW V1.01.0010`; model defaults to `UT8805A` |
| E85 / E100 / B101 | `UNI-T,<model>,<SN>,SW V0.01.0085` / `SW V0.01.0100` / `SW V1.01.0101`; model defaults to `UT8806` |

The model and SN fields are the strings set with `FACTORY:MIS:MODEL` and
`FACTORY:MIS:SN`, registered in every image [VENDOR] [INFERRED as the
source of the fields]. B101's version string reads `SW V1.01.0101`
although its listing says V0.01.0101; the About screen photographed in
the UT8806 user manuals shows `SW V1.01.0100` (U6 p65; U6E p76) [KNOWN].

The manuals' examples are `UNI-T UT8805A, UT1A13460051200, V0.01.0000`
(AP p.5; S6 p6; S6E p6) and `UNI-T UT8806A, UT1A13460051100, V0.01.0000`
(S6A p6) [KNOWN]: a space for the first comma, no `SW` prefix. UNI-T's
tools expect four comma-separated fields (§12).

### 5.4 Common Commands

Registered per image [VENDOR, `*.norm.txt`]:

| | N | A | E85 | E100, B101 |
|---|---|---|---|---|
| `*CLS *ESE *ESE? *ESR? *IDN? *OPC *OPC? *RST *SRE *SRE? *STB? *TRG *TST? *WAI` | ✓ | ✓ | ✓ | ✓ |
| `*OPT? *PSC *PSC? *RCL *SAV` | — | ✓ | ✓ | ✓ |
| `*UNREMOTE` | ✓ | ✓ | — | ✓ |
| `*RSTCAL` | ✓ | — | — | — |

Documented: AP p.5-8 has the first row less `*ESR?`, plus `*PSC` and
`*RCL`/`*SAV {0-4}`; S6E p5-8 the first row plus `*PSC`, without
`*RCL`/`*SAV`; NP/EP show common commands only in examples [KNOWN].
`*TST?` returns `+0` (pass) or `+1` (AP p.7; S6E p7).

### 5.5 Error Queue and Status

`SYSTem:ERRor[:NEXT]?` returns `<code>,"<string>"`, e.g.
`-113,"Undefined header"` (EP p.46; AP p.55; S6E p74); no manual
publishes a code table [KNOWN]. The N image holds an SCPI error-string
table at 0x080312C4 (codes 0, -101 … -310, +263) [VENDOR]. Every image
registers `SYSTem:ERRor:COUNt?`; E85 alone an explicit `SYSTem:ERRor?`
[VENDOR]. The questionable register carries bit 11 lower-limit fail, bit
12 upper-limit fail (EP p.4, p.8; S6E p62) and bit 14 "Reading Mem Ovfl"
(EP p.4); the images register `STATus:PRESet`,
`STATus:QUEStionable:ENABle[?]` and `STATus:QUEStionable[:EVENt]?` (N) /
`:EVENt?` (H7) [VENDOR]. Without USB488 requests there is no status byte
over USB (§2.2).

---

## 6. Functions and Configuration

### 6.1 Function Keywords — [KNOWN]

Twelve, for `[SENSe:]FUNCtion[:ON] "<f>"`, `CONFigure:<f>` and
`MEASure:<f>?` (EP p.29; AP p.31; S6E p26): `VOLTage[:DC]`, `VOLTage:AC`,
`CURRent[:DC]`, `CURRent:AC`, `RESistance`, `FRESistance`, `CAPacitance`,
`TEMPerature`, `CONTinuity`, `DIODe`, `FREQuency`, `PERiod`. Default
`VOLTage[:DC]`. No ratio, no AC+DC, no secondary display over SCPI.
`CONFigure` resets the function's parameters and starts nothing;
`MEASure:<f>?` is CONFigure plus READ? (EP p.17, p.24; S6E p9-10, p18).

### 6.2 `CONFigure?` and `FUNCtion?` Replies

Firmware [VENDOR]: **`CONFigure?` writes the function token unquoted, a
space, then the number**: N 0x08012F89 (text via 0x08031FB2, number via
0x08032008, which prefixes `' '` at 0x08032048), A 0x9016A74D (0x900EB1E6,
then 0x900EB314 → 0x900EAE44, `' '` at 0x900EB084), B101 0x90101001
(0x9008FF1E, then 0x9009004C → 0x9008FB64). **`FUNCtion?` writes the name
in double quotes**: N 0x08015879 → 0x08032100 (`'"'` at 0x08032140), A
0x90133BF1 → 0x900EB356 (0x900EB604), B101 0x900CFC25 → 0x9009008E
(0x9009033C). So the meter sends `VOLT:DC +2.00000000E+01` and
`"VOLT:DC"`.

The manuals differ from this in part [KNOWN]: EP p.18 / NP p.16 print
`CONF?` quoted, `“VOLT +2.00000000E-01”`; AP p.21 with a comma,
`VOLT:DC,+2.00000000E+01`; S6E p9 and S6A p9 unquoted with a space, the
firmware's form; every manual prints `FUNC?` unquoted, `VOLT:AC` (AP
p.31; S6 p23) or `VOLT: AC` (S6E p27). UNI-T's tools [VENDOR]: the IA plugin's
`decodeCONFigure` strips `"`, CR and LF, matches the prefix (`VOLT:DC`,
`VOLT:AC`, `CURR:DC`, `CURR:AC`, `RES`, `FRES`, `CAP`, `TEMP`, `CONT`,
`DIOD`, `FREQ`, `PER`) and reads the range after the space; the UT8805
app (`FUN_00416a50`) splits on the space; LabVIEW uses `FUNC?`. The
bytes from a real meter remain [UNVERIFIED] until captured (§13).

### 6.3 Ranges and Terminals — [KNOWN]

`[SENSe:]<f>:RANGe {<range>|MIN|MAX|DEF}` and `…:RANGe:AUTO {ON|1|OFF|0}`
for V and I (AC and DC), RES, FRES, CAP and the FREQ/PER input
(`FREQuency|PERiod:VOLTage:RANGe`, the AC-voltage range, S6E p16-17);
CONF/MEAS take `[{<range>|AUTO|MIN|MAX|DEF}]` and, on the UT8806, an
optional resolution (S6E p9). Every image registers the same RANGe set
[VENDOR]. The ladders themselves are spec-table data: the programming
manuals list them per model (EP p.31-45; AP; S6E p27-32; S6A), and for
the UT8806 the user manual lists wider ones than S6E (DCI from 2 µA, U6E
p41; ACI from 200 µA, p46; Ω to 1 GΩ, p49; C to 100 mF, p55; U6 p8
agrees), so they wait for hardware (§13).

Current terminals: `[SENSe:]CURRent:{AC|DC}:TERMinals {Small|Big}`, the
200 mA vs 10 A jack (EP p.32; S6E p54). The UT8806 images add
`ROUTe:TERMinals?` (B101 0x9009B955) for the front/rear input switch (U6
p18) [VENDOR] [INFERRED meaning].

### 6.4 Rate, Resolution, Impedance, Autozero, Temperature

Registered [VENDOR, `*.norm.txt`] and documented [KNOWN] per model:

| Item | UT8805 (N, A) | UT8806 (E85, E100, B101) |
|---|---|---|
| NPLC | `[SENSe:]<f>:NPLC {Slow|Medium|Fast}` for `VOLTage[:DC]`, `CURRent[:DC]`, `RESistance`, `FRESistance` (EP p.43; AP); strings `Slow Medium Fast` at N 0xBEBC, A 0x191928; EP's AC NPLC headings (§6.2.6 p.32, §6.6.6 p.43) are not registered | numeric `NPLC {<PLCs>|MIN|MAX}` for the same four (S6E p36-37) |
| `<f>:RESolution` | — | V and I AC/DC, RES, FRES (S6E p33) |
| `ZERO:AUTO {OFF|ONCE|ON}` | — | `VOLTage|CURRent[:DC]`, `RESistance`, `FRESistance` (S6E p51-53) |
| `FREQuency|PERiod:APERture`, `:RANGe:LOWer` | — | ✓ (S6E p53-54; p50-51) |
| `VOLTage|CURRent:AC:BANDwidth` | — | ✓ (S6E p50) |
| `VOLTage[:DC]:FILTer[:STATe]` | N only | — |
| `VOLTage[:DC]:IMPedance:AUTO {ON|1|OFF|0}` | ✓; default OFF (EP p.43) | ✓; default AUTO (U6E p69) |
| `UNIT:TEMPerature {C|F|K}`, `TEMPerature:TRANsducer:TYPE`, `TCouple:TYPE`, `RJUNction[:TYPE]`, `RTD|FRTD:RESistance[:REFerence]` | ✓ (EP p.27; R0 range 50-2000 Ω from AP p.52 only) | ✓ (S6E p49: R0 50-2100 Ω, default 100 Ω) |

The NPLC values, digits per NPLC, impedance thresholds and thermocouple
lists are in the manuals and conflict: the UT8806A user manual gives
0.001-100 PLC (U6A p8, p28) where S6A repeats the UT8806 list; the N/E
TC types are E J K N R T on EP p.21, p.40 and B E J K N R S T on EP p.27
/ NP p.24 (§13).

---

## 7. Taking Readings

### 7.1 Documented Queries — [KNOWN]

| Command | Effect | Source |
|---|---|---|
| `INITiate[:IMMediate]` | Arms the trigger, clears the reading memory; overlapped | EP p.5; S6E p58 |
| `READ?` | Arms, waits, returns the readings and erases them | EP p.6, p.17; S6E p56 |
| `FETCh?` | Waits for the current measurement, returns all readings in memory; does not erase | EP p.4, p.6; S6E p56 |
| `MEASure:<f>? [range[,res]]` | CONFigure defaults, then READ? | EP p.24; S6E p18 |
| `DATA:LAST?` | "Return the latest measured results. You can execute this query at any time, even during the series of measurements." Reply carries a unit: `-4.79221344E-04 VDC`. With no data: "9.9E37 with the unit" | EP p.23; NP p.20; AP p.26; S6E p57 (no "any time" quote) |
| `DATA:POINts?` | Readings in memory, e.g. `+20` (EP) or `100` (AP, S6E) | EP p.23; AP p.26; S6E p57 |
| `DATA:REMove? <n> [,WAIT]` | Returns and erases n readings, comma-separated (S6E p57 has no `[,WAIT]`) | EP p.23-24; S6E p57 |
| `R?` | Named in prose only (EP p.5, p.24) | — |

`DATA:LAST?` stands apart because it can be sent at any time, during a
measurement series included; whether it follows the free-running
front-panel reading without `INIT` is not stated [UNVERIFIED]. Reading
memory: "1,000" (NP p.20; AP p.26; S6E p57) against "10,000" (NP p.16;
EP p.18) and "1,0000" (EP p.4-7, p.23, p.47); `DATA:REMove?` takes
1-10000; the datasheets say 10k (ND p.16; ED p.10; D6 p11) and 50k for
the UT8806A (D6A p10) [UNVERIFIED which].

### 7.2 Registered but Undocumented — [VENDOR, `*.norm.txt`, `*.scpi.txt`]

| Command | N | A | E85 | E100 | B101 | Note |
|---|---|---|---|---|---|---|
| `MEASurement:CONTinuous` | 0x08019015 | 0x900EFC85 | — | ✓ | ✓ | the UT8805N V2.0 app sends it once at connect (§12) |
| `READ:LAST?` | 0x080176F9 | 0x900EFD75 | — | ✓ | ✓ | the same app then polls it every 100-200 ms |
| `:SYNC:DATA?` | 0x0800CCD1, enters remote | null handler | — | — | — | all three UT8805 apps send it (`ut8805_*_scpi_clean.txt`) |
| `R?` | 0x08017859 | null handler | null | null | null | registered on the H7 images with no handler |
| `CREAD?`, `WREAD?` | 0x08017529, 0x08017611 | — | — | — | — | meaning unknown |
| `READ:DISP?` | — | — | — | — | 0x9009B281 | new in B101; reply unknown |
| `DATA:FORMat[?]` | — | — | — | — | 0x901116C5 | new in B101 |
| `KEY:SET` | 0x08015995 | — | — | — | — | injects a front-panel key (summary table) |

What `MEASurement:CONTinuous` changes and what `READ:LAST?`,
`:SYNC:DATA?`, `CREAD?`, `WREAD?` and `READ:DISP?` return is
[UNVERIFIED]; the UT8805N V2.0 app parses `READ:LAST?` as a plain number
with `toDouble` (`ut8805_n20_decomp.c`; §12).

### 7.3 Trigger Model — [KNOWN]

`TRIGger:SOURce {IMMediate|EXTernal|BUS}` (default IMM; `*TRG` for BUS),
`TRIGger:COUNt`, `SAMPle:COUNt` 1-100000, `TRIGger:DELay[:AUTO]`,
`TRIGger:SLOPe {POSitive|NEGative}` (default NEG on the UT8805, EP p.17;
POS on the UT8806, S6E p9-10, p18) and `OUTPut:TRIGger:SLOPe` for VM Comp
(EP p.5 §1.9; S6E p72) (EP p.6-7, p.47-49; S6E p56-60). `TRIGger:DELay`
range conflicts on both families: EP p.47 "1 to 3600 s", default 1 s,
versus 6-10000 ms (ND p.15; AD p.9; ED p.10); S6E p59 "10 µs-10 s" versus
6-10000 ms (D6, D6A, D6E p10) [UNVERIFIED]. `OUTPut:TRIGger:SLOPe` is
registered by the N and A images only; **no UT8806 image registers it**
[VENDOR].

---

## 8. Reply Formats and Special Values

- **Numbers.** `±d.ddddddddE±dd`, 15 characters, e.g. `+5.21209585E+04`
  [KNOWN, every example; VENDOR, summary table `+d.ddddddddE±XX`].
  Several readings are comma-separated, `, ` with a space in some
  examples (EP p.4; S6E p56). AP p.4 and S6E p4 say "three decimals and
  a three-digit exponent", which no example follows [KNOWN,
  contradictory]. S6E shows unnormalised mantissas such as
  `+0.20000000E-01` (S6E p20, p27).
- **Units.** None on `READ?`, `FETCh?`, `MEAS?`, `DATA:REMove?`; the
  function implies it. `DATA:LAST?` appends a token; `VDC` is the only
  documented one. The images carry `uVDC` … `kVDC`, `uVAC` … `kVAC`,
  `uADC` … `kADC`, `uAAC` … `AAC`, `OHM`, `KOHM`, `MOHM` (N 0xE48F8 and
  0xEA798 areas; A 0x191820; B101 0x135DFC) [VENDOR]; which of them
  `DATA:LAST?` sends per function is [UNVERIFIED].
- **Overload.** The panel shows "OL" (EU p.13; U6E p29); the remote
  reply is `+9.90000000E+37` (EP p.19-22; NP p.17-20) [KNOWN]. Firmware:
  `+9.90000000E+37` / `-9.90000000E+37` by sign on the N and the UT8806
  images; **the A's `READ?` replaces a negative overload with
  `+9.9E37`** [VENDOR, summary table]. The UT8806 manuals do not
  document the overload value [KNOWN absence].
- **Not a number.** `9.91E37` / `+9.91000000E+37` (EP p.12, p.16);
  `DATA:LAST?` with no data returns "9.9E37 with the unit" (EP p.23);
  math results out of range become `-9.9E37`, `0` or `9.9E37` (EP p.14)
  [KNOWN].
- **Invalid data** "is indicated by `*`" (AP p.4; S6E p4) [KNOWN]; not
  seen in any example or image string [UNVERIFIED].
- **Continuity and diode.** Queries return the measured value
  "regardless of the size" (EP p.25; S6E p24-25: `MEAS:CONT?` →
  `+9.84739065E+02`, `MEAS:DIOD?` → `+9.84733701E-01`) [KNOWN]; the
  open-circuit reply is not stated [UNVERIFIED]. UNI-T's UT8805 app
  shows "OL" for continuity above 1200 (V1.10 `FUN_0041aff0`, constant
  0x45A5A0) [VENDOR]. `[SENSe:]CONTinuity:THReshold:VALue` 0-2000 Ω:
  default 30 Ω on the UT8806 (S6E p15, p24; U6E p61, p69); 0 (EP p.45)
  or 30 Ω (EU p.28) on the UT8805 [UNVERIFIED].
- **NULL** subtracts the relative value from every returned reading
  (EP p.29; S6E p37) [KNOWN]. Whether dB/dBm scaling, limits or
  statistics change `READ?` is not stated [UNVERIFIED].

---

## 9. Remote and Local

Firmware [VENDOR, summary table; handlers in `*.scpi.txt`]:

| | N | A | E85 | E100, B101 |
|---|---|---|---|---|
| Enters remote on | `*IDN?` and every query (`:SYNC:DATA?` included) | `READ?`, `FETCh?`, `MEAS?` | every command except `*IDN?` | same as E85 |
| `*UNREMOTE` | 0x08018F2D, clears the remote flags | 0x900EFC79 | — | 0x9009B9B5 |
| `SYSTem:LOCal` / `REMote` / `RWLock` | — | — | 0x90120435 / 0x90120443 / 0x9012045B (B101 addresses) | same |
| `SYSTem:RMTote` | — | — | — | alias: same handler as `REMote` |
| Key lock string | "The key has been remotely locked!" (0xE6D80) | present (0x1886D8) | present | present (0x12C588) |

The UT8806 tables flag `*IDN?` alone (third column `0x1`, B101
0x900FB430), consistent with the dispatcher exempting it [VENDOR]
[INFERRED]. `KEY:SET` on the N injects a key press.

Manuals: `*UNREMOTE` "returns to local mode and unlocks the front-panel
keys" (EP p.7; AP p.11), absent from NP (its §1.13 is missing) although
the N image registers it; `SYSTem:LOCal` unlocks, `SYSTem:REMote` enters
remote without locking, `SYSTem:RWLock` enters remote and locks the keys
(S6E p74-75; S6 p69-70); the UT8806's Shift key carries a "Local" legend
(U6E p19) [KNOWN]. No manual says what enters remote. No USB488
REN/GTL/LLO exists over USB (§2.2). What the tools send is in §12.

---

## 10. Math and the Secondary Display

Every model has per-function NULL, dB/dBm scaling (DCV/ACV), statistics,
limits and a histogram under `CALCulate` (EP p.8-9, p.12-16; S6E p60-72; the
S6E p70 `AVERage:ALL?` example order is mean, min, max, sdev against its
text's mean, sdev, max, min) [KNOWN]; `CALCulate:RELATive` and
`LIMit:BEEPer:STATe` are documented for the UT8806 only (S6E p60-64) and
registered by every image [VENDOR]; the UT8806 adds `DISPlay:TEXT[:DATA]`
and `DISPlay:TEXT:CLEar` (S6E p72-73). The front panel's dual display
(ACV+FREQ, ACI+FREQ, FREQ+PER/ACV, temperature + input voltage or
resistance) has **no SCPI access**: "the data on the secondary display
cannot be saved" (EU p.28; NU p.40; U6E p91-92) [KNOWN]; no query returns
the displayed reading (`READ:DISP?`, B101 only, is undocumented, §7.2).

---

## 11. Model Differences

Facts not already placed in §2-§10 [KNOWN] [VENDOR, `*.norm.txt` diffs
and `*.scpi.txt`]:

| Item | UT8805N / UT8805E (N image) | UT8805A (A image) | UT8806 / UT8806E (E85, E100, B101) |
|---|---|---|---|
| Registered commands | 281 | 278 | 314 / 317 / 320 |
| SENSe prefix spelling | `[SENSe]:` on 126 of 130 SENSe entries | `[SENSe:]` | `[SENSe:]` on 158 of 160; `[SENSe]:` on two, `[SENSe]:FUNCtion[:ON]?` included (B101 0x900FB6F4; E85 the same) |
| `FUNCtion` | `[SENSe:]FUNCtion[:ON]` | `[SENSe:]FUNCtion` (no `:ON`) | `[SENSe:]FUNCtion[:ON]` |
| `INITiate` | `INITiate[:IMMediate]` | `INITiate` | `INITiate` |
| `CURRent:DC:NPLC` | `CURRent:DC:NPLC` (DC required) | `CURRent[:DC]:NPLC` | `CURRent[:DC]:NPLC` |
| `CALCulate:RELATive` | `[:DATA]` | `:DATA` | `:DATA` |
| `CALC:TRAN:HIST` variants | `:AUTO:ONCE` | `:RANGe:AUTO[?]` | `:AUTO`, `:RANGe:AUTO?` |
| `DIODe:BEEPer:STATe` | — | — | ✓ |
| `:SYSTem:INFO?` | ✓ | null handler | — |
| `PRODUCTion:DATE`, `TEST:TEXT` | ✓ | — | — |
| 4-wire Ω top range | 100 MΩ (EP p.36) | 2 MΩ (AP p.42) | 2 MΩ (S6E p28) |

The UT8806A has no fetched image. Its manual's differences from the
UT8806 manual are a mechanical "2→1" edit that also hit unrelated text
("ISO9001:1008", S6A p2; `MAX_CNT = 100`, S6A p82), so every UT8806A
value that differs only by 2→1 — ranges, `VOLT:AC:BAND {3|20|100}` (S6A
p46), the missing `FRES:ZERO:AUTO` (S6A p48) — is [UNVERIFIED]. The
UT8805E firmware (V1.88.001, SharePoint) was not fetched; the E is placed
on the N line from the manuals [INFERRED].

---

## 12. How UNI-T's Tools Drive the Meter — [VENDOR]

| Step | UT8805 apps (V1.10, N2.0, E1.09) | Instrument Application V3.0 (IA) | IVI-C driver | LabVIEW driver |
|---|---|---|---|---|
| Find | `viFindRsrc("USB?*INSTR")` (V1.10 `FUN_00414380`); `ASRL<n>::INSTR`; `TCPIP0::<ip>::INSTR` | `USB[0-9]*::?*INSTR`, `ASRL…`, `GPIB…`, `TCPIP0::<ip>::INSTR` or `::5025::SOCKET` | resource given | resource given |
| Identify | never; trusts the 0483:7540 hot-plug filter | `*IDN?\n`, four comma fields, model `UT8806` or `UT8806A` case-insensitively (`findEquipmentViewInfo` @140003450) | `*IDN?\n`, four fields, model `UT8806` or `UT8806E` (`Models.cpp:20`) only with IdQuery | `*IDN?` |
| Start | nothing | `SYST:REM` (`MyDMM::open`) | `*RST` + `*OPC?` when Reset is set | `*ESE 60;*SRE 56;*CLS;`, then `SYST:LOC|REM|RWL` with an `*OPC?` loop |
| Function | `CONF?` at connect and after changes; `:SYNC:DATA?` among the settings queries | `:CONF?` every cycle | never asked; `CONF:<f> <range>[,<res>]` then `*OPC?` | `FUNC?` |
| Read | V1.10/E1.09: `<f>:RANG?` then `READ?`, each ≤1 s (CONT, DIOD, TEMP skip `RANG?`). **N2.0: `MEASurement:CONTinuous` once, then `READ:LAST?` every 100-200 ms** (`FUN_00416060`) | `:CONF?`, `:READ?`, 1-7 `:SENS:<f>:…?` per cycle, 1 ms interval, 200 s timeout | `READ?`, or `INIT` then `FETC?`; 60 s; first field before `,` via `atof` | `READ?`, `:FETCh?` or `DATA:LAST?` |
| Reply | strips CR/LF; "OL" at ≥ 9.9e37 and continuity > 1200 | strips `"`, CR, LF; "OL" for two non-numeric kinds | no check | numeric parse |
| End | `*UNREMOTE` | `SYST:LOC` | — | — |

Vendor bugs: IVI-C passes its resolution enum (0/1/2) as the CONF
resolution and sends `*PSC` with no argument; IA's TEMP cycle drops
`:CONF?`. The UNI-T SDK (`uci.dll`) has no UT8805/UT8806 entry and its
readme routes SCPI instruments to NI-VISA (`uci-sdk.md`).

---

## 13. What Needs Hardware Verification

No UT8805 or UT8806 has been on the bench. Every [UNVERIFIED] item:

- `*IDN?` as sent: separators, `SW ` prefix, model field on an E model,
  `FACTORY:MIS:MODEL/SN` as its source (§5.3)
- Reply terminator per line; RS-232 terminator and handshake (§4, §5.1)
- `CONFigure?` and `FUNCtion?` bytes as sent (§6.2)
- `DATA:LAST?` without `INIT` on the free-running panel; its unit token
  per function (§7.1, §8)
- `MEASurement:CONTinuous`, `READ:LAST?`, `:SYNC:DATA?`, `CREAD?`,
  `WREAD?`, `READ:DISP?`, `SYSTem:COMMunication:TCPIP:CONTROL?` (§3.1,
  §7.2)
- Reading-memory size, 1,000 or 10,000 (§7.1)
- Overload signs per line, the A's `READ?` sign quirk, the `*` marker,
  NaN; continuity and diode open-circuit replies; the UT8805 continuity
  threshold default, 0 or 30 Ω; whether dB/dBm, limits or statistics
  change `READ?` (§8)
- Range ladders per model (UT8806 user manual against S6E; UT8805
  capacitance top range, EP p.22 "10000uF"; every UT8806A "2→1" figure,
  `FRES:ZERO:AUTO` and AC bandwidth included); the UT8806A NPLC list;
  the N/E thermocouple types (EP p.21 against p.27) (§6.3, §6.4, §11)
- `TRIGger:DELay` range on both families; `OUTPut:TRIGger:SLOPe` on a
  UT8806 (§7.3)
- What enters remote per line and whether keys lock; `*UNREMOTE` on
  the N (§9)
- Firmware line and VID:PID of the UT8806A and the UT8805E (§1, §11)
- LAN ports per line and VXI-11 `inst0` (§3); GPIB address setting and
  the E models' GPIB option (§1)
- `0x5345:0x1234` matching nothing (§2.1); GET_CAPABILITIES bytes as
  sent (§2.2); the meaning of `ROUTe:TERMinals?` (§6.3)
