//! Help and version text shared by the `dmm-cli` and `dmm-gui` binaries.
//!
//! The lists come from the authorities on what exists (`dmm-lib`'s registry
//! for devices, [`MockMode::ALL`] for mock scenarios), so an addition there
//! reaches both binaries' `--help`. The prose (setup hint, experimental
//! warning) is here because the two copies had already drifted apart, and
//! comes from `i18n/<locale>/shared.ftl` through [`Help`], in the language
//! of the [`Localizer`] it is built on. The long name of a link stays in
//! `dmm-lib` ([`Link::full_name`]), whose error messages use it.
//!
//! The per-crate build values (`CARGO_PKG_VERSION`, `GIT_HASH`) are passed in
//! rather than read here: `env!` would capture *this* crate's values, not the
//! binary's.

use crate::l10n::{Localizer, Resource};
use crate::reopen::SeveralCables;
use crate::tr;
use dmm_lib::error::{BluetoothOnlyMiss, Error, ErrorKind, FrameRemedy, Rejected, WrongMeter};
use dmm_lib::list_devices;
use dmm_lib::mock::MockMode;
use dmm_lib::protocol::registry::SelectableDevice;
use dmm_lib::protocol::{Stability, registry};
use dmm_lib::text::TextKind;
use dmm_lib::transport::Link;
use std::borrow::Cow;
use std::sync::OnceLock;

/// Version text for `--version`, with the git hash appended on dev builds.
///
/// Leaks on the dev path because clap's `version` attribute needs a
/// `&'static str`; it happens once per process.
pub fn version_string(version: &'static str, git_hash: &str) -> &'static str {
    if version.contains("-dev") {
        Box::leak(format!("{version} ({git_hash})").into_boxed_str())
    } else {
        version
    }
}

/// Short version label for display in a UI, e.g. `v0.6.0-dev (abc1234)`.
pub fn version_label(version: &str, git_hash: &str) -> String {
    if version.contains("-dev") {
        format!("v{version} ({git_hash})")
    } else {
        format!("v{version}")
    }
}

/// The project page, where the docs and the issue tracker are.
pub const PROJECT_URL: &str = "https://github.com/antoinecellerier/dmm-tools";

/// The setup guide, which the Linux cable help points a headless machine to.
#[cfg(target_os = "linux")]
const SETUP_GUIDE_URL: &str =
    "https://github.com/antoinecellerier/dmm-tools/blob/main/docs/setup.md";

/// Where Windows gets the CP2110's driver.
#[cfg(target_os = "windows")]
const CP2110_DRIVER_URL: &str = "https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers";

/// The device catalog, which lists the Bluetooth adapters and meters.
const CATALOG_URL: &str =
    "https://github.com/antoinecellerier/dmm-tools/blob/main/docs/supported-devices.md";

/// The messages of [`Help`]: `i18n/<locale>/shared.ftl`, in every language
/// the binaries ship.
pub const RESOURCES: &[Resource] = &[
    crate::ftl!("en-US", "shared.ftl"),
    crate::ftl!("fr", "shared.ftl"),
    crate::ftl!("pt-BR", "shared.ftl"),
    crate::ftl!("zh-CN", "shared.ftl"),
    crate::ftl!("es", "shared.ftl"),
    crate::ftl!("ja", "shared.ftl"),
];

/// The help in English, for what stays English whatever the language: shell
/// completions, and the docs' generated tables.
pub fn english() -> Help<'static> {
    static ENGLISH: OnceLock<Localizer> = OnceLock::new();
    Help(
        ENGLISH.get_or_init(|| Localizer::en_us(&[RESOURCES, crate::l10n::LIB_RESOURCES].concat())),
    )
}

impl Localizer {
    /// The help text in this localizer's language. Its resources must
    /// include [`RESOURCES`] and the library's names
    /// ([`LIB_RESOURCES`](crate::l10n::LIB_RESOURCES)), which the device
    /// list shows.
    pub fn help(&self) -> Help<'_> {
        Help(self)
    }
}

/// The help text both binaries print, in one language.
#[derive(Clone, Copy)]
pub struct Help<'a>(&'a Localizer);

