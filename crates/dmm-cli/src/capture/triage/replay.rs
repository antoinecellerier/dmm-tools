//! A step's recorded frames played back to its protocol as the link they
//! came over, so the driver itself decodes them.
//!
//! A polled meter answers what it is asked, so a request the protocol
//! writes releases the reply recorded after the same request, and reads
//! stop at the next recorded request: a protocol that drops whatever is
//! queued before asking (bm86x) would otherwise drop the rest of the step.
//! A streaming meter sends on its own clock, so its frames are served
//! straight through and the keys the capture sent are only timeline marks.

use super::super::report::{FrameDir, FrameRecord, hex_bytes};
use dmm_lib::error::Result;
use dmm_lib::transport::{Link, Transport};
use std::cell::{Cell, RefCell};

/// One recorded transfer, its hex decoded.
pub(super) struct Record {
    pub at_ms: u64,
    pub dir: FrameDir,
    pub bytes: Vec<u8>,
}

/// The records a report's frames hold, link set-up left out.
pub(super) fn records(frames: &[FrameRecord]) -> Vec<Record> {
    frames
        .iter()
        .filter(|f| !f.feature && f.baud.is_none())
        .filter_map(|f| {
            Some(Record {
                at_ms: f.at_ms,
                dir: f.dir,
                bytes: hex_bytes(&f.hex)?,
            })
        })
        .collect()
}

pub(super) struct Replay<'a> {
    records: &'a [Record],
    link: Option<Link>,
    /// Closed while the protocol's `init` runs: what it writes is kept, but
    /// it reads nothing, so a purge there cannot eat the step.
    open: Cell<bool>,
    polled: Cell<bool>,
    /// The record served next, and how much of it is gone.
    cursor: Cell<usize>,
    offset: Cell<usize>,
    /// The last record a byte was served from.
    last_served: Cell<Option<usize>>,
    /// Something was served or a request released a reply since the last
    /// [`Replay::take_progress`].
    progress: Cell<bool>,
    /// Every write, in order.
    writes: RefCell<Vec<Vec<u8>>>,
    /// The recorded requests the protocol's writes matched.
    polls: RefCell<Vec<usize>>,
    /// Received records a request skipped unread: the reply to a key the
    /// capture pressed between two requests.
    unread: RefCell<Vec<usize>>,
}

impl<'a> Replay<'a> {
    pub(super) fn new(records: &'a [Record], link: Option<Link>) -> Self {
        Replay {
            records,
            link,
            open: Cell::new(false),
            polled: Cell::new(false),
            cursor: Cell::new(0),
            offset: Cell::new(0),
            last_served: Cell::new(None),
            progress: Cell::new(false),
            writes: RefCell::new(Vec::new()),
            polls: RefCell::new(Vec::new()),
            unread: RefCell::new(Vec::new()),
        }
    }

    /// Start serving, for a meter that answers requests or one that
    /// streams.
    pub(super) fn open(&self, polled: bool) {
        self.polled.set(polled);
        self.open.set(true);
    }

    pub(super) fn take_progress(&self) -> bool {
        self.progress.replace(false)
    }

    pub(super) fn last_served(&self) -> Option<usize> {
        self.last_served.get()
    }

    /// Nothing left to serve.
    pub(super) fn exhausted(&self) -> bool {
        self.records[self.cursor.get()..]
            .iter()
            .all(|r| r.dir == FrameDir::Tx)
    }

    /// Whether a recorded transmission is one the protocol writes itself (a
    /// request, a stream start) rather than a key the capture pressed.
    pub(super) fn written_by_protocol(&self, bytes: &[u8]) -> bool {
        self.writes.borrow().iter().any(|w| w == bytes)
    }

    pub(super) fn polls(&self) -> Vec<usize> {
        self.polls.borrow().clone()
    }

    pub(super) fn unread(&self) -> Vec<usize> {
        self.unread.borrow().clone()
    }

    /// Jump past the next recorded request with `data`'s bytes, or past the
    /// next request of any bytes when the step holds none like it (a build
    /// that asked differently).
    fn release(&self, data: &[u8]) {
        let from = self.cursor.get();
        let ahead = || {
            self.records
                .iter()
                .enumerate()
                .skip(from)
                .filter(|(_, r)| r.dir == FrameDir::Tx)
        };
        let matching = ahead().find(|(_, r)| r.bytes == data).map(|(i, _)| i);
        let anywhere = self
            .records
            .iter()
            .any(|r| r.dir == FrameDir::Tx && r.bytes == data);
        let target = match matching {
            Some(i) => Some(i),
            None if !anywhere => ahead().next().map(|(i, _)| i),
            None => None,
        };
        let Some(target) = target else {
            return;
        };
        let first_unread = from + usize::from(self.offset.get() > 0);
        self.unread
            .borrow_mut()
            .extend((first_unread..target).filter(|&i| self.records[i].dir == FrameDir::Rx));
        self.polls.borrow_mut().push(target);
        self.cursor.set(target + 1);
        self.offset.set(0);
        self.progress.set(true);
    }
}

impl Transport for Replay<'_> {
    fn write(&self, data: &[u8]) -> Result<()> {
        self.writes.borrow_mut().push(data.to_vec());
        if self.open.get() && self.polled.get() {
            self.release(data);
        }
        Ok(())
    }

    fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
        if !self.open.get() {
            return Ok(0);
        }
        loop {
            let i = self.cursor.get();
            let Some(record) = self.records.get(i) else {
                return Ok(0);
            };
            if record.dir == FrameDir::Tx {
                if self.polled.get() {
                    return Ok(0);
                }
                self.cursor.set(i + 1);
                continue;
            }
            let rest = &record.bytes[self.offset.get()..];
            let n = rest.len().min(buf.len());
            buf[..n].copy_from_slice(&rest[..n]);
            if n == rest.len() {
                self.cursor.set(i + 1);
                self.offset.set(0);
            } else {
                self.offset.set(self.offset.get() + n);
            }
            if n == 0 {
                continue;
            }
            self.last_served.set(Some(i));
            self.progress.set(true);
            return Ok(n);
        }
    }

    fn set_baud(&self, _baud: u32) -> Result<()> {
        Ok(())
    }

    fn link(&self) -> Option<Link> {
        self.link
    }
}
