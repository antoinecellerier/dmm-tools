//! Markers on readings: what `N` and `Ctrl+N` do, how long a marker lasts,
//! and the Recording panel's log, where each marker sits among the samples
//! at its reading and its note is written.

use dmm_shared::tr;
use eframe::egui::{self, Key, RichText, Ui};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::ops::Range;
use std::time::Instant;

use super::toast::Toast;
use super::{App, BigMeterMode};
use crate::a11y::ResponseA11yExt;
use crate::markers::{Marker, NOTE_MAX_CHARS};
use crate::recording::{BufferRole, Sample};
use dmm_lib::measurement::{MeasuredValue, Measurement, aux_summary_line};
use dmm_lib::text::TextKind;
use dmm_shared::l10n::Localizer;

/// Why notes cannot be written now: what hides the list they are written
/// in.
#[derive(Debug, Clone, Copy)]
enum NotesHidden {
    BigMeter,
    Recording,
}

impl NotesHidden {
    /// The selector the messages saying so take.
    fn key(self) -> &'static str {
        match self {
            Self::BigMeter => "big-meter",
            Self::Recording => "recording",
        }
    }
}

/// Narrower than this, a marker's number and note go on a line of their own
/// under the reading rather than beside it.
const NARROW_ROW_WIDTH: f32 = 460.0;

/// The note keeps at least this much room beside its reading.
const MIN_NOTE_WIDTH: f32 = 120.0;

/// Past this many pixels an `f32` position steps by more than one, so the log
/// shows no more rows than fit in it.
const LOG_MAX_HEIGHT: f32 = 16_777_216.0;

/// Space between a marker tag's edge and its number.
const TAG_PAD: f32 = 3.0;
/// How far a marker tag's point reaches out towards its reading.
const TAG_TIP: f32 = 5.0;

/// The log's font: monospace, so the time, value and unit columns line up.
fn log_font() -> egui::FontId {
    egui::FontId::monospace(11.0)
}

/// Where a marker's tag goes in `slot`: the graph's flag turned to point
/// left at its reading, right-aligned and as wide as a label `label_width`
/// wide needs. Its body, and the tip of its point.
fn tag_shape(slot: egui::Rect, label_width: f32) -> (egui::Rect, egui::Pos2) {
    let body = egui::Rect::from_min_max(
        egui::pos2(slot.right() - label_width - 2.0 * TAG_PAD, slot.top()),
        slot.right_bottom(),
    );
    (body, egui::pos2(body.left() - TAG_TIP, body.center().y))
}

/// Where a tag in `body`, with its point at `tip`, takes a click: the body
/// and the point's reach.
fn tag_hit_rect(body: egui::Rect, tip: egui::Pos2) -> egui::Rect {
    egui::Rect::from_min_max(egui::pos2(tip.x, body.top()), body.max)
}

/// The list's own state, between frames.
#[derive(Debug, Default)]
pub(super) struct MarkerList {
    /// The marker whose note takes the focus when its row is next drawn:
    /// the one `Ctrl+N` just placed.
    focus: Option<u32>,
    /// The note being edited.
    editing: Option<NoteEdit>,
    /// Whether the log was scrolled to its newest row last frame.
    following: bool,
    /// Scroll the log back to its newest row on the next frame.
    refollow: bool,
}

/// A note being edited.
#[derive(Debug)]
struct NoteEdit {
    number: u32,
    /// The note when editing began: what Esc puts back.
    before: String,
    /// Whether this edit has brought the marker into view yet.
    revealed: bool,
    /// Whether the log was following its newest row when editing began. The
    /// text field keeps its cursor in view while typing, which stops the log
    /// following; once the edit ends, it follows again.
    following: bool,
}

/// A reading as a log line shows it after the time: value and unit, flags,
/// then any sub-values — which trail so the value and unit columns stay put
/// for meters that report none. A frame without a main reading has no value
/// for the unit to follow; its sub-values carry their own.
///
/// A marker keeps its reading's line, so a marker without a sample in the log
/// reads like one.
pub(super) fn log_line(l10n: &Localizer, m: &Measurement) -> String {
    let flags: Vec<Cow<'_, str>> = m.flags.active().filter_map(|f| l10n.flag(f)).collect();
    let flags = if flags.is_empty() {
        String::new()
    } else {
        format!(" [{}]", flags.join(" "))
    };
    let labels: Vec<Cow<'_, str>> = m
        .present_aux()
        .map(|aux| l10n.label(TextKind::Aux, &aux.label))
        .collect();
    let summary = aux_summary_line(m.present_aux().zip(&labels).map(|(aux, label)| {
        (
            label.as_ref(),
            aux.value_str(),
            aux.unit_or(&m.unit),
            aux.elapsed_secs,
        )
    }));
    let aux = if summary.is_empty() {
        String::new()
    } else {
        format!("  {summary}")
    };
    let unit = match m.value {
        MeasuredValue::Absent => "",
        _ => m.unit.as_ref(),
    };
    format!("{val:>10} {unit}{flags}{aux}", val = m.value_display_str())
}

/// One row of the log: a sample, a marker, or a marked sample. Samples by
/// their index in the buffer, markers by theirs in the caller's list.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Row {
    Sample(usize),
    Marker(usize),
    Marked(usize, usize),
}

/// The log's rows in time order: the samples it shows with each marker on
/// its sample's row, and the markers whose sample it doesn't show — placed
/// before Record, or older than the samples shown — in rows of their own.
///
/// Worked out by binary search, so a frame costs the markers, not the
/// samples: a recording can hold half a million.
struct LogRows {
    /// The buffer's samples the log shows.
    shown: Range<usize>,
    /// Each marker's row, its index and the sample it sits on, in order.
    markers: Vec<(usize, usize, Option<usize>)>,
    /// The rows of the markers without a sample shown, in order.
    own: Vec<usize>,
}

impl LogRows {
    /// `markers` are the markers' readings, in time order.
    fn new(samples: &VecDeque<Sample>, shown: Range<usize>, markers: &[Instant]) -> Self {
        let mut rows = Vec::with_capacity(markers.len());
        let mut own = Vec::new();
        for (i, &at) in markers.iter().enumerate() {
            let p = samples.partition_point(|s| s.measurement.timestamp < at);
            // Every earlier own row comes before this marker: they are in
            // time order.
            let row = p.clamp(shown.start, shown.end) - shown.start + own.len();
            if shown.contains(&p) && samples[p].measurement.timestamp == at {
                rows.push((row, i, Some(p)));
            } else {
                own.push(row);
                rows.push((row, i, None));
            }
        }
        Self {
            shown,
            markers: rows,
            own,
        }
    }

    fn len(&self) -> usize {
        self.shown.len() + self.own.len()
    }

    fn row(&self, r: usize) -> Row {
        let n = self.markers.partition_point(|&(row, ..)| row < r);
        match self.markers.get(n) {
            Some(&(row, i, None)) if row == r => Row::Marker(i),
            Some(&(row, i, Some(k))) if row == r => Row::Marked(k, i),
            _ => Row::Sample(self.shown.start + r - self.own.partition_point(|&row| row < r)),
        }
    }
}

/// Where the log's rows sit: a pitch each, and two for the `tall` ones — a
/// marker's row in a narrow panel, its controls on a line of their own.
struct RowGeometry {
    pitch: f32,
    tall: Vec<usize>,
    /// Where each tall row ends, in pitches.
    ends: Vec<usize>,
}

impl RowGeometry {
    fn new(pitch: f32, tall: Vec<usize>) -> Self {
        let ends = tall.iter().enumerate().map(|(m, &t)| t + m + 2).collect();
        Self { pitch, tall, ends }
    }

