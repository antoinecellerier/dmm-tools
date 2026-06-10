---
paths:
  - "crates/dmm-gui/**"
---

# GUI rules (dmm-gui)

## Correctness

- **Test every visual change in both dark and light themes.** Colors tuned for dark mode routinely fail WCAG contrast on light backgrounds — the single largest source of rework in this project.
- All colors must be theme-aware (`ui.visuals().dark_mode`).
- WCAG 2.1 AA contrast: ≥4.5:1 for text, ≥3:1 for graphical elements. Verify numerically when adding/changing colors.
- Never rely on color alone — add line style, text, or bold as a secondary indicator.
- Minimum font size 11pt throughout.
- Display value strings use `display_raw` for stable width (no jitter).
- Icon-only or custom-painted interactive widgets need an AccessKit label via `accesskit_node_builder`. Buttons with text get this automatically; icon buttons and custom widgets do not.
- User-initiated actions (export, clear, connect) need visible feedback — toast, status message, or log line. Silent success is a UX bug.
- Think through boundary conditions before writing code: extreme window sizes (very wide, very narrow, quarter-screen, maximized), high zoom, empty/no-data state, mode transitions.
- Graph rendering has two tiers. The minimap uses a full-history segment cache invalidated by the monotonic `history_version` counter; the main graph builds segments from the visible slice via `visible_index_range()` binary search. Per-frame helpers (stats, y-bounds, envelope, crossings) must also iterate only the visible slice — do not regress them to full-history scans.

## Semantics

- **Pause halts acquisition entirely** — it is not a display-only freeze. The separate live-view toggle is the scroll-lock that freezes the view while data keeps arriving. Don't conflate the two.
- **User-facing text says "USB cable", not chip names.** CP2110/CH9329/CH9325 are internal transport details; help text, errors, and labels should talk about the cable/connection the user can see.

## egui pitfalls learned the hard way

- `set_plot_bounds()` overrides both axes — use `set_plot_bounds_x()` / `_y()` (egui_plot 0.33+) to constrain one axis.
- `allow_drag(false)` also suppresses pointer position events; use `plot.reset()` per frame to pin the view while keeping events.
- After mode changes or data clears, call `plot.reset()` to avoid stale bounds from the previous state.
- `set_pixels_per_point()` and `set_visuals()` called every frame reset egui's internal panel state (resize positions, scroll offsets). Only call when the value changes.
- egui API naming is inconsistent — verify method names against docs (`fill_color()` not `color()`, `Vec2b` not `Axis` for `allow_drag`/`allow_zoom`).
