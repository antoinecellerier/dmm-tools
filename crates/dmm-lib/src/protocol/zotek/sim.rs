//! A simulated ZT-5B / V05B, the `mock-zt5b` device
//! (`docs/research/zotek/reverse-engineered-protocol.md`).
//!
//! Nobody on the project owns a ZOTEK meter, so this stands in for one to
//! run the real driver against. [`SimulatedMeter`] is a transport: it
//! streams scrambled type-2 packets drawn with the layout's own bit table
//! (§4, §6.2, §7.3) and takes the key frames written to it (§8.1). [`MockZt5b`]
//! puts the unchanged [`ZotekProtocol`] on top, so every reading goes
//! through the extractor and the decoder, and every key through the frame
//! builder, as with a meter.
//!
//! The keys do what ZOTEK's app intends; no meter has confirmed them (§8.2
//! leaves open which keys a model honours). Where the app's intent leaves
//! the effect open, the choice made here is named at the key. There is no Ω
//! or mV key, as the ZT-5B has none (§8.2): AUTO finds the resistor.
//!
//! The probes move on their own, as a function of session time since the
//! function was picked: a battery and the mains in V, those and a resistor
//! lifted off now and then in AUTO, a probe nearing a live wire in NCV. So
//! `dmm-cli read` has something to show without anyone pressing a key.

use super::ZotekProtocol;
use super::frame;
use super::glyph::{Cell, Glyph};
use super::layout::{Meaning, Prefix, Unit, ZT5B};
use crate::clock::Clock;
use crate::error::{Error, Result};
use crate::measurement::Measurement;
use crate::protocol::registry::{SelectableDevice, factory};
use crate::protocol::{DeviceFamily, DeviceProfile, Protocol, Stability};
use crate::transport::Transport;
use log::debug;
use std::cell::RefCell;
use std::f64::consts::TAU;
use std::time::{Duration, Instant};

/// The registry id of the simulated meter.
pub(crate) const MOCK_ID: &str = "mock-zt5b";

/// A ZT-5B for trying the ZOTEK driver and its remote keys on; never
/// detected and never looked for over Bluetooth.
pub(crate) static MOCK_ZT5B: SelectableDevice = SelectableDevice {
    id: MOCK_ID,
    display_name: "Mock ZT-5B / V05B (simulated)",
    aliases: &[],
    requires_hardware: false,
    activation_instructions: crate::mock::devices::ACTIVATION,
    family: DeviceFamily::Mock,
    new_protocol: factory::<MockZt5b>,
    fingerprint: None,
    manual_url: Some(
        "https://github.com/antoinecellerier/dmm-tools/blob/main/docs/cli-reference.md#zotek-mock",
    ),
    links: &[],
    bluetooth_only: false,
    bluetooth_names: &[],
};

/// What the simulated meter is set to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Function {
    /// AUTO: the `Auto` word until the probes find a signal (spec §6.4).
    Auto,
    Volts,
    Capacitance,
    Frequency,
    Diode,
    Continuity,
    Ncv,
    Current,
    Celsius,
    Fahrenheit,
}

/// What the LCD shows: the four digits, most significant first, the minus,
/// and the annunciators lit.
#[derive(Clone, PartialEq, Debug)]
struct Display {
    cells: [Cell; 4],
    negative: bool,
    lit: Vec<Meaning>,
}

const BLANK: Cell = Cell {
    glyph: Glyph::Blank,
    dp: false,
};

/// A row of four glyphs with no point.
fn word(glyphs: [Glyph; 4]) -> [Cell; 4] {
    glyphs.map(|glyph| Cell { glyph, dp: false })
}

/// `Auto`, and `EF`, as spec §6.4 draws them.
const AUTO: [Glyph; 4] = [Glyph::A, Glyph::U, Glyph::T, Glyph::O];
const EF: [Glyph; 4] = [Glyph::Blank, Glyph::E, Glyph::F, Glyph::Blank];

