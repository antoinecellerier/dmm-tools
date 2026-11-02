//! A capture step: what the user is asked to set up, what they need on the
//! bench for it, and what a correct parse then shows (`expect.rs`).

use super::Expect;

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
    /// The meter's power-measurement adapter, plugged into a live outlet with
    /// a load plugged into it.
    PowerAdapter,
    /// A transistor to sit in the hFE socket.
    Transistor,
    /// A thyristor for the SCR test.
    Scr,
    /// A 4-20 mA current loop with its DC supply (15-48 V on OWON's and
    /// Voltcraft's meters).
    LoopSource,
    /// A meter's optional DC or USB power measurement module, with what it
    /// measures: a DC supply or USB charger and a load.
    PowerModule,
    /// The VC925 PV's LX-925 irradiance adapter, linked to the meter.
    PvAdapter,
}

impl Need {
    /// Every need, in the order the up-front checklist lists them.
    pub const ALL: [Need; 10] = [
        Need::ShortedLeads,
        Need::DcSource,
        Need::Thermocouple,
        Need::LiveWire,
        Need::PowerAdapter,
        Need::Transistor,
        Need::Scr,
        Need::LoopSource,
        Need::PowerModule,
        Need::PvAdapter,
    ];

    /// What to call this on the checklist, as an article-less noun phrase: it
    /// reads after both "you will need" and "skipped: no".
    pub fn label(self) -> &'static str {
        match self {
            Need::ShortedLeads => "shorted test leads",
            Need::DcSource => "battery or other DC source",
            Need::Thermocouple => "K-type thermocouple",
            Need::LiveWire => "live mains wire nearby",
            Need::PowerAdapter => "power adapter, live outlet and load",
            Need::Transistor => "transistor",
            Need::Scr => "SCR (thyristor)",
            Need::LoopSource => "4-20 mA loop and its 15-48 V supply",
            Need::PowerModule => "DC or USB power measurement module, supply and load",
            Need::PvAdapter => "LX-925 irradiance adapter",
        }
    }
}

/// A step definition for the guided protocol capture wizard.
#[derive(Clone, Copy)]
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
    /// Only the operator's Enter captures: the instruction is a sequence of
    /// presses, and the meter reports the state each one leaves.
    pub wait_for_enter: bool,
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
            wait_for_enter: false,
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
            wait_for_enter: false,
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

    /// Capture on Enter alone. Without it a step with no expectation captures
    /// the first new state that holds, which on a multi-press instruction is
    /// the first press's, or the previous step's clean-up.
    pub const fn wait_for_enter(mut self) -> Self {
        self.wait_for_enter = true;
        self
    }
}
