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
use crate::tr;
use dmm_lib::list_devices;
use dmm_lib::mock::MockMode;
use dmm_lib::protocol::registry::SelectableDevice;
use dmm_lib::protocol::{Stability, registry};
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
pub const RESOURCES: &[Resource] = &[crate::ftl!("en-US", "shared.ftl")];

/// The help in English, for what stays English whatever the language: shell
/// completions, and the docs' generated tables.
pub fn english() -> Help<'static> {
    static ENGLISH: OnceLock<Localizer> = OnceLock::new();
    Help(ENGLISH.get_or_init(|| Localizer::en_us(RESOURCES)))
}

impl Localizer {
    /// The help text in this localizer's language. Its resources must
    /// include [`RESOURCES`].
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
        for (brand, rows) in registry::brand_groups() {
            help.push_str(&format!("\n  {}\n", brand.name()));
            for row in rows {
                let tag = self
                    .device_tag(row.device, false)
                    .map_or(String::new(), |t| format!(" ({t})"));
                help.push_str(&format!(
                    "    {:<width$} {}{}\n",
                    row.id(),
                    row.model_name(),
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
        if !device.requires_hardware {
            return Some(tr!(self.0, "device-no-hardware"));
        }
        let stability = (device.new_protocol)().profile().stability;
        (verified_too || !stability.is_verified()).then(|| self.stability(stability))
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

/// `cmd`, with the `--device` and `--mock-mode` flags on it and its
/// subcommands offering their values, for a completion script.
///
/// Both flags take a plain string so a bad one gets our own error, with the
/// hint to run `--help`; clap's possible values would replace it with its
/// own. So they are added only to the command a script is generated from,
/// never to the one that parses. Aliases are left out: the ids cover every
/// device, and `ut61e+` would need quoting. In English: a completion script
/// outlives the language it was generated in.
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
    /// The radio alone, for a meter with Bluetooth built in: its display
    /// name and the registry's steps that switch its radio on.
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
/// Both binaries print the same lines ([`Help::adapter_lines`]) but only
/// attach their "pick another adapter" hint when devices were actually
/// listed. Naming the three answers keeps that decision off a line count.
pub enum ConnectedAdapters {
    /// Nothing is plugged in.
    None,
    /// One `  [i] dev` line per device, without the header.
    Listed(Vec<String>),
    /// HID enumeration itself failed, so there is nothing to list.
    Unavailable,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every message this crate looks up exists in en-US with the variables
    /// it is given, and every message in `shared.ftl` is used.
    #[test]
    fn the_shared_messages_match_their_uses() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let problems = crate::l10n::check::check(&src, RESOURCES, &["shared.ftl"]);
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
}
