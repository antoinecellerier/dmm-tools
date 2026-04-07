# Setup

## Prerequisites

- Rust toolchain (stable, 2024 edition)
- A supported UNI-T multimeter connected via USB (see [supported devices](supported-devices.md) for the full list: UT61E+, UT61B+, UT61D+, UT161 series, UT8803, UT171, UT181A)

**Linux:** `libudev-dev` (Debian/Ubuntu) or `systemd-devel` (Fedora) for hidapi.

**Windows:** Install the [CP2110 HID USB-to-UART bridge driver](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers) from Silicon Labs. The CH9329 adapter is driverless HID on all platforms and needs no separate driver.

## Build

```sh
cargo build --workspace
```

## Platform setup

### Linux — udev rule

To allow non-root access to the HID device (covers both CP2110 and CH9329 adapters):

```sh
sudo cp udev/99-dmm-tools.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Then re-plug the meter or log out/in.

Your user must be in the `plugdev` group:

```sh
sudo usermod -aG plugdev $USER
```

Log out and back in for the group change to take effect.

### Windows — driver

Install the CP2110 driver from [Silicon Labs](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers). After installation, verify the device appears in Device Manager under "Human Interface Devices" or "USB Devices".

### macOS — no driver needed (experimental)

macOS recognizes the CP2110 as a standard HID device via IOKit — no driver installation is required. Plug in the USB adapter and it should appear automatically.

If the device is not detected, check **System Settings > Privacy & Security > Input Monitoring** and ensure your terminal app (or the GUI binary) has permission to access input devices.

> **macOS support is experimental.** It compiles and should work but has not been tested against real hardware. If you have a Mac and a supported meter, please try it and [report your experience](https://github.com/antoinecellerier/dmm-tools/issues/2) — even "it works" is valuable feedback.

## Troubleshooting

### "USB adapter not found"

- Verify the USB adapter is plugged in
- **Linux:** `lsusb | grep -E '10C4:EA80|1A86:E429'` — look for CP2110 (`10C4:EA80`) or CH9329 (`1A86:E429`). If missing, check the udev rule (see above)
- **Windows:** check Device Manager for the CP2110 device — if missing or showing an error, reinstall the driver
- **macOS:** `ioreg -p IOUSB -l | grep CP2110` — if missing, try a different USB port or hub. Check System Settings > Privacy & Security > Input Monitoring if the device appears in `ioreg` but the tool can't open it

### "No response from meter"

The USB adapter is detected but the meter isn't transmitting data:

1. Insert the USB module into the meter's IR port
2. Turn the meter on
3. Long press the **USB/Hz** button until the **S** icon appears on the LCD
4. The S icon confirms USB data transmission is active

### GUI won't start (Linux, Wayland/X11)

The GUI uses eframe/egui which supports both Wayland and X11. If you encounter display issues, try forcing X11:

```sh
WINIT_UNIX_BACKEND=x11 ut61eplus-gui
```
