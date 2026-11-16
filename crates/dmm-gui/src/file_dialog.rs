//! The system's open and save dialogs: `rfd`, or on Android, where it has
//! no backend, a stand-in that never asks. Same path either way, so
//! Import…, Export… and saving a theme need no `cfg`. On Android a save
//! goes, under the name offered, to the folder the dialog would have opened
//! in, or to [`set_save_folder`]'s, replacing a file of that name unasked
//! (a caller that must not offers an [`unused`] name); opening a file asks
//! whatever
//! [`set_file_picker`] was given, the system's document picker, reached
//! through the activity's Java side.

#[cfg(not(target_os = "android"))]
pub(crate) use rfd::FileDialog;

#[cfg(target_os = "android")]
pub use absent::FilePicker;
#[cfg(target_os = "android")]
pub(crate) use absent::{FileDialog, set_file_picker, set_save_folder};

/// `path`, or while a file is there, the first of `name-2.ext`,
/// `name-3.ext`, … that is free: a name to offer where no dialog asks
/// before replacing a file.
#[cfg(any(test, target_os = "android"))]
pub(crate) fn unused(path: std::path::PathBuf) -> std::path::PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().map(|x| x.to_string_lossy());
    (2..1000)
        .map(|n| match &ext {
            Some(ext) => path.with_file_name(format!("{stem}-{n}.{ext}")),
            None => path.with_file_name(format!("{stem}-{n}")),
        })
        .find(|candidate| !candidate.exists())
        .unwrap_or(path)
}

#[cfg(target_os = "android")]
mod absent {
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    /// Where a save goes when the caller names no folder.
    static SAVE_FOLDER: OnceLock<PathBuf> = OnceLock::new();

    /// Name the folder saves go to unless the caller names one. The first
    /// call stands.
    pub(crate) fn set_save_folder(dir: PathBuf) {
        let _ = SAVE_FOLDER.set(dir);
    }

    /// Asks the user for a file to open and waits for the answer: `None`
    /// when they picked none. Called on the dialog's own thread.
    pub type FilePicker = Box<dyn Fn() -> Option<PathBuf> + Send + Sync>;

    static FILE_PICKER: OnceLock<FilePicker> = OnceLock::new();

    /// Name what opens a file. The first call stands.
    pub(crate) fn set_file_picker(picker: FilePicker) {
        let _ = FILE_PICKER.set(picker);
    }

    /// `rfd::FileDialog`, down to what the app calls.
    #[derive(Default)]
    pub(crate) struct FileDialog {
        name: Option<String>,
        dir: Option<PathBuf>,
    }

    impl FileDialog {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        pub(crate) fn add_filter(
            self,
            _name: impl ToString,
            _extensions: &[impl ToString],
        ) -> Self {
            self
        }

        pub(crate) fn set_file_name(mut self, name: impl Into<String>) -> Self {
            self.name = Some(name.into());
            self
        }

        pub(crate) fn set_directory(mut self, dir: impl AsRef<Path>) -> Self {
            self.dir = Some(dir.as_ref().to_path_buf());
            self
        }

        pub(crate) fn pick_file(self) -> Option<PathBuf> {
            FILE_PICKER.get().and_then(|pick| pick())
        }

        pub(crate) fn save_file(self) -> Option<PathBuf> {
            let dir = self.dir.or_else(|| SAVE_FOLDER.get().cloned())?;
            Some(dir.join(self.name?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::unused;

    #[test]
    fn a_save_takes_the_next_free_name() {
        let dir = std::env::temp_dir().join(format!("dmm-gui-unused-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = dir.join("my-theme.json");
        assert_eq!(unused(first.clone()), first);
        std::fs::write(&first, "").unwrap();
        let second = dir.join("my-theme-2.json");
        assert_eq!(unused(first.clone()), second);
        std::fs::write(&second, "").unwrap();
        assert_eq!(unused(first), dir.join("my-theme-3.json"));

        let bare = dir.join("notes");
        std::fs::write(&bare, "").unwrap();
        assert_eq!(unused(bare), dir.join("notes-2"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
