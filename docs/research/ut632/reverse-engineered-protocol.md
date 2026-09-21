# UT632 / UT632N: Reverse-Engineered Protocol Specification

What UNI-T's software shows of the wire protocol of the UT632 and UT632N
bench multimeters. No UT632 has been captured yet; §5 lists what a
capture must settle.

**In short:** UT803.exe carries a UT632 configuration. Selected, it reads
the serial port at 2400 baud, ends each frame at a byte whose high nibble
is E, and decodes nothing; it installs no HID handler at all. The frame
length and the payload encoding are not in the binary.

Based on:
- UNI-T's bench programming manual V1.1 (the UCI manual), whose device
  list names the UT632 — see
  `../uci-bench-family/reverse-engineered-protocol.md`
- Ghidra 12.1.3 decompilation of UT803.exe V1.01's form event handlers,
  seeded from the Delphi RTTI (2026-09-16), with disassembly of the
  conditions and the form resource (read 2026-09-21)
- UT804.exe V2.00's `H60BRData`, for comparison
- The uci.dll findings in `../uci-bench-family/reverse-engineered-protocol.md`

Confidence levels:
- **[KNOWN]** — documented in UNI-T's UCI programming manual
- **[VENDOR]** — confirmed by analyzing UNI-T's official software binaries
- **[DEDUCED]** — logical inferences from available evidence
- **[UNVERIFIED]** — requires real device testing to confirm

---

## 1. Transport Layer

### 1.1 USB HID Bridge — [KNOWN]

The UCI manual lists the UT632 as a HID device with the address
`[C:DM][D:T632][T:HID][PID:0xe008][VID:0x1a86]`: VID 0x1A86, PID 0xE008,
the WCH CH9325 USB-to-UART bridge of the UT803 and UT804
(`../ut803/reverse-engineered-protocol.md` §1.1;
`../uci-bench-family/reverse-engineered-protocol.md` §2.1).

### 1.2 UART Parameters — [VENDOR]

The app's UT632 configuration runs at 2400 baud on both of its paths:

- **Serial port.** The `UT632` branch of `FormCreate` sets no rate, so
  `COMM2` stays at the form data's `br2400`, with the component's 8 data
  bits, 1 stop bit and no parity (§2.1;
  `../ut803/reverse-engineered-protocol.md` §1.2).
- **CH9325 feature report.** `SetFeatureClick` does not read the `UT632`
  box, so the report carries its unconditional 2400 (the report layout
  and the site are in `../ut803/reverse-engineered-protocol.md` §1.2).

The line format on the wire — data bits and parity — is [UNVERIFIED]; the
app's 8N1 port settings do not show it.

---

## 2. UT803.exe with the UT632 Selected — [VENDOR]

### 2.1 Selection

`FormCreate` (VA 0x5584B8) tests the form's check boxes in this order:
`IFUT60E` (field 0x4B4), `UT632` (0x5A0), `IFUT70B`, `IFUT60D`, `IFUT803`,
`IFPR3315`, `IFUT61C`. Each checked box overwrites `COMM2`'s
`OnReceiveData` and the window caption:

- `IFUT60E` (captioned "UT60E") installs `H60BRData` and sets the caption
  "OEM 25394 Interface Program _Ver: 2.00" (VA 0x5585BC).
- `UT632` installs `H60BRData` and sets the caption "UT632 Interface
  Program _Ver: 2.00" (VA 0x558602).

Neither branch sets a port speed (§1.2).

In the shipped form `UT632` is unchecked, at Top 432, below the 407-px
client area. `IFUT803` is checked, and its later branch overrides both
the handler and the caption, so the shipped exe runs as the UT803 app; a
UT632 build needs different form data. `UT632` is read only by
`FormCreate` (the single xref is VA 0x5585E7).

### 2.2 RS232 Handler `H60BRData`

`H60BRData` (VA 0x5580F0-0x55827E) converts each received byte to two
hex digits with `IntToHex(b, 2)` (VA 0x4090E4, uppercase through `CvtInt`)
and compares the first digit with `'E'` (constant at VA 0x55828C, compare
at VA 0x558159, `je` 0x5581CA):

- **High digit other than E:** the low digit is appended to the string at
  0x56769C and the high digit to the string at 0x567698.
- **High digit E:** both digits are appended, `DAT_005676A0` is set to the
  low-digit string, and both strings are cleared.

The handler checks no length and no other high digit, and calls out to
nothing. `0x5676A0` is read only by `LcdDisplay70B`, by the HID handlers
0x55CF20 and 0x55D19C, and by unit finalization (VA 0x55EAB9);
`LcdDisplay70B` is called only from `H70BRData` (VA 0x55842D) and from
0x55D19C (VA 0x55D3F9), neither of which the UT632 configuration
installs. Over RS232 the UT632 configuration therefore decodes nothing
and displays nothing.

