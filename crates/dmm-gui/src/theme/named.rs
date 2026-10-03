//! Named themes: a palette and the light or dark mode it is drawn in, read
//! from a JSON file.
//!
//! A theme is an override set laid over the Default preset, in the same
//! field names `settings.json` uses for per-colour overrides, so a colour it
//! leaves out is Default's. It fixes its mode rather than pairing a dark and a
//! light palette: "Midnight" has no light form worth having.
//!
//! The built-in themes are files in `crates/dmm-gui/themes/`, embedded at
//! compile time and parsed on first use, so they work for a bare binary and
//! stay under the palette's contrast tests. More are read from the `themes`
//! folder beside `settings.json`, where a user drops a file to add one or to
//! replace a built-in of the same name.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::settings::{ColorPreset, PaletteOverrides};
use crate::theme::{ThemeColors, contrast};

/// What a theme file holds.
#[derive(Deserialize)]
struct ThemeFile {
    name: String,
    mode: Mode,
    /// The preset under the colours; Default when left out.
    #[serde(default)]
    preset: ColorPreset,
    #[serde(default)]
    colors: PaletteOverrides,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Dark,
    Light,
}

/// A theme, parsed and checked.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NamedTheme {
    /// Shown on its chip, saved as `named_theme` and matched by `--theme`.
    pub(crate) name: String,
    /// The mode it is drawn in, whatever the Theme row's Dark/Light says.
    pub(crate) dark: bool,
    /// The preset its colours are laid over: Default for the built-ins, the
    /// one a saved theme was made from.
    pub(crate) preset: ColorPreset,
    /// Its colours, over the preset.
    pub(crate) colors: PaletteOverrides,
    /// The file a user theme was read from; `None` for a built-in.
    pub(crate) file: Option<String>,
}

/// The user's themes folder, as last read.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ThemeCatalog {
    /// The themes it offers, in file-name order.
    pub(crate) themes: Vec<NamedTheme>,
    /// The files it holds that are not offered, by name, with why.
    pub(crate) skipped: Vec<(String, String)>,
}

/// The most theme files read from the folder: a bound on what a scan can
/// cost, far beyond any row of chips anyone would want.
const MAX_FILES: usize = 100;

/// The largest theme file read, in bytes. A theme with every colour set is
/// under 2 KiB.
const MAX_FILE_BYTES: u64 = 64 * 1024;

/// The longest name a theme may have, in characters: a chip, not a caption.
const MAX_NAME_CHARS: usize = 32;

/// The least contrast a theme's text may have on its panel and its button
/// captions on their fill. Lower and the chips that would switch it away are
/// unreadable too. Well under AA on purpose: this is a floor against a
/// broken file, not a design bar — the built-ins are held to AA by the
/// palette tests.
const MIN_READABLE_CONTRAST: f64 = 3.0;

/// The built-in theme files, in chip order.
const BUILTIN_FILES: &[(&str, &str)] = &[
    (
        "bubble-gum.json",
        include_str!("../../themes/bubble-gum.json"),
    ),
    ("desert.json", include_str!("../../themes/desert.json")),
    ("midnight.json", include_str!("../../themes/midnight.json")),
    ("phosphor.json", include_str!("../../themes/phosphor.json")),
];

/// The built-in themes. A file that fails to parse is logged and left out;
/// `every_builtin_theme_parses` keeps that from shipping.
pub(crate) fn builtin() -> &'static [NamedTheme] {
    static BUILTIN: LazyLock<Vec<NamedTheme>> = LazyLock::new(|| {
        BUILTIN_FILES
            .iter()
            .filter_map(|(file, text)| {
                parse(text)
                    .inspect_err(|e| log::error!("built-in theme {file} is broken: {e}"))
                    .ok()
            })
            .collect()
    });
    &BUILTIN
}

/// What Save as theme writes: the shape [`ThemeFile`] reads.
#[derive(Serialize)]
struct ThemeFileOut<'a> {
    name: &'a str,
    mode: &'static str,
    #[serde(skip_serializing_if = "is_default_preset")]
    preset: ColorPreset,
    colors: &'a PaletteOverrides,
}

fn is_default_preset(preset: &ColorPreset) -> bool {
    *preset == ColorPreset::Default
}

