pub(crate) mod cycle;
mod expect;
pub(crate) mod framing;
pub(crate) mod fs9721;
pub mod registry;
pub(crate) mod ut171;
pub(crate) mod ut181a;
// `ut61eplus` stays `pub`: the GUI specs panel consumes its tables, the CLI
// its remote-control commands, and the GUI's export tests its parser.
pub mod ut61eplus;
pub(crate) mod ut8802;
pub(crate) mod ut8803;
pub(crate) mod vc880;
pub(crate) mod vc890;
mod vc8x0_common;

pub use expect::{Expect, RangeExpect, ValueExpect};

use crate::error::{Error, Result};
use crate::measurement::Measurement;
use crate::specs::{ModeSpecInfo, SpecInfo};
use crate::transport::Transport;
use std::borrow::Cow;

/// The length guard every parser opens with: a payload shorter than the frame
/// its family defines cannot be indexed, so reject it before any field read.
///
/// `family` prefixes the message as each parser did.
pub(crate) fn check_len(family: &str, payload: &[u8], expected: usize) -> Result<()> {
    if payload.len() < expected {
        return Err(Error::invalid_response(
            format!(
                "{family} payload too short: {} bytes, expected {expected}",
                payload.len()
            ),
            payload,
        ));
    }
    Ok(())
}

/// The `Unknown(0x..)` mode string a parser falls back to for a one-byte code
/// its table doesn't list.
pub(crate) fn unknown_mode(code: u8) -> Cow<'static, str> {
    Cow::Owned(format!("Unknown({code:#04x})"))
}

/// Same as [`unknown_mode`], for families whose mode code is two bytes wide.
pub(crate) fn unknown_mode16(code: u16) -> Cow<'static, str> {
    Cow::Owned(format!("Unknown({code:#06x})"))
}

/// Helpers shared by the per-family parser tests.
#[cfg(test)]
pub(crate) mod test_support {
    use crate::measurement::Measurement;

    /// Render a parsed [`Measurement`] as deterministic `key=value` lines.
    ///
    /// Every field a parser decides is printed, so a test that pins this
    /// string fails on *any* change to the parsed reading rather than only
    /// on the handful of fields the test thought to assert. The timestamp is
    /// excluded (it is `Instant::now()`), and the sub-value list and raw
    /// payload appear as lengths — the payload is the test's own input, and
    /// no family covered by this helper produces sub-values.
    pub(crate) fn snapshot(m: &Measurement) -> String {
        let flags: Vec<&str> = m
            .flags
            .as_pairs()
            .iter()
            .filter(|(_, set)| *set)
            .map(|(name, _)| *name)
            .collect();
        format!(
            "mode={}\n\
             mode_raw={:#04x}\n\
             range_raw={:#04x}\n\
             value={:?}\n\
             unit={}\n\
             range_label={}\n\
             display_raw={:?}\n\
             flags={}\n\
             aux={}\n\
             raw_payload={}",
            m.mode,
            m.mode_raw,
            m.range_raw,
            m.value,
            m.unit,
            m.range_label,
            m.display_raw,
            flags.join(","),
            m.aux_values.len(),
            m.raw_payload.len(),
        )
    }
}

/// Protocol stability level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stability {
    /// Verified against real hardware.
    Verified,
    /// Based on reverse engineering, not yet verified against real hardware.
    Experimental,
}

/// Static profile information about a device.
///
/// `Copy` so consumers can cache one without holding the protocol instance
/// alive — every field is already a `'static` reference or a small value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceProfile {
    pub family_name: &'static str,
    pub model_name: &'static str,
    pub stability: Stability,
    pub supported_commands: &'static [&'static str],
    /// Most sub-values one frame can carry (`Measurement::aux_values`). Sizes the
    /// fixed CSV slot columns in the CLI and GUI; 0 for single-display meters.
    pub max_aux_values: usize,
    /// GitHub issue number for verification tracking (e.g. `Some(3)` → issue #3).
    pub verification_issue: Option<u16>,
}

