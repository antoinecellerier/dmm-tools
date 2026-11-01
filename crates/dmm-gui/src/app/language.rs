//! The language the window speaks: the one it starts in, the switch the
//! Settings **Language** row makes, and what follows the language — the
//! system fonts' shapes, the screen reader's language and the check that the
//! system can draw it at all.
//!
//! A switch takes effect on the next frame: every label is drawn from the
//! new localizer, and the caches built from text (the big meter's fit, the
//! device list's width, the screen-reader summaries) are keyed on the
//! language, so they rebuild by themselves. Text already built keeps the
//! language it was built in: a toast or a notice on screen, the reading a
//! marker recorded.

use dmm_shared::l10n::{EN_US, Language, Localizer, Options};
use dmm_shared::tr;
use eframe::egui;
use egui::epaint::text::{FallbackRequest, FontFamily, FontProvider};
use egui_system_font_provider::SystemFontProvider;
use std::sync::{Arc, OnceLock, mpsc};

use super::App;
use super::toast::Toast;

/// Where the notice under the **Language** row sends a reader who can help
/// review a machine-translated language.
pub(super) const TRANSLATION_GUIDE_URL: &str =
    "https://github.com/antoinecellerier/dmm-tools/blob/main/CONTRIBUTING.md#translations";

/// What the window speaks and could speak: the languages on offer, the one
/// the system asks for, and what the fonts were set up for.
pub(super) struct Speech {
    /// The languages the **Language** row offers. The row shows only when
    /// there is more than one.
    pub(super) languages: &'static [Language],
    /// What following the system resolves to: the row's System entry.
    pub(super) system: &'static Language,
    /// How every localizer of this session is built (the pseudo-locale, the
    /// override folder), as the environment set it at startup.
    pub(super) options: Options,
    /// The language the system font provider was installed for, `None`
    /// before the first.
    fonts_for: Option<&'static str>,
    /// How many providers were installed: one at startup, one per switch.
    pub(super) font_installs: u32,
    /// The look for a font that draws the language's script, while it runs.
    font_check: Option<mpsc::Receiver<Option<bool>>>,
    /// Each machine-translated language's mark, in its own words: built on
    /// first use ([`Speech::own_mark`]).
    own_marks: OnceLock<Vec<OwnMark>>,
}

/// How the **Language** list marks a machine-translated language, in that
/// language rather than the window's, like its name: its speakers read it
/// whatever the window speaks.
pub(super) struct OwnMark {
    /// The language's id.
    id: &'static str,
    /// Drawn after the name.
    pub(super) mark: String,
    /// What a screen reader names the entry: the name, then the mark.
    pub(super) spoken: String,
}

impl Speech {
    /// `language`'s mark, `None` for a reviewed language. The first call
    /// builds them all, a localizer per machine-translated language from the
    /// settings file alone, never per frame: the languages and the options
    /// stay as the session started. A language lacking the message marks
    /// itself in English.
    pub(super) fn own_mark(&self, language: &Language) -> Option<&OwnMark> {
        let marks = self.own_marks.get_or_init(|| {
            let resources = crate::l10n::settings_resources();
            self.languages
                .iter()
                .filter(|l| !l.reviewed)
                .map(|l| {
                    let own = Localizer::new(l, &resources, &self.options);
                    OwnMark {
                        id: l.id,
                        mark: tr!(own, "settings-language-machine-translated").into_owned(),
                        spoken: tr!(
                            own,
                            "settings-language-machine-translated-entry",
                            language = l.native_name
                        )
                        .into_owned(),
                    }
                })
                .collect()
        });
        marks.iter().find(|m| m.id == language.id)
    }
}

impl Default for Speech {
    /// English, with no other language on offer and no fonts installed: what
    /// tests and an app built without a window start with.
    fn default() -> Self {
        Self {
            languages: std::slice::from_ref(&EN_US),
            system: &EN_US,
            options: Options::default(),
            fonts_for: None,
            font_installs: 0,
            font_check: None,
            own_marks: OnceLock::new(),
        }
    }
}

