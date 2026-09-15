//! Writing what `read --record` saves, for `read --replay` to play back.
//!
//! Every line comes from [`dmm_lib::replay`], which also parses them, so the
//! two ends of a recording cannot drift apart.

use chrono::{Local, SecondsFormat};
use dmm_lib::measurement::Measurement;
use dmm_lib::replay;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// A recording being written, one line per frame the meter answered with.
pub(crate) struct ReplayWriter {
    out: BufWriter<File>,
    /// When the first frame arrived. Offsets are measured from it, so a
    /// recording starts at zero however long the meter took to answer.
    first: Option<Instant>,
    count: usize,
    path: PathBuf,
}

impl ReplayWriter {
    /// Create the file and write its header.
    ///
    /// `recorded` is taken here rather than at the first frame: one poll
    /// earlier at most, and it is the time the user started the run.
    pub(crate) fn create(path: &Path, device_id: &str, model: Option<&str>) -> io::Result<Self> {
        let out = BufWriter::new(File::create(path).map_err(|e| at(path, e))?);
        let mut writer = Self {
            out,
            first: None,
            count: 0,
            path: path.to_path_buf(),
        };
        let recorded = Local::now().to_rfc3339_opts(SecondsFormat::Millis, false);
        writer.write(&replay::header(device_id, &recorded, model))?;
        Ok(writer)
    }

    /// Append one frame, exactly as the meter sent it.
    pub(crate) fn push(&mut self, m: &Measurement) -> io::Result<()> {
        let first = *self.first.get_or_insert(m.timestamp);
        let offset = m
            .timestamp
            .checked_duration_since(first)
            .unwrap_or_default();
        self.write(&replay::sample_line(offset, &m.raw_payload))?;
        self.count += 1;
        Ok(())
    }

    /// Close the recording and say how many frames it holds.
    pub(crate) fn finish(mut self) -> io::Result<usize> {
        self.out.flush().map_err(|e| at(&self.path, e))?;
        Ok(self.count)
    }

    /// Write one line and put it on disk straight away.
    ///
    /// Flushed per line on purpose: a bench recording ends with Ctrl-C or a
    /// pulled cable, and what it holds by then is hours of measurements.
    fn write(&mut self, line: &str) -> io::Result<()> {
        self.out
            .write_all(line.as_bytes())
            .and_then(|()| self.out.flush())
            .map_err(|e| at(&self.path, e))
    }
}

/// Name the recording in an io failure: the same run may be writing `--output`
/// as well, and "permission denied" alone says nothing about which.
fn at(path: &Path, e: io::Error) -> io::Error {
    io::Error::new(e.kind(), format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::protocol::ut61eplus::make_test_measurement;
    use dmm_lib::replay::Replay;
    use std::time::Duration;

    /// The one check that writer and parser agree, short of a meter: a bench
    /// recording has to come back as the session it was.
    #[test]
    fn a_recording_parses_back_as_a_replay() {
        let dir = std::env::temp_dir().join(format!("dmm-cli-record-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("bench.replay");

        // The `dcv_battery` golden frame, 1.6109 V on the 2.2V range.
        let mut m = make_test_measurement(0x02, 0x30, b" 1.6109", (0x03, 0x02), (0x30, 0x30, 0x30));
        let first = m.timestamp;
        let mut writer = ReplayWriter::create(&path, "ut61eplus", Some("UT61E+")).expect("create");
        writer.push(&m).expect("first frame");
        m.timestamp = first + Duration::from_millis(250);
        writer.push(&m).expect("second frame");
        assert_eq!(writer.finish().expect("finish"), 2);

        let text = std::fs::read_to_string(&path).expect("read the recording back");
        let replay = Replay::parse(&text).expect("parses as a replay");
        assert_eq!(replay.device.id, "ut61eplus");
        assert_eq!(replay.model.as_deref(), Some("UT61E+"));
        // Offsets run from the first frame, not from wherever the session was.
        assert_eq!(replay.duration(), Duration::from_millis(250));
    }
}
