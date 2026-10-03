//! Reading a reporter's file as a capture report, whatever build wrote it
//! and however it travelled, and keeping their meter's address out of what
//! triage prints.

use super::super::report::CaptureReport;
use dmm_lib::protocol::registry::{self, SelectableDevice};
use std::borrow::Cow;
use std::path::Path;

/// The report at `path`, its older shapes brought up to date as a resumed
/// run reads them.
pub(super) fn load(path: &Path) -> Result<CaptureReport, String> {
    let shown = path.display();
    let bytes = std::fs::read(path).map_err(|e| format!("{shown}: {e}"))?;
    let not_a_report = || format!("{shown}: not a capture report");
    let text = String::from_utf8(bytes).map_err(|_| not_a_report())?;
    let text = unescape_utf8(&text);
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).map_err(|_| not_a_report())?;
    if value.get("tool_version").is_none() {
        // A plan is a step list too, and the file a reporter would most
        // likely mix up with their report.
        return Err(if value.get("steps").is_some() {
            format!("{shown}: a capture plan, not a report; pass it as --plan beside the report")
        } else {
            not_a_report()
        });
    }
    let mut report: CaptureReport =
        serde_yaml_ng::from_value(value).map_err(|e| format!("{shown}: {e}"))?;
    for step in &mut report.steps {
        step.normalize_legacy();
    }
    Ok(report)
}

/// The meter to decode with: `--device` as typed, else the report's own.
/// `None` when neither names one, and triage then reads the report as it
/// stands.
pub(super) fn device(
    named: Option<&str>,
    report: &CaptureReport,
) -> Result<Option<&'static SelectableDevice>, String> {
    match named {
        Some(id) => registry::resolve_device(id)
            .map(Some)
            .ok_or_else(|| format!("--device {id} names no meter to decode the report with")),
        None => Ok(report.device_id.as_deref().and_then(registry::find_device)),
    }
}

/// Text with runs of `\XX` byte escapes put back as the UTF-8 they spell.
///
/// A report pasted out of some terminals arrives with every non-ASCII
/// character escaped that way (`\CE\A9` for Ω), step ids and mode names
/// included, so nothing would match the parser's own strings. Only a run
/// that decodes to non-ASCII UTF-8 is taken: a Windows path's `\AB` stays.
fn unescape_utf8(text: &str) -> Cow<'_, str> {
    if !text.contains('\\') {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find('\\') {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos..];
        let (bytes, used) = escaped_run(tail);
        match std::str::from_utf8(&bytes) {
            Ok(s) if !s.is_ascii() => {
                out.push_str(s);
                rest = &tail[used..];
            }
            _ => {
                out.push('\\');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

/// The bytes a run of `\XX` escapes at the start of `text` spells, and how
/// many characters of `text` it takes.
fn escaped_run(text: &str) -> (Vec<u8>, usize) {
    let hex = |c: u8| matches!(c, b'0'..=b'9' | b'A'..=b'F');
    let b = text.as_bytes();
    let mut bytes = Vec::new();
    let mut i = 0;
    while b.len() >= i + 3 && b[i] == b'\\' && hex(b[i + 1]) && hex(b[i + 2]) {
        // Two ASCII hex digits, so the slice is on character boundaries.
        if let Ok(byte) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
            bytes.push(byte);
        }
        i += 3;
    }
    (bytes, i)
}

/// What stands in for a hardware address in triage output.
const ADDRESS_MASK: &str = "XX:XX:XX:XX:XX:XX";

/// `text` with every device identifier masked: a run of hex digits, `:`
/// and `-` holding four or more separators and twelve or more digits
/// (`AA:BB:CC:DD:EE:FF`, Windows' dashes, the two addresses a Windows
/// Bluetooth device id joins, a macOS peripheral UUID), or twelve hex
/// digits alone (an address written as one number). A Bluetooth meter's
/// address is in `transport_info` and can turn up in any error text, and
/// triage output is pasted into issues. Frame hex is space-separated and
/// times and dates have fewer separators, so neither is caught.
pub(super) fn scrub_addresses(text: &str) -> String {
    let in_run = |c: &u8| c.is_ascii_hexdigit() || *c == b':' || *c == b'-';
    let b = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut i = 0;
    while i < b.len() {
        if !in_run(&b[i]) {
            i += 1;
            continue;
        }
        let end = i + b[i..].iter().take_while(|c| in_run(c)).count();
        let run = &b[i..end];
        let separators = run.iter().filter(|c| !c.is_ascii_hexdigit()).count();
        let digits = run.len() - separators;
        if (separators >= 4 && digits >= 12) || (separators == 0 && digits == 12) {
            // ASCII both ends, so these are character boundaries.
            out.push_str(&text[copied..i]);
            out.push_str(ADDRESS_MASK);
            copied = end;
        }
        i = end;
    }
    out.push_str(&text[copied..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 88e80ed reports came through a terminal that escaped every
    /// non-ASCII byte; Ω and → are back, and a lone backslash is kept.
    #[test]
    fn escaped_utf8_is_restored() {
        assert_eq!(
            unescape_utf8(r"id: ohm_ranges/range:600\CE\A9"),
            "id: ohm_ranges/range:600Ω"
        );
        assert_eq!(unescape_utf8(r"a \E2\86\92 b"), "a → b");
        assert_eq!(unescape_utf8(r"C:\AB\cd \41"), r"C:\AB\cd \41");
        assert_eq!(unescape_utf8("no escapes"), "no escapes");
    }

    #[test]
    fn addresses_are_masked_and_frame_hex_is_not() {
        assert_eq!(
            scrub_addresses("Bluetooth DMM (00:11:22:AA:BB:CC) and 00-11-22-aa-bb-cc."),
            "Bluetooth DMM (XX:XX:XX:XX:XX:XX) and XX:XX:XX:XX:XX:XX."
        );
        assert_eq!(
            scrub_addresses("BluetoothLE#BluetoothLE00:11:22:33:44:55-66:77:88:99:aa:bb"),
            "BluetoothLE#BluetoothLXX:XX:XX:XX:XX:XX"
        );
        assert_eq!(
            scrub_addresses("peripheral 8F3A1B2C-1234-5678-9ABC-DEF012345678, addr 001122AABBCC."),
            "peripheral XX:XX:XX:XX:XX:XX, addr XX:XX:XX:XX:XX:XX."
        );
        let frame = "AB CD 03 5E 01 D9 00:11:22:33:44 Ω at 2026-10-03T12:30:05 (eea85ba)";
        assert_eq!(scrub_addresses(frame), frame);
    }
}