    fn top(&self, r: usize) -> f32 {
        (r + self.tall.partition_point(|&t| t < r)) as f32 * self.pitch
    }

    fn height(&self, r: usize) -> f32 {
        let pitches = if self.tall.binary_search(&r).is_ok() {
            2.0
        } else {
            1.0
        };
        pitches * self.pitch
    }

    /// The row at `y`.
    fn row_at(&self, y: f32) -> usize {
        let above = self.ends.partition_point(|&e| e as f32 * self.pitch <= y);
        let r = ((y.max(0.0) / self.pitch) as usize).saturating_sub(above);
        self.tall.get(above).map_or(r, |&tall| r.min(tall))
    }
}

/// Whether the graph or the sample buffer still holds the reading taken at
/// `at`.
fn reading_held(
    graph: &crate::graph::Graph,
    recording: &crate::recording::Recording,
    at: Instant,
) -> bool {
    graph.holds(at) || recording.holds(at)
}

impl App {
    /// Drop the markers whose readings have left both the graph and the
    /// sample buffer. Every frame, after the new readings are in.
    ///
    /// Each reading is looked up in both rather than cut at one point: what
    /// the two hold together need not be one stretch of time — a stopped
    /// recording, then a graph restarted long after it.
    pub(super) fn trim_markers(&mut self) {
        let (graph, recording) = (&self.graph, &self.capture.recording);
        self.markers.retain(|at| reading_held(graph, recording, at));
    }

    /// Put the `--replay` file's next marker on `m` when `m` is the first
    /// reading played at or after the marker's offset, so a frame the
    /// playback skipped passes its marker to the next one. One per reading:
    /// a second marker due by now waits for the next reading rather than
    /// losing its place to the first.
    pub(super) fn place_replay_marker(&mut self, m: &dmm_lib::measurement::Measurement) {
        let Some((start, _)) = self.clock.wall_origin() else {
            return;
        };
        let due = |offset| {
            start
                .checked_add(offset)
                .is_some_and(|at| at <= m.timestamp)
        };
        if !self
            .source
            .replay_markers
            .front()
            .is_some_and(|next| due(next.offset))
        {
            return;
        }
        let Some(next) = self.source.replay_markers.pop_front() else {
            return;
        };
        let wall_time = m.wall_time.into();
        if !self.markers.insert(
            m.timestamp,
            next.number,
            next.note.clone(),
            wall_time,
            log_line(&self.l10n, m),
        ) {
            // A marker the user put on this reading first: this one waits.
            self.source.replay_markers.push_front(next);
        }
    }

    /// Drop the queued `--replay` markers the playback has passed: a Connect
    /// resumes at the newest frame due, and the frames before it never play,
    /// so their markers would pile onto the first readings after it, far
    /// from their own moment. `cadence` spares the frame playback resumes
    /// on.
    pub(super) fn drop_replay_markers_passed(&mut self, cadence: std::time::Duration) {
        let Some((start, _)) = self.clock.wall_origin() else {
            return;
        };
        let resume = self
            .clock
            .now()
            .saturating_duration_since(start)
            .saturating_sub(cadence);
        let before = self.source.replay_markers.len();
        self.source.replay_markers.retain(|m| m.offset >= resume);
        let dropped = before - self.source.replay_markers.len();
        if dropped > 0 {
            log::info!("replay resumed past {dropped} markers; they are left out");
        }
    }

    /// Put the `--replay` file's saved view in place once `m` is at or past
    /// the furthest moment it shows, so its window and cursors land on
    /// readings the graph already holds.
    pub(super) fn apply_replay_view(&mut self, m: &dmm_lib::measurement::Measurement) {
        let Some((start, _)) = self.clock.wall_origin() else {
            return;
        };
        let due = self.source.replay_view.as_ref().is_some_and(|(_, reach)| {
            start
                .checked_add(*reach)
                .is_some_and(|at| at <= m.timestamp)
        });
        if due && let Some((view, _)) = self.source.replay_view.take() {
            self.graph.apply_view_state(&view, start);
        }
    }

    /// `N`, or with `write_note` `Ctrl+N`: mark the reading on screen.
    ///
    /// The reading on screen rather than the moment of the key press: paused
    /// or disconnected, that reading is the moment the user is looking at.
    pub(super) fn add_marker(&mut self, write_note: bool) {
        let Some(m) = self.last_measurement.as_ref() else {
            self.toast = Some(Toast::info(tr!(self.l10n, "markers-no-reading")));
            return;
        };
        let (at, reading) = (m.timestamp, log_line(&self.l10n, m));
        self.mark_reading(at, reading, write_note);
    }

    /// Mark the reading taken at `at`, which `reading` shows; with
    /// `write_note`, put the cursor in its note, the one already there if the
    /// reading has a marker.
    ///
    /// A reading neither the graph nor the buffer holds any more — cleared,
    /// or gone with a graph restart while the plot's menu was open — gets no
    /// marker: the next frame's trim would take it away unseen.
    fn mark_reading(&mut self, at: Instant, reading: String, write_note: bool) {
        if !reading_held(&self.graph, &self.capture.recording, at) {
            self.toast = Some(Toast::info(tr!(self.l10n, "markers-no-reading")));
            return;
        }
        // The reading's own wall time where the buffer holds it; a point
        // only the graph still has maps through the session clock.
        let wall_time = self
            .capture
            .recording
            .sample_at(at)
            .map(|s| s.wall_time)
            .unwrap_or_else(|| self.clock.wall_time_for(at).into());
        let (number, added) = match self.markers.add(at, wall_time, reading) {
            Ok(number) => (number, true),
            // Ctrl+N opens the note already there.
            Err(number) if write_note => (number, false),
            Err(number) => {
                self.toast = Some(Toast::info(tr!(
                    self.l10n,
                    "markers-already-marked",
                    number = number
                )));
                return;
            }
        };
        match self.where_notes_are_written() {
            None if write_note => self.marker_list.focus = Some(number),
            None => {}
            Some(hidden) => {
                let text = if added {
                    tr!(
                        self.l10n,
                        "markers-added",
                        number = number,
                        hidden = hidden.key()
                    )
                } else {
                    tr!(
                        self.l10n,
                        "markers-already-marked-hidden",
                        number = number,
                        hidden = hidden.key()
                    )
                };
                self.toast = Some(Toast::info(text));
            }
        }
    }

    /// What the graph asked of the markers this frame: the note of a flag
    /// that was clicked.
    pub(super) fn take_clicked_marker(&mut self) {
        if let Some(number) = self.graph.take_clicked_marker() {
            self.open_marker_note(number);
        }
    }

    /// The plot's menu asked for a marker on the reading taken at `at`, of
    /// `value` on the trace.
    pub(super) fn mark_from_graph(&mut self, at: Instant, value: f64) {
        // The sample's line when the buffer has it; otherwise all the
        // graph knows, its trace's value — without the meter's own
        // digits, its flags or its sub-values.
        let reading = match self.capture.recording.sample_at(at) {
            Some(s) => log_line(&self.l10n, &s.measurement),
            None => format!(
                "{:>10} {}",
                format!("{value:.prec$}", prec = self.graph.decimals(true)),
                self.graph.plotted_unit()
            ),
        };
        self.mark_reading(at, reading, true);
    }

    /// A marker's flag was clicked on the graph: put the cursor in its note,
    /// or say how to bring back the panel it is written in.
    fn open_marker_note(&mut self, number: u32) {
        match self.where_notes_are_written() {
            None => self.marker_list.focus = Some(number),
            Some(hidden) => {
                let text = tr!(self.l10n, "markers-note-hidden", hidden = hidden.key());
                self.toast = Some(Toast::info(text));
            }
        }
    }

