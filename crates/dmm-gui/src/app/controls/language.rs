//! The **Language** row, first of the window group: a dropdown of System
//! and every language by its own name, a machine-translated one marked so in
//! its own words, and, while such a language is spoken, a line asking for a
//! review.

use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, RichText, Ui};

use crate::a11y::ResponseA11yExt;
use crate::app::App;
use crate::app::language::{LanguageChoice, OwnMark, Speech, TRANSLATION_GUIDE_URL};

impl App {
    /// The **Language** row and its notice. Hidden while the build speaks
    /// one language only: there is nothing to pick.
    pub(super) fn show_language_rows(&mut self, ui: &mut Ui) {
        if self.speech.languages.len() < 2 {
            return;
        }
        let l10n = self.l10n.clone();
        let l10n = l10n.as_ref();
        let choice = self.language_choice();
        let picked = ui
            .horizontal_wrapped(|ui| {
                ui.label(tr!(l10n, "settings-language"));
                language_dropdown(ui, l10n, &self.speech, choice)
            })
            .inner;
        if !l10n.language().reviewed {
            ui.horizontal_wrapped(|ui| {
                let weak = ui.visuals().weak_text_color();
                ui.label(
                    RichText::new(tr!(l10n, "settings-language-unreviewed"))
                        .small()
                        .color(weak),
                );
                ui.hyperlink_to(
                    RichText::new(tr!(l10n, "settings-language-review-link")).small(),
                    TRANSLATION_GUIDE_URL,
                )
                .on_hover_text(tr!(l10n, "settings-language-review-link.hover"));
            });
        }
        if let Some(choice) = picked {
            let ctx = ui.ctx().clone();
            self.choose_language(&ctx, choice);
        }
    }
}

/// The **Language** dropdown: System, naming what it resolves to, then each
/// language in itself, a machine-translated one marked so in its own words.
/// Keyboard as in every choice list ([`listbox_dropdown`]). Returns the
/// entry picked this frame, the live one included.
///
/// [`listbox_dropdown`]: crate::listbox::listbox_dropdown
fn language_dropdown(
    ui: &mut Ui,
    l10n: &Localizer,
    speech: &Speech,
    choice: LanguageChoice,
) -> Option<LanguageChoice> {
    use crate::listbox::{listbox_dropdown, navigate_choice_entries};
    use egui::ComboBox;

    let system = tr!(
        l10n,
        "settings-language-system",
        language = speech.system.native_name
    );
    let shown = match choice {
        LanguageChoice::System => system.as_ref(),
        LanguageChoice::Language(l) => l.native_name,
    };
    // Called through the type, as in `device_dropdown`.
    let combo = |combo| ComboBox::selected_text(combo, shown);
    let entries = std::iter::once(LanguageChoice::System)
        .chain(speech.languages.iter().map(LanguageChoice::Language));
    let (response, picked) = listbox_dropdown(
        ui,
        "language",
        || 0,
        combo,
        |ui, was_open| {
            let mut picked = None;
            let mut responses = Vec::new();
            for entry in entries {
                let live = entry == choice;
                let (name, own): (&str, Option<&OwnMark>) = match entry {
                    LanguageChoice::System => (&system, None),
                    LanguageChoice::Language(l) => (l.native_name, speech.own_mark(l)),
                };
                let (text, spoken) = entry_text(ui, name, own, live);
                let mut response = ui.selectable_label(live, text).a11y_label(&spoken);
                if entry == LanguageChoice::System {
                    response = response.on_hover_text(tr!(l10n, "settings-language-system.hover"));
                }
                // Focus lands on the live entry as the list opens.
                if !was_open && live {
                    response.request_focus();
                }
                if response.clicked() {
                    picked = Some(entry);
                }
                responses.push(response);
            }
            navigate_choice_entries(ui.ctx(), &responses);
            picked
        },
    );
    response
        .on_hover_text(tr!(l10n, "settings-language.hover"))
        .a11y_label(&tr!(l10n, "settings-language.a11y"));
    picked
}

/// An entry of the open list — `name`, marked when it is the live one, and
/// for a machine-translated language its mark after it, weak unless live —
/// and what a screen reader names it.
fn entry_text(
    ui: &Ui,
    name: &str,
    own: Option<&OwnMark>,
    live: bool,
) -> (egui::text::LayoutJob, String) {
    use egui::{Align, FontSelection, TextStyle};
    let font = || FontSelection::Style(TextStyle::Button);
    let mut text = egui::text::LayoutJob::default();
    let style = ui.style();
    RichText::new(crate::listbox::choice_entry_text(name, live)).append_to(
        &mut text,
        style,
        font(),
        Align::Center,
    );
    let Some(own) = own else {
        return (text, name.to_string());
    };
    let mut mark = RichText::new(format!("  {}", own.mark));
    // Not on the live entry: weak text on the selection's fill is too faint
    // to read.
    if !live {
        mark = mark.color(ui.visuals().weak_text_color());
    }
    mark.append_to(&mut text, style, font(), Align::Center);
    (text, own.spoken.clone())
}
