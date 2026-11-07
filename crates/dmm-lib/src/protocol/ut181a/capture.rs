//! The UT181A's capture steps.

use crate::flags::Flag;
use crate::protocol::steps::{self, Ohms, Volts};
use crate::protocol::{CaptureStep, Expect, Need, RangeExpect, command};

pub(super) fn steps() -> Vec<CaptureStep> {
    // A real meter has run every step (issue #5, 2026-09-27) but
    // `ohm_ranges`, `vdc_ranges`, `manual_range` and `auto`, which came or
    // changed form since and wait for a run that drives them
    // (docs/research/ut181/verification.md).
    let [vdc, dcv_short, dcv_negative, ohm, ohm_body, ohm_short] = steps::gate_steps(
        Volts::VDc,
        CaptureStep::basic("vdc", "Set meter to V DC"),
        Ohms::Word,
        CaptureStep::basic(
            "ohm",
            "Set meter to Resistance. Leave leads open (should show OL).",
        ),
    )
    .map(CaptureStep::verified);

    // Core UT181A modes. The gate closes before any other step, so every
    // step after it gets swept.
    vec![
        vdc,
        dcv_short,
        dcv_negative,
        ohm,
        ohm_body,
        ohm_short,
        // A gate step is never swept — the step after it assumes the state
        // it left — so the Ω and V DC ladders each get a plain step.
        CaptureStep::basic(
            "ohm_ranges",
            "Set meter to Resistance. Leads open or shorted, either will do.",
        )
        .samples(3)
        .expect(Expect::mode("\u{03A9}")),
        CaptureStep::basic("vdc_ranges", "Set meter to V DC. Leave leads open.")
            .samples(3)
            .expect(Expect::mode("V DC")),
        // Each dial family reaches several mode words (spec §6.1); the
        // steps below name them as the meter reports them, one per word,
        // because SET_MODE only ever moves inside the family the dial is
        // already on.
        CaptureStep::basic("vdc_acdc", "Set meter to V DC AC+DC")
            .verified()
            .expect(Expect::mode("V DC AC+DC")),
        CaptureStep::basic("vdc_peak", "Set meter to V DC Peak")
            .verified()
            .expect(Expect::mode("V DC Peak")),
        CaptureStep::basic("vac", "Set meter to V AC")
            .verified()
            .expect(Expect::mode("V AC")),
        CaptureStep::basic("vac_hz", "Set meter to V AC Hz")
            .verified()
            .expect(Expect::mode("V AC Hz")),
        CaptureStep::basic("vac_lpf", "Set meter to V AC LPF")
            .verified()
            .expect(Expect::mode("V AC LPF")),
        CaptureStep::basic("vac_dbv", "Set meter to V AC dBV")
            .verified()
            .expect(Expect::mode("V AC dBV")),
        CaptureStep::basic("vac_dbm", "Set meter to V AC dBm")
            .verified()
            .expect(Expect::mode("V AC dBm")),
        CaptureStep::basic("mvdc", "Set meter to mV DC")
            .verified()
            .expect(Expect::mode("mV DC")),
        CaptureStep::basic("mvdc_peak", "Set meter to mV DC Peak")
            .verified()
            .expect(Expect::mode("mV DC Peak")),
        CaptureStep::basic("mvac", "Set meter to mV AC")
            .verified()
            .expect(Expect::mode("mV AC")),
        CaptureStep::basic("mvac_hz", "Set meter to mV AC Hz")
            .verified()
            .expect(Expect::mode("mV AC Hz")),
        CaptureStep::basic("mvac_peak", "Set meter to mV AC Peak")
            .verified()
            .expect(Expect::mode("mV AC Peak")),
        // The vendor UI offers this one though its own decoder has no
        // label for it (spec §6.1 "Caveats"); a real meter reports it.
        CaptureStep::basic("mvac_acdc", "Set meter to mV AC AC+DC")
            .verified()
            .expect(Expect::mode("mV AC AC+DC")),
        CaptureStep::basic("cont", "Set meter to Continuity")
            .verified()
            .expect(Expect::mode("Continuity")),
        // Nibble 0 = 2 is a second function on these two families, not
        // REL: the open-circuit beeper and the diode alarm (spec §6.1).
        CaptureStep::basic("cont_open", "Continuity: switch to the open-circuit beeper")
            .verified()
            .expect(Expect::mode("Continuity (open)")),
        CaptureStep::basic("ns", "Set meter to Conductance (nS)")
            .verified()
            .expect(Expect::mode("nS")),
        CaptureStep::basic("diode", "Set meter to Diode")
            .verified()
            .expect(Expect::mode("Diode")),
        CaptureStep::basic("diode_alarm", "Diode: switch to the alarm function")
            .verified()
            .expect(Expect::mode("Diode Alarm")),
        CaptureStep::basic("cap", "Set meter to Capacitance")
            .verified()
            .expect(Expect::mode("Capacitance")),
        CaptureStep::basic("hz", "Set meter to Frequency (Hz)")
            .verified()
            .expect(Expect::mode("Hz")),
        CaptureStep::basic("duty", "Set meter to Duty Cycle (%)")
            .verified()
            .expect(Expect::mode("Duty %")),
        CaptureStep::basic("pulse", "Set meter to Pulse Width (ms)")
            .verified()
            .expect(Expect::mode("Pulse Width")),
        CaptureStep::basic("uadc", "Set meter to µA DC")
            .verified()
            .expect(Expect::mode("µA DC")),
        CaptureStep::basic("uadc_acdc", "Set meter to µA DC AC+DC")
            .verified()
            .expect(Expect::mode("µA DC AC+DC")),
        CaptureStep::basic("uadc_peak", "Set meter to µA DC Peak")
            .verified()
            .expect(Expect::mode("µA DC Peak")),
        CaptureStep::basic("uaac", "Set meter to µA AC")
            .verified()
            .expect(Expect::mode("µA AC")),
        CaptureStep::basic("uaac_hz", "Set meter to µA AC Hz")
            .verified()
            .expect(Expect::mode("µA AC Hz")),
        CaptureStep::basic("uaac_peak", "Set meter to µA AC Peak")
            .verified()
            .expect(Expect::mode("µA AC Peak")),
        CaptureStep::basic("madc", "Set meter to mA DC")
            .verified()
            .expect(Expect::mode("mA DC")),
        CaptureStep::basic("madc_acdc", "Set meter to mA DC AC+DC")
            .verified()
            .expect(Expect::mode("mA DC AC+DC")),
        CaptureStep::basic("madc_peak", "Set meter to mA DC Peak")
            .verified()
            .expect(Expect::mode("mA DC Peak")),
        CaptureStep::basic("maac", "Set meter to mA AC")
            .verified()
            .expect(Expect::mode("mA AC")),
        CaptureStep::basic("maac_hz", "Set meter to mA AC Hz")
            .verified()
            .expect(Expect::mode("mA AC Hz")),
        CaptureStep::basic("maac_peak", "Set meter to mA AC Peak")
            .verified()
            .expect(Expect::mode("mA AC Peak")),
        CaptureStep::basic("adc", "Set meter to A DC")
            .verified()
            .expect(Expect::mode("A DC")),
        CaptureStep::basic("adc_acdc", "Set meter to A DC AC+DC")
            .verified()
            .expect(Expect::mode("A DC AC+DC")),
        CaptureStep::basic("adc_peak", "Set meter to A DC Peak")
            .verified()
            .expect(Expect::mode("A DC Peak")),
        CaptureStep::basic("aac", "Set meter to A AC")
            .verified()
            .expect(Expect::mode("A AC")),
        CaptureStep::basic("aac_hz", "Set meter to A AC Hz")
            .verified()
            .expect(Expect::mode("A AC Hz")),
        CaptureStep::basic("aac_peak", "Set meter to A AC Peak")
            .verified()
            .expect(Expect::mode("A AC Peak")),
        CaptureStep::basic("tempc", "Set meter to Temperature C")
            .verified()
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°C")),
        // Nibble 1 is the probe arrangement on the temperature families,
        // so T2 and the two differentials need the second probe in T2.
        CaptureStep::basic(
            "tempc_t2",
            "Temperature C: switch the main display to T2 (second thermocouple in T2)",
        )
        .verified()
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°C T2")),
        CaptureStep::basic(
            "tempc_t1_t2",
            "Temperature C: switch to the T1-T2 difference (two thermocouples)",
        )
        .verified()
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°C T1-T2")),
        CaptureStep::basic(
            "tempc_t2_t1",
            "Temperature C: switch to the T2-T1 difference (two thermocouples)",
        )
        .verified()
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°C T2-T1")),
        CaptureStep::basic("tempf", "Set meter to Temperature F")
            .verified()
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°F")),
        CaptureStep::basic(
            "tempf_t2",
            "Temperature F: switch the main display to T2 (second thermocouple in T2)",
        )
        .verified()
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°F T2")),
        CaptureStep::basic(
            "tempf_t1_t2",
            "Temperature F: switch to the T1-T2 difference (two thermocouples)",
        )
        .verified()
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°F T1-T2")),
        CaptureStep::basic(
            "tempf_t2_t1",
            "Temperature F: switch to the T2-T1 difference (two thermocouples)",
        )
        .verified()
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°F T2-T1")),
        // Format and command steps, all on V DC: the REL key by hand, then
        // the tool's own commands. `manual_range` is a command step so no
        // sweep puts the meter back on auto before `auto` checks the way
        // back.
        CaptureStep::basic(
            "rel",
            "V DC mode: long-press REL to enable relative. \
                          The report should list Reference and Absolute \
                          sub-values under each sample.",
        )
        .verified()
        .expect(Expect::new().flags(&[(Flag::Rel, true)])),
        CaptureStep::basic("rel_off", "Long-press REL again to disable relative mode.")
            .verified()
            .samples(3)
            .expect(Expect::new().flags(&[(Flag::Rel, false)])),
        CaptureStep::with_command(
            "manual_range",
            "V DC mode: we will switch to a manual range.",
            command::RANGE,
            3,
        )
        // Waits for the meter to be set up; "V DC" alone is REL off, as REL
        // names its own mode (#45).
        .starts_from(Expect::mode("V DC"))
        .expect(Expect::new().range(RangeExpect::Manual)),
        CaptureStep::with_command("auto", "We will set auto-range.", command::AUTO, 3)
            .expect(Expect::new().range(RangeExpect::Auto)),
        CaptureStep::with_command("hold", "V DC mode: we will send HOLD.", command::HOLD, 3)
            .starts_from(Expect::mode("V DC").flags(&[(Flag::Hold, false)]))
            .verified()
            .expect(Expect::new().flags(&[(Flag::Hold, true)])),
        CaptureStep::with_command(
            "hold_off",
            "We will send HOLD again to turn it off.",
            command::HOLD,
            3,
        )
        .verified()
        .expect(Expect::new().flags(&[(Flag::Hold, false)])),
        CaptureStep::with_command("minmax", "We will enable MIN/MAX.", command::MINMAX, 3)
            .verified()
            .expect(Expect::new().flags(&[(Flag::Min, true), (Flag::Max, true)])),
        CaptureStep::with_command(
            "minmax_off",
            "We will disable MIN/MAX.",
            command::EXIT_MINMAX,
            3,
        )
        .verified()
        .expect(Expect::new().flags(&[(Flag::Min, false), (Flag::Max, false)])),
        CaptureStep::basic(
            "peak",
            "V AC mode: enable Peak mode (FUNC button). \
                          The report should list a Peak Min sub-value \
                          under each sample.",
        )
        .verified()
        .expect(Expect::new().flags(&[(Flag::PeakMax, true), (Flag::PeakMin, true)])),
        CaptureStep::basic("peak_off", "Disable Peak mode.")
            .verified()
            .samples(3)
            .expect(Expect::new().flags(&[(Flag::PeakMax, false), (Flag::PeakMin, false)])),
    ]
}
