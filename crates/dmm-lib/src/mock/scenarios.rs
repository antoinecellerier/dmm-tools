//! The scenario catalogue: what the mock measures and how its readings move.
//!
//! One entry per [`MockMode`], each a mode's fixed fields plus the waveform
//! its readings trace. Nothing here knows about button presses or meter
//! state — [`super::state`] holds that.

use std::f64::consts::TAU;

use super::MockMode;
use crate::measurement::MeasuredValue;

/// A scenario defines a measurement mode with a time-varying value pattern.
///
/// Values are a pure function of elapsed seconds since the scenario started,
/// so displayed waveforms trace smooth curves regardless of read cadence or
/// scheduling jitter. Each waveform is periodic over `duration_secs` — it
/// returns identical values at `t = 0` and `t = duration_secs`, so the
/// scenario can loop without a visible jump when it wraps.
///
/// `Copy` so a reading can be built from a snapshot of the live scenario
/// while the meter state it is applied to is borrowed mutably.
#[derive(Clone, Copy)]
pub(super) struct Scenario {
    pub(super) id: MockMode,
    pub(super) mode: &'static str,
    pub(super) mode_raw: u16,
    pub(super) range_raw: u8,
    pub(super) unit: &'static str,
    pub(super) range_label: &'static str,
    pub(super) range_max: f64,
    /// Period of the waveform, and of the auto-cycle's stay on this
    /// scenario. Must be positive — the value functions divide by it, which
    /// `every_scenario_has_a_positive_duration` enforces.
    pub(super) duration_secs: f64,
    pub(super) value_fn: fn(f64, f64) -> MeasuredValue,
    /// Secondary readings emitted with every measurement. Empty for the
    /// single-display scenarios.
    pub(super) aux: &'static [AuxSpec],
}

impl Scenario {
    /// Whether the Peak command does anything here.
    ///
    /// The UT61E+ the mock stands in for silently ignores Peak on DC
    /// (spec §2.7), so the DC scenarios ignore it too.
    pub(super) fn peak_applies(&self) -> bool {
        !matches!(
            self.id,
            MockMode::DcV | MockMode::DcMa | MockMode::OhmOl | MockMode::Noise
        )
    }
}

/// A secondary reading a scenario emits alongside its main value.
///
/// Same `(elapsed, duration)` signature as the main `value_fn`, so sub-values
/// are evaluated at the same instant as the main reading and stay consistent
/// with it (and with each other) at any read cadence.
pub(super) struct AuxSpec {
    pub(super) label: &'static str,
    /// Unit of this sub-value. Empty means "same as the main reading", the
    /// convention `AuxValue::unit_or` resolves.
    pub(super) unit: &'static str,
    pub(super) value_fn: fn(f64, f64) -> MeasuredValue,
}

/// Triangle wave on `[lo, hi]`, period-1 in phase. `phase = 0` and `phase = 1`
/// both map to `lo`, so the wave loops continuously.
fn triangle(phase: f64, lo: f64, hi: f64) -> f64 {
    let p = phase.rem_euclid(1.0);
    let span = hi - lo;
    if p < 0.5 {
        lo + p * 2.0 * span
    } else {
        hi - (p - 0.5) * 2.0 * span
    }
}

fn dcv_value(t: f64, duration: f64) -> MeasuredValue {
    // One full sine cycle per duration.
    MeasuredValue::Normal(5.0 + 3.0 * (t / duration * TAU).sin())
}

fn acv_value(t: f64, duration: f64) -> MeasuredValue {
    // Two sine cycles per duration.
    MeasuredValue::Normal(120.0 + 2.0 * (t / duration * 2.0 * TAU).sin())
}

fn ohm_value(t: f64, duration: f64) -> MeasuredValue {
    MeasuredValue::Normal(triangle(t / duration, 1.0, 10.0))
}

fn cap_value(t: f64, duration: f64) -> MeasuredValue {
    MeasuredValue::Normal(triangle(t / duration, 1.0, 20.0))
}

fn hz_value(t: f64, duration: f64) -> MeasuredValue {
    MeasuredValue::Normal(60.0 + 0.5 * (t / duration * TAU).sin())
}

/// T1, the thermocouple every temperature scenario puts on the main display,
/// in °C: one slow ramp across 20–30 °C per duration.
///
/// Named separately from [`temp_value`] so the differential arrangements can
/// subtract the very same waveform rather than restating the math.
fn temp_t1_celsius(t: f64, duration: f64) -> f64 {
    triangle(t / duration, 20.0, 30.0)
}