impl<'a> Help<'a> {
    /// What both binaries call the link to [`PROJECT_URL`].
    pub fn project_link(self) -> Cow<'a, str> {
        tr!(self.0, "project-link")
    }

    /// The `--help` footer line naming the project page.
    pub fn project_link_line(self) -> String {
        tr!(self.0, "project-link-line", url = PROJECT_URL).into_owned()
    }

    /// Long help for a `--device` flag: `intro`, then one line per known
    /// device, under its brand as the GUI's device list groups them
    /// ([`registry::brand_groups`]).
    ///
    /// `intro` is the only part that differs between the binaries — the
    /// CLI's is bare, the GUI's notes that the flag overrides saved settings.
    pub fn device_help(self, intro: &str) -> String {
        let rows = registry::listings().count();
        let mut help = String::with_capacity(intro.len() + rows * 48 + 160);
        help.push_str(intro);
        help.push_str("\n\n");
        help.push_str(&tr!(self.0, "device-list-heading"));
        help.push('\n');
        // The names line up past the longest id.
        let width = registry::listings()
            .map(|l| l.id().len())
            .max()
            .unwrap_or(0);
        // Leads the list: it is what the flag does when nothing names a
        // meter, and the one value the registry does not carry.
        help.push_str(&format!(
            "  {:<width$} {}\n",
            registry::AUTO_DEVICE_ID,
            tr!(self.0, "device-auto-default")
        ));
        // Brands and model names are proper nouns but for the simulated
        // ones, which the localizer translates; ids stay as typed.
        for (brand, rows) in registry::brand_groups() {
            help.push_str(&format!("\n  {}\n", self.0.brand(brand)));
            for row in rows {
                let tag = self
                    .device_tag(row.device, false)
                    .map_or(String::new(), |t| format!(" ({t})"));
                help.push_str(&format!(
                    "    {:<width$} {}{}\n",
                    row.id(),
                    self.0.label(TextKind::Model, row.model_name()),
                    tag
                ));
            }
        }
        help.push('\n');
        help.push_str(&tr!(self.0, "device-aliases"));
        help
    }

    /// What `auto` does: beside it in `--help`, a shell's completion menu
    /// and the CLI reference's table, and the GUI's Auto-detect hover.
    pub fn auto_description(self) -> Cow<'a, str> {
        tr!(self.0, "device-auto")
    }

    /// What a device listing says beside a device's name: that it needs no
    /// hardware, or how far its support is verified.
    ///
    /// `--help` leaves a verified meter untagged and `docs/cli-reference.md`'s
    /// table tags every row, so `verified_too` asks for that meter's tag too.
    /// Shared so the two cannot disagree about which devices are
    /// experimental.
    pub fn device_tag(self, device: &SelectableDevice, verified_too: bool) -> Option<Cow<'a, str>> {
        self.tag_text(DeviceTag::of(device), verified_too)
    }

    /// [`Help::device_tag`] for a tag worked out beforehand.
    pub fn tag_text(self, tag: DeviceTag, verified_too: bool) -> Option<Cow<'a, str>> {
        match tag {
            DeviceTag::NoHardware => Some(tr!(self.0, "device-no-hardware")),
            DeviceTag::Support(stability) => {
                (verified_too || !stability.is_verified()).then(|| self.stability(stability))
            }
        }
    }

    /// How far a meter's support is verified, as device lists tag it.
    pub fn stability(self, stability: Stability) -> Cow<'a, str> {
        match stability {
            Stability::Verified => tr!(self.0, "stability-verified"),
            Stability::PartlyVerified => tr!(self.0, "stability-partly-verified"),
            Stability::Experimental => tr!(self.0, "stability-experimental"),
        }
    }

    /// Long help for a `--mock-mode` flag: `intro`, the mode list, then
    /// `example`.
    ///
    /// The list is rendered from [`MockMode::ALL`] for the same reason the
    /// device list comes from the registry — the CLI's hardcoded copy had
    /// fallen four modes behind the mock. `intro` and `example` differ
    /// between the binaries: the CLI's flag sits on a subcommand, the GUI's
    /// implies `--device mock`.
    pub fn mock_mode_help(self, intro: &str, example: &str) -> String {
        format!(
            "{intro}\n\n{}\n\n{}",
            tr!(self.0, "mock-mode-list", modes = MockMode::label_list()),
            tr!(self.0, "mock-mode-example", example = example)
        )
    }

    /// What both binaries answer when the clock flags meet a hardware
    /// device.
    ///
    /// `--mock-clock-scale` and `--mock-clock-preseed` bend session time,
    /// and a real meter is paced by USB: honouring them there would stamp
    /// readings with instants the meter never produced. A replay takes them
    /// too — its readings are already recorded, so time can be run through
    /// them at any speed. Shared so the CLI and the GUI refuse with the same
    /// sentence.
    pub fn mock_clock_mock_only(self) -> Cow<'a, str> {
        tr!(self.0, "mock-clock-mock-only")
    }

    /// The name of a transport's link
    /// ([`dmm_lib::transport::Transport::link`]) for a status line that
    /// already names the meter.
    ///
    /// "Bluetooth adapter" doubles the width of a UT61E+ label for a word
    /// the label around it no longer needs. The long form is the library's
    /// own [`Link::full_name`].
    pub fn short_name(self, link: Link) -> Cow<'a, str> {
        match link {
            Link::UsbCable => tr!(self.0, "link-usb-cable"),
            Link::Bluetooth => tr!(self.0, "link-bluetooth"),
        }
    }

    /// The GUI setting that lets an open search the radio, as its checkbox
    /// reads. The CLI reads the same saved setting, so its help names it
    /// too.
    pub fn bluetooth_setting(self) -> Cow<'a, str> {
        tr!(self.0, "bluetooth-setting")
    }

    /// What turns the radio search back on in the CLI, for a meter with the
    /// radio built in that was not looked for: `flag_given` when
    /// `--no-bluetooth` switched it off, else the saved setting did.
    pub fn cli_bluetooth_off_hint(self, flag_given: bool) -> Cow<'a, str> {
        if flag_given {
            tr!(self.0, "bluetooth-off-cli-flag")
        } else {
            tr!(self.0, "bluetooth-off-cli-setting")
        }
    }

    /// How to open the meter as the model it is, after a message saying it
    /// is another, as dmm-gui says it: an entry under the pick's brand, and
    /// Auto-detect as well unless nothing was `picked`, Auto-detect having
    /// opened it.
    pub fn gui_choose_meter(
        self,
        choice: MeterChoice,
        picked: Option<registry::Listing>,
    ) -> String {
        self.choose_meter(true, choice, picked)
    }

    /// [`Self::gui_choose_meter`] as dmm-cli says it.
    pub fn cli_choose_meter(
        self,
        choice: MeterChoice,
        picked: Option<registry::Listing>,
    ) -> String {
        self.choose_meter(false, choice, picked)
    }

    fn choose_meter(
        self,
        gui: bool,
        choice: MeterChoice,
        picked: Option<registry::Listing>,
    ) -> String {
        let (key, listing) = match choice {
            // Auto-detect opened it, so it is no way out.
            MeterChoice::Entry(device) if picked.is_none() => {
                ("entry-only", Some(device.listing()))
            }
            MeterChoice::Entry(device) => {
                ("entry", Some(registry::named_under_pick(picked, device)))
            }
            MeterChoice::AutoDetect => ("auto-detect", None),
            MeterChoice::OwnModel => ("own-model", None),
        };
        if gui {
            let model = listing.map_or(Cow::Borrowed(""), |l| {
                self.0.label(TextKind::Model, l.display_name())
            });
            tr!(self.0, "choose-meter-gui", choice = key, model = model)
        } else {
            let id = listing.map_or("", |l| l.id());
            tr!(self.0, "choose-meter-cli", choice = key, id = id)
        }
        .into_owned()
    }

    /// The same in the GUI, where ticking the setting also clears a
    /// `--no-bluetooth` given at start.
    pub fn gui_bluetooth_off_hint(self) -> Cow<'a, str> {
        tr!(self.0, "bluetooth-off-gui")
    }

    /// The line that says what was not found.
    ///
    /// No full stop: the CLI ends the sentence, the GUI's notice titles
    /// carry none.
    pub fn not_found_title(self, links: &LinksSearched<'_>) -> String {
        match *links {
            LinksSearched::Usb => tr!(self.0, "not-found-usb").into_owned(),
            LinksSearched::UsbAndBluetooth => {
                tr!(self.0, "not-found-usb-or-bluetooth").into_owned()
            }
            LinksSearched::BluetoothAt(address) => {
                tr!(self.0, "not-found-bluetooth-at", address = address).into_owned()
            }
            LinksSearched::BluetoothOnly { model, .. } => {
                tr!(self.0, "not-found-bluetooth-only", model = model).into_owned()
            }
        }
    }

    /// The sections to print under that title, in the order they are
    /// printed.
    pub fn sections(self, links: &LinksSearched<'_>) -> Vec<SetupSection> {
        match *links {
            LinksSearched::Usb => vec![self.usb_section()],
            LinksSearched::UsbAndBluetooth => vec![
                self.usb_section(),
                self.bluetooth_section(vec![
                    Step::Prose(tr!(self.0, "setup-bluetooth-catalog")),
                    Step::Literal(CATALOG_URL),
                ]),
            ],
            // No cable was looked at, so nothing about cables applies — and
            // the address the user typed is the only thing that did not
            // answer.
            LinksSearched::BluetoothAt(_) => {
                vec![self.bluetooth_section(vec![Step::Prose(tr!(self.0, "setup-bluetooth-scan"))])]
            }
            // No adapter to wake, and the meter named: its own steps say how
            // to switch its radio on.
            LinksSearched::BluetoothOnly { activation, .. } => {
                let activation = self.0.label(TextKind::Activation, activation);
                let steps = activation
                    .lines()
                    .map(|l| l.to_string())
                    .chain(self.bluetooth_host_steps().into_iter().map(Step::line))
                    .collect();
                vec![SetupSection {
                    link: self.short_name(Link::Bluetooth).into_owned(),
                    check: tr!(self.0, "setup-bluetooth-check").into_owned(),
                    steps,
                }]
            }
        }
    }

    /// A section's label line, `<link>: <check>`. `link` is passed apart
    /// so the CLI can style it.
    pub fn section_label(self, link: &str, check: &str) -> String {
        tr!(self.0, "setup-label", link = link, check = check).into_owned()
    }

    /// A section as plain text, the steps indented under the label.
    pub fn section_text(self, section: &SetupSection) -> String {
        let mut out = self.section_label(&section.link, &section.check);
        for step in &section.steps {
            out.push_str("\n  ");
            out.push_str(step);
        }
        out
    }

    fn usb_section(self) -> SetupSection {
        SetupSection {
            link: self.short_name(Link::UsbCable).into_owned(),
            check: tr!(self.0, "setup-usb-check").into_owned(),
            steps: self.usb_steps().into_iter().map(Step::line).collect(),
        }
    }

    #[cfg(target_os = "linux")]
    fn usb_steps(self) -> Vec<Step<'a>> {
        vec![
            Step::Prose(tr!(self.0, "setup-usb-linux-rule")),
            Step::Literal("sudo cp udev/70-dmm-tools.rules /etc/udev/rules.d/"),
            Step::Literal("sudo udevadm control --reload-rules"),
            Step::Prose(tr!(self.0, "setup-usb-linux-replug")),
            Step::Literal(SETUP_GUIDE_URL),
        ]
    }

    #[cfg(target_os = "windows")]
    fn usb_steps(self) -> Vec<Step<'a>> {
        vec![
            Step::Prose(tr!(self.0, "setup-usb-windows-open")),
            Step::Prose(tr!(self.0, "setup-usb-windows-cp2110")),
            Step::Prose(tr!(self.0, "setup-usb-windows-input")),
            Step::Prose(tr!(self.0, "setup-usb-windows-warning")),
            Step::Literal(CP2110_DRIVER_URL),
            Step::Prose(tr!(self.0, "setup-usb-windows-nothing")),
        ]
    }

    #[cfg(target_os = "macos")]
    fn usb_steps(self) -> Vec<Step<'a>> {
        vec![
            Step::Prose(tr!(self.0, "setup-usb-macos-auto")),
            Step::Prose(tr!(self.0, "setup-usb-macos-privacy")),
        ]
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    fn usb_steps(self) -> Vec<Step<'a>> {
        Vec::new()
    }

    /// The radio's section: wake the adapter (it stops advertising in
    /// standby, and its own power switch wakes it), what this computer's
    /// radio may need, then `closing`.
    fn bluetooth_section(self, closing: Vec<Step<'a>>) -> SetupSection {
        let steps = std::iter::once(Step::Prose(tr!(self.0, "setup-bluetooth-adapter")))
            .chain(self.bluetooth_host_steps())
            .chain(closing)
            .map(Step::line)
            .collect();
        SetupSection {
            link: self.short_name(Link::Bluetooth).into_owned(),
            check: tr!(self.0, "setup-bluetooth-check").into_owned(),
            steps,
        }
    }

    /// What this computer's radio may need, adapter or not.
    #[cfg(target_os = "macos")]
    fn bluetooth_host_steps(self) -> Vec<Step<'a>> {
        vec![Step::Prose(tr!(self.0, "setup-bluetooth-macos-privacy"))]
    }

    #[cfg(not(target_os = "macos"))]
    fn bluetooth_host_steps(self) -> Vec<Step<'a>> {
        Vec::new()
    }

    /// The title for a link where detection ran and no meter answered it.
    ///
    /// No full stop, like [`Help::not_found_title`]. Not the same claim as
    /// [`Help::no_response`]: here nothing was identified, there the meter
    /// in use has gone quiet.
    pub fn no_meter_answered(self) -> Cow<'a, str> {
        tr!(self.0, "no-meter-answered")
    }

    /// The title for a meter that was picked or found and is not answering.
    pub fn no_response(self) -> Cow<'a, str> {
        tr!(self.0, "no-response")
    }

    /// Why a lost link can't be reopened; each binary follows it with what
    /// to do about it.
    pub fn several_cables(self, several: SeveralCables) -> Cow<'a, str> {
        tr!(self.0, "several-cables", count = several.count)
    }

    /// Why a reopen is still waiting with another cable plugged in; each
    /// binary follows it with how to switch to that cable.
    pub fn waiting_for_own_cable(self) -> Cow<'a, str> {
        tr!(self.0, "waiting-for-own-cable")
    }

    /// What both binaries say under a reading the meter shows on its own
    /// screen only (`Measurement::screen_only`).
    pub fn screen_only_reading(self) -> Cow<'a, str> {
        tr!(self.0, "screen-only-reading")
    }

    /// The sentence both binaries use to say a protocol is not fully
    /// verified.
    ///
    /// Only the claim is shared: each site appends its own call to action
    /// (the CLI a `capture` command, the GUI a link), because they differ in
    /// what the user can do next.
    pub fn stability_warning(self, model_name: &str, stability: Stability) -> String {
        let stability = match stability {
            Stability::Verified => "verified",
            Stability::PartlyVerified => "partly-verified",
            Stability::Experimental => "experimental",
        };
        tr!(
            self.0,
            "stability-warning",
            model = model_name,
            stability = stability
        )
        .into_owned()
    }

    /// The lines both binaries print for what an `--adapter` selector could
    /// have matched, header included.
    ///
    /// No styling and no trailing hint — the CLI tells the user to re-run
    /// with a different `--adapter`, the GUI to restart with one, and only
    /// the list of what is plugged in is common to both.
    pub fn adapter_lines(self, adapters: &ConnectedAdapters) -> Vec<String> {
        match adapters {
            ConnectedAdapters::None => vec![tr!(self.0, "adapters-none").into_owned()],
            ConnectedAdapters::Listed(devices) => {
                let mut lines = Vec::with_capacity(devices.len() + 1);
                lines.push(tr!(self.0, "adapters-heading").into_owned());
                lines.extend(devices.iter().cloned());
                lines
            }
            ConnectedAdapters::Unavailable => {
                vec![tr!(self.0, "adapters-unavailable").into_owned()]
            }
        }
    }

    /// The line both binaries add when an `--adapter` value nothing matched
    /// is twelve hex digits — a Bluetooth address without its colons, the
    /// way Windows' Device Manager shows one.
    ///
    /// Only a hint: a USB cable's serial number can be the same twelve
    /// digits, so the value is not opened as an address. `None` for any
    /// other value, and in a build without the radio.
    pub fn colonless_address_hint(self, selector: &str) -> Option<String> {
        if !dmm_lib::BLUETOOTH_SUPPORTED
            || selector.len() != 12
            || !selector.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return None;
        }
        let pairs: Vec<String> = selector
            .as_bytes()
            .chunks(2)
            .map(|pair| String::from_utf8_lossy(pair).to_ascii_uppercase())
            .collect();
        Some(tr!(self.0, "colonless-address-hint", address = pairs.join(":")).into_owned())
    }
}

