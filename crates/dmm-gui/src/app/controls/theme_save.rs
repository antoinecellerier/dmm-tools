//! Save as theme: the colours on screen written to a theme file through the
//! save dialog, which opens in the user's themes folder.
//!
//! Offered only while the user has changed a colour: an unchanged palette is
//! already a preset or a theme. A file saved into the themes folder is a
//! theme at once — the Theme row offers it and switches to it, and one under
//! a built-in's name takes the built-in's place. Saved anywhere else it is a
//! file to share. The dialog asks before replacing the file it names, as it
//! does for Export…; a name typed without `.json` gets it added, and is not
//! saved over a file of that name the dialog never asked about.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use eframe::egui::{self, Ui};

use crate::app::App;
use crate::app::toast::Toast;
use crate::settings::PaletteOverrides;
use crate::theme::named::{self, NamedTheme};

/// The answer of a save dialog: the path picked, `None` when cancelled, or
/// the dialog's panic.
type DialogAnswer = Result<Option<PathBuf>, String>;

/// The save dialog, while it is open.
#[derive(Default)]
pub(in crate::app) struct ThemeSave {
    dialog: Option<Receiver<DialogAnswer>>,
}

/// Why nothing was saved when the dialog itself fell over.
const DIALOG_FAILED: &str = "the save dialog failed";

impl App {
    /// Whether the colours on screen differ from the preset's or the
    /// theme's own.
    fn has_color_changes(&self, dark: bool) -> bool {
        self.settings
            .color_tweaks(dark)
            .is_some_and(|t| *t != PaletteOverrides::default())
    }

    /// The Save as theme button, under the colour swatches.
    pub(super) fn show_theme_save_row(&mut self, ui: &mut Ui, dark: bool) {
        if !self.has_color_changes(dark) {
            return;
        }
        if ui
            .button("Save as theme\u{2026}")
            .on_hover_text(
                "Save these colors as a theme file; one saved in your themes folder joins the \
                 Theme row",
            )
            .clicked()
        {
            self.choose_theme_file(ui.ctx());
        }
    }

