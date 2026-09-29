//! The reading pipeline: what every reading from the meter goes through —
//! the software transform, the session statistics, the graph, the history
//! and the recording — in that order, and the stores it is kept in.

use dmm_lib::measurement::{MainLabel, Measurement};
use dmm_lib::protocol::registry::SelectableDevice;
use dmm_lib::stats::SeriesStats;
use dmm_lib::transform::Transform;
use dmm_lib::transport::Link;

use super::Connection;
use super::plot_input::{PlotInput, Plotted, resolve_plot_input};
use crate::graph::{Graph, PlotSample};
use crate::recording::Recording;

/// Provenance and column layout the sample buffer is exported with. Taken
/// when recording starts — or, for the history, from the connection its first
/// reading arrived on — and kept across disconnect, so a file describes the
/// meter its samples came from rather than whatever is selected at export
/// time.
#[derive(Default)]
pub(super) struct CaptureLayout {
    /// Meter the buffered samples came from. Outlives disconnect so a capture
    /// can still be exported with the right provenance after the meter is
    /// unplugged. Owned for an imported file, which may name a meter the
    /// registry doesn't.
    pub(super) device: Option<std::borrow::Cow<'static, str>>,
    /// Registry id of that meter, for a replay file's `# device:` line, taken
    /// at the same moment and for the same reason as `device`.
    ///
    /// `None` for the mock, whose readings are synthesised rather than decoded
    /// from frames — there is nothing to play back.
    pub(super) device_id: Option<&'static str>,
    /// Whether that meter's protocol was short of verified, for the JSON
    /// export's `experimental` field. Taken alongside `device` and for the
    /// same reason: disconnecting clears the connection's stability, so read
    /// at export time it would mark an unplugged UT181A's readings verified.
    ///
    /// `None` until a meter has been connected during the recording — Record
    /// works while disconnected, and the stability read then is the default
    /// the disconnect left behind, not a meter's. The export falls back to the
    /// live connection, as `device` and `device_id` do.
    pub(super) experimental: Option<bool>,
    /// The link those samples came over, for the replay export's `# link:`
    /// line. Taken alongside `device_id` and for the same reason: a
    /// disconnect clears the connection's link, and a file exported after
    /// unplugging would then claim the cable every unmarked file is read as.
    pub(super) link: Option<Link>,
    /// Sub-value slots the meter itself can fill in the buffered samples,
    /// taken alongside `device` and for the same reason: the CSV column layout has to describe the meter the
    /// samples came from, not whatever is selected at export time.
    pub(super) aux_slots: usize,
    /// Extra sub-value slots the export reserves *after* the meter's own, for
    /// the ones software appends (a transform's `Raw`). Kept apart from
    /// `aux_slots` so `Raw` gets a fixed trailing column instead of sliding
    /// forward whenever the meter sends fewer sub-values. Only ever grows
    /// during a recording — turning a scale off mid-capture leaves the
    /// trailing group empty rather than renumbering the columns already
    /// written into the user's mental model of the file.
    pub(super) extra_slots: usize,
}

impl CaptureLayout {
    /// The layout for samples from `meter`, as Record and the history's
    /// first reading both latch it.
    pub(super) fn new(
        meter: Option<&'static SelectableDevice>,
        experimental: Option<bool>,
        link: Option<Link>,
        aux_slots: usize,
        extra_slots: usize,
    ) -> Self {
        Self {
            device: meter.map(|d| d.display_name.into()),
            // Only a meter's frames can be replayed, so the mock names no
            // device here and the export offers no replay file for it.
            device_id: meter.filter(|d| d.requires_hardware).map(|d| d.id),
            experimental,
            link,
            aux_slots,
            extra_slots,
        }
    }
}

/// The stores the reading pipeline fills, apart from the graph: the session
/// statistics, the sample buffer, and the layouts its two exports are
/// written with.
pub(super) struct Capture {
    /// Min/max/avg and the running integral of the current series. The GUI
    /// always integrates: the stats panel shows the integral whenever the
    /// current unit has a meaningful one.
    pub(super) session: SeriesStats,
    pub(super) recording: Recording,
    /// What the recording's export names and lays out, latched at Record.
    pub(super) recording_layout: CaptureLayout,
    /// The same for the graph's history, latched as it starts.
    pub(super) history_layout: CaptureLayout,
    /// Sub-value slots the connected meter family can report, from its
    /// profile. 0 until the first `Connected`, and kept on disconnect so a
    /// capture stays exportable with its full column layout.
    pub(super) device_aux_slots: usize,
}