/// `--mock-mode`'s parser in both binaries: what `MockMode`'s own parser
/// takes (a label or an alias, in any letter case), and anything else
/// refused as clap refuses a value outside its list, which
/// [`arg_errors`](crate::l10n::arg_errors) words in the user's language.
/// `MockMode`'s own refusal is the library's English. It offers clap no
/// possible values: `--help` lists the modes itself
/// ([`Help::mock_mode_help`]).
#[derive(Clone, Copy, Debug)]
pub struct MockModeParser;

impl clap::builder::TypedValueParser for MockModeParser {
    type Value = MockMode;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<MockMode, clap::Error> {
        if let Some(mode) = value.to_str().and_then(|s| s.parse().ok()) {
            return Ok(mode);
        }
        // A label is lower case, so the parse above took any that matches:
        // this only words the refusal.
        let labels = MockMode::ALL.iter().map(|m| m.label());
        clap::builder::PossibleValuesParser::new(labels)
            .try_map(|label| label.parse::<MockMode>())
            .parse_ref(cmd, arg, value)
    }
}

/// `cmd`, with the `--device` and `--mock-mode` flags on it and its
/// subcommands offering their values, for a completion script.
///
/// A bad `--device` gets our own error, and clap's possible values would
/// replace it with its own; `--mock-mode`'s would add a second list of the
/// modes to `--help`. So they are added only to the command a script is
/// generated from, never to the one that parses. Aliases are left out:
/// the ids cover every device, and `ut61e+` would need quoting. In English:
/// a completion script outlives the language it was generated in.
pub fn with_completion_values(mut cmd: clap::Command) -> clap::Command {
    use clap::builder::{PossibleValue, PossibleValuesParser};

    let subcommands: Vec<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    for name in subcommands {
        cmd = cmd.mut_subcommand(name, with_completion_values);
    }
    let auto = english().auto_description().into_owned();
    let devices = std::iter::once(PossibleValue::new(registry::AUTO_DEVICE_ID).help(auto))
        .chain(registry::listings().map(|l| PossibleValue::new(l.id()).help(l.display_name())));
    let modes = MockMode::ALL
        .iter()
        .map(|m| PossibleValue::new(m.label()).help(m.description()));
    let values: [(&str, Vec<PossibleValue>); 2] = [
        ("device", devices.collect()),
        ("mock_mode", modes.collect()),
    ];
    for (id, values) in values {
        if cmd.get_arguments().any(|a| a.get_id() == id) {
            cmd = cmd.mut_arg(id, |a| a.value_parser(PossibleValuesParser::new(values)));
        }
    }
    cmd
}

