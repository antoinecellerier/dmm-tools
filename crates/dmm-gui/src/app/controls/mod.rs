//! The settings panel, its rows and the meter's remote buttons.

mod colors;
mod device_list;
pub(super) mod remote;
mod settings_panel;
pub(super) mod theme_save;

use eframe::egui::{self, Ui};

/// Show a selectable_label with a hover tooltip; returns `true` if clicked.
fn setting_selectable(
    ui: &mut Ui,
    selected: bool,
    label: impl Into<egui::WidgetText>,
    tooltip: &str,
) -> bool {
    ui.selectable_label(selected, label)
        .on_hover_text(tooltip)
        .clicked()
}

/// One chip in a settings row: the setting it stands for, what it says, what
/// it says on hover, and whether it is the current selection.
pub(super) struct Chip<T> {
    pub(super) value: T,
    pub(super) selected: bool,
    pub(super) label: String,
    pub(super) tooltip: String,
}

/// A caption followed by a run of selectable chips, as every settings row in
/// the panel draws them. Returns the value of the chip clicked this frame.
///
/// The caller keeps its own layout container and its own trailing hint: the
/// rows differ in whether they wrap and in what they add after the chips.
pub(super) fn chip_row<T>(
    ui: &mut Ui,
    caption: &str,
    chips: impl IntoIterator<Item = Chip<T>>,
) -> Option<T> {
    ui.label(caption);
    let mut picked = None;
    for chip in chips {
        if setting_selectable(ui, chip.selected, chip.label, &chip.tooltip) {
            picked = Some(chip.value);
        }
    }
    picked
}