### 2.3 HID Path

`USBConClick` (VA 0x55DA80) installs a HID handler in two cases only:
0x55D19C when `IFUT803` is checked (VA 0x55DB3E-0x55DB5C) and 0x55CF20
when `IFUT60E` is checked (VA 0x55DB61-0x55DB7F). It never reads `UT632`.
With `UT632` checked alone, `USB Connect` installs no HID handler.
`SetFeatureClick` does not read `UT632` either (§1.2).

The `Read` button (`ReadBtnClick`, VA 0x55CD3C; Left 656 on a 620-px
client width) installs 0x55CF20 whatever the configuration. That handler:

- formats each report as `"R %.2x  "`, then `"%.2x "` per byte, and reads
  the payload count from character 8;
- when the count is 1-9, appends the payload's hex to 0x56774C;
- on the next report with any other count, takes the first 14 bytes:
  their high digits go into a local that nothing reads
  (VA 0x55D09B-0x55D0C4); their low digits go into 0x5676A0
  (VA 0x55D0C6-0x55D0F3) and the hidden `USBtxt` label (VA 0x55D104);
- makes no check and calls no parser.

No code path in UT803.exe reads a UT632 over HID.

---

## 3. The UT804.exe Twin — [VENDOR]

UT804.exe's `H60BRData` (VA 0x557E54-0x557FF6) is the same routine:
`IntToHex` at 0x409230, `'E'` at 0x558010, `je` at 0x557EC6. It differs
in what happens on the E byte:

- it also compares the high-digit string with `"123456789ABCDE"`
  (VA 0x557FAD, `jne` 0x557FB2);
- on a match it calls `LcdDisplay60B` (VA 0x557FBC), the 7-segment
  decoder of that app's UT60A/B/C path
  (`../ut803/reverse-engineered-protocol.md` §2.4).

UT803.exe's `H60BRData` and its `Read` handler are UT804.exe's routines
with that check and that call removed. UT803.exe has no `LcdDisplay60B`.

---

## 4. Wire Format, as Far as It Follows

- **[VENDOR]** UNI-T's UT632 configuration reads 2400 baud and ends each
  frame at a byte whose high nibble is E.
- **[DEDUCED]** It is not the UT803/UT804's 11-byte CR LF packets. None
  of their bytes has high nibble E (data `0x3_`/`0xB_`, CR `0x0D`, LF
  `0x0A`/`0x8A`; `../ut803/reverse-engineered-protocol.md` §2.1), so
  `H60BRData` would never close a frame on them.
- **[DEDUCED]** The frames are 14 bytes with high nibbles 1-E and the data
  in the low nibbles: UT804.exe's twin requires exactly that (§3), and the
  sibling `IFUT60E` HID handler takes 14 bytes (§2.3).
- **[DEDUCED]** The CH9325 passes the UART stream through unchanged: the
  UT804's HID reports carried its RS232 packet format
  (`../ut803/reverse-engineered-protocol.md` §2).
- **[DEDUCED]** The UCI manual lists the UT632 on HID, but uci.dll's
  QinHeng path recognises only `AC` and `AB CD` headers [VENDOR]
  (`../uci-bench-family/reverse-engineered-protocol.md` §2.3). A 14-byte
  index frame starts `1_`, so uci.dll would decode none of it either.
- No UT632 HID path exists in UT803.exe (§2.3), so no vendor code shows
  the UT632's HID stream.

---

## 5. What Needs Hardware — [UNVERIFIED]

Still open, and settled only by a UT632 capture:

- the payload encoding — the UT804.exe twin decodes LCD segments for the
  UT60A/B/C; UT803.exe holds no decoder;
- the frame length — UT803.exe checks none;
- the line format (§1.2);
- whether the meter needs a button press to send;
- whether the UT632N differs.

The other vendor source not yet opened is UNI-T's general-purpose PC
software (`docs/verification-backlog.md`, "Vendor sources not yet read").

---

## 6. Sources

- UT803.exe V1.01 (MD5: 6dd98644d82edaa4fb0e2e230cf68bc6; the app
  calls itself Ver 1.10) — Ghidra 12.1.3 decompilation of the form's
  published methods and the handlers they install, seeded from the Delphi
  RTTI (2026-09-16); disassembly of the conditions; the form resource
- UT804.exe V2.00 (MD5: 9ef22cff570ba9e8b79e6f1867aad2e5) — its
  `H60BRData`, for comparison
- UNI-T bench programming manual V1.1 — the device list's UT632 entry
- uci.dll (UNI-T SDK V2.3) — QinHeng path, through
  `../uci-bench-family/reverse-engineered-protocol.md`
