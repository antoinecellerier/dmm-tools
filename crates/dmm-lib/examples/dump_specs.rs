//! Dump the specification data of every device, to check it against the
//! manuals.
//!
//! Walks the device registry (or the device ids given) and prints each
//! model's `Protocol::spec_sheet`, skipping models without spec data unless
//! named. Three formats:
//!
//! - `text` (default): boxed tables for side-by-side reading with the PDF.
//! - `json`: the shape of a manual transcription, so the code's tables can be
//!   diffed against a verified transcription: `{model, tables: [{table, page,
//!   input_impedance, overload_protection, notes, ranges: [{range,
//!   resolution, accuracy: [{freq_range, accuracy}]}]}]}`. One device.
//! - `html`: a review sheet, each table laid out as the manual prints it,
//!   beside the render of its manual page (`--pages-dir`), with the cells
//!   listed in `--marks` highlighted. One device.
//!
//! Usage:
//!   cargo run -p dmm-lib --example dump_specs
//!   cargo run -p dmm-lib --example dump_specs -- ut61eplus ut61d+
//!   cargo run -p dmm-lib --example dump_specs -- --format json ut61b+
//!   cargo run -p dmm-lib --example dump_specs -- --format html \
//!       --pages-dir pages --marks marks.json ut61eplus > sheet.html

use dmm_lib::protocol::registry::{self, SelectableDevice};
use dmm_lib::specs::{AccuracyBand, ModeSpecInfo, SpecSheetRow, SpecSheetTable};
use serde_json::{Value, json};
use std::cell::Cell;
use std::fmt::Write as _;
use std::path::Path;

const USAGE: &str = "usage: dump_specs [--format text|json|html] [--pages-dir DIR] \
                     [--marks FILE] [DEVICE_ID...]";

#[derive(Clone, Copy, PartialEq)]
enum Format {
    Text,
    Json,
    Html,
}

struct Options {
    format: Format,
    pages_dir: Option<String>,
    marks: Option<String>,
    ids: Vec<String>,
}

fn parse_args() -> Result<Options, String> {
    let mut o = Options {
        format: Format::Text,
        pages_dir: None,
        marks: None,
        ids: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--format" => {
                o.format = match value()?.as_str() {
                    "text" => Format::Text,
                    "json" => Format::Json,
                    "html" => Format::Html,
                    other => return Err(format!("unknown format {other:?}")),
                }
            }
            "--pages-dir" => o.pages_dir = Some(value()?),
            "--marks" => o.marks = Some(value()?),
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            _ => o.ids.push(arg),
        }
    }
    if o.format != Format::Html && (o.pages_dir.is_some() || o.marks.is_some()) {
        return Err("--pages-dir and --marks apply to --format html only".into());
    }
    Ok(o)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("dump_specs: {e}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let o = parse_args().map_err(|e| format!("{e}\n{USAGE}"))?;
    let named = !o.ids.is_empty();
    let devices: Vec<&SelectableDevice> = if named {
        o.ids
            .iter()
            .map(|id| registry::resolve_device(id).ok_or_else(|| format!("unknown device {id:?}")))
            .collect::<Result<_, _>>()?
    } else {
        registry::DEVICES.iter().collect()
    };

    if o.format == Format::Text {
        let mut first = true;
        for dev in devices {
            let sheet = (dev.new_protocol)().spec_sheet();
            if sheet.is_empty() && !named {
                continue;
            }
            if !first {
                println!();
            }
            first = false;
            dump_text(dev, &sheet);
        }
        return Ok(());
    }

    let [dev] = devices[..] else {
        return Err(format!(
            "--format json and html take exactly one device id\n{USAGE}"
        ));
    };
    let protocol = (dev.new_protocol)();
    let sheet = protocol.spec_sheet();
    if sheet.is_empty() {
        return Err(format!("{} has no specification data", dev.id));
    }
    if o.format == Format::Json {
        let text =
            serde_json::to_string_pretty(&to_json(dev, &sheet)).map_err(|e| e.to_string())?;
        println!("{text}");
    } else {
        let marks = match &o.marks {
            Some(path) => Marks::load(path)?,
            None => Marks::default(),
        };
        let model = protocol.profile().model_name;
        print!(
            "{}",
            to_html(dev, model, &sheet, o.pages_dir.as_deref(), &marks)
        );
    }
    Ok(())
}

