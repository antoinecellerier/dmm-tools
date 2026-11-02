//! The mode and range readouts beside the reading, and the dropdowns that
//! pick another mode, range or setting from them.

use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::{Choice, MeterKey, Setting};
use dmm_lib::text::TextKind;
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{Color32, ComboBox, FontId, RichText, Stroke, Ui};
use std::borrow::Cow;

use super::{MAX_CHOICE_POPUP_FONT_SIZE, MIN_AUX_FONT_SIZE};
use crate::a11y::ResponseA11yExt;
use crate::listbox::{choice_entry_text, listbox_dropdown, navigate_choice_entries};

/// The lists the two readout dropdowns offer, as the acquisition thread last
/// reported them. Passed as one value so the reading widgets keep a short
/// signature as further settings join them.
#[derive(Clone, Copy, Default)]
pub struct ReadoutChoices<'a> {
    /// Modes reachable from the current dial position.
    pub mode: &'a [Choice],
    /// `Auto` plus the rungs of the current mode's ladder.
    pub range: &'a [Choice],
    /// The meter's function keys, from its profile: the mode readout lists
    /// them when `mode` has nothing to pick.
    pub keys: &'a [MeterKey],
}

impl ReadoutChoices<'_> {
    /// Whether either readout has something to pick. The single-line layouts
    /// draw plain labels inside the spoken row when neither does, and the
    /// selector row when one does.
    pub(super) fn any_offered(&self) -> bool {
        self.mode_offered() || mode_switch_offered(self.range)
    }

    /// Whether the mode readout is a dropdown: of modes, or of keys.
    pub(crate) fn mode_offered(&self) -> bool {
        mode_switch_offered(self.mode) || !self.keys.is_empty()
    }
}

/// What a pick in a readout dropdown asks of the meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadoutPick {
    /// Switch a setting to one of the ids `Dmm::choices` listed.
    Select(Setting, u16),
    /// Press one of the meter's keys, by its command.
    Press(&'static str),
}

/// Whether a readout is a selector rather than a plain label.
///
/// A single choice is the live mode on its own — the single-variant UT181A
/// dials (Ohm, nS, Cap, Hz, Duty, Pulse Width) report exactly that — so it
/// means what an empty list means: nothing to pick, and no control to draw.
pub(crate) fn mode_switch_offered(choices: &[Choice]) -> bool {
    choices.len() > 1
}

/// One readout dropdown: the widget id it keys its popup and focus state
/// under — two dropdowns on the same line must not share one — and which
/// readout it is, which picks its wording.
struct Dropdown {
    id_salt: &'static str,
    kind: DropdownKind,
}

/// The readout a dropdown is, for its wording.
#[derive(Clone, Copy)]
enum DropdownKind {
    Mode,
    Range,
    Keys,
}

impl Dropdown {
    /// The hover text.
    fn hover<'a>(&self, l10n: &'a Localizer) -> Cow<'a, str> {
        match self.kind {
            DropdownKind::Mode => tr!(l10n, "reading-mode.hover"),
            DropdownKind::Range => tr!(l10n, "reading-range.hover"),
            DropdownKind::Keys => tr!(l10n, "reading-keys.hover"),
        }
    }

    /// The name a screen reader gives it.
    fn a11y<'a>(&self, l10n: &'a Localizer) -> Cow<'a, str> {
        match self.kind {
            DropdownKind::Mode => tr!(l10n, "reading-mode.a11y"),
            DropdownKind::Range => tr!(l10n, "reading-range.a11y"),
            DropdownKind::Keys => tr!(l10n, "reading-keys.a11y"),
        }
    }

    /// The caption over its list, if any.
    fn heading<'a>(&self, l10n: &'a Localizer) -> Option<Cow<'a, str>> {
        match self.kind {
            DropdownKind::Mode | DropdownKind::Range => None,
            DropdownKind::Keys => Some(tr!(l10n, "reading-keys")),
        }
    }
}

/// A dropdown over one setting's choices, the setting a pick switches, and
/// the kind its labels are looked up as.
pub(super) struct ChoiceReadout {
    setting: Setting,
    kind: TextKind,
    dropdown: Dropdown,
}

/// The mode readout: which mode of the current dial position the meter is in.
pub(super) const MODE_READOUT: ChoiceReadout = ChoiceReadout {
    setting: Setting::Mode,
    kind: TextKind::Mode,
    dropdown: Dropdown {
        id_salt: "mode_select",
        kind: DropdownKind::Mode,
    },
};

