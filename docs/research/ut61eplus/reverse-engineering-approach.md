# Reverse Engineering Approach: UT61E+ Protocol

## Objective

Reconstruct the UT61E+ USB communication protocol using only official,
publicly available sources:

1. **UT61E+ User Manual** (from UNI-T, `ut61e_manual.pdf`)
2. **CP2110 Datasheet** (from Silicon Labs, `CP2110_datasheet.pdf`)
3. **AN434: CP2110/4 Interface Specification** (from Silicon Labs)
4. **UNI-T official Windows software** (from meters.uni-trend.com)

No community implementations, forum posts, or third-party reverse
engineering work.

## What Each Source Provides

### UT61E+ User Manual

The manual is a consumer-facing document. It does not describe the USB
protocol, but it fully defines the **application-layer semantics** that the
protocol must encode:

- **Measurement modes**: ACV, DCV, AC+DC V, ACmV, DCmV, Hz, Duty%, Ohm,
  Continuity, Diode, Capacitance, hFE, DC/AC uA/mA/A, NCV (18+ modes for
  the UT61E+; UT61D+ adds TempC/TempF, UT61B+ has fewer)
- **Display**: 22,000 counts maximum, implying at least 5 display digits
- **Bar graph**: 46 segments, 30 updates/sec
- **Numeric refresh rate**: 2-3 times/sec
- **Features that must appear as flags**: HOLD, REL, MIN/MAX, Peak MIN/MAX,
  AUTO range, HV alarm, Low Battery
- **Range structure**: Each mode has well-defined ranges (e.g., DCV has
  220mV / 2.2V / 22V / 220V / 1000V)
- **USB data transmission**: Enabled by long-pressing a button; requires an
  insertable USB communication module; software downloadable from
  uni-trend.com

The manual tells us *what* data the protocol must carry, but not *how*.

### CP2110 Datasheet

The datasheet defines the **hardware capabilities** of the USB-to-UART
bridge chip:

- **USB identification**: Default VID 0x10C4, PID 0xEA80 (one-time
  programmable; UNI-T may or may not have changed these)
- **UART range**: 300 bps to 1 Mbps, 5/6/7/8 data bits, configurable
  parity and stop bits
- **FIFOs**: 480 bytes TX, 480 bytes RX
- **Default UART config**: Disabled, 115200 baud, 8N1, no flow control
  (this is the *chip* default; the host software must configure the actual
  baud rate before communication begins)
- **USB class**: HID (Human Interface Device) - no custom driver needed

Key insight: The CP2110 is a transparent UART bridge. It does not define or
constrain the application-layer protocol. The baud rate, framing, and
message format are entirely determined by the meter's firmware and must be
set by the host software via HID feature reports.

### AN434: CP2110/4 Interface Specification

AN434 is the most technically detailed source. It fully specifies the
**transport layer** between the host and the CP2110 chip:

**Data transfer (Report IDs 0x01-0x3F):**
- UART data is carried in HID interrupt transfers
- Report ID = number of data bytes (1-63 bytes per transfer)
- Data starts at byte index 1; index 0 is the report ID
- The CP2110 delivers received UART bytes in interrupt IN reports
  automatically

**Device configuration (Feature Reports):**
- **0x41**: Get/Set UART Enable (0x00=disabled, 0x01=enabled)
- **0x42**: Get UART Status (TX/RX FIFO counts, error/break status)
- **0x43**: Set Purge FIFOs (0x01=TX, 0x02=RX, 0x03=both)
- **0x46**: Get Version Information (CP2110 returns 0x0A as part number)
- **0x50**: Get/Set UART Config:
  - Bytes 1-4: Baud rate (big-endian unsigned 32-bit)
  - Byte 5: Parity (0x00=None, 0x01=Odd, 0x02=Even, 0x03=Mark, 0x04=Space)
  - Byte 6: Flow control (0x00=None, 0x01=Hardware RTS/CTS)
  - Byte 7: Data bits (0x00=5, 0x01=6, 0x02=7, 0x03=8)
  - Byte 8: Stop bits (0x00=Short/1, 0x01=Long/1.5-2)

**PROM-programmable parameters (one-time write):**
- **0x60**: VID, PID, power, power mode, flush buffers
- **0x61-0x64**: Manufacturing and product strings
- **0x65**: Serial string
- **0x66**: Pin configuration
- **0x47**: Lock byte (indicates which PROM fields are already programmed)

**Operational characteristics:**
- Multibyte values in reports are LSB-first (except baud rate in 0x50
  which is MSB-first/big-endian)