/// What the user picks in the **Language** row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum LanguageChoice {
    /// Follow the system's language: the setting left empty.
    System,
    /// One language, whatever the system speaks.
    Language(&'static Language),
}

/// The locale the system font provider picks between fonts of one script
/// by. Han characters are drawn differently in China and Japan, and the
/// provider's default is the system's locale, so a Chinese UI on a Japanese
/// desktop would get Japanese shapes. English keeps the system's locale, as
/// eframe's own fallback did: `None` here would pick fontique's default Han
/// font, often the Japanese one, on a Chinese desktop.
fn font_locale(language: &Language) -> Option<&'static str> {
    match language.id {
        id if id == EN_US.id => None,
        "zh-CN" => Some("zh-Hans"),
        id => Some(id),
    }
}

/// A character of the language's script that the bundled fonts lack: the
/// languages only a system font can draw, and what to look for.
fn script_probe(language: &Language) -> Option<&'static str> {
    match language.id {
        "zh-CN" => Some("\u{4E2D}"),
        _ => None,
    }
}

impl App {
    /// Speak the language `setting` names (empty: follow the system), as the
    /// app does at startup in this process's environment.
    pub(super) fn start_speaking(&mut self, ctx: &egui::Context) {
        let setting = &self.settings.shared.language;
        let language = dmm_shared::l10n::resolve_from_env(None, setting);
        self.speech = Speech {
            languages: dmm_shared::l10n::languages(),
            system: dmm_shared::l10n::resolve_from_env(None, ""),
            options: Options::from_env(),
            ..Speech::default()
        };
        let l10n = Localizer::new(language, &crate::l10n::resources(), &self.speech.options);
        // `main` installs the logger first, so what loading found is logged.
        l10n.log_load_report();
        self.l10n = Arc::new(l10n);
        self.install_fonts(ctx);
    }

    /// The language the **Language** row shows as picked: the one the
    /// setting names, matched as resolution matches it (`fr-FR` is French),
    /// or System when it names none on offer — which is what resolution
    /// falls back to as well.
    pub(super) fn language_choice(&self) -> LanguageChoice {
        dmm_shared::l10n::find_in(self.speech.languages, &self.settings.shared.language)
            .map_or(LanguageChoice::System, LanguageChoice::Language)
    }

    /// Save `choice` and speak it from the next frame on. The localizer and
    /// the fonts change only when the language does: System and the
    /// language it resolves to speak alike.
    pub(super) fn choose_language(&mut self, ctx: &egui::Context, choice: LanguageChoice) {
        let (setting, language) = match choice {
            LanguageChoice::System => ("", self.speech.system),
            LanguageChoice::Language(l) => (l.id, l),
        };
        self.settings.shared.language = setting.to_string();
        self.settings.save();
        if language.id == self.l10n.language().id {
            return;
        }
        log::info!("UI: language {}", language.id);
        let l10n = Localizer::new(language, &crate::l10n::resources(), &self.speech.options);
        // `main` installs the logger first, so what loading found is logged.
        l10n.log_load_report();
        self.l10n = Arc::new(l10n);
        self.install_fonts(ctx);
    }

    /// Hand egui a system font provider picking shapes for the window's
    /// language, unless it already has that one: building one enumerates
    /// the system's fonts on a thread of its own, and replacing it reloads
    /// every font, so this runs at startup and on a switch, never per frame.
    /// One is installed for every language, English too, so a name in
    /// another script — a theme's, a file's — still draws. Glyphs it finds
    /// arrive a few frames after a switch, once the enumeration is done.
    fn install_fonts(&mut self, ctx: &egui::Context) {
        let language = self.l10n.language();
        if self.speech.fonts_for == Some(language.id) {
            return;
        }
        let locale = font_locale(language);
        let provider = SystemFontProvider::new();
        let provider = Arc::new(match locale {
            Some(locale) => provider.with_locale(Some(locale)),
            None => provider,
        });
        ctx.set_font_providers(vec![provider.clone()]);
        self.speech.fonts_for = Some(language.id);
        self.speech.font_installs += 1;
        log::debug!("UI: system fonts for {} ({locale:?})", language.id);
        self.speech.font_check =
            script_probe(language).map(|probe| check_font(ctx.clone(), provider, probe));
    }