impl Capture {
    /// Empty stores, the sample buffer bounded at `max_samples`.
    pub(super) fn new(max_samples: usize) -> Self {
        let mut recording = Recording::new();
        // A fresh buffer holds nothing, so this cannot stop anything.
        recording.set_max_samples(max_samples);
        Self {
            session: SeriesStats::new(true),
            recording,
            recording_layout: CaptureLayout::default(),
            history_layout: CaptureLayout::default(),
            device_aux_slots: 0,
        }
    }

    /// Take one reading from `connection` through the pipeline: `transform`,
    /// then the session statistics, then `graph`, then the history and a
    /// running recording.
    ///
    /// Returns the reading as transformed, for the display, and whether it
    /// was the one that filled the recording.
    pub(super) fn ingest(
        &mut self,
        mut m: Measurement,
        transform: &Transform,
        graph: &mut Graph,
        connection: &Connection,
    ) -> (Measurement, bool) {
        // The single point a software transform is applied. Every consumer
        // below — the session statistics, the graph's series list and plot
        // input, the recording buffer and `last_measurement` — then sees one
        // already-scaled reading, and none of them has to know transforms
        // exist. No-op when identity.
        transform.apply(&mut m);

        // Session stats follow the meter's *main* reading whatever the graph
        // plots: they describe the reading, not the view. `SeriesStats`
        // resets them on a mode *or* unit change — a dial turn, or an
        // auto-range step that moves the unit a decade (mV→V) without
        // touching the mode — so the panel never labels volt-scale numbers
        // with an ohms unit. `Graph::push_sample` clears its history on the
        // same condition, and the GUI resets silently, so the returned
        // `SeriesChange` is not needed here.
        self.session.push(&m);

        plot(graph, &m);

        // `m` has already been through the transform, so the count it
        // appended is what this sample carries.
        let filled = self.keep_sample(&m, transform.extra_aux_count(), graph, connection);
        (m, filled)
    }

    /// Take one reading of an imported file through the pipeline: as
    /// [`Capture::ingest`], less the software transform — the file holds the
    /// readings as they were shown, scaled or not — and less the meter the
    /// connection names: the recording the import runs was latched from the
    /// file. Returns whether it filled the recording.
    pub(super) fn ingest_imported(&mut self, m: &Measurement, graph: &mut Graph) -> bool {
        self.session.push(m);
        plot(graph, m);
        if let Some(start) = graph.first_point_time() {
            self.recording.trim_before(start);
        }
        self.recording.push(m, 0)
    }

    /// Keep a reading: in the graph's history, which Export… saves with
    /// nothing recorded, cut to what the graph holds — and in a running
    /// recording. Returns whether it filled the recording.
    ///
    /// Runs after the graph has taken the reading, so a mode change the graph
    /// restarted on already shows as its first point. An empty graph cuts
    /// nothing: over-range readings add no point, and are still readings to
    /// export.
    fn keep_sample(
        &mut self,
        m: &Measurement,
        extra_aux: usize,
        graph: &Graph,
        connection: &Connection,
    ) -> bool {
        // `detected`, not the selection: a device picked in Settings takes
        // effect at the next connect, and this reading may still be the old
        // meter's. A file names one meter, so another one starts another
        // history.
        let meter = connection.detected();
        if meter.map(|d| d.display_name) != self.history_layout.device.as_deref() {
            self.recording.clear_history();
        }
        if let Some(start) = graph.first_point_time() {
            self.recording.trim_before(start);
        }
        if self.recording.history_is_empty() {
            // The history's counterpart of what Record latches, from the
            // connection its first reading arrived on. A reading only
            // arrives on a live connection, so this stability and this link
            // are the meter's rather than the defaults a disconnect leaves
            // behind. A scale change clears the history, so the transform in
            // force now is the one every sample in it went through.
            self.history_layout = CaptureLayout::new(
                meter,
                Some(!connection.stability().is_verified()),
                connection.link(),
                self.device_aux_slots,
                extra_aux,
            );
        }
        self.recording.push(m, extra_aux)
    }
}

