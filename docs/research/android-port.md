# Android port feasibility

*Last updated 2026-11-17. Options A and B below were built as an experiment
and run on two phones: "Experimental build" has what they showed. The rooted
variant remains untested.*

## Summary

An Android port of `dmm-gui` is technically feasible. Beyond the usual
cross-compilation concerns it faces three obstacles:

1. Android's USB security model does not expose `/dev/hidraw*` or
   `/dev/bus/usb/*` to regular applications. Devices must be opened through
   the Java `UsbManager` API, which returns a file descriptor to native
   code.
2. The `hidapi` crate that `dmm-lib` depends on does not build for
   `aarch64-linux-android` in any of its supported configurations. The
   experimental build leaves it out there: `transport/hid_android.rs` stands
   in for it, over `nusb`.
3. Bluetooth LE on Android is reachable only through the Java Bluetooth
   API. Every Rust route found ships Java classes in the APK and needs
   runtime permissions requested from an Activity.

Rooting the device removes obstacle (1). It removes neither (2) nor (3).

Bluetooth matters because of which meters it reaches. ZOTEK's meters,
OWON's and the Voltcraft and UNI-T (UT217A, UT217B) models OWON makes, the
EEVblog 121GW, the Brymen BM78xBT, the UT60BT and the UT202BT have no
other link. The UT61+, UT161, UT171 and UT181A series are on the radio
too, through the UT-D07B adapter. The device tables list a cable as the
only link for the UT8802 and UT8803, the VC-880, VC650BT and VC-890, the
CH9325 meters (UT803, UT804, UT71, VC920 and their relatives; the UT71
has an untested Bluetooth route through the UT-D07A) and the Brymen series
on the BU-86X cable.

That leaves two ends to start from:

- A Bluetooth-only build reaches the most meters and needs no root. This
  is the one that was built first; `btleplug` under a `NativeActivity` was
  its largest unknown, and it works. The cables were added to it next,
  through `UsbManager`, still without root.
- A rooted, USB-only build on `nusb` has the fewest unknowns and needs no
  Java. It reaches the cabled meters only, and the Java glue returns as
  soon as Bluetooth is wanted.

The UI work is the same for both. eframe has upstream Android support, and
the GUI reaches the meter only through `dmm-lib`'s transport abstraction.

## Experimental build