const REPO_ISSUES_URL: &str = "https://github.com/antoinecellerier/dmm-tools/issues";

impl DeviceProfile {
    /// URL for verification feedback — links to the device-specific issue if available,
    /// otherwise the general issues page.
    pub fn feedback_url(&self) -> String {
        match self.verification_issue {
            Some(n) => format!("{REPO_ISSUES_URL}/{n}"),
            None => REPO_ISSUES_URL.to_string(),
        }
    }
}

/// Device family selector for opening a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceFamily {
    /// UT61E+, UT61B+, UT61D+, UT161B, UT161D, UT161E
    Ut61EPlus,
    /// UT8802 / UT8802N bench multimeter
    Ut8802,
    /// UT8803 / UT8803E bench multimeter
    Ut8803,
    /// UT803 / UT804 bench multimeter (FS9721-style framing)
    Fs9721,
    /// UT171A / UT171B / UT171C
    Ut171,
    /// UT181A
    Ut181a,
    /// Voltcraft VC-880 / VC650BT
    Vc880,
    /// Voltcraft VC-890
    Vc890,
    /// Simulated device for testing and demos
    Mock,
}

impl std::fmt::Display for DeviceFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceFamily::Ut61EPlus => write!(f, "ut61eplus"),
            DeviceFamily::Ut8802 => write!(f, "ut8802"),
            DeviceFamily::Ut8803 => write!(f, "ut8803"),
            DeviceFamily::Fs9721 => write!(f, "fs9721"),
            DeviceFamily::Ut171 => write!(f, "ut171"),
            DeviceFamily::Ut181a => write!(f, "ut181a"),
            DeviceFamily::Vc880 => write!(f, "vc880"),
            DeviceFamily::Vc890 => write!(f, "vc890"),
            DeviceFamily::Mock => write!(f, "mock"),
        }
    }
}

impl std::str::FromStr for DeviceFamily {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        registry::resolve_device(s)
            .map(|d| d.family)
            .ok_or_else(|| format!("unknown device family: {s}"))
    }
}

/// A meter setting the host can read the options of and switch between.
///
/// All six are implemented; which of them a given family offers is the
/// family's own answer, and an empty choice list means "not offered".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Setting {
    /// Measurement mode. Choice ids are the family's own mode ids (UT181A: the
    /// mode word; the cycling families: the mode byte).
    Mode,
    /// Measurement range. Choice id [`AUTO_RANGE_ID`] is autorange, and id
    /// `n >= 1` is the n-th rung of the current mode's ladder, counting from
    /// one — not the family's own range value, because the UT61+ range byte
    /// is 0-based and rung 0 would collide with autorange. On the UT181A `n`
    /// is exactly what SET_RANGE takes; elsewhere `n - 1` indexes the
    /// family's own table.
    Range,
    /// Display hold. Choice id 0 is off, 1 is on.
    Hold,
    /// Relative (delta) reading. Choice id 0 is off, 1 is on.
    Rel,
    /// Minimum/maximum tracking. Choice id 0 is off, 1 is MAX, 2 is MIN, and
    /// 3 is AVG on the Voltcraft meters, whose button cycles all three — the
    /// UT181A has only the 0/1 pair.
    MinMax,
    /// Peak hold. Choice id 0 is off, 1 is P-MAX, 2 is P-MIN.
    Peak,
}

impl Setting {
    /// Every setting, in the order the CLI and GUI list them.
    pub const ALL: [Setting; 6] = [
        Setting::Mode,
        Setting::Range,
        Setting::Hold,
        Setting::Rel,
        Setting::MinMax,
        Setting::Peak,
    ];

