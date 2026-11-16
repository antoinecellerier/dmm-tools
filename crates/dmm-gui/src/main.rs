//! The desktop binary. Everything is in the library, which the Android
//! build links too.

fn main() -> eframe::Result<()> {
    dmm_gui::run()
}
