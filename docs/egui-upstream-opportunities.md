# egui upstream opportunities

Catalogue of accessibility, focus, and modal/popup gaps observed while
implementing the `dmm-gui` accessibility pass against
**egui 0.36.2** + **AccessKit 0.24** + **egui_plot 0.37**. Each entry
captures the symptom we hit, the root cause in the egui source (with
`file:line`), and a concrete suggested fix worth raising upstream.

The intent of this file is to make it easy to file focused issues
against [`emilk/egui`](https://github.com/emilk/egui) and
[`emilk/egui_plot`](https://github.com/emilk/egui_plot) one at a time.
Each section is self-contained — copy and paste it into a GitHub issue
and add a minimal repro.

All file references resolve under
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.36.2/src/`
unless otherwise noted.

**Local wrapper layer.** The recurring workaround patterns are
encapsulated as two extension traits in `crates/dmm-gui/src/a11y.rs`:

- `ResponseA11yExt` — chainable `Response::a11y_label(&str)`,
  `Response::a11y_toggled(bool)`, `Response::a11y_role(Role)`. Lets
  call sites attach an AccessKit label, toggle state, or role directly
  onto an existing widget chain (e.g.
  `ui.button("?").on_hover_text("…").a11y_label("Settings")`).
  Workarounds for gaps #2/#3 in `Button` live here.
- `UiA11yExt` — `Ui::landmark(id_salt, role, add_contents)` and
  `Ui::live_region_horizontal(fp, make_label, add_contents)`. Both
  bake in the stable-id-salt workaround for gap #5 and the cached
  fingerprint logic for gap #6.

The chainable / extension-method signatures are deliberately shaped
like the upstream APIs we'd want — when egui ships e.g.
`Button::toggled(bool)` the local wrapper becomes a one-line delegator
and then disappears.

## Submission status

We upstream these to `emilk/egui` **one at a time**, most-impactful-but-
least-disruptive first, to gauge what the maintainers accept before
investing in anything that needs design alignment. This table (and the
`**Status:**` line on each entry below) is the durable resume point — check
it, not chat history.

**Route** = how we open an item:
- **Direct PR** — straightforward additive / opt-in change, obvious shape,
  no behavior change (this is what item #3 was).
- **Issue-first (B)** — *behavioral* change to shared widget/focus/modal
  logic that may be intentional; confirm intent before coding.
- **Issue-first (D)** — new concept / wide *design* space; align on the API
  shape before a PR forces one.

A split is only legitimate when **each half is worthwhile on its own** — not
a trivial carve-off to dodge design alignment. Three items split that way
(suffixed `a`/`b`): **#8** (the `a` half is the substantive fix), and **#4**
and **#6** (the `a` halves are independent doc clarifications that do *not*
fix the underlying gap). **#14** is deliberately *not* split: its two fixes
are mutually-exclusive alternatives, so choosing between them is itself the
design decision and it goes to an Issue whole. #8130 has since merged and
shipped in egui 0.35, so nothing is gated on it any more.

| # | Item | PR / Issue | State | Route |
|---|------|-----------|-------|-------|
| 3 | Button toggled state → AccessKit | [#8130](https://github.com/emilk/egui/pull/8130) | Shipped in egui 0.35 (#8130) | Direct PR ✅ |
| 2 | Button non-Button role (color-well) | — | **Queued next** (gate lifted) | Direct PR |
| 12 | `Memory::with_temp` borrow accessor | — | Not started | Direct PR |
| 15 | `Modal::initial_focus` builder | — | Not started | Direct PR |
| 8a | Add `Context::wants_text_input()` | — | Not started | Direct PR |
| 4a | Doc `accesskit_node_builder` as post-hoc setter | — | Not started | Direct (doc) PR |
| 6a | Doc the re-apply-every-frame contract | — | Not started | Direct (doc) PR |
| 1 | `Role::Label` honours `set_label` | — | Not started | Issue-first (B) |
| 9 | `Focus::begin_pass` arrow snapshot | — | Not started | Issue-first (B) |
| 10 | Bare-Escape clears focus | — | Not started | Issue-first (B) |
| 11 | `set_focus_lock_filter` one-frame hole | — | Not started | Issue-first (B) |
| 13 | `create_widget` surrenders focus under modal | — | Not started | Issue-first (B) |
| 16 | `Modal::should_close` Escape carve-out | — | Not started | Issue-first (B) |
| 8b | Rename `egui_wants_keyboard_input` | — | Not started | Issue-first (B) |
| 14 | `top_modal_layer` one-frame staleness | — | Not started | Issue-first (B) |
| 4b | `accesskit_only` `widget_info` variant | — | Not started | Issue-first (D) |
| 5 | Public landmark helper | — | Not started | Issue-first (D) |
| 6b | Persistent AccessKit label API | — | Not started | Issue-first (D) |
| 7 | `egui_plot::Plot` accessibility | — | Not started | Issue-first (D) |
| 17 | `Popup::modal(true)` | — | Not started | Issue-first (D) |
| 18 | Color-slider keyboard support | — | Not started | Issue-first (D) |
| 19 | `Panel` resize handle a11y | — | Not started | Issue-first (D) |

_Last updated: 2026-09-08._

## Table of contents

- [Accessibility — labels and roles](#accessibility--labels-and-roles)
  - [1. `Role::Label` silently swallows `set_label` overrides](#1-rolelabel-silently-swallows-set_label-overrides)
  - [2. `Button` cannot carry a non-Button AccessKit role](#2-button-cannot-carry-a-non-button-accesskit-role)
  - [3. Toggleable `Button`s cannot announce pressed/not-pressed state](#3-toggleable-buttons-cannot-announce-pressednot-pressed-state)
  - [4. `Response::widget_info` re-dispatch pushes duplicate click events](#4-responsewidget_info-re-dispatch-pushes-duplicate-click-events)
- [Accessibility — landmarks and live regions](#accessibility--landmarks-and-live-regions)
  - [5. No public landmark helper; `ui.scope` ids are unstable](#5-no-public-landmark-helper-uiscope-ids-are-unstable)
  - [6. AccessKit tree is rebuilt every frame, forcing label re-application](#6-accesskit-tree-is-rebuilt-every-frame-forcing-label-re-application)
  - [7. `egui_plot::Plot` is opaque to assistive tech](#7-egui_plotplot-is-opaque-to-assistive-tech)
- [Focus and keyboard handling](#focus-and-keyboard-handling)
  - [8. `Context::egui_wants_keyboard_input` is misleadingly named](#8-contextegui_wants_keyboard_input-is-misleadingly-named)
  - [9. `Focus::begin_pass` snapshots arrow events before widgets can consume them](#9-focusbegin_pass-snapshots-arrow-events-before-widgets-can-consume-them)
  - [10. `Focus::begin_pass` clears focus on bare Escape unconditionally](#10-focusbegin_pass-clears-focus-on-bare-escape-unconditionally)
  - [11. `Memory::set_focus_lock_filter` has a one-frame hole](#11-memoryset_focus_lock_filter-has-a-one-frame-hole)
  - [12. `Memory::get_temp` forces a clone on read](#12-memoryget_temp-forces-a-clone-on-read)
- [Modals and popups](#modals-and-popups)
  - [13. `create_widget` surrenders focus on widgets covered by a modal](#13-create_widget-surrenders-focus-on-widgets-covered-by-a-modal)
  - [14. `top_modal_layer` is one frame stale](#14-top_modal_layer-is-one-frame-stale)
  - [15. `Modal::show` has no initial-focus helper](#15-modalshow-has-no-initial-focus-helper)
  - [16. `Modal::should_close` consumes Escape unconditionally](#16-modalshould_close-consumes-escape-unconditionally)
  - [17. `Popup::menu` is not modal and has no built-in Escape handling](#17-popupmenu-is-not-modal-and-has-no-built-in-escape-handling)
- [Color picker](#color-picker)
  - [18. `color_slider_1d` / `color_slider_2d` are private and mouse-only](#18-color_slider_1d--color_slider_2d-are-private-and-mouse-only)
- [Resize handles](#resize-handles)
  - [19. `Panel` resize handles are silent and have no public id](#19-panel-resize-handles-are-silent-and-have-no-public-id)

---

## Accessibility — labels and roles

### 1. `Role::Label` silently swallows `set_label` overrides

**Status:** Not started — Issue-first (B). The `set_value`-not-`set_label`
branch may be intentional; confirm intent before a PR. Unchanged as of
egui 0.36.2 — same branch, at `response.rs:962-968`.

**Where:** `response.rs:962-968`

```rust
if let Some(label) = info.label {
    if matches!(builder.role(), Role::Label) {
        builder.set_value(label);   // <-- value, not label
    } else {
        builder.set_label(label);
    }
}
```

**Symptom.** Calling `ctx.accesskit_node_builder(label_id, |b|
b.set_label("Show release notes"))` on an `egui::Label` (e.g. a
clickable version label) is a silent no-op. Screen readers still read
the literal text. The override succeeds — it just writes to a field
the role doesn't expose as the accessible name.

**Workaround we used.** Switched the version label from `Label` to
`Button` with `frame_when_inactive(false)`, which gives `Role::Button`
where `set_label` works. See `show_top_bar_right` in
`crates/dmm-gui/src/app/top_bar.rs`.

**Suggested fix.** Either:
- honour `set_label` on `Role::Label` nodes (route to both `value` and
  `label`, or override `value` from `set_label`), or
- document this asymmetry on `Context::accesskit_node_builder` (it is
  currently undocumented), or
- add `Response::override_accessible_label(&str)` that knows the role
  and does the right thing.

---

### 2. `Button` cannot carry a non-Button AccessKit role

**Status:** **Queued next** — Direct PR; #8130 has merged, so the gate is
lifted. Add `Button::color_swatch`/`role`. Plumbing exists:
`WidgetType::ColorButton → Role::ColorWell` already in `response.rs`.
Unchanged as of egui 0.36.2 — `Button` still hard-codes `WidgetType::Button`
in every arm of its `widget_info` (`widgets/button.rs:391-403`).

**Where:** `response.rs:949`, `widgets/button.rs:391-403`

The `Button` widget always emits `WidgetType::Button` and so always
ends up as `Role::Button`. There is no builder method to set a
different `WidgetType` / role.

**Symptom.** A `Button::new("").fill(color)` used as a color swatch
loses the `Role::ColorWell` that `color_edit_button_srgba` emits via
`WidgetType::ColorButton`. AT users hear "button" instead of "color
picker".

**Workaround we used.** None — accepted the regression so we could
build a keyboard-navigable color popup (see issue #18 below). See
`crates/dmm-gui/src/app/controls.rs:577`.

**Suggested fix.** Add `Button::role(WidgetType)` (or, more
specifically, `Button::color_swatch(Color32) -> Self` since color
buttons are the obvious case).

---

### 3. Toggleable `Button`s cannot announce pressed/not-pressed state

**Status:** **Shipped in egui 0.35** ([#8130](https://github.com/emilk/egui/pull/8130)).
First contribution, shipped as `Button::selected: Option<bool>`. Verified in
0.36.2: `Button::selected` at `widgets/button.rs:270`, and `widget_info` now
emits `WidgetInfo::selected(WidgetType::Button, …)` when it was called
(`widgets/button.rs:391-403`), which `response.rs:975-981` reads into
`builder.set_toggled`. `ResponseA11yExt::a11y_toggled` in
`crates/dmm-gui/src/a11y.rs` is therefore redundant at our
`selectable_label` / `selectable_value` call sites, which route through
`Button::selectable` (`ui.rs:1929-1931`, `:1939-1951`, `widgets/button.rs:78-82`).
It is *not* yet redundant at our plain-`Button` toggles (HOLD, REL, LIVE, …):
a `Button` that never calls `.selected(..)` still reports
`WidgetInfo::labeled` (`widgets/button.rs:401`). The helper is left in place;
switching those call sites to `Button::selected` and retiring it is a
follow-up.

**Where:** `widgets/button.rs:391-403`, `response.rs:975-981`

`WidgetInfo::selected` is read into `builder.set_toggled(...)`. Before
#8130, plain `Button` (`atom_ui` path) never set `info.selected`, so a
`Button` styled as a toggle (HOLD, REL, RANGE, AUTO, MIN-MAX, PEAK,
LIVE, etc.) could not announce its state to a screen reader without
bypassing `widget_info` entirely.

**Workaround we used.** Added a chainable `Response::a11y_toggled`
extension (`ResponseA11yExt` in `crates/dmm-gui/src/a11y.rs`) which
calls `ctx.accesskit_node_builder(id, |b| b.set_toggled(...))`
directly, bypassing `widget_info`. Call sites:
`crates/dmm-gui/src/app/controls.rs` and `crates/dmm-gui/src/graph/toolbar.rs`.
The chainable signature `Response::a11y_toggled(bool) -> Response`
prefigures the proposed upstream `Button::toggled(bool) -> Self`.

We tried using `Response::widget_info(WidgetInfo::selected(...))` for
this first and discovered it pushes a duplicate `OutputEvent::Clicked`
on the click frame because Button's `atom_ui` already calls
`widget_info` internally. See issue #4 below.

**Suggested fix.** Add `Button::toggled(bool) -> Self` (matching
`SelectableLabel`'s built-in selected state), or add
`Response::set_toggled(bool)` that writes only the AccessKit toggle
state without re-emitting the click event.

---

### 4. `Response::widget_info` re-dispatch pushes duplicate click events

**Status:** Not started — legitimate split (halves are independent): **4a**
document `accesskit_node_builder` as the supported post-hoc setter = Direct
(doc) PR — clarifies an undocumented escape hatch but does **not** fix the
duplicate-`OutputEvent::Clicked` foot-gun; **4b** new `accesskit_only`
`widget_info` variant (the actual fix) = Issue-first (D). Unchanged as of
egui 0.36.2 — same clicked/double/triple/focus-gained/changed dispatch into
`output_event`, at `response.rs:869-894`.

**Where:** `response.rs:869-894`

Calling `Response::widget_info` twice on the same response — once
implicitly by the widget itself, once explicitly by the caller —
queues two `OutputEvent::Clicked` events on the click frame. There is
no documented "AccessKit-only" setter for after-the-fact state
updates.

**Symptom.** AT announces the click twice when a toggle button is
activated. Forces use of `accesskit_node_builder` directly (an
"escape hatch" that is itself undocumented as the correct path for
this).

**Suggested fix.** Document `accesskit_node_builder` as the correct
post-hoc state setter, OR add an `accesskit_only` variant of
`widget_info` that updates the AccessKit node without queuing
`OutputEvent`s.

---

## Accessibility — landmarks and live regions

### 5. No public landmark helper; `ui.scope` ids are unstable

**Status:** Not started — Issue-first (D). New public landmark concept;
align on shape first. Unchanged as of egui 0.36.2 — `new_child` still salts
the child id with `next_auto_id_salt` (`ui.rs:251-260`) and still writes
`Role::GenericContainer` (`ui.rs:317-319`).

**Where:** `ui.rs:209-321`, `ui.rs:251-260`, `ui.rs:317-319`

`ui.scope` derives its child id from
`parent.id.with("child").with(next_auto_id_salt)`, where
`next_auto_id_salt` is a running counter inside the parent. If the
parent's child layout changes (e.g. an `if connected { ... } else {
... }` swaps in a different number of widgets), every scope's id
flips.

`new_child` also writes `Role::GenericContainer` to the scope's
AccessKit node, which then collides with anything the caller writes
via `set_role` later (last-writer-wins).

**Symptom.** We wrap the connection-status row in `ui.scope(...)` +
`ctx.accesskit_node_builder(scope.id, |b| b.set_role(Role::Status))`.
Connection state flipping (Disconnect → Connect) changes the number
of sibling widgets in the surrounding `horizontal()`, which shifts
the auto-salt counter, which assigns a different id to the scope,
which makes Orca lose track of the `Role::Status` landmark.

**Workaround.** Use `ui.scope_builder(UiBuilder::new().id_salt("status_landmark"), ...)`
with an explicit id salt so the id is stable across sibling-count
changes. Encapsulated locally as `Ui::landmark(id_salt, role,
add_contents)` (see `UiA11yExt` in `crates/dmm-gui/src/a11y.rs`); the
signature directly prefigures the proposed upstream API below.

**Suggested fix.** Add a first-class landmark API:

```rust
ui.landmark(accesskit::Role::Toolbar, |ui| { ... });
// or
UiBuilder::new().landmark(accesskit::Role::Status)
```

that picks a stable id_salt internally and writes the role without
the `Role::GenericContainer` collision.

---

### 6. AccessKit tree is rebuilt every frame, forcing label re-application

**Status:** Not started — legitimate split (halves are independent): **6a**
document the per-frame re-apply contract on `accesskit_node_builder` = Direct
(doc) PR — clarifies an undocumented lifetime but does **not** remove the
re-apply burden; **6b** persistent-label API (the actual fix) =
Issue-first (D). Unchanged as of egui 0.36.2 — the builder still writes into
`this_pass.accesskit_state` (`context.rs:601`), which is re-created every pass.

**Where:** `context.rs:600-632`

`Context::accesskit_node_builder` writes into
`this_pass.accesskit_state`, which is initialised fresh every frame.
There is no "sticky" override — every consumer that wants a stable
label or live-region marker on a non-standard widget has to re-apply
it on every frame, even when nothing has changed.

**Symptom.** Our `set_live_region_cached` helper has to cache the
formatted label string in `ctx.data` and re-issue `set_label` +
`set_live(Live::Polite)` every frame. Same for the plot summary.

**Suggested fix.** Either:
- offer a sticky API:
  `Context::set_accesskit_label_persistent(id, Arc<str>)`, or
- document the "you must re-apply every frame" contract clearly on
  `Context::accesskit_node_builder` (currently the doc comment says
  nothing about lifetime).

---

### 7. `egui_plot::Plot` is opaque to assistive tech

**Status:** Not started — Issue-first (D). Cross-crate (egui_plot), large
new API surface. Unchanged as of egui_plot 0.37.0 — the crate does not
mention `accesskit` anywhere, and each axis is still a bare
`ui.allocate_rect(widget.rect, Sense::drag())`.

**Where:** `egui_plot` 0.37.0: `plot.rs:859`, `:871` (axis responses),
`:1629` (the plot rect).

The `Plot` widget renders a focusable interactive surface but exposes
no AccessKit label, role, or description. Each axis allocates a
focusable drag response, with no label and no focus ring.

**Symptom.** A user Tab-walking the GUI hits the plot and several
axis Tab stops with no AT feedback at all. We worked around this by
calling `accesskit_node_builder` on the plot response id after
`plot.show()` returns, but the axis Tab stops remain unlabelled —
their ids are not exposed to the caller.

**Workaround.** See `crates/dmm-gui/src/graph/render.rs` (search for
`a11y_label`).

**Suggested fix (against `egui_plot`).**
- Add `Plot::accesskit_label(impl Into<String>) -> Self` and
  `Plot::accesskit_description(impl Into<String>) -> Self`.
- Default the plot widget's role to `accesskit::Role::Graphic` /
  `Role::Figure` so it at least appears in flat-review.
- Either suppress focusability on the per-axis drag responses, or
  expose their ids and a way to label them.

---

## Focus and keyboard handling

### 8. `Context::egui_wants_keyboard_input` is misleadingly named

**Status:** Not started — strong split (both halves independently valuable):
**8a** add `Context::wants_text_input()` (true only for a focused
`TextEdit`/`DragValue`) = Direct PR — this is the **substantive fix** for the
foot-gun; **8b** rename the misleadingly-named existing method = separate
deprecation, Issue-first (B). Unchanged as of egui 0.36.2 — still
`self.memory(|m| m.focused().is_some())`.

**Where:** `context.rs:2986-2988`

The name reads as "is a text input focused" but the implementation is
"is *any* widget focused". Every custom keyboard shim that wants to
share arrow keys with a focused non-text widget has to additionally
check `text_edit_focused()` or its own widget-id equality.

**Symptom.** Our `Graph::handle_keyboard` originally guarded with
`if ctx.egui_wants_keyboard_input() { return; }`. This silently
prevented arrow-key panning while the minimap was focused — because
the minimap *itself* counts as "any focused widget". We had to put
the minimap-focused branch *before* the guard. See
`crates/dmm-gui/src/graph/view.rs` `handle_keyboard`.

**Suggested fix.**
- Rename `egui_wants_keyboard_input` to `wants_focus` /
  `any_widget_focused`, or
- add `Context::wants_text_input(&self) -> bool` that only returns
  true when a `TextEdit` / `DragValue` actually has focus.

---

### 9. `Focus::begin_pass` snapshots arrow events before widgets can consume them

**Status:** Not started — Issue-first (B). Changes `focus_direction`
handling for every focusable widget. Unchanged as of egui 0.36.2 —
`begin_pass` still assigns `focus_direction` from the arrow snapshot
(`memory/mod.rs:607`) and `end_pass` still commits it without checking
consumption (`:624-628`).

**Where:** `memory/mod.rs:570-621`, `:623-628`

`Focus::begin_pass` walks pending input events at the *start* of the
frame and sets `focus_direction` from arrow-key events. `end_pass`
later commits a directional focus move via `find_widget_in_direction`
based on `focus_direction`, **without checking whether the originating
event was consumed by a widget during the frame**.

**Symptom.** A custom widget that wants to use Left/Right arrows
(minimap pan, color picker hue slider, side-panel divider resize)
*does* receive the arrow keys via `consume_key`, but `end_pass` then
also walks `focus_direction` and steals focus to the spatially nearest
widget. The user perceives the arrow press as both panning the
minimap AND jumping focus to the nearest button. We have to call
`memory.move_focus(FocusDirection::None)` immediately after
`consume_key` to undo the snapshot.

This is a foot-gun for every custom focusable widget that wants
keyboard navigation.

**Suggested fix.**
- Make `consume_key` clear `focus_direction` for the matching
  direction, OR
- have `end_pass` skip `find_widget_in_direction` if the originating
  arrow event has been consumed.

---

### 10. `Focus::begin_pass` clears focus on bare Escape unconditionally

**Status:** Not started — Issue-first (B). Changes bare-Escape
focus-clearing globally. Unchanged as of egui 0.36.2 — the
`Key::Escape if !modifiers.any()` arm still clears `focused_widget` outright.

**Where:** `memory/mod.rs:599-602` (the `Key::Escape` arm in
`Focus::begin_pass`)

Bare Escape unconditionally clears `focused_widget`, even when a
modal (or any widget) is also handling Escape. Every modal-focus
implementation has to play games to restore focus afterwards.

**Symptom.** Closing the shortcut-help modal with Escape clears
focus globally; restoring it requires deferred logic (see issue #13).

**Suggested fix.** One-line guard:
```rust
if self.top_modal_layer.is_none() { /* clear focused_widget */ }
```
or expose a setting that lets the consumer opt out.

---

### 11. `Memory::set_focus_lock_filter` has a one-frame hole

**Status:** Not started — Issue-first (B). Relaxing the
`had_focus_last_frame` gate affects all focus-lock users. Unchanged as of
egui 0.36.2 — 0.36 rewrote the body as a let-chain and added a redundant
`focused.id == id` test, but the gate is the same.

**Where:** `memory/mod.rs:903-911`

```rust
pub fn set_focus_lock_filter(&mut self, id: Id, event_filter: EventFilter) {
    if self.had_focus_last_frame(id)
        && self.has_focus(id)
        && let Some(focused) = &mut self.focus_mut().focused_widget
        && focused.id == id
    {
        focused.filter = event_filter;
    }
}
```

The `had_focus_last_frame && has_focus` gate means the filter is a
silent no-op on the first frame after focus is granted. So a
"request_focus then set_focus_lock_filter" pattern doesn't work as
written — the filter only takes effect on frame +2.

**Symptom.** Color picker arrow trapping doesn't work on the first
frame the user Tabs onto a slider — the next arrow press escapes the
popup.

**Workaround.** Cover the one-frame hole by also calling
`memory.move_focus(FocusDirection::None)` if any arrow is held this
frame. See `crates/dmm-gui/src/app/controls.rs` (search for
`set_focus_lock_filter`).

**Suggested fix.** Drop the `had_focus_last_frame` gate, or make
`request_focus` also set `had_focus_last_frame` to true so the gate
passes immediately.

---

### 12. `Memory::get_temp` forces a clone on read

**Status:** Not started — Direct PR. Additive `Memory::with_temp` borrow
accessor mirroring `get_temp`. Unchanged as of egui 0.36.2 — still no borrow
accessor, and `get_temp` still ends in `.cloned()`.

**Where:** `util/id_type_map.rs:446-449`

```rust
/// The call clones the value (if found), so make sure it is cheap to clone!
pub fn get_temp<T: 'static + Clone>(&self, id: Id) -> Option<T> {
    let key = RawKey::new::<T>(id);
    self.map.get(&key).and_then(|x| x.get_temp()).cloned()
}
```

There is no `with_temp(&self, id, |&T| -> R) -> Option<R>` borrow API,
so any cached `String`, `Vec`, etc. is cloned out of `IdTypeMap` on
every read.

**Symptom.** Our `set_live_region_cached` claims (in its doc-comment)
to be allocation-free on the no-change path, but actually clones the
cached `String` out of `ctx.data` every frame just to pass it to
`builder.set_label`.

**Workaround.** Cache as `Arc<str>` so the clone is a refcount bump
instead of a heap allocation.

**Suggested fix.** Add:

```rust
pub fn with_temp<T: 'static + Send + Sync, R>(
    &self,
    id: Id,
    f: impl FnOnce(&T) -> R,
) -> Option<R>
```

so consumers can borrow without cloning.

---

## Modals and popups

### 13. `create_widget` surrenders focus on widgets covered by a modal

**Status:** Not started — Issue-first (B). Touches modal focus-surrender
semantics in `create_widget`. Restructured in 0.36 — the `allows_interaction`
test now feeds an `interested_in_focus` local — but the behaviour is unchanged.

**Where:** `context.rs:1256-1258`, `:1274-1277`

```rust
let interested_in_focus = w.enabled
    && w.sense.is_focusable()
    && self.memory(|mem| mem.allows_interaction(w.layer_id));
