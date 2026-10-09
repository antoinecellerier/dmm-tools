//! Markers the user placed on readings (`N`, `Ctrl+N`), each with an optional
//! note.
//!
//! Kept apart from both stores of the sample stream, because neither outlives
//! the other: the graph restarts on every mode or unit change while a
//! recording carries on, and Record empties the sample buffer while the graph
//! keeps its trace. A marker lives as long as its reading is in either — see
//! [`Markers::retain`].

use chrono::{DateTime, Local};
use std::collections::VecDeque;
use std::time::Instant;

pub(crate) use dmm_shared::export::NOTE_MAX_CHARS;

/// One marked reading.
#[derive(Debug, Clone)]
pub(crate) struct Marker {
    /// The marked reading's `measurement.timestamp`, which is also how the
    /// graph and the sample buffer know it.
    pub(crate) at: Instant,
    /// Shown on the graph and written to exports. Never reused while the
    /// store holds markers, so a note saying "see 3" keeps meaning the same
    /// marker after an earlier one is deleted.
    pub(crate) number: u32,
    pub(crate) note: String,
    /// When the reading was taken and what it read, as the list shows them.
    /// Kept here rather than looked up, because the reading may have left the
    /// sample buffer while the graph still holds it.
    pub(crate) wall_time: DateTime<Local>,
    pub(crate) reading: String,
}

/// The markers, oldest first.
///
/// Bounded by the readings the graph and the sample buffer hold, one marker
/// per reading at most.
#[derive(Debug, Default)]
pub(crate) struct Markers {
    list: VecDeque<Marker>,
    /// Number the next marker gets.
    next_number: u32,
    /// Highest number the file being played or imported puts back on its
    /// readings: a marker added meanwhile is numbered past it, or the file's
    /// would come in under a number already taken.
    reserved: u32,
}

impl Markers {
    /// Mark the reading taken at `at`, or return the number of the marker
    /// already on it. The new marker's number comes back as `Ok`, an
    /// existing one's as `Err`.
    pub(crate) fn add(
        &mut self,
        at: Instant,
        wall_time: DateTime<Local>,
        reading: String,
    ) -> Result<u32, u32> {
        let i = self.list.partition_point(|m| m.at < at);
        if let Some(existing) = self.list.get(i).filter(|m| m.at == at) {
            return Err(existing.number);
        }
        if self.list.is_empty() {
            self.next_number = 1;
        }
        let number = self.next_number.max(self.reserved.saturating_add(1));
        self.next_number = number.saturating_add(1);
        self.list.insert(
            i,
            Marker {
                at,
                number,
                note: String::new(),
                wall_time,
                reading,
            },
        );
        Ok(number)
    }

    /// Mark the reading taken at `at` with `note`, or put the note after the
    /// one its marker already has: an alarm's breach goes on whatever the
    /// reading carries. Returns the marker's number.
    pub(crate) fn add_noted(
        &mut self,
        at: Instant,
        wall_time: DateTime<Local>,
        reading: String,
        note: &str,
    ) -> u32 {
        let number = match self.add(at, wall_time, reading) {
            Ok(number) | Err(number) => number,
        };
        if let Some(marker) = self.list.iter_mut().find(|m| m.number == number) {
            dmm_shared::export::append_note(&mut marker.note, note);
        }
        number
    }

    /// Put back a marker a file was saved with, under its own number, on the
    /// reading taken at `at`. `false` when that reading already has one.
    ///
    /// Later markers are numbered past it, so a note saying "see 3" keeps
    /// pointing at the marker the file called 3.
    pub(crate) fn insert(
        &mut self,
        at: Instant,
        number: u32,
        note: String,
        wall_time: DateTime<Local>,
        reading: String,
    ) -> bool {
        let i = self.list.partition_point(|m| m.at < at);
        if self.list.get(i).is_some_and(|m| m.at == at) {
            return false;
        }
        if self.list.is_empty() {
            self.next_number = 1;
        }
        self.next_number = self.next_number.max(number.saturating_add(1));
        self.list.insert(
            i,
            Marker {
                at,
                number,
                note,
                wall_time,
                reading,
            },
        );
        true
    }

    /// Keep the numbers up to `highest` for the markers a file is putting
    /// back; 0 when there is none.
    pub(crate) fn reserve(&mut self, highest: u32) {
        self.reserved = highest;
    }

    /// Drop every marker: a new session starts from none.
    pub(crate) fn clear(&mut self) {
        self.list.clear();
    }

    /// Keep the markers whose reading `held` says is still somewhere.
    pub(crate) fn retain(&mut self, held: impl Fn(Instant) -> bool) {
        self.list.retain(|m| held(m.at));
    }

    pub(crate) fn remove(&mut self, number: u32) -> Option<Marker> {
        let i = self.list.iter().position(|m| m.number == number)?;
        self.list.remove(i)
    }