/// A theme file's text for the palette `tc`: its preset and the colours set
/// over it, exactly — not every colour resolved, which would count the ones
/// left to egui (a preset's border, its accent) as set and so redraw the
/// chrome that follows them. Checked the way the folder will read it back:
/// a file the folder would skip is never written.
pub(crate) fn to_json(name: &str, tc: &ThemeColors) -> Result<String, String> {
    let file = ThemeFileOut {
        name,
        mode: if tc.is_dark() { "dark" } else { "light" },
        preset: tc.preset(),
        colors: tc.overrides(),
    };
    let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())? + "\n";
    parse(&text)?;
    Ok(text)
}

/// The file name a theme is saved under, without `.json`: its `--theme`
/// spelling, letters, digits and dashes only.
pub(crate) fn file_stem(name: &str) -> String {
    let stem: String = flag_name(name)
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect();
    if stem.is_empty() {
        "theme".to_string()
    } else {
        stem
    }
}

/// The name a theme saved as `file` takes: its stem in words, each
/// capitalised if the stem had no capitals of its own, so
/// `bubble-gum-pop.json` is "Bubble Gum Pop" and `MyTheme.json` stays
/// "MyTheme". Cut to the longest name a theme may have.
pub(crate) fn name_from_file(file: &str) -> String {
    let stem = Path::new(file)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let lower = !stem.chars().any(char::is_uppercase);
    let words = stem
        .split(['-', '_', ' '])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) if lower => first.to_uppercase().chain(chars).collect(),
                _ => w.to_string(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ");
    words
        .chars()
        .take(MAX_NAME_CHARS)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Parse and check one theme file.
fn parse(text: &str) -> Result<NamedTheme, String> {
    let file: ThemeFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let name = file.name.trim().to_string();
    check_name(&name)?;
    let theme = NamedTheme {
        name,
        dark: matches!(file.mode, Mode::Dark),
        preset: file.preset,
        colors: file.colors,
        file: None,
    };
    check_readable(&theme)?;
    Ok(theme)
}

/// A name has to fit a chip and read as what it is: no control characters,
/// and none of the bidirectional overrides that would make one theme's chip
/// or log line show another's name.
pub(crate) fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("the name is empty".into());
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(format!("the name is over {MAX_NAME_CHARS} characters"));
    }
    // Nothing invisible — controls, text-direction overrides, zero-width
    // characters — that could make one theme's chip read as another's, or
    // as Dark; and no `$` or backtick, which a shell sourcing a file the
    // name was copied into would act on.
    if name
        .chars()
        .any(|c| unprintable(c) || matches!(c, '$' | '`'))
    {
        return Err("the name has characters a theme name can't use".into());
    }
    let key = normalize(name);
    if ["dark", "light", "system"].contains(&key.as_str()) {
        return Err(format!("{name:?} is taken by the Theme row"));
    }
    Ok(())
}

/// A theme whose text can't be read on its panel, or whose button captions
/// can't be read on their fill, would leave the user unable to read the very
/// chips that switch it off.
fn check_readable(theme: &NamedTheme) -> Result<(), String> {
    let tc = ThemeColors::new(theme.dark, theme.preset, &theme.colors);
    for (what, fg, ground) in [
        ("text on the background", tc.text(), tc.background()),
        ("button text on the button", tc.button_text(), tc.button()),
    ] {
        // `contrast` reads no alpha: a transparent text colour would pass.
        if !fg.is_opaque() || !ground.is_opaque() {
            return Err(format!("{what} is not opaque"));
        }
        let ratio = contrast(fg, ground);
        if ratio < MIN_READABLE_CONTRAST {
            return Err(format!(
                "{what} is {ratio:.1}:1, under the {MIN_READABLE_CONTRAST}:1 floor"
            ));
        }
    }
    Ok(())
}

/// Whether `c` draws as nothing, or as something other than itself: a
/// control, a format character such as a zero-width space or a
/// text-direction override, a line separator. Rust's debug escaping knows
/// them all.
fn unprintable(c: char) -> bool {
    c.escape_debug().next() == Some('\\') && !matches!(c, '\'' | '"' | '\\')
}

/// `text` with every unprintable character shown as `?`: for a file name or
/// a parse error, which reach the log and a tooltip.
fn printable(text: &str) -> String {
    text.chars()
        .map(|c| if unprintable(c) { '?' } else { c })
        .collect()
}

