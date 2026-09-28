//! Fixtures more than one of the CLI's test modules uses: a fake meter
//! offering the settings a test lays out, and device selections.

use dmm_lib::measurement::MeasuredValue;
use dmm_lib::protocol::registry::{self, Selection};
use dmm_lib::protocol::{Choice, Setting};

/// A device string, resolved as `main` resolves it.
pub(crate) fn selection(s: &str) -> Selection {
    registry::resolve_selection(s).unwrap_or_else(|| panic!("{s} must resolve"))
}

pub(crate) fn mode_choice(id: u16, label: &'static str, current: bool) -> Choice {
    Choice {
        id,
        label: std::borrow::Cow::Borrowed(label),
        current,
    }
}

/// Ids the fake meter below gives its choices, spaced like the UT181A's
/// variant nibble so what `select` is asked for is realistic.
pub(crate) fn fake_mode_id(index: usize) -> u16 {
    0x1111 + (index as u16) * 0x10
}

static FAKE_PROFILE: dmm_lib::protocol::DeviceProfile = dmm_lib::protocol::DeviceProfile {
    family_name: "Fake",
    model_name: "Fake meter",
    stability: dmm_lib::protocol::Stability::Experimental,
    supported_commands: &[],
    max_aux_values: 0,
    verification_issue: None,
    meter_keys: dmm_lib::protocol::MeterKeys::NONE,
};

/// The ids a family gives a setting's choices. Mode ids are the family's
/// own, spaced like the UT181A's variant nibble; every other setting
/// counts from zero, where zero is off or auto.
fn fake_choice_id(setting: Setting, index: usize) -> u16 {
    match setting {
        Setting::Mode => fake_mode_id(index),
        _ => index as u16,
    }
}

/// One setting the fake meter offers: its labels, and which of them it
/// sits on.
pub(crate) type FakeList = (Setting, &'static [&'static str], usize);

/// A meter offering exactly `lists`, each sitting where the list says.
struct FakeMeter {
    lists: Vec<FakeList>,
    live: std::collections::HashMap<Setting, usize>,
    switched: bool,
    quirks: Quirks,
    /// What `select` was asked for, so a test can assert the meter was
    /// left alone.
    selected: SelectedIds,
}

/// How the fake meter misbehaves after a `select`. The default is a meter
/// that simply works.
#[derive(Default)]
pub(crate) struct Quirks {
    /// Handed out in place of the readings that follow a successful
    /// switch — a garbled frame, or a meter gone quiet across it.
    pub(crate) post_switch_errors: Vec<dmm_lib::error::Error>,
    /// What every `select` answers instead of switching.
    pub(crate) refusal: Option<&'static str>,
    /// Takes the command and then never reports the new value.
    pub(crate) deaf: bool,
}

impl FakeMeter {
    fn labels(&self, setting: Setting) -> Option<&'static [&'static str]> {
        self.lists
            .iter()
            .find(|(s, _, _)| *s == setting)
            .map(|(_, labels, _)| *labels)
    }

    fn live_index(&self, setting: Setting) -> usize {
        self.live.get(&setting).copied().unwrap_or(0)
    }
}

impl dmm_lib::protocol::Protocol for FakeMeter {
    // Answers each request at once.
    fn delivery(&self) -> dmm_lib::protocol::Delivery {
        dmm_lib::protocol::Delivery::Polled
    }

    // Nothing queues: each reading is built when asked for.
    fn discard_input(
        &mut self,
        _t: &dyn dmm_lib::transport::Transport,
    ) -> dmm_lib::error::Result<()> {
        Ok(())
    }

    fn init(&mut self, _t: &dyn dmm_lib::transport::Transport) -> dmm_lib::error::Result<()> {
        Ok(())
    }

