//! The logger both binaries install.
//!
//! `RUST_LOG` and `RUST_LOG_STYLE` are read as `env_logger::init` reads them.
//! Unset or empty, `dmm_lib` logs at WARN and everything else at ERROR, so
//! what the meter library has to say about a connection — data it does not
//! recognise, a cable it had to pick among several — reaches the terminal
//! without the user asking for it. A set `RUST_LOG` is used as it is: a
//! `dmm_lib` directive beside it would outrank a global `RUST_LOG=trace`.

use log::LevelFilter;
use std::ffi::OsStr;

/// Install the logger. Call once, first thing in `main`.
pub fn init() {
    let mut builder = env_logger::Builder::from_default_env();
    default_unless_set(
        &mut builder,
        std::env::var_os(env_logger::DEFAULT_FILTER_ENV).as_deref(),
    );
    builder.init();
}

fn default_unless_set(builder: &mut env_logger::Builder, rust_log: Option<&OsStr>) {
    if rust_log.is_none_or(OsStr::is_empty) {
        // Both, because a lone module directive turns every other target off.
        builder
            .filter_level(LevelFilter::Error)
            .filter_module("dmm_lib", LevelFilter::Warn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{Level, Log, Metadata};

    /// The logger `init` would install for this `RUST_LOG`, without
    /// installing it — so the environment of the test run plays no part.
    fn logger(rust_log: Option<&str>) -> env_logger::Logger {
        let mut builder = env_logger::Builder::new();
        if let Some(spec) = rust_log {
            builder.parse_filters(spec);
        }
        default_unless_set(&mut builder, rust_log.map(OsStr::new));
        builder.build()
    }

    fn enabled(logger: &env_logger::Logger, target: &str, level: Level) -> bool {
        logger.enabled(&Metadata::builder().target(target).level(level).build())
    }

    #[test]
    fn unset_shows_dmm_lib_warnings_and_other_errors() {
        for rust_log in [None, Some("")] {
            let l = logger(rust_log);
            assert!(enabled(&l, "dmm_lib::protocol::unrecognised", Level::Warn));
            assert!(!enabled(&l, "dmm_lib::detect", Level::Info));
            assert!(enabled(&l, "dmm_gui::app", Level::Error));
            assert!(!enabled(&l, "dmm_cli", Level::Warn));
            assert!(!enabled(&l, "eframe", Level::Warn));
        }
    }

    /// A set `RUST_LOG` filters exactly as it would on its own.
    #[test]
    fn a_set_rust_log_is_used_as_is() {
        let levels = [
            Level::Error,
            Level::Warn,
            Level::Info,
            Level::Debug,
            Level::Trace,
        ];
        let targets = ["dmm_lib::protocol::framing", "dmm_cli", "eframe"];
        for spec in [
            "trace",
            "dmm_lib=trace",
            "dmm_lib=debug",
            "info,dmm_lib=off",
            "off",
        ] {
            let ours = logger(Some(spec));
            let plain = env_logger::Builder::new().parse_filters(spec).build();
            for target in targets {
                for level in levels {
                    assert_eq!(
                        enabled(&ours, target, level),
                        enabled(&plain, target, level),
                        "RUST_LOG={spec}: {target} at {level}"
                    );
                }
            }
        }
    }
}