    /// `None` while the Recording panel is on screen; otherwise what hides
    /// it, where notes are written.
    fn where_notes_are_written(&self) -> Option<NotesHidden> {
        if self.big_meter_mode != BigMeterMode::Off {
            Some(NotesHidden::BigMeter)
        } else if !self.settings.show_recording {
            Some(NotesHidden::Recording)
        } else {
            None
        }
    }

    /// The log under the Record row: while recording, the whole recording
    /// with each marker on its reading's row; otherwise just the markers.
    /// One scroller, to the end of the panel, following the newest row while
    /// scrolled to it. Nothing when there is nothing to list.
    ///
    /// Only the rows in view are drawn, and every marker's, wherever it is:
    /// Tab reaches each note, and brings it into view.
    ///
    /// A marker's number, note and `×` sit in a column of their own at the
    /// right, so they line up whatever the reading's line says.
    pub(super) fn show_log(&mut self, ui: &mut Ui, compact: bool) {
        let markers = &self.markers;
        if let Some(edit) = self
            .marker_list
            .editing
            .take_if(|e| markers.get(e.number).is_none())
        {
            // Trimmed away while its note had the focus: the row whose note
            // ends the edit is gone, so the edit ends here.
            self.marker_list.refollow = edit.following;
        }
        // A handle, as the end of the log borrows the app.
        let l10n = self.l10n.clone();
        let (samples, recorded) = self.capture.recording.recording_slice();
        if recorded.is_empty() && self.markers.is_empty() {
            return;
        }
        if let Some(n) = self.marker_list.focus
            && self.markers.get(n).is_none()
        {
            // Trimmed away before its row was drawn.
            self.marker_list.focus = None;
        }

        // Every row is one line of the log font tall, a marked one too, so
        // the controls on a marker's row are sized down to it.
        let line = ui.fonts_mut(|f| f.row_height(&log_font()));
        let pitch = line + ui.spacing().item_spacing.y;
        let delete_width = ui.spacing().interact_size.y;
        let weak = ui.visuals().weak_text_color();
        let first = recorded
            .end
            .saturating_sub((LOG_MAX_HEIGHT / pitch) as usize)
            .max(recorded.start);
        if first > recorded.start {
            ui.label(
                RichText::new(tr!(
                    l10n,
                    "markers-log-cut",
                    time = samples[first].wall_time.format("%H:%M:%S").to_string()
                ))
                .small()
                .color(weak),
            );
        }
        // The rest of the panel, floored: past it, the column scrolls.
        let rest = ui.available_height().max(0.0).floor();
        let max_height = if compact { rest.min(80.0) } else { rest };
        // A panel squeezed below one row has nowhere to show the note Ctrl+N
        // opened: say so rather than type into a row no one can see.
        if max_height < line
            && let Some(n) = self.marker_list.focus.take()
        {
            self.toast = Some(Toast::info(tr!(l10n, "markers-log-too-short", number = n)));
        }

        let tc = self.settings.theme_colors(ui.visuals().dark_mode);
        // The graph's flag colours, a pair its contrast test covers.
        let (flag, flag_text) = (tc.graph_marker(), tc.plot_background());
        let recording = &self.capture.recording;
        let graph = &self.graph;
        let list = &mut self.marker_list;
        let mut markers: Vec<&mut Marker> = self.markers.iter_mut().collect();
        let times: Vec<Instant> = markers.iter().map(|m| m.at).collect();
        let rows = LogRows::new(samples, first..recorded.end, &times);
        let labels: Vec<(String, Option<Cow<str>>)> = rows
            .markers
            .iter()
            .map(|&(_, i, sample)| {
                let sample = sample.map(|k| &samples[k].measurement);
                marker_row_label(&l10n, markers[i], sample, graph, recording)
            })
            .collect();
        // The marker column starts past the widest of them, so it lines up
        // on every row.
        let reading_width = reading_column_width(ui, &labels, weak);
        let spacing = ui.spacing().item_spacing.x;
        // A floating scroll bar is drawn over the rows' right end: the ×
        // stays clear of it.
        let scroll = &ui.spacing().scroll;
        let gutter = if scroll.floating {
            scroll.bar_width + scroll.bar_outer_margin
        } else {
            0.0
        };
        let mut labels = labels.into_iter();
        let mut reveal = None;
        let mut delete = None;
        let mut mark = None;

        let mut area = egui::ScrollArea::vertical()
            .id_salt("recording_log")
            .max_height(max_height)
            // egui's floor for a scrolling area is 64 px, taller than the
            // panel it is in when that is short.
            .min_scrolled_height(max_height)
            .auto_shrink([false, true])
            .stick_to_bottom(true);
        if std::mem::take(&mut list.refollow) {
            // Past any log's height, clamped to the end, where
            // `stick_to_bottom` takes over again. Not infinity: egui does
            // arithmetic with it before clamping.
            area = area.vertical_scroll_offset(1.0e9);
        }
        let output = area.show_viewport(ui, |ui, viewport| {
            ui.spacing_mut().interact_size.y = line;
            ui.spacing_mut().button_padding.y = 0.0;
            let narrow = ui.available_width() < NARROW_ROW_WIDTH;
            // A tag with room for three digits, so the notes line up.
            let number_width =
                TAG_TIP + 2.0 * TAG_PAD + 3.0 * ui.fonts_mut(|f| f.glyph_width(&log_font(), '0'));
            let marker_rows = rows.markers.iter().map(|&(row, ..)| row);
            let geometry = RowGeometry::new(
                pitch,
                if narrow {
                    marker_rows.clone().collect()
                } else {
                    Vec::new()
                },
            );
            let height = geometry.top(rows.len()) - ui.spacing().item_spacing.y;
            // The rows are children placed by rect, which the content's own
            // rect doesn't grow to hold: it is set here, whole, as
            // `scroll_to_focus` checks a focused widget against it.
            ui.set_min_size(egui::vec2(ui.available_width(), height.max(0.0)));
            // The rows at last frame's offset, which this frame is drawn at
            // — clamped, as a jump to the end asks for past it. While
            // following, the last screenful too: `stick_to_bottom` moves down
            // to it only once this frame is drawn. Only a screenful, however
            // many rows came in while the log was hidden.
            let screen = |top: f32| {
                geometry.row_at(top).saturating_sub(1)
                    ..(geometry.row_at(top + viewport.height()) + 2).min(rows.len())
            };
            let last = (height - viewport.height()).max(0.0);
            let in_view = screen(viewport.min.y.min(last));
            let newest = if list.following { screen(last) } else { 0..0 };
            // In row order, so Tab goes through the notes in time order.
            let mut drawn: Vec<usize> = in_view.chain(newest).chain(marker_rows).collect();
            drawn.sort_unstable();
            drawn.dedup();
            let origin = ui.max_rect().min;
            let width = ui.available_width();
            // `LogLayout::fixed`'s terms, kept in this order: the column lands
            // on the same pixel as it always has.
            let column = reading_width
                .min(width - number_width - delete_width - 3.0 * spacing - gutter - MIN_NOTE_WIDTH)
                .max(0.0);
            let layout = LogLayout {
                line,
                number_width,
                delete_width,
                spacing,
                row_spacing: ui.spacing().item_spacing.y,
                gutter,
                column,
                narrow,
                flag,
                flag_text,
                weak,
            };
            for r in drawn {
                let row = rows.row(r);
                let at = match row {
                    Row::Sample(k) | Row::Marked(k, _) => samples[k].measurement.timestamp,
                    Row::Marker(i) => markers[i].at,
                };
                // Its own id, so the ids of the widgets in it don't shift as
                // rows scroll in and out of view: a salt alone would still
                // be mixed with a count of the rows drawn before it.
                let ui = &mut ui.new_child(
                    egui::UiBuilder::new()
                        .scope_id(egui::Id::unique(("log_row", at)))
                        .max_rect(egui::Rect::from_min_size(
                            origin + egui::vec2(0.0, geometry.top(r)),
                            egui::vec2(width, geometry.height(r) - layout.row_spacing),
                        ))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                let i = match row {
                    Row::Sample(k) => {
                        if let Some(offered) = sample_row(ui, &layout, &l10n, &samples[k]) {
                            mark = Some(offered);
                        }
                        continue;
                    }
                    Row::Marker(i) | Row::Marked(_, i) => i,
                };
                let m = &mut *markers[i];
                let number = m.number;
                let at = m.at;
                let on_graph = graph.holds(at);
                // One per marker row, and every marker row is drawn, in order.
                let Some((text, tag)) = labels.next() else {
                    continue;
                };
                // number, note, ×, in that order for Tab.
                let mut marker_column = |ui: &mut Ui, note_width: f32| {
                    if number_tag(ui, &layout, &l10n, m, on_graph) {
                        reveal = Some(at);
                    }
                    let response = note_field(ui, &l10n, m, note_width);
                    if delete_button(ui, &layout, &l10n, number) {
                        delete = Some(number);
                    }
                    response
                };
                let response = if layout.narrow {
                    reading_label(ui, &layout, &text, tag.as_deref());
                    ui.horizontal(|ui| {
                        marker_column(ui, (ui.available_width() - layout.fixed()).max(40.0))
                    })
                    .inner
                } else {
                    ui.horizontal(|ui| {
                        // Its own rect, whatever the reading's text: a child
                        // ui would hand back only as much as its text took.
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(layout.column, layout.line),
                            egui::Sense::hover(),
                        );
                        reading_label(
                            &mut ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(rect)
                                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                            ),
                            &layout,
                            &text,
                            tag.as_deref(),
                        );
                        let note_width = ui.available_width() - layout.fixed() + layout.spacing;
                        marker_column(ui, note_width.max(MIN_NOTE_WIDTH))
                    })
                    .inner
                };
                if list.track_note(ui, &response, number, &mut m.note) {
                    reveal = Some(at);
                }
            }
            // Tab and Shift+Tab reach every row, drawn or scrolled out of
            // sight, and bring the one they land on into view.
            crate::a11y::scroll_to_focus(ui);
        });
        list.following = output.state.clamped_offset().y + output.inner_rect.height()
            >= output.content_size.y - 1.0;

