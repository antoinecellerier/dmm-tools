//! Reducing one measurement to what the graph should plot: which series is
//! drawn, in which unit, and which same-unit sub-values ride along beside it.

use dmm_lib::measurement::{MeasuredValue, Measurement};

use crate::graph::MAX_OVERLAYS;

/// One measurement reduced to what the graph should plot.
///
/// The graph never sees `dmm_lib` measurement types; this is where the
/// selected series and its same-unit companions are picked out.
pub(super) struct PlotInput<'a> {
    /// The plotted series' value, or `None` when it is over range.
    pub value: Option<f64>,
    /// Unit of the plotted series — the meter's, or the sub-value's own.
    pub unit: &'a str,
    pub display_raw: Option<&'a str>,
    /// Label of the plotted sub-value, or `None` for the main reading.
    pub series: Option<&'a str>,
    /// Same-unit sub-values to draw beside it.
    pub overlays: Vec<(&'a str, Option<f64>)>,
}

/// What a measured value contributes to a plot: `Some(Some(v))` for a
/// reading, `Some(None)` for an over-range one (a break in the trace, but
/// still a measurement), `None` for something with no place on a value axis.
fn plottable_value(v: &MeasuredValue) -> Option<Option<f64>> {
    match v {
        MeasuredValue::Normal(v) => Some(Some(*v)),
        MeasuredValue::Overload => Some(None),
        // NCV is a bar-graph level, not a quantity — plotting it against a
        // volt axis would be meaningless.
        MeasuredValue::NcvLevel(_) => None,
    }
}

