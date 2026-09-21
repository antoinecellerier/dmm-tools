# UT632 / UT632N: Reverse Engineering Approach

## Sources Used

1. **UT803.exe V1.01** and its form resource (`references/ut800/ut803/`)
   — the app's UT632 configuration: the `UT632` check box, the
   `FormCreate` branch it selects and the handler it installs.
2. **The 2026-09-16 Ghidra 12.1.3 RTTI-seeded handler decompile** of
   UT803.exe (`ut803-handlers-decompiled.txt`), made for the UT803/UT804
   work (`../ut803/reverse-engineering-approach.md`, Methodology step 5).
3. **objdump disassembly** of the same bytes, for every branch condition.
4. **UT804.exe V2.00**'s `H60BRData`, for comparison with UT803.exe's.
5. **UNI-T's bench programming manual V1.1** — its device list, which
   gives the UT632's transport and device address (findings in
   `../uci-bench-family/`).
6. **The uci-bench-family findings** on uci.dll's QinHeng path.

### Avoided (clean-room boundary)

- No community source and no other implementation was consulted for the
  UT632 (as of 2026-09-21).
- The 2026-09-16 opening of the UT803/UT804 boundary
  (`../ut803/reverse-engineering-approach.md`) covered the UT803/UT804
  validation only; nothing consulted then was read for the UT632.

## Methodology

1. **RTTI → `FormCreate` → handlers.** The form's published methods and
   fields came from the Delphi RTTI; `FormCreate`'s check-box branches
   gave the handler each configuration installs; `USBConClick`,
   `SetFeatureClick` and `ReadBtnClick` gave the HID side.
2. **Ghidra plus disassembly.** Ghidra's decompile was the primary read,
   but it drops the branch that follows Delphi's `LStrCmp` (the result is
   in the flags), so each string-compare branch — the `'E'` test, the
   `Read` handler's count tests, UT804.exe's `123456789ABCDE` test — was
   read from the disassembly (`je`/`jne`). The check-box tests are plain
   `test al,al` branches, which Ghidra decompiles correctly.
3. **Full-listing cross-reference.** The readers of the handler's output
   string (`0x5676A0`) were found by cross-referencing the whole listing,
   which is how the absence of any decoder on the UT632 path was
   established.
4. **Twin comparison.** UT804.exe's `H60BRData` was read side by side
   with UT803.exe's; the check and the call it has and UT803.exe's lacks
   are the basis of the 14-byte deduction.

## Confidence Assessment

- **Transport (HID 1A86:E008, the CH9325 of the UT803/UT804):** HIGH —
  the UCI manual's device list gives the VID and PID.
- **2400 baud in the app:** HIGH — the form data and `SetFeatureClick`.
- **Frames end at a high-nibble-E byte; nothing decodes them:** HIGH —
  the handler's only branch, and no reader of its output on that path.
- **No HID path for the UT632:** HIGH — `USBConClick` reads `IFUT803` and
  `IFUT60E` only.
- **14-byte frames, index high nibbles, data in the low nibbles:** MEDIUM
  — deduced from UT804.exe's twin and the sibling `IFUT60E` HID handler;
  UT803.exe checks no length.
- **Payload encoding, line format, need for a button press, UT632N
  differences:** none — nothing in the binaries; needs a capture.