        if let Some(at) = reveal {
            self.graph.reveal(at);
        }
        if let Some(number) = delete {
            self.markers.remove(number);
        }
        if let Some((at, reading)) = mark {
            self.mark_reading(at, reading, true);
        }
    }
}

impl MarkerList {
    /// The focus and the edit of marker `number`'s note, whose field gave
    /// `response`: focus it when `focus` names it (Ctrl+N, a flag clicked),
    /// start an edit as it gains the focus, put the note back on Esc and
    /// follow the log again as the edit ends. Called each frame the field is
    /// drawn, after it. Whether to bring the marker into view: acting on a
    /// note does.
    fn track_note(
        &mut self,
        ui: &Ui,
        response: &egui::Response,
        number: u32,
        note: &mut String,
    ) -> bool {
        if self.focus == Some(number) {
            response.request_focus();
            // Only when out of view: a scroll that isn't needed would unstick
            // the log from its newest row.
            if !ui.clip_rect().contains_rect(response.rect) {
                response.scroll_to_me(None);
            }
            // Asked again next frame after a click — a marker's flag, the
            // row's +: egui takes the focus back from a note on a click
            // outside it, the next time it reads the note.
            if !ui.input(|i| i.pointer.any_click()) {
                self.focus = None;
            }
        }
        if response.gained_focus() {
            self.editing = Some(NoteEdit {
                number,
                before: note.clone(),
                revealed: false,
                following: self.following,
            });
        }
        // Acting on a note brings its marker into view: a click, or the first
        // keystroke of an edit. Tabbing through the list does not, and neither
        // do the keystrokes after — by then the view is the user's, live or
        // not.
        let edit = self.editing.as_mut().filter(|e| e.number == number);
        let reveal =
            response.clicked() || response.changed() && edit.as_ref().is_some_and(|e| !e.revealed);
        if reveal && let Some(edit) = edit {
            edit.revealed = true;
        }
        if response.lost_focus()
            && let Some(edit) = self.editing.take_if(|e| e.number == number)
        {
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                *note = edit.before;
            }
            self.refollow = edit.following;
        }
        reveal
    }
}

/// What the log's rows are sized and coloured by, this frame.
#[derive(Clone, Copy)]
struct LogLayout {
    /// One line of the log font: a row's height, a marked one's too (two in a
    /// narrow panel).
    line: f32,
    /// A marker's number tag, with room for three digits.
    number_width: f32,
    /// The × that deletes a marker.
    delete_width: f32,
    /// The gap between the items of a row.
    spacing: f32,
    /// The gap between rows.
    row_spacing: f32,
    /// What a floating scroll bar takes of the rows' right end.
    gutter: f32,
    /// Where the marker column starts, the same on every row.
    column: f32,
    /// A panel too narrow for the marker column beside the reading, which
    /// puts it on a line of its own.
    narrow: bool,
    /// A marker tag's fill, the graph's flag colour, and the faint `+` tag's
    /// outline.
    flag: egui::Color32,
    /// A marker's number on its tag.
    flag_text: egui::Color32,
    /// A dropped reading's tag, and a greyed number tag's fill.
    weak: egui::Color32,
}

impl LogLayout {
    /// What a marker row's number, ×, three gaps and the scroll bar's gutter
    /// take, the note aside.
    fn fixed(&self) -> f32 {
        self.number_width + self.delete_width + 3.0 * self.spacing + self.gutter
    }
}

/// A marker row's text, its wall time then its reading — the sample's line
/// when the log shows it, otherwise the line the marker kept — and what the
/// row says of a reading the graph or the recording has dropped.
fn marker_row_label<'a>(
    l10n: &'a Localizer,
    m: &Marker,
    sample: Option<&Measurement>,
    graph: &crate::graph::Graph,
    recording: &crate::recording::Recording,
) -> (String, Option<Cow<'a, str>>) {
    let reading = match sample {
        Some(measurement) => log_line(l10n, measurement),
        None => m.reading.clone(),
    };
    let tag = if !graph.holds(m.at) {
        Some(tr!(l10n, "markers-off-graph"))
    } else if recording.role() == BufferRole::Recording && !recording.in_recording(m.at) {
        Some(tr!(l10n, "markers-off-recording"))
    } else {
        None
    };
    (
        format!("{}  {reading}", m.wall_time.format("%H:%M:%S%.3f")),
        tag,
    )
}

/// The widest of the marker rows' readings with their tags: where the
/// marker column starts, room permitting.
fn reading_column_width(
    ui: &Ui,
    labels: &[(String, Option<Cow<str>>)],
    weak: egui::Color32,
) -> f32 {
    let small = egui::TextStyle::Small.resolve(ui.style());
    let spacing = ui.spacing().item_spacing.x;
    ui.fonts_mut(|f| {
        labels
            .iter()
            .map(|(text, tag)| {
                let mut width = |text: &str, font: &egui::FontId| {
                    f.layout_no_wrap(text.to_string(), font.clone(), weak)
                        .size()
                        .x
                };
                let reading = width(text, &log_font());
                reading + tag.as_deref().map_or(0.0, |t| spacing + width(t, &small))
            })
            .fold(0.0, f32::max)
    })
}

