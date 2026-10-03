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
/// columns, the tallest as short as it can be. Returns each column's range
/// of groups; a tie goes to the split whose earlier columns are taller, so
/// the eye reads down the long ones first.
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
    let tallest = |ranges: &[std::ops::Range<usize>]| {
        ranges
            .iter()
            .map(|r| heights[r.clone()].iter().sum::<usize>())
            .max()
            .unwrap_or(0)
    };
    splits
        .iter()
        .map(|cuts| ranges(cuts))
        // Reversed, the latest cuts come first, and `min_by_key` keeps the
        // first of equals: the taller columns lead.
        .rev()
        .min_by_key(|ranges| tallest(ranges))
        .unwrap_or_default()
}

/// How the open **Device** list lays out: the brand groups and the columns
/// they are spread over, as many as the window has room for.
struct DeviceListLayout {
    groups: Vec<(registry::Brand, Vec<&'static registry::SelectableDevice>)>,
    columns: Vec<std::ops::Range<usize>>,
    col_w: f32,
    gap: f32,
}

impl DeviceListLayout {
    fn new(ctx: &egui::Context, style: &egui::Style) -> Self {
        let groups = registry::brand_groups();
        // Column width: the widest heading or marked entry, plus a
        // selectable's padding. Measured in the font the list draws in.
        let font = egui::TextStyle::Button.resolve(style);
        let measure = |text: &str| {
            ctx.fonts_mut(|f| {
                f.layout_no_wrap(text.to_string(), font.clone(), egui::Color32::WHITE)
                    .size()
                    .x
            })
        };
        let padding = 2.0 * style.spacing.button_padding.x;
        let col_w = groups
            .iter()
            .flat_map(|(brand, devices)| {
                let entries = devices.iter().map(|d| {
                    measure(&crate::display::choice_entry_text(d.model_name(), true)) + padding
                });
                std::iter::once(measure(brand.name())).chain(entries)
            })
            .fold(0.0, f32::max)
            .ceil();
        let gap = style.spacing.item_spacing.x * 3.0;
        // The menu's frame and the room egui keeps from the window's edge.
        let margins = 4.0 * style.spacing.menu_margin.sum().x;
        let k = (((ctx.content_rect().width() - margins + gap) / (col_w + gap)).floor() as usize)
            .clamp(1, MAX_DEVICE_COLUMNS);
        let heights: Vec<usize> = groups.iter().map(|(_, d)| d.len() + 1).collect();
        let columns = pack_columns(&heights, k);
        Self {
            groups,
            columns,
            col_w,
            gap,
        }
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
    let list = || list.get_or_init(|| DeviceListLayout::new(&ctx, &style));

    // Below the button and no further: a list taller than that scrolls,
    // where egui would slide it up over the button. egui also caps it at
    // the `default_area_size` its popups are first measured in (400 pt),
    // which four columns fit; a narrower window scrolls within that.
    let row = ui.spacing().interact_size.y;
    let below = ctx.content_rect().bottom()
        - ui.cursor().top()
        - row
        - 2.0 * ui.spacing().menu_margin.sum().y;
    let height = below.max(MIN_DEVICE_LIST_ROWS * row);

    let combo = |combo: ComboBox| {
        combo
            .selected_text(label)
            // On its own row, the box would otherwise widen past a narrow
            // panel for a long name; the hover text gives it whole.
            .wrap_mode(TextWrapMode::Truncate)
            .height(height)
    };
    let shape = || list().columns.len();
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
        let mut entries = Vec::with_capacity(registry::DEVICES.len() + 1);
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
            "Work out which meter is connected",
        );
        entries.push((auto, None));
        ui.add_space(ui.spacing().item_spacing.y);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = *gap;
            for (c, range) in columns.iter().enumerate() {
                let layout = Layout::top_down_justified(Align::LEFT);
                ui.allocate_ui_with_layout(vec2(*col_w, 0.0), layout, |ui| {
                    for (i, (brand, devices)) in groups[range.clone()].iter().enumerate() {
                        if i > 0 {
                            ui.add_space(ui.spacing().item_spacing.y * 2.0);
                        }
                        ui.label(RichText::new(brand.name()).color(ui.visuals().weak_text_color()));
                        for d in devices {
                            let hover = format!("Talk to a {}", d.display_name);
                            let entry = add(ui, d.id, d.model_name(), d.display_name, &hover);
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
        // More columns than groups: one group each.
        assert_eq!(pack_columns(&[1, 1], 4), vec![0..1, 1..2]);
        // A tie goes to the taller columns first.
        assert_eq!(pack_columns(&[1, 1, 1], 2), vec![0..2, 2..3]);
    }
}
