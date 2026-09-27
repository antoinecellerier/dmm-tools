//! `list`: the USB cables on the bus, and the Bluetooth adapters and meters
//! in range, with the setup help when nothing turned up.

use crate::open::{ADAPTER_SELECTOR, print_setup_sections};
use console::style;
use dmm_shared::help::LinksSearched;

/// The links a listing looked at, for the help it prints when it found
/// nothing. The open path answers this for itself — this is for `list`, which
/// searches whatever it was told to and never opens anything.
fn links_searched(bluetooth: bool) -> LinksSearched<'static> {
    LinksSearched::from_usb_failure(scans_bluetooth(bluetooth))
}

/// Whether `list` scans the radio: only where probing is on and the build
/// has a radio to scan.
fn scans_bluetooth(bluetooth: bool) -> bool {
    bluetooth && dmm_lib::BLUETOOTH_SUPPORTED
}

/// List what is reachable: the cables on the bus, and the adapters and meters
/// in range when `bluetooth` says the radio may be searched.
pub(crate) fn cmd_list(bluetooth: bool) -> Result<(), Box<dyn std::error::Error>> {
    // A HID API that cannot enumerate is no reason to skip the radio: the
    // error goes out now and the scan still runs.
    let (cables, cables_failed) = match dmm_lib::list_devices() {
        Ok(cables) => (cables, false),
        Err(e) => {
            eprintln!("{} {e}", style("Error:").red().bold());
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
        eprintln!("{}", style("Scanning over Bluetooth\u{2026}").dim());
        match dmm_lib::list_bluetooth_devices() {
            Ok(adapters) => adapters,
            // A stack that is off or missing is not a device fault: the
            // cables above still stand, so the reason goes out dim and the
            // listing ends normally.
            Err(e) => {
                eprintln!("{}", style(e.to_string()).dim());
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
            style(format!(
                "{dev}, paired but not heard: check it is switched on"
            ))
            .dim()
        );
    }
    // The doc-screenshot scenes match both headings whole before picturing
    // the app with no meter, so keep them in step with the script.
    if found == 0 {
        let heading = if not_heard.is_empty() {
            "No devices found."
        } else {
            "No devices heard in range."
        };
        eprintln!("{}", style(heading).yellow());
        print_setup_sections(links_searched(bluetooth));
    }
    if found == 0 {
        // Already reported above; the status still says the listing failed.
        if cables_failed {
            std::process::exit(1);
        }
        return Ok(());
    }
    if found + not_heard.len() > 1 {
        eprintln!(
            "\n{}",
            style(format!(
                "Tip: use {ADAPTER_SELECTOR} to select a specific device"
            ))
            .dim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With the radio out of the picture, the help that follows an empty
    /// listing must not offer steps for it.
    #[test]
    fn a_listing_that_skipped_the_radio_offers_no_bluetooth_steps() {
        let links: Vec<&str> = links_searched(false)
            .sections()
            .iter()
            .map(|s| s.link)
            .collect();
        assert_eq!(links, ["USB cable"]);
        assert_eq!(
            links_searched(true).sections().len(),
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
