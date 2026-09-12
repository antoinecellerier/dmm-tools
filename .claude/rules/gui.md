---
paths:
  - "crates/dmm-gui/**"
---

# GUI rules (dmm-gui)

## Correctness

- **Test every visual change in both dark and light themes.** Colors tuned for dark mode routinely fail WCAG contrast on light backgrounds — the single largest source of rework in this project.
- **Verify visual and interaction changes on a private display via the `verify-gui` skill.** Never launch `dmm-gui` on the live desktop and never inject input into it.
- All colors must be theme-aware (`ui.visuals().dark_mode`).
- WCAG 2.1 AA contrast: ≥4.5:1 for text, ≥3:1 for graphical elements. Verify numerically when adding/changing colors.
- Never rely on color alone — add line style, text, or bold as a secondary indicator.
- Minimum font size 11pt throughout.
- Display value strings use `display_raw` for stable width (no jitter).
- Icon-only or custom-painted interactive widgets need an AccessKit label via `accesskit_node_builder`. Buttons with text get this automatically; icon buttons and custom widgets do not.
- User-initiated actions (export, clear, connect) need visible feedback — toast, status message, or log line. Silent success is a UX bug.
- Think through boundary conditions before writing code: extreme window sizes (very wide, very narrow, quarter-screen, maximized), high zoom, empty/no-data state, mode transitions.
- Sample timestamps come from the session clock (`App.clock`, cloned into the mock's open closure), not `Instant::now()`; UI cadence — toasts, `request_repaint_after`, control-channel waits — stays on real time. On the private display use `--mock-clock-preseed <SECS>` / `--mock-clock-scale <FACTOR>` (mock-only, hidden from `--help`) instead of waiting for history to accumulate.
- Graph rendering has two tiers. The minimap keeps an incremental min/max level (`MinimapLevel`) over fixed session-time buckets about a pixel wide (`bucket_secs`), appended to per push and recut only when the width steps, drawn as one polyline per run of buckets — never rebuild the level per frame or per push, and never bucket by screen column, the strip rescales every sample and the trace flickers; the main graph builds segments from the visible slice via `visible_index_range()` binary search. Per-frame helpers (stats, y-bounds, envelope, crossings) must also iterate only the visible slice — do not regress them to full-history scans.

## Semantics

- **Pause halts acquisition entirely** — it is not a display-only freeze. The separate live-view toggle is the scroll-lock that freezes the view while data keeps arriving. Don't conflate the two.
- **User-facing text says "USB cable", not chip names.** CP2110/CH9329/CH9325 are internal transport details; help text, errors, and labels should talk about the cable/connection the user can see.

## egui pitfalls learned the hard way

- `set_plot_bounds()` overrides both axes — use `set_plot_bounds_x()` / `_y()` (egui_plot 0.33+) to constrain one axis.
- `allow_drag(false)` also suppresses pointer position events; use `plot.reset()` per frame to pin the view while keeping events.
- `plot.reset()` also clears egui_plot's `hidden_items`, so its `Legend` cannot act as a show/hide control while the view is pinned — paint a static key and put the toggles in the toolbar (the graph's **Show:** chips).
- After mode changes or data clears, call `plot.reset()` to avoid stale bounds from the previous state.
- Popups and dropdowns (`ComboBox`, `Popup::menu`) leave keyboard focus on the opener, so an opened popup is unreachable without Tab and Tab walks out of it. Every popup must move focus into itself on the frame it opens, handle Arrow/Enter/Esc, close on Tab, and return focus to the opener on close — `color_edit` in `app/controls.rs` and `show_choice_readout` in `display.rs` are the pattern.
- egui wraps id salts in `IdSalt`, so code that reproduces a widget's id must do the same: `make_persistent_id(IdSalt::new(salt))` matches `ComboBox::from_id_salt`; `Id::new(salt)` or the bare `&str` give a different id and nothing fails at compile time — `show_choice_readout` in `display.rs` is the example.
- `set_pixels_per_point()` and `set_visuals()` called every frame reset egui's internal panel state (resize positions, scroll offsets). Only call when the value changes.
- A wrapping label inside `egui::Grid` needs `num_columns(n)` on the grid: without it the last cell is handed the previous frame's column width (the `min_col_width` floor) and a wrapped label never grows past it, so every row folds into a narrow ribbon — the shortcut help grid is the example.
- `ui.set_max_height()` after content has been placed unions the new bound with the ui's `min_rect` and moves the cursor back to the top of the ui (`placer.rs`), so a `ScrollArea` placed after it paints over the rows above — hand the scroller a capped region with `ui.allocate_ui(vec2(w, cap), ..)` instead, as `show_settings_panel` in `app/controls.rs` does. Inside a `Panel::top` or an `Area`/`Modal` the content ui's `max_rect` is last frame's rect, so a bare `ScrollArea::max_height` sizes itself from stale space: compute the cap from `ctx.content_rect()` (the settings rows) or call `set_max_height` at the very top of the closure, before anything is placed (the shortcut help modal).
- A `ScrollArea` driven by keys needs `animated(false)`: with the default animation egui parks the delta as a target that the *next* pass turns into an offset and requests no repaint, so a keyboard scroll lands a frame late or not at all — see the comment on the help modal's scroller in `shortcut_help.rs`.
- The plain mouse wheel scrolls the panels; only Ctrl+wheel (`zoom_delta()`) may act on the graph, and a scroll area must expose no bar while its content fits — size the last element to `available_height()` floored, as the reading and graph columns do with `MIN_SPLIT_HEIGHT` (`app/recording_panel.rs`), and assert `content_size.y <= inner_rect.height()` in a headless test.
- egui API naming is inconsistent — verify method names against docs (`fill_color()` not `color()`, `Vec2b` not `Axis` for `allow_drag`/`allow_zoom`).