// --- text ---

/// Display counts for the title, where the dump has always named them.
const COUNTS: &[(&str, &str)] = &[
    ("ut61eplus", "22,000"),
    ("ut61b+", "6,000"),
    ("ut61d+", "6,000"),
];

/// Inner width between left and right box borders.
const W: usize = 72;

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

fn dump_text(dev: &SelectableDevice, sheet: &[SpecSheetTable]) {
    let name = match COUNTS.iter().find(|(id, _)| *id == dev.id) {
        Some((_, counts)) => format!("{} ({counts} counts)", dev.display_name),
        None => dev.display_name.to_string(),
    };
    let title = format!("{} \u{2014} Specification Data", name);
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

    if sheet.is_empty() {
        println!();
        println!("  (no specification data available for this device)");
        return;
    }

    for table in sheet {
        // Table header
        println!();
        let header = format!(" {} (PDF p. {}) ", table.name, table.page);
        let header_len = unicode_display_width(&header);
        let fill = W.saturating_sub(header_len + 1); // +1 for the `─` after `┌`
        println!("┌─{}{}┐", header, "─".repeat(fill));

        // Mode-level info
        print_mode_spec(table.mode);

        // Per-range table
        if !table.rows.is_empty() {
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

            for r in &table.rows {
                let range = r.range_raw.map_or("any".to_string(), |b| b.to_string());
                let spec = r.spec;

                if let Some(first) = spec.accuracy.first() {
                    row(&format!(
                        " {:>5}  {:<10}  {:<12}  {}",
                        range,
                        r.label,
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
                        range, r.label, spec.resolution,
                    ));
                }
            }
        }

        println!("└{}┘", "─".repeat(W));
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

// --- json ---

/// The sheet in the shape of a manual transcription.
fn to_json(dev: &SelectableDevice, sheet: &[SpecSheetTable]) -> Value {
    let tables: Vec<Value> = sheet
        .iter()
        .map(|t| {
            let ranges: Vec<Value> = t
                .rows
                .iter()
                .map(|r| {
                    let accuracy: Vec<Value> = r
                        .spec
                        .accuracy
                        .iter()
                        .map(|b| json!({"freq_range": b.freq_range, "accuracy": b.accuracy}))
                        .collect();
                    json!({
                        "range": r.label,
                        "resolution": r.spec.resolution,
                        "accuracy": accuracy,
                    })
                })
                .collect();
            json!({
                "table": t.name,
                "page": t.page,
                "input_impedance": t.mode.input_impedance,
                "overload_protection": t.mode.overload_protection,
                "notes": t.mode.notes,
                "ranges": ranges,
            })
        })
        .collect();
    json!({"model": dev.display_name, "tables": tables})
}

// --- html ---

/// A mark's colour: a provenance status, or `Note` for a plain-string mark.
/// Ordered from the least to the most pressing, which a cell carrying
/// several marks shows.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Status {
    /// The two transcriptions disagreed and one was right (`a` / `b`).
    Settled,
    /// Flagged, then confirmed as printed.
    Confirmed,
    /// A secondary source differs; the manual is kept.
    CrossSource,
    Note,
    /// Both transcriptions were wrong; the value was corrected.
    Other,
    /// Nobody could read it.
    Unknown,
}

impl Status {
    fn parse(status: &str) -> Option<Self> {
        Some(match status {
            "a" | "b" => Self::Settled,
            "confirmed" => Self::Confirmed,
            "cross-source" => Self::CrossSource,
            "other" => Self::Other,
            "unknown" => Self::Unknown,
            _ => return None,
        })
    }

    fn class(self) -> &'static str {
        match self {
            Self::Settled => "mark settled",
            Self::Confirmed => "mark confirmed",
            Self::CrossSource => "mark cross",
            Self::Note => "mark",
            Self::Other => "mark other",
            Self::Unknown => "mark unknown",
        }
    }

    const LEGEND: [(Self, &'static str); 5] = [
        (Self::Unknown, "unknown: nobody could read it"),
        (
            Self::Other,
            "other: both transcriptions were wrong, value corrected",
        ),
        (
            Self::Confirmed,
            "confirmed: flagged, then confirmed as printed",
        ),
        (
            Self::CrossSource,
            "cross-source: another source differs, the manual is kept",
        ),
        (
            Self::Settled,
            "a / b: the transcriptions disagreed, one was right",
        ),
    ];
}

/// The marks on one cell: their notes, one per line, and the most pressing
/// status among them.
struct Mark {
    notes: String,
    status: Status,
}

/// Cells to highlight: `"<table> / <range> / <field>"` or `"<table> /
/// <field>"` → note. A note may also be an object with `status` and
/// `evidence`, the shape of a transcription's provenance file, which
/// colours the cell by status.
#[derive(Default)]
struct Marks {
    /// (key, note, status, whether it matched a cell)
    entries: Vec<(String, String, Status, Cell<bool>)>,
}

impl Marks {
    fn load(path: &str) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let value: Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        let Value::Object(map) = value else {
            return Err(format!("{path}: not a JSON object"));
        };
        let entries = map
            .into_iter()
            .map(|(key, note)| {
                let (text, status) = note_text(&note);
                (mark_key(&key), text, status, Cell::new(false))
            })
            .collect();
        Ok(Self { entries })
    }

    /// The marks on any of `keys`, or `None` if there are none.
    fn find(&self, keys: &[String]) -> Option<Mark> {
        let keys: Vec<String> = keys.iter().map(|k| mark_key(k)).collect();
        let found: Vec<_> = self
            .entries
            .iter()
            .filter(|(key, ..)| keys.contains(key))
            .collect();
        let status = found.iter().map(|(_, _, status, _)| *status).max()?;
        let notes: Vec<String> = found
            .iter()
            .map(|(key, note, _, used)| {
                used.set(true);
                format!("{key}: {note}")
            })
            .collect();
        Some(Mark {
            notes: notes.join("\n"),
            status,
        })
    }
}