- Set reports produce no acknowledgement; verify with corresponding Get
- Max report size: 64 bytes (index 0-63)
- The UART is disabled by default after power-on; must be enabled via 0x41

### UNI-T Official Windows Software — COMPLETED

**Source**: UT61E+ Software V2.02 downloaded from
meters.uni-trend.com (~41 MB ZIP containing NSIS installer).

#### Extraction
The NSIS installer was extracted with `7z`, yielding 69 files including:
- `DMM.exe` — Qt 5 GUI application (main executable)
- `Lib/CustomDmm.dll` — Protocol plugin (framing, parsing, commands)
- `Lib/CP2110.dll` — Transport plugin (CP2110 HID bridge)
- `DeviceSelector.dll` — USB device discovery
- `SLABHIDtoUART.dll` / `SLABHIDDevice.dll` — Silicon Labs runtime
- `CH9329DLL.dll` — Alternate USB bridge chip support
- `options.xml` — User configuration (model, sample rate)
- `Software User Manual.pdf` — End-user documentation

#### Analysis Techniques Used

**1. String extraction** (`strings -a` and `strings -el`):
- Found mode names in CustomDmm.dll: ACV, ACmV, DCV, DCmV, FREQ, Duty
  Cycle, RES, Short-Circuit, Diode, CAP, Celsius, DCuA, ACuA, DCmA,
  ACmA, DCA, ACA, hFE, Live, NCV, LozV, LPF, AC+DC
- Found Qt signal/slot names: `slotSerialDeviceRead(QByteArray)`,
  `nextCommand(Command*)`, `opened()`, `closed()`
- Found CP2110 API imports: `HidUart_Open`, `HidUart_SetUartConfig`,
  `HidUart_SetUartEnable`, `HidUart_Read`, `HidUart_Write`, etc.
- Found class names: `MyDmm`, `CommandPool`, `CommandPoolThread`,
  `OnceCommandPool`, `LoopCommandPool`, `DeviceSelector`

**2. Binary pattern search** (Python struct/find):
- Searched DMM.exe for baud rates as 32-bit LE values:
  9600 (0x2580) found at 2 locations, 19200 at 3, 115200 at 1
- Searched for VID/PID as 16-bit LE values:
  0x10C4 at 80 locations (many false positives), 0xEA80 at 2
- Found VID, PID, and baud rate clustered at offset 0x4A90-0x4AE0,
  confirming they are part of the same initialization sequence

**3. Ghidra headless decompilation** (Java postScript):
- `CustomDmm.dll` → 422 KB of decompiled C, 13115 lines
- `CP2110.dll` → 100 KB of decompiled C, 3179 lines
- `DMM.exe` → 1.6 MB of decompiled C, 48396 lines
- `DeviceSelector.dll` → 309 KB of decompiled C, 9850 lines

Key functions identified in CustomDmm.dll:
- `FUN_10001000` — Static initializer: builds SI prefix table
- `FUN_100016d0` — MyDmm constructor: sets up command pools, builds
  GetMeasurement command (0x5E)
- `FUN_10002170` — Hold command: sends 0x4A ('J')
- `FUN_100021f0` — Range command: sends 0x46 ('F')
- `FUN_10002460` — **Frame builder**: AB CD header, length, checksum
- `FUN_10002540` — **Frame parser**: validates header and checksum
- `FUN_10007d50` — **Response parser**: extracts mode, range, display,
  flags from measurement response
- `FUN_100023f0` — Mode/range table lookup
- `FUN_100026a0` — OL (overload) detection in display string
- `FUN_100027e0` — Mode/range table builder (too complex for Ghidra
  decompiler; analyzed via disassembly instead — see below)

Key functions identified in CP2110.dll:
- Constructor at 0x10001100: stores baud rate 0x2580 (9600) and
  data bits 3 (= 8 bits)
- Dynamic loading of SLABHIDtoUART.dll via QLibrary::resolve()

Key functions identified in DMM.exe:
- Same protocol code duplicated from CustomDmm.dll (Qt plugin model)
- UI display function reads DmmData fields to set LCD labels:
  - offset 0x38 → "H" (HOLD)
  - offset 0x39 → "Rel" (REL)
  - offset 0x3b → "AUTO" (inverted: field set = hide label)
  - offset 0x3c → passed to widget method (likely LowBat indicator)
  - offsets 0x3a, 0x3d → stored but never read by UI
- Exhaustive search of all `QByteArray::append` calls in all 4 binaries
  confirmed only 3 command bytes are ever constructed: 0x5E, 0x4A, 0x46

