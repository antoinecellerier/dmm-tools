//! The Android build of dmm-gui, experimental and Bluetooth only: the
//! shared library the activity in `android/` loads. What a desktop `main`
//! would find around it is set up here — where logs go, where settings
//! live, btleplug's Java half — and then the window runs as it does
//! everywhere (`dmm_gui::run_android`).
//!
//! System properties stand in for a command line and the environment
//! while the port is brought up, set with `adb shell setprop` before a
//! launch: `debug.dmm.log` takes what `RUST_LOG` takes, `debug.dmm.mock=1`
//! opens the simulated meter, `debug.dmm.corner_px` gives the radius of the
//! screen's rounded corners where the system does not report it, and
//! `debug.dmm.ble_short_interval=1` does what `DMM_BLE_SHORT_INTERVAL=1`
//! does.
#![cfg(target_os = "android")]

use android_system_properties::AndroidSystemProperties;
use dmm_gui::{AndroidApp, AndroidStart};
use dmm_lib::{UsbCable, UsbHost, UsbRefusal};
use jni::objects::{JObject, JString};
use jni::{JValue, jni_sig, jni_str};
use std::os::fd::{FromRawFd, OwnedFd};
use std::path::PathBuf;

/// What both desktop binaries log unasked (`dmm_shared::logging`).
const DEFAULT_LOG: &str = "error,dmm_lib=warn";

/// Called by android-activity's glue on a thread of its own, already
/// attached to the Java VM with the application's class loader, once the
/// activity is created.
#[expect(
    unsafe_code,
    reason = "the unmangled symbol android-activity's glue calls"
)]
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    let properties = AndroidSystemProperties::new();
    let set = |name: &str| properties.get(name).filter(|value| !value.is_empty());

    let spec = set("debug.dmm.log").unwrap_or_else(|| DEFAULT_LOG.to_string());
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("dmm")
            // The filter decides; the level only has to let everything by.
            .with_max_level(log::LevelFilter::Trace)
            .with_filter(android_logger::FilterBuilder::new().parse(&spec).build()),
    );

    match app.internal_data_path() {
        Some(dir) => dmm_shared::set_config_dir(dir),
        None => log::error!("no application data folder: settings will not be kept"),
    }
    // Before the window reads its language. Unread, the system's own
    // language stands, without the one picked for the application.
    match app_languages(&app) {
        Ok(tags) => dmm_shared::l10n::set_system_languages(tags),
        Err(e) => log::warn!("the application's language could not be read: {e}"),
    }

    // An open on a btleplug that was not initialised panics, so a failure
    // here keeps this session off the radio.
    let bluetooth = match start_bluetooth() {
        Ok(()) => true,
        Err(e) => {
            log::error!("Bluetooth is unavailable: {e}");
            false
        }
    };

    dmm_lib::set_usb_host(Box::new(Usb(app.clone())));
    // At each start: the process can outlive a launch.
    dmm_lib::set_short_interval_for_every_peer(
        set("debug.dmm.ble_short_interval").as_deref() == Some("1"),
    );

    let activity = app.clone();
    let offers = app.clone();
    let bars = app.clone();
    let start = AndroidStart {
        mock: set("debug.dmm.mock").as_deref() == Some("1"),
        bluetooth,
        // The property is for a system that reports square corners on a
        // screen that has round ones.
        screen_corner_px: set("debug.dmm.corner_px")
            .and_then(|px| px.parse().ok())
            .unwrap_or_else(|| {
                screen_corner_radius(&app).unwrap_or_else(|e| {
                    log::warn!("the screen's corner radius is unknown: {e}");
                    0.0
                })
            }),
        cable_offered: Some(Box::new(move || {
            cable_offered(&offers).unwrap_or_else(|e| {
                log::debug!("no word of a USB cable's offer: {e}");
                false
            })
        })),
        full_screen: Some(Box::new(move |on| {
            if let Err(e) = set_full_screen(&bars, on) {
                log::warn!("the system's bars stay as they are: {e}");
            }
        })),
        pick_file: Some(Box::new(move || match pick_file(&activity) {
            Ok(picked) => picked,
            Err(e) => {
                log::error!("the file picker could not be shown: {e}");
                None
            }
        })),
    };
    if let Err(e) = dmm_gui::run_android(app, start) {
        log::error!("the window could not run: {e}");
    }
}

/// Call the activity's Java side from this thread, attached to the Java
/// VM for the call.
#[expect(
    unsafe_code,
    reason = "the activity reference android-activity holds, borrowed for one call"
)]
fn with_activity<T>(
    app: &AndroidApp,
    call: impl FnOnce(&mut jni::Env<'_>, &JObject<'_>) -> Result<T, jni::errors::Error>,
) -> Result<T, jni::errors::Error> {
    jni::JavaVM::singleton()?.attach_current_thread(|env| {
        // SAFETY: the pointer is the global reference to the activity that
        // android-activity keeps for as long as `app` lives.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        call(env, &activity)
    })
}