// ...
if allow_focus && !interested_in_focus {
    // Not interested or allowed input:
    self.memory_mut(|mem| mem.surrender_focus(w.id));
}
```

`Memory::create_widget` is called for every widget every frame. When
a `top_modal_layer` is set, every widget *below* the layer fails
`allows_interaction` and has its focus surrendered — even if the
caller just called `request_focus(below_layer_id)` to restore focus
after closing the modal.

**Symptom.** A naive `request_focus(opener_id)` on the frame the
modal closes is undone next frame. Restoring focus after a modal
close requires a deferred one-shot that waits until
`top_modal_layer.is_none()` before calling `request_focus`.

**Workaround.** See `crates/dmm-gui/src/app/mod.rs` —
`ShortcutHelp::restore_focus: Option<Id>` field plus a check at the
start of `ui()` that fires once the modal layer is gone.

**Suggested fix.** Either:
- add `Memory::request_focus_after_modal(id)` that survives
  `surrender_focus` while the modal layer is committed, or
- key the surrender check on whether the widget has *ever* held focus
  rather than wiping unconditionally, or
- have `Modal::show` automatically restore focus to the widget that
  was focused before the modal opened.

---

### 14. `top_modal_layer` is one frame stale

**Status:** Not started — Issue-first (B). **Not split:** the two suggested
fixes (de-stale `top_modal_layer` vs expose a `top_modal_layer_current_frame()`
accessor) are mutually-exclusive alternatives — exposing the accessor only
makes sense if you decide *not* to de-stale, so the choice itself is the
design decision. Present both options in the Issue. Unchanged as of egui
0.36.2 — `top_modal_layer` is still taken from `top_modal_layer_current_frame`
at `end_pass` (`memory/mod.rs:640`).

**Where:** `memory/mod.rs:640`, `:691-696`, `:995-997`

`Memory::set_modal_layer` writes `top_modal_layer_current_frame`, but
`top_modal_layer()` reads from `top_modal_layer`, which is only
updated at `end_pass`. So `top_modal_layer()` returns one-frame-stale
data.

**Symptom.** `Modal::show`'s in-frame focus trap races with
below-layer widgets that still pass `allows_interaction` on the
modal's first frame. First-frame focus trapping is silently broken
unless the caller knows to defer.

**Suggested fix.** Either:
- update `top_modal_layer` immediately when `set_modal_layer` is
  called (not at `end_pass`), or
- expose `top_modal_layer_current_frame()` for end-of-close detection
  and document the difference.

---

### 15. `Modal::show` has no initial-focus helper

**Status:** Not started — Direct PR *if* first-frame focus lands reliably via
a deferred one-shot (additive `Modal::initial_focus` builder). That timing
leans on #14's `top_modal_layer` staleness — if a deferred one-shot is not
enough, this folds into the #14 Issue rather than shipping alone. Unchanged
as of egui 0.36.2 — `containers/modal.rs` is byte-identical to 0.34.1, and the
builder is still `new`/`default_area`/`frame`/`backdrop_color`/`area` with no
focus API.

**Where:** `containers/modal.rs`

`Modal::show` exposes `should_close()` but no way to land keyboard
focus on a designated "initial focus" widget when the modal first
appears. Every consumer reinvents a `focus_pending: bool` flag and a
manual `request_focus` on the first frame.

**Symptom.** Boilerplate. See `ShortcutHelp::focus_pending` in
`crates/dmm-gui/src/app/mod.rs`.

**Suggested fix.** Add:

```rust
Modal::new(id).initial_focus(content_id).show(ctx, |ui| { ... })
```

or

```rust
Modal::show_with_initial_focus(ctx, id, |ui| { ... })
```

---

### 16. `Modal::should_close` consumes Escape unconditionally

**Status:** Not started — Issue-first (B). Changes when Escape closes a
modal (TextEdit carve-out). Unchanged as of egui 0.36.2 — `should_close` still
carves out only `any_popup_open`.

**Where:** `containers/modal.rs:151-163`

`Modal::should_close()` returns true on bare Escape whenever it is
the topmost modal. There is a carve-out for `any_popup_open` but not
for focused text entry — so Escape inside a `TextEdit` always closes
the parent modal, even when the user is trying to e.g. cancel an IME
composition.

**Suggested fix.** Add a carve-out for focused `TextEdit` /
`DragValue` (e.g. check `text_edit_focused()` before consuming
Escape).

---

### 17. `Popup::menu` is not modal and has no built-in Escape handling

**Status:** Not started — Issue-first (D). New `Popup::modal` behavior +
open-transition API. Unchanged as of egui 0.36.2 in the parts that matter:
`Popup` still never calls `set_modal_layer`, and still exposes no
open-transition accessor. 0.36 added an unrelated `Popup::interactable(bool)`
(`popup.rs:378`).

**Where:** `containers/popup.rs:378`, `:624-626`, contrast with
`containers/modal.rs:85`, `:155-157`

`Modal::show` calls `Memory::set_modal_layer` so Tab focus stays
inside it. `Popup::menu` does not — Tab from a focused widget inside
a popup escapes to the layer below, which is rarely what you want.
`Popup` does close on bare Escape, but it reads the key with a
non-consuming `ctx.input(|i| i.key_pressed(Key::Escape))`
(`popup.rs:625`), so the same press still reaches `Focus::begin_pass` and any
enclosing `Modal`; `Modal::should_close` consumes it instead
(`modal.rs:155-157`).

There is also no clean way to detect the "open transition" of a
popup. `Popup::is_id_open` must be read *before* `show()` to know if
this frame is the open frame, otherwise the toggle-inside-`show`
semantics make it impossible to distinguish.

**Symptom.** Our color picker popup had to call
`Memory::set_modal_layer` itself, handle Escape itself, and track
"was this the click that just opened me" via a separate
`was_open_key` in `ctx.data`. Lots of hand-rolled boilerplate.

**Workaround.** See `color_edit` in
`crates/dmm-gui/src/app/controls.rs`.

**Suggested fix.** Add a `Popup::modal(true) -> Self` builder that:
- calls `set_modal_layer` for the popup's duration,
- handles Escape (closing the popup), and
- exposes the open transition via `PopupResponse::just_opened()`.

---

## Color picker

### 18. `color_slider_1d` / `color_slider_2d` are private and mouse-only

**Status:** Not started — Issue-first (D). Biggest gap: keybinding design +
making private sliders public. Unchanged as of egui 0.36.2 — both sliders are
still module-private and still driven only by `interact_pointer_pos`
(`:144`, `:211`). 0.36 reworked the swatch painting
(`show_srgba_unmultiplied`) but added no keyboard handling.

**Where:** `widgets/color_picker.rs:138, 202`

`color_slider_1d` and `color_slider_2d` are the actual hue (1D
gradient) and saturation/value (2D square) widgets behind
`color_picker_color32` / `color_picker_hsva_2d`. Both are module-private
plain `fn`s and only consume `interact_pointer_pos` — they have no
keyboard handling at all. `color_picker_hsva_2d` is public but the
sliders inside it cannot be keyboard-navigated, and there is no
public way to identify them by id from outside the crate.

**Symptom.** The Customize-colors popup is opened by keyboard, the
user Tabs through to the saturation/value square, and arrow keys do
nothing (or worse: steal focus to a neighbouring widget via the
issue #9 race).

**Workaround.** We classify the focused widget inside the popup by
*rect shape* (square ≥ 50px → 2D slider; wide-and-short ≥ 50px wide
and width > 3× height → 1D hue slider) and apply a 0.02 step to the
cached `Hsva` ourselves. The 50px threshold was chosen because the
actual slider widths in our theme happen to be exactly 100px — the
heuristic is acknowledged-brittle and would break under non-default
DPI or theme tweaks. See `crates/dmm-gui/src/app/controls.rs`
`color_edit`.

**This is the single biggest upstream gap in the entire pass.**

**Suggested fix.**
- Make `color_slider_1d` / `color_slider_2d` public, OR
- handle Arrow / Home / End / PageUp / PageDown on the focused
  sliders inside `color_picker_color32` and `color_picker_hsva_2d`,
  with `Shift` for fine and `Ctrl` for coarse steps.

Even just keyboard support inside the existing public
`color_picker_hsva_2d` would let downstream consumers drop the
rect-shape heuristic.

---

## Resize handles

### 19. `Panel` resize handles are silent and have no public id

**Status:** Not started — Issue-first (D). Multiple decisions: expose handle
id, built-in keyboard resize, focus indicator, response type. Still open as of
egui 0.36.2 (`SidePanel` and `TopBottomPanel` are one `Panel` type now): the
handle id is computed by a private `Panel::resize_id`
(`containers/panel.rs:942-944`) over a private `resize_id_source` field
(`:234`, set only by the crate-internal `with_resize_id_source` at `:1145`), so
callers still cannot ask for it. The salt itself is unchanged, so replicating
it still works.

**Where:** `containers/panel.rs:35-37` (the `__resize` salt), `:942-944`,
`:999`, `:1092`; `sense.rs:81-83` (`click_and_drag` includes FOCUSABLE)

`Panel::left(..).resizable(true)` allocates an internal resize handle.
The handle is focusable (`Sense::click_and_drag → FOCUSABLE`) but:
- it has no AccessKit label — panel handles never call `widget_info`, so they
  do not even reach the `WidgetType::ResizeHandle → Role::Splitter` mapping
  that `response.rs:954` already provides and `containers/window.rs:1085`
  already uses,
- it has no keyboard resize binding (only mouse drag works),
- it has no visible focus indicator (its own paint overdraws the
  default focus rect),
- its id is private — the `__resize` salt is an internal
  implementation detail that we have to replicate to attach a label
  and a keyboard handler.

**Symptom.** Our left-panel resize handle is a Tab stop with no
feedback whatsoever for keyboard / AT users.

**Workaround.** Replicate the internal id salt:
```rust
let reading_panel_resize_id = Id::new("reading_panel").with("__resize");
```
and use `egui::PanelState::load` + `data_mut().insert_persisted` to
move the panel boundary in response to Left/Right arrow presses on
that id. Brittle — any change to the salt internals breaks us
silently.

**Suggested fix.**
- Expose the resize handle id via the `Panel` builder (make
  `resize_id`/`resize_id_source` public), OR
- add built-in keyboard resize (Arrow keys when focused) AND a
  visible focus indicator AND an AccessKit label, OR
- include the handle id in what `Panel::show` returns, alongside the inner
  content.