**4. Ghidra headless disassembly** (Java postScript):
- The mode/range table builder (`FUN_00413f30` in DMM.exe, equivalent
  to `FUN_100027e0` in CustomDmm.dll) was too complex for Ghidra's
  decompiler ("Too many branches"). Used a custom Ghidra Java script
  to extract the raw disassembly (2000 instructions).
- Each table entry is built by calling `FUN_00411c10` with 3 arguments
  (pushed in reverse order): `PUSH bar_graph_range, PUSH 0x0,
  PUSH packed_mode_range`. The packed value is `(mode << 8) | range_byte`.
- This revealed that **range bytes use a 0x30 prefix**: every range
  byte in the table is 0x30 + range_index (e.g., 0x30, 0x31, 0x32...).
- Mode bytes are raw (0x00, 0x01, 0x02...) with no prefix.
- The complete table was extracted with all mode/range combinations and
  their bar graph full-scale values.

**5. Configuration file analysis**:
- `options.xml` contains: Model ("UT61D+"), Version ("2.02"),
  SampleRate (1000 ms), SamplePoints (1000)

#### Decompilation output preserved at:
- `references/vendor-software/CustomDmm_decompiled.txt`
- `references/vendor-software/CP2110_decompiled.txt`
- `references/vendor-software/DMM_decompiled.txt`
- `references/vendor-software/DeviceSelector_decompiled.txt`

## Commands Reference

Exact commands used for each non-obvious analysis step, for
reproducibility.

### Installer extraction

Identify installer type, then extract with 7z:

```sh
file "Software V2.02/Setup.exe"
# → PE32 ... Nullsoft Installer self-extracting archive
7z x -o"extracted" "Software V2.02/Setup.exe"
```

### String extraction

ASCII and UTF-16LE strings from key binaries:

```sh
strings -a Lib/CustomDmm.dll | head -200
strings -a Lib/CP2110.dll | head -200
strings -el DMM.exe | head -200   # UTF-16 strings
```

Targeted searches for protocol-related strings:

```sh
strings -a Lib/CustomDmm.dll | grep -iE 'ACV|DCV|command|baud|uart'
strings -a DMM.exe | grep -iE 'UT61|CP2110|Device|command|model'
```

### Binary pattern search for numeric constants

Search for baud rate (9600 = 0x2580) and VID/PID as little-endian
packed integers in the executable:

```python
import struct
with open('DMM.exe', 'rb') as f:
    data = f.read()

# Search for 9600 as 32-bit LE
pattern = struct.pack('<I', 9600)  # → b'\x80\x25\x00\x00'
pos = data.find(pattern)           # → found at 0x4AC0

# Search for VID 0x10C4 as 16-bit LE
pattern = struct.pack('<H', 0x10C4)  # → b'\xC4\x10'
pos = data.find(pattern)              # → found at 0x4A9A
```

The cluster at offsets 0x4A9A (VID), 0x4AAD (PID), 0x4AC0 (baud)
confirmed they are part of the same initialization function.

### Ghidra headless decompilation

Decompile all non-thunk, non-external functions using a Java postScript:

```java
// GhidraDecompile.java
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;

public class GhidraDecompile extends GhidraScript {
    @Override
    public void run() throws Exception {
        DecompInterface decomp = new DecompInterface();
        decomp.openProgram(currentProgram);
        FunctionIterator funcs = currentProgram.getFunctionManager().getFunctions(true);
        while (funcs.hasNext()) {
            Function func = funcs.next();
            if (func.isThunk() || func.isExternal()) continue;
            DecompileResults results = decomp.decompileFunction(func, 30, monitor);
            if (results.decompileCompleted()) {
                String code = results.getDecompiledFunction().getC();
                println("=== FUNCTION: " + func.getName()
                    + " at " + func.getEntryPoint() + " ===");
                println(code);
                println("");
            }
        }
        decomp.dispose();
    }
}
```

Invoked with:

```sh
GHIDRA=/path/to/ghidra_12.0.4_PUBLIC
$GHIDRA/support/analyzeHeadless \
    /tmp/ghidra_project project_name \
    -import Lib/CustomDmm.dll \
    -postScript GhidraDecompile.java \
    -deleteProject \
    -scriptPath /tmp
```

Note: Python postScripts require PyGhidra (`pip install pyghidra`)
which was not available, so Java scripts were used instead.

Multiple binaries can be decompiled in parallel by running separate
analyzeHeadless processes with different `-import` targets and
different project directories.

### Ghidra headless disassembly (for functions too complex to decompile)

When Ghidra's decompiler fails with "Too many branches" (e.g., large
switch-based table builders), fall back to raw disassembly via a custom
Java script:

```java
// GhidraDisasm.java
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.*;
import ghidra.program.model.address.*;

public class GhidraDisasm extends GhidraScript {
    @Override
    public void run() throws Exception {
        Address addr = toAddr(0x00413f30L);  // target function
        Listing listing = currentProgram.getListing();
        int count = 0;
        while (count < 2000 && addr != null) {
            Instruction insn = listing.getInstructionAt(addr);
            if (insn == null) break;
            println(String.format("  %s: %s", addr, insn.toString()));
            addr = insn.getFallThrough();
            if (addr == null) addr = insn.getMaxAddress().add(1);
            count++;
            if (insn.getMnemonicString().equals("RET")) break;
        }
    }
}
```

### Ghidra headless data dump (reading static data at known addresses)

When decompilation resolves references to `DAT_XXXXXXXX` but doesn't
show their content, use a script to read the raw bytes via Ghidra's
Memory API:

```java
// GhidraDumpData.java
import ghidra.app.script.GhidraScript;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.address.Address;

public class GhidraDumpData extends GhidraScript {
    @Override
    public void run() throws Exception {
        Memory mem = currentProgram.getMemory();
        Address a = toAddr(0x0043a620L);
        // Read a null-terminated ASCII string
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 30; i++) {
            byte b = mem.getByte(a.add(i));
            if (b == 0) break;
            if (b >= 32 && b < 127) sb.append((char)b);
        }
        println("String: " + sb.toString());
    }
}
```

### Exhaustive search for command bytes

To confirm that no command bytes were missed, search all four decompiled
outputs for any `QByteArray::append` call with a single-character
argument in the command byte range (0x41-0x5F / 'A'-'_'):

```sh
grep "append.*'[A-Z^_]'" \
    CustomDmm_decompiled.txt \
    CP2110_decompiled.txt \
    DMM_decompiled.txt \
    DeviceSelector_decompiled.txt
```

Result: only `'^'` (0x5E), `'J'` (0x4A), `'F'` (0x46) found.

### Tracing DmmData flag field semantics

The response parser writes flag bits to DmmData struct fields via
setter functions. To identify what each flag means, the approach was:

1. Read each setter's decompiled body to find the struct offset it
   writes to (e.g., `this + 0x3b`).
2. Search the UI binary (DMM.exe) for getter functions that return
   the same offset (e.g., `return *(param_1 + 0x3b)`).
3. Find call sites of those getters and check what string label or
   widget they're associated with.

```sh
# Find all setters (write to struct offsets 0x38-0x3d)
grep '0x3[89a-d])' CustomDmm_decompiled.txt

# Find corresponding getters in DMM.exe
grep 'return.*+ 0x3[89a-d])' DMM_decompiled.txt

# Trace getter call sites with surrounding context
grep -n -C 3 'FUN_0040d3a0' DMM_decompiled.txt
# → shows "AUTO" label logic for offset 0x3b
```

## Deductive Analysis

### What We Can Determine Without a Device

**Transport layer** — Fully specified by AN434:
- The initialization sequence must: enable UART (0x41), configure baud rate
  (0x50), optionally purge FIFOs (0x43)
- Data flows through HID interrupt reports (0x01-0x3F)

**Application constraints** from the manual:
- A measurement response must encode: mode (18+ values), range, display
  value (5+ digits for 22000 counts), bar graph position (0-46), and
  feature flags (HOLD, REL, MIN, MAX, PeakMIN, PeakMAX, AUTO, HV, LowBat,
  polarity)
- Minimum payload size estimate: 1 (mode) + 1 (range) + 5-7 (display) +
  2 (bar graph) + 2-3 (flags) = ~12-14 bytes
- The 2-3 Hz refresh rate and UART throughput constrain the protocol: at
  9600 baud 8N1, each byte takes ~1.04 ms, so a 20-byte message takes
  ~21 ms — easily fits 10+ messages/sec

**Baud rate reasoning:**
- The CP2110 defaults to 115200 but this is the chip default, not the
  meter's choice
- Common multimeter baud rates: 2400 (older UT61E), 9600, 19200
- The 2-3 Hz display refresh and 30 Hz bar graph update suggest moderate
  bandwidth needs; 9600 baud is sufficient and common for this class of
  instrument

### What Was Resolved by Vendor Software Analysis