/// The range readout beside it: which rung of the current mode's ladder the
/// meter is on, or `Auto` while it picks the rung itself.
pub(super) const RANGE_READOUT: ChoiceReadout = ChoiceReadout {
    setting: Setting::Range,
    kind: TextKind::Range,
    dropdown: Dropdown {
        id_salt: "range_select",
        kind: DropdownKind::Range,
    },
};

/// The mode readout on a meter that lists function keys instead of modes.
/// A screen reader still calls it "Mode"; the caption says the entries are
/// key presses, not modes the meter is sure to land on.
const KEYS_READOUT: Dropdown = Dropdown {
    id_salt: "mode_keys",
    kind: DropdownKind::Keys,
};

/// The mode readout: a dropdown of the modes the meter can be switched to,
/// else of its function keys, else the plain label.
///
/// In the key list the marked entry is the key whose function the reading
/// shows, and every pick is a press — the marked one too, as a key can
/// cycle within its function (diode and continuity, °C and °F).
pub(super) fn show_mode_readout(
    ui: &mut Ui,
    l10n: &Localizer,
    m: &Measurement,
    size: f32,
    choices: ReadoutChoices<'_>,
) -> Option<ReadoutPick> {
    if mode_switch_offered(choices.mode) || choices.keys.is_empty() {
        return show_choice_readout(ui, l10n, &MODE_READOUT, &m.mode, size, choices.mode);
    }
    let entries: Vec<(Cow<'_, str>, bool)> = choices
        .keys
        .iter()
        .map(|k| (l10n.label(TextKind::Key, k.label), (k.applies)(m)))
        .collect();
    let mode = l10n.label(TextKind::Mode, &m.mode);
    show_dropdown(ui, l10n, &KEYS_READOUT, &mode, size, &entries)
        .map(|i| ReadoutPick::Press(choices.keys[i].command))
}

/// What the range readout leaves behind on a meter with no rung to pick.
///
/// The two-line layout has always drawn the range as a plain label beside the
/// mode, so it keeps it; the single-line layouts never drew one, and a meter
/// that cannot switch ranges must not gain text there.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RangeAtRest {
    Label,
    Nothing,
}

/// The range readout beside the mode one.
///
/// The closed text is always the live label the meter reports, so it reads
/// the same whether the meter picked the rung or the user did; the marked
/// entry is `Auto` while the meter is auto-ranging and the live rung
/// otherwise. `at_rest` decides what a meter with nothing to pick gets.
pub(super) fn show_range_readout(
    ui: &mut Ui,
    l10n: &Localizer,
    m: &Measurement,
    size: f32,
    choices: &[Choice],
    at_rest: RangeAtRest,
) -> Option<ReadoutPick> {
    // A meter that reports no range at all has nothing to label, in any
    // layout — the UT61E+ temperature and NCV positions, for two.
    if m.range_label.is_empty() {
        return None;
    }
    if at_rest == RangeAtRest::Nothing && !mode_switch_offered(choices) {
        return None;
    }
    show_choice_readout(ui, l10n, &RANGE_READOUT, &m.range_label, size, choices)
}