/// The meters on a bridge, grouped by the steps that switch their
/// transmission on.
///
/// Several entries share one instruction block — every UT61+/UT161 model, the
/// UT8802 and the UT8803 — so a list per device would print the same four
/// lines six times over. Registry order is kept, and a group is named by the
/// meters that share it, the brand once per run of its models: "UNI-T UT61E+,
/// UT61B+".
pub fn activation_groups(
    devices: &[&'static SelectableDevice],
) -> Vec<(&'static str, Vec<&'static str>)> {
    let mut groups: Vec<(&'static str, Vec<&'static SelectableDevice>)> = Vec::new();
    for &device in devices {
        match groups
            .iter_mut()
            .find(|(instructions, _)| *instructions == device.activation_instructions)
        {
            Some((_, members)) => members.push(device),
            None => groups.push((device.activation_instructions, vec![device])),
        }
    }
    groups
        .into_iter()
        .map(|(instructions, members)| {
            let names = members
                .iter()
                .enumerate()
                .map(|(i, d)| match i.checked_sub(1).map(|p| members[p].brand) {
                    Some(brand) if brand == d.brand => d.model,
                    _ => d.display_name(),
                })
                .collect();
            (instructions, names)
        })
        .collect()
}

/// What a device listing tags a device with, worked out once: finding a
/// meter's level builds its protocol, too much to do per frame for a list
/// of every meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTag {
    /// A simulated meter.
    NoHardware,
    /// A real meter, and how far its support is verified.
    Support(Stability),
}

impl DeviceTag {
    pub fn of(device: &SelectableDevice) -> Self {
        if device.requires_hardware {
            Self::Support((device.new_protocol)().profile().stability)
        } else {
            Self::NoHardware
        }
    }
}

/// One step of a setup section: prose to translate, or a command line or
/// address that stays as written.
enum Step<'a> {
    Prose(Cow<'a, str>),
    Literal(&'static str),
}

impl Step<'_> {
    /// The step as [`SetupSection::steps`] holds it: a command or address
    /// starts with a space, which tells the renderers to set it apart.
    fn line(self) -> String {
        match self {
            Step::Prose(text) => text.into_owned(),
            Step::Literal(text) => format!("  {text}"),
        }
    }
}

/// One link's worth of "nothing found" help: a label line, then its steps.
///
/// Keyed by the link the user knows it by, so the two links never interleave
/// and a third (LAN, for the bench meters) is another section rather than
/// more lines in this one. The CLI prints `link` bold and dims the steps that
/// are commands or URLs; the GUI draws the label line bold above them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupSection {
    /// The link, as the user names it: "USB cable", "Bluetooth".
    pub link: String,
    /// The rest of the label line ([`Help::section_label`]).
    pub check: String,
    /// One step per line, which the renderer indents. A step that starts with
    /// a space of its own is a command to run or a URL to open.
    pub steps: Vec<String>,
}

/// Which links a failed open looked at — what the help is allowed to talk
/// about, and what its title says was not found.
///
/// Narrowed by what the user asked for: a meter that only ships with a cable,
/// or Bluetooth switched off, gets the cable section alone, and an address on
/// `--adapter` gets the radio alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinksSearched<'a> {
    /// The USB cables alone.
    Usb,
    /// Both links, which is what an unnamed meter gets.
    UsbAndBluetooth,
    /// One Bluetooth adapter, named by address or peripheral id.
    BluetoothAt(&'a str),
    /// The radio alone, for a meter with Bluetooth built in: its name as
    /// the help shows it, and the registry's steps that switch its radio
    /// on, in English: [`Help::sections`] looks them up.
    BluetoothOnly {
        model: &'a str,
        activation: &'static str,
    },
}

impl LinksSearched<'_> {
    /// What a failed open searched, from whether it reached the radio.
    pub fn from_usb_failure(bluetooth_searched: bool) -> Self {
        if bluetooth_searched {
            Self::UsbAndBluetooth
        } else {
            Self::Usb
        }
    }
}

/// What the bus held when an `--adapter` selector matched nothing.
///
/// Both binaries print the same lines ([`Help::adapter_lines`]), and attach
/// their "pick another adapter" hint on [`Self::offers_choice`]. Naming the
/// three answers keeps that decision off a line count.
#[derive(Debug)]
pub enum ConnectedAdapters {
    /// Nothing is plugged in.
    None,
    /// One `  [i] dev` line per device, without the header.
    Listed(Vec<String>),
    /// HID enumeration itself failed, so there is nothing to list.
    Unavailable,
}

impl ConnectedAdapters {
    /// Whether there is a list to pick another adapter from. "Nothing is
    /// connected" and "couldn't look" are complete on their own.
    pub fn offers_choice(&self) -> bool {
        matches!(self, Self::Listed(_))
    }
}

/// What is on the bus, for the "adapter not found" error both binaries print.
///
/// Walks every HID device on the system, so call it once when the error
/// arrives rather than from a render path. The answer is a snapshot either
/// way: acting on it means restarting with a different selector.
pub fn connected_adapters() -> ConnectedAdapters {
    match list_devices() {
        Ok(devices) if devices.is_empty() => ConnectedAdapters::None,
        Ok(devices) => ConnectedAdapters::Listed(
            devices
                .iter()
                .enumerate()
                .map(|(i, dev)| format!("  [{i}] {dev}"))
                .collect(),
        ),
        // The HID API itself failed, so there is nothing to list; `list` says
        // the same thing with the setup help attached.
        Err(_) => ConnectedAdapters::Unavailable,
    }
}

/// Which help a failed open or command gets: the one classification of a
/// [`Error`] both binaries word, each in its own layout — the CLI as lines
/// under its `Error:`, the GUI as a notice in the reading column.
///
/// Data only, so a notice can be worded again in another language: a
/// renderer takes the error's own words (what the Bluetooth stack said, the
/// selector that matched nothing) from the error beside it, and the rest
/// from [`Help`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnosis {
    /// Nothing answered on the links the open searched.
    NotFound { bluetooth_searched: bool },
    /// `--adapter` named a Bluetooth address and nothing answered at it.
    NothingAtAddress { address: String },
    /// A meter with the radio built in, and nothing in range carried its
    /// name: the name it was picked by, and its steps that switch the radio
    /// on.
    BluetoothOnlyNotFound {
        model: &'static str,
        activation: &'static str,
    },
    /// `--adapter` named a USB device that nothing matched, or a path that
    /// is not one: the error says which ([`Localizer::adapter_named`]). What
    /// the bus holds is for the caller to list ([`connected_adapters`]): it
    /// walks every HID device.
    AdapterNotFound,
    /// The cable is there and nothing on it answered the detection probe.
    NotIdentified {
        bridge: &'static str,
        built_in_radio: bool,
    },
    /// The stack could not reach the Bluetooth adapter `--adapter` named —
    /// most often one paired but asleep.
    AdapterUnreachable { address: String },
    /// The stack could not search for the picked meter with the radio
    /// built in: radio off, permission denied.
    BluetoothOnlyUnreachable {
        model: &'static str,
        activation: &'static str,
    },
    /// A meter with the radio built in, and the radio was not searched: the
    /// name it was picked by, and why ([`Localizer::bluetooth_only`]).
    /// Never [`BluetoothOnlyMiss::NotInRange`].
    BluetoothNotSearched {
        model: &'static str,
        miss: BluetoothOnlyMiss,
    },
    /// The meter on the link is another than the one chosen: what to pick
    /// instead.
    OtherMeter(MeterChoice),
    /// The meter stopped answering.
    NoResponse,
    /// The meter answered with data that could not be read.
    BadData,
    /// Nothing to add to the error's own words.
    Other,
}

