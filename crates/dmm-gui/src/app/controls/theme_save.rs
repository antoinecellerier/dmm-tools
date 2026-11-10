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

use dmm_shared::tr;
use eframe::egui::{self, Ui};

use crate::app::App;
use crate::app::task::Task;
use crate::app::toast::Toast;
use crate::settings::{PaletteOverrides, ThemeChoice, ThemeMode};
use crate::theme::named::{self, NamedTheme};

/// The save dialog, while it is open.
#[derive(Default)]
pub(in crate::app) struct ThemeSave {
    /// The path picked, `None` when cancelled.
    dialog: Option<Task<Option<PathBuf>>>,
}

impl App {
    /// Whether the colours on screen differ from the preset's or the
    /// theme's own.
    pub(super) fn has_color_changes(&self, dark: bool) -> bool {
        self.settings
            .color_tweaks(dark)
            .is_some_and(|t| *t != PaletteOverrides::default())
    }

    /// The Save as theme button, under the colour swatches.
    pub(super) fn show_theme_save_row(&mut self, ui: &mut Ui) {
        if ui
            .button(tr!(self.l10n, "colors-save"))
            .on_hover_text(tr!(self.l10n, "colors-save.hover"))
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
        let file_type = tr!(self.l10n, "colors-save-file-type").into_owned();
        self.theme_save.dialog = Some(Task::spawn("theme-save-dialog", ctx, move || {
            let mut dialog = rfd::FileDialog::new()
                .set_file_name(file_name)
                .add_filter(file_type, &["json"]);
            if let Some(dir) = dir {
                dialog = dialog.set_directory(dir);
            }
            dialog.save_file()
        }));
    }

    /// Take in the save dialog's answer, and save.
    pub(in crate::app) fn poll_theme_save(&mut self) {
        let Some(answer) = self.theme_save.dialog.as_ref().and_then(Task::poll) else {
            return;
        };
        // The colours on screen when the answer comes: the theme may have
        // changed while the dialog was open.
        let dark = self.applied.theme != Some(ThemeMode::Light);
        self.theme_save.dialog = None;
        match answer {
            Ok(Some(path)) => {
                let folder = named::user_dir();
                self.save_theme(path, dark, folder.as_deref());
            }
            Ok(None) => {}
            Err(why) => {
                log::error!("the theme's save dialog failed: {why}");
                let message = tr!(self.l10n, "colors-save-dialog-failed");
                self.toast = Some(Toast::error(message));
            }
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
                let path = path.display().to_string();
                let message = tr!(self.l10n, "colors-save-exists", path = path);
                self.toast = Some(Toast::error(message));
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
            let message = tr!(
                self.l10n,
                "colors-save-name-taken",
                name = &name,
                file = other.file.as_deref().unwrap_or_default(),
            );
            self.toast = Some(Toast::error(message));
            return;
        }
        let tc = self.settings.theme_colors(dark);
        let text = match named::to_json(&name, &tc) {
            Ok(text) => text,
            Err(why) => {
                let reason = why.text(&self.l10n);
                let message = tr!(self.l10n, "colors-save-invalid", reason = reason);
                self.toast = Some(Toast::error(message));
                return;
            }
        };
        if let Err(e) = dmm_shared::write_atomic(&path, text.as_bytes()) {
            let message = tr!(
                self.l10n,
                "colors-save-failed",
                path = path.display().to_string(),
                error = e.to_string(),
            );
            self.toast = Some(Toast::error(message));
            return;
        }
        if !in_folder {
            let message = tr!(self.l10n, "colors-saved-elsewhere", file = &file);
            self.toast = Some(Toast::info(message));
            return;
        }

        // The catalog takes the file at once, so the switch below lands on
        // it; the next read of the folder finds the same, and one already
        // under way, which might not, is dropped.
        self.theme_scan.folder_written();
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
        // After the push, so the theme is found and brings its mode.
        self.settings.pick_theme(ThemeChoice::Named(name.clone()));
        self.applied.ui_colors = None;
        self.toast = Some(Toast::info(tr!(self.l10n, "colors-saved", name = &name)));
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
        app.settings.theme = ThemeMode::Dark;
        app.save_theme(folder.0.join("plum-light"), false, Some(&folder.0));

        let text = std::fs::read_to_string(folder.0.join("plum-light.json")).unwrap();
        assert!(text.contains("\"name\": \"Plum Light\""), "{text}");
        let theme = app
            .settings
            .active_theme()
            .expect("switched to the saved theme");
        assert_eq!(theme.name, "Plum Light");
        assert!(!theme.dark);
        // Its own mode is the fallback should its file go, as when it is
        // picked on the Theme row.
        assert_eq!(app.settings.theme, ThemeMode::Light);
        assert_eq!(
            app.settings.theme_colors(false).accent(),
            Color32::from_rgb(0x7A, 0x00, 0x62)
        );
        assert!(app.toast.as_ref().is_some_and(|t| !t.is_error()));
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
        assert!(app.toast.as_ref().is_some_and(|t| t.is_error()));
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
        assert!(app.toast.as_ref().is_some_and(|t| t.is_error()));
    }
}