pub(super) fn temp_value(t: f64, duration: f64) -> MeasuredValue {
    MeasuredValue::Normal(temp_t1_celsius(t, duration))
}

fn dcma_value(t: f64, duration: f64) -> MeasuredValue {
    // Two sine cycles per duration.
    MeasuredValue::Normal(50.0 + 5.0 * (t / duration * 2.0 * TAU).sin())
}

fn ohm_ol_value(_t: f64, _duration: f64) -> MeasuredValue {
    MeasuredValue::Overload
}

fn ncv_value(t: f64, duration: f64) -> MeasuredValue {
    // Discrete triangle: 0,1,2,3,4,3,2,1 stepped across the duration, so the
    // level at t=duration is the starting level at t=0.
    const LEVELS: [u8; 8] = [0, 1, 2, 3, 4, 3, 2, 1];
    let phase = (t / duration).rem_euclid(1.0);
    let idx = ((phase * LEVELS.len() as f64) as usize).min(LEVELS.len() - 1);
    MeasuredValue::NcvLevel(LEVELS[idx])
}

/// Line frequency behind the `acv-hz` sub-displays, in Hz. Drifts by 0.05 Hz
/// around 60 — the scale a mains-frequency reading actually moves on.
///
/// Three cycles per duration against the main AC voltage's two, so on the
/// graph the frequency trace visibly runs at its own rate instead of looking
/// like a rescaled copy of the voltage it sits beside.
fn acv_line_hz(t: f64, duration: f64) -> f64 {
    60.0 + 0.05 * (t / duration * 3.0 * TAU).sin()
}

fn acv_hz_freq_value(t: f64, duration: f64) -> MeasuredValue {
    MeasuredValue::Normal(acv_line_hz(t, duration))
}

fn acv_hz_period_value(t: f64, duration: f64) -> MeasuredValue {
    // Derived from the same frequency so the two sub-displays never disagree,
    // as they can't on the meter either.
    MeasuredValue::Normal(1000.0 / acv_line_hz(t, duration))
}

/// T2, the second thermocouple of the `temp2` scenario, in °C.
///
/// Deliberately not derived from T1: two sine cycles per duration with a
/// smaller swing, against T1's single triangle ramp. The two traces then cross
/// repeatedly instead of running parallel, which is the point of the scenario —
/// a graph with two sub-values on it has to show them apart. The 21–25 °C swing
/// stays inside T1's 20–30 °C band so both share one Y axis.
///
/// Named separately from [`temp2_value`] for the same reason as
/// [`temp_t1_celsius`]: the differential arrangements subtract this waveform.
fn temp_t2_celsius(t: f64, duration: f64) -> f64 {
    23.0 + 2.0 * (t / duration * 2.0 * TAU).sin()
}

pub(super) fn temp2_value(t: f64, duration: f64) -> MeasuredValue {
    MeasuredValue::Normal(temp_t2_celsius(t, duration))
}

/// The `temp-diff` scenario's reading: T1 − T2, the real difference of the two
/// probes `temp2` displays, at the same elapsed time. All four temperature
/// scenarios share a duration, so switching between them shows arithmetic that
/// adds up.
pub(super) fn temp_diff_value(t: f64, duration: f64) -> MeasuredValue {
    MeasuredValue::Normal(temp_t1_celsius(t, duration) - temp_t2_celsius(t, duration))
}

/// The `temp-diff-rev` scenario's reading: the same difference the other way
/// round, T2 − T1.
pub(super) fn temp_diff_rev_value(t: f64, duration: f64) -> MeasuredValue {
    MeasuredValue::Normal(temp_t2_celsius(t, duration) - temp_t1_celsius(t, duration))
}

/// Noise samples per second: one fresh value per 100 ms, the cadence a
/// reading arrives on, so a spike occupies exactly one displayed sample
/// instead of smearing across its neighbours.
const NOISE_STEPS_PER_SEC: f64 = 10.0;

/// Spike strides, in steps. Coprime and offset so the two trains never land
/// on the same step within a period (`noise_spikes_are_single_samples`
/// checks it) — a step carrying both would show as neither spike.
const NOISE_SPIKE_UP: (u64, u64) = (73, 5);
const NOISE_SPIKE_DOWN: (u64, u64) = (111, 41);

