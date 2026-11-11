//! Marks under what a text field holds that won't be used as typed: a
//! squiggle in the error colour under an entry that doesn't read, held back
//! while the caret is in it, and grey text dotted under one kept for another
//! quantity than the one on screen. The graph's reference and Y fields and
//! the Alarm and Scale rows share them, so one look means one thing.

use std::ops::Range;

use eframe::egui::{self, Ui};

/// How an entry is marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkStyle {
    /// It doesn't read, or is refused: a squiggle in the error colour. A
    /// half-typed entry reads as none, so it is held back while the caret
    /// is in it.
    Unread,
    /// It reads, but for another quantity than the one on screen, and is
    /// kept for when that one comes back: greyed and dotted under.
    Waiting,
}

/// One marked entry: where it is in the text, and how it is marked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextMark {
    pub(crate) span: Range<usize>,
    pub(crate) style: MarkStyle,
}

impl TextMark {
    /// The whole of `text`, the spaces around it aside, marked `style`: a
    /// field holding one entry. `None` for a blank one.
    pub(crate) fn whole(text: &str, style: MarkStyle) -> Option<Self> {
        let start = text.find(|c: char| !c.is_whitespace())?;
        let end = text.trim_end().len();
        Some(Self {
            span: start..end,
            style,
        })
    }

    /// Whether to say nothing of it yet: it may be half typed, and the
    /// `caret` (a byte offset into the text, while it is typed in) is in it
    /// or right after it.
    pub(crate) fn held_back(&self, caret: Option<usize>) -> bool {
        self.style == MarkStyle::Unread
            && caret.is_some_and(|c| self.span.start <= c && c <= self.span.end)
    }
}

/// Where the caret is in `text`, as a byte offset, while the edit has the
/// focus.
pub(crate) fn caret(output: &egui::text_edit::TextEditOutput, text: &str) -> Option<usize> {
    output
        .cursor_range
        .filter(|_| output.response.has_focus())
        .map(|r| {
            text.char_indices()
                .nth(r.primary.index.0)
                .map_or(text.len(), |(b, _)| b)
        })
}

/// `text` laid out as a marked field shows it, in `font_id`: an entry
/// [`MarkStyle::Waiting`] greyed in `weak`. The rest takes the edit's own
/// colour (`PLACEHOLDER`) and line height, as egui's own single-line layout
/// gives them.
pub(crate) fn marked_job(
    ui: &Ui,
    font_id: &egui::FontId,
    text: &str,
    marks: &[TextMark],
    weak: egui::Color32,
) -> egui::text::LayoutJob {
    let line_height =
        ui.fonts_mut(|f| f.row_height(font_id)) + ui.spacing().extra_text_line_spacing;
    let format = |color| egui::TextFormat {
        font_id: font_id.clone(),
        color,
        line_height: Some(line_height),
        ..Default::default()
    };
    // Left-aligned, as egui's own: the edit scrolls to the caret by its
    // place in the galley, which another alignment moves.
    let mut job = egui::text::LayoutJob {
        keep_trailing_whitespace: true,
        ..Default::default()
    };
    let mut at = 0;
    for m in marks.iter().filter(|m| m.style == MarkStyle::Waiting) {
        job.append(
            &text[at..m.span.start],
            0.0,
            format(egui::Color32::PLACEHOLDER),
        );
        job.append(&text[m.span.clone()], 0.0, format(weak));
        at = m.span.end;
    }
    job.append(&text[at..], 0.0, format(egui::Color32::PLACEHOLDER));
    job
}

