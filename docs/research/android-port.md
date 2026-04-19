# Android port feasibility

*Last updated 2026-04-19.*

## Summary

An Android port of `dmm-gui` is technically feasible but faces two
substantive obstacles beyond the usual cross-compilation concerns:

1. Android's USB security model does not expose `/dev/hidraw*` or
   `/dev/bus/usb/*` to regular applications. Devices must be opened through
   the Java `UsbManager` API, which returns a file descriptor to native
   code.
2. The `hidapi` crate that `dmm-lib` depends on does not build for
   `aarch64-linux-android` in any of its supported configurations.

Obstacle (1) disappears on rooted devices but (2) does not, so any Android
build requires either a different Rust USB crate (`nusb` is the leading
candidate) or a cross-compiled libusb vendored into the build. The
UI layer itself is the least of the work — eframe has upstream Android
support and the GUI's existing transport abstraction means no GUI-side
refactor is needed.

## Scope

This document evaluates what an Android build of `dmm-gui` would require.
It covers the UI toolkit, the USB access path on Android, Rust USB crate
options, the implications of rooting the target device, and the build
toolchain. It does not cover alternative delivery approaches (remote
viewer over network, WebAssembly/browser build) nor UX redesign for small
screens.

Claims below that were confirmed against source or authoritative upstream
material are stated plainly. Claims drawn from external reports are
attributed inline. Claims that remain untested are called out as such.

## UI: eframe on Android

