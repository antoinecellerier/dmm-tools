use dmm_lib::protocol::registry;
use eframe::egui::{self, RichText, Ui};

use crate::a11y::ResponseA11yExt;

/// The most columns the **Device** list spreads over: at four the tallest
/// is UNI-T's, and more would only leave short columns side by side.
const MAX_DEVICE_COLUMNS: usize = 4;

/// The fewest rows the **Device** list keeps below its button before it
/// lets egui slide it up over the row instead.
const MIN_DEVICE_LIST_ROWS: f32 = 8.0;

/// Splits groups `heights` tall, kept whole and in order, into at most `k`
/// columns, the tallest as short as it can be, then the next tallest, and so
/// on: one tall group sets the tallest column alone, and the rest still
/// spread evenly. Returns each column's range of groups; a tie goes to the
/// split whose earlier columns are taller, so the eye reads down the long
/// ones first.
fn pack_columns(heights: &[usize], k: usize) -> Vec<std::ops::Range<usize>> {
    /// Every way to cut `n` groups into `k` non-empty runs, as the index
    /// each run after the first starts at.
    fn cuts(from: usize, n: usize, k: usize, acc: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if k == 1 {
            out.push(acc.clone());
            return;
        }
        for cut in from + 1..=n - (k - 1) {
            acc.push(cut);
            cuts(cut, n, k - 1, acc, out);
            acc.pop();
        }
    }
    let n = heights.len();
    let k = k.clamp(1, n.max(1));
    let mut splits = Vec::new();
    cuts(0, n, k, &mut Vec::new(), &mut splits);
    let ranges = |cuts: &[usize]| -> Vec<std::ops::Range<usize>> {
        let starts = std::iter::once(0).chain(cuts.iter().copied());
        let ends = cuts.iter().copied().chain(std::iter::once(n));
        starts.zip(ends).map(|(a, b)| a..b).collect()
    };
    // The column heights, tallest first, compared in that order.
    let tallest_first = |ranges: &[std::ops::Range<usize>]| {
        let mut columns: Vec<usize> = ranges
            .iter()
            .map(|r| heights[r.clone()].iter().sum::<usize>())
            .collect();
        columns.sort_unstable_by(|a, b| b.cmp(a));
        columns
    };
    splits
        .iter()
        .map(|cuts| ranges(cuts))
        // Reversed, the latest cuts come first, and `min_by_key` keeps the
        // first of equals: the taller columns lead.
        .rev()
        .min_by_key(|ranges| tallest_first(ranges))
        .unwrap_or_default()
}

/// A brand and its entries, each with its description: its full name and
/// how far its support is verified, what its hover says and a screen reader
/// announces.
type Group = (registry::Brand, Vec<(registry::Listing, String)>);

/// The brand groups the list shows: the registry's, which never change. The
/// descriptions are built here, once, since reading a level builds a
/// protocol; the tag is `--help`'s, so the two binaries agree.
static GROUPS: std::sync::LazyLock<Vec<Group>> = std::sync::LazyLock::new(|| {
    registry::brand_groups()
        .into_iter()
        .map(|(brand, rows)| {
            let rows = rows
                .into_iter()
                .map(|l| {
                    let name = l.display_name();
                    let described = dmm_shared::help::device_tag(l.device, true)
                        .map_or_else(|| name.to_string(), |tag| format!("{name} ({tag})"));
                    (l, described)
                })
                .collect();
            (brand, rows)
        })
        .collect()
});

/// How the open **Device** list lays out: the brand groups and the columns
/// they are spread over, as many as the window has room for.
struct DeviceListLayout {
    groups: &'static [Group],
    columns: Vec<std::ops::Range<usize>>,
    col_w: f32,
    gap: f32,
}

/// The font and spacing the list's column width is measured with, at the
/// scale it is measured at: what the width kept between frames is keyed on.
#[derive(Clone, PartialEq)]
struct MeasuredWith {
    font: egui::FontId,
    pixels_per_point: f32,
    padding: f32,
}

