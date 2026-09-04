# Setup

You need a [supported multimeter](supported-devices.md) (UNI-T UT61+/UT161, UT8802, UT8803, UT803/UT804, UT171, UT181A, or Voltcraft VC-880/VC650BT/VC-890) connected via USB.

## Install from pre-built binaries

Download the latest release for your platform from the [Releases](https://github.com/antoinecellerier/dmm-tools/releases) page. Extract and run — no build tools needed.

## Build from source

Requires the [Rust toolchain](https://rustup.rs/) (stable, 2024 edition).

**Linux** also needs `libudev-dev` (Debian/Ubuntu) or `systemd-devel` (Fedora) for hidapi.

### Clone and build

```sh
git clone https://github.com/antoinecellerier/dmm-tools.git
cd dmm-tools
cargo build --release --workspace
```

The binaries are `target/release/dmm-cli` and `target/release/dmm-gui`. Run them from there, e.g. `./target/release/dmm-cli read`, or let cargo build and run in one step:

```sh
cargo run --release -p dmm-cli -- read
cargo run --release -p dmm-gui
```

### Install with cargo

```sh
cargo install --git https://github.com/antoinecellerier/dmm-tools.git dmm-cli
cargo install --git https://github.com/antoinecellerier/dmm-tools.git dmm-gui
```

This builds the binaries and copies them to `~/.cargo/bin`. If that directory is not on your `PATH`, run them from there, e.g. `~/.cargo/bin/dmm-cli read`.

See the [CLI reference](cli-reference.md) and [GUI reference](gui-reference.md) for command-line options.

## Platform setup

### Linux — udev rule

To allow non-root access to the HID device (covers CP2110, CH9329, and CH9325 adapters):

```sh
sudo cp udev/70-dmm-tools.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Then re-plug the meter. The rule tags the device `uaccess`, and logind puts an
ACL on the device node for whoever is logged in at the local seat, dropping it
again at logout. Check it with `getfacl /dev/hidrawN` — your user should appear
as `user:<you>:rw-`.

The file name matters. logind acts on the tag from its own
`73-seat-late.rules`, so a rule that sets the tag has to sort before it — a
`99-` rule is applied too late and silently does nothing. If you installed an
earlier release's rule, remove it:

```sh
sudo rm -f /etc/udev/rules.d/99-dmm-tools.rules
```

#### Distribution notes

The steps above are all that is needed on Fedora, RHEL and its rebuilds, Debian,
Ubuntu and their systemd-based derivatives, Raspberry Pi OS, Arch, and openSUSE.
Two families need something different:

| Distribution | What to do |
| --- | --- |
| NixOS | Install the file through `services.udev.packages` so it keeps its `70-` name. `services.udev.extraRules` writes `99-local.rules`, too late for the tag ([nixpkgs#308681](https://github.com/NixOS/nixpkgs/issues/308681)). |
| Alpine, Void, Artix, Devuan, Gentoo/OpenRC | Use the group fallback below — eudev does not implement the `uaccess` builtin, so without logind the tag is silently ignored. |

#### Headless machines, and distributions without logind

A machine with no local seat — an SSH-only server or Raspberry Pi — gets no ACL,
and neither does a system running eudev without logind. Fall back to a group:
pick one your user is already in, or create a dedicated one, and append it to
each rule in `/etc/udev/rules.d/70-dmm-tools.rules`:

```
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="10c4", ATTRS{idProduct}=="ea80", TAG+="uaccess", GROUP="dmm", MODE="0660"
```

```sh
sudo groupadd -f dmm
sudo usermod -aG dmm $USER
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Log out and back in for the group change to take effect. Leave the tag in place
— it is harmless where it does nothing, so one file covers both cases.

Do not use the `input` group for this, even though it exists everywhere and does
work. Its members can read `/dev/input/event*` — every keystroke and mouse event
on the machine, from every application and the login screen. That is a far wider
grant than access to one multimeter cable
([#17](https://github.com/antoinecellerier/dmm-tools/issues/17)).

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
- **Linux, cable listed by `lsusb` but still not found:** `ls -l /dev/hidraw*` — the meter's node should show a trailing `+`, marking the ACL. For the detail, `getfacl /dev/hidrawN` (from the `acl` package) should list your user as `user:<you>:rw-`. If it doesn't, the udev rule isn't installed under a name that sorts before `73-seat-late.rules`, or you're on a headless machine (see above)
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