/// `0L` with the point before cell `dp_at`, a form spec §11.4 lists.
fn overload(dp_at: usize) -> [Cell; 4] {
    let mut cells = word([Glyph::Blank, Glyph::Digit(0), Glyph::L, Glyph::Blank]);
    cells[dp_at].dp = true;
    cells
}

/// One to four dashes, filled from the left (spec §11.4).
fn dashes(n: usize) -> [Cell; 4] {
    let mut cells = [BLANK; 4];
    for cell in cells.iter_mut().take(n) {
        cell.glyph = Glyph::Dash;
    }
    cells
}

/// The places after the point a 6000-count auto-ranging meter shows
/// `value` with (spec §1's counts).
fn auto_decimals(value: f64) -> usize {
    match value.abs() {
        v if v < 6.0 => 3,
        v if v < 60.0 => 2,
        v if v < 600.0 => 1,
        _ => 0,
    }
}

/// `value` in four digits with `decimals` (at most 3) after the point,
/// leading zeros blanked up to the units digit, and the minus; `None` past
/// four digits.
fn number(value: f64, decimals: usize) -> Option<([Cell; 4], bool)> {
    let decimals = decimals.min(3);
    let scaled = (value.abs() * 10f64.powi(decimals as i32)).round();
    if !scaled.is_finite() || scaled >= 10_000.0 {
        return None;
    }
    let mut n = scaled as u32;
    let mut cells = [BLANK; 4];
    for cell in cells.iter_mut().rev() {
        cell.glyph = Glyph::Digit((n % 10) as u8);
        n /= 10;
    }
    if decimals > 0 {
        cells[4 - decimals].dp = true;
    }
    for cell in cells.iter_mut().take(3 - decimals) {
        if cell.glyph != Glyph::Digit(0) {
            break;
        }
        cell.glyph = Glyph::Blank;
    }
    Some((cells, value < 0.0 && scaled > 0.0))
}

/// A slow wander of `amplitude` around nothing: two sines of unrelated
/// periods, so the readings drift rather than repeat.
fn wander(t: f64, amplitude: f64, slow: f64, fast: f64) -> f64 {
    amplitude * (0.8 * (t * TAU / slow).sin() + 0.2 * (t * TAU / fast).sin())
}

/// `t`'s place in a cycle of `period` seconds.
fn phase(t: f64, period: f64) -> f64 {
    t.rem_euclid(period)
}

/// The Bluetooth icon, byte 7 bit 7: set in every community notification
/// (spec §7.3, §11.4), and read by neither the driver nor the apps.
const BLUETOOTH_ICON: (usize, u8) = (7, 0x80);

/// Where the probes are in V and AUTO.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Source {
    /// Off anything: AUTO shows its word.
    None,
    /// A 9 V battery, part used.
    Battery,
    /// The mains.
    Mains,
}

/// The volts the probes see, as the meter shows them. The ZT-5B picks AC or
/// DC itself, as the auto-only meter it is (spec §1).
fn volts(source: Source, t: f64) -> Option<Display> {
    let (value, coupling) = match source {
        Source::None => return None,
        Source::Battery => (9.137 + wander(t, 0.012, 37.0, 5.3), Meaning::Dc),
        Source::Mains => (229.6 + wander(t, 1.6, 23.0, 3.1), Meaning::Ac),
    };
    let mut lit = vec![Meaning::Unit(Unit::Volt), coupling];
    // Community captures have over-voltage set at 180 and 233 V AC and
    // clear at low voltage (spec §7.3, §11.2).
    if source == Source::Mains {
        lit.push(Meaning::OverVoltage);
    }
    reading(value, auto_decimals(value), lit)
}

/// A number and its annunciators, or OL where it does not fit.
fn reading(value: f64, decimals: usize, lit: Vec<Meaning>) -> Option<Display> {
    let (cells, negative) = number(value, decimals).unwrap_or((overload(2), false));
    Some(Display {
        cells,
        negative,
        lit,
    })
}