eframe gained upstream Android support in [egui PR
#5318](https://github.com/emilk/egui/pull/5318), released December 2024.
Support is delivered via `winit`'s `android-activity` backend: an
application receives an `AndroidApp` in its `NativeOptions` and exposes a
`#[no_mangle] fn android_main(app: AndroidApp)` instead of a conventional
`main`. Working minimal examples exist in [`rib/android-activity`'s
`agdk-egui`](https://github.com/rib/android-activity/tree/main/examples/agdk-egui)
and [`inferrna/hello_world_android_egui`](https://github.com/inferrna/hello_world_android_egui).

Reported caveats from the PR discussion and example projects:

- Window lifecycle (pause/resume) requires dropping and rebuilding the
  graphics context; mishandling results in a `winit` panic.
- Soft-keyboard input works; full IME behaviour is minimal.
- Status-bar and navigation-bar insets are not handled by default.

`dmm-gui` uses `eframe = "0.34"` (`crates/dmm-gui/Cargo.toml:11`).
Compatibility of the examples above against this specific version has not
been tested.

## GUI coupling to the desktop platform

Inspection of the GUI crate on 2026-04-19 confirms that the UI never calls
`hidapi` directly. It opens the meter through `dmm_lib::open_device_by_id_auto`
and communicates with the background device thread via `mpsc` channels
defined in `crates/dmm-gui/src/connection.rs`. The Transport abstraction in
`dmm-lib` is therefore the substitution point for an Android-specific USB
backend; no GUI refactor is required to accommodate it.

Desktop-specific call sites that would need to be conditionally excluded on
Android:

- `install_desktop_integration()` at `crates/dmm-gui/src/main.rs:191-252`
  (already gated to `target_os = "linux"`, no change needed).
- `rfd` file dialog at `crates/dmm-gui/src/app/mod.rs:1491` (CSV export).
- `directories` crate used to resolve the settings path (`dmm-settings`
  would need an Android branch).
- The `eframe::run_native()` entry at `crates/dmm-gui/src/main.rs:254`
  (replaced by `android_main`).

## USB access model on Android

Android does not expose `/dev/hidraw*` to applications under its default
security model. USB access goes through `android.hardware.usb.UsbManager`,
which issues a runtime permission dialog and returns a `UsbDeviceConnection`
from which native code can obtain a raw file descriptor via
`getFileDescriptor()`. `libusb` supports initialisation from a
pre-opened descriptor via `libusb_wrap_sys_device()` combined with
`LIBUSB_OPTION_NO_DEVICE_DISCOVERY`; that pattern is how most
Rust-on-Android USB code handles the Android security model.

Kernel-driver binding is device-specific:

- The Silicon Labs CP2110 (VID `0x10C4`, PID `0xEA80`) is a vendor-specific
  HID device and is not normally claimed by Android's built-in HID drivers.
- CH9329 and CH9325 can enumerate as standard keyboard/mouse HID devices;
  whether a given Android kernel binds them is device- and firmware-dependent
  and has not been tested for this investigation.

On Android 9+, `UsbHidDevice` provides HID *peripheral-mode* APIs (the phone
acts as a HID device to another host). This is unrelated to consuming HID
devices and is not applicable here.

## Rust USB crate landscape

### `hidapi` (v2.6)

`hidapi = "2.6"` does not build for `aarch64-linux-android`. The crate's
`build.rs` dispatches on `target.contains("linux")` at line 31; the
Android triple matches, so all Linux backends are reached. Each fails:

- `linux-static-hidraw` / `linux-shared-hidraw` call
  `pkg_config::probe_library("libudev")`. `libudev` is not available in
  the Android NDK sysroot.
- `linux-static-libusb` / `linux-shared-libusb` require a
  `libusb-1.0` accessible to `pkg-config`, which the NDK does not
  provide.
- `linux-native` and `linux-native-basic-udev` pull in `udev`/`basic-udev`
  crates that also wrap `libudev` and use netlink/sysfs paths that differ
  on Android.

[Issue #122](https://github.com/ruabmbua/hidapi-rs/issues/122) is the
canonical upstream thread; it remained open as of 2026-01 with no
feature flag, fork, or merged solution. One reporter confirmed that a
hand-cross-compiled libusb fed into the `linux-static-libusb` path
worked, while describing the build process as "horrible". No published
Rust-on-Android project using `hidapi` is known.

### `nusb`

[`nusb`](https://crates.io/crates/nusb) is a pure-Rust USB crate that
talks to usbfs (`/dev/bus/usb/*`) directly via ioctls, with no libusb or C
dependencies. Issue #122's most recent comment (2026-01-03) recommends it
as the replacement for hidapi on Android. It has no HID-class helpers, so
consumers must implement HID report framing themselves.

For this project that cost is small: CP2110's UART-over-HID layer in
`crates/dmm-lib/src/cp2110.rs` is approximately 150 lines of
interrupt-report chunking plus `SET_REPORT`/`GET_REPORT` feature handling.
The translation to `nusb`'s interrupt-transfer and control-transfer
primitives is mechanical. Two structural choices are available:

- Duplicate the framing into an Android-only module (no refactor of
  existing code; some duplication).
- Extract a small `HidIo` trait that `Cp2110` consumes, with implementations
  backed by `hidapi::HidDevice` on non-Android targets and `nusb` on
  Android (no duplication; small refactor touching the desktop build).

`nusb` on `aarch64-linux-android` has not been built or tested as part of
this investigation.

### `rusb` / vendored libusb

`rusb` is the standard Rust wrapper over libusb and exposes
`wrap_sys_device`, which is the pattern used with Java-`UsbManager`-issued
file descriptors. It has the same build requirement as hidapi's libusb
backend — a cross-compiled `libusb-1.0` in the NDK sysroot. Including
libusb also pulls in LGPL-2.1 licensing, a consideration for any
redistributable APK.

## Rooted-device variant

On rooted devices the `UsbManager` permission flow can be bypassed:

- `/dev/bus/usb/*` becomes reachable after `chmod 666` issued via `su`, or
  by launching the application process under `su`.
- `libusb_detach_kernel_driver()` is unrestricted, which may unblock
  CH9329/CH9325 if their interfaces are bound by kernel HID drivers.
- No Kotlin/JNI shim, no USB permission dialog, no `device_filter.xml` is
  required.

SELinux remains relevant. The default domain of an Android application
typically denies `/dev/bus/usb/*` access even with root shell available;
Magisk-installed applications usually inherit a permissive-enough context,
and a narrow `magiskpolicy` rule covers cases where they do not. The
specific behaviour on /e/OS under Magisk has not been tested.

Rooting does not address the `hidapi` build failure. The choice of Rust
USB stack (`nusb` versus vendored libusb) is independent of whether the
target device is rooted.

## Build toolchain

- `cargo-apk` and `cargo-ndk` are not installed by default on Debian.
  `cargo-apk` is the less friction-y of the two for a first-pass build;
  `cargo-ndk` with a hand-written Gradle project is the fallback when
  `cargo-apk` trips on NDK layout.
- Debian packages installers for NDK releases up to at least r28c under
  the name `google-android-ndk-rXX-installer`. These are thin wrappers
  that download the actual NDK on package installation.
- The Android SDK, `adb`, `apksigner`, and `aapt` are available as
  Debian packages and are sufficient for APK signing and installation.
- `rustup` shares `~/.cargo/bin` shims with Debian's packaged `cargo`,
  which produces a PATH conflict on systems that use the distro Rust.
  The conflict is avoidable by installing rustup with `--no-modify-path`
  and invoking it via its absolute path, or by removing the distro Rust
  packages in favour of rustup.
- The `aarch64-linux-android` target requires rustup; it is not available
  through Debian's `rustc` packaging.

## Option comparison

| Option                                            | Rust-side changes              | Build complexity                    | Runtime prerequisites         |
|---------------------------------------------------|--------------------------------|-------------------------------------|-------------------------------|
| A. `nusb`, Android-only CP2110 duplicate, rooted  | Small, self-contained          | Low (pure Rust)                     | Root / Magisk                 |
| B. `nusb`, `HidIo` trait abstraction, rooted      | Moderate; touches `cp2110.rs`  | Low (pure Rust)                     | Root / Magisk                 |
| C. JNI to `UsbManager` + `rusb::wrap_sys_device`  | Moderate                       | Medium (Kotlin + JNI)               | None                          |
| D. Vendored cross-compiled libusb + `hidapi`      | None                           | High (NDK libusb cross-build)       | Root or JNI glue still needed |

Option A is the lowest-friction path for a proof of concept on hardware
that is already rooted. Option B is the equivalent with less duplication
and is the natural foundation if CH9329/CH9325 follow. Option C is the
shape a general-consumer port would take. Option D is not recommended;
Issue #122 suggests the build effort is substantial and LGPL-2.1 licensing
must be accommodated.

## Open questions

- Whether `eframe = "0.34"` works against the current `android-activity`
  example code without version adjustment.
- Whether `nusb` builds cleanly for `aarch64-linux-android`.
- Whether `/dev/bus/usb/*` is reachable from a `su`-launched process on
  /e/OS under its default SELinux policy, or whether a `magiskpolicy` rule
  is required.
- Whether CH9329 and CH9325 are claimed by Android kernel HID drivers in
  practice.
- CP2110 endpoint addresses and interface indices required for raw
  USB operation — these need to be read from the device descriptor and
  are not recorded in `dmm-lib`.

## References

- [egui PR #5318 — Android support (2024-12)](https://github.com/emilk/egui/pull/5318)
- [hidapi-rs issue #122 — Android support discussion](https://github.com/ruabmbua/hidapi-rs/issues/122)
- [`android-activity` `agdk-egui` example](https://github.com/rib/android-activity/tree/main/examples/agdk-egui)
- [`hello_world_android_egui`](https://github.com/inferrna/hello_world_android_egui)
- [`nusb` crate](https://crates.io/crates/nusb)
- [`tauri-plugin-hid` — reference implementation of the `UsbManager` + JNI pattern](https://crates.io/crates/tauri-plugin-hid)
- [libusb `libusb_wrap_sys_device` documentation](https://libusb.sourceforge.io/api-1.0/group__libusb__dev.html#ga98f0b3a3685d4a1f3056b0e3e53d7d60)