/// One reproducible pseudo-random value per step index, in [-0.5, 0.5).
///
/// splitmix64's finalizer, written out: `dmm-lib` carries no utility crates,
/// and what the waveform needs is a hash that decorrelates neighbouring
/// indices, not a stateful generator — state would make the reading depend on
/// how often it was sampled.
fn noise_hash(i: u64) -> f64 {
    let mut z = i.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    // Top 53 bits: as many as an f64 mantissa holds exactly.
    (z >> 11) as f64 / (1u64 << 53) as f64 - 0.5
}

/// The `noise` scenario's reading, in mV: a slow drift with per-sample noise
/// on it and the occasional single-sample spike.
///
/// Every other scenario traces a smooth curve, which survives any amount of
/// decimation — so none of them can tell whether the minimap's per-bucket
/// min/max is working. Here a spike one sample wide has to stay visible once
/// a bucket covers dozens of samples, and the noise band has to stay a band
/// rather than collapsing to a line.
///
/// Still a pure function of elapsed time like the rest: the noise is a hash
/// of the step index, and the index is taken within the period, so `t` and
/// `t + duration` read the same.
fn noise_value(t: f64, duration: f64) -> MeasuredValue {
    let cycle_t = t.rem_euclid(duration);
    let base = 12.0 + 1.5 * (cycle_t / duration * TAU).sin();
    let i = (cycle_t * NOISE_STEPS_PER_SEC) as u64;
    let mut v = base + noise_hash(i);
    if i % NOISE_SPIKE_UP.0 == NOISE_SPIKE_UP.1 {
        v += 8.0;
    }
    if i % NOISE_SPIKE_DOWN.0 == NOISE_SPIKE_DOWN.1 {
        v -= 6.0;
    }
    MeasuredValue::Normal(v)
}

/// Frequency then period — the order the UT181A's `0x1121` frame sends them.
const ACV_HZ_AUX: &[AuxSpec] = &[
    AuxSpec {
        label: "Frequency",
        unit: "Hz",
        value_fn: acv_hz_freq_value,
    },
    AuxSpec {
        label: "Period",
        unit: "ms",
        value_fn: acv_hz_period_value,
    },
];

/// The second thermocouple of the UT181A's `0x4211` frame.
const TEMP_DUAL_AUX: &[AuxSpec] = &[AuxSpec {
    label: "T2",
    unit: "\u{00B0}C",
    value_fn: temp2_value,
}];