/// What to pick for a meter that is another than the one chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterChoice {
    /// The entry the meter names.
    Entry(&'static SelectableDevice),
    /// Detection finds it.
    AutoDetect,
    /// Nothing says which model it is: the user has to pick it.
    OwnModel,
}

impl MeterChoice {
    /// The entry registry id `id` names, which an error carries.
    fn entry(id: &str) -> Option<Self> {
        registry::find_device(id).map(Self::Entry)
    }

    /// What a refused command's `reason` says to pick, when it is the
    /// meter being another model.
    pub fn for_rejected(reason: &Rejected) -> Option<Self> {
        match reason {
            Rejected::OtherModel { id, .. } => Self::entry(id),
            _ => None,
        }
    }

    /// What `reason` says to pick, when it says anything.
    fn for_wrong_meter(reason: &WrongMeter) -> Option<Self> {
        match reason {
            WrongMeter::OtherFrame { id, .. } | WrongMeter::OtherSeries { id, .. } => {
                Self::entry(id)
            }
            WrongMeter::OtherFrameKind { remedy, .. } => Some(match remedy {
                FrameRemedy::AutoDetect => Self::AutoDetect,
                FrameRemedy::OwnModel => Self::OwnModel,
            }),
            WrongMeter::NoFifteenByteFrames => Some(Self::AutoDetect),
            WrongMeter::UnreadModel { .. }
            | WrongMeter::NoSixByteFrames
            | WrongMeter::UntoldFrameKind => None,
        }
    }
}

impl Diagnosis {
    /// Classify `err`, for a session given `adapter` on `--adapter` and with
    /// `picked` the meter the user chose (`None` under Auto-detect).
    ///
    /// Matched by variant rather than by [`ErrorKind`]: the help needs what
    /// each one carries, and the coarse kind carries none of it. Only
    /// unreadable data, which needs no more than its text, goes by kind.
    pub fn of(err: &Error, adapter: Option<&str>, picked: Option<registry::Listing>) -> Self {
        // A meter with the radio built in, by the name it was picked by when
        // the pick is that entry; the error names the maker's.
        let named = |model: &'static str| {
            picked
                .filter(|l| l.device.display_name() == model)
                .map_or(model, |l| l.display_name())
        };
        match err {
            // An address is explained by the link it named, not by the USB
            // bus: nothing on the bus could have answered it.
            Error::AdapterNotFound(selector) if dmm_lib::is_bluetooth_selector(selector) => {
                Self::NothingAtAddress {
                    address: selector.clone(),
                }
            }
            Error::AdapterNotFound(_) | Error::NotAnAdapter(_) => Self::AdapterNotFound,
            Error::NoTransportFound {
                bluetooth_searched, ..
            } => Self::NotFound {
                bluetooth_searched: *bluetooth_searched,
            },
            // A `Timeout` kind too, but no quiet meter: no meter was found.
            Error::DeviceNotIdentified {
                bridge,
                built_in_radio,
            } => Self::NotIdentified {
                bridge,
                built_in_radio: *built_in_radio,
            },
            Error::Bluetooth(_) => {
                if let Some(address) = adapter.filter(|a| dmm_lib::is_bluetooth_selector(a)) {
                    Self::AdapterUnreachable {
                        address: address.to_string(),
                    }
                } else if let Some(picked) = picked.filter(|l| l.device.bluetooth_only()) {
                    Self::BluetoothOnlyUnreachable {
                        model: picked.display_name(),
                        activation: picked.device.activation_instructions,
                    }
                } else {
                    Self::Other
                }
            }
            Error::BluetoothOnly {
                model,
                activation,
                miss: BluetoothOnlyMiss::NotInRange,
            } => Self::BluetoothOnlyNotFound {
                model: named(model),
                activation,
            },
            Error::BluetoothOnly { model, miss, .. } => Self::BluetoothNotSearched {
                model: named(model),
                miss: *miss,
            },
            // Unreadable as the entry chosen, but saying which to choose.
            Error::WrongMeter { reason, .. } => {
                MeterChoice::for_wrong_meter(reason).map_or(Self::BadData, Self::OtherMeter)
            }
            Error::CommandRejected(reason)
                if let Some(choice) = MeterChoice::for_rejected(reason) =>
            {
                Self::OtherMeter(choice)
            }
            // The variant, not the kind: a pulled cable's HID error names a
            // timeout too, and detection finding nothing is that kind.
            Error::Timeout => Self::NoResponse,
            _ if err.kind() == ErrorKind::Protocol => Self::BadData,
            _ => Self::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::error::BluetoothIssue;

    /// Every message this crate looks up exists in en-US with the variables
    /// it is given, and every message in its hand-written files is used.
    /// The generated `lib.ftl` is looked up by text, not by `tr!`.
    #[test]
    fn the_shared_messages_match_their_uses() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let resources = [RESOURCES, crate::l10n::LIB_RESOURCES].concat();
        let owned = [
            "shared.ftl",
            "lib-names.ftl",
            "lib-errors.ftl",
            "lib-alarm.ftl",
        ];
        let problems = crate::l10n::check::check(&src, &resources, &owned);
        assert!(problems.is_empty(), "{problems:#?}");
    }

    #[test]
    fn release_version_has_no_hash() {
        assert_eq!(version_string("0.5.0", "abc1234"), "0.5.0");
        assert_eq!(version_label("0.5.0", "abc1234"), "v0.5.0");
    }

    #[test]
    fn dev_version_carries_the_hash() {
        assert_eq!(
            version_string("0.6.0-dev", "abc1234"),
            "0.6.0-dev (abc1234)"
        );
        assert_eq!(
            version_label("0.6.0-dev", "abc1234"),
            "v0.6.0-dev (abc1234)"
        );
    }

    /// Both flags get their values wherever they sit, a subcommand included,
    /// as `dmm-cli read --mock-mode` does.
    #[test]
    fn completion_values_reach_the_flags_on_subcommands() {
        let cmd = clap::Command::new("t")
            .arg(clap::Arg::new("device").long("device"))
            .subcommand(
                clap::Command::new("read").arg(clap::Arg::new("mock_mode").long("mock-mode")),
            );
        let mut cmd = with_completion_values(cmd);
        let values = |cmd: &clap::Command, id: &str| -> Vec<String> {
            let arg = cmd.get_arguments().find(|a| a.get_id() == id).unwrap();
            arg.get_possible_values()
                .iter()
                .map(|v| v.get_name().to_string())
                .collect()
        };
        let devices = values(&cmd, "device");
        assert_eq!(devices[0], registry::AUTO_DEVICE_ID);
        assert_eq!(devices.len(), registry::listings().count() + 1);
        let read = cmd.find_subcommand_mut("read").unwrap();
        assert_eq!(values(read, "mock_mode").len(), MockMode::ALL.len());
    }

    /// `--mock-mode` takes what `MockMode`'s parser takes, and refuses the
    /// rest as a value outside clap's list, which `arg_errors` words: no
    /// English reason from the library, and no list in `--help`.
    #[test]
    fn mock_mode_parser_takes_aliases_and_words_a_refusal() {
        let cmd = || {
            clap::Command::new("t").arg(
                clap::Arg::new("mock_mode")
                    .long("mock-mode")
                    .value_parser(MockModeParser),
            )
        };
        let parse = |value: &str| {
            cmd()
                .try_get_matches_from(["t", "--mock-mode", value])
                .map(|m| *m.get_one::<MockMode>("mock_mode").unwrap())
        };
        for value in ["dcv", "DCV", "Dc-V", "ohms", "OVERLOAD", "acv-hz"] {
            assert_eq!(parse(value).ok(), value.parse::<MockMode>().ok(), "{value}");
            assert!(parse(value).is_ok(), "{value}");
        }
        let e = parse("volts").unwrap_err();
        assert_eq!(e.kind(), clap::error::ErrorKind::InvalidValue);
        let rendered = crate::l10n::arg_errors::render(&e, english().0, &[]);
        let rendered = rendered.to_string();
        assert!(
            rendered.starts_with(&format!(
                "error: invalid value 'volts' for '--mock-mode <mock_mode>'\n  \
                 [possible values: {}]\n",
                MockMode::label_list()
            )),
            "{rendered}"
        );
        assert!(!rendered.contains("unknown mock mode"), "{rendered}");
        assert!(
            cmd()
                .get_arguments()
                .all(|a| a.get_possible_values().is_empty())
        );
    }

