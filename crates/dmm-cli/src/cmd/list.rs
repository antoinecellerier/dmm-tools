//! `list`: the USB cables on the bus, and the Bluetooth adapters and meters
//! in range, with the setup help when nothing turned up.
//!
//! The device lines are the system's own and stay as they are; what is
//! printed around them is in the run's language (`cli/list.ftl`).

use crate::l10n::error_label;
use crate::open::{ADAPTER_SELECTOR, print_setup_sections};
use console::style;
use dmm_shared::help::LinksSearched;
use dmm_shared::tr;

/// The links a listing looked at, for the help it prints when it found
/// nothing, with `off_hint` the way to switch the radio's search on when it
/// is off. The open path answers this for itself — this is for `list`, which
/// searches whatever it was told to and never opens anything.
fn links_searched(bluetooth: bool, off_hint: &str) -> LinksSearched<'_> {
    if !bluetooth && dmm_lib::BLUETOOTH_SUPPORTED {
        LinksSearched::UsbBluetoothOff { hint: off_hint }
    } else {
        LinksSearched::from_usb_failure(scans_bluetooth(bluetooth))
    }
}

/// Whether `list` scans the radio: only where probing is on and the build
/// has a radio to scan.
fn scans_bluetooth(bluetooth: bool) -> bool {
    bluetooth && dmm_lib::BLUETOOTH_SUPPORTED
}

/// List what is reachable: the cables on the bus, and the adapters and meters
/// in range when `bluetooth` says the radio may be searched. False when the
/// cables could not be listed and nothing else turned up: the error is
/// printed already, and only the exit status is left to say it.
pub(crate) fn cmd_list(bluetooth: bool) -> bool {
    let l = crate::l10n::get();
    // A HID API that cannot enumerate is no reason to skip the radio: the
    // error goes out now and the scan still runs.
    let (cables, cables_failed) = match dmm_lib::list_devices() {
        Ok(cables) => (cables, false),
        Err(e) => {
            eprintln!("{} {}", error_label(), l.error(&e));
            (Vec::new(), true)
        }
    };
    for (i, dev) in cables.iter().enumerate() {
        println!("{} {dev}", style(format!("[{i}]")).cyan());
    }
    // Probing off, or a build with no radio, the scan is skipped whole — and
    // so is the line announcing it, which would promise a wait that never
    // happens.
    let adapters = if scans_bluetooth(bluetooth) {
        // The radio scan takes seconds where the bus listing is instant, so
        // say what the wait is for before it starts.
        eprintln!("{}", style(tr!(l, "list-scanning")).dim());
        match dmm_lib::list_bluetooth_devices() {
            Ok(adapters) => adapters,
            // A stack that is off or missing is not a device fault: the
            // cables above still stand, so the reason goes out dim and the
            // listing ends normally.
            Err(e) => {
                eprintln!("{}", style(l.error(&e)).dim());
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };
    // Heard ones first, as the library sorts them; the known ones the scan
    // missed keep the numbering, since `--adapter` takes them too.
    let (heard, not_heard): (Vec<_>, Vec<_>) = adapters.iter().partition(|d| !d.not_heard);
    for (i, dev) in heard.iter().enumerate() {
        println!("{} {dev}", style(format!("[{}]", cables.len() + i)).cyan());
    }
    let found = cables.len() + heard.len();
    // A paired adapter the scan missed is still a device the user may mean,
    // so it is listed with the rest, before any help.
    for (i, dev) in not_heard.iter().enumerate() {
        println!(
            "{} {}",
            style(format!("[{}]", found + i)).cyan(),
            style(tr!(l, "list-not-heard", device = dev.to_string())).dim()
        );
    }
    // The doc-screenshot scenes match both headings whole, in English,
    // before picturing the app with no meter, so keep them in step with the
    // script.
    if found == 0 {
        let heading = if not_heard.is_empty() {
            tr!(l, "list-none-found")
        } else {
            tr!(l, "list-none-heard")
        };
        eprintln!("{}", style(heading).yellow());
        print_setup_sections(links_searched(
            bluetooth,
            &crate::open::bluetooth_off_hint(l),
        ));
    }
    if found == 0 {
        return !cables_failed;
    }
    if found + not_heard.len() > 1 {
        let tip = tr!(l, "list-select-tip", selector = ADAPTER_SELECTOR);
        eprintln!("\n{}", style(tip).dim());
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With the radio's search switched off, the help that follows an empty
    /// listing says so and how to switch it on, rather than offering its
    /// steps; a build without the radio says nothing of it.
    #[test]
    fn a_listing_that_skipped_the_radio_says_how_to_search_it() {
        let help = crate::l10n::get().help();
        let off = help.sections(&links_searched(false, "switch it on"));
        let links: Vec<&str> = off.iter().map(|s| s.link.as_str()).collect();
        if dmm_lib::BLUETOOTH_SUPPORTED {
            assert_eq!(links, ["USB cable", "Bluetooth"]);
            assert_eq!(off[1].steps, ["switch it on"]);
        } else {
            assert_eq!(links, ["USB cable"]);
        }
        assert_eq!(
            help.sections(&links_searched(true, "unused")).len(),
            if dmm_lib::BLUETOOTH_SUPPORTED { 2 } else { 1 }
        );
    }

    /// A build with no radio neither scans nor announces a scan, whatever
    /// the setting says.
    #[test]
    fn a_build_without_bluetooth_never_scans() {
        assert_eq!(scans_bluetooth(true), dmm_lib::BLUETOOTH_SUPPORTED);
        assert!(!scans_bluetooth(false));
    }
}