    fn request_measurement(
        &mut self,
        _t: &dyn dmm_lib::transport::Transport,
    ) -> dmm_lib::error::Result<dmm_lib::measurement::Measurement> {
        if self.switched && !self.quirks.post_switch_errors.is_empty() {
            return Err(self.quirks.post_switch_errors.remove(0));
        }
        let mode_index = self.live_index(Setting::Mode);
        let mode = self
            .labels(Setting::Mode)
            .map_or("DC V", |labels| labels[mode_index]);
        // Autoranging settled on 22V; a manual rung reports itself.
        let range_index = self.live_index(Setting::Range);
        let range = match self.labels(Setting::Range) {
            Some(labels) if range_index != 0 => labels[range_index],
            _ => "22V",
        };
        Ok(dmm_lib::measurement::Measurement {
            mode: mode.into(),
            mode_raw: fake_mode_id(mode_index),
            range_label: range.into(),
            ..dmm_lib::measurement::Measurement::test_fixture(
                MeasuredValue::Normal(1.0),
                "V",
                dmm_lib::flags::StatusFlags::default(),
            )
        })
    }

    fn parse_payload(
        &self,
        _payload: &[u8],
    ) -> dmm_lib::error::Result<dmm_lib::measurement::Measurement> {
        Err(dmm_lib::error::Error::UnsupportedCommand(
            "parse_payload: the fake meter has no wire format".to_string(),
        ))
    }

    fn send_command(
        &mut self,
        _t: &dyn dmm_lib::transport::Transport,
        command: &str,
    ) -> dmm_lib::error::Result<()> {
        Err(dmm_lib::error::Error::UnsupportedCommand(
            command.to_string(),
        ))
    }

    fn get_name(
        &mut self,
        _t: &dyn dmm_lib::transport::Transport,
    ) -> dmm_lib::error::Result<Option<String>> {
        Ok(None)
    }

    fn profile(&self) -> &dmm_lib::protocol::DeviceProfile {
        &FAKE_PROFILE
    }

    fn choices(
        &self,
        setting: Setting,
        _current: &dmm_lib::measurement::Measurement,
    ) -> Vec<Choice> {
        let Some(labels) = self.labels(setting) else {
            return Vec::new();
        };
        let live = self.live_index(setting);
        labels
            .iter()
            .enumerate()
            .map(|(i, label)| Choice {
                id: fake_choice_id(setting, i),
                label: std::borrow::Cow::Borrowed(label),
                current: i == live,
            })
            .collect()
    }

    fn select(
        &mut self,
        _t: &dyn dmm_lib::transport::Transport,
        setting: Setting,
        id: u16,
    ) -> dmm_lib::error::Result<()> {
        self.selected.lock().expect("poisoned").push((setting, id));
        if let Some(detail) = self.quirks.refusal {
            return Err(dmm_lib::error::Error::CommandRejected(detail.to_string()));
        }
        let Some(len) = self.labels(setting).map(<[&str]>::len) else {
            return Err(dmm_lib::error::Error::UnsupportedCommand(format!(
                "{setting} cannot be set on this meter"
            )));
        };
        match (0..len).find(|&i| fake_choice_id(setting, i) == id) {
            Some(i) => {
                if !self.quirks.deaf {
                    self.live.insert(setting, i);
                }
                self.switched = true;
                Ok(())
            }
            None => Err(dmm_lib::error::Error::UnsupportedCommand(format!(
                "{setting} {id:#06x}"
            ))),
        }
    }
}

pub(crate) type SelectedIds = std::sync::Arc<std::sync::Mutex<Vec<(Setting, u16)>>>;
pub(crate) type FakeDmm = dmm_lib::Dmm<dmm_lib::transport::NullTransport>;

pub(crate) fn fake_meter(lists: &[FakeList]) -> (FakeDmm, SelectedIds) {
    fake_meter_with(lists, Quirks::default())
}

pub(crate) fn fake_meter_with(lists: &[FakeList], quirks: Quirks) -> (FakeDmm, SelectedIds) {
    let selected: SelectedIds = Default::default();
    let meter = FakeMeter {
        lists: lists.to_vec(),
        live: lists.iter().map(|&(s, _, live)| (s, live)).collect(),
        switched: false,
        quirks,
        selected: std::sync::Arc::clone(&selected),
    };
    let dmm = dmm_lib::Dmm::new(dmm_lib::transport::NullTransport, Box::new(meter))
        .expect("the fake meter needs no transport");
    (dmm, selected)
}