/// A readout under the reading: a plain label, or — when the meter lists more
/// than one value the setting can take from where it sits — a dropdown
/// listing them, with the live one marked.
///
/// Returns the setting and the id of a value the user picked that differs
/// from the live one.
pub(super) fn show_choice_readout(
    ui: &mut Ui,
    l10n: &Localizer,
    readout: &ChoiceReadout,
    label: &str,
    size: f32,
    choices: &[Choice],
) -> Option<ReadoutPick> {
    let label = l10n.label(readout.kind, label);
    if !mode_switch_offered(choices) {
        ui.label(
            RichText::new(label)
                .font(FontId::proportional(size))
                .color(ui.visuals().weak_text_color()),
        );
        return None;
    }
    let entries: Vec<(Cow<'_, str>, bool)> = choices
        .iter()
        .map(|c| (l10n.label(readout.kind, &c.label), c.current))
        .collect();
    let picked = &choices[show_dropdown(ui, l10n, &readout.dropdown, &label, size, &entries)?];
    (!picked.current).then_some(ReadoutPick::Select(readout.setting, picked.id))
}

/// A readout drawn as a dropdown over `entries`, each a label and whether it
/// is the live one, which the list marks. Returns the index of the entry
/// picked this frame, the live one included.
///
/// The dropdown is drawn at the label's size and with no frame at rest, so
/// it reads as the readout it replaces; hover and the open state keep egui's
/// own highlight so it still answers as a control. Its popup is capped at
/// [`MAX_CHOICE_POPUP_FONT_SIZE`], under the dropdown's caption if it has one.
///
/// The open list behaves as a native listbox through [`listbox_dropdown`]:
/// focus lands on the live entry as it opens (the first when none is live),
/// Up/Down (Home/End) move it, Enter/Space or a click picks, Esc or Tab
/// closes without a pick, and focus returns to the readout on every close.
fn show_dropdown(
    ui: &mut Ui,
    l10n: &Localizer,
    dropdown: &Dropdown,
    label: &str,
    size: f32,
    entries: &[(Cow<'_, str>, bool)],
) -> Option<usize> {
    let focus_on_open = entries.iter().position(|(_, live)| *live).unwrap_or(0);
    let popup_size = size.clamp(MIN_AUX_FONT_SIZE, MAX_CHOICE_POPUP_FONT_SIZE);
    let ctx = ui.ctx().clone();
    let row_height = ctx.fonts_mut(|f| f.row_height(&FontId::proportional(size)));
    let (response, picked) = ui
        .scope(|ui| {
            let widgets = &mut ui.visuals_mut().widgets;
            widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
            widgets.inactive.bg_stroke = Stroke::NONE;
            // egui draws its arrow to `icon_width`, a fixed 14 px that is
            // lost beside a big-meter readout; follow the text instead.
            let spacing = ui.spacing_mut();
            spacing.icon_width = (size * 0.8).max(8.0);
            spacing.icon_spacing = (size * 0.25).max(2.0);
            // `ComboBox::show_ui` lays the box out in a nested `ui.horizontal`,
            // whose row starts `interact_size.y` (~18 px) tall and pushes
            // taller content down rather than over the row above (the quirk
            // noted on the sub-value grid). Start that row at the box's own
            // height — egui's text-or-icon height plus the button padding —
            // so a big-meter readout stays centred beside its label. No
            // vertical padding: the box is the label's height, so it also
            // sits level in a wrapped row, which aligns tops.
            spacing.button_padding.y = 0.0;
            let box_height = row_height.max(spacing.icon_width);
            spacing.interact_size.y = spacing.interact_size.y.max(box_height);
            // The interactive colour, not the label's weak one: this is a
            // control, and it has to clear the text contrast bar as one.
            let text_color = ui.visuals().text_color();
            // Called through the type, as in `device_dropdown`.
            let combo = |combo| {
                ComboBox::width(combo, 0.0)
                    // egui's default cap scrolls a list of ten keys; let it
                    // grow to the window, scrolling only when the window is
                    // shorter.
                    .height(ctx.content_rect().height())
                    .selected_text(
                        RichText::new(label)
                            .font(FontId::proportional(size))
                            .color(text_color),
                    )
            };
            listbox_dropdown(
                ui,
                dropdown.id_salt,
                || 0,
                combo,
                |ui, was_open| {
                    if let Some(heading) = dropdown.heading(l10n) {
                        ui.label(
                            RichText::new(heading)
                                .font(FontId::proportional(popup_size))
                                .color(ui.visuals().weak_text_color()),
                        );
                    }
                    let mut picked = None;
                    let mut responses = Vec::with_capacity(entries.len());
                    for (i, (text, live)) in entries.iter().enumerate() {
                        let live = *live;
                        let entry = ui.selectable_label(
                            live,
                            RichText::new(choice_entry_text(text, live))
                                .font(FontId::proportional(popup_size)),
                        );
                        // Focus lands on the live entry as the list opens — by
                        // click, or by Enter/Space on the readout — so a screen
                        // reader announces it and Enter picks it.
                        if !was_open && i == focus_on_open {
                            entry.request_focus();
                        }
                        if entry.clicked() {
                            picked = Some(i);
                        }
                        responses.push(entry);
                    }
                    navigate_choice_entries(ui.ctx(), &responses);
                    picked
                },
            )
        })
        .inner;
    response
        .on_hover_text(dropdown.hover(l10n))
        .a11y_label(&dropdown.a11y(l10n));
    picked
}