/// A name as `--theme` matches it: case, spaces, `-` and `_` ignored, so
/// `bubble-gum` finds "Bubble Gum".
pub(crate) fn normalize(name: &str) -> String {
    name_key(name).collect()
}

fn name_key(name: &str) -> impl Iterator<Item = char> + '_ {
    name.chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
}

/// Whether two names are one theme's, as [`normalize`] has it — without
/// building either: the palette asks many times a frame.
pub(crate) fn same_name(a: &str, b: &str) -> bool {
    name_key(a).eq(name_key(b))
}

/// A name as `--theme` would be typed: lower case, words joined by `-`.
pub(crate) fn flag_name(name: &str) -> String {
    name.split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("-")
}

/// The themes the Theme row offers, in its order: the built-ins, each
/// replaced by a user theme of the same name if there is one, then the rest
/// of the user's.
pub(crate) fn listed(user: &[NamedTheme]) -> Vec<&NamedTheme> {
    let shadow = |t: &NamedTheme| user.iter().find(|u| same_name(&u.name, &t.name));
    let builtins = builtin().iter().map(|t| shadow(t).unwrap_or(t));
    let others = user
        .iter()
        .filter(|u| !builtin().iter().any(|t| same_name(&t.name, &u.name)));
    builtins.chain(others).collect()
}

/// The theme named `name` — saved, picked on the row, or typed after
/// `--theme` — the user's first. Matched as the Theme row lists names, case,
/// spaces, `-` and `_` aside, so `bubble-gum` finds "Bubble Gum" and a user
/// file named "midnight" that takes Midnight's place answers to "Midnight".
pub(crate) fn find<'a>(name: &str, user: &'a [NamedTheme]) -> Option<&'a NamedTheme> {
    user.iter()
        .chain(builtin())
        .find(|t| same_name(&t.name, name))
}

/// The themes folder beside `settings.json`. `None` under test, as
/// `Settings::config_path` is, so the suite never reads a developer's own.
pub(crate) fn user_dir() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    // Asked for on every frame Settings is open; the platform lookup behind
    // it is a system call on Windows.
    static DIR: LazyLock<Option<PathBuf>> =
        LazyLock::new(|| Some(dmm_shared::config_path()?.parent()?.join("themes")));
    DIR.clone()
}

/// Read the user's themes folder, if there is one.
pub(crate) fn discover_user() -> ThemeCatalog {
    user_dir().map(|dir| discover(&dir)).unwrap_or_default()
}

/// Read every `*.json` file in `dir`, in file-name order. A folder that
/// doesn't exist holds no themes; a file that can't be offered is listed in
/// `skipped` with the reason, for the Theme row to show.
pub(crate) fn discover(dir: &Path) -> ThemeCatalog {
    let mut catalog = ThemeCatalog::default();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return catalog;
    };
    // A folder of a million entries is listed only so far.
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("json"))
        })
        .take(100 * MAX_FILES)
        .collect();
    paths.sort();
    if paths.len() > MAX_FILES {
        let more = paths.len() - MAX_FILES;
        catalog.skipped.push((
            format!("{more} more"),
            format!("over the {MAX_FILES}-file limit"),
        ));
        paths.truncate(MAX_FILES);
    }
    for path in &paths {
        let file = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        // The file name is shown and logged, and kept to save over it.
        if file.chars().any(unprintable) {
            let reason = "the file name has characters that don't print".to_string();
            catalog.skipped.push((printable(&file), reason));
            continue;
        }
        match read_theme(path) {
            Ok(mut theme) => {
                if let Some(taken) = catalog
                    .themes
                    .iter()
                    .find(|t| same_name(&t.name, &theme.name))
                {
                    let reason = format!(
                        "{:?} is already the name of {}",
                        theme.name,
                        taken.file.as_deref().unwrap_or_default()
                    );
                    catalog.skipped.push((file, reason));
                } else {
                    theme.file = Some(file);
                    catalog.themes.push(theme);
                }
            }
            Err(reason) => catalog.skipped.push((file, printable(&reason))),
        }
    }
    catalog
}

/// Log, at WARN, each file `catalog` skips that `previous` did not, so a
/// rescan each time Settings opens doesn't repeat itself.
pub(crate) fn log_skipped(catalog: &ThemeCatalog, previous: Option<&ThemeCatalog>) {
    for skipped in &catalog.skipped {
        if previous.is_none_or(|p| !p.skipped.contains(skipped)) {
            let (file, reason) = skipped;
            log::warn!("theme file {file:?} skipped: {reason}");
        }
    }
}