    /// Open the save dialog on a thread of its own, in the themes folder,
    /// named after the theme in use.
    fn choose_theme_file(&mut self, ctx: &egui::Context) {
        if self.theme_save.dialog.is_some() {
            return;
        }
        let dir = named::user_dir();
        if let Some(dir) = &dir {
            // So the dialog can open there on a first save.
            let _ = std::fs::create_dir_all(dir);
        }
        let file_name = match self.settings.active_theme() {
            Some(theme) => theme
                .file
                .clone()
                .unwrap_or_else(|| format!("{}.json", named::file_stem(&theme.name))),
            None => "my-theme.json".to_string(),
        };
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let answer = std::panic::catch_unwind(|| {
                let mut dialog = rfd::FileDialog::new()
                    .set_file_name(file_name)
                    .add_filter("Theme", &["json"]);
                if let Some(dir) = dir {
                    dialog = dialog.set_directory(dir);
                }
                dialog.save_file()
            })
            .map_err(|_| DIALOG_FAILED.to_string());
            let _ = tx.send(answer);
            ctx.request_repaint();
        });
        self.theme_save.dialog = Some(rx);
    }

    /// Take in the save dialog's answer, and save.
    pub(in crate::app) fn poll_theme_save(&mut self) {
        let Some(rx) = &self.theme_save.dialog else {
            return;
        };
        let answer = match rx.try_recv() {
            Ok(answer) => answer,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Err(DIALOG_FAILED.to_string()),
        };
        // The colours on screen when the answer comes: the theme may have
        // changed while the dialog was open.
        let dark = self.applied.theme != Some(crate::settings::ThemeMode::Light);
        self.theme_save.dialog = None;
        match answer {
            Ok(Some(path)) => {
                let folder = named::user_dir();
                self.save_theme(path, dark, folder.as_deref());
            }
            Ok(None) => {}
            Err(why) => self.toast = Some(Toast::error(format!("Not saved: {why}"))),
        }
    }

    /// Write the theme to `path`; into the themes `folder`, switch to it.
    fn save_theme(&mut self, mut path: PathBuf, dark: bool, folder: Option<&Path>) {
        // Only `*.json` is read back from the folder: a file named otherwise
        // would be a theme until the next read.
        if !path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("json"))
        {
            let mut name = path.file_name().unwrap_or_default().to_os_string();
            name.push(".json");
            path.set_file_name(name);
            // The dialog asked about replacing the name as typed, not this.
            if path.exists() {
                self.toast = Some(Toast::error(format!(
                    "Not saved: {} already exists; pick it in the dialog to replace it",
                    path.display()
                )));
                return;
            }
        }
        let file = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let in_folder = folder.is_some_and(|dir| same_dir(&path, dir));
        // Saving a theme back over its own file keeps its name; any other
        // file names the theme.
        let name = self
            .settings
            .active_theme()
            .filter(|t| in_folder && t.file.as_deref() == Some(&file))
            .map(|t| t.name.clone())
            .unwrap_or_else(|| named::name_from_file(&file));
        if in_folder
            && let Some(other) = self
                .settings
                .user_themes
                .themes
                .iter()
                .find(|t| named::same_name(&t.name, &name) && t.file.as_deref() != Some(&file))
        {
            self.toast = Some(Toast::error(format!(
                "Not saved: {name} is already the name of {} in your themes folder; pick \
                 another file name",
                other.file.as_deref().unwrap_or_default()
            )));
            return;
        }
        let tc = self.settings.theme_colors(dark);
        let text = match named::to_json(&name, &tc) {
            Ok(text) => text,
            Err(why) => {
                self.toast = Some(Toast::error(format!("Not saved: {why}")));
                return;
            }
        };
        if let Err(e) = dmm_shared::write_atomic(&path, text.as_bytes()) {
            self.toast = Some(Toast::error(format!(
                "Could not save {}: {e}",
                path.display()
            )));
            return;
        }
        if !in_folder {
            self.toast = Some(Toast::info(format!(
                "Saved {file}. Copy it into your themes folder to use it as a theme"
            )));
            return;
        }

        // The catalog takes the file at once, so the switch below lands on
        // it; the next read of the folder finds the same, and one already
        // under way, which might not, is dropped.
        self.theme_folder_writes += 1;
        let mut catalog = (*self.settings.user_themes).clone();
        catalog.themes.retain(|t| t.file.as_deref() != Some(&file));
        catalog.skipped.retain(|(f, _)| *f != file);
        catalog.themes.push(NamedTheme {
            name: name.clone(),
            dark,
            preset: tc.preset(),
            colors: tc.overrides().clone(),
            file: Some(file.clone()),
        });
        catalog.themes.sort_by(|a, b| a.file.cmp(&b.file));
        self.settings.user_themes = Arc::new(catalog);

        // Tweaks to a theme of this name are in the file now.
        self.settings
            .color_overrides
            .set_for_theme(&name, PaletteOverrides::default());
        self.settings.named_theme = Some(name.clone());
        self.settings.overrides.theme = None;
        self.settings.save();
        self.applied.ui_colors = None;
        self.toast = Some(Toast::info(format!("Saved {name} to your themes folder")));
    }
}