    /// The lowercase word the CLI takes and prints for this setting.
    pub fn name(self) -> &'static str {
        match self {
            Setting::Mode => "mode",
            Setting::Range => "range",
            Setting::Hold => "hold",
            Setting::Rel => "rel",
            Setting::MinMax => "minmax",
            Setting::Peak => "peak",
        }
    }
}

impl std::fmt::Display for Setting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// A value the host can switch a meter setting to from where it sits now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// Family-specific id handed back to `select` (UT181A mode: the mode word).
    pub id: u16,
    /// Display name in the same vocabulary as `Measurement::mode`.
    pub label: Cow<'static, str>,
    /// The meter sits on this value now.
    pub current: bool,
}

/// The [`Setting::Range`] choice id that means autorange, listed first and
/// reached by a command of its own rather than by pressing RANGE.
pub(crate) const AUTO_RANGE_ID: u16 = 0;

/// Label of the autorange choice, in the vocabulary of
/// `Measurement::range_label`.
pub(crate) const AUTO_RANGE_LABEL: &str = "Auto";

/// The refusal a family returns for a setting it cannot drive, kept in one
/// place so every family words it the same way.
pub(crate) fn unsupported_setting(setting: Setting) -> Error {
    Error::UnsupportedCommand(format!("{setting} cannot be set on this meter"))
}

/// A physical thing a capture step needs the user to have on the bench.
///
/// Capture lists these up front so the user can gather them once, and skip
/// the steps for anything they do not have rather than discovering it
/// halfway through the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Need {
    /// The two probe tips touched together.
    ShortedLeads,
    /// Any DC source to read, and to reverse the leads on for a negative.
    DcSource,
    /// A temperature probe; every family that measures °C uses a K-type.
    Thermocouple,
    /// Mains wiring to hold the meter against for the NCV detector.
    LiveWire,
    /// A transistor to sit in the hFE socket.
    Transistor,
    /// A thyristor for the SCR test.
    Scr,
}

impl Need {
    /// Every need, in the order the up-front checklist lists them.
    pub const ALL: [Need; 6] = [
        Need::ShortedLeads,
        Need::DcSource,
        Need::Thermocouple,
        Need::LiveWire,
        Need::Transistor,
        Need::Scr,
    ];

    /// What to call this on the checklist, as an article-less noun phrase: it
    /// reads after both "you will need" and "skipped: no".
    pub fn label(self) -> &'static str {
        match self {
            Need::ShortedLeads => "shorted test leads",
            Need::DcSource => "battery or other DC source",
            Need::Thermocouple => "K-type thermocouple",
            Need::LiveWire => "live mains wire nearby",
            Need::Transistor => "transistor",
            Need::Scr => "SCR (thyristor)",
        }
    }
}

/// A step definition for the guided protocol capture wizard.
pub struct CaptureStep {
    /// Unique identifier for this step (e.g. "dcv", "hold_on").
    pub id: &'static str,
    /// Human-readable instruction for the user (e.g. "Set meter to DC V mode").
    pub instruction: &'static str,
    /// Optional command to send before capturing (e.g. "hold").
    pub command: Option<&'static str>,
    /// Number of samples to capture for this step.
    pub samples: usize,
    /// This step's wire behaviour has been confirmed on real hardware, so a
    /// capture run only re-files what is already known.
    pub verified: bool,
    /// One of the few steps that establish the core semantics — mode byte,
    /// digits, decimal point, OL, sign. A run that skips these files samples
    /// nothing can be concluded from.
    pub gate: bool,
    /// What a correctly parsed reading looks like once the user has done the
    /// instruction; `None` where nothing can be asserted without guessing.
    pub expect: Option<Expect>,
    /// Equipment the instruction asks for beyond the meter and its leads, so
    /// a run can be planned — and pruned — before it starts.
    pub needs: &'static [Need],
}

impl CaptureStep {
    /// A plain measurement-mode step: no command sent first, five samples — what
    /// most steps are.
    pub const fn basic(id: &'static str, instruction: &'static str) -> Self {
        Self {
            id,
            instruction,
            command: None,
            samples: 5,
            verified: false,
            gate: false,
            expect: None,
            needs: &[],
        }
    }

