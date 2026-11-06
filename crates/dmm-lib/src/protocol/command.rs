//! The names of the buttons the GUI's remote row draws, as a family's
//! `supported_commands` lists them and
//! [`Protocol::send_command`](super::Protocol::send_command) takes them.
//!
//! The row finds its buttons by these names, so a family that spells one
//! otherwise gets no button for it. They are also the words `dmm-cli command`
//! takes: changing one breaks the scripts that type it. Any other key stays a
//! literal in its family.

pub const HOLD: &str = "hold";
pub const REL: &str = "rel";
pub const RANGE: &str = "range";
pub const AUTO: &str = "auto";
pub const MINMAX: &str = "minmax";
pub const EXIT_MINMAX: &str = "exit_minmax";
pub const PEAK: &str = "peak";
pub const EXIT_PEAK: &str = "exit_peak";
pub const SELECT: &str = "select";
pub const LIGHT: &str = "light";