/// Whether `path` is a file directly in `dir`, through any links either
/// takes.
fn same_dir(path: &Path, dir: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    path.parent()
        .is_some_and(|parent| canon(parent) == canon(dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{HexColor, Settings, ThemeMode};
    use eframe::egui::Color32;

    /// A scratch themes folder, removed when dropped.
    struct Folder(PathBuf);

    impl Folder {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("dmm-gui-theme-save-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Light mode with its accent changed.
    fn customized_app() -> App {
        let mut settings = Settings {
            theme: ThemeMode::Light,
            auto_connect: false,
            ..Settings::default()
        };
        settings.color_overrides.light.accent = Some(HexColor(Color32::from_rgb(0x7A, 0x00, 0x62)));
        App::from_settings(settings, dmm_lib::Clock::real())
    }

    #[test]
    fn a_save_into_the_folder_becomes_the_theme_in_use() {
        let folder = Folder::new("into");
        let mut app = customized_app();
        app.save_theme(folder.0.join("plum-light"), false, Some(&folder.0));

        let text = std::fs::read_to_string(folder.0.join("plum-light.json")).unwrap();
        assert!(text.contains("\"name\": \"Plum Light\""), "{text}");
        let theme = app
            .settings
            .active_theme()
            .expect("switched to the saved theme");
        assert_eq!(theme.name, "Plum Light");
        assert!(!theme.dark);
        assert_eq!(
            app.settings.theme_colors(false).accent(),
            Color32::from_rgb(0x7A, 0x00, 0x62)
        );
        assert!(app.toast.as_ref().is_some_and(|t| !t.is_error));
    }

    #[test]
    fn a_save_elsewhere_only_writes_the_file() {
        let folder = Folder::new("folder");
        let elsewhere = Folder::new("elsewhere");
        let mut app = customized_app();
        app.save_theme(elsewhere.0.join("share.json"), false, Some(&folder.0));

        assert!(elsewhere.0.join("share.json").is_file());
        assert_eq!(app.settings.active_theme(), None);
        let toast = app.toast.as_ref().unwrap();
        assert!(
            toast.message.contains("Copy it into your themes folder"),
            "{}",
            toast.message
        );
    }

    /// Two files may not carry one theme name: the folder would skip one.
    #[test]
    fn a_save_under_another_files_theme_name_is_refused() {
        let folder = Folder::new("clash");
        let mut app = customized_app();
        app.save_theme(folder.0.join("plum.json"), false, Some(&folder.0));
        let saved = std::fs::read(folder.0.join("plum.json")).unwrap();
        app.settings.named_theme = None;
        app.settings.color_overrides.light.accent = Some(HexColor(Color32::from_rgb(1, 2, 3)));
        app.save_theme(folder.0.join("PLUM.json"), false, Some(&folder.0));
        assert!(app.toast.as_ref().is_some_and(|t| t.is_error));
        // Listed rather than probed: on a case-insensitive file system
        // (Windows, macOS) `PLUM.json` names `plum.json`.
        let files: Vec<_> = std::fs::read_dir(&folder.0)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(files, ["plum.json"]);
        assert_eq!(std::fs::read(folder.0.join("plum.json")).unwrap(), saved);
    }

    /// Saving a tweaked theme back over its own file keeps its name and
    /// puts the tweaks in the file.
    #[test]
    fn a_save_over_its_own_file_folds_the_tweaks_in() {
        let folder = Folder::new("own");
        let mut app = customized_app();
        app.save_theme(folder.0.join("plum.json"), false, Some(&folder.0));
        app.settings.color_overrides.named.insert(
            "Plum".into(),
            PaletteOverrides {
                text: Some(HexColor(Color32::from_rgb(0x20, 0x10, 0x40))),
                ..Default::default()
            },
        );
        app.save_theme(folder.0.join("plum.json"), false, Some(&folder.0));
        assert!(app.settings.color_overrides.named.is_empty());
        assert_eq!(app.settings.active_theme().unwrap().name, "Plum");
        assert_eq!(
            app.settings.theme_colors(false).text(),
            Color32::from_rgb(0x20, 0x10, 0x40)
        );
    }

    /// A name typed without `.json` is saved with it — but not over a file
    /// of that name, which the dialog never asked to replace.
    #[test]
    fn a_name_without_json_never_replaces_a_file_unasked() {
        let folder = Folder::new("unasked");
        let mut app = customized_app();
        app.save_theme(folder.0.join("plum"), false, Some(&folder.0));
        let saved = std::fs::read(folder.0.join("plum.json")).unwrap();

        app.settings.color_overrides.light.accent = Some(HexColor(Color32::from_rgb(1, 2, 3)));
        app.save_theme(folder.0.join("plum"), false, Some(&folder.0));
        assert_eq!(std::fs::read(folder.0.join("plum.json")).unwrap(), saved);
        assert!(app.toast.as_ref().is_some_and(|t| t.is_error));
    }
}
