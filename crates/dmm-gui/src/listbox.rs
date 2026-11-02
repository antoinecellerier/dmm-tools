//! The open-list widgets the readout selectors, the language and device
//! pickers, the graph menu and the export menu share: an entry marked when
//! it is the live one, a listbox popup that takes focus, and arrow-key
//! navigation over the entries.

use eframe::egui::{
    ComboBox, Context, EventFilter, FocusDirection, IdSalt, Key, Modifiers, Popup, Response, Ui,
};

/// Marks the live entry in a readout selector's popup, in text as well as in
/// the selection colour.
pub(crate) const LIVE_CHOICE_MARK: &str = "\u{25CF}";

/// An entry of an open choice list: `text`, marked when it is the live one.
pub(crate) fn choice_entry_text(text: &str, live: bool) -> String {
    // Three spaces sit close enough under the mark to keep the entries
    // aligned without a figure space the bundled fonts may not have.
    if live {
        format!("{LIVE_CHOICE_MARK} {text}")
    } else {
        format!("   {text}")
    }
}

/// A `ComboBox` whose open list behaves as a native listbox, for every
/// dropdown that lists choices: the readouts' and the Settings device list.
///
/// `combo` styles the box (`selected_text`, width, height); `add_entries`
/// draws the list and returns what was picked this frame. It is handed
/// whether the list was already open last frame, false on the frame it
/// opens, which is when it moves focus onto an entry; arrow keys between
/// entries are its own (`navigate_choice_entries`).
///
/// `layout` names the list's shape, such as its column count, and is asked
/// only while the list is open: egui sizes a popup on the frame it opens and
/// only ever widens it after, so when the shape changes under an open list,
/// the list closes for a frame and opens again at its new size.
///
/// This owns the rest. Esc and Tab close the list without a pick, a pick
/// closes it, and focus returns to the box on every close but a click
/// elsewhere. The mechanisms are the ones `color_edit` uses for its picker:
/// a was-open flag to see the open and close transitions, consumed keys plus
/// a cancelled focus move, and a focus lock filter on the entry.
pub(crate) fn listbox_dropdown<'a, R>(
    ui: &mut Ui,
    id_salt: &'static str,
    layout: impl Fn() -> usize,
    combo: impl FnOnce(ComboBox<'a>) -> ComboBox<'a>,
    add_entries: impl FnOnce(&mut Ui, bool) -> Option<R>,
) -> (Response, Option<R>) {
    let ctx = ui.ctx().clone();
    // The id `ComboBox::from_id_salt` derives below, known up front so the
    // list's state can be read before the box is drawn. The salt is wrapped
    // in `IdSalt::new` the way `from_id_salt` wraps it: hashing the bare
    // string, or an `Id`, gives a different id.
    let button_id = ui.make_persistent_id(IdSalt::new(id_salt));
    let was_open_key = button_id.with("was_open");
    let was_open: bool = ctx.data(|d| d.get_temp(was_open_key)).unwrap_or(false);

    // The popup's id as `ComboBox` derives it from the box's (its private
    // `widget_to_popup_id`, behind `ComboBox::is_open`).
    let popup_id = button_id.with("popup");
    let layout_key = button_id.with("layout");
    let reopen_key = button_id.with("reopen");
    if ctx.data_mut(|d| d.remove_temp::<bool>(reopen_key)) == Some(true) {
        Popup::open_id(&ctx, popup_id);
    }
    // Keys that leave the list, handled before it is drawn so this frame
    // already shows it closed: Tab and Shift+Tab step out of a listbox
    // rather than through it, Esc abandons it. `consume_key` drops the press;
    // `move_focus(None)` cancels the focus jump egui queued from it as the
    // frame began.
    if was_open {
        let leave = ctx.input_mut(|i| {
            i.consume_key(Modifiers::NONE, Key::Tab)
                | i.consume_key(Modifiers::SHIFT, Key::Tab)
                | i.consume_key(Modifiers::NONE, Key::Escape)
        });
        if leave {
            Popup::close_all(&ctx);
            ctx.memory_mut(|m| m.move_focus(FocusDirection::None));
        }
        // A list left this frame stays closed, whatever its shape did.
        let shape = layout();
        let last: Option<usize> = ctx.data(|d| d.get_temp(layout_key));
        ctx.data_mut(|d| d.insert_temp(layout_key, shape));
        if !leave && last.is_some_and(|last| last != shape) {
            Popup::close_id(&ctx, popup_id);
            ctx.data_mut(|d| d.insert_temp(reopen_key, true));
        }
    }

    let mut picked = None;
    // No `set_modal_layer` here, unlike `color_edit`: Tab and the arrows are
    // handled outright, and the modal layer outlives the list by a frame, in
    // which egui surrenders the focus just handed back to the box
    // (`Context::create_widget` on a layer below the modal).
    let inner =
        combo(ComboBox::from_id_salt(id_salt)).show_ui(ui, |ui| picked = add_entries(ui, was_open));

    // Enter/Space "clicks" the focused entry without a pointer click, which
    // is the only thing a menu popup closes on by itself.
    let activated = picked.is_some();
    if activated {
        Popup::close_all(&ctx);
    }
    let is_open = ComboBox::is_open(&ctx, button_id);
    if is_open && !was_open {
        // The shape it opened at, for the next frame to compare against.
        ctx.data_mut(|d| d.insert_temp(layout_key, layout()));
    }
    // A click outside the list that closed it may have landed on another
    // widget, which took the focus as it was drawn; that click is the user's
    // choice of focus. A click on an entry is `activated` and not
    // "elsewhere" in this sense.
    let clicked_away = inner.response.clicked_elsewhere() && !activated;
    if was_open && !is_open && !clicked_away {
        // Closed by pick, Esc or Tab: focus goes back to the box rather than
        // to the top of the Tab order.
        ctx.memory_mut(|m| m.request_focus(button_id));
    }
    ctx.data_mut(|d| d.insert_temp(was_open_key, is_open));
    (inner.response, picked)
}

/// Keyboard navigation inside an open readout list: ArrowDown/ArrowUp move
/// focus between the entries, clamped at the ends; Home/End jump to the
/// first and last. Nothing reaches the meter until Enter, Space or a click
/// picks the focused entry.
pub(crate) fn navigate_choice_entries(ctx: &Context, entries: &[Response]) {
    let Some(i) = entries.iter().position(Response::has_focus) else {
        return;
    };
    let last = entries.len() - 1;
    let pressed = |key| ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key));
    let target = if pressed(Key::ArrowDown) {
        (i + 1).min(last)
    } else if pressed(Key::ArrowUp) {
        i.saturating_sub(1)
    } else if pressed(Key::Home) {
        0
    } else if pressed(Key::End) {
        last
    } else {
        i
    };
    // egui queued its own spatial move from the same press (and would move
    // on Left/Right too); cancel it so the list's is the only one.
    ctx.memory_mut(|m| m.move_focus(FocusDirection::None));
    if target != i {
        entries[target].request_focus();
    }
    // From the next frame on, arrows, Tab and Esc are delivered to the
    // focused entry instead of moving focus. The filter only binds once the
    // entry has held focus for a frame, which is what the reset above covers.
    ctx.memory_mut(|m| {
        m.set_focus_lock_filter(
            entries[i].id,
            EventFilter {
                tab: true,
                vertical_arrows: true,
                escape: true,
                ..Default::default()
            },
        );
    });
}
