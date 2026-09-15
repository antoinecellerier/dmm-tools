//! Where a `read` run writes its readings.
//!
//! Stdout and `-o FILE` are open before the first reading. A bare `-o` names
//! its file after the meter, the mode and the moment the run started — the
//! same name [`dmm-gui`'s Export… opens on](dmm_settings::export::default_name)
//! — so it cannot be created until a reading has arrived, and the header waits
//! in memory until then.

use chrono::{DateTime, Local};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

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
            let path = PathBuf::from(dmm_settings::export::default_name(
                &auto.meter,
                Some(mode),
                at,
                auto.extension,
            ));
            let mut file = create(&path)?;
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

    /// Close the run, and say where the readings ended up — `None` unless the
    /// run named the file itself, which is the only case the user cannot see
    /// from the command they typed.
    pub(crate) fn finish(self) -> io::Result<Option<PathBuf>> {
        let Self { sink, auto } = self;
        // Closed before the rename below: Windows refuses to rename a file
        // that is still open.
        if let Sink::Open(mut open) = sink {
            open.flush()?;
            drop(open);
        }
        let Some(Auto {
            meter,
            extension,
            named: Some(named),
        }) = auto
        else {
            return Ok(None);
        };
        if named.one_mode {
            return Ok(Some(named.path));
        }
        let renamed = PathBuf::from(dmm_settings::export::default_name(
            &meter,
            None,
            named.start,
            extension,
        ));
        std::fs::rename(&named.path, &renamed).map_err(|e| at_path(&renamed, e))?;
        Ok(Some(renamed))
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

/// Name the file in an io failure: "permission denied" on its own says
/// nothing about which file the run could not write.
fn at_path(path: &Path, e: io::Error) -> io::Error {
    io::Error::new(e.kind(), format!("{}: {e}", path.display()))
}