| Question | Answer | How Found |
|----------|--------|-----------|
| Baud rate | 9600 bps | Binary search in DMM.exe + CP2110.dll constructor |
| UART format | 8N1, no flow control | CP2110.dll constructor |
| VID/PID | 0x10C4/0xEA80 (defaults) | Binary search in DMM.exe |
| Frame header | 0xAB 0xCD | Ghidra decompilation: frame builder |
| Length encoding | payload + 2 (includes checksum) | Ghidra decompilation: frame builder |
| Checksum | 16-bit BE sum of all preceding bytes | Ghidra decompilation: frame builder + parser |
| Communication model | Polled (LoopCommandPool) | Ghidra decompilation: MyDmm constructor |
| Command format | AB CD 03 cmd chk_hi chk_lo | Ghidra decompilation: frame builder |
| GetMeasurement cmd | 0x5E | Ghidra decompilation: MyDmm constructor |
| Hold cmd | 0x4A | Ghidra decompilation: FUN_10002170 |
| Range cmd | 0x46 | Ghidra decompilation: FUN_100021f0 |
| Response size | 19 bytes | Ghidra decompilation: response parser |
| Mode byte position | byte[3], raw (no prefix) | Ghidra decompilation: response parser |
| Range byte position | byte[4], 0x30 prefix | Ghidra disassembly: table builder |
| Range byte encoding | 0x30 + index (mask with & 0x0F) | Ghidra disassembly: table builder |
| Display format | bytes[5-11], ASCII Latin-1 | Ghidra decompilation: response parser |
| Display parsing | strip spaces, toDouble | Ghidra decompilation: response parser |
| OL detection | "O"+"L" substring match | Ghidra decompilation: FUN_100026a0 |
| Flag byte positions | bytes[14-16] | Ghidra decompilation: response parser |
| Flag bit layout (byte 14) | bit0=REL, bit1=HOLD, bit2=MIN, bit3=MAX | Ghidra decompilation + DMM.exe UI |
| Flag bit layout (byte 15) | bit2=!AUTO (inverted) | Ghidra decompilation + DMM.exe UI |
| Flag byte 15 bit 1 | UI widget indicator (likely LowBat) | DMM.exe UI code |
| Flag byte 15 bits 0, 3 | Stored, never displayed by vendor UI | DMM.exe exhaustive search |
| Flag bit layout (byte 16) | bit1=P-MIN, bit2=P-MAX, bit3=DC | Ghidra decompilation: response parser |
| Mode values | 0x00-0x19 table (26 modes) | String table + code path checks |
| SI prefix table | T/G/M/k/K/space/empty/m/µ/n/p | Ghidra decompilation: static initializer |
| Complete mode/range table | All entries with bar graph ranges | Ghidra disassembly: table builder |
| Vendor command set | Only 3 commands (0x5E, 0x4A, 0x46) | Exhaustive search of all 4 binaries |
| Bar graph response bytes | Not used by vendor software | Exhaustive search of response parser |

### What Still Requires Real Device Verification

1. **Flag byte 15 bit 0 and bit 3 names** — Likely HV and reserved.
   The vendor software stores these values but never reads them back
   for display. Only real device testing or a protocol capture can
   confirm the semantics.

2. **Bar graph position encoding (bytes 12-13)** — The vendor software
   does not parse these bytes at all. The bar graph full-scale range
   comes from the mode/range table, but the actual position value
   encoding is unknown.

3. **Commands beyond 0x5E/0x4A/0x46** — The vendor software V2.02 only
   implements GetMeasurement, Hold, and Range. All other commands
   (MinMax, ExitMinMax, Rel, Auto, Light, Select, Peak, ExitPeak,
   GetName) are not present in any of the four decompiled binaries.
   These must be discovered empirically or from a different software
   version.

4. **Timing** — Actual response latency, maximum sustainable polling
   rate.

5. **Edge cases** — NCV display format, hFE display format, temperature
   handling, OL behavior in different modes.

## File Inventory

| Source | File | What it provides |
|--------|------|-----------------|
| UNI-T | `ut61e_manual.pdf` | Measurement modes, ranges, display specs |
| Silicon Labs | `CP2110_datasheet.pdf` | Hardware capabilities, VID/PID, UART |
| Silicon Labs | `AN434_CP2110_interface_spec.pdf` | HID report specification |
| UNI-T | `vendor-software/Software V2.02/Setup.exe` | NSIS installer |
| UNI-T | `vendor-software/extracted/` | Extracted software (69 files) |
| Analysis | `vendor-software/CustomDmm_decompiled.txt` | Ghidra decompilation (422 KB) |
| Analysis | `vendor-software/CP2110_decompiled.txt` | Ghidra decompilation (100 KB) |
| Analysis | `vendor-software/DMM_decompiled.txt` | Ghidra decompilation (1.6 MB) |
| Analysis | `vendor-software/DeviceSelector_decompiled.txt` | Ghidra decompilation (309 KB) |