/// A sample's row: its line, and while the pointer is on the row, a faint
/// tag where a marker's would be that marks it. The reading to mark when
/// the tag was clicked.
fn sample_row(
    ui: &mut Ui,
    layout: &LogLayout,
    l10n: &Localizer,
    s: &Sample,
) -> Option<(Instant, String)> {
    let time = s.wall_time.format("%H:%M:%S%.3f");
    let row_rect = ui.max_rect();
    // One line, cut short of the tag it may offer: a wrapped line would run
    // into the row below.
    ui.set_max_width((row_rect.width() - layout.number_width - layout.spacing).max(0.0));
    let label = ui.add(
        egui::Label::new(
            RichText::new(format!("{time}  {}", log_line(l10n, &s.measurement))).font(log_font()),
        )
        .truncate(),
    );
    // A hovered row offers a marker: a faint tag where a marker's would be,
    // or past a longer reading. Over the row's share of the gaps too, so the
    // tag doesn't flicker off between rows.
    let pitch_rect = row_rect.expand2(egui::vec2(0.0, layout.row_spacing / 2.0));
    if !ui.rect_contains_pointer(pitch_rect) {
        return None;
    }
    let column = if layout.narrow { 0.0 } else { layout.column };
    let x = (label.rect.width().max(column) + layout.spacing)
        .min(row_rect.width() - layout.number_width);
    let slot = egui::Rect::from_min_size(
        row_rect.min + egui::vec2(x, 0.0),
        egui::vec2(layout.number_width, layout.line),
    );
    let plus = ui
        .painter()
        .layout_no_wrap("+".into(), log_font(), layout.flag);
    let (body, tip) = tag_shape(slot, plus.size().x);
    let add = tr!(l10n, "markers-add", time = time.to_string());
    let response = ui
        .interact(
            tag_hit_rect(body, tip),
            egui::Id::unique(("marker_add", s.measurement.timestamp)),
            egui::Sense::CLICK,
        )
        .on_hover_text(add.as_ref())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, add.as_ref()));
    let stroke = egui::Stroke::new(if response.hovered() { 1.5 } else { 1.0 }, layout.flag);
    ui.painter().add(egui::Shape::closed_line(
        vec![
            tip,
            body.left_top(),
            body.right_top(),
            body.right_bottom(),
            body.left_bottom(),
        ],
        stroke,
    ));
    ui.painter()
        .galley(body.center() - plus.size() / 2.0, plus, layout.flag);
    response
        .clicked()
        .then(|| (s.measurement.timestamp, log_line(l10n, &s.measurement)))
}

/// A marker row's reading and its tag, if any. The tag keeps its room: the
/// reading is what gets cut.
fn reading_label(ui: &mut Ui, layout: &LogLayout, text: &str, tag: Option<&str>) {
    let tag = tag.map(|tag| {
        egui::WidgetText::from(RichText::new(tag).small().color(layout.weak)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Small,
        )
    });
    let tag_width = tag.as_ref().map_or(0.0, |g| g.size().x + layout.spacing);
    ui.allocate_ui_with_layout(
        egui::vec2((ui.available_width() - tag_width).max(0.0), layout.line),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| ui.add(egui::Label::new(RichText::new(text).font(log_font())).truncate()),
    );
    if let Some(tag) = tag {
        ui.label(tag);
    }
}

/// Marker `m`'s number tag: a button, so Tab stops on it, that brings the
/// marker into view, where tabbing past it moves nothing. Drawn as the
/// graph's flag, pointing at its reading, and greyed once the graph has
/// dropped the reading (`on_graph`), with nowhere left to show it. Whether
/// it was pressed.
fn number_tag(
    ui: &mut Ui,
    layout: &LogLayout,
    l10n: &Localizer,
    m: &Marker,
    on_graph: bool,
) -> bool {
    let (number, at) = (m.number, m.at);
    let show = tr!(l10n, "markers-show", number = number);
    let tag = |ui: &mut Ui| {
        let (slot, _) = ui.allocate_exact_size(
            egui::vec2(layout.number_width, layout.line),
            egui::Sense::hover(),
        );
        let fill = if ui.is_enabled() {
            layout.flag
        } else {
            layout.weak
        };
        let galley = ui
            .painter()
            .layout_no_wrap(number.to_string(), log_font(), layout.flag_text);
        let (body, tip) = tag_shape(slot, galley.size().x);
        let response = ui.interact(
            tag_hit_rect(body, tip),
            egui::Id::unique(("marker_tag", at)),
            egui::Sense::click(),
        );
        let painter = ui.painter();
        painter.add(egui::Shape::convex_polygon(
            vec![body.left_top(), tip, body.left_bottom()],
            fill,
            egui::Stroke::NONE,
        ));
        painter.rect_filled(
            body,
            egui::CornerRadius {
                nw: 0,
                ne: 3,
                sw: 0,
                se: 3,
            },
            fill,
        );
        painter.galley(
            body.center() - galley.size() / 2.0,
            galley,
            layout.flag_text,
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), show.as_ref())
        });
        response
    };
    let button = ui
        .add_enabled(on_graph, tag)
        .on_hover_text(show.as_ref())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_disabled_hover_text(tr!(l10n, "markers-show.hover-disabled", number = number).as_ref())
        .a11y_label(&show);
    crate::a11y::paint_focus_ring(ui, &button);
    button.clicked()
}

/// Marker `number`'s ×, which deletes it. Whether it was pressed.
fn delete_button(ui: &mut Ui, layout: &LogLayout, l10n: &Localizer, number: u32) -> bool {
    let label = tr!(l10n, "markers-delete", number = number);
    ui.add_sized(
        [layout.delete_width, layout.line],
        egui::Button::new(RichText::new("\u{00D7}").font(log_font())),
    )
    .on_hover_text(label.as_ref())
    .a11y_label(&label)
    .clicked()
}