    #[test]
    fn device_help_lists_every_registry_device() {
        let help = english().device_help("Device to connect to.");
        assert!(help.starts_with("Device to connect to.\n\nDevices:\n"));
        // Auto is not a registry entry, so nothing below would list it — and
        // it leads, being what a user who names nothing gets.
        let listing = help.split("Devices:\n").nth(1).expect("device list");
        assert!(
            listing.starts_with("  auto          Detect the connected meter (default)\n"),
            "{listing}"
        );
        for row in registry::listings() {
            assert!(help.contains(row.id()), "missing {}", row.id());
            assert!(
                help.contains(row.model_name()),
                "missing {}",
                row.display_name()
            );
        }
        assert!(help.contains("Also accepts aliases"));
    }

    /// Each device sits under its brand's heading, by its model name: the
    /// heading carries the brand the name would otherwise repeat.
    #[test]
    fn device_help_groups_devices_under_their_brand() {
        let help = english().device_help("x");
        let mut brand = None;
        let mut seen = 0;
        for line in help.split("Devices:\n").nth(1).unwrap().lines() {
            if let Some(heading) = line.strip_prefix("  ").filter(|l| !l.starts_with(' ')) {
                brand = registry::brand_groups()
                    .into_iter()
                    .find(|(b, _)| b.name() == heading);
                continue;
            }
            let Some(entry) = line.strip_prefix("    ") else {
                continue;
            };
            let (_, rows) = brand.as_ref().expect("an entry before any heading");
            let id = entry.split_whitespace().next().unwrap();
            let row = rows.iter().find(|l| l.id() == id).expect(line);
            assert!(entry.contains(row.model_name()), "{line}");
            seen += 1;
        }
        assert_eq!(seen, registry::listings().count());
        assert!(
            help.contains("\n  Voltcraft\n    vc650bt       VC650BT"),
            "{help}"
        );
        assert!(help.contains("\n  UNI-T\n"), "{help}");
        assert!(help.contains("    ut217a        UT217A"), "{help}");
    }

    #[test]
    fn only_a_list_offers_a_choice_of_adapter() {
        assert!(ConnectedAdapters::Listed(vec!["  [0] one".into()]).offers_choice());
        assert!(!ConnectedAdapters::None.offers_choice());
        assert!(!ConnectedAdapters::Unavailable.offers_choice());
    }

    /// Both binaries render these lines verbatim, so the header and the empty
    /// and unavailable wordings are part of their output, not an internal
    /// detail.
    #[test]
    fn connected_adapter_lines_match_each_outcome() {
        let lines = |a: ConnectedAdapters| english().adapter_lines(&a);
        assert_eq!(
            lines(ConnectedAdapters::None),
            ["No devices currently connected."]
        );
        assert_eq!(
            lines(ConnectedAdapters::Listed(vec![
                "  [0] one".into(),
                "  [1] two".into()
            ])),
            ["Connected devices:", "  [0] one", "  [1] two"]
        );
        assert_eq!(
            lines(ConnectedAdapters::Unavailable),
            ["Run 'dmm-cli list' to see connected devices."]
        );
    }

    /// A Bluetooth address typed the way Device Manager shows it gets the
    /// colon form back; anything else that is not twelve hex digits gets
    /// nothing. A build without the radio gets nothing either.
    #[test]
    fn a_colonless_address_is_shown_its_colon_form() {
        if !dmm_lib::BLUETOOTH_SUPPORTED {
            assert!(english().colonless_address_hint("123456789abc").is_none());
            return;
        }
        assert_eq!(
            english().colonless_address_hint("123456789abc").as_deref(),
            Some("If that is a Bluetooth address, write it 12:34:56:78:9A:BC.")
        );
        assert!(
            english()
                .colonless_address_hint("12:34:56:78:9A:BC")
                .is_none()
        );
        assert!(english().colonless_address_hint("123456789AB").is_none());
        assert!(english().colonless_address_hint("123456789ABCD").is_none());
        assert!(english().colonless_address_hint("12345678ZABC").is_none());
        assert!(english().colonless_address_hint("/dev/hidraw0").is_none());
        // Twelve bytes but not hex: no pair may split a character.
        assert!(english().colonless_address_hint("éééééé").is_none());
    }

    /// The CLI's copy of this list went four modes stale; rendering it from
    /// the table is only worth it if every mode really reaches the help.
    #[test]
    fn mock_mode_help_lists_every_mode() {
        let help = english().mock_mode_help("Pin the mock.", "--mock-mode dcv");
        assert!(help.starts_with("Pin the mock.\n\nModes: "));
        for mode in MockMode::ALL {
            assert!(help.contains(mode.label()), "missing {}", mode.label());
        }
        assert!(help.ends_with("Example: --mock-mode dcv"));
    }

    /// Both binaries render these lines verbatim: the CLI dims the indented
    /// ones, the GUI joins them with newlines, so a blank line would show up
    /// as a gap in the middle of a section.
    #[test]
    fn every_section_is_printable_on_every_platform() {
        for links in [
            LinksSearched::Usb,
            LinksSearched::UsbAndBluetooth,
            LinksSearched::BluetoothAt("12:34:56:78:9A:BC"),
            ut60bt(),
        ] {
            for section in english().sections(&links) {
                assert!(!section.link.is_empty());
                assert!(section.check.ends_with('.'), "{}", section.check);
                assert!(section.steps.iter().all(|line| !line.trim().is_empty()));
            }
        }
    }

    /// The help covers the links the open path actually tried, and no others:
    /// a cable-only meter, a build without the feature and a switched-off
    /// radio all reach the cable section alone.
    #[test]
    fn the_sections_are_the_links_that_were_searched() {
        let links = |l: LinksSearched| -> Vec<String> {
            english().sections(&l).into_iter().map(|s| s.link).collect()
        };
        assert_eq!(links(LinksSearched::Usb), ["USB cable"]);
        assert_eq!(
            links(LinksSearched::UsbAndBluetooth),
            ["USB cable", "Bluetooth"]
        );
        assert_eq!(links(LinksSearched::BluetoothAt("4C:3C")), ["Bluetooth"]);
        assert_eq!(links(ut60bt()), ["Bluetooth"]);
    }