    /// A step that sends `command` before sampling (hold, rel, range…);
    /// `samples` as given.
    pub const fn with_command(
        id: &'static str,
        instruction: &'static str,
        command: &'static str,
        samples: usize,
    ) -> Self {
        Self {
            id,
            instruction,
            command: Some(command),
            samples,
            verified: false,
            gate: false,
            expect: None,
            needs: &[],
        }
    }

    /// Take `n` samples instead of the default five.
    pub const fn samples(mut self, n: usize) -> Self {
        self.samples = n;
        self
    }

    /// Mark the step as confirmed on real hardware.
    pub const fn verified(mut self) -> Self {
        self.verified = true;
        self
    }

    /// Mark the step verified only for models that have actually been tested —
    /// families whose step list is shared by a verified meter and its
    /// experimental siblings.
    pub const fn verified_if(mut self, verified: bool) -> Self {
        self.verified = verified;
        self
    }

    /// Mark the step as one the family's core semantics rest on.
    pub const fn gate(mut self) -> Self {
        self.gate = true;
        self
    }

    /// Attach what a correct reading looks like after the instruction.
    pub const fn expect(mut self, expect: Expect) -> Self {
        self.expect = Some(expect);
        self
    }

    /// Declare the equipment this step's instruction asks for.
    pub const fn needs(mut self, needs: &'static [Need]) -> Self {
        self.needs = needs;
        self
    }
}

/// Each device family implements this trait. Object-safe.
///
/// The Protocol owns its internal state (rx buffer, streaming trigger state, etc).
/// I/O is performed through the Transport reference passed to each method.
pub trait Protocol: Send {
    /// Post-transport initialization (e.g. send streaming trigger, purge FIFOs).
    fn init(&mut self, transport: &dyn Transport) -> Result<()>;

    /// Get the next measurement.
    /// For polled protocols: sends request + reads response.
    /// For streaming protocols: reads the next frame from the stream.
    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement>;

    /// Parse one measurement payload off the wire, with no I/O.
    ///
    /// `payload` is exactly what [`Measurement::raw_payload`] carries for this
    /// family, which is what a `dmm-cli capture` report writes to `raw_hex` —
    /// so a sample copied out of a report is a golden fixture verbatim.
    ///
    /// Required rather than defaulted: a family that skipped it would have its
    /// golden fixtures silently never run.
    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement>;

    /// Send a named command ("hold", "range", "auto", etc.).
    /// Returns UnsupportedCommand for unknown commands.
    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()>;

    /// Request device name. Returns None if the protocol doesn't support it.
    fn get_name(&mut self, transport: &dyn Transport) -> Result<Option<String>>;

    /// Static device profile information.
    fn profile(&self) -> &DeviceProfile;

    /// Capture steps for the guided protocol capture wizard.
    /// Returns basic measurement mode steps that any user can run.
    fn capture_steps(&self) -> Vec<CaptureStep> {
        vec![]
    }