/// Hand one transformed reading to the graph: the sub-values it offers, and
/// the point, break or gap it adds to the plotted series.
fn plot(graph: &mut Graph, m: &Measurement) {
    // Offer this frame's sub-values before resolving what to plot, so that
    // the frame the graph finally gives the selection up on is also the one
    // that plots the main reading again, not the one after it. Until then a
    // frame of the plotted mode missing the selected sub-value adds no point
    // to its trace, only to the ones beside it (see `resolve_plot_input`). A
    // sub-value with no value in this frame (a scale's Raw beside the
    // UT61E+'s AC+DC V AC component) is not offered.
    let options: Vec<(&str, &str)> = m
        .present_aux()
        .map(|aux| (aux.label.as_ref(), aux.unit_or(&m.unit)))
        .collect();
    graph.set_series_options(&options, m.timestamp);
    // Owned, so the plot input doesn't hold the graph borrowed while it is
    // pushed into. Only while a sub-value is selected — the common case
    // allocates nothing.
    let selected = graph
        .selected_series_offer()
        .map(|(label, unit)| (label.to_string(), unit.to_string()));
    let plotted_mode = selected
        .as_ref()
        .and_then(|_| graph.plotted_mode().map(str::to_string));
    let input = resolve_plot_input(
        m,
        selected.as_ref().map(|(l, u)| (l.as_str(), u.as_str())),
        plotted_mode.as_deref(),
    );
    let Some(PlotInput {
        plotted,
        levels,
        unit,
        display_raw,
        series,
        overlays,
    }) = input
    else {
        return;
    };
    let sample = |value| PlotSample {
        value,
        timestamp: m.timestamp,
        mode: &m.mode,
        unit,
        display_raw,
        series,
        main_label: m.main_label.map(MainLabel::as_str),
        levels,
        overlays: &overlays,
    };
    match plotted {
        Plotted::Point(v) => graph.push_sample(sample(Some(v))),
        // A word instead of a reading ("Auto" with the probes lifted): a
        // break too, but not an over-range one. It never reaches
        // `push_sample`, so its mode and unit cannot restart the trace
        // either.
        Plotted::NoReading => graph.push_no_reading(m.timestamp),
        // The plotted series is over range: no point, but the trace has to
        // break so it isn't drawn straight through the excursion. The
        // sub-values beside it are not over range and keep theirs. The
        // sample goes first: a frame that switches the plotted series swaps
        // it in, or one in a new mode restarts the trace, and the break lands
        // on it.
        Plotted::OverRange => {
            graph.push_sample(sample(None));
            graph.push_break(m.timestamp);
        }
        Plotted::Absent => graph.push_sample(sample(None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::flags::StatusFlags;
    use dmm_lib::measurement::MeasuredValue;

    /// The transform comes first: the statistics, the graph, the history and
    /// the reading handed back for the display all see the scaled value in
    /// the relabelled unit, never the meter's own.
    #[test]
    fn every_store_sees_the_transformed_reading() {
        let clock = dmm_lib::Clock::real();
        let transform = Transform::linear(2.0, 0.0, Some("A".to_string()));
        let mut capture = Capture::new(100);
        let mut graph = Graph::new();
        let reading = Measurement {
            timestamp: clock.now(),
            ..Measurement::test_fixture(MeasuredValue::Normal(1.5), "V", StatusFlags::default())
        };

        let (shown, filled) =
            capture.ingest(reading, &transform, &mut graph, &Connection::default());

        assert!(!filled, "nothing is recording");
        let scaled = |v: &MeasuredValue| matches!(v, MeasuredValue::Normal(x) if *x == 3.0);
        assert!(scaled(&shown.value), "shown {:?}", shown.value);
        assert_eq!(shown.unit, "A");
        assert_eq!(capture.session.stats.min, Some(3.0));
        assert_eq!(graph.plotted_unit(), "A");
        let kept: Vec<_> = capture.recording.history_samples().collect();
        assert_eq!(kept.len(), 1);
        assert!(scaled(&kept[0].measurement.value));
        assert_eq!(kept[0].measurement.unit, "A");
        assert_eq!(
            capture.history_layout.extra_slots,
            transform.extra_aux_count(),
            "the history latched the scale in force"
        );
    }
}
