# Setup

You need a [supported multimeter](supported-devices.md) and its USB cable, or one of the Bluetooth adapters listed there for the meters that take one.

## Install from pre-built binaries

Download the [latest release](https://github.com/antoinecellerier/dmm-tools/releases/latest) for your platform. Extract and run. The first launch shows a security warning on [Windows](#windows-protected-your-pc) and [macOS](#macos-wont-open-dmm-cli-or-dmm-gui). The GUI links to newer versions as they come out ([update checks](gui-reference.md#update-checks)).

To build it yourself instead, see [Build from source](#build-from-source).

Once it runs, the [CLI reference](cli-reference.md) and [GUI reference](gui-reference.md) describe the commands and options.

### Dev builds

To try unreleased changes without installing a Rust toolchain, use a dev build — built nightly from `main`.

Find them in the [dev build listing](https://github.com/antoinecellerier/dmm-tools/releases?q=prerelease%3Atrue), newest first. Archives are named `dmm-tools-v<version>-dev-<commit>-<platform>`. Their first launch shows the same security warning on [Windows](#windows-protected-your-pc) and [macOS](#macos-wont-open-dmm-cli-or-dmm-gui).

Dev builds may be broken, and only the newest seven are kept. When reporting a problem, include the version you're running, from `dmm-cli --version`.

### Android (experimental)

An experimental build of the GUI runs on phones and tablets with Android 7 or later and a 64-bit ARM processor. It reaches meters over Bluetooth and, on a device that can host USB, over their cable. The [Android port note](research/android-port.md#experimental-build) lists what was tried.

1. On the phone, download the file ending in `-android-arm64.apk` from the newest [dev build](#dev-builds), or from a release from v0.11.0 on, and open it.
2. Android asks whether the browser or file manager may install unknown apps. Allow it, then choose **Install**. Play Protect may offer to scan the app first.
3. On first start, allow the permission Android asks for: it covers the search for Bluetooth meters. Plug a cable in and Android offers to open dmm-tools for it.

Where Android refuses apps from unverified developers, either [allow them in Android's settings](https://support.google.com/android/answer/17588095), or install from a computer with `adb install <file>.apk`.

To update, install a newer file the same way; settings are kept. The top bar links to a newer build when one is out ([update checks](gui-reference.md#update-checks)).

**Full screen** in Settings hides Android's status and navigation bars; a swipe from the screen's edge shows them.

On Android 13 and later, Android's settings page for the app offers its languages. The app starts in the one picked there while its own Language setting is on Auto.

## Platform setup

### Linux — udev rule

To allow non-root access to the USB cable:

```sh
sudo cp udev/70-dmm-tools.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Then re-plug the cable. The rule tags the device `uaccess`, so whoever is
logged in at the local seat gets access. Check it with
`getfacl /dev/hidrawN` — your user should appear as `user:<you>:rw-`.

The file name matters: a rule that sorts after `73-seat-late.rules` is
applied too late and silently does nothing. If you installed an earlier
release's rule, remove it:

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
| Alpine, Void, Artix, Devuan, Gentoo/OpenRC | Use the group fallback below — eudev does not implement `uaccess`. |

#### Headless machines, and distributions without logind

A machine with no local seat — an SSH-only server or Raspberry Pi — gets no
access from the tag, and neither does a system running eudev without logind.
Fall back to a group: pick one your user is already in, or create a dedicated
one, and append it to each rule in `/etc/udev/rules.d/70-dmm-tools.rules`:

```
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="10c4", ATTRS{idProduct}=="ea80", TAG+="uaccess", GROUP="dmm", MODE="0660"
```

```sh
sudo groupadd -f dmm
sudo usermod -aG dmm $USER
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Log out and back in for the group change to take effect. Leave the tag in
place; one file then covers both cases.

Do not use the `input` group for this. Its members can read every keystroke
and mouse event on the machine, a far wider grant than one multimeter cable
([#17](https://github.com/antoinecellerier/dmm-tools/issues/17)).

### Windows — driver

Open Device Manager with the cable plugged in. A CP2110 cable shown with a yellow warning icon under "Other devices" needs the driver from [Silicon Labs](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers); under "Human Interface Devices" it is ready. The CH9329, CH9325 and Brymen BU-86X cables are standard HID devices and need no driver.

### macOS — no driver needed

macOS recognizes every supported cable as a standard HID device — plug the cable in and it appears automatically.

If the device is not detected, check **System Settings > Privacy & Security > Input Monitoring** and ensure your terminal app (or the GUI binary) has permission to access input devices.

> **macOS Intel note:** macOS ARM (Apple Silicon) has been confirmed working against real hardware. Intel Mac builds are provided but have not been tested yet — if you have an Intel Mac, please [report your experience](https://github.com/antoinecellerier/dmm-tools/issues/2).

### Bluetooth

For the UT-D07B adapter, turn on Bluetooth on the computer, put batteries in
the adapter and fit it to the meter, turn the meter on and switch its data
transmission on — the step, and which meters take the adapter, are in
[supported devices](supported-devices.md). `dmm-cli list` then shows the
adapter, `auto` finds it when no USB cable is plugged in, and
`--adapter <address>` pins it. Pair the adapter in the system's Bluetooth
settings for a quicker connection.

The UT-D07A takes the same steps and has not been run on this tool — please
[report what it does](https://github.com/antoinecellerier/dmm-tools/issues/25).

The UT60BT and UT202BT have Bluetooth built in and take the same steps
without the adapter: turn the meter on, then long-press SEL on the UT60BT,
or short-press the Bluetooth button on the UT202BT, until the Bluetooth
symbol shows. The UT202BT switches Bluetooth off after 5 minutes without a
connection. For either, keep **Look for Bluetooth devices** ticked and
leave out `--no-bluetooth`; with that box unticked, `dmm-cli --bluetooth`
scans for one run.

The ZOTEK meters (ZOYI, BSIDE, ANENG) have Bluetooth built in too and need no
pairing: switch Bluetooth on at the meter as
[supported devices](supported-devices.md) lists, and it shows up as
"Bluetooth DMM".

The EEVblog 121GW has Bluetooth built in and needs no pairing either: hold
1ms PEAK until BT shows, and it shows up as "121GW". It switches off after
30 minutes; set APO.oF in its SETUP menu to disable that.

The Brymen BM788BT and BM787BT have Bluetooth built in and need no pairing
either: disconnect any other app, then, on any function but Auto V/LoZ, hold
Δ for one second or more until ((D)) shows, and it shows up as "BM78xBT". It
switches off after 15 to 30 minutes idle; hold SELECT while turning it on to
disable that. A renamed meter opens with
`--device bm78xbt --adapter <address>`, the address from the other devices
`dmm-cli list` heard, or in dmm-gui from the **Bluetooth device** list in
Settings.

OWON's meters and the Voltcraft VC871, VC891, VC915 and VC925 PV have
Bluetooth built in and need no pairing either: disconnect any other app,
then hold the meter's Bluetooth key, as
[supported devices](supported-devices.md) lists, until the Bluetooth symbol
shows. The meter shows up as "BDM", or as "Lilliput" on some systems (B35T+,
B41T+); it may show up under its model name. Bluetooth switches off
after 10 minutes idle on the B33, B35T+, B41T+, OW16B, OW18B and OW18E, and
after 5 on the CM2100B and OW67B; switch it on again after every power-on
of a VC871 or VC891. A meter under any other name opens with
`--device <id> --adapter <address>`, the id from
[Devices](cli-reference.md#devices) and the address from the other devices
`dmm-cli list` heard, or in dmm-gui from the **Bluetooth device** list in
Settings.

A device opened by its address, either way, is taken for the meter: if it
offers a Bluetooth service the supported meters use, it is sent that meter's
requests. Choose only your own meter.

Readings arrive as fast as the link delivers: on a UT61E+, about 3 a second
over the adapter, against about 10 over the cable.

**Linux:** BlueZ must be running, as it is by default on desktop installs.

**Windows:** nothing beyond the steps above.

**macOS:** untested — please
[report your experience](https://github.com/antoinecellerier/dmm-tools/issues).
`dmm-cli list` names the adapter by a UUID rather than an address, and the
first connection asks you to allow Bluetooth for your terminal app (or the
GUI binary).

### Language

dmm-gui speaks the language picked in its Settings **Language** row, and dmm-cli the one its `--lang` option names, else the row's. On **System** (or `--lang system`), the apps speak the first of the computer's languages they have, and English if they have none:

- **Linux:** each language in `LANGUAGE`, then those of `LC_ALL`, `LC_MESSAGES` and `LANG`. `LC_ALL=C` (or `POSIX`) means English, whatever `LANGUAGE` says.
- **Windows:** the display language.
- **macOS:** the preferred languages, in order.

Chinese needs a Chinese font on the computer; Windows and macOS include one. On Linux, see ["No font for Chinese characters was found"](#no-font-for-chinese-characters-was-found).

## Troubleshooting

### "Windows protected your PC"

SmartScreen shows this on the first launch. Click **More info**, then
**Run anyway**.

### macOS won't open dmm-cli or dmm-gui

macOS blocks the first launch of each binary. Close the dialog, open
**System Settings > Privacy & Security**, click **Open Anyway** and confirm.

To allow both binaries at once, run this in the extracted folder instead:

```sh
xattr -dr com.apple.quarantine .
```

### "App not installed" (Android)

The phone's processor is not 64-bit ARM, or it holds a copy of dmm-tools built on another machine. Uninstall that copy, then install again.

### GUI shows a black screen or won't render

On devices with older GPUs (e.g. Raspberry Pi 3B+, OpenGL 2.1), the default wgpu renderer may fail. The GUI automatically falls back to the glow (OpenGL) renderer, but you can also force it explicitly:

```sh
dmm-gui --renderer glow
```

### GUI won't start (Linux, Wayland/X11)

If you encounter display issues on Wayland, try forcing X11:

```sh
WAYLAND_DISPLAY= dmm-gui
```

### "No meter found over USB or Bluetooth"

The GUI puts the steps for your platform on screen as its [connection
help](../assets/gui-connection-help.webp); the full list is:

- Verify the USB cable is plugged in
- **Linux:** `lsusb | grep -iE '10c4:ea80|1a86:e429|1a86:e008|0820:0001'` — one of the cables should be listed. If missing, try another port
- **Linux, cable listed by `lsusb` but still not found:** `ls -l /dev/hidraw*` — the cable's node should show a trailing `+`, marking the ACL. For the detail, `getfacl /dev/hidrawN` (from the `acl` package) should list your user as `user:<you>:rw-`. If it doesn't, the udev rule isn't installed under a name that sorts before `73-seat-late.rules`, or you're on a [headless machine](#headless-machines-and-distributions-without-logind)
- **Windows:** check Device Manager for the cable: the CP2110 under its own name, the other cables as "USB Input Device". If nothing appears, try a different USB port; on a yellow warning icon under "Other devices", install the [driver](#windows--driver)
- **macOS:** `ioreg -p IOUSB -l -w0 | grep -E '"idVendor" = (4292|6790|2080)'` — one of the cables should be listed. If missing, try a different USB port or hub. Check System Settings > Privacy & Security > Input Monitoring if the device appears in `ioreg` but the tool can't open it
- **Bluetooth:** see [Bluetooth adapter not found](#bluetooth-adapter-not-found)

### "No response from meter"

The cable is found but the meter isn't sending. The tool lists what each
meter needs switched on; the same steps are under each family in
[supported devices](supported-devices.md). If you named a `--device`, check
it matches the meter.

With several cables plugged in, `auto` probes only one, and a Brymen BU-86X
only when no other cable is plugged in: name the meter, or pass `--adapter`.

Over Bluetooth, a meter switched off gives the same message; readings resume
on their own once it transmits again.

### Bluetooth adapter not found

`dmm-cli list` scans but shows no adapter:

- The adapter's blue LED shows its state: one flash every 3 s is waiting,
  two every 1.5 s is connected, off is standby after 5 minutes without a
  connection or data. From standby, switch the adapter off and on to wake
  it.
- Blinking its waiting pattern and listed as paired, yet it never connects:
  remove (forget) it in the system's Bluetooth settings and connect again.
  Seen after the same dual-boot computer paired it under Windows.
- A phone app connected to the adapter keeps it off the air — the adapter takes
  one connection at a time. Disconnect it there first.
- A USB cable wins over Bluetooth: with a cable plugged in, `auto` uses it even
  when the meter behind it says nothing. Unplug the cable, or pass
  `--adapter <address>`. Naming the meter skips a Brymen BU-86X unless the
  meter is one of its own.
- Tick **Look for Bluetooth devices** in the GUI's settings, or pass
  `dmm-cli --bluetooth` for one run; `--no-bluetooth` turns scanning off.

A UT-D07A that is listed and connects but fails with "the Bluetooth device
has no data … characteristic we recognise" carries a different service
layout — please report it on
[#25](https://github.com/antoinecellerier/dmm-tools/issues/25) with
the output of `bluetoothctl info <address>`.

### "Some Bluetooth readings arrive late"

Over Bluetooth, a reading sometimes arrives late, grouped with the next
one. None are lost; only their timestamps are off.

- **The UT-D07B on Linux:** the message ends with a command. Run it as
  printed while the meter is connected; it lasts until the meter
  disconnects. It comes from BlueZ's `hcitool`, in the `bluez` package on
  Debian and Ubuntu; without it the message has none.
- **The UT-D07B on Windows 11 and Android:** the tool shortens the link
  itself.
- **Any other case:** the message asks for a report. Name the meter and the
  operating system. On Windows 11 or Android, try the UT-D07B's fix on your
  meter first, and say in the report whether the message came back: on
  Windows, start the tool with `DMM_BLE_SHORT_INTERVAL=1` set; on Android,
  run `adb shell setprop debug.dmm.ble_short_interval 1`, then restart the
  app.
- **Any system:** a USB cable gives on-time readings.

### "No font for Chinese characters was found"

The computer has no font that draws Chinese. Install one, then restart dmm-gui:

```sh
sudo apt install fonts-noto-cjk                # Debian, Ubuntu, Raspberry Pi OS
sudo dnf install google-noto-sans-cjk-fonts    # Fedora
sudo pacman -S noto-fonts-cjk                  # Arch
```

## Build from source

Requires the [Rust toolchain](https://rustup.rs/) (stable, 2024 edition).

**Linux** also needs `libudev-dev` (Debian/Ubuntu) or `systemd-devel` (Fedora) for hidapi, and `libdbus-1-dev` (Debian/Ubuntu) or `dbus-devel` (Fedora) for Bluetooth.

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