impl MeasuredWith {
    fn of(ctx: &egui::Context, style: &egui::Style) -> Self {
        Self {
            font: egui::TextStyle::Button.resolve(style),
            pixels_per_point: ctx.pixels_per_point(),
            padding: 2.0 * style.spacing.button_padding.x,
        }
    }
}

/// Where egui's memory keeps the column width and what it was measured with.
fn width_id() -> egui::Id {
    egui::Id::unique("device-list-width")
}

impl DeviceListLayout {
    /// The layout for this frame. Only the column width is kept between
    /// frames, in egui's memory: measuring every name is the costly part,
    /// and it changes only with the font, scale and spacing. The columns
    /// follow the window's width every frame.
    fn current(ctx: &egui::Context, style: &egui::Style) -> Self {
        let with = MeasuredWith::of(ctx, style);
        let col_w = match ctx.data(|d| d.get_temp::<(MeasuredWith, f32)>(width_id())) {
            Some((kept, col_w)) if kept == with => col_w,
            _ => {
                let col_w = Self::measure(ctx, &with);
                ctx.data_mut(|d| d.insert_temp(width_id(), (with, col_w)));
                col_w
            }
        };
        let gap = style.spacing.item_spacing.x * 3.0;
        // The menu's frame and the room egui keeps from the window's edge.
        let margins = 4.0 * style.spacing.menu_margin.sum().x;
        let k = (((ctx.content_rect().width() - margins + gap) / (col_w + gap)).floor() as usize)
            .clamp(1, MAX_DEVICE_COLUMNS);
        let heights: Vec<usize> = GROUPS.iter().map(|(_, d)| d.len() + 1).collect();
        Self {
            groups: &GROUPS,
            columns: pack_columns(&heights, k),
            col_w,
            gap,
        }
    }

    /// Column width: the widest heading or marked entry, plus a selectable's
    /// padding. Measured in the font the list draws in.
    fn measure(ctx: &egui::Context, with: &MeasuredWith) -> f32 {
        let measure = |text: &str| {
            ctx.fonts_mut(|f| {
                f.layout_no_wrap(text.to_string(), with.font.clone(), egui::Color32::WHITE)
                    .size()
                    .x
            })
        };
        GROUPS
            .iter()
            .flat_map(|(brand, rows)| {
                let entries = rows.iter().map(|(d, _)| {
                    measure(&crate::display::choice_entry_text(d.model_name(), true)) + with.padding
                });
                std::iter::once(measure(brand.name())).chain(entries)
            })
            .fold(0.0, f32::max)
            .ceil()
    }
}