pub(super) fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            id: MockMode::DcV,
            mode: "DC V",
            mode_raw: 0x02,
            range_raw: 1,
            unit: "V",
            range_label: "22V",
            range_max: 22.0,
            duration_secs: 10.0,
            value_fn: dcv_value,
            aux: &[],
        },
        Scenario {
            id: MockMode::AcV,
            mode: "AC V",
            mode_raw: 0x00,
            range_raw: 2,
            unit: "V",
            range_label: "220V",
            range_max: 220.0,
            duration_secs: 10.0,
            value_fn: acv_value,
            aux: &[],
        },
        Scenario {
            id: MockMode::Ohm,
            mode: "\u{03A9}",
            mode_raw: 0x06,
            range_raw: 2,
            unit: "k\u{03A9}",
            range_label: "22k\u{03A9}",
            range_max: 22.0,
            duration_secs: 10.0,
            value_fn: ohm_value,
            aux: &[],
        },
        Scenario {
            id: MockMode::Capacitance,
            mode: "Capacitance",
            mode_raw: 0x09,
            range_raw: 3,
            unit: "\u{00B5}F",
            range_label: "22\u{00B5}F",
            range_max: 22.0,
            duration_secs: 10.0,
            value_fn: cap_value,
            aux: &[],
        },
        Scenario {
            id: MockMode::Hz,
            mode: "Hz",
            mode_raw: 0x04,
            range_raw: 1,
            unit: "Hz",
            range_label: "220Hz",
            range_max: 220.0,
            duration_secs: 8.0,
            value_fn: hz_value,
            aux: &[],
        },
        Scenario {
            id: MockMode::Temp,
            mode: "Temp \u{00B0}C",
            mode_raw: 0x0A,
            range_raw: 0,
            unit: "\u{00B0}C",
            range_label: "",
            range_max: 400.0,
            duration_secs: 8.0,
            value_fn: temp_value,
            aux: &[],
        },
        Scenario {
            id: MockMode::DcMa,
            mode: "DC mA",
            mode_raw: 0x0E,
            range_raw: 1,
            unit: "mA",
            range_label: "220mA",
            range_max: 220.0,
            duration_secs: 8.0,
            value_fn: dcma_value,
            aux: &[],
        },
        Scenario {
            id: MockMode::OhmOl,
            mode: "\u{03A9}",
            mode_raw: 0x06,
            range_raw: 5,
            unit: "M\u{03A9}",
            range_label: "22M\u{03A9}",
            range_max: 22.0,
            duration_secs: 2.0,
            value_fn: ohm_ol_value,
            aux: &[],
        },
        Scenario {
            id: MockMode::Ncv,
            mode: "NCV",
            mode_raw: 0x14,
            range_raw: 0,
            unit: "",
            range_label: "",
            range_max: 4.0,
            duration_secs: 4.0,
            value_fn: ncv_value,
            aux: &[],
        },
        // Multi-display scenarios, appended so the auto-cycle order of the
        // single-display ones above is unchanged. They exercise the
        // `aux_values` path (UT181A, UT171) without the hardware.
        //
        // Their mode strings name the sub-displays, as a multi-display meter
        // does — the UT181A calls `0x1121` "V AC Hz", not "V AC". Without
        // that, each of these read identically to the single-display scenario
        // it sits beside, and the mode selector would offer two entries with
        // the same label.
        Scenario {
            id: MockMode::AcVHz,
            mode: "AC V Hz",
            mode_raw: 0x00,
            range_raw: 2,
            unit: "V",
            range_label: "220V",
            range_max: 220.0,
            duration_secs: 10.0,
            value_fn: acv_value,
            aux: ACV_HZ_AUX,
        },
        Scenario {
            id: MockMode::TempDual,
            mode: "Temp \u{00B0}C T1 (T2)",
            mode_raw: 0x0A,
            range_raw: 0,
            unit: "\u{00B0}C",
            range_label: "",
            range_max: 400.0,
            duration_secs: 8.0,
            value_fn: temp_value,
            aux: TEMP_DUAL_AUX,
        },
        // The temperature dial's two arithmetic arrangements, so the mode
        // group has the four entries a real UT181A offers there. They run on
        // the same 8 s clock as `temp` and `temp2` and subtract those very
        // waveforms, so the four readings agree with each other.
        //
        // No sub-values: the meter's aux layout in the differential
        // arrangements is unverified. `aux_labels` in the UT181A decoder
        // deliberately falls back to the positional "Aux1"/"Aux2" for 0x4231
        // and 0x4241 because no source says which probe feeds the slot, so
        // the mock asserts nothing here either.
        Scenario {
            id: MockMode::TempDiff,
            mode: "Temp \u{00B0}C T1-T2",
            mode_raw: 0x0A,
            range_raw: 0,
            unit: "\u{00B0}C",
            range_label: "",
            range_max: 400.0,
            duration_secs: 8.0,
            value_fn: temp_diff_value,
            aux: &[],
        },
        Scenario {
            id: MockMode::TempDiffRev,
            mode: "Temp \u{00B0}C T2-T1",
            mode_raw: 0x0A,
            range_raw: 0,
            unit: "\u{00B0}C",
            range_label: "",
            range_max: 400.0,
            duration_secs: 8.0,
            value_fn: temp_diff_rev_value,
            aux: &[],
        },
        // Appended last for the reason the block above was: the auto-cycle
        // order of everything before it stays unchanged.
        //
        // The one scenario that is not a smooth curve, and the one that runs
        // a minute — the spike trains need room to repeat, and a graph filled
        // with a minute of 100 ms samples is what the minimap has to condense.
        Scenario {
            id: MockMode::Noise,
            mode: "DC mV",
            mode_raw: 0x03,
            range_raw: 0,
            unit: "mV",
            range_label: "220mV",
            range_max: 220.0,
            duration_secs: 60.0,
            value_fn: noise_value,
            aux: &[],
        },
    ]
}