/// A 4.7 kΩ resistor, lifted off for 4 s in 18: open leads read OL on the
/// MΩ range.
fn resistance(t: f64) -> Option<Display> {
    if phase(t, 18.0) >= 14.0 {
        return Some(Display {
            cells: overload(2),
            negative: false,
            lit: vec![Meaning::Unit(Unit::Ohm), Meaning::Prefix(Prefix::Mega, &[])],
        });
    }
    let kohm = 4.702 + wander(t, 0.004, 41.0, 3.7);
    reading(
        kohm,
        auto_decimals(kohm),
        vec![Meaning::Unit(Unit::Ohm), Meaning::Prefix(Prefix::Kilo, &[])],
    )
}

/// The simulated meter: its function and what its keys have done.
#[derive(Debug)]
struct Meter {
    function: Function,
    /// When the function was picked; the probes move from there.
    since: Instant,
    /// What HOLD froze, while it is on.
    held: Option<Display>,
    /// What ZERO took off the capacitance, in nF.
    zero_nf: f64,
}

/// The capacitance the probes see, in nF: the leads open (their own
/// stray capacitance) for 5 s, then a 100 nF film capacitor for 15 s.
fn capacitance_nf(t: f64) -> f64 {
    let stray = 0.418 + wander(t, 0.006, 13.0, 2.9);
    if phase(t, 20.0) < 5.0 {
        stray
    } else {
        stray + 98.87 + wander(t, 0.25, 31.0, 4.3)
    }
}

impl Meter {
    fn new(now: Instant) -> Self {
        Self {
            function: Function::Auto,
            since: now,
            held: None,
            zero_nf: 0.0,
        }
    }

    /// Seconds since the function was picked.
    fn elapsed(&self, now: Instant) -> f64 {
        now.checked_duration_since(self.since)
            .unwrap_or(Duration::ZERO)
            .as_secs_f64()
    }

    /// Switch to `function`, as its key does: HOLD and ZERO end with the
    /// function they were set in.
    fn pick(&mut self, function: Function, now: Instant) {
        self.function = function;
        self.since = now;
        self.held = None;
        self.zero_nf = 0.0;
    }

    /// Press the key `code` (spec §8.2).
    fn press(&mut self, code: u8, now: Instant) {
        let pick = match code {
            0xB8 => Function::Auto,
            // The ZT-5B chooses AC or DC itself (spec §1), so one V key.
            0xC4 => Function::Volts,
            0xB0 => Function::Capacitance,
            0xB3 => Function::Frequency,
            // One key for diode and continuity: the first press picks the
            // diode, the next ones swap between the two, as the meter's own
            // SEL cycles its modes (spec §8.2).
            0xB1 if self.function == Function::Diode => Function::Continuity,
            0xB1 => Function::Diode,
            0xB2 => Function::Ncv,
            // Every current code picks the one current function the
            // simulation has; the driver sends `C9` from type 2 (spec §8.2).
            0xC8..=0xCB => Function::Current,
            // The app sends `B7` while °C shows, else `B6` (spec §8.2): `B6`
            // asks for °C and `B7` for °F.
            0xB6 => Function::Celsius,
            0xB7 => Function::Fahrenheit,
            0xB4 => {
                self.held = match self.held {
                    Some(_) => None,
                    None => self.display(now),
                };
                return;
            }
            // Sent only while F shows (spec §8.2): the reading becomes zero.
            0xB5 if self.function == Function::Capacitance && self.held.is_none() => {
                self.zero_nf = capacitance_nf(self.elapsed(now));
                return;
            }
            // ZERO outside capacitance, or a key type 2 has no use for.
            _ => {
                debug!(
                    "zotek sim: key {code:02X} does nothing in {:?}",
                    self.function
                );
                return;
            }
        };
        self.pick(pick, now);
    }