/// The **Device** dropdown: the selected meter on the box, and in the open
/// list Auto-detect over every meter under its brand, the brands spread over
/// as many columns as the window has room for. Returns the id picked this
/// frame, [`registry::AUTO_DEVICE_ID`] for Auto-detect.
///
/// Keyboard as in every choice list ([`listbox_dropdown`]), plus Left/Right
/// to the nearest entry of the next column over. The list's layout is worked
/// out only while it is open: the closed box is drawn every frame the
/// Settings panel is.
pub(super) fn device_dropdown(ui: &mut Ui, selected_id: &str, label: &str) -> Option<&'static str> {
    use crate::display::{choice_entry_text, listbox_dropdown, navigate_choice_entries};
    use egui::{Align, ComboBox, Layout, TextWrapMode, vec2};

    let ctx = ui.ctx().clone();
    let style = ui.style().clone();
    let list = std::cell::OnceCell::new();
    let list = || list.get_or_init(|| DeviceListLayout::current(&ctx, &style));

    // Below the button and no further: a list taller than that scrolls,
    // where egui would slide it up over the button.
    let row = ui.spacing().interact_size.y;
    let below = ctx.content_rect().bottom()
        - ui.cursor().top()
        - row
        - 2.0 * ui.spacing().menu_margin.sum().y;
    let height = below.max(MIN_DEVICE_LIST_ROWS * row);

    let shape = || list().columns.len();
    // Called through the type: a `combo: ComboBox` annotation makes the
    // closure generic over the box's lifetime, which egui 0.37's borrowed
    // text no longer satisfies. On its own row, the box would otherwise widen
    // past a narrow panel for a long name; the hover text gives it whole.
    let combo = |combo| {
        ComboBox::selected_text(combo, label)
            .wrap_mode(TextWrapMode::Truncate)
            .height(height)
    };
    let (response, picked) = listbox_dropdown(ui, "device", shape, combo, |ui, was_open| {
        let DeviceListLayout {
            groups,
            columns,
            col_w,
            gap,
        } = list();
        let mut picked = None;
        // Every entry in reading order, and the column each sits in (none
        // for Auto-detect, which spans them).
        let mut entries = Vec::with_capacity(registry::listings().count() + 1);
        let mut add = |ui: &mut Ui, id: &'static str, text: &str, full: &str, hover: &str| {
            let live = id == selected_id;
            let entry = ui
                .selectable_label(live, choice_entry_text(text, live))
                .on_hover_text(hover)
                .a11y_label(full);
            if !was_open && live {
                entry.request_focus();
            }
            if entry.gained_focus() {
                entry.scroll_to_me(None);
            }
            if entry.clicked() {
                picked = Some(id);
            }
            entry
        };
        let auto = add(
            ui,
            registry::AUTO_DEVICE_ID,
            "Auto-detect",
            "Auto-detect",
            dmm_shared::help::AUTO_DESCRIPTION,
        );
        entries.push((auto, None));
        ui.add_space(ui.spacing().item_spacing.y);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = *gap;
            for (c, range) in columns.iter().enumerate() {
                let layout = Layout::top_down_justified(Align::LEFT);
                ui.allocate_ui_with_layout(vec2(*col_w, 0.0), layout, |ui| {
                    for (i, (brand, rows)) in groups[range.clone()].iter().enumerate() {
                        if i > 0 {
                            ui.add_space(ui.spacing().item_spacing.y * 2.0);
                        }
                        ui.label(RichText::new(brand.name()).color(ui.visuals().weak_text_color()));
                        for (row, described) in rows {
                            let entry = add(ui, row.id(), row.model_name(), described, described);
                            entries.push((entry, Some(c)));
                        }
                    }
                });
            }
        });
        navigate_columns(ui.ctx(), &entries);
        let responses: Vec<_> = entries.into_iter().map(|(r, _)| r).collect();
        navigate_choice_entries(ui.ctx(), &responses);
        picked
    });
    response.on_hover_text(label).a11y_label("Device");
    picked
}

