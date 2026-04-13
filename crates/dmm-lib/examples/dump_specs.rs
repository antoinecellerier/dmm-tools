//! Dump all specification data for manual verification.
//!
//! Prints formatted tables for each supported device, enumerating every mode
//! and range with resolution, accuracy bands, input impedance, and notes.
//! Compare side-by-side with the PDF manuals in `references/`.
//!
//! Usage:
//!   cargo run -p dmm-lib --example dump_specs
//!   cargo run -p dmm-lib --example dump_specs -- ut61b+
//!   cargo run -p dmm-lib --example dump_specs -- ut61eplus ut61d+

use dmm_lib::protocol::ut61eplus::mode::Mode;
use dmm_lib::protocol::ut61eplus::tables::{
    AccuracyBand, DeviceTable, ModeSpecInfo, RangeInfo, SpecInfo, lookup_mode_spec, lookup_spec,
};

/// All mode bytes in protocol order (0x00..=0x1E).
const ALL_MODES: &[(u8, &str)] = &[
    (0x00, "AC V"),
    (0x01, "AC mV"),
    (0x02, "DC V"),
    (0x03, "DC mV"),
    (0x04, "Hz"),
    (0x05, "Duty %"),
    (0x06, "Ω"),
    (0x07, "Continuity"),
    (0x08, "Diode"),
    (0x09, "Capacitance"),
    (0x0A, "°C"),
    (0x0B, "°F"),
    (0x0C, "DC µA"),
    (0x0D, "AC µA"),
    (0x0E, "DC mA"),
    (0x0F, "AC mA"),
    (0x10, "DC A"),
    (0x11, "AC A"),
    (0x12, "hFE"),
    (0x13, "Live"),
    (0x14, "NCV"),
    (0x15, "LoZ V"),
    (0x16, "LoZ V2"),
    (0x17, "LPF"),
    (0x18, "LPF V"),
    (0x19, "AC+DC V"),
    (0x1A, "LPF mV"),
    (0x1B, "AC+DC mV"),
    (0x1C, "LPF A"),
    (0x1D, "AC+DC A"),
    (0x1E, "Inrush"),
];

/// Devices to dump (id, display name).
const DEVICES: &[(&str, &str)] = &[
    ("ut61eplus", "UT61E+ (22,000 counts)"),
    ("ut61b+", "UT61B+ (6,000 counts)"),
    ("ut61d+", "UT61D+ (6,000 counts)"),
];

/// Inner width between left and right box borders.
const W: usize = 72;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let devices: Vec<(&str, &str)> = if args.is_empty() {
        DEVICES.to_vec()
    } else {
        args.iter()
            .map(|id| {
                let name = DEVICES
                    .iter()
                    .find(|(d, _)| *d == id.as_str())
                    .map(|(_, n)| *n)
                    .unwrap_or(id.as_str());
                (id.as_str(), name)
            })
            .collect()
    };

    for (i, (device_id, device_name)) in devices.iter().enumerate() {
        if i > 0 {
            println!();
        }
        dump_device(device_id, device_name);
    }
}

/// Print a row: `│` + content padded to W + `│`.
/// `content` is the text between the borders (no leading `│`).
fn row(content: &str) {
    // Count display width (ASCII chars = 1 each; multi-byte Unicode also 1 each
    // for the box-drawing and symbols we use). This is approximate but works for
    // our content which is all single-width characters.
    let display_len = unicode_display_width(content);
    let pad = W.saturating_sub(display_len);
    println!("│{}{}│", content, " ".repeat(pad));
}

/// Approximate display width: count Unicode scalar values.
/// All characters we use (ASCII, box-drawing, Greek, degree sign, etc.) are
/// single-width in a monospace terminal.
fn unicode_display_width(s: &str) -> usize {
    s.chars().count()
}