/// Read and parse one theme file. Only a regular file, after following
/// links, and only its first [`MAX_FILE_BYTES`]: a link to a device or a
/// pipe is never read, and an oversized file never read whole.
fn read_theme(path: &Path) -> Result<NamedTheme, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("not a file".into());
    }
    let mut text = String::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_FILE_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if text.len() as u64 > MAX_FILE_BYTES {
        return Err(format!("over {} KiB", MAX_FILE_BYTES / 1024));
    }
    parse(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::HexColor;
    use crate::theme::PaletteField;
    use eframe::egui::Color32;

    #[test]
    fn every_builtin_theme_parses() {
        for (file, text) in BUILTIN_FILES {
            if let Err(e) = parse(text) {
                panic!("{file}: {e}");
            }
        }
        assert_eq!(builtin().len(), BUILTIN_FILES.len());
    }

    /// The palette tests check contrast with `contrast()`, which ignores
    /// alpha, so a translucent colour would pass them while drawing fainter.
    /// And a built-in sets every colour: one left on Default's would be
    /// Default's light or dark grey on a tinted panel.
    #[test]
    fn builtin_themes_set_every_colour_opaque() {
        for theme in builtin() {
            let mut colors = theme.colors.clone();
            for &field in PaletteField::ALL {
                let slot = field.override_slot(&mut colors);
                let Some(HexColor(c)) = *slot else {
                    panic!("{}: {field:?} is not set", theme.name);
                };
                assert!(
                    c.is_opaque(),
                    "{}: {field:?} {c:?} is translucent",
                    theme.name
                );
            }
        }
    }

    #[test]
    fn builtin_names_are_unique_and_flaggable() {
        for (k, theme) in builtin().iter().enumerate() {
            assert_eq!(find(&theme.name, &[]), Some(theme));
            assert_eq!(find(&flag_name(&theme.name), &[]), Some(theme));
            for other in &builtin()[..k] {
                assert_ne!(normalize(&theme.name), normalize(&other.name));
            }
        }
    }

    #[test]
    fn flag_matching_ignores_case_spaces_dashes_and_underscores() {
        let bubble = find("Bubble Gum", &[]).unwrap();
        for input in ["bubble-gum", "Bubble Gum", "BUBBLE_GUM", "bubblegum"] {
            assert_eq!(find(input, &[]), Some(bubble), "{input}");
        }
        assert_eq!(flag_name("Bubble Gum"), "bubble-gum");
        assert_eq!(find("bubble-gun", &[]), None);
    }

    #[test]
    fn a_theme_file_reads_its_name_mode_and_colours() {
        let theme =
            parse(r##"{"name": " Dusk ", "mode": "light", "colors": {"accent": "#123456"}}"##)
                .unwrap();
        assert_eq!(theme.name, "Dusk");
        assert!(!theme.dark);
        assert_eq!(
            theme.colors.accent,
            Some(HexColor(Color32::from_rgb(0x12, 0x34, 0x56)))
        );
        assert_eq!(theme.colors.text, None);
        // Colours are optional, and a key from a later version is ignored.
        let theme = parse(r#"{"name": "Plain", "mode": "dark", "future": 1}"#).unwrap();
        assert_eq!(theme.colors, PaletteOverrides::default());
    }

    #[test]
    fn a_bad_theme_file_says_why() {
        for (text, why) in [
            (r#"{"name": "X", "mode": "dim"}"#, "unknown variant"),
            (r#"{"mode": "dark"}"#, "missing field `name`"),
            (
                r##"{"name": "X", "mode": "dark", "colors": {"text": "#12"}}"##,
                "#RRGGBB",
            ),
            (r#"{"name": "  ", "mode": "dark"}"#, "empty"),
            (
                r#"{"name": "A name far too long for any chip at all", "mode": "dark"}"#,
                "over 32",
            ),
            // JSON's escape, so the source holds no right-to-left override.
            (
                r#"{"name": "Evil\u202Edrawkcab", "mode": "dark"}"#,
                "can't use",
            ),
            (r#"{"name": "Two\nlines", "mode": "dark"}"#, "can't use"),
            (r#"{"name": "Dar\u200Bk", "mode": "dark"}"#, "can't use"),
            (r#"{"name": "$(rm -rf ~)", "mode": "dark"}"#, "can't use"),
            (
                r##"{"name": "Clear", "mode": "light", "colors": {"text": "#00000000"}}"##,
                "not opaque",
            ),
            (r#"{"name": "dark", "mode": "dark"}"#, "taken"),
            (r#"{"name": "S Y S T E M", "mode": "dark"}"#, "taken"),
            (
                r##"{"name": "Ghost", "mode": "dark",
                     "colors": {"text": "#1B1B1B", "background": "#1B1B1B"}}"##,
                "text on the background",
            ),
            (
                r##"{"name": "Ghost", "mode": "light",
                     "colors": {"text": "#505050", "button": "#505050"}}"##,
                "button text on the button",
            ),
        ] {
            let err = parse(text).unwrap_err();
            assert!(err.contains(why), "{text}: {err:?} lacks {why:?}");
        }
    }

    /// A scratch themes folder, removed when dropped.
    struct Folder(PathBuf);

    impl Folder {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("dmm-gui-themes-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn put(&self, file: &str, text: &str) {
            std::fs::write(self.0.join(file), text).unwrap();
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn reason<'a>(catalog: &'a ThemeCatalog, file: &str) -> &'a str {
        catalog
            .skipped
            .iter()
            .find(|(f, _)| f == file)
            .map(|(_, r)| r.as_str())
            .unwrap_or_else(|| panic!("{file} not skipped: {catalog:?}"))
    }

    #[test]
    fn a_missing_folder_holds_no_themes() {
        let folder = Folder::new("missing");
        assert_eq!(discover(&folder.0.join("nope")), ThemeCatalog::default());
    }

    #[test]
    fn the_folder_offers_its_good_files_and_says_why_it_skips_the_rest() {
        let folder = Folder::new("mixed");
        folder.put("b-dusk.json", r#"{"name": "Dusk", "mode": "dark"}"#);
        folder.put("a-dawn.JSON", r#"{"name": "Dawn", "mode": "light"}"#);
        folder.put("c-dusk-again.json", r#"{"name": "dusk", "mode": "light"}"#);
        folder.put(
            "d-bad-hex.json",
            r##"{"name": "X", "mode": "dark", "colors": {"text": "#zz"}}"##,
        );
        folder.put("e-big.json", &" ".repeat(MAX_FILE_BYTES as usize + 1));
        folder.put("notes.txt", "not a theme");
        std::fs::create_dir(folder.0.join("f-folder.json")).unwrap();

        let catalog = discover(&folder.0);
        let names: Vec<&str> = catalog.themes.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["Dawn", "Dusk"], "file-name order");
        assert_eq!(catalog.themes[0].file.as_deref(), Some("a-dawn.JSON"));
        assert!(reason(&catalog, "c-dusk-again.json").contains("b-dusk.json"));
        assert!(reason(&catalog, "d-bad-hex.json").contains("hex"));
        assert!(reason(&catalog, "e-big.json").contains("KiB"));
        assert_eq!(reason(&catalog, "f-folder.json"), "not a file");
        assert_eq!(catalog.skipped.len(), 4, "notes.txt is not a theme file");
    }

    /// A user file named like a built-in takes its place on the Theme row
    /// and under its saved name; the other user themes follow the built-ins.
    #[test]
    fn a_user_file_replaces_the_built_in_of_its_name() {
        let folder = Folder::new("shadow");
        folder.put(
            "midnight.json",
            r##"{"name": "Midnight", "mode": "dark", "colors": {"accent": "#FF00FF"}}"##,
        );
        folder.put("zest.json", r#"{"name": "Zest", "mode": "light"}"#);
        let catalog = discover(&folder.0);

        let mine = find("Midnight", &catalog.themes).unwrap();
        assert_eq!(mine.file.as_deref(), Some("midnight.json"));
        assert_eq!(find("midnight", &catalog.themes), Some(mine));
        let listed = listed(&catalog.themes);
        assert_eq!(listed.len(), builtin().len() + 1);
        assert!(listed.iter().any(|t| std::ptr::eq(*t, mine)));
        assert_eq!(listed.last().unwrap().name, "Zest");
    }

    #[test]
    fn the_folder_reads_at_most_its_limit_of_files() {
        let folder = Folder::new("many");
        for k in 0..=MAX_FILES {
            folder.put(
                &format!("t{k:03}.json"),
                &format!(r#"{{"name": "T{k}", "mode": "dark"}}"#),
            );
        }
        let catalog = discover(&folder.0);
        assert_eq!(catalog.themes.len(), MAX_FILES);
        assert!(reason(&catalog, "1 more").contains("limit"));
    }

    /// A saved theme reads back as the palette it was saved from, under its
    /// name and mode, and lands in a file named like its `--theme` spelling.
    #[test]
    fn a_saved_theme_reads_back_as_the_palette_it_was_saved_from() {
        let midnight = find("Midnight", &[]).unwrap();
        let tc = ThemeColors::new(true, ColorPreset::Default, &midnight.colors);
        let text = to_json("Night Shift", &tc).unwrap();
        let back = parse(&text).unwrap();
        assert_eq!(back.name, "Night Shift");
        assert!(back.dark);
        assert_eq!(back.preset, ColorPreset::Default);
        assert_eq!(back.colors, midnight.colors);
        assert!(!text.contains("preset"), "Default goes unsaid: {text}");

        // A preset with one colour changed comes back as that preset with
        // that colour: its border and the rest still the preset's own.
        let accent = PaletteOverrides {
            accent: Some(HexColor(Color32::from_rgb(0x7A, 0x00, 0x62))),
            ..Default::default()
        };
        let tc = ThemeColors::new(false, ColorPreset::HighContrast, &accent);
        let back = parse(&to_json("Loud", &tc).unwrap()).unwrap();
        assert_eq!(back.preset, ColorPreset::HighContrast);
        assert_eq!(back.colors, accent);
        assert_eq!(file_stem("Night Shift"), "night-shift");
        assert_eq!(file_stem("A/B: test"), "ab-test");
        assert_eq!(file_stem("!!!"), "theme");
    }

    /// The readability floor holds for a save too, so a file is never
    /// written that the folder would then skip.
    #[test]
    fn an_unreadable_palette_is_not_saved() {
        let colors = PaletteOverrides {
            text: Some(HexColor(Color32::from_gray(30))),
            background: Some(HexColor(Color32::from_gray(25))),
            ..Default::default()
        };
        let tc = ThemeColors::new(true, ColorPreset::Default, &colors);
        let err = to_json("Murk", &tc).unwrap_err();
        assert!(err.contains("text on the background"), "{err}");
    }

    /// With every graph element on at once, each has to read as its own
    /// colour — see `links::too_close`, which the editor warns with.
    #[test]
    fn a_built_in_theme_keeps_its_graph_colours_apart() {
        for theme in builtin() {
            let tc = ThemeColors::new(theme.dark, theme.preset, &theme.colors);
            for &field in crate::theme::links::GRAPH_SERIES {
                if let Some((other, d, floor)) = crate::theme::links::too_close(&tc, field) {
                    panic!(
                        "{}: {field:?} and {other:?} are ΔE {d:.1} apart, under {floor}",
                        theme.name
                    );
                }
            }
        }
    }

    #[test]
    fn a_file_name_names_its_theme() {
        assert_eq!(name_from_file("bubble-gum-pop.json"), "Bubble Gum Pop");
        assert_eq!(name_from_file("my_theme.json"), "My Theme");
        assert_eq!(name_from_file("MyTheme.json"), "MyTheme");
        assert_eq!(name_from_file("night-SHIFT.json"), "night SHIFT");
        assert_eq!(
            name_from_file(&format!("{}.json", "a".repeat(40)))
                .chars()
                .count(),
            MAX_NAME_CHARS
        );
    }

    /// A parse error can carry what the file holds — a mode's value, say —
    /// and goes to the log and a tooltip: nothing in it may draw as a line
    /// break or turn the text around.
    #[test]
    fn a_skip_reason_prints_as_it_reads() {
        let folder = Folder::new("reasons");
        folder.put(
            "forged.json",
            "{\"name\": \"X\", \"mode\": \"x\\n[WARN] forged\\u202E\"}",
        );
        let catalog = discover(&folder.0);
        let why = reason(&catalog, "forged.json");
        assert!(!why.chars().any(unprintable), "{why:?}");
        assert!(why.contains("?[WARN] forged?"), "{why:?}");
    }
}