/// Left/Right in the open **Device** list: focus moves to the entry of the
/// next column over nearest the focused one's height. Up/Down and the rest
/// are [`navigate_choice_entries`]'s, run after this.
fn navigate_columns(ctx: &egui::Context, entries: &[(egui::Response, Option<usize>)]) {
    let Some((focused, Some(column))) = entries
        .iter()
        .find(|(r, _)| r.has_focus())
        .map(|(r, c)| (r, *c))
    else {
        return;
    };
    let pressed = |key| ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
    let target = if pressed(egui::Key::ArrowRight) {
        column + 1
    } else if pressed(egui::Key::ArrowLeft) && column > 0 {
        column - 1
    } else {
        return;
    };
    let y = focused.rect.center().y;
    let nearest = entries
        .iter()
        .filter(|(_, c)| *c == Some(target))
        .min_by(|(a, _), (b, _)| {
            let da = (a.rect.center().y - y).abs();
            let db = (b.rect.center().y - y).abs();
            da.total_cmp(&db)
        });
    if let Some((entry, _)) = nearest {
        entry.request_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The layout a frame of a `width`-wide window gets, with a frame run
    /// first so a zoom set before it has settled.
    fn layout_at(ctx: &egui::Context, width: f32) -> DeviceListLayout {
        let mut layout = None;
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 800.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                layout = Some(DeviceListLayout::current(ui.ctx(), ui.style()));
            });
            out.textures_delta.clear();
        }
        layout.expect("the frame ran")
    }

    /// The width measured once is what later frames use, while the font,
    /// scale and spacing stay put; a zoom measures it again. The columns
    /// follow the window either way.
    #[test]
    fn the_measured_width_is_kept_until_the_zoom_moves() {
        let ctx = crate::app::styled_context();
        let wide = layout_at(&ctx, 1600.0);
        // A kept width stands in for measuring: prove it is the one read.
        let (with, _) = ctx
            .data(|d| d.get_temp::<(MeasuredWith, f32)>(width_id()))
            .expect("the width is kept");
        ctx.data_mut(|d| d.insert_temp(width_id(), (with, 1000.0_f32)));
        assert_eq!(layout_at(&ctx, 1600.0).col_w, 1000.0);
        assert_eq!(
            layout_at(&ctx, 1600.0).columns.len(),
            1,
            "one 1000 pt column fits"
        );

        ctx.set_pixels_per_point(1.5);
        let zoomed = layout_at(&ctx, 1600.0);
        assert_ne!(zoomed.col_w, 1000.0, "a zoom measures the width again");
        assert!(wide.columns.len() > layout_at(&ctx, 500.0).columns.len());
    }

    /// Every entry says how far it is verified, in `--help`'s words, and a
    /// meter sold under another name shares its entry's level.
    #[test]
    fn each_entry_names_its_support_level() {
        let described = |id: &str| {
            GROUPS
                .iter()
                .flat_map(|(_, rows)| rows)
                .find(|(l, _)| l.id() == id)
                .map(|(_, d)| d.as_str())
        };
        assert_eq!(described("ut61eplus"), Some("UNI-T UT61E+ (verified)"));
        assert_eq!(described("ut181a"), Some("UNI-T UT181A (partly verified)"));
        assert_eq!(described("ut8803"), Some("UNI-T UT8803 (experimental)"));
        assert_eq!(
            described("et2517a"),
            Some("Minipa ET-2517A (partly verified)")
        );
        assert_eq!(
            described("mock"),
            Some("Mock UT61E+ (no hardware required)")
        );
    }

    #[test]
    fn device_columns_keep_groups_whole_and_balance_the_tallest() {
        // Brymen, EEVblog, OWON, UNI-T, Voltcraft, ZOTEK, Simulated, each
        // with its heading.
        let heights = [5, 2, 12, 17, 9, 5, 3];
        assert_eq!(pack_columns(&heights, 1), vec![0..7]);
        assert_eq!(pack_columns(&heights, 4), vec![0..2, 2..3, 3..4, 4..7]);
        // Three: UNI-T alone cannot be beaten, so the split puts the rest
        // either side of it.
        assert_eq!(pack_columns(&heights, 3), vec![0..3, 3..4, 4..7]);
        // Today's brands, ANENG to Simulated: UNI-T sets the tallest column
        // alone, and Minipa goes beside OWON rather than piling up first.
        let heights = [5, 5, 4, 2, 3, 12, 19, 9, 5, 3];
        assert_eq!(pack_columns(&heights, 4), vec![0..4, 4..6, 6..7, 7..10]);
        assert_eq!(pack_columns(&heights, 3), vec![0..6, 6..7, 7..10]);
        // More columns than groups: one group each.
        assert_eq!(pack_columns(&[1, 1], 4), vec![0..1, 1..2]);
        // A tie goes to the taller columns first.
        assert_eq!(pack_columns(&[1, 1, 1], 2), vec![0..2, 2..3]);
    }
}
