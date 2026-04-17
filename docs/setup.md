# Setup

You need a [supported multimeter](supported-devices.md) (UNI-T UT61+/UT161, UT8802, UT8803, UT803/UT804, UT171, UT181A, or Voltcraft VC-880/VC650BT/VC-890) connected via USB.

## Install from pre-built binaries

Download the latest release for your platform from the [Releases](https://github.com/antoinecellerier/dmm-tools/releases) page. Extract and run — no build tools needed.

## Build from source

Requires the [Rust toolchain](https://rustup.rs/) (stable, 2024 edition).

**Linux** also needs `libudev-dev` (Debian/Ubuntu) or `systemd-devel` (Fedora) for hidapi.

```sh
cargo build --workspace
```

Or install directly:

```sh
cargo install --git https://github.com/antoinecellerier/dmm-tools.git dmm-cli
cargo install --git https://github.com/antoinecellerier/dmm-tools.git dmm-gui
```

## Platform setup

### Linux — udev rule

To allow non-root access to the HID device (covers CP2110, CH9329, and CH9325 adapters):

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

The CP2110 adapter may require a driver from [Silicon Labs](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers). After installation, verify the device appears in Device Manager under "Human Interface Devices" or "USB Devices". The CH9329 and CH9325 adapters are standard HID devices and need no driver.

### macOS — no driver needed

macOS recognizes all three USB adapters (CP2110, CH9329, CH9325) as standard HID devices via IOKit — no driver installation is required. Plug in the USB adapter and it should appear automatically.

If the device is not detected, check **System Settings > Privacy & Security > Input Monitoring** and ensure your terminal app (or the GUI binary) has permission to access input devices.

> **macOS Intel note:** macOS ARM (Apple Silicon) has been confirmed working against real hardware. Intel Mac builds are provided but have not been tested yet — if you have an Intel Mac, please [report your experience](https://github.com/antoinecellerier/dmm-tools/issues/2).

## Troubleshooting

### "USB adapter not found"

- Verify the USB adapter is plugged in
- **Linux:** `lsusb | grep -E '10C4:EA80|1A86:E429|1A86:E008'` — look for CP2110 (`10C4:EA80`), CH9329 (`1A86:E429`), or CH9325 (`1A86:E008`). If missing, check the udev rule (see above)
- **Windows:** check Device Manager for the CP2110 device — if missing or showing an error, reinstall the driver
- **macOS:** `ioreg -p IOUSB -l | grep CP2110` — if missing, try a different USB port or hub. Check System Settings > Privacy & Security > Input Monitoring if the device appears in `ioreg` but the tool can't open it

### "No response from meter"

The USB adapter is detected but the meter isn't transmitting data:

1. Insert the USB module into the meter's IR port
2. Turn the meter on
3. Long press the **USB/Hz** button until the **S** icon appears on the LCD
4. The S icon confirms USB data transmission is active

### GUI shows a black screen or won't render

On devices with older GPUs (e.g. Raspberry Pi 3B+, OpenGL 2.1), the default wgpu renderer may fail. The GUI automatically falls back to the glow (OpenGL) renderer, but you can also force it explicitly:

```sh
dmm-gui --renderer glow
```

### GUI won't start (Linux, Wayland/X11)

If you encounter display issues on Wayland, try forcing X11:

```sh
WINIT_UNIX_BACKEND=x11 dmm-gui
```
