//! Where a `read` run writes its readings.
//!
//! Stdout and `-o FILE` are open before the first reading. A bare `-o` names
//! its file after the meter, the mode and the moment the run started — the
//! same name [`dmm-gui`'s Export… opens on](dmm_shared::export::default_name)
//! — so it cannot be created until a reading has arrived, and the header waits
//! in memory until then.

use chrono::{DateTime, Local};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// How many names a self-named file may step through before giving up:
/// itself, then `-2` up to `-99`. The name has second resolution, so this is
/// a run started every second for a minute and a half.
const MOST_SAME_NAME: u32 = 99;

/// What `-o` asked for.
pub(crate) enum Destination {
    /// No `-o`: the readings go to stdout.
    Stdout,
    /// `-o FILE`.
    Path(PathBuf),
    /// `-o` with no file name: the run names the file itself, with the
    /// extension of the format it writes.
    Auto { extension: &'static str },
}

/// The stream a run writes through.
pub(crate) struct Writer {
    sink: Sink,
    /// Set for a bare `-o` only.
    auto: Option<Auto>,
}

enum Sink {
    Open(Box<dyn Write>),
    /// A bare `-o` before the first reading: the header waits here.
    Unnamed(Vec<u8>),
}

/// A file the run names itself.
struct Auto {
    /// The meter, and the extension of the format being written.
    meter: String,
    extension: &'static str,
    /// What the first reading settled, once one has arrived.
    named: Option<Named>,
}

/// Where a finished run's readings ended up.
pub(crate) enum Wrote {
    /// Stdout, or the file named on the command line: the user typed it, so
    /// there is nothing to tell them.
    AsAsked,
    /// The file the run named itself.
    Named(PathBuf),
    /// A bare `-o` whose run never got a reading: the name comes from the
    /// first one, so there was nothing to name a file after and nothing to
    /// put in it.
    Nothing,
}

struct Named {
    path: PathBuf,
    start: DateTime<Local>,
    /// The mode the name carries, and whether every reading since has stayed
    /// in it — a run that crossed a function switch is no one mode's, so the
    /// name loses that segment when the run ends.
    mode: String,
    one_mode: bool,
}

impl Writer {
    /// Open the stream. `meter` names the meter the readings come from, for a
    /// file the run has to name itself.
    pub(crate) fn new(destination: Destination, meter: &str) -> io::Result<Self> {
        let (sink, auto) = match destination {
            Destination::Stdout => (Sink::Open(Box::new(io::stdout().lock())), None),
            Destination::Path(path) => (Sink::Open(Box::new(create(&path)?)), None),
            Destination::Auto { extension } => (
                Sink::Unnamed(Vec::new()),
                Some(Auto {
                    meter: meter.to_string(),
                    extension,
                    named: None,
                }),
            ),
        };
        Ok(Self { sink, auto })
    }

    /// Take in a reading before it is written: the first one names a bare
    /// `-o`'s file, and a later one in another mode drops the mode from it.
    pub(crate) fn saw(&mut self, mode: &str, at: DateTime<Local>) -> io::Result<()> {
        let Some(auto) = &mut self.auto else {
            return Ok(());
        };
        let Some(named) = &mut auto.named else {
            let (path, file) = create_new(&PathBuf::from(dmm_shared::export::default_name(
                &auto.meter,
                Some(mode),
                at,
                auto.extension,
            )))?;
            let mut file = BufWriter::new(file);
            if let Sink::Unnamed(pending) = &self.sink {
                file.write_all(pending).map_err(|e| at_path(&path, e))?;
            }
            self.sink = Sink::Open(Box::new(file));
            auto.named = Some(Named {
                path,
                start: at,
                mode: mode.to_string(),
                one_mode: true,
            });
            return Ok(());
        };
        named.one_mode &= named.mode == mode;
        Ok(())
    }