/// A mark key with each `/`-separated part trimmed.
fn mark_key(key: &str) -> String {
    key.split('/')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" / ")
}

/// A mark's text and colour. A provenance entry that gathered several
/// verdicts appends each later one to its evidence as ` | <verdict>: …`;
/// the most pressing of them colours the cell.
fn note_text(note: &Value) -> (String, Status) {
    let field = |name| note.get(name).and_then(Value::as_str).unwrap_or("");
    match note {
        Value::String(s) => (s.clone(), Status::Note),
        Value::Object(_) if !field("status").is_empty() => {
            let evidence = field("evidence");
            let status = evidence
                .split(" | ")
                .skip(1)
                .filter_map(|part| Status::parse(part.split_once(": ")?.0))
                .chain(Status::parse(field("status")))
                .max()
                .unwrap_or(Status::Note);
            let text = match evidence {
                "" => field("status").to_string(),
                evidence => format!("{}: {evidence}", field("status")),
            };
            (text, status)
        }
        other => (other.to_string(), Status::Note),
    }
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// ` class="mark …" title="…"` for a marked element, nothing otherwise.
fn mark_attr(mark: &Option<Mark>) -> String {
    match mark {
        Some(m) => format!(
            " class=\"{}\" title=\"{}\"",
            m.status.class(),
            esc(&m.notes)
        ),
        None => String::new(),
    }
}

fn td(out: &mut String, rowspan: usize, text: &str, mark: &Option<Mark>) {
    let span = if rowspan > 1 {
        format!(" rowspan={rowspan}")
    } else {
        String::new()
    };
    let _ = write!(out, "<td{span}{}>{}</td>", mark_attr(mark), esc(text));
}

/// The render of PDF page `page` in `dir`, if it is there. `pdftoppm` pads
/// the number to the width of the document's last page number, so each
/// width is tried; the path is taken as given, which only finds the file
/// when it is relative to the working directory.
fn existing_page_image(dir: &str, page: u16) -> Option<String> {
    let dir = dir.trim_end_matches('/');
    (1..=4)
        .map(|width| format!("{dir}/p-{page:0width$}.png"))
        .find(|path| Path::new(path).exists())
}

/// The render of PDF page `page` in `dir`, two digits assumed when no file
/// is found.
fn page_image(dir: &str, page: u16) -> String {
    existing_page_image(dir, page).unwrap_or_else(|| {
        let dir = dir.trim_end_matches('/');
        format!("{dir}/p-{page:02}.png")
    })
}

const CSS: &str = "
:root { color-scheme: light dark; --bg: #ffffff; --fg: #1a1a1a; --line: #8c8c8c;
  --head: #ececec; --muted: #555555; --link: #0b57d0; --mark-fg: #000000;
  --note: #ffd84d; --settled: #fff3b0; --confirmed: #a8c8ff; --cross: #d9b8f5;
  --other: #ffb366; --unknown: #ff8a8a; }
@media (prefers-color-scheme: dark) {
  :root { --bg: #161616; --fg: #e8e8e8; --line: #6b6b6b; --head: #2b2b2b;
    --muted: #a8a8a8; --link: #8ab4f8; --mark-fg: #ffffff;
    --note: #7a5c00; --settled: #4a4214; --confirmed: #1f3f7a; --cross: #56307a;
    --other: #8a4300; --unknown: #8f1f1f; }
}
body { background: var(--bg); color: var(--fg); margin: 16px;
  font: 14px/1.4 system-ui, sans-serif; }
a { color: var(--link); }
h1 { font-size: 1.4em; margin: 0 0 4px; }
h2 { font-size: 1.15em; margin: 0 0 8px; }
header p, .page-no { color: var(--muted); }
section { border-top: 1px solid var(--line); margin-top: 20px; padding-top: 12px; }
.cols { display: flex; flex-wrap: wrap; gap: 16px; align-items: flex-start; }
.img { flex: 1 1 45%; min-width: 280px; }
.img img { width: 100%; border: 1px solid var(--line); background: #ffffff; }
.tbl { flex: 1 1 45%; min-width: 280px; overflow-x: auto; }
table { border-collapse: collapse; }
th, td { border: 1px solid var(--line); padding: 3px 8px; text-align: left;
  vertical-align: middle; }
th { background: var(--head); }
tbody + tbody tr:first-child td { border-top: 2px solid var(--fg); }
.mark { background: var(--note); color: var(--mark-fg); cursor: help; }
.mark.settled { background: var(--settled); }
.mark.confirmed { background: var(--confirmed); }
.mark.cross { background: var(--cross); }
.mark.other { background: var(--other); }
.mark.unknown { background: var(--unknown); }
.legend { list-style: none; padding: 0; display: flex; flex-wrap: wrap; gap: 6px 16px; }
.legend li { padding: 1px 6px; }
details { margin-top: 12px; }
summary { cursor: pointer; }
";

fn to_html(
    dev: &SelectableDevice,
    model: &str,
    sheet: &[SpecSheetTable],
    pages_dir: Option<&str>,
    marks: &Marks,
) -> String {
    let mut body = String::new();
    // The parts of a split manual table follow each other and share a name.
    let tables: Vec<&[SpecSheetTable]> = sheet.chunk_by(|a, b| a.name == b.name).collect();
    let count = tables.len();
    for (i, parts) in tables.iter().enumerate() {
        // The next table's page bounds this one's continuation pages.
        let next_page = tables.get(i + 1).map(|next| next[0].page);
        html_section(&mut body, parts, next_page, pages_dir, marks);
    }

    let mut out = String::new();
    let _ = write!(
        out,
        "<!DOCTYPE html>\n<html lang=en>\n<head>\n<meta charset=utf-8>\n\
         <meta name=viewport content=\"width=device-width, initial-scale=1\">\n\
         <title>{} spec sheet</title>\n<style>{CSS}</style>\n</head>\n<body>\n\
         <header>\n<h1>{} — specification sheet</h1>\n<p>Device <code>{}</code>, \
         model {}, {count} tables.",
        esc(dev.display_name),
        esc(dev.display_name),
        esc(dev.id),
        esc(model),
    );
    if let Some(url) = dev.manual_url {
        let _ = write!(out, " <a href=\"{}\">Product page</a>.", esc(url));
    }
    out.push_str("</p>\n");

    if !marks.entries.is_empty() {
        out.push_str("<ul class=legend>");
        for (status, text) in Status::LEGEND {
            let _ = write!(out, "<li class=\"{}\">{text}</li>", status.class());
        }
        let _ = writeln!(
            out,
            "<li class=\"{}\">a note from the marks file</li></ul>",
            Status::Note.class()
        );
    }

    let unmatched: Vec<_> = marks
        .entries
        .iter()
        .filter(|(.., used)| !used.get())
        .collect();
    if !unmatched.is_empty() {
        let _ = writeln!(
            out,
            "<details>\n<summary>Marks that match no cell ({})</summary>\n<ul>",
            unmatched.len()
        );
        for (key, note, status, _) in unmatched {
            let _ = writeln!(
                out,
                "<li><code class=\"{}\">{}</code>: {}</li>",
                status.class(),
                esc(key),
                esc(note)
            );
        }
        out.push_str("</ul>\n</details>\n");
    }
    out.push_str("</header>\n");

    out.push_str(&body);
    out.push_str("</body>\n</html>\n");
    out
}

/// One manual table: its heading and page render, then its parts stacked
/// in one table, then its notes. The renders of the pages before
/// `next_page` follow the table's own, as its remarks may run onto them.
fn html_section(
    out: &mut String,
    parts: &[SpecSheetTable],
    next_page: Option<u16>,
    pages_dir: Option<&str>,
    marks: &Marks,
) {
    let name = parts[0].name;
    let key = |field: &str| vec![format!("{name} / {field}")];
    let mut pages: Vec<u16> = parts.iter().map(|t| t.page).collect();
    pages.dedup();
    let page = match pages.as_slice() {
        [p] => format!("PDF p. {p}"),
        many => format!(
            "PDF pp. {}",
            many.iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    let _ = write!(
        out,
        "<section>\n<h2><span{}>{}</span> <span class=page-no><span{}>{}</span></span></h2>\n\
         <div class=cols>\n",
        mark_attr(&marks.find(&key("table"))),
        esc(name),
        mark_attr(&marks.find(&key("page"))),
        page,
    );
    if let Some(dir) = pages_dir {
        let mut images: Vec<(u16, String)> =
            pages.iter().map(|&p| (p, page_image(dir, p))).collect();
        if let (Some(&last), Some(next)) = (pages.last(), next_page) {
            images.extend((last + 1..next).filter_map(|p| Some((p, existing_page_image(dir, p)?))));
        }
        for (p, img) in images {
            let img = esc(&img);
            let _ = writeln!(
                out,
                "<div class=img><a href=\"{img}\"><img src=\"{img}\" alt=\"PDF page {p}\"></a></div>"
            );
        }
    }
    out.push_str(
        "<div class=tbl>\n<table>\n<thead><tr><th>Range</th><th>Resolution</th>\
         <th>Accuracy band</th><th>Accuracy</th><th>Input impedance</th><th>Overload</th>\
         </tr></thead>\n",
    );
    for part in parts {
        html_part(out, part, marks);
    }
    out.push_str("</table>\n");

    // The manual prints one set of remarks under the whole table.
    let mut notes: Vec<&str> = Vec::new();
    for note in parts.iter().flat_map(|t| t.mode.notes) {
        if !notes.contains(note) {
            notes.push(note);
        }
    }
    let _ = write!(out, "<ul{}>", mark_attr(&marks.find(&key("notes"))));
    if notes.is_empty() {
        out.push_str("<li>no notes</li>");
    }
    for note in notes {
        let _ = write!(out, "<li>{}</li>", esc(note));
    }
    out.push_str("</ul>\n</div>\n</div>\n</section>\n");
}

/// One part of a table as the manual prints it: a range spans its accuracy
/// bands' sub-rows, consecutive single-band rows with the same accuracy
/// share one cell, and impedance and overload, which hold for the whole
/// part, one cell each.
fn html_part(out: &mut String, t: &SpecSheetTable, marks: &Marks) {
    let key = |parts: &[&str]| vec![parts.join(" / ")];
    out.push_str("<tbody>\n");

    // Marks on a part-wide cell: the table's own, and every row's.
    let whole = |field: &str| {
        let keys: Vec<String> = std::iter::once(format!("{} / {field}", t.name))
            .chain(
                t.rows
                    .iter()
                    .map(|r| format!("{} / {} / {field}", t.name, r.label)),
            )
            .collect();
        marks.find(&keys)
    };
    let impedance = t.mode.input_impedance.unwrap_or("—");
    let overload = t.mode.overload_protection.unwrap_or("—");
    let (impedance_mark, overload_mark) = (whole("input_impedance"), whole("overload_protection"));

    let rows: Vec<&SpecSheetRow> = t.rows.iter().collect();
    let rows = &rows[..];
    if rows.is_empty() {
        out.push_str("<tr><td colspan=4>no ranges</td>");
        td(out, 1, impedance, &impedance_mark);
        td(out, 1, overload, &overload_mark);
        out.push_str("</tr>\n");
    }
    let sub_rows = |r: &&SpecSheetRow| r.spec.accuracy.len().max(1);
    let total: usize = rows.iter().map(sub_rows).sum();
    let spans = accuracy_spans(rows);
    // A table-wide mark on the accuracy or the ranges lands on every such
    // cell of the table.
    let accuracy_mark = |rows: &[&SpecSheetRow]| {
        let keys: Vec<String> = std::iter::once(format!("{} / accuracy", t.name))
            .chain(
                rows.iter()
                    .map(|r| format!("{} / {} / accuracy", t.name, r.label)),
            )
            .collect();
        marks.find(&keys)
    };
    for (i, r) in rows.iter().enumerate() {
        let n = sub_rows(r);
        let bands = r.spec.accuracy;
        for j in 0..n {
            out.push_str("<tr>");
            if j == 0 {
                let row_mark = marks.find(&[
                    format!("{} / {} / row", t.name, r.label),
                    format!("{} / {} / range", t.name, r.label),
                    format!("{} / range", t.name),
                    format!("{} / ranges", t.name),
                ]);
                let res_mark = marks.find(&key(&[t.name, r.label, "resolution"]));
                td(out, n, r.label, &row_mark);
                td(out, n, r.spec.resolution, &res_mark);
            }
            match (bands.len(), spans[i]) {
                (0, _) => {
                    let mark = accuracy_mark(&rows[i..=i]);
                    td(out, 1, "", &mark);
                    td(out, 1, "not specified", &mark);
                }
                // Continues the merged cell of a row above.
                (1, 0) => {}
                (1, span) => {
                    let mark = accuracy_mark(&rows[i..i + span]);
                    td(out, span, bands[0].freq_range.unwrap_or(""), &mark);
                    td(out, span, &accuracy_text(&bands[0]), &mark);
                }
                _ => {
                    let mark = accuracy_mark(&rows[i..=i]);
                    td(out, 1, bands[j].freq_range.unwrap_or(""), &mark);
                    td(out, 1, &accuracy_text(&bands[j]), &mark);
                }
            }
            if i == 0 && j == 0 {
                td(out, total, impedance, &impedance_mark);
                td(out, total, overload, &overload_mark);
            }
            out.push_str("</tr>\n");
        }
    }
    out.push_str("</tbody>\n");
}

/// For each row with a single accuracy band: how many rows, from it down,
/// share that band and accuracy (0 for a row inside such a run). 1 for the
/// other rows.
fn accuracy_spans(rows: &[&SpecSheetRow]) -> Vec<usize> {
    let single = |r: &SpecSheetRow| match r.spec.accuracy {
        [band] => Some((band.freq_range, band.accuracy)),
        _ => None,
    };
    let mut spans = vec![1; rows.len()];
    let mut i = 0;
    while i < rows.len() {
        let mut end = i + 1;
        if let Some(band) = single(rows[i]) {
            while end < rows.len() && single(rows[end]) == Some(band) {
                spans[end] = 0;
                end += 1;
            }
            spans[i] = end - i;
        }
        i = end;
    }
    spans
}

fn accuracy_text(band: &AccuracyBand) -> String {
    format!("\u{00b1}({})", band.accuracy)
}