/// Decide what the graph plots for this measurement, given the toolbar's
/// series selection.
///
/// Only sub-values sharing the plotted series' unit become overlays. A
/// frequency in Hz beside an AC voltage measures something else entirely, and
/// drawing it on the volt axis would invent a relationship that isn't there —
/// it stays reachable through the selector instead.
pub(super) fn resolve_plot_input<'a>(
    m: &'a Measurement,
    selected: Option<&str>,
) -> Option<PlotInput<'a>> {
    let main_unit: &str = &m.unit;
    // A frame that omits the selected sub-value is skipped, not plotted:
    // falling back to the main reading would hand `push_sample` a `series` of
    // `None`, which it reads as a change of plotted series and answers by
    // clearing the history, the cursors and the pinned Y range. A short or
    // bit-clear frame must not do that. Gap detection is time-based, so
    // dropping one frame at the normal cadence leaves no visible hole, and
    // once the graph has given the selection up for good (`selected` is
    // `None`) the main reading is plotted and that restart is the intended
    // "the meter left the mode" behaviour.
    let plotted = match selected {
        Some(sel) => Some(m.aux_values.iter().find(|a| a.label.as_ref() == sel)?),
        None => None,
    };

    let (value, unit, display_raw, series) = match plotted {
        Some(aux) => (
            plottable_value(&aux.value)?,
            aux.unit_or(main_unit),
            aux.display_raw.as_deref(),
            // Borrowed from the aux rather than from `selected`, so the
            // caller's borrow of the graph ends at the call.
            Some(aux.label.as_ref()),
        ),
        None => (
            plottable_value(&m.value)?,
            main_unit,
            m.display_raw.as_deref(),
            None,
        ),
    };

    let mut overlays: Vec<(&str, Option<f64>)> = Vec::new();
    for aux in &m.aux_values {
        if overlays.len() >= MAX_OVERLAYS {
            break;
        }
        if Some(aux.label.as_ref()) == series || aux.unit_or(main_unit) != unit {
            continue;
        }
        if let Some(v) = plottable_value(&aux.value) {
            overlays.push((aux.label.as_ref(), v));
        }
    }
    // Plotting a sub-value: the meter's own reading is the natural companion
    // whenever it measures the same quantity — choosing T2 should still show
    // T1 next to it.
    if series.is_some()
        && main_unit == unit
        && overlays.len() < MAX_OVERLAYS
        && let Some(v) = plottable_value(&m.value)
    {
        overlays.push(("Main", v));
    }

    Some(PlotInput {
        value,
        unit,
        display_raw,
        series,
        overlays,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use dmm_lib::flags::StatusFlags;
    use dmm_lib::measurement::AuxValue;
    use dmm_lib::transform::{RAW_LABEL, Transform};

    fn aux(label: &'static str, unit: &'static str, value: MeasuredValue) -> AuxValue {
        AuxValue {
            label: label.into(),
            value,
            unit: unit.into(),
            display_raw: None,
            elapsed_secs: None,
        }
    }

    fn meter(value: f64, unit: &'static str, aux_values: Vec<AuxValue>) -> Measurement {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(value), unit, StatusFlags::default());
        m.aux_values = aux_values;
        m
    }

    /// A UT181A in V AC + Hz sends the frequency and the period beside the
    /// voltage. Neither measures volts, so neither belongs on the volt axis —
    /// they are reachable only through the selector.
    #[test]
    fn different_unit_sub_values_are_never_overlaid() {
        let m = meter(
            239.22,
            "VAC",
            vec![
                aux("Frequency", "Hz", MeasuredValue::Normal(50.01)),
                aux("Period", "ms", MeasuredValue::Normal(20.0)),
                aux("Max", "VAC", MeasuredValue::Normal(240.5)),
            ],
        );
        let plot = resolve_plot_input(&m, None).expect("main reading is plottable");
        assert_eq!(plot.value, Some(239.22));
        assert_eq!(plot.unit, "VAC");
        assert_eq!(plot.series, None);
        assert_eq!(plot.overlays, vec![("Max", Some(240.5))]);
    }

    /// Protocols leave the unit empty when a sub-value shares the main
    /// reading's — a relative reference or a MIN/MAX sample always measures
    /// the same quantity as the value it tracks.
    #[test]
    fn an_empty_aux_unit_resolves_to_the_main_unit_and_is_overlaid() {
        let m = meter(
            4.9871,
            "V",
            vec![aux("Max", "", MeasuredValue::Normal(5.0123))],
        );
        let plot = resolve_plot_input(&m, None).expect("plottable");
        assert_eq!(plot.overlays, vec![("Max", Some(5.0123))]);
    }

    /// Selecting a sub-value in a different unit rescales the whole plot to
    /// it, and nothing else on the frame shares that unit.
    #[test]
    fn a_selected_different_unit_sub_value_is_plotted_alone() {
        let m = meter(
            239.22,
            "VAC",
            vec![
                aux("Frequency", "Hz", MeasuredValue::Normal(50.01)),
                aux("Period", "ms", MeasuredValue::Normal(20.0)),
            ],
        );
        let plot = resolve_plot_input(&m, Some("Frequency")).expect("plottable");
        assert_eq!(plot.value, Some(50.01));
        assert_eq!(plot.unit, "Hz");
        assert_eq!(plot.series, Some("Frequency"));
        assert!(plot.overlays.is_empty(), "got {:?}", plot.overlays);
    }

    /// Plotting T2 must still show T1 — the reading the meter calls its main
    /// one measures the same quantity.
    #[test]
    fn a_selected_same_unit_sub_value_keeps_the_main_reading_beside_it() {
        let m = meter(
            23.5,
            "\u{00B0}C",
            vec![aux("T2", "\u{00B0}C", MeasuredValue::Normal(24.1))],
        );
        let plot = resolve_plot_input(&m, Some("T2")).expect("plottable");
        assert_eq!(plot.value, Some(24.1));
        assert_eq!(plot.series, Some("T2"));
        assert_eq!(plot.overlays, vec![("Main", Some(23.5))]);
    }

    /// The App picks which series make the cut, so it must hand the graph no
    /// more than the graph will register — `push_sample` drops the surplus
    /// silently, which would show as a series vanishing from the key.
    #[test]
    fn same_unit_sub_values_past_the_cap_are_dropped_here_not_by_the_graph() {
        let aux_values = ["A1", "A2", "A3", "A4", "A5", "A6"]
            .into_iter()
            .enumerate()
            .map(|(i, label)| aux(label, "V", MeasuredValue::Normal(i as f64)))
            .collect::<Vec<_>>();
        assert_eq!(
            aux_values.len(),
            MAX_OVERLAYS + 2,
            "fixture overfills the cap"
        );
        let m = meter(4.9, "V", aux_values);
        let plot = resolve_plot_input(&m, None).expect("plottable");
        assert_eq!(plot.overlays.len(), MAX_OVERLAYS, "got {:?}", plot.overlays);
    }

    /// "Main" is an overlay like any other: with the cap already full it has
    /// to be left out, not appended past it.
    #[test]
    fn the_main_reading_is_not_overlaid_past_the_cap() {
        let mut aux_values = vec![aux("Sel", "V", MeasuredValue::Normal(1.0))];
        aux_values.extend(
            ["A1", "A2", "A3", "A4"]
                .into_iter()
                .map(|label| aux(label, "V", MeasuredValue::Normal(2.0))),
        );
        assert_eq!(
            aux_values.len() - 1,
            MAX_OVERLAYS,
            "fixture must fill the cap without the main reading"
        );
        let m = meter(4.9, "V", aux_values);
        let plot = resolve_plot_input(&m, Some("Sel")).expect("plottable");
        assert_eq!(plot.series, Some("Sel"));
        assert_eq!(plot.overlays.len(), MAX_OVERLAYS, "got {:?}", plot.overlays);
        assert!(
            !plot.overlays.iter().any(|(label, _)| *label == "Main"),
            "got {:?}",
            plot.overlays
        );
    }

    /// A frame that omits the selected sub-value is skipped, not plotted as
    /// the main reading: the graph reads a change of series as a restart and
    /// would throw the trace away over one short frame.
    #[test]
    fn a_frame_without_the_selected_sub_value_plots_nothing() {
        let m = meter(4.9, "V", vec![]);
        assert!(resolve_plot_input(&m, Some("T2")).is_none());
    }

    /// Once the graph has given the selection up, the main reading is plotted
    /// again — and the restart that comes with it is the intended answer to
    /// the meter having left the mode.
    #[test]
    fn a_dropped_selection_plots_the_main_reading() {
        let m = meter(4.9, "V", vec![]);
        let plot = resolve_plot_input(&m, None).expect("plottable");
        assert_eq!(plot.value, Some(4.9));
        assert_eq!(plot.series, None);
    }

    /// An over-range sub-value is still a measurement — it breaks its own
    /// trace without removing it from the plot.
    #[test]
    fn an_overloaded_sub_value_overlays_as_a_break() {
        let m = meter(4.9871, "V", vec![aux("Max", "", MeasuredValue::Overload)]);
        let plot = resolve_plot_input(&m, None).expect("plottable");
        assert_eq!(plot.overlays, vec![("Max", None)]);
    }

    /// An over-range *plotted* series yields no point at all; the App turns
    /// this into a break in the trace.
    #[test]
    fn an_overloaded_selected_series_has_no_value() {
        let m = meter(4.9871, "V", vec![aux("Max", "", MeasuredValue::Overload)]);
        let plot = resolve_plot_input(&m, Some("Max")).expect("still a series");
        assert_eq!(plot.value, None);
        assert_eq!(plot.series, Some("Max"));
    }

    /// A software scale with no relabel leaves both readings in the same
    /// unit, so plotting the meter's own `Raw` value still shows the scaled
    /// reading beside it.
    #[test]
    fn a_same_unit_raw_sub_value_keeps_the_scaled_reading_beside_it() {
        let mut m = meter(5.678, "V", vec![]);
        Transform::linear(0.1, 0.0, None).apply(&mut m);
        assert_eq!(m.unit, "V");

        let plot = resolve_plot_input(&m, Some(RAW_LABEL)).expect("plottable");
        assert_eq!(plot.series, Some(RAW_LABEL));
        assert_eq!(plot.value, Some(5.678));
        assert_eq!(plot.overlays.len(), 1, "got {:?}", plot.overlays);
        assert_eq!(plot.overlays[0].0, "Main");
        let main = plot.overlays[0].1.expect("the scaled reading is plottable");
        assert!((main - 0.5678).abs() < 1e-9, "got {main}");
    }

    /// A 10 mV/A clamp relabelled to amps: `Raw` is still millivolts, so it
    /// must not be drawn on the amp axis — but it stays selectable.
    #[test]
    fn a_relabelled_raw_sub_value_is_selectable_but_never_overlaid() {
        let mut m = meter(123.4, "mV", vec![]);
        Transform::linear(100.0, 0.0, Some("A".to_string())).apply(&mut m);
        assert_eq!(m.unit, "A");

        let plot = resolve_plot_input(&m, None).expect("plottable");
        assert_eq!(plot.unit, "A");
        assert!(plot.overlays.is_empty(), "got {:?}", plot.overlays);

        let raw = resolve_plot_input(&m, Some(RAW_LABEL)).expect("Raw is a series");
        assert_eq!(raw.series, Some(RAW_LABEL));
        assert_eq!(raw.unit, "mV");
        assert_eq!(raw.value, Some(123.4));
        assert!(raw.overlays.is_empty(), "got {:?}", raw.overlays);
    }

    /// NCV is a bar-graph level, not a quantity on a value axis.
    #[test]
    fn an_ncv_reading_is_not_plotted() {
        let m = Measurement::test_fixture(MeasuredValue::NcvLevel(3), "", StatusFlags::default());
        assert!(resolve_plot_input(&m, None).is_none());
    }
}