    /// Close the run and say where the readings ended up.
    pub(crate) fn finish(self) -> io::Result<Wrote> {
        let Self { sink, auto } = self;
        // Closed before the rename below: Windows refuses to rename a file
        // that is still open.
        if let Sink::Open(mut open) = sink {
            open.flush()?;
            drop(open);
        }
        let Some(auto) = auto else {
            return Ok(Wrote::AsAsked);
        };
        let Auto {
            meter,
            extension,
            named: Some(named),
        } = auto
        else {
            return Ok(Wrote::Nothing);
        };
        if named.one_mode {
            return Ok(Wrote::Named(named.path));
        }
        let target = PathBuf::from(dmm_shared::export::default_name(
            &meter,
            None,
            named.start,
            extension,
        ));
        // The name is taken before the rename, not just tested: `fs::rename`
        // replaces whatever is at the target, and a run started in the same
        // second as this one is what would be there.
        let (renamed, _) = create_new(&target)?;
        std::fs::rename(&named.path, &renamed).map_err(|e| at_path(&renamed, e))?;
        Ok(Wrote::Named(renamed))
    }
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &mut self.sink {
            Sink::Open(open) => open.write(buf),
            Sink::Unnamed(pending) => pending.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.sink {
            Sink::Open(open) => open.flush(),
            // Nothing to flush to yet: the file is still unnamed.
            Sink::Unnamed(_) => Ok(()),
        }
    }
}

fn create(path: &Path) -> io::Result<BufWriter<File>> {
    File::create(path)
        .map(BufWriter::new)
        .map_err(|e| at_path(path, e))
}

/// Create a file the run named itself, stepping aside rather than writing
/// over one that is there: the name has second resolution, so two runs
/// started in the same second ask for it, and the second would truncate the
/// first's readings away. An `-o FILE` the user typed still overwrites — they
/// named that file.
pub(crate) fn create_new(path: &Path) -> io::Result<(PathBuf, File)> {
    for n in 1..=MOST_SAME_NAME {
        let candidate = if n == 1 {
            path.to_path_buf()
        } else {
            beside(path, n)
        };
        match File::options()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(at_path(&candidate, e)),
        }
    }
    Err(at_path(
        path,
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "this and the {} names beside it are taken",
                MOST_SAME_NAME - 1
            ),
        ),
    ))
}

/// The `n`th name to try once `path` is taken: `…-2.csv`, then `…-3.csv`.
fn beside(path: &Path, n: u32) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    match path.extension() {
        Some(extension) => {
            path.with_file_name(format!("{stem}-{n}.{}", extension.to_string_lossy()))
        }
        None => path.with_file_name(format!("{stem}-{n}")),
    }
}

/// Name the file in an io failure: "permission denied" on its own says
/// nothing about which file the run could not write.
fn at_path(path: &Path, e: io::Error) -> io::Error {
    io::Error::new(e.kind(), format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bare `-o` is named after its first reading, so a run that never gets
    /// one has no file to report — the caller says so rather than leaving the
    /// user looking for a path the reference promised.
    ///
    /// The file cases are exercised end to end in `tests/replay.rs`: a
    /// self-named file lands in the working directory, which a test process
    /// of its own is the only way to pin down.
    #[test]
    fn a_run_with_no_reading_names_no_file() {
        let writer = Writer::new(Destination::Auto { extension: "csv" }, "UT61E+")
            .expect("the writer opens");
        assert!(
            matches!(writer.finish().expect("the run closes"), Wrote::Nothing),
            "a bare -o that saw nothing wrote nothing"
        );

        // Stdout and a file the user named are theirs to know about.
        let writer = Writer::new(Destination::Stdout, "UT61E+").expect("the writer opens");
        assert!(matches!(
            writer.finish().expect("the run closes"),
            Wrote::AsAsked
        ));
    }

    /// The names a taken one steps aside to, in order.
    #[test]
    fn a_taken_name_steps_aside_by_number() {
        assert_eq!(
            beside(Path::new("measurements-UT61E+-2026-09-15_14-30-05.csv"), 2),
            Path::new("measurements-UT61E+-2026-09-15_14-30-05-2.csv")
        );
        // A name with no extension keeps its shape too.
        assert_eq!(beside(Path::new("readings"), 3), Path::new("readings-3"));
    }
}