    /// What the LCD shows at `now`, HOLD aside.
    fn live(&self, now: Instant) -> Option<Display> {
        let t = self.elapsed(now);
        let lit = |meanings: &[Meaning]| meanings.to_vec();
        match self.function {
            // The word for 6 s, a battery for 10, the word again, a
            // resistor for 18, the word again, the mains for 10: the ZT-5B
            // matches V and Ω itself (spec §8.2).
            Function::Auto => {
                let source = match phase(t, 56.0) {
                    p if p < 6.0 => Source::None,
                    p if p < 16.0 => Source::Battery,
                    p if p < 22.0 => Source::None,
                    p if p < 40.0 => return resistance(t),
                    p if p < 46.0 => Source::None,
                    _ => Source::Mains,
                };
                volts(source, t).or(Some(Display {
                    cells: word(AUTO),
                    negative: false,
                    lit: Vec::new(),
                }))
            }
            Function::Volts => {
                let source = if phase(t, 24.0) < 12.0 {
                    Source::Battery
                } else {
                    Source::Mains
                };
                volts(source, t)
            }
            Function::Capacitance => {
                let nf = capacitance_nf(t) - self.zero_nf;
                reading(
                    nf,
                    auto_decimals(nf),
                    lit(&[
                        Meaning::Unit(Unit::Farad),
                        Meaning::Prefix(Prefix::Nano, &[]),
                    ]),
                )
            }
            Function::Frequency => {
                let hz = 50.01 + wander(t, 0.03, 31.0, 4.1);
                reading(hz, auto_decimals(hz), lit(&[Meaning::Unit(Unit::Hertz)]))
            }
            // A silicon diode, reversed for 4 s in 16.
            Function::Diode => {
                let diode = [Meaning::Diode, Meaning::Unit(Unit::Volt)];
                if phase(t, 16.0) >= 12.0 {
                    return Some(Display {
                        cells: overload(1),
                        negative: false,
                        lit: lit(&diode),
                    });
                }
                reading(0.612 + wander(t, 0.004, 33.0, 2.7), 3, lit(&diode))
            }
            // A short, opened for 4 s in 14.
            Function::Continuity => {
                let continuity = [Meaning::Continuity, Meaning::Unit(Unit::Ohm)];
                if phase(t, 14.0) >= 10.0 {
                    return Some(Display {
                        cells: overload(3),
                        negative: false,
                        lit: lit(&continuity),
                    });
                }
                reading(0.43 + wander(t, 0.12, 17.0, 1.9), 1, lit(&continuity))
            }
            // A probe nearing a live wire and backing off: EF with nothing
            // near, then one to four dashes and back (spec §6.4, §11.4).
            Function::Ncv => {
                let cells = match phase(t, 12.0) {
                    p if p < 4.0 => word(EF),
                    p if p < 7.0 => dashes(1 + (p - 4.0) as usize),
                    p if p < 9.0 => dashes(4),
                    p => dashes(3 - ((p - 9.0) as usize).min(2)),
                };
                Some(Display {
                    cells,
                    negative: false,
                    lit: Vec::new(),
                })
            }
            Function::Current => {
                let ma = 23.47 + wander(t, 0.08, 27.0, 3.3);
                reading(
                    ma,
                    auto_decimals(ma),
                    lit(&[
                        Meaning::Unit(Unit::Amp),
                        Meaning::Prefix(Prefix::Milli, &[]),
                        Meaning::Dc,
                    ]),
                )
            }
            Function::Celsius | Function::Fahrenheit => {
                let celsius = 23.4 + wander(t, 0.3, 47.0, 6.1);
                if self.function == Function::Celsius {
                    reading(celsius, 1, lit(&[Meaning::Unit(Unit::Celsius)]))
                } else {
                    let fahrenheit = celsius * 9.0 / 5.0 + 32.0;
                    reading(fahrenheit, 1, lit(&[Meaning::Unit(Unit::Fahrenheit)]))
                }
            }
        }
    }

    /// What the LCD shows at `now`: HOLD's frozen display, or the live one.
    fn display(&self, now: Instant) -> Option<Display> {
        match &self.held {
            Some(held) => {
                let mut held = held.clone();
                held.lit.push(Meaning::Hold);
                Some(held)
            }
            None => self.live(now),
        }
    }
}

