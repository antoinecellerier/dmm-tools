//! The Android build of dmm-gui, experimental and Bluetooth only: the
//! shared library the activity in `android/` loads. What a desktop `main`
//! would find around it is set up here — where logs go, where settings
//! live, btleplug's Java half — and then the window runs as it does
//! everywhere (`dmm_gui::run_android`).
//!
//! Two system properties stand in for a command line while the port is
//! brought up, set with `adb shell setprop` before a launch:
//! `debug.dmm.log` takes what `RUST_LOG` takes, `debug.dmm.mock=1` opens the
//! simulated meter, and `debug.dmm.corner_px` gives the radius of the
//! screen's rounded corners where the system does not report it.
#![cfg(target_os = "android")]

use android_system_properties::AndroidSystemProperties;
use dmm_gui::{AndroidApp, AndroidStart};
use jni::objects::{JObject, JString};
use jni::{jni_sig, jni_str};
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

    // An open on a btleplug that was not initialised panics, so a failure
    // here keeps this session off the radio.
    let bluetooth = match start_bluetooth() {
        Ok(()) => true,
        Err(e) => {
            log::error!("Bluetooth is unavailable: {e}");
            false
        }
    };

    let activity = app.clone();
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

/// Ask the user for a file through the system's document picker, which
/// `MainActivity.pickFile` shows and waits on; it hands back a copy of the
/// file in the application's cache, as the picker gives no path to read.
#[expect(
    unsafe_code,
    reason = "the activity reference android-activity holds, borrowed for one call"
)]
fn pick_file(app: &AndroidApp) -> Result<Option<PathBuf>, jni::errors::Error> {
    jni::JavaVM::singleton()?.attach_current_thread(|env| {
        // SAFETY: the pointer is the global reference to the activity that
        // android-activity keeps for as long as `app` lives.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        let path = env
            .call_method(
                &activity,
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

/// The radius of the screen's rounded corners in pixels, 0 for square ones,
/// from `MainActivity.screenCornerRadius`.
#[expect(
    unsafe_code,
    reason = "the activity reference android-activity holds, borrowed for one call"
)]
fn screen_corner_radius(app: &AndroidApp) -> Result<f32, jni::errors::Error> {
    jni::JavaVM::singleton()?.attach_current_thread(|env| {
        // SAFETY: as in `pick_file`.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        let radius = env
            .call_method(
                &activity,
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