/// Duration of the scenario driving `mode`, for sampling its waveforms.
#[cfg(test)]
pub(super) fn scenario_duration(mode: MockMode) -> f64 {
    scenarios()
        .into_iter()
        .find(|s| s.id == mode)
        .unwrap_or_else(|| panic!("no scenario for {mode:?}"))
        .duration_secs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The value functions divide by it to compute phase, so a zero would
    /// make the whole scenario NaN. All current entries use 2–20 s.
    #[test]
    fn every_scenario_has_a_positive_duration() {
        for s in scenarios() {
            assert!(s.duration_secs > 0.0, "{:?}: {}", s.id, s.duration_secs);
        }
    }

    #[test]
    fn test_dcv_is_smooth_function_of_time() {
        // dcv_value completes one full sine cycle over `duration`, so samples
        // one period apart must match exactly and samples a quarter period
        // apart must be symmetric about the centre value. This is what makes
        // the displayed waveform jitter-free — the value depends only on the
        // sample time, not on the read cadence.
        let duration = 10.0;
        let a = match dcv_value(0.0, duration) {
            MeasuredValue::Normal(v) => v,
            _ => panic!("expected Normal"),
        };
        let b = match dcv_value(duration, duration) {
            MeasuredValue::Normal(v) => v,
            _ => panic!("expected Normal"),
        };
        let c = match dcv_value(2.0 * duration, duration) {
            MeasuredValue::Normal(v) => v,
            _ => panic!("expected Normal"),
        };
        assert!((a - b).abs() < 1e-9, "period mismatch: {a} vs {b}");
        assert!((b - c).abs() < 1e-9, "period mismatch: {b} vs {c}");

        // Half-period (sin is odd about zero): values symmetric about 5.0.
        let left = match dcv_value(duration / 4.0, duration) {
            MeasuredValue::Normal(v) => v,
            _ => panic!("expected Normal"),
        };
        let right = match dcv_value(3.0 * duration / 4.0, duration) {
            MeasuredValue::Normal(v) => v,
            _ => panic!("expected Normal"),
        };
        assert!(
            ((left - 5.0) + (right - 5.0)).abs() < 1e-9,
            "half-period samples should be symmetric about 5.0: {left}, {right}"
        );
    }

    #[test]
    fn test_waveforms_loop_continuously() {
        // Every scenario must satisfy f(0) == f(duration): when the pattern
        // wraps back to t=0 the displayed value must not jump. Sub-value
        // waveforms wrap on the same schedule, so they get the same check.
        fn assert_loops(start: &MeasuredValue, end: &MeasuredValue, what: &str) {
            match (start, end) {
                (MeasuredValue::Normal(a), MeasuredValue::Normal(b)) => {
                    assert!((a - b).abs() < 1e-9, "{what}: f(0)={a} but f(duration)={b}");
                }
                (MeasuredValue::Overload, MeasuredValue::Overload) => {}
                (MeasuredValue::NcvLevel(a), MeasuredValue::NcvLevel(b)) => {
                    assert_eq!(a, b, "{what}: ncv level jumps at wrap");
                }
                _ => panic!("{what}: variant differs at wrap: {start:?} vs {end:?}"),
            }
        }

        for s in scenarios() {
            assert_loops(
                &(s.value_fn)(0.0, s.duration_secs),
                &(s.value_fn)(s.duration_secs, s.duration_secs),
                &format!("{:?}", s.id),
            );
            for spec in s.aux {
                assert_loops(
                    &(spec.value_fn)(0.0, s.duration_secs),
                    &(spec.value_fn)(s.duration_secs, s.duration_secs),
                    &format!("{:?} aux {}", s.id, spec.label),
                );
            }
        }
    }

    /// Assert `aux_fn` is not `a * main_fn + b`: fit the line through the two
    /// samples whose main values are furthest apart, then require some other
    /// sample to miss it by a visible fraction of the sub-value's own swing.
    /// A sub-value that passed would be drawn as a parallel copy of the main
    /// trace, which is exactly what these scenarios exist to avoid.
    fn assert_not_affine_in_main(
        main_fn: fn(f64, f64) -> MeasuredValue,
        aux_fn: fn(f64, f64) -> MeasuredValue,
        duration: f64,
        label: &str,
    ) {
        const SAMPLES: usize = 32;
        let points: Vec<(f64, f64)> = (0..SAMPLES)
            .map(|i| {
                let t = i as f64 / SAMPLES as f64 * duration;
                match (main_fn(t, duration), aux_fn(t, duration)) {
                    (MeasuredValue::Normal(m), MeasuredValue::Normal(a)) => (m, a),
                    other => panic!("{label}: expected Normal values, got {other:?}"),
                }
            })
            .collect();

        let (x0, y0) = points[0];
        let &(x1, y1) = points
            .iter()
            .max_by(|a, b| {
                (a.0 - x0)
                    .abs()
                    .partial_cmp(&(b.0 - x0).abs())
                    .expect("waveform produced NaN")
            })
            .expect("SAMPLES > 0");
        assert!((x1 - x0).abs() > 1e-9, "{label}: main waveform is constant");
        let slope = (y1 - y0) / (x1 - x0);
        let offset = y0 - slope * x0;

        let worst = points
            .iter()
            .map(|&(x, y)| (y - (slope * x + offset)).abs())
            .fold(0.0_f64, f64::max);
        let aux_span = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max)
            - points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        assert!(
            worst > 0.1 * aux_span,
            "{label}: sub-value is an affine copy of the main value \
             (worst deviation {worst}, sub-value swing {aux_span})"
        );
    }

    fn normal(value: MeasuredValue, what: &str) -> f64 {
        match value {
            MeasuredValue::Normal(v) => v,
            other => panic!("{what}: expected Normal, got {other:?}"),
        }
    }

    /// Every noise step index within one period, sampled mid-step so the
    /// value sits inside the 100 ms the index covers.
    fn noise_steps() -> impl Iterator<Item = (u64, f64)> {
        let duration = scenario_duration(MockMode::Noise);
        let steps = (duration * NOISE_STEPS_PER_SEC) as u64;
        (0..steps).map(move |i| {
            let t = (i as f64 + 0.5) / NOISE_STEPS_PER_SEC;
            (i, normal(noise_value(t, duration), "noise"))
        })
    }

    /// `test_waveforms_loop_continuously` only checks the wrap point, which a
    /// hash-driven waveform can pass while being different everywhere else —
    /// so check the whole period, and check it twice for purity.
    #[test]
    fn noise_repeats_exactly_and_wraps_with_its_period() {
        let duration = scenario_duration(MockMode::Noise);
        for (i, v) in noise_steps() {
            let t = (i as f64 + 0.5) / NOISE_STEPS_PER_SEC;
            let again = normal(noise_value(t, duration), "noise");
            assert_eq!(v, again, "noise_value is not pure at t={t}");
            let wrapped = normal(noise_value(t + duration, duration), "noise");
            assert!(
                (v - wrapped).abs() < 1e-9,
                "t={t} reads {v} but t+{duration} reads {wrapped}"
            );
        }
    }

    /// The scenario exists to give the graph and the minimap something that
    /// decimates badly: a band the drift and noise never leave, and spikes
    /// one sample wide that clear it.
    #[test]
    fn noise_spikes_are_single_samples() {
        // Drift 12 ± 1.5, noise ± 0.5.
        let band = 10.0..=14.0;
        let mut up = 0;
        let mut down = 0;
        for (i, v) in noise_steps() {
            let is_up = i % NOISE_SPIKE_UP.0 == NOISE_SPIKE_UP.1;
            let is_down = i % NOISE_SPIKE_DOWN.0 == NOISE_SPIKE_DOWN.1;
            assert!(!(is_up && is_down), "step {i} carries both spikes");
            if is_up {
                up += 1;
                assert!(v > 17.0, "up spike at step {i} only reached {v}");
            } else if is_down {
                down += 1;
                assert!(v < 9.0, "down spike at step {i} only reached {v}");
            } else {
                assert!(band.contains(&v), "step {i} left the band at {v}");
            }
        }
        assert!(up > 0 && down > 0, "{up} up spikes, {down} down spikes");
    }

    /// A hash that returned the same value for neighbouring indices would
    /// pass every check above while drawing a staircase, not noise.
    #[test]
    fn noise_actually_varies_between_steps() {
        let quiet: Vec<f64> = noise_steps()
            .filter(|(i, _)| {
                i % NOISE_SPIKE_UP.0 != NOISE_SPIKE_UP.1
                    && i % NOISE_SPIKE_DOWN.0 != NOISE_SPIKE_DOWN.1
            })
            .map(|(_, v)| v)
            .collect();
        let jumps = quiet
            .windows(2)
            .filter(|w| (w[1] - w[0]).abs() > 0.1)
            .count();
        assert!(
            jumps > quiet.len() / 2,
            "only {jumps} of {} steps moved",
            quiet.len()
        );
    }

    #[test]
    fn sub_values_are_not_affine_copies_of_the_main_waveform() {
        assert_not_affine_in_main(
            temp_value,
            temp2_value,
            scenario_duration(MockMode::TempDual),
            "T2",
        );
        assert_not_affine_in_main(
            acv_value,
            acv_hz_freq_value,
            scenario_duration(MockMode::AcVHz),
            "Frequency",
        );
    }
}
