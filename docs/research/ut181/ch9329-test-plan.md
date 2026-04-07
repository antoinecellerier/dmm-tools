# CH9329 (UT-D09 Cable) Test Plan

This document describes the step-by-step testing procedure for validating
CH9329 transport support. It is designed for someone with a UT181A (or
UT171/UT243) and the WCH CH9329-based UT-D09 cable.

## Prerequisites

1. Build from source:
   ```sh
   git clone https://github.com/antoinecellerier/dmm-tools.git
   cd dmm-tools
   cargo build --release
   ```

2. Install udev rule (Linux only):
   ```sh
   sudo cp udev/99-dmm-tools.rules /etc/udev/rules.d/
   sudo udevadm control --reload-rules
   ```
   Unplug and replug the cable after installing the rule.

3. Enable communication on the UT181A: go to SETUP menu, set
   Communication to ON.

## Step 1: Verify USB detection

**Goal:** Confirm the CH9329 cable is visible to the OS.

```sh
lsusb | grep 1A86
```

**Expected:** A line containing `1A86:E429` (WCH CH9329).

If nothing appears, the cable is not recognized — check the physical
connection and that the meter is powered on.

**Also run:**
```sh
lsusb -v -d 1a86:e429 2>/dev/null | head -40
```

**Report:** Paste the full output — we need the interface descriptors
and usage pages to understand which HID mode the CH9329 is configured in.

## Step 2: Check hidraw access

```sh
ls -la /dev/hidraw*
```

Note which hidraw devices exist. Then plug in the cable and run again:

```sh
ls -la /dev/hidraw*
```

**Report:** Which new hidraw device(s) appeared? If more than one
appeared for the same cable, list all of them — this means the CH9329
is in composite mode (keyboard + mouse + custom HID) and we need to
identify the correct interface.

Check permissions:
```sh
# Should show rw for your user after udev rule is installed
test -r /dev/hidrawN && echo "readable" || echo "NOT readable"
test -w /dev/hidrawN && echo "writable" || echo "NOT writable"
```

## Step 3: Run `list` to verify adapter detection

```sh
./target/release/ut61eplus list
```

**Expected:** The CH9329 adapter appears with `[CH9329]` label:
```
[0] /dev/hidrawN [CH9329] — WCH UART TO KB-MS (S/N: ...)
```

**If it shows `No devices found`:** The udev rule may not be installed,
or the VID:PID doesn't match. Report the `lsusb` output from Step 1.

**If multiple entries appear for CH9329:** The cable is in composite
mode. Note which hidraw paths appeared — we may need to select the
correct HID interface (the custom HID one, not keyboard/mouse).

## Step 4: Attempt basic communication (the critical test)

```sh
RUST_LOG=ut61eplus_lib=trace ./target/release/ut61eplus --device ut181a debug --count 5
```

### Outcome A: Data flows

You see measurement output like:
```
transport: CH9329
[0] mode_raw=... display="..." → 1.234 V DC
[1] ...
```

**This means CH9329 support works!** Proceed to Step 5.

### Outcome B: Timeout / no data

You see:
```
transport: CH9329
[0] error: timeout waiting for response
```

The TRACE log above will show lines like:
```
CH9329 TX: [00, 06, AB, CD, 03, 5E, 01, D9]
```

**If you see TX lines but no RX lines:** The write path works but the
meter isn't responding. This likely means the CH9329 needs UART
configuration (baud rate init) before data flows.

**Try reading raw HID data directly:**
```sh
sudo timeout 5 cat /dev/hidrawN | xxd | head -20
```

If you see data here but not from our tool, the issue is in our report
framing. **Report the xxd output** — it tells us the exact byte layout.

**If no data from `cat` either:** The meter may not be sending data.
Verify Communication is ON in the meter's SETUP menu, and that the
cable is fully inserted.

### Outcome C: Wrong interface (composite mode)

If `debug` hangs or shows errors about HID open failure, and Step 2
showed multiple hidraw devices for the cable:

```sh
# Try each hidraw device manually to find the data one
for dev in /dev/hidraw*; do
  echo "=== $dev ==="
  sudo timeout 2 cat "$dev" | xxd | head -5
  echo
done
```

**Report:** Which device(s) produced data? This tells us which HID
interface carries the UART stream.

### Outcome D: Report ID mismatch

If the TRACE log shows received bytes but the parsed data looks wrong
(garbage values, wrong lengths), there may be a platform-specific
report ID handling issue.

**Report the full TRACE output** — specifically lines like:
```
CH9329 RX (N bytes, raw[0]=0xXX): [...]
```

The `raw[0]` value tells us whether the report ID byte is included
(0x00) or stripped (some other value) by hidapi on your platform.

## Step 5: Full measurement validation (if Step 4 succeeded)

```sh
./target/release/ut61eplus --device ut181a read --count 10
```

Compare each value shown against the meter's LCD display. They should
match within the displayed precision.

**Try different modes:**
- DC V: touch the probes together (should show ~0.000 mV)
- AC V: leave probes open (should show noise)
- Ω: short the probes (should show low resistance)

## Step 6: Remote commands (if Step 5 works)

```sh
./target/release/ut61eplus --device ut181a command hold
```

**Expected:** The meter toggles HOLD mode. This confirms bidirectional
communication.

## Step 7: Capture wizard (comprehensive validation)

```sh
./target/release/ut61eplus --device ut181a capture
```

This walks through each measurement mode and records raw data for
analysis. Follow the prompts. At the end it saves a YAML report file.

**Share the YAML file** — it contains everything we need to verify
protocol correctness.

## What to report

Please include:
1. Operating system and version
2. Output from Steps 1-4 (even if something fails — partial results
   help diagnose)
3. The YAML capture file from Step 7 (if you get that far)
4. Screenshots of the LCD alongside tool output are very helpful

## Troubleshooting quick reference

| Symptom | Likely cause | What to try |
|---------|-------------|-------------|
| `No devices found` | udev rule missing or wrong VID:PID | Check `lsusb`, reinstall udev rule |
| Timeout, no RX data | CH9329 needs baud rate config | Report TRACE output, try raw `cat /dev/hidraw` |
| Garbage/wrong values | Report ID byte handling differs | Report TRACE output with `raw[0]` values |
| Multiple hidraw devices | Composite mode (Mode 0) | Try each device, report which has data |
| Permission denied | udev rule not loaded | `sudo udevadm control --reload-rules`, replug cable |