    /// A meter with the radio built in, as the binaries build its help from
    /// its registry entry.
    fn ut60bt() -> LinksSearched<'static> {
        let device = registry::find_device("ut60bt").expect("registry entry");
        LinksSearched::BluetoothOnly {
            model: device.display_name(),
            activation: device.activation_instructions,
        }
    }

    /// A meter with the radio built in is sent to its own switch, not to an
    /// adapter's, and not to the catalog: the user already named it.
    #[test]
    fn a_bluetooth_only_meter_gets_its_own_steps() {
        let links = ut60bt();
        assert_eq!(
            english().not_found_title(&links),
            "No UNI-T UT60BT found over Bluetooth"
        );
        let text = english().section_text(&english().sections(&links)[0]);
        assert!(
            text.starts_with(
                "Bluetooth: turn on Bluetooth on this computer and data transmission on the meter.\n  \
                 1. Turn the meter on\n  2. Long press SEL"
            ),
            "{text}"
        );
        assert!(!text.contains("adapter"), "{text}");
        assert!(!text.contains("supported-devices.md"), "{text}");
    }

    /// The titles are the user's first line of both binaries' help, and the
    /// GUI's notice title on top of that.
    #[test]
    fn the_title_names_what_was_looked_for() {
        assert_eq!(
            english().not_found_title(&LinksSearched::Usb),
            "No USB cable found"
        );
        assert_eq!(
            english().not_found_title(&LinksSearched::UsbAndBluetooth),
            "No meter found over USB or Bluetooth"
        );
        assert_eq!(
            english().not_found_title(&LinksSearched::BluetoothAt("12:34:56:78:9A:BC")),
            "No Bluetooth device found at 12:34:56:78:9A:BC"
        );
        assert_eq!(
            LinksSearched::from_usb_failure(true),
            LinksSearched::UsbAndBluetooth
        );
        assert_eq!(LinksSearched::from_usb_failure(false), LinksSearched::Usb);
    }

    /// An address nobody answered is answered with the scan, not with the
    /// catalog: the user already knows which adapter they meant.
    #[test]
    fn a_named_address_is_sent_to_the_scan() {
        let selected = english().sections(&LinksSearched::BluetoothAt("4C:3C"));
        let text = english().section_text(&selected[0]);
        assert!(text.contains("dmm-cli list"), "{text}");
        assert!(!text.contains("supported-devices.md"), "{text}");

        let unnamed = english().sections(&LinksSearched::UsbAndBluetooth);
        let text = english().section_text(&unnamed[1]);
        assert!(text.contains("supported-devices.md"), "{text}");
        assert!(!text.contains("dmm-cli list"), "{text}");
    }

    /// No product name in either binary: which adapters work is the catalog's
    /// business, and a message that names one dates the moment a second is
    /// supported.
    #[test]
    fn the_sections_name_no_product() {
        for links in [
            LinksSearched::Usb,
            LinksSearched::UsbAndBluetooth,
            LinksSearched::BluetoothAt("4C:3C"),
            ut60bt(),
        ] {
            for section in english().sections(&links) {
                let text = english().section_text(&section);
                assert!(!text.contains("UT-D07"), "{text}");
            }
        }
    }

    /// Each binary names its own way to turn the search back on, and the
    /// GUI's checkbox by the label it draws.
    #[test]
    fn each_binary_names_its_own_bluetooth_switch() {
        assert_eq!(
            english().cli_bluetooth_off_hint(true),
            "Leave out --no-bluetooth."
        );
        assert_eq!(
            english().cli_bluetooth_off_hint(false),
            "Tick \"Look for Bluetooth devices\" in dmm-gui's settings, which dmm-cli reads too."
        );
        assert_eq!(
            english().gui_bluetooth_off_hint(),
            "Tick \"Look for Bluetooth devices\" in Settings (\u{2699})."
        );
    }

    /// The short form drops the word the status line no longer needs; the
    /// full form is the one the errors use.
    #[test]
    fn the_two_forms_of_a_link_name() {
        assert_eq!(english().short_name(Link::Bluetooth), "Bluetooth");
        assert_eq!(Link::Bluetooth.full_name(false), "Bluetooth adapter");
        assert_eq!(english().short_name(Link::UsbCable), "USB cable");
        assert_eq!(Link::UsbCable.full_name(false), "USB cable");
    }

    /// The "no meter answered" help lists what to switch on, once per set of
    /// steps: every UT61+/UT161 model shares one block, and so do the UT8802
    /// and the UT8803.
    #[test]
    fn activation_help_lists_each_set_of_steps_once() {
        let devices = dmm_lib::devices_on_bridge("CP2110");
        assert!(devices.len() > 1, "CP2110 carries several meters");
        let groups = activation_groups(&devices);
        assert!(groups.len() < devices.len(), "nothing was grouped");

        let instructions: Vec<&str> = groups.iter().map(|(i, _)| *i).collect();
        let mut unique = instructions.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), instructions.len(), "a block is listed twice");

        // Every meter is named exactly once, under its own block.
        let named: Vec<&str> = groups.iter().flat_map(|(_, names)| names.clone()).collect();
        assert_eq!(named.len(), devices.len());
        let ut61 = groups
            .iter()
            .find(|(_, names)| names.contains(&"UNI-T UT61E+"))
            .expect("the UT61E+ is on the CP2110");
        assert!(ut61.1.contains(&"UT61B+"), "{:?}", ut61.1);
        assert!(ut61.0.contains("USB/Hz"), "{}", ut61.0);
    }

    #[test]
    fn stability_warning_names_the_model_and_the_level() {
        assert_eq!(
            english().stability_warning("UNI-T UT8803", Stability::Experimental),
            "UNI-T UT8803 support is experimental (unverified against real hardware)."
        );
        assert_eq!(
            english().stability_warning("UNI-T UT181A", Stability::PartlyVerified),
            "UNI-T UT181A support is partly verified (some modes and commands unconfirmed on real hardware)."
        );
    }

    /// The mock needs no hardware and the families short of verified carry
    /// their level; the tags tell the user what to expect before they plug
    /// anything in.
    #[test]
    fn device_help_tags_mock_and_unverified_devices() {
        let help = english().device_help("x");
        let mock_line = help
            .lines()
            .find(|l| l.trim_start().starts_with("mock"))
            .expect("mock listed");
        assert!(mock_line.contains("(no hardware required)"), "{mock_line}");
        assert!(help.contains("(experimental)"));
        assert!(help.contains("(partly verified)"));
    }

    /// The row `id` names, as Settings or `--device` would pick it.
    fn pick(id: &str) -> Option<registry::Listing> {
        Some(registry::resolve_listing(id).expect("a registry row"))
    }

    const ADDRESS: &str = "12:34:56:78:9A:BC";

    /// Each failure a user can act on gets its own help, and the selector
    /// decides which link explains an adapter that matched nothing.
    #[test]
    fn each_open_failure_gets_its_own_diagnosis() {
        let of = |e: &Error| Diagnosis::of(e, None, None);
        assert_eq!(
            of(&Error::NoTransportFound {
                cables: vec![],
                bluetooth_searched: true
            }),
            Diagnosis::NotFound {
                bluetooth_searched: true
            }
        );
        assert_eq!(
            of(&Error::AdapterNotFound(ADDRESS.into())),
            Diagnosis::NothingAtAddress {
                address: ADDRESS.into()
            }
        );
        assert_eq!(
            of(&Error::AdapterNotFound("00C5B27A".into())),
            Diagnosis::AdapterNotFound
        );
        // A path is never an address, whatever it reads like.
        assert_eq!(
            of(&Error::NotAnAdapter("/dev/hidraw3".into())),
            Diagnosis::AdapterNotFound
        );
        assert_eq!(
            of(&Error::DeviceNotIdentified {
                bridge: "CP2110",
                built_in_radio: false
            }),
            Diagnosis::NotIdentified {
                bridge: "CP2110",
                built_in_radio: false
            }
        );
        assert_eq!(of(&Error::Timeout), Diagnosis::NoResponse);
        assert_eq!(
            of(&Error::InvalidResponse {
                message: "bad frame".into(),
                raw: vec![]
            }),
            Diagnosis::BadData
        );
        assert_eq!(
            of(&Error::NoTransportFound {
                cables: vec![],
                bluetooth_searched: false
            }),
            Diagnosis::NotFound {
                bluetooth_searched: false
            }
        );
        assert_eq!(of(&Error::LinkLost), Diagnosis::Other);
        // A read a signal cut short is the cable's error, not bad data.
        assert_eq!(of(&Error::interrupted()), Diagnosis::Other);
        // An address on `--adapter` explains only what the stack said.
        assert_eq!(
            Diagnosis::of(&Error::LinkLost, Some(ADDRESS), None),
            Diagnosis::Other
        );
    }

    /// Only the timeout variant is a quiet meter: a pulled cable's HID error
    /// names a timeout in its text, and must keep its own words.
    #[test]
    fn a_pulled_cable_is_not_a_quiet_meter() {
        let pulled = Error::cable_pulled();
        assert!(pulled.to_string().contains("timeout"), "{pulled}");
        assert_eq!(Diagnosis::of(&pulled, None, None), Diagnosis::Other);
    }

    /// A stack failure is explained by a Bluetooth address on `--adapter`
    /// first, then by a picked meter with the radio built in; a meter behind
    /// an adapter, or a USB selector, keeps the bare error.
    #[test]
    fn a_stack_failure_is_explained_by_what_it_was_reaching() {
        let stack = Error::Bluetooth(BluetoothIssue::Stack("Timed out after 10s".into()));
        assert_eq!(
            Diagnosis::of(&stack, Some(ADDRESS), pick("ut60bt")),
            Diagnosis::AdapterUnreachable {
                address: ADDRESS.into()
            }
        );
        let ut60bt = pick("ut60bt").expect("a row");
        assert_eq!(
            Diagnosis::of(&stack, Some("00C5B27A"), pick("ut60bt")),
            Diagnosis::BluetoothOnlyUnreachable {
                model: ut60bt.display_name(),
                activation: ut60bt.device.activation_instructions,
            }
        );
        assert_eq!(
            Diagnosis::of(&stack, None, pick("ut61eplus")),
            Diagnosis::Other
        );
        assert_eq!(Diagnosis::of(&stack, None, None), Diagnosis::Other);
    }

    /// A meter that is another than the one chosen says what to pick: the
    /// entry it names, or Auto-detect, or its own model; one that names
    /// nothing to pick stays unreadable data.
    #[test]
    fn another_meter_says_what_to_pick() {
        let wrong = |reason| Error::WrongMeter {
            reason,
            raw: Vec::new(),
        };
        let entry = |id| MeterChoice::Entry(registry::find_device(id).expect("an entry"));
        let of = |e: &Error| Diagnosis::of(e, None, None);
        assert_eq!(
            of(&wrong(WrongMeter::OtherSeries {
                found: "Brymen BM82x",
                opened: "Brymen BM86x",
                id: "bm82x",
            })),
            Diagnosis::OtherMeter(entry("bm82x"))
        );
        assert_eq!(
            of(&wrong(WrongMeter::OtherFrameKind {
                sent: 15,
                expected: 6,
                opened: "OWON B35T+",
                remedy: FrameRemedy::OwnModel,
            })),
            Diagnosis::OtherMeter(MeterChoice::OwnModel)
        );
        assert_eq!(
            of(&wrong(WrongMeter::NoFifteenByteFrames)),
            Diagnosis::OtherMeter(MeterChoice::AutoDetect)
        );
        assert_eq!(
            of(&wrong(WrongMeter::OtherFrame {
                code: 87,
                model: "Voltcraft VC871",
                frame: 15,
                opened: "OWON B35T+",
                id: "vc871",
            })),
            Diagnosis::OtherMeter(entry("vc871"))
        );
        assert_eq!(
            of(&wrong(WrongMeter::OtherFrameKind {
                sent: 6,
                expected: 15,
                opened: "Voltcraft VC871",
                remedy: FrameRemedy::AutoDetect,
            })),
            Diagnosis::OtherMeter(MeterChoice::AutoDetect)
        );
        for nothing_to_pick in [
            WrongMeter::NoSixByteFrames,
            WrongMeter::UntoldFrameKind,
            WrongMeter::UnreadModel {
                code: 83,
                why: dmm_lib::error::Unread::NotYet,
            },
        ] {
            assert_eq!(of(&wrong(nothing_to_pick)), Diagnosis::BadData);
        }
        assert_eq!(
            of(&Error::CommandRejected(Rejected::RelIsOn)),
            Diagnosis::Other
        );
        assert_eq!(
            of(&Error::CommandRejected(Rejected::OtherModel {
                code: 61,
                model: "OWON CMS061",
                opened: "OWON B35T+",
                id: "cms061",
            })),
            Diagnosis::OtherMeter(entry("cms061"))
        );
    }

    /// Each app says what to pick in its own terms, by the name the pick's
    /// brand sells the meter under.
    #[test]
    fn each_app_says_what_to_pick_in_its_terms() {
        let cms061 = MeterChoice::Entry(registry::find_device("cms061").expect("an entry"));
        let help = english();
        assert_eq!(
            help.gui_choose_meter(cms061, pick("b35t+")),
            "Pick OWON CMS061 or Auto-detect in Settings (\u{2699})."
        );
        assert_eq!(
            help.cli_choose_meter(cms061, pick("b35t+")),
            "Pass --device cms061, or --device auto."
        );
        // Under Auto-detect, which opened it, only the entry.
        assert_eq!(
            help.gui_choose_meter(cms061, None),
            "Pick OWON CMS061 in Settings (\u{2699})."
        );
        assert_eq!(help.cli_choose_meter(cms061, None), "Pass --device cms061.");
        assert_eq!(
            help.gui_choose_meter(MeterChoice::OwnModel, None),
            "Pick the meter's own model in Settings (\u{2699})."
        );
        assert_eq!(
            help.cli_choose_meter(MeterChoice::OwnModel, None),
            "Pass --device with the meter's own model; --help lists them."
        );
        assert_eq!(
            help.gui_choose_meter(MeterChoice::AutoDetect, None),
            "Pick Auto-detect in Settings (\u{2699})."
        );
        let sold_as = pick("ut217a");
        assert_eq!(
            help.gui_choose_meter(cms061, sold_as),
            "Pick UNI-T UT217A or Auto-detect in Settings (\u{2699})."
        );
        assert_eq!(
            help.cli_choose_meter(cms061, sold_as),
            "Pass --device ut217a, or --device auto."
        );
        assert_eq!(
            help.cli_choose_meter(MeterChoice::AutoDetect, None),
            "Pass --device auto."
        );
    }

    /// A meter with the radio built in, not found or not looked for, goes by
    /// the name it was picked by, a sold-as name included; with no pick, or
    /// a pick of another meter since, by the error's.
    #[test]
    fn a_radio_meter_goes_by_its_pick() {
        let missed = |miss| Error::BluetoothOnly {
            model: "OWON CMS061",
            activation: "steps",
            miss,
        };
        let sold_as = pick("ut217a").expect("a row");
        assert_ne!(sold_as.display_name(), "OWON CMS061");
        assert_eq!(
            Diagnosis::of(&missed(BluetoothOnlyMiss::NotInRange), None, Some(sold_as)),
            Diagnosis::BluetoothOnlyNotFound {
                model: sold_as.display_name(),
                activation: "steps",
            }
        );
        assert_eq!(
            Diagnosis::of(&missed(BluetoothOnlyMiss::NotInRange), None, None),
            Diagnosis::BluetoothOnlyNotFound {
                model: "OWON CMS061",
                activation: "steps",
            }
        );
        for miss in [
            BluetoothOnlyMiss::SwitchedOff,
            BluetoothOnlyMiss::NotBuilt,
            BluetoothOnlyMiss::UsbAdapter,
        ] {
            assert_eq!(
                Diagnosis::of(&missed(miss), None, Some(sold_as)),
                Diagnosis::BluetoothNotSearched {
                    model: sold_as.display_name(),
                    miss
                }
            );
            assert_eq!(
                Diagnosis::of(&missed(miss), None, None),
                Diagnosis::BluetoothNotSearched {
                    model: "OWON CMS061",
                    miss
                }
            );
        }
        // Settings now naming another meter does not rename this one.
        assert_eq!(
            Diagnosis::of(
                &missed(BluetoothOnlyMiss::SwitchedOff),
                None,
                pick("ut60bt")
            ),
            Diagnosis::BluetoothNotSearched {
                model: "OWON CMS061",
                miss: BluetoothOnlyMiss::SwitchedOff
            }
        );
    }
}