/// The key a written frame presses: descrambled, `AB CD 03 <key> 00 00 00
/// 00` and their big-endian sum (spec §8.1, §8.2). Anything else is
/// refused, so a fault in the driver's frames shows up as an error rather
/// than as a key the meter ignores.
fn key_code(data: &[u8]) -> Result<u8> {
    let mut frame = data.to_vec();
    frame::xor_key(&mut frame);
    let &[0xAB, 0xCD, 0x03, key, 0, 0, 0, 0, hi, lo] = frame.as_slice() else {
        return Err(Error::invalid_response(
            "zotek sim: not a key-press frame",
            &frame,
        ));
    };
    let expected: u16 = frame[..8].iter().map(|&b| u16::from(b)).sum();
    let actual = u16::from_be_bytes([hi, lo]);
    if actual != expected {
        return Err(Error::ChecksumMismatch { expected, actual });
    }
    Ok(key)
}

/// The simulated meter as a transport: each read hands out the next
/// packet, scrambled as it goes on air; each write is a key press.
pub(crate) struct SimulatedMeter {
    clock: Clock,
    meter: RefCell<Meter>,
    /// What of the last packet no read has taken yet.
    pending: RefCell<Vec<u8>>,
}

impl SimulatedMeter {
    fn new(clock: Clock) -> Self {
        let meter = Meter::new(clock.now());
        Self {
            clock,
            meter: RefCell::new(meter),
            pending: RefCell::new(Vec::new()),
        }
    }

    /// The next packet, descrambled.
    fn packet(&self) -> Result<Vec<u8>> {
        let display = self.meter.borrow().display(self.clock.now());
        let packet = display.and_then(|d| {
            let mut packet = ZT5B.draw(&d.cells, d.negative, &d.lit)?;
            packet[BLUETOOTH_ICON.0] |= BLUETOOTH_ICON.1;
            Some(packet)
        });
        // Every display above uses bits the layout lists; a test holds it.
        packet.ok_or_else(|| Error::invalid_response_msg("zotek sim: undrawable display"))
    }
}

impl Transport for SimulatedMeter {
    fn write(&self, data: &[u8]) -> Result<()> {
        let key = key_code(data)?;
        self.meter.borrow_mut().press(key, self.clock.now());
        Ok(())
    }

    fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
        let mut pending = self.pending.borrow_mut();
        if pending.is_empty() {
            *pending = self.packet()?;
            frame::xor_key(&mut pending);
        }
        let n = buf.len().min(pending.len());
        buf[..n].copy_from_slice(&pending[..n]);
        pending.drain(..n);
        Ok(n)
    }

    fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// The `mock-zt5b` device: the ZT-5B's own driver, reading from and
/// pressing keys on [`SimulatedMeter`] instead of a Bluetooth link.
///
/// The simulated meter lives here rather than in the session's transport so
/// that every path that opens a registry entry by its factory opens it
/// whole, as it does the UT61E+ mock.
pub(crate) struct MockZt5b {
    meter: SimulatedMeter,
    driver: ZotekProtocol,
    profile: DeviceProfile,
}

impl MockZt5b {
    /// The meter on `clock`, starting in AUTO.
    pub(crate) fn new(clock: Clock) -> Self {
        let driver = ZotekProtocol::new_zt5b();
        let profile = DeviceProfile {
            family_name: "mock",
            model_name: "Mock ZT-5B / V05B",
            // As the UT61E+ mock: a simulation needs no hardware run, and
            // the GUI shows no EXPERIMENTAL badge for it.
            stability: Stability::Verified,
            ..*driver.profile()
        };
        Self {
            meter: SimulatedMeter::new(clock),
            driver,
            profile,
        }
    }
}

impl Default for MockZt5b {
    fn default() -> Self {
        Self::new(Clock::real())
    }
}

