# Setup

## Prerequisites

- Rust toolchain (stable, 2024 edition)
- `libudev-dev` (Linux, for hidapi)
- UNI-T UT61E+ multimeter connected via USB

## Build

```sh
cargo build --workspace
```

## udev Rule (Linux)

To allow non-root access to the HID device:

```sh
sudo cp udev/99-cp2110-unit.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Then re-plug the meter or log out/in.

## Troubleshooting

### "USB adapter not found"

- Verify the CP2110 USB adapter is plugged in: `lsusb | grep 10C4:EA80`
- Ensure the udev rule is installed (see above)
- After installing the rule, re-plug the adapter or run `sudo udevadm trigger`

### "No response from meter"

The USB adapter is detected but the meter isn't transmitting data:

1. Insert the USB module into the meter's IR port
2. Turn the meter on
3. Long press the **USB/Hz** button until the **S** icon appears on the LCD
4. The S icon confirms USB data transmission is active

### Permission denied on /dev/hidrawX

The udev rule grants access to the `plugdev` group. Ensure your user is in that group:

```sh
sudo usermod -aG plugdev $USER
```

Log out and back in for the group change to take effect.

### GUI won't start (Wayland/X11)

The GUI uses eframe/egui which supports both Wayland and X11. If you encounter display issues, try forcing X11:

```sh
WINIT_UNIX_BACKEND=x11 ut61eplus-gui
```