    pub(crate) fn get(&self, number: u32) -> Option<&Marker> {
        self.list.iter().find(|m| m.number == number)
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = &Marker> + DoubleEndedIterator {
        self.list.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = &mut Marker> {
        self.list.iter_mut()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// The markers on readings taken between `start` and `end`, both
    /// included, found by binary search: the per-frame graph code asks for
    /// the ones in view.
    pub(crate) fn between(&self, start: Instant, end: Instant) -> impl Iterator<Item = &Marker> {
        let from = self.list.partition_point(|m| m.at < start);
        let to = self.list.partition_point(|m| m.at <= end);
        self.list.range(from..to.max(from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn add(markers: &mut Markers, at: Instant) -> Result<u32, u32> {
        markers.add(at, Local::now(), "1.234 V".to_string())
    }

    #[test]
    fn a_reading_takes_one_marker() {
        let mut m = Markers::default();
        let t = Instant::now();
        assert_eq!(add(&mut m, t), Ok(1));
        assert_eq!(add(&mut m, t), Err(1), "the same reading again");
        assert_eq!(add(&mut m, t + Duration::from_millis(300)), Ok(2));
        assert_eq!(m.iter().count(), 2);
    }

    #[test]
    fn a_noted_marker_adds_to_a_note_already_there() {
        let mut m = Markers::default();
        let t = Instant::now();
        let reading = || "1.234 V".to_string();
        assert_eq!(
            m.add_noted(t, Local::now(), reading(), "Above high limit 1 V"),
            1
        );
        assert_eq!(m.get(1).unwrap().note, "Above high limit 1 V");
        add(&mut m, t + Duration::from_secs(1)).unwrap();
        m.iter_mut().last().unwrap().note = "load on".to_string();
        let later = t + Duration::from_secs(1);
        assert_eq!(
            m.add_noted(later, Local::now(), reading(), "Below low limit 0 V"),
            2
        );
        assert_eq!(m.get(2).unwrap().note, "load on; Below low limit 0 V");
    }

    /// Numbers survive a delete, so a gap is left; they start over only once
    /// the store is empty.
    #[test]
    fn numbers_are_stable_until_the_store_empties() {
        let mut m = Markers::default();
        let t = Instant::now();
        for i in 0..3 {
            add(&mut m, t + Duration::from_secs(i)).unwrap();
        }
        m.remove(2);
        assert_eq!(add(&mut m, t + Duration::from_secs(5)), Ok(4));
        let numbers: Vec<u32> = m.iter().map(|k| k.number).collect();
        assert_eq!(numbers, [1, 3, 4]);

        m.retain(|_| false);
        assert!(m.is_empty());
        assert_eq!(add(&mut m, t + Duration::from_secs(9)), Ok(1));
    }

    /// A marker added while a file is putting its own back is numbered past
    /// the file's, so the file's later ones keep their numbers unshared.
    #[test]
    fn a_files_numbers_are_kept_for_it() {
        let mut m = Markers::default();
        let t = Instant::now();
        m.reserve(2);
        assert!(m.insert(t, 1, "start".into(), Local::now(), String::new()));
        let breach = m.add_noted(t + Duration::from_secs(5), Local::now(), String::new(), "x");
        assert_eq!(breach, 3);
        assert!(m.insert(
            t + Duration::from_secs(9),
            2,
            "end".into(),
            Local::now(),
            String::new()
        ));
        let numbers: Vec<u32> = m.iter().map(|k| k.number).collect();
        assert_eq!(numbers, [1, 3, 2]);
    }

    /// The readings still held need not be one stretch: a marker between
    /// two held ones goes when its own reading does.
    #[test]
    fn retain_keeps_exactly_the_held_readings() {
        let mut m = Markers::default();
        let t = Instant::now();
        for i in 0..4 {
            add(&mut m, t + Duration::from_secs(i)).unwrap();
        }
        m.retain(|at| at != t + Duration::from_secs(1));
        let numbers: Vec<u32> = m.iter().map(|k| k.number).collect();
        assert_eq!(numbers, [1, 3, 4]);
    }

    /// A marker placed on an older reading than the newest marker still
    /// lands in time order.
    #[test]
    fn markers_stay_in_time_order() {
        let mut m = Markers::default();
        let t = Instant::now();
        add(&mut m, t + Duration::from_secs(2)).unwrap();
        add(&mut m, t).unwrap();
        let at: Vec<Instant> = m.iter().map(|k| k.at).collect();
        assert_eq!(at, [t, t + Duration::from_secs(2)]);
    }

    #[test]
    fn between_includes_both_ends() {
        let mut m = Markers::default();
        let t = Instant::now();
        for i in 0..5 {
            add(&mut m, t + Duration::from_secs(i)).unwrap();
        }
        let numbers: Vec<u32> = m
            .between(t + Duration::from_secs(1), t + Duration::from_secs(3))
            .map(|k| k.number)
            .collect();
        assert_eq!(numbers, [2, 3, 4]);
        assert_eq!(
            m.between(t + Duration::from_secs(9), t + Duration::from_secs(10))
                .count(),
            0
        );
    }
}