/// Underline the marked entries of a field shown as `output`: a squiggle in
/// `error` under one [`MarkStyle::Unread`], dots in `weak` under one
/// [`MarkStyle::Waiting`].
pub(crate) fn underline(
    ui: &Ui,
    output: &egui::text_edit::TextEditOutput,
    text: &str,
    marks: &[TextMark],
    error: egui::Color32,
    weak: egui::Color32,
) {
    let galley = &output.galley;
    if galley.text() != text {
        return;
    }
    let Some(row) = galley.rows.first() else {
        return;
    };
    let origin = output.galley_pos - egui::vec2(galley.rect.left(), 0.0) + row.pos.to_vec2();
    // From the ink of the entry's first glyph to that of its last, rather
    // than from caret to caret: a caret stop sits a side bearing away from
    // the ink, and the line looked to start under the space before.
    let ink = |span: &Range<usize>| {
        let first = text[..span.start].chars().count();
        let glyphs = row
            .row
            .glyphs
            .get(first..first + text[span.clone()].chars().count())?;
        let left = |g: &egui::epaint::text::Glyph| g.pos.x + g.uv_rect.offset.x;
        let (start, end) = (glyphs.first()?, glyphs.last()?);
        Some((
            origin.x + left(start),
            origin.x + left(end) + end.uv_rect.size.x,
        ))
    };
    let y = origin.y + row.row.size.y - 1.5;
    let painter = ui.painter_at(output.text_clip_rect);
    for m in marks {
        let Some((start, end)) = ink(&m.span) else {
            continue;
        };
        match m.style {
            MarkStyle::Unread => {
                let mut points = Vec::new();
                let mut x = start;
                let mut up = true;
                while x < end {
                    points.push(egui::pos2(x, if up { y - 1.0 } else { y + 1.0 }));
                    x += 2.0;
                    up = !up;
                }
                points.push(egui::pos2(end, y));
                painter.add(egui::Shape::line(points, egui::Stroke::new(1.0, error)));
            }
            MarkStyle::Waiting => {
                let line = [egui::pos2(start, y), egui::pos2(end, y)];
                painter.extend(egui::Shape::dotted_line(&line, weak, 3.0, 0.75));
            }
        }
    }
}

/// What a field holding one entry says of it: how it is marked, and the
/// hover line saying why. `None` when it reads and is used.
pub(crate) type Judge<'a> = &'a dyn Fn(&str) -> Option<(MarkStyle, String)>;

/// Show a single-line edit of `text`, a field holding one entry, set up by
/// `edit`, in `font_id`, its entry marked as `judge` says, with `error` the
/// squiggle's colour. Gives the edit's output and the hover line saying why
/// its entry is marked, when it is and the mark isn't held back.
pub(crate) fn show_judged(
    ui: &mut Ui,
    text: &mut String,
    edit: impl for<'t> FnOnce(egui::TextEdit<'t>) -> egui::TextEdit<'t>,
    font_id: &egui::FontId,
    error: egui::Color32,
    judge: Judge<'_>,
) -> (egui::text_edit::TextEditOutput, Option<String>) {
    let weak = ui.visuals().weak_text_color();
    let mark = |text: &str| {
        let (style, why) = judge(text)?;
        Some((TextMark::whole(text, style)?, why))
    };
    let mut layouter = |ui: &Ui, buffer: &dyn egui::TextBuffer, _wrap: f32| {
        let text = buffer.as_str();
        let marks: Vec<TextMark> = mark(text).into_iter().map(|(m, _)| m).collect();
        // Built before the fonts are locked: it measures a row with them.
        let job = marked_job(ui, font_id, text, &marks, weak);
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let output = edit(egui::TextEdit::singleline(text))
        .layouter(&mut layouter)
        .show(ui);
    let text = output.galley.text().to_string();
    let caret = caret(&output, &text);
    let shown = mark(&text).filter(|(m, _)| !m.held_back(caret));
    let marks: Vec<TextMark> = shown.iter().map(|(m, _)| m.clone()).collect();
    underline(ui, &output, &text, &marks, error, weak);
    (output, shown.map(|(_, why)| why))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_whole_field_is_its_entry_without_the_spaces() {
        assert_eq!(
            TextMark::whole("  5 x ", MarkStyle::Unread).map(|m| m.span),
            Some(2..5)
        );
        assert_eq!(TextMark::whole("   ", MarkStyle::Unread), None);
    }

    /// An entry that doesn't read is held back while the caret is in it or
    /// right after it; one waiting for its quantity never is.
    #[test]
    fn only_an_unread_entry_is_held_back_under_the_caret() {
        let unread = TextMark::whole("5 x", MarkStyle::Unread).unwrap();
        assert!(unread.held_back(Some(3)));
        assert!(!unread.held_back(None));
        let waiting = TextMark::whole("5 V", MarkStyle::Waiting).unwrap();
        assert!(!waiting.held_back(Some(3)));
    }
}