    /// Per-range resolution/accuracy specs for the current measurement.
    /// Default `None` — families without a spec table can leave this unimplemented.
    fn spec_info(&self, _mode_raw: u16, _range_raw: u8) -> Option<&'static SpecInfo> {
        None
    }

    /// Per-mode specs (input impedance, overload protection, notes).
    /// Default `None` — families without a spec table can leave this unimplemented.
    fn mode_spec_info(&self, _mode_raw: u16) -> Option<&'static ModeSpecInfo> {
        None
    }

    /// Values `setting` can be switched to without touching the dial, given
    /// the reading the meter is producing now.
    ///
    /// An empty list — the default — means the family cannot drive that
    /// setting remotely, so consumers hide the control rather than
    /// special-casing families. A one-entry list means the same: that entry is
    /// the value the meter already sits on, and offering it switches nothing.
    /// Return the family's own list either way; consumers decide what to draw.
    fn choices(&self, _setting: Setting, _current: &Measurement) -> Vec<Choice> {
        Vec::new()
    }

    /// Switch `setting` to the value `id` identifies, one of the ids
    /// [`Protocol::choices`] just returned for that setting.
    fn select(&mut self, _transport: &dyn Transport, setting: Setting, _id: u16) -> Result<()> {
        Err(unsupported_setting(setting))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every device's capture steps, with the device id for failure messages.
    fn all_steps() -> Vec<(&'static str, Stability, Vec<CaptureStep>)> {
        registry::DEVICES
            .iter()
            .map(|d| {
                let proto = (d.new_protocol)();
                (d.id, proto.profile().stability, proto.capture_steps())
            })
            .collect()
    }

    /// A gate step without an expectation files samples nobody can judge.
    #[test]
    fn every_gate_step_has_an_expect() {
        for (id, _, steps) in all_steps() {
            for step in steps.iter().filter(|s| s.gate) {
                assert!(
                    step.expect.is_some(),
                    "{id} gate step {} has no expect",
                    step.id
                );
            }
        }
    }

    /// An experimental device is one nobody has finished running, so its step
    /// list must still have something left to confirm.
    #[test]
    fn experimental_devices_leave_steps_to_verify() {
        for (id, stability, steps) in all_steps() {
            if stability == Stability::Experimental {
                assert!(
                    steps.iter().any(|s| !s.verified),
                    "{id} is experimental but every capture step is verified"
                );
            }
        }
    }

    /// The UT61+/UT161 step list is shared, so the siblings would inherit the
    /// UT61E+'s hardware history unless `verified_if` gates it.
    #[test]
    fn ut61_siblings_declare_nothing_verified() {
        for (id, stability, steps) in all_steps() {
            if id == "ut61eplus" {
                assert!(
                    steps.iter().any(|s| s.verified),
                    "the UT61E+ is verified hardware and must say so"
                );
                continue;
            }
            let ut61_sibling = (d_family(id) == Some(DeviceFamily::Ut61EPlus))
                && stability == Stability::Experimental;
            if ut61_sibling {
                assert!(
                    steps.iter().all(|s| !s.verified),
                    "{id} has never been run, so no step may claim verification"
                );
            }
        }
    }

    /// A word each need's instruction must contain, so a `needs` tag pinned to
    /// the wrong step is caught rather than shipped into the checklist.
    fn need_keyword(need: Need) -> &'static str {
        match need {
            Need::ShortedLeads => "together",
            Need::DcSource => "revers",
            Need::Thermocouple => "temperature",
            Need::LiveWire => "ncv",
            Need::Transistor => "transistor",
            Need::Scr => "thyristor",
        }
    }

    /// The checklist prints labels, so each must say something and say it
    /// only once.
    #[test]
    fn need_labels_are_distinct_and_non_empty() {
        for (i, need) in Need::ALL.iter().enumerate() {
            assert!(!need.label().is_empty(), "{need:?} has no label");
            for other in &Need::ALL[i + 1..] {
                assert_ne!(
                    need.label(),
                    other.label(),
                    "{need:?} and {other:?} share a label"
                );
            }
        }
    }

    /// A step asking for equipment must name it, or the user reads a
    /// checklist that does not match the instructions they are given.
    #[test]
    fn tagged_steps_name_what_they_need() {
        for (id, _, steps) in all_steps() {
            for step in &steps {
                let text = step.instruction.to_lowercase();
                for &need in step.needs {
                    let keyword = need_keyword(need);
                    assert!(
                        text.contains(keyword),
                        "{id} step {} claims {need:?} but its instruction never says {keyword:?}",
                        step.id
                    );
                }
            }
        }
    }

    fn d_family(id: &str) -> Option<DeviceFamily> {
        registry::DEVICES
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.family)
    }
}