impl Protocol for MockZt5b {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        self.driver.init(&self.meter)
    }

    fn request_measurement(&mut self, _transport: &dyn Transport) -> Result<Measurement> {
        self.driver.request_measurement(&self.meter)
    }

    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        self.driver.parse_payload(payload)
    }

    fn send_command(&mut self, _transport: &dyn Transport, command: &str) -> Result<()> {
        self.driver.send_command(&self.meter, command)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measurement::MeasuredValue;
    use crate::protocol::capture_reports;
    use crate::transport::NullTransport;

    fn mock() -> (MockZt5b, Clock) {
        let clock = Clock::manual();
        (MockZt5b::new(clock.clone()), clock)
    }

    /// Read one reading, asserting the decoder reported nothing.
    fn read(mock: &mut MockZt5b) -> Measurement {
        let (m, reports) = capture_reports(|| mock.request_measurement(&NullTransport));
        assert!(reports.is_empty(), "{reports:?}");
        m.unwrap()
    }

    fn press(mock: &mut MockZt5b, command: &str) {
        mock.send_command(&NullTransport, command).unwrap();
    }

    fn value(m: &Measurement) -> f64 {
        match m.value {
            MeasuredValue::Normal(v) => v,
            ref other => panic!("{}: not a number: {other:?}", m.mode),
        }
    }

    fn secs(s: f64) -> Duration {
        Duration::from_secs_f64(s)
    }

    /// Every function, HOLD and ZERO included, over two of its cycles:
    /// every packet decodes without a report, and every display is one the
    /// layout can draw.
    #[test]
    fn every_packet_decodes_quietly() {
        let cases: &[&[&str]] = &[
            &["auto_function"],
            &["volts"],
            &["capacitance"],
            &["hz"],
            &["diode_continuity"],
            &["diode_continuity", "diode_continuity"],
            &["ncv"],
            &["current"],
            &["temp_unit"],
            &["temp_unit", "temp_unit"],
        ];
        for keys in cases {
            let (mut mock, clock) = mock();
            for key in *keys {
                press(&mut mock, key);
            }
            for step in 0..260 {
                clock.advance(secs(0.25));
                let m = read(&mut mock);
                assert!(!m.mode.starts_with("Unknown"), "{keys:?}: {}", m.mode);
                if step == 100 || step == 110 {
                    press(&mut mock, "hold");
                }
            }
        }
        let (mut mock, clock) = mock();
        press(&mut mock, "capacitance");
        clock.advance(secs(2.0));
        press(&mut mock, "zero");
        for _ in 0..100 {
            clock.advance(secs(0.25));
            read(&mut mock);
        }
    }

    /// The meter starts in AUTO: the word, then a battery, the word, a
    /// resistor, the word, then the mains with over-voltage lit.
    #[test]
    fn auto_shows_its_word_until_the_probes_find_a_signal() {
        let (mut mock, clock) = mock();
        let is_word = |m: &Measurement| matches!(m.value, MeasuredValue::NoReading("Auto"));
        let m = read(&mut mock);
        assert!(is_word(&m), "{m:?}");
        assert_eq!(m.mode, "Auto");

        clock.advance(secs(8.0));
        let m = read(&mut mock);
        assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("DC V", "V"));
        assert!((9.0..9.3).contains(&value(&m)), "{m:?}");
        assert!(!m.flags.hv_warning);

        clock.advance(secs(10.0));
        assert!(is_word(&read(&mut mock)));

        clock.advance(secs(6.0));
        let m = read(&mut mock);
        assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("Ω", "kΩ"));
        assert!((4.6..4.8).contains(&value(&m)), "{m:?}");

        clock.advance(secs(18.0));
        assert!(is_word(&read(&mut mock)));

        clock.advance(secs(6.0));
        let m = read(&mut mock);
        assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("AC V", "V"));
        assert!((225.0..235.0).contains(&value(&m)), "{m:?}");
        assert!(m.flags.hv_warning);

        press(&mut mock, "volts");
        press(&mut mock, "auto_function");
        let m = read(&mut mock);
        assert!(matches!(m.value, MeasuredValue::NoReading("Auto")), "{m:?}");
    }

    /// Each function key lands on its function: (key, mode, unit).
    #[test]
    fn each_key_picks_its_function() {
        for (key, mode, unit) in [
            ("volts", "DC V", "V"),
            ("capacitance", "Capacitance", "nF"),
            ("hz", "Hz", "Hz"),
            ("diode_continuity", "Diode", "V"),
            ("current", "DC A", "mA"),
            ("temp_unit", "°C", "°C"),
        ] {
            let (mut mock, clock) = mock();
            press(&mut mock, key);
            // Past the capacitance's open-lead stretch.
            clock.advance(secs(6.0));
            let m = read(&mut mock);
            assert_eq!((m.mode.as_ref(), m.unit.as_ref()), (mode, unit), "{key}");
            value(&m);
        }
        let (mut mock, _) = mock();
        press(&mut mock, "ncv");
        let m = read(&mut mock);
        assert!(matches!(m.value, MeasuredValue::NcvLevel(0)), "{m:?}");
    }

    #[test]
    fn diode_continuity_swaps_between_the_two() {
        let (mut mock, _) = mock();
        for mode in ["Diode", "Continuity", "Diode"] {
            press(&mut mock, "diode_continuity");
            assert_eq!(read(&mut mock).mode, mode);
        }
    }

    /// The driver sends `B7` while °C shows, so the key swaps the scale.
    #[test]
    fn temp_unit_swaps_celsius_and_fahrenheit() {
        let (mut mock, _) = mock();
        press(&mut mock, "temp_unit");
        let celsius = value(&read(&mut mock));
        press(&mut mock, "temp_unit");
        let m = read(&mut mock);
        assert_eq!(m.unit, "°F");
        assert!((value(&m) - (celsius * 9.0 / 5.0 + 32.0)).abs() < 0.1);
        press(&mut mock, "temp_unit");
        assert_eq!(read(&mut mock).unit, "°C");
    }

    /// NCV climbs from EF through the four levels and back.
    #[test]
    fn ncv_moves_through_its_levels() {
        let (mut mock, clock) = mock();
        press(&mut mock, "ncv");
        let mut levels = Vec::new();
        for _ in 0..12 {
            let m = read(&mut mock);
            assert_eq!(m.mode, "NCV");
            let MeasuredValue::NcvLevel(level) = m.value else {
                panic!("{m:?}");
            };
            if levels.last() != Some(&level) {
                levels.push(level);
            }
            clock.advance(secs(1.0));
        }
        assert_eq!(levels, [0, 1, 2, 3, 4, 3, 2, 1]);
    }

    /// HOLD freezes the display and lights its flag; a second press lets go.
    #[test]
    fn hold_freezes_the_reading() {
        let (mut mock, clock) = mock();
        press(&mut mock, "hz");
        clock.advance(secs(3.0));
        press(&mut mock, "hold");
        let held = read(&mut mock);
        assert!(held.flags.hold);
        clock.advance(secs(7.0));
        let still = read(&mut mock);
        assert_eq!(still.display_raw, held.display_raw);
        assert!(still.flags.hold);
        press(&mut mock, "hold");
        let live = read(&mut mock);
        assert!(!live.flags.hold);
        assert_ne!(live.display_raw, held.display_raw);
    }

    /// ZERO takes the open leads' stray capacitance off; the driver refuses
    /// it outside capacitance, and a function key ends it.
    #[test]
    fn zero_clears_the_stray_capacitance() {
        let (mut mock, clock) = mock();
        let err = mock.send_command(&NullTransport, "zero").unwrap_err();
        assert!(matches!(err, Error::CommandRejected(_)), "{err:?}");

        press(&mut mock, "capacitance");
        clock.advance(secs(1.0));
        let stray = value(&read(&mut mock));
        assert!(stray > 0.3, "{stray}");
        press(&mut mock, "zero");
        assert!(value(&read(&mut mock)).abs() < 0.02);
        clock.advance(secs(6.0));
        let zeroed = value(&read(&mut mock));
        assert!((95.0..100.0).contains(&zeroed), "{zeroed}");

        press(&mut mock, "capacitance");
        clock.advance(secs(1.0));
        assert!(value(&read(&mut mock)) > 0.3);
    }

    /// MAX/MIN has nothing to show on type 2, so it is not offered.
    #[test]
    fn minmax_is_not_offered() {
        let (mut mock, _) = mock();
        assert!(!mock.profile().supported_commands.contains(&"minmax"));
        let err = mock.send_command(&NullTransport, "minmax").unwrap_err();
        assert!(matches!(err, Error::UnsupportedCommand(_)), "{err}");
    }

    /// AUTO's resistor and the diode lift off now and then and read OL.
    #[test]
    fn open_probes_read_ol() {
        for (key, at, unit) in [
            ("auto_function", 33.0, "MΩ"),
            ("diode_continuity", 13.0, "V"),
        ] {
            let (mut mock, clock) = mock();
            press(&mut mock, key);
            clock.advance(secs(at));
            let m = read(&mut mock);
            assert!(matches!(m.value, MeasuredValue::Overload), "{key}: {m:?}");
            assert_eq!(m.unit, unit);
        }
    }

    /// The readings drift, not sit on a set point.
    #[test]
    fn readings_move() {
        let (mut mock, clock) = mock();
        press(&mut mock, "current");
        let mut seen = Vec::new();
        for _ in 0..20 {
            seen.push(read(&mut mock).display_raw);
            clock.advance(secs(0.4));
        }
        seen.dedup();
        assert!(seen.len() > 10, "{seen:?}");
    }

    #[test]
    fn a_frame_that_is_not_a_key_press_is_refused() {
        let meter = SimulatedMeter::new(Clock::manual());
        let good = crate::protocol::zotek::keys::frame(0xB4);
        assert!(meter.write(&good).is_ok());
        let mut bad_sum = good;
        bad_sum[9] ^= 1;
        assert!(matches!(
            meter.write(&bad_sum),
            Err(Error::ChecksumMismatch { .. })
        ));
        assert!(meter.write(&good[..9]).is_err());
        let mut clock_set = [0xAB, 0xCD, 0x04, 12, 0, 0, 0, 0, 0, 0];
        let sum: u16 = clock_set[..8].iter().map(|&b| u16::from(b)).sum();
        clock_set[8..].copy_from_slice(&sum.to_be_bytes());
        frame::xor_key(&mut clock_set);
        assert!(meter.write(&clock_set).is_err());
    }

    /// Reads that take less than a packet get the rest on the next read.
    #[test]
    fn short_reads_take_a_packet_in_pieces() {
        let meter = SimulatedMeter::new(Clock::manual());
        let mut first = [0u8; 4];
        let mut rest = [0u8; 64];
        assert_eq!(meter.read_timeout(&mut first, 0).unwrap(), 4);
        assert_eq!(meter.read_timeout(&mut rest, 0).unwrap(), 6);
        let mut whole = first.to_vec();
        whole.extend_from_slice(&rest[..6]);
        let (packet, consumed) = frame::extract_packet(&whole).unwrap().unwrap();
        assert_eq!(consumed, 10);
        assert_eq!(packet[..3], [0x5A, 0xA5, 2]);
    }

    #[test]
    fn number_draws_the_meters_digits() {
        let text = |value: f64, decimals: usize| {
            let (cells, negative) = number(value, decimals).unwrap();
            let mut s = String::from(if negative { "-" } else { "" });
            for cell in cells {
                if cell.dp {
                    s.push('.');
                }
                s.push(match cell.glyph {
                    Glyph::Digit(d) => char::from(b'0' + d),
                    _ => ' ',
                });
            }
            s
        };
        assert_eq!(text(9.137, 2), " 9.14");
        assert_eq!(text(229.64, 1), "229.6");
        assert_eq!(text(0.612, 3), "0.612");
        assert_eq!(text(0.43, 1), "  0.4");
        assert_eq!(text(-0.0021, 3), "-0.002");
        assert_eq!(text(-0.0001, 3), "0.000");
        assert!(number(10_000.0, 0).is_none());
        assert!(number(f64::NAN, 1).is_none());
    }
}