Option B of the comparison below, which is option A with the cables
added, is in the repository: the `crates/dmm-android` library,
the `android/` Gradle project and `scripts/android-build.sh` ("Building for
Android" in `docs/development.md`). It was run on one phone, Android 12
(API 32), arm64, with a UT61E+ on the UT-D07B adapter and on its CP2110
cable, on 2026-11-16. The same package was then installed on a second
phone, Android 14 (API 34), arm64, whose screen has square corners: it
connected over a cable, opened by the system's offer at plug-in, and over
Bluetooth. What follows is the first phone's unless it names the second.

CI builds the package on every push, and dev builds publish it signed with
the project's key. The first of those, of 2026-11-17, was installed on the
Android 12 phone from its release page and ran.

What worked:

- The GUI draws through eframe's `android-native-activity` and glow and
  follows the screen into landscape. Taps, drags and pinch zoom work on
  the graph, and the Settings panel is usable; the controls keep their
  desktop size, which is small for a finger. It survives being sent to the
  background and brought back.
- `btleplug::platform::init`, called from `android_main`, and the
  transport's blocking calls on the acquisition thread: scan, connect,
  profile choice, detection, the readings stream and its pairs notice.
- The permission request, made by a small Java activity before the native
  one starts.
- Settings, kept in the application's own folder.
- The language picked for the application in Android's settings, on the
  second phone: the package lists its languages (Android 13 and later
  offer them), and the activity's Java side reads the one picked, which
  the system's locale property does not hold. It applies at the next
  start.
- Export, which writes unasked to a `dmm-tools` folder in the shared
  Documents; Import, through the system's document picker, shown by the
  activity's Java side; links, opened in the browser.
- Reconnecting after the adapter was switched off, for a few seconds and
  for a minute.
- The CP2110 cable. The activity's Java side lists the USB devices, asks
  the permission and opens the cable; the library drives the descriptor it
  is handed with `nusb`, behind the calls the bridges make of hidapi
  (`transport/hid_android.rs`), so the bridges are the desktop's. The
  session reads the meter at the desktop's pace and comes back by itself
  after the cable was pulled and plugged in again.

What it showed about Android:

- Scan limit. A lost session retries about every five seconds, six scans
  in thirty. Every sixth scan heard nothing at all, so one retry in six is
  wasted; the session still came back once the adapter did.
- With the radio switched off, `btleplug` reports no adapter state and its
  scan fails as "no adapter"; the transport reports that as Bluetooth
  turned off.
- Switching the radio off under a live session leaves `btleplug`'s
  peripheral marked connected, and a service discovery on it is never
  answered. An open on Android therefore forgets the peers of earlier
  sessions and scans, and discovery and subscribe have a time limit on
  every platform. The session itself ends on a failed write, not a lost
  link, so it does not reconnect by itself.
- The first link after the adapter was powered back on carried no data
  once; the session dropped it and the next one worked. Not explained, and
  not compared with a desktop.
- winit builds one event loop per process and does not end it when the
  activity is destroyed. The activity is single-task, so opening the app
  again returns to it, and it ends the process when it finishes. Back
  sends the root activity to the background on Android 12, so that last
  path ran on the second phone only: swiped away from the recent
  applications, the process ended. It ends before the native side hears
  of it, so a recording not exported goes without the question a desktop
  asks before closing.
- The status bar covered the top bar (egui #7748); the activity is full
  screen.
- The kernel binds the CP2110 to `usbhid`, and `/dev/hidraw*` is root's
  alone. `nusb`'s disconnect-and-claim request is accepted on a
  `UsbManager` descriptor, and a duplicate of the descriptor outlives the
  Java connection it came from.
- A USB permission asked for at run time lasts until the cable is
  unplugged. The manifest names the four cables, so Android offers the
  application when one is plugged in; taken for good, the offer opens the
  application at each plug-in with the permission given and no dialog.
  A running application leaves a cable just plugged in to that offer, as
  its own request would put a second dialog over it, and connects when the
  system hands the cable over. The system says nothing of an offer turned
  down; the application takes it from the focus coming back with the cable
  still not allowed.
  On the Android version tried, the system's page for the application
  takes that back: "Clear default preferences" under "Open by default",
  shown only while there is one.
  Android keeps a cable's serial number from an application that has no
  permission yet, so a session on Android is not tied to its cable's serial
  number.
- Full screen, the window reaches into the screen's rounded corners. The
  build asks the system for their radius and keeps a margin clear of them,
  but this phone's system reported square corners; with the radius given
  by hand the margin held in portrait and landscape.

Not done or not tested: the CH9329, CH9325 and BU-86X cables, a meter that
sends unasked on a cable, two cables at once, choosing where an export
goes, naming a saved theme, the soft keyboard, the system clipboard, a
layout for a phone screen, Android versions other than 12 and 14, other
meters. The texts are the desktop's but for three errors that name the
phone, its cable prompt or its radio, so help and hovers still speak of a
computer.

## Scope

This document evaluates what an Android build of `dmm-gui` would require.
It covers the UI toolkit, the USB and Bluetooth LE access paths on Android,
Rust crate options for each, the implications of rooting the target device,
the Android versions in reach and the build toolchain. It does not cover
alternative delivery approaches (remote viewer over network,
WebAssembly/browser build) nor UX redesign for small screens.

Claims confirmed against this repository's source or authoritative upstream
material are stated plainly. Claims drawn from external reports are
attributed inline. Claims deduced from source without running anything are
marked as derived. Claims that remain untested are called out as such.

## UI: eframe on Android

eframe gained upstream Android support in [egui PR
#5318](https://github.com/emilk/egui/pull/5318), released in eframe 0.30
(December 2024). Support is delivered via `winit`'s `android-activity`
backend: an application receives an `AndroidApp`, passes it in
`NativeOptions::android_app`, and exposes a
`#[unsafe(no_mangle)] fn android_main(app: AndroidApp)` instead of a
conventional `main`. eframe selects the activity type through one of two
features, `android-native-activity` or `android-game-activity`. The egui
repository carries a working example,
[`examples/hello_android`](https://github.com/emilk/egui/tree/main/examples/hello_android),
which uses the `glow` renderer with `android-native-activity`.

`dmm-gui` uses eframe 0.37 with the `glow` and `wgpu` renderers
(`crates/dmm-gui/Cargo.toml`) and enables neither Android feature. It is a
binary crate whose `main` parses a `clap` command line before opening the
window; an Android build needs a `cdylib` target exposing `android_main`.
The experimental build does this: `dmm-gui` is a library with the desktop
`main` beside it, and `crates/dmm-android` exposes `android_main`.

Caveats, from eframe's source and egui's issue tracker:

- Suspend and resume: eframe drops the window surface on suspend and
  recreates it on resume. An application built on `winit` directly must do
  this itself.
- Soft keyboard: reported not to open on `NativeActivity` ([egui
  #5590](https://github.com/emilk/egui/issues/5590)); the workaround given
  there is a JNI call to `showSoftInput`. `dmm-gui` has typed fields, among
  them alarm limits, marker notes and the graph's fields.
- Status-bar and navigation-bar insets are not reported on Android ([egui
  #7748](https://github.com/emilk/egui/issues/7748)); eframe's safe-area
  support covers iOS only.
- Rendering: `wgpu` on the Vulkan drivers of older phones is untested.
  `dmm-gui` falls back to `glow` when `wgpu` fails to start.

## GUI coupling to the desktop platform

`dmm-gui`, `dmm-shared` and `dmm-cli` depend on neither `hidapi`,
`btleplug` nor `tokio`. The GUI opens the meter through
`dmm_lib::open_device()` or `open_auto()`, and talks to its acquisition
thread over `mpsc` channels whose message types are defined in
`crates/dmm-gui/src/app/connection.rs`. The USB cable listing,
`list_devices()`, reaches it through `dmm-shared`. The `Transport`
trait in `crates/dmm-lib/src/transport/mod.rs` is therefore the
substitution point for an Android-specific backend; no GUI refactor is
required to accommodate one.

One hidapi type does cross the library boundary: `dmm_lib::Error::Hid`
wraps `hidapi::HidError`, and `dmm-shared` (`l10n/errors.rs`) and `dmm-cli`
(`open.rs`) match on it. Building `dmm-lib` without hidapi changes that
public error type.

Desktop-specific pieces an Android build would need to replace or leave
out:

- `rfd` file dialogs in `crates/dmm-gui/src/app/export.rs`,
  `app/import.rs` and `app/controls/theme_save.rs`. The `rfd` version in
  use has no Android backend; `file_dialog.rs` stands in for it there.
- The `directories` crate: `config_path()` in `dmm-shared` resolves the
  settings path, and the user themes folder and the update-check cache
  follow it; `app/import.rs` uses it for the default folder. An Android
  application has no home directory for it to start from, so the
  experimental build passes the application's own folder
  (`dmm_shared::set_config_dir`).
- The update check: `ureq` with `rustls` and the platform certificate
  verifier. On Android that verifier needs a Kotlin component which Gradle
  fetches from the verifier project's repository, the Kotlin standard
  library, and a network security configuration allowing clear-text
  requests to certificate authorities' revocation hosts. The experimental
  build carries Mozilla's CA list (`webpki-roots`) there instead; the check
  is the reason for its `INTERNET` permission.
  On the Android 12 phone the check found a newer dev build, and its
  notice opened the release page in the browser.
- Logging: `env_logger` writes to stderr (`dmm-shared`'s `logging`);
  Android applications log to logcat.
- The command line, parsed from `std::env::args_os()` in
  `crates/dmm-gui/src/lib.rs`, and the `eframe::run_native()` calls beside
  it (replaced by `android_main`).
- Window size, maximise and decoration commands, and the system-font
  lookup through `fontique`; both untested on Android.

`install_desktop_integration()` (Linux) and `free_own_console()` (Windows)
are already excluded, since Android's `target_os` is `android`. Locale
files, built-in themes, the changelog and the icons are compiled into the
binary and need no change.

## USB access model on Android

Android does not expose `/dev/hidraw*` to applications under its default
security model. USB access goes through `android.hardware.usb.UsbManager`,
which issues a runtime permission dialog and returns a
[`UsbDeviceConnection`](https://developer.android.com/reference/android/hardware/usb/UsbDeviceConnection)
from which native code can obtain a raw file descriptor via
`getFileDescriptor()`. `libusb` supports initialisation from a
pre-opened descriptor via `libusb_wrap_sys_device()` combined with
`LIBUSB_OPTION_NO_DEVICE_DISCOVERY`; `nusb` offers the same hand-off as
`Device::from_fd`.

Kernel-driver binding, derived from the Linux kernel source and seen on a
phone for the CP2110 alone: `usbhid` matches every interface of the HID class, and
`hid-generic` takes any HID device no specialised driver claims.
`hid-quirks.c` lists none of the four bridges (CP2110, CH9329, CH9325,
BU-86X) among the devices it ignores. A kernel built with USB HID support
should therefore bind all four, and an application must take the interface
from the kernel: `claimInterface(intf, true)` on the Java side, or a detach
under root.

## Bluetooth LE on Android

`dmm-lib` reaches Bluetooth meters through `btleplug`, behind the default
`bluetooth` feature (`crates/dmm-lib/src/transport/ble/`). The transport
owns a current-thread `tokio` runtime and runs each `btleplug` call to
completion on the caller's thread. It has five GATT profiles, each with
one characteristic to write and one to take notifications or indications
from (the same one in two of them); two also read a characteristic while
bringing the link up. It never pairs or bonds. Its Windows and Linux
extras have fallbacks on every other platform.

`btleplug` lists Android as supported in its
[README](https://github.com/deviceplug/btleplug). From the README, its
build script and issue #418:

- It is a hybrid of Rust and Java. The Java classes build with Gradle
  into a library (an AAR) that the application bundles into its APK.
- `btleplug::platform::init` must be called from a thread whose context
  class loader sees the application's classes. A thread attached from
  native code must set that first.
- It requires Android 7.0 (API 24).
- It cannot query the adapter's state. `central()` in `ble/mod.rs` already
  accepts an unknown state.

No other project combining `btleplug` with `android-activity`, `winit` or
egui was found (btleplug README and issues, web search, 2026-11-16). btleplug
issues [#418](https://github.com/deviceplug/btleplug/issues/418) (from a
Slint application without a Gradle build) and
[#501](https://github.com/deviceplug/btleplug/issues/501) ask for setup
steps or a minimal Android example and hold none; the maintainer's advice
in #418 is a Gradle project with the AAR in its `libs` folder. The
experimental build compiles btleplug's Java sources from the locked crate
into the application instead, and `init` and the blocking calls work from
`dmm-gui`'s acquisition thread under `NativeActivity`.

Two other crates can do without a Gradle build.
[`android-ble`](https://github.com/uglyoldbob/android-ble-rs) embeds its
compiled classes in the Rust library and loads them at run time.
[`blec`](https://github.com/MnlPhlp/tauri-plugin-blec) does the same
behind an optional feature; its default is a Gradle module, and its README
notes that a device may block code loaded at run time. Every route found
ships Java or Kotlin bytecode in some form (crates.io and GitHub,
2026-11-16).

Platform requirements, from Android's [Bluetooth permissions
page](https://developer.android.com/develop/connectivity/bluetooth/bt-permissions):

- An application targeting API 31 or later, on Android 12 and later: the
  `BLUETOOTH_SCAN` and `BLUETOOTH_CONNECT` runtime permissions.
- On Android 11 (API 30) and earlier: `BLUETOOTH`, `BLUETOOTH_ADMIN` and
  the `ACCESS_FINE_LOCATION` runtime permission.
- Runtime permissions are requested through an `Activity` method.
  `NativeActivity` passes no permission result to native code, so the
  crates above either request through JNI and check again on resume, or
  start a small Java activity for the request.

Two behaviours of the transport meet Android limits ("Experimental build"
has what the phone showed of the first; the second is untested):

- Scanning. `search()` in `ble/search.rs` scans only when the platform's
  list of known peripherals does not settle which peer to open, for up to
  three seconds. A lost session reopens on its address and retries every
  two seconds. Once an application has started five scans within thirty
  seconds, Android does not start its next one, and reports neither an
  error nor results
  ([AOSP](https://android.googlesource.com/platform/packages/apps/Bluetooth/+/1fdc7c1)).
  The `btleplug` version in use has no call on Android for the peripherals
  the system knows, so the known list is the adapter's own list for this
  process, which an open on Android now clears: every open scans, and the
  retry loop runs at the limit.
- Opening an address that neither the known list nor the scan holds
  exists on Windows only (`by_address` in `ble/search.rs`); elsewhere it
  is reported as not found. `btleplug`'s README lists adding a peripheral
  by address, the call `by_address` uses, as implemented on Android.

Root does not open a second route. Derived from [AOSP's Bluetooth
architecture](https://source.android.com/docs/core/connect/bluetooth) and
the notes of [Bumble's Android Remote
HCI](https://google.github.io/bumble/extras/android_remote_hci.html):
applications reach the stack through Binder only, and taking the controller
directly means shutting the system's Bluetooth stack down first. No raw HCI
path usable beside the system stack was found (web search, 2026-11-16).

## Android versions

- The floor is Android 7.0 (API 24), set by `btleplug`'s Android backend.
  egui's `hello_android` example declares API 23, and USB host support is
  older than both.
- Android 12 (API 31) is where the Bluetooth permissions change. Covering
  both sides takes the manifest entries for each and one runtime request.
- Google Play's target-API requirements are rules for Play submissions.
- A test should include a phone on Android 12 or later for the current
  permission model, and one on Android 14 or later, where the first MTU
  request negotiates 517 bytes whatever value the application passes.

## Rust USB crate landscape

### `hidapi` (v2.6)

`hidapi = "2.6"` does not build for `aarch64-linux-android`. On Android
`dmm-lib` takes its `cfg(not(target_os = "linux"))` dependency line, which
enables hidapi's default features. The crate's `build.rs` dispatches on
`target.contains("linux")`; the Android triple matches, so the Linux
backends are reached. Each fails:

- `linux-static-hidraw` probes `libudev` through `pkg-config`, and
  `linux-shared-hidraw` probes a system `hidapi-hidraw`. Neither is in the
  Android NDK sysroot.
- `linux-static-libusb` / `linux-shared-libusb` require a
  `libusb-1.0` accessible to `pkg-config` (the shared one a system
  `hidapi-libusb` too), which the NDK does not provide.
- `linux-native` and `linux-native-basic-udev` select nothing on Android:
  hidapi gates their dependencies and the backend module itself on
  `target_os = "linux"`.

[Issue #122](https://github.com/ruabmbua/hidapi-rs/issues/122), "Native
linux backend based on usbfs", is the upstream thread closest to Android.
Its author, whose use case was Android, reports getting the libusb backend
to work while describing the build process as "horrible". A later comment
suggests building such a backend on `nusb`. No Android support has been
merged, and no published Rust-on-Android project using `hidapi` was found
(hidapi-rs repository and issues, web search, 2026-11-16).

With its libusb features, hidapi-rs exposes `HidApi::wrap_sys_device()`
and `HidApi::disable_device_discovery()`, the pair needed to open a device
from a `UsbManager` file descriptor.

### `nusb`

[`nusb`](https://crates.io/crates/nusb) is a pure-Rust USB crate that
talks to usbfs (`/dev/bus/usb/*`) directly via ioctls, with no libusb or C
dependencies. On Android:

- `Device::from_fd` wraps a file descriptor from `UsbDeviceConnection`.
  The [pull request](https://github.com/kevinmehall/nusb/pull/80) that
  added it, titled for non-rooted phones, reports a test on a device
  running Android 14.
- Device listing is compiled out
  ([issue #165](https://github.com/kevinmehall/nusb/issues/165)). The
  application finds the device on the Java side, or under root opens the
  usbfs node itself.
- `detach_kernel_driver` does nothing outside `target_os = "linux"`.
  `detach_and_claim_interface` is compiled for Android and issues the
  usbfs disconnect-and-claim request, which the experimental build uses.
  `claimInterface(intf, true)` on the Java side is the documented
  alternative.

It has no HID-class helpers, so consumers must implement HID report framing
themselves. For this project that cost is small: each bridge in
`crates/dmm-lib/src/transport/` (`cp2110.rs`, `ch9329.rs`, `ch9325.rs`,
`bu86x.rs`) is one file of interrupt-report framing plus, for some,
`SET_REPORT`/`GET_REPORT` feature handling. The translation to `nusb`'s
interrupt-transfer and control-transfer primitives is mechanical. The
experimental build does it once for all four: a module with hidapi's calls
and hidraw's bytes over `nusb`, which the bridges use unchanged.

`nusb` 0.2.9 builds for `aarch64-linux-android` in this workspace with no
C code, on crates the workspace already built.

### `rusb` / vendored libusb

`rusb` is the standard Rust wrapper over libusb and exposes
`open_device_with_fd`, which wraps `libusb_wrap_sys_device()`. Its
`libusb1-sys` crate has a `vendored` feature that compiles libusb for the
target, Android included, so the cross-compiled `libusb-1.0` that hidapi's
libusb backend needs is not a requirement here. Including libusb brings
LGPL-2.1-or-later licensing, a consideration for any redistributable APK.
`nusb`'s `from_fd` covers the same hand-off without the C build or the
licence, which is why the comparison below carries `nusb`.

## Rooted-device variant

On rooted devices the `UsbManager` permission flow can be bypassed. This
applies to USB only.

- libusb's Android notes give two ways to reach `/dev/bus/usb/*`: run the
  code under `su`, or change the nodes' permissions through `su`.
- Detaching the kernel's HID driver from the interface is unrestricted.
- No Kotlin/JNI shim, no USB permission dialog, no `device_filter.xml` is
  required.

SELinux remains relevant. AOSP's policy gives application domains read,
write and ioctl on USB device nodes but not open (`private/app.te`), so a
change of permissions alone is not expected to let the application open
one. Processes started through Magisk's `su` run in its unconstrained
`magisk` domain (Magisk documentation); the application's own process
does not. Derived from those two sources: a helper started through `su`
opens the node and passes the descriptor to the application, or a
`magiskpolicy` rule adds the open permission. Neither has been tested.

Rooting does not address the `hidapi` build failure, and `nusb` still
cannot list devices.

## Build toolchain

- Java classes are needed for Bluetooth, and for USB on an unrooted phone,
  and that decides the build tool. `cargo-ndk` with a Gradle application
  module is what the
  [`android-activity`](https://github.com/rust-mobile/android-activity)
  README and btleplug's maintainer point to, and the only route found that
  consumes btleplug's AAR. `cargo-apk`, which egui's example uses in a
  patched build, documents no way to add Java classes.
  [`cargo-apk2`](https://github.com/mzdk100/cargo-apk2) compiles Java
  sources into the APK; no AAR support was found in it (tool READMEs and
  btleplug issues, 2026-11-16).
- None of the three is installed by default on Debian.
- Debian packages installers for NDK releases under the name
  `google-android-ndk-rXX-installer`. These are thin wrappers that
  download the actual NDK on package installation.
- The Android SDK, `adb`, `apksigner`, and `aapt` are available as
  Debian packages and are sufficient for APK signing and installation.
- rustup's installer stops to ask when it finds the distro `rustc` and
  `cargo` on `PATH`, and which `cargo` runs afterwards depends on `PATH`
  order; Debian's own `rustup` package conflicts with its `cargo` and
  `rustc` packages. Either install rustup with `--no-modify-path` and
  invoke it via its absolute path, or replace the distro Rust packages
  with rustup.
- The `aarch64-linux-android` target requires rustup; it is not available
  through Debian's `rustc` packaging.

## Option comparison

Every option needs the eframe work above and a `dmm-lib` that builds
without `hidapi`. The options are not exclusive: B is A with USB added.

| Option                                                     | Meters reached       | Rust-side changes                                   | Build                    | Runtime prerequisites   |
|------------------------------------------------------------|----------------------|-----------------------------------------------------|--------------------------|-------------------------|
| A. Bluetooth only, `btleplug`'s Android backend            | Bluetooth meters     | `btleplug` initialisation, permission request (JNI) | Gradle + bundled AAR     | Bluetooth permissions   |
| B. A plus USB: `UsbManager` descriptor, `nusb`             | All                  | A, plus bridge framing on `nusb` and a Java shim    | Gradle + bundled AAR     | Bluetooth, USB dialogs  |
| C. USB only, rooted, `nusb` on usbfs                       | Cabled meters        | Bridge framing on `nusb`                            | Pure Rust                | Root / Magisk           |
| D. USB only, `hidapi` on libusb with `wrap_sys_device`     | Cabled meters        | Descriptor hand-off; bridges unchanged              | NDK libusb cross-build   | USB dialog, or root     |

Option A is the suggested first experiment: it reaches the most meters on
an unrooted phone, and it settles the largest unknown, `btleplug` under
`NativeActivity`. Option B is the shape a full port would take. Option C
has the fewest unknowns, each of its parts having an upstream test report,
and is the shortest path for cabled meters on a rooted phone. Option D is
not recommended; Issue #122 suggests the build effort is substantial and
LGPL licensing must be accommodated.

Options A to C add `jni`, `nusb` or both as direct dependencies. In the
experimental build `jni` stays in `crates/dmm-android`, and `nusb` joins
`dmm-lib`'s dependency list for Android alone, where `hidapi` is absent.

## Open questions

- Whether opening a lost peer by its address, which `btleplug` implements
  on Android, spares the reconnect loop the scan Android drops one time in
  six.
- Why the first link after an adapter's power-up carried no data.
- Whether the application comes back with its settings after the exit on
  finish, and what a phone layout needs.
- How to end the process without dropping a recording that was not
  exported: on Android 11 and older, Back finishes the activity.
- Whether a meter that sends unasked on a cable comes back from a pause
  with old readings. The input queue holds 64 reports and then holds the
  device back, where a desktop's HID driver keeps reading; what the bridge
  kept meanwhile would arrive after the queue was emptied.
- Keeping the system's status bar on screen as an option: the margin that
  clears the rounded corners could carry the bar's height too, read from
  the window's insets by the activity.
- Whether the CH9329, CH9325 and BU-86X work as the CP2110 does. Their
  HID interface and endpoints are read from the descriptors when the cable
  opens; a cable with no interrupt OUT endpoint takes its reports on the
  control endpoint, a path the CP2110 does not use.
- Whether `/dev/bus/usb/*` is reachable from a `su`-launched process on a
  custom ROM under its default SELinux policy, or whether a `magiskpolicy`
  rule is required.

## References

- [egui PR #5318 — Android support (2024-12)](https://github.com/emilk/egui/pull/5318)
- [egui `hello_android` example](https://github.com/emilk/egui/tree/main/examples/hello_android)
- [egui issue #5590 — soft keyboard on Android](https://github.com/emilk/egui/issues/5590)
- [egui issue #7748 — safe area on Android](https://github.com/emilk/egui/issues/7748)
- [`android-activity`](https://github.com/rust-mobile/android-activity)
- [`btleplug`](https://github.com/deviceplug/btleplug), issues
  [#418](https://github.com/deviceplug/btleplug/issues/418) and
  [#501](https://github.com/deviceplug/btleplug/issues/501)
- [`blec`](https://github.com/MnlPhlp/tauri-plugin-blec) and
  [`android-ble`](https://github.com/uglyoldbob/android-ble-rs)
- [Android Bluetooth permissions](https://developer.android.com/develop/connectivity/bluetooth/bt-permissions)
- [AOSP Bluetooth architecture](https://source.android.com/docs/core/connect/bluetooth)
- [Bumble — Android Remote HCI](https://google.github.io/bumble/extras/android_remote_hci.html)
- [Android `UsbDeviceConnection`](https://developer.android.com/reference/android/hardware/usb/UsbDeviceConnection)
- [hidapi-rs issue #122 — usbfs backend, Android use case](https://github.com/ruabmbua/hidapi-rs/issues/122)
- [`nusb` crate](https://crates.io/crates/nusb), [PR #80](https://github.com/kevinmehall/nusb/pull/80)
  and [issue #165](https://github.com/kevinmehall/nusb/issues/165)
- [`tauri-plugin-hid`](https://crates.io/crates/tauri-plugin-hid) — a
  Kotlin `UsbManager` backend with `claimInterface` and `bulkTransfer`
  calls on the interface's endpoints; no feature reports and no descriptor
  hand-off, so a partial reference for these bridges
- [`cargo-apk2`](https://github.com/mzdk100/cargo-apk2)
- [libusb `libusb_wrap_sys_device` documentation](https://libusb.sourceforge.io/api-1.0/group__libusb__dev.html#ga98f783e115ceff4eaf88a60e6439563c)