/// Ask the user for a file through the system's document picker, which
/// `MainActivity.pickFile` shows and waits on; it hands back a copy of the
/// file in the application's cache, as the picker gives no path to read.
fn pick_file(app: &AndroidApp) -> Result<Option<PathBuf>, jni::errors::Error> {
    with_activity(app, |env, activity| {
        let path = env
            .call_method(
                activity,
                jni_str!("pickFile"),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )?
            .l()?;
        if path.is_null() {
            return Ok(None);
        }
        let path = env.cast_local::<JString>(path)?;
        Ok(Some(PathBuf::from(path.try_to_string(env)?)))
    })
}

/// The languages the user wants the application in, most wanted first,
/// from `MainActivity.appLanguages`.
fn app_languages(app: &AndroidApp) -> Result<Vec<String>, jni::errors::Error> {
    with_activity(app, |env, activity| {
        let tags = env
            .call_method(
                activity,
                jni_str!("appLanguages"),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )?
            .l()?;
        let tags = env.cast_local::<JString>(tags)?.try_to_string(env)?;
        Ok(tags.split(',').map(str::to_string).collect())
    })
}

/// The radius of the screen's rounded corners in pixels, 0 for square ones,
/// from `MainActivity.screenCornerRadius`.
fn screen_corner_radius(app: &AndroidApp) -> Result<f32, jni::errors::Error> {
    with_activity(app, |env, activity| {
        let radius = env
            .call_method(
                activity,
                jni_str!("screenCornerRadius"),
                jni_sig!("()I"),
                &[],
            )?
            .i()?;
        Ok(radius as f32)
    })
}

/// Load btleplug's Java classes and make its adapter, which must happen on
/// a thread whose class loader sees the application's classes: this one.
/// The acquisition thread, attached from native code, finds them ready.
fn start_bluetooth() -> Result<(), btleplug::Error> {
    jni::JavaVM::singleton()?.attach_current_thread(btleplug::platform::init)
}

/// The USB cables, which only the activity's Java side can list and open.
struct Usb(AndroidApp);

impl UsbHost for Usb {
    fn list(&self) -> Vec<UsbCable> {
        match usb_cables(&self.0) {
            Ok(lines) => lines.lines().filter_map(usb_cable).collect(),
            Err(e) => {
                log::error!("the USB devices could not be listed: {e}");
                Vec::new()
            }
        }
    }

    #[expect(
        unsafe_code,
        reason = "taking over the descriptor MainActivity.openUsb duplicated for this side"
    )]
    fn open(&self, name: &str) -> Result<OwnedFd, UsbRefusal> {
        let failed = |why: &str| Err(UsbRefusal::Failed(why.to_string()));
        match open_usb(&self.0, name).map_err(|e| UsbRefusal::Failed(e.to_string()))? {
            // SAFETY: a descriptor `openUsb` detached for its caller to
            // own; nothing else holds or closes it.
            fd if fd >= 0 => Ok(unsafe { OwnedFd::from_raw_fd(fd) }),
            -1 => failed("the device is gone"),
            -2 => Err(UsbRefusal::NotAllowed),
            _ => failed("Android did not open it"),
        }
    }
}

/// One line of `MainActivity.usbCables`: name, vendor id, product id and
/// product name, tab separated.
fn usb_cable(line: &str) -> Option<UsbCable> {
    let mut fields = line.split('\t');
    Some(UsbCable {
        name: fields.next().filter(|name| !name.is_empty())?.to_string(),
        vid: fields.next()?.parse().ok()?,
        pid: fields.next()?.parse().ok()?,
        product: fields
            .next()
            .filter(|product| !product.is_empty())
            .map(str::to_string),
    })
}

/// The attached USB devices, from `MainActivity.usbCables`.
fn usb_cables(app: &AndroidApp) -> Result<String, jni::errors::Error> {
    with_activity(app, |env, activity| {
        let lines = env
            .call_method(
                activity,
                jni_str!("usbCables"),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )?
            .l()?;
        let lines = env.cast_local::<JString>(lines)?;
        lines.try_to_string(env)
    })
}

/// Have `MainActivity.openUsb` open a USB device, which shows the system's
/// permission dialog and waits on it where one is due. A descriptor to own,
/// or the negative number that says why there is none.
fn open_usb(app: &AndroidApp, name: &str) -> Result<i32, jni::errors::Error> {
    with_activity(app, |env, activity| {
        let name = env.new_string(name)?;
        env.call_method(
            activity,
            jni_str!("openUsb"),
            jni_sig!("(Ljava/lang/String;)I"),
            &[JValue::Object(&name)],
        )?
        .i()
    })
}

/// Hide the system's status and navigation bars, or show them again, by
/// `MainActivity.setFullScreen`.
fn set_full_screen(app: &AndroidApp, on: bool) -> Result<(), jni::errors::Error> {
    with_activity(app, |env, activity| {
        env.call_method(
            activity,
            jni_str!("setFullScreen"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(on)],
        )?
        .v()
    })
}

/// Whether the system's offer of a USB cable was answered since the last
/// call, from `MainActivity.takeCableOffered`.
fn cable_offered(app: &AndroidApp) -> Result<bool, jni::errors::Error> {
    with_activity(app, |env, activity| {
        env.call_method(activity, jni_str!("takeCableOffered"), jni_sig!("()Z"), &[])?
            .z()
    })
}