/// Marker `m`'s note, `width` wide.
fn note_field(ui: &mut Ui, l10n: &Localizer, m: &mut Marker, width: f32) -> egui::Response {
    let number = m.number;
    // Keyed by the reading, not the number: numbers start over once the list
    // empties, and a new marker must not inherit an old note's undo history.
    let id = egui::Id::unique(("marker_note", m.at));
    ui.add(
        egui::TextEdit::singleline(&mut m.note)
            .id(id)
            .hint_text(RichText::new(tr!(l10n, "markers-note")).font(log_font()))
            .font(log_font())
            .margin(egui::Margin::symmetric(4, 0))
            .char_limit(NOTE_MAX_CHARS)
            .desired_width(width),
    )
    .a11y_label(&tr!(l10n, "markers-note.a11y", number = number))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::connection::DmmMessage;
    use crate::app::test_support::test_app;
    use crate::recording::Recording;
    use dmm_lib::flags::StatusFlags;
    use eframe::egui::{Event, Modifiers, Pos2, Rect, vec2};
    use std::time::Duration;

    /// The log and the CSV must never disagree about an overload or an NCV
    /// level: the export has always written the word, so the log line has to
    /// as well, even when the protocol left digits in `display_raw`.
    #[test]
    fn the_log_line_names_an_overload_and_an_ncv_level() {
        let mut m = Measurement::test_fixture(MeasuredValue::Overload, "V", StatusFlags::default());
        m.display_raw = Some("      0".into());
        assert_eq!(log_line(&crate::l10n::english(), &m).trim(), "OL V");
        assert_eq!(m.value_export_str(), "OL");
        m.value = MeasuredValue::NcvLevel(2);
        assert!(
            log_line(&crate::l10n::english(), &m)
                .trim()
                .starts_with("NCV:2"),
            "{}",
            log_line(&crate::l10n::english(), &m)
        );
        assert_eq!(m.value_export_str(), "NCV:2");
    }

    /// Samples at `seconds` after `t0`.
    fn samples_at(t0: Instant, seconds: impl Iterator<Item = u64>) -> VecDeque<Sample> {
        seconds
            .map(|i| {
                let mut m = Measurement::test_fixture(
                    MeasuredValue::Normal(1.0),
                    "V",
                    StatusFlags::default(),
                );
                m.timestamp = secs(t0, i);
                Sample::from_measurement(&m, 0)
            })
            .collect()
    }

    /// Every row of `rows`, as `s<seconds>` for a sample, `m<index>` for a
    /// marker, and both for a marked sample.
    fn row_names(rows: &LogRows, samples: &VecDeque<Sample>, t0: Instant) -> Vec<String> {
        let at = |k: usize| (samples[k].measurement.timestamp - t0).as_secs();
        (0..rows.len())
            .map(|r| match rows.row(r) {
                Row::Sample(k) => format!("s{}", at(k)),
                Row::Marker(i) => format!("m{i}"),
                Row::Marked(k, i) => format!("s{}m{i}", at(k)),
            })
            .collect()
    }

    /// Each marker lands at its reading: on a shown sample's row, or in time
    /// order among them when the log doesn't show its sample.
    #[test]
    fn markers_sit_among_the_samples_in_time_order() {
        let t0 = Instant::now();
        let samples = samples_at(t0, 1..=3);
        let markers = [t0, secs(t0, 2), secs(t0, 9)];
        let rows = LogRows::new(&samples, 0..3, &markers);
        assert_eq!(
            row_names(&rows, &samples, t0),
            ["m0", "s1", "s2m1", "s3", "m2"]
        );
    }

    /// A marker on a sample the log doesn't show, older than its first,
    /// gets a row of its own at the top; with no samples shown, the
    /// markers are the log.
    #[test]
    fn a_marker_before_the_samples_shown_gets_its_own_row() {
        let t0 = Instant::now();
        let samples = samples_at(t0, 1..=4);
        let markers = [secs(t0, 1), secs(t0, 3)];
        let rows = LogRows::new(&samples, 2..4, &markers);
        assert_eq!(row_names(&rows, &samples, t0), ["m0", "s3m1", "s4"]);

        let rows = LogRows::new(&samples, 0..0, &markers);
        assert_eq!(row_names(&rows, &samples, t0), ["m0", "m1"]);
    }

    /// Two-pitch rows push the rows under them down, and a position inside
    /// one finds it.
    #[test]
    fn tall_rows_take_two_pitches() {
        let geometry = RowGeometry::new(10.0, vec![1, 3]);
        let tops: Vec<f32> = (0..5).map(|r| geometry.top(r)).collect();
        assert_eq!(tops, [0.0, 10.0, 30.0, 40.0, 60.0]);
        for (y, row) in [
            (0.0, 0),
            (9.0, 0),
            (10.0, 1),
            (29.0, 1),
            (30.0, 2),
            (45.0, 3),
            (59.0, 3),
            (60.0, 4),
        ] {
            assert_eq!(geometry.row_at(y), row, "at {y}");
        }
        assert_eq!(geometry.height(3), 20.0);
        assert_eq!(geometry.height(4), 10.0);
    }

    /// Marking a reading that has a marker, from the log or the graph's
    /// menu, opens the note already there.
    #[test]
    fn marking_a_marked_reading_opens_its_note() {
        let mut app = test_app();
        let at = Instant::now();
        send(&mut app, "DC V", at);
        app.add_marker(false);
        app.mark_reading(at, "1.234 V".into(), true);
        assert_eq!(numbers(&app), [1]);
        assert_eq!(app.marker_list.focus, Some(1));
    }

    /// A reading gone from both stores by the time it is marked — the plot's
    /// menu open across a graph restart — says so rather than adding a
    /// marker the next trim takes away.
    #[test]
    fn marking_a_reading_no_longer_held_says_so() {
        let mut app = test_app();
        let at = Instant::now();
        send(&mut app, "DC V", at);
        send(&mut app, "AC V", at + Duration::from_secs(1));
        app.mark_reading(at, "1.234 V".into(), true);
        assert!(app.markers.is_empty());
        assert_eq!(toast(&app), Some("No reading to mark yet"));
    }

    /// The log draws the rows in view, so a frame costs no more for a
    /// longer recording.
    #[test]
    #[ignore = "timing-sensitive; run with --release"]
    fn a_log_frame_costs_the_same_however_long_the_recording() {
        fn measure(samples: u64) -> Duration {
            let mut app = test_app();
            app.capture.recording.set_max_samples(samples as usize);
            app.toggle_recording();
            let t0 = Instant::now();
            let mut m = Measurement::test_fixture(
                MeasuredValue::Normal(1.234),
                "V",
                StatusFlags::default(),
            );
            for i in 0..samples {
                m.timestamp = t0 + Duration::from_millis(i * 10);
                app.capture.recording.push(&m, 0);
            }
            let ctx = crate::app::styled_context();
            let frame = |app: &mut App| {
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 400.0))),
                        ..Default::default()
                    },
                    |ui| app.show_log(ui, false),
                );
                out.textures_delta.clear();
            };
            for _ in 0..3 {
                frame(&mut app);
            }
            let start = Instant::now();
            for _ in 0..50 {
                frame(&mut app);
            }
            start.elapsed()
        }

        let short = measure(5_000);
        let long = measure(500_000);
        let ratio = long.as_secs_f64() / short.as_secs_f64().max(1e-9);
        println!("5K: {short:?}, 500K: {long:?}, ratio {ratio:.2}x");
        assert!(ratio < 2.0, "a log frame's cost grew with the recording");
    }

    /// Hand `app` a 1.234 reading in `mode` taken at `at`, then run the
    /// per-frame trim, as a frame does after draining.
    fn send(app: &mut App, mode: &'static str, at: Instant) {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(1.234), "V", StatusFlags::default());
        m.mode = mode.into();
        m.timestamp = at;
        app.deliver(DmmMessage::Measurement(m));
        app.trim_markers();
    }

    fn numbers(app: &App) -> Vec<u32> {
        app.markers.iter().map(|m| m.number).collect()
    }

    fn toast(app: &App) -> Option<&str> {
        app.toast.as_ref().map(|t| t.message.as_str())
    }

    fn secs(t0: Instant, s: u64) -> Instant {
        t0 + Duration::from_secs(s)
    }

    #[test]
    fn nothing_is_marked_before_a_reading() {
        let mut app = test_app();
        app.add_marker(false);
        assert!(app.markers.is_empty());
        assert_eq!(toast(&app), Some("No reading to mark yet"));
    }

    /// The reading on screen is what gets marked, once.
    #[test]
    fn n_marks_the_reading_on_screen() {
        let mut app = test_app();
        let t0 = Instant::now();
        send(&mut app, "DC V", t0);
        send(&mut app, "DC V", secs(t0, 1));
        app.add_marker(false);
        let m = app.markers.iter().next().expect("a marker");
        assert_eq!((m.number, m.at), (1, secs(t0, 1)));
        // Its log line, as the log shows the sample: the fixture's digits,
        // right-aligned in the value column.
        assert_eq!(m.reading, "     5.678 V");
        assert_eq!(toast(&app), None, "the new row is the feedback");

        app.add_marker(false);
        assert_eq!(numbers(&app), [1]);
        assert_eq!(toast(&app), Some("Marker 1 is already on this reading"));

        // Ctrl+N on the same reading opens the note already there.
        app.toast = None;
        app.add_marker(true);
        assert_eq!(numbers(&app), [1]);
        assert_eq!(app.marker_list.focus, Some(1));
        assert_eq!(toast(&app), None);
    }

    /// After Clear there is no reading left to mark, even if one is still
    /// on record somewhere.
    #[test]
    fn nothing_is_marked_after_clear() {
        let mut app = test_app();
        send(&mut app, "DC V", Instant::now());
        app.clear_session();
        app.add_marker(false);
        assert!(app.markers.is_empty());
        assert_eq!(toast(&app), Some("No reading to mark yet"));
    }

    /// With the Recording panel out of view, the toast says where notes are
    /// written.
    #[test]
    fn a_hidden_panel_says_how_to_bring_it_back() {
        let mut app = test_app();
        let t0 = Instant::now();
        send(&mut app, "DC V", t0);
        app.settings.show_recording = false;
        app.add_marker(true);
        assert_eq!(
            toast(&app),
            Some("Marker 1 added. Turn on Recording in Settings to add a note.")
        );
        assert_eq!(app.marker_list.focus, None, "no note to focus");

        send(&mut app, "DC V", secs(t0, 1));
        app.big_meter_mode = BigMeterMode::Full;
        app.add_marker(false);
        assert_eq!(
            toast(&app),
            Some("Marker 2 added. Leave big meter mode to add a note.")
        );
    }

    /// With nothing recorded, markers go with the readings the graph drops.
    #[test]
    fn history_markers_go_with_the_graph_restart() {
        let mut app = test_app();
        let t0 = Instant::now();
        send(&mut app, "DC V", t0);
        app.add_marker(false);
        send(&mut app, "AC V", secs(t0, 1));
        assert!(app.markers.is_empty(), "the graph restarted on AC V");
    }

    /// Record empties the sample buffer but the graph keeps its trace, and
    /// the markers on it.
    #[test]
    fn markers_placed_before_record_stay_while_the_graph_holds_them() {
        let mut app = test_app();
        let t0 = Instant::now();
        send(&mut app, "DC V", t0);
        app.add_marker(false);
        app.toggle_recording();
        assert!(app.capture.recording.active);
        send(&mut app, "DC V", secs(t0, 1));
        assert_eq!(numbers(&app), [1]);
        assert!(
            app.capture.recording.marked(app.markers.iter()).is_empty(),
            "not in the recording, so not in its export"
        );
        assert!(
            app.capture.recording.unsaved_marker_count(&app.markers) == 0,
            "a marker outside the recording is not the recording's to lose"
        );

        // Clear takes the graph's trace, and the recording never had it.
        app.clear_session();
        app.trim_markers();
        assert!(app.markers.is_empty());
    }

    /// A recording spans the graph's restarts, and so do its markers.
    #[test]
    fn a_recordings_markers_outlive_the_graph_restart() {
        let mut app = test_app();
        let t0 = Instant::now();
        app.toggle_recording();
        send(&mut app, "DC V", t0);
        app.add_marker(false);
        send(&mut app, "AC V", secs(t0, 1));
        assert_eq!(numbers(&app), [1]);
        assert_eq!(app.capture.recording.marked(app.markers.iter()).len(), 1);

        // Discard hands the buffer back to the graph, which no longer
        // holds that reading.
        app.capture.recording.toggle(Instant::now());
        app.capture.recording.discard();
        app.trim_markers();
        assert!(app.markers.is_empty());
    }

    /// A marker or note the recording holds and no file does is asked about
    /// before Record or Discard drops it; one outside the recording is not,
    /// and neither is an edit that ends where the file has it.
    #[test]
    fn a_recordings_marker_changes_count_as_unsaved() {
        let mut app = test_app();
        let t0 = Instant::now();
        send(&mut app, "DC V", t0);
        app.add_marker(false); // on the history, before Record
        app.toggle_recording();
        send(&mut app, "DC V", secs(t0, 1));
        let epoch = app.capture.recording.epoch();
        let saved =
            |app: &App| Recording::marker_keys(&app.capture.recording.marked(app.markers.iter()));
        app.capture.recording.mark_exported(epoch, 1, saved(&app));
        let asks = |app: &App| {
            app.capture.recording.unexported_count() > 0
                || app.capture.recording.unsaved_marker_count(&app.markers) > 0
        };
        assert!(!asks(&app));

        app.markers.iter_mut().next().unwrap().note = "before Record".into();
        assert!(!asks(&app), "marker 1 is on no sample of the recording");

        app.add_marker(false);
        assert!(asks(&app), "a new marker");
        app.markers.remove(2);
        assert!(!asks(&app), "added and deleted: the file has it as it is");

        app.add_marker(false);
        app.capture.recording.mark_exported(epoch, 1, saved(&app));
        app.markers.remove(3);
        assert!(!asks(&app), "deleted since the export: the file has more");
        app.add_marker(false);
        assert!(asks(&app), "a marker added since the export");
        app.capture
            .recording
            .mark_exported(epoch - 1, 1, saved(&app));
        assert!(asks(&app), "a file of an earlier recording saves none");
        app.capture.recording.mark_exported(epoch, 1, saved(&app));
        assert!(!asks(&app));
    }

    /// The two stores together need not hold one stretch of time: a marker
    /// on a reading between a stopped recording and a restarted graph is
    /// held by neither, and goes.
    #[test]
    fn a_marker_between_a_recording_and_a_restarted_graph_goes() {
        let mut app = test_app();
        let t0 = Instant::now();
        app.toggle_recording();
        send(&mut app, "DC V", t0);
        app.toggle_recording(); // stopped: the buffer keeps t0
        send(&mut app, "DC V", secs(t0, 5));
        app.add_marker(false); // on t0 + 5, on the graph only
        assert_eq!(numbers(&app), [1]);
        send(&mut app, "AC V", secs(t0, 9)); // the graph restarts
        assert!(app.markers.is_empty(), "t0 + 5 is in neither store");

        app.add_marker(false);
        assert_eq!(numbers(&app), [1], "the new reading, on the graph");
    }

    /// A click on a marker's flag opens its note, or says how to bring back
    /// the panel notes are written in.
    #[test]
    fn a_flag_click_opens_the_note() {
        let mut app = test_app();
        send(&mut app, "DC V", Instant::now());
        app.add_marker(false);
        app.open_marker_note(1);
        assert_eq!(app.marker_list.focus, Some(1));

        app.marker_list.focus = None;
        app.settings.show_recording = false;
        app.open_marker_note(1);
        assert_eq!(app.marker_list.focus, None);
        assert_eq!(
            toast(&app),
            Some("Turn on Recording in Settings to add a note.")
        );
    }

    /// Ctrl+N on a reading already marked opens that marker's note; with
    /// the panel hidden, the toast doesn't claim it was added.
    #[test]
    fn ctrl_n_on_a_marked_reading_says_so() {
        let mut app = test_app();
        send(&mut app, "DC V", Instant::now());
        app.add_marker(false);
        app.big_meter_mode = BigMeterMode::Full;
        app.add_marker(true);
        assert_eq!(
            toast(&app),
            Some("Marker 1 is already on this reading. Leave big meter mode to add a note.")
        );
    }

    /// The Markers list in a headless window, with the app's shortcuts
    /// handled first, as a frame does.
    struct Run {
        app: App,
        ctx: egui::Context,
        seconds: f64,
    }

    impl Run {
        fn new() -> Self {
            let mut app = test_app();
            send(&mut app, "DC V", Instant::now());
            Self {
                app,
                ctx: crate::app::styled_context(),
                seconds: 0.0,
            }
        }

        /// One frame; every key pressed in it is released at its end, as a
        /// tap is.
        fn frame(&mut self, mut events: Vec<Event>) {
            let released: Vec<Event> = events
                .iter()
                .filter_map(|e| match e {
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some(Event::Key {
                        key: *key,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers: *modifiers,
                    }),
                    _ => None,
                })
                .collect();
            events.extend(released);
            self.frame_held(events);
        }

        /// One frame, the keys pressed in it left held down.
        fn frame_held(&mut self, events: Vec<Event>) {
            self.seconds += 1.0;
            let app = &mut self.app;
            let mut out = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 400.0))),
                    events,
                    time: Some(self.seconds),
                    ..Default::default()
                },
                |ui| {
                    let ctx = ui.ctx().clone();
                    app.handle_keyboard_shortcuts(&ctx);
                    app.show_log(ui, false);
                },
            );
            out.textures_delta.clear();
        }

        /// A new reading, one second after the last.
        fn reading(&mut self) {
            let last = self.app.last_measurement.as_ref().map(|m| m.timestamp);
            send(
                &mut self.app,
                "DC V",
                last.unwrap_or_else(Instant::now) + Duration::from_secs(1),
            );
        }

        fn note_focused(&self) -> Option<u32> {
            let focused = self.ctx.memory(|m| m.focused())?;
            self.app
                .markers
                .iter()
                .find(|m| egui::Id::unique(("marker_note", m.at)) == focused)
                .map(|m| m.number)
        }

        fn note(&self, number: u32) -> &str {
            &self.app.markers.get(number).expect("the marker").note
        }
    }

    fn key(key: Key, modifiers: Modifiers, repeat: bool) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat,
            modifiers,
        }
    }

    fn text(s: &str) -> Event {
        Event::Text(s.to_string())
    }

    /// N places a marker and leaves the keyboard where it was; a held N
    /// places one.
    #[test]
    fn n_adds_a_marker_without_taking_the_focus() {
        let mut run = Run::new();
        run.frame_held(vec![key(Key::N, Modifiers::NONE, false), text("n")]);
        run.frame_held(vec![]);
        assert_eq!(numbers(&run.app), [1]);
        assert_eq!(run.note_focused(), None);

        // Still held: egui reports the next press as a repeat.
        run.reading();
        run.frame_held(vec![key(Key::N, Modifiers::NONE, true), text("n")]);
        assert_eq!(numbers(&run.app), [1], "a repeat is not a press");
    }

    /// A note opened by a click — a marker's flag on the graph — keeps the
    /// focus: the click landed outside the note, which egui answers by
    /// taking the focus back the frame the note asks for it.
    #[test]
    fn a_note_opened_by_a_click_keeps_the_focus() {
        let mut run = Run::new();
        run.frame(vec![key(Key::N, Modifiers::NONE, false), text("n")]);
        let at = Pos2::new(790.0, 390.0);
        let button = |pressed| Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        // The click lands as the note is asked for: a flag's click is
        // answered in the frame it is read, before the log is drawn.
        run.app.open_marker_note(1);
        run.frame_held(vec![Event::PointerMoved(at), button(true), button(false)]);
        run.frame(vec![]);
        assert_eq!(run.note_focused(), Some(1));
        run.frame(vec![text("load on")]);
        assert_eq!(run.note(1), "load on");
    }

    /// Ctrl+N, a note, Enter: the note is kept and the keys are the app's
    /// again, so the next N places the next marker.
    #[test]
    fn ctrl_n_then_typing_then_enter_writes_the_note() {
        let mut run = Run::new();
        run.frame(vec![key(Key::N, Modifiers::COMMAND, false)]);
        run.frame(vec![]);
        assert_eq!(run.note_focused(), Some(1));

        // N inside a note is a letter.
        run.frame(vec![
            text("load o"),
            key(Key::N, Modifiers::NONE, false),
            text("n"),
        ]);
        assert_eq!(numbers(&run.app), [1]);
        assert_eq!(run.note(1), "load on");

        run.frame(vec![key(Key::Enter, Modifiers::NONE, false)]);
        run.frame(vec![]);
        assert_eq!(run.note_focused(), None);
        assert_eq!(run.note(1), "load on");

        run.reading();
        run.frame(vec![key(Key::N, Modifiers::NONE, false), text("n")]);
        assert_eq!(numbers(&run.app), [1, 2]);
    }

    /// A marker trimmed away while its note is being written ends the edit,
    /// and the log follows its newest row again as it did before.
    #[test]
    fn a_marker_trimmed_mid_edit_ends_the_edit() {
        let mut run = Run::new();
        run.frame(vec![key(Key::N, Modifiers::COMMAND, false)]);
        run.frame(vec![]);
        assert_eq!(run.note_focused(), Some(1));
        assert!(run.app.marker_list.editing.is_some());
        run.app.graph.clear();
        run.app.capture.recording.clear_history();
        run.app.trim_markers();
        run.frame(vec![]);
        assert!(run.app.marker_list.editing.is_none());
    }

    /// Esc leaves the note as it was before editing began.
    #[test]
    fn esc_puts_the_note_back() {
        let mut run = Run::new();
        run.frame(vec![key(Key::N, Modifiers::COMMAND, false)]);
        run.frame(vec![]);
        run.frame(vec![text("fan on")]);
        run.frame(vec![key(Key::Enter, Modifiers::NONE, false)]);

        run.app.marker_list.focus = Some(1);
        run.frame(vec![]);
        run.frame(vec![]);
        assert_eq!(run.note_focused(), Some(1));
        run.frame(vec![text(" and off")]);
        assert_eq!(run.note(1), "fan on and off");
        run.frame(vec![key(Key::Escape, Modifiers::NONE, false)]);
        run.frame(vec![]);
        assert_eq!(run.note_focused(), None);
        assert_eq!(run.note(1), "fan on");
    }

    /// Numbers start over once the list empties, but a note's undo
    /// history stays with its own marker: undo in the new marker 1's note
    /// brings back nothing of the old one's.
    #[test]
    fn a_renumbered_marker_starts_with_a_clean_note() {
        let mut run = Run::new();
        run.frame(vec![key(Key::N, Modifiers::COMMAND, false)]);
        run.frame(vec![]);
        run.frame(vec![text("fan on")]);
        run.frame(vec![key(Key::Enter, Modifiers::NONE, false)]);
        run.app.clear_session();
        run.app.trim_markers();
        assert!(run.app.markers.is_empty());

        run.reading();
        run.frame(vec![key(Key::N, Modifiers::COMMAND, false)]);
        run.frame(vec![]);
        assert_eq!(run.note_focused(), Some(1));
        run.frame(vec![key(Key::Z, Modifiers::COMMAND, false)]);
        run.frame(vec![]);
        assert_eq!(run.note(1), "");
    }

    /// Ctrl+N from inside one note moves on to a new marker's.
    #[test]
    fn ctrl_n_moves_from_one_note_to_the_next() {
        let mut run = Run::new();
        run.frame(vec![key(Key::N, Modifiers::COMMAND, false)]);
        run.frame(vec![]);
        run.frame(vec![text("load on")]);
        run.reading();
        run.frame(vec![key(Key::N, Modifiers::COMMAND, false)]);
        run.frame(vec![]);
        assert_eq!(run.note_focused(), Some(2));
        assert_eq!(run.note(1), "load on");
    }
}
