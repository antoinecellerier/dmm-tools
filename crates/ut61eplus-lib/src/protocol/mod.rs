pub mod framing;
pub mod ut171;
pub mod ut181a;
pub mod ut61eplus;
pub mod ut8803;

use crate::error::Result;
use crate::measurement::Measurement;
use crate::transport::Transport;

/// Protocol stability level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stability {
    /// Verified against real hardware.
    Verified,
    /// Based on reverse engineering, not yet verified against real hardware.
    Experimental,
}

/// Static profile information about a device.
pub struct DeviceProfile {
    pub family_name: &'static str,
    pub model_name: &'static str,
    pub stability: Stability,
    pub supported_commands: &'static [&'static str],
}

/// Device family selector for opening a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceFamily {
    /// UT61E+, UT61B+, UT61D+, UT161B, UT161D, UT161E
    Ut61EPlus,
    /// UT8803 / UT8803E bench multimeter
    Ut8803,
    /// UT171A / UT171B / UT171C
    Ut171,
    /// UT181A
    Ut181a,
    /// Simulated device for testing and demos
    Mock,
}

impl std::fmt::Display for DeviceFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceFamily::Ut61EPlus => write!(f, "ut61eplus"),
            DeviceFamily::Ut8803 => write!(f, "ut8803"),
            DeviceFamily::Ut171 => write!(f, "ut171"),
            DeviceFamily::Ut181a => write!(f, "ut181a"),
            DeviceFamily::Mock => write!(f, "mock"),
        }
    }
}

impl DeviceFamily {
    /// User-facing instructions for enabling data transmission on this device.
    pub fn activation_instructions(&self) -> &'static str {
        match self {
            DeviceFamily::Ut61EPlus => {
                "\
1. Insert the USB module into the meter
2. Turn the meter on
3. Long press the USB/Hz button
4. The S icon appears on the LCD"
            }
            DeviceFamily::Ut8803 => {
                "\
1. Connect the USB cable to the meter
2. Turn the meter on"
            }
            DeviceFamily::Ut171 => {
                "\
1. Connect the USB cable to the meter
2. Turn the meter on
3. Go to SETUP -> Communication -> ON"
            }
            DeviceFamily::Ut181a => {
                "\
1. Connect the USB cable to the meter
2. Turn the meter on
3. Go to SETUP -> Communication -> ON
Note: this setting resets on power cycle."
            }
            DeviceFamily::Mock => "No setup required \u{2014} this is a simulated device.",
        }
    }
}

impl std::str::FromStr for DeviceFamily {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ut61eplus" | "ut61e+" | "ut61e" | "ut61bplus" | "ut61b+" | "ut61b" | "ut61dplus"
            | "ut61d+" | "ut61d" | "ut161b" | "ut161d" | "ut161e" | "ut161" => {
                Ok(DeviceFamily::Ut61EPlus)
            }
            "ut8803" | "ut8803e" => Ok(DeviceFamily::Ut8803),
            "ut171" | "ut171a" | "ut171b" | "ut171c" => Ok(DeviceFamily::Ut171),
            "ut181a" | "ut181" => Ok(DeviceFamily::Ut181a),
            "mock" => Ok(DeviceFamily::Mock),
            _ => Err(format!("unknown device family: {s}")),
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
}