fn dump_device(device_id: &str, device_name: &str) {
    let title = format!("{} \u{2014} Specification Data", device_name);
    let title_len = unicode_display_width(&title);
    let title_pad_total = W.saturating_sub(title_len);
    let title_pad_left = title_pad_total / 2;
    let title_pad_right = title_pad_total - title_pad_left;

    println!("┌{}┐", "─".repeat(W));
    println!(
        "│{}{}{}│",
        " ".repeat(title_pad_left),
        title,
        " ".repeat(title_pad_right)
    );
    println!("└{}┘", "─".repeat(W));

    let mut any_mode = false;

    for &(mode_byte, mode_label) in ALL_MODES {
        let mode_spec = lookup_mode_spec(device_id, mode_byte as u16);

        let mut ranges: Vec<(u8, &SpecInfo, Option<(&RangeInfo, &str)>)> = Vec::new();
        for r in 0..20u8 {
            if let Some(spec) = lookup_spec(device_id, mode_byte as u16, r) {
                // Skip placeholder entries that have no range label AND no accuracy
                // (e.g. LPF mV 220mV — range exists in protocol but not in manual).
                let range_label = get_range_label(device_id, mode_byte, r);
                if spec.accuracy.is_empty() && range_label.is_none() {
                    continue;
                }
                ranges.push((r, spec, range_label));
            }
        }

        if mode_spec.is_none() && ranges.is_empty() {
            continue;
        }
        any_mode = true;

        // Sort ranges by physical value (increasing) like the manuals.
        ranges.sort_by(|a, b| {
            let val_a = a.2.map(|(ri, _)| label_sort_key(ri.label)).unwrap_or(0.0);
            let val_b = b.2.map(|(ri, _)| label_sort_key(ri.label)).unwrap_or(0.0);
            val_a
                .partial_cmp(&val_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Mode header
        println!();
        let header = format!(" Mode 0x{:02X}: {} ", mode_byte, mode_label);
        let header_len = unicode_display_width(&header);
        let fill = W.saturating_sub(header_len + 1); // +1 for the `─` after `┌`
        println!("┌─{}{}┐", header, "─".repeat(fill));

        // Mode-level info
        if let Some(ms) = mode_spec {
            print_mode_spec(ms);
        }

        // Per-range table
        if !ranges.is_empty() {
            row("");
            row(&format!(
                " {:>5}  {:<10}  {:<12}  {}",
                "Range", "Label", "Resolution", "Accuracy"
            ));
            row(&format!(
                " {}  {}  {}  {}",
                "─".repeat(5),
                "─".repeat(10),
                "─".repeat(12),
                "─".repeat(W - 5 - 10 - 12 - 8)
            ));

            for (r, spec, range_label) in &ranges {
                let label = range_label.map(|(ri, _)| ri.label).unwrap_or("\u{2014}");

                if let Some(first) = spec.accuracy.first() {
                    row(&format!(
                        " {:>5}  {:<10}  {:<12}  {}",
                        r,
                        label,
                        spec.resolution,
                        format_accuracy(first),
                    ));
                    for band in spec.accuracy.iter().skip(1) {
                        row(&format!(
                            " {:>5}  {:<10}  {:<12}  {}",
                            "",
                            "",
                            "",
                            format_accuracy(band),
                        ));
                    }
                } else {
                    row(&format!(
                        " {:>5}  {:<10}  {:<12}  (not specified)",
                        r, label, spec.resolution,
                    ));
                }
            }
        }

        println!("└{}┘", "─".repeat(W));
    }

    if !any_mode {
        println!();
        println!("  (no specification data available for this device)");
    }
}

fn print_mode_spec(ms: &ModeSpecInfo) {
    if let Some(z) = ms.input_impedance {
        row(&format!(" Input impedance:      {}", z));
    }
    if let Some(p) = ms.overload_protection {
        row(&format!(" Overload protection:  {}", p));
    }
    if !ms.notes.is_empty() {
        row(&format!(" Notes:                {}", ms.notes.join(", ")));
    }
}

fn format_accuracy(band: &AccuracyBand) -> String {
    match band.freq_range {
        Some(freq) => format!("\u{00b1}({})  [{}]", band.accuracy, freq),
        None => format!("\u{00b1}({})", band.accuracy),
    }
}

/// Parse a range label like "220mV", "2.2kΩ", "22µF" into a sortable
/// numeric value in base units (e.g. 0.22 for "220mV", 2200.0 for "2.2kΩ").
/// Non-numeric labels (e.g. "Duty", "Cont") return 0.0.
fn label_sort_key(label: &str) -> f64 {
    // Find where the numeric part ends.
    let num_end = label
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(label.len());
    let num: f64 = match label[..num_end].parse() {
        Ok(v) => v,
        Err(_) => return 0.0,
    };
    let suffix = &label[num_end..];
    let multiplier = if suffix.starts_with('m') {
        1e-3
    } else if suffix.starts_with("µ") || suffix.starts_with("u") {
        1e-6
    } else if suffix.starts_with('n') {
        1e-9
    } else if suffix.starts_with('p') {
        1e-12
    } else if suffix.starts_with('k') {
        1e3
    } else if suffix.starts_with('M') {
        1e6
    } else if suffix.starts_with('G') {
        1e9
    } else {
        1.0
    };
    num * multiplier
}

/// Get the RangeInfo for a given mode+range from the device table.
fn get_range_label(
    device_id: &str,
    mode_byte: u8,
    range: u8,
) -> Option<(&'static RangeInfo, &'static str)> {
    use dmm_lib::protocol::ut61eplus::tables::{
        ut61b_plus::Ut61bPlusTable, ut61d_plus::Ut61dPlusTable, ut61e_plus::Ut61ePlusTable,
    };
    use std::sync::LazyLock;

    static UT61E: LazyLock<Ut61ePlusTable> = LazyLock::new(Ut61ePlusTable::new);
    static UT61B: LazyLock<Ut61bPlusTable> = LazyLock::new(Ut61bPlusTable::new);
    static UT61D: LazyLock<Ut61dPlusTable> = LazyLock::new(Ut61dPlusTable::new);

    let table: &dyn DeviceTable = match device_id {
        "ut61eplus" | "ut161e" | "mock" => &*UT61E,
        "ut61b+" | "ut161b" => &*UT61B,
        "ut61d+" | "ut161d" => &*UT61D,
        _ => return None,
    };

    let mode = Mode::from_byte(mode_byte).ok()?;
    let ri = table.range_info(mode, range)?;
    Some((ri, ri.unit))
}