    /// Take in the font check's answer. With no font for the language's
    /// script, a toast names what to install — in English, which the bundled
    /// fonts draw, where the language's own words would be boxes.
    pub(super) fn poll_font_check(&mut self) {
        let Some(check) = &self.speech.font_check else {
            return;
        };
        let found = match check.try_recv() {
            Err(mpsc::TryRecvError::Empty) => return,
            Ok(found) => found,
            Err(mpsc::TryRecvError::Disconnected) => None,
        };
        self.speech.font_check = None;
        match found {
            Some(true) => {}
            Some(false) => {
                log::warn!(
                    "UI: no system font draws {}",
                    self.l10n.language().native_name
                );
                let english = crate::l10n::english();
                let message = tr!(english, "settings-language-no-chinese-font").into_owned();
                self.toast = Some(Toast::error(message));
            }
            None => log::warn!("UI: the font check stopped without an answer"),
        }
    }

    /// Tell a screen reader which language the window's text is in, on the
    /// root of the accessibility tree. Every pass: egui builds the tree anew
    /// each one.
    pub(super) fn tell_screen_reader_language(&self, ctx: &egui::Context) {
        let id = self.l10n.language().id;
        ctx.accesskit_node_builder(egui::accesskit_root_id(), |node| node.set_language(id));
    }
}

/// Ask `provider`, on a thread of its own, whether any system font draws
/// `probe`. The provider waits for its font enumeration first, which takes
/// a moment the window shouldn't. The answer is `None` if the look itself
/// failed.
fn check_font(
    ctx: egui::Context,
    provider: Arc<SystemFontProvider>,
    probe: &'static str,
) -> mpsc::Receiver<Option<bool>> {
    let (tx, rx) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("font-check".into())
        .spawn(move || {
            let found = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let request = FallbackRequest {
                    cluster: probe,
                    family: &FontFamily::Proportional,
                };
                provider.font_for(&request).is_some()
            }));
            let _ = tx.send(found.ok());
            ctx.request_repaint();
        });
    if let Err(e) = spawned {
        log::warn!("UI: can't start the font check: {e}");
    }
    rx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    /// Every language but English gets its own shapes, Chinese in the
    /// simplified script.
    #[test]
    fn the_fonts_follow_the_language() {
        let fr = Language {
            id: "fr",
            native_name: "Français",
            reviewed: true,
        };
        let zh = Language {
            id: "zh-CN",
            native_name: "简体中文",
            reviewed: false,
        };
        assert_eq!(font_locale(&EN_US), None);
        assert_eq!(font_locale(&fr), Some("fr"));
        assert_eq!(font_locale(&zh), Some("zh-Hans"));
        assert_eq!(script_probe(&fr), None);
        assert!(script_probe(&zh).is_some());
    }

    /// The root of the accessibility tree carries the window's language,
    /// pass after pass.
    #[test]
    fn the_screen_reader_hears_the_language() {
        let app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        let ctx = crate::app::styled_context();
        ctx.enable_accesskit();
        for _ in 0..2 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                app.tell_screen_reader_language(ui.ctx());
                ui.label("Settings");
            });
            out.textures_delta.clear();
            let update = out.platform_output.accesskit_update.expect("a tree");
            let root = egui::accesskit_root_id().accesskit_id();
            let (_, node) = update
                .nodes
                .iter()
                .find(|(id, _)| *id == root)
                .expect("the root node");
            assert_eq!(node.language(), Some("en-US"));
        }
    }
}
