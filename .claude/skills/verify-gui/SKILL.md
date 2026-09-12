---
name: verify-gui
description: >-
  Runs dmm-gui headless on a private Xvfb display against the mock device to
  take screenshots, do visual checks, measure contrast, and exercise keyboard
  shortcuts, clicks, wheel ticks and window resizes — without ever opening a
  window on the live desktop. Use for any GUI change that needs to be seen or
  driven: a screenshot, a theme or contrast check, a layout check at any window
  size, or a shortcut/click/scroll test.
paths:
  - "crates/dmm-gui/**"
allowed-tools: Bash(${CLAUDE_SKILL_DIR}/scripts/gui-display.sh *)
---

# Headless dmm-gui verification

## Safety

- MUST launch dmm-gui only through `scripts/gui-display.sh`. Never `cargo run -p dmm-gui`, never `target/debug/dmm-gui` directly: winit 0.30 ignores `WINIT_UNIX_BACKEND` and opens on Wayland whenever `WAYLAND_DISPLAY` is set, putting the window on the user's screen.
- MUST NOT run `xdotool`, `import`, or any other input or capture tool against the user's display (`:0`, `:1`). On GNOME this raises a "Remote Desktop — Allow Remote Interaction" prompt.
- MUST run `stop` when finished, including after a failure.
- Hyperlinks in the app (Help / GitHub, Manual, Report feedback) never reach the user's browser: `run` points `BROWSER` at `scripts/blocked-browser.sh`, which appends `blocked browser open: <url>` to the log instead. Grep the log for that line to check a link was activated; do not press Enter on a focused link expecting anything else.
- The CSV save dialog (Ctrl+E, Export CSV) cannot reach the user's desktop either: `run` gives the app a dead session-bus address and `GDK_BACKEND=x11`, so the desktop-portal dialog is replaced by rfd's zenity fallback on the private display, or by nothing if zenity is absent.

## Workflow

1. `start` — bring up the private display.
2. `run [dmm-gui args…]` — build and launch; prints `WID=<window id>` and the log path.
3. `key <chord>` / `click <x> <y>` / `wheel <x> <y> [up|down] [ctrl]` — drive the window; `resize <width> <height>` to check a layout at another size.
4. `shot <out.png>` — capture the private display.
5. View the PNG with the Read tool, or sample pixels with python3 + PIL to compute contrast numerically. Write screenshots to the session scratchpad directory.
6. `stop` — kill dmm-gui and the display.

## Commands

Run the script rather than reading it:

```sh
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh start
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh run --device mock
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh run --mock-mode ohms
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh run --mock-mode dcv --mock-clock-preseed 90
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh key ctrl+o
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh click 125 12
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh wheel 400 300 down
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh wheel 400 300 up ctrl
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh resize 420 300
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh shot <dir>/before.png
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh status
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh stop
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh selftest
```

`run` defaults to `--device mock`, refuses any other `--device` unless the user has approved real hardware (`VERIFY_GUI_ALLOW_HW=1`), uses a private `XDG_CONFIG_HOME` and `XDG_DATA_HOME` so the user's `settings.json` and desktop entries are untouched, and waits for the window plus the first frames. `resize` reshapes the window for small-window checks: there is no window manager on the private display, so the app's `MinInnerSize` hint is not enforced, but the app re-grows a window below its own computed minimum — the command prints the size it settled on. `VERIFY_GUI_GEOMETRY=WxHxDEPTH` (default `1600x1000x24`) sets the root window; `start` reuses a running Xvfb, so `stop` before changing it, and note that an env-var prefix falls outside this skill's allowed-tools pattern and will prompt. `shot` writes plain `.png` paths only. Every subcommand exits non-zero with a message naming the log when something fails — a missing window means the app died or drew elsewhere, so read the log before retrying.

## Input behaviour

- `key` takes xdotool keysym names joined by `+`: `ctrl+o`, `space`, `question`, `bracketleft`, `Home`. `click` and `wheel` coordinates are window-relative pixels at 1×.
- `key` holds each modifier down across a frame and releases it after: egui reads its modifier snapshot when the frame runs, so a chord released within a millisecond can arrive with no modifiers.
- `wheel <x> <y> [up|down] [ctrl]` sends one wheel tick at that point (default `down`): plain wheel scrolls the panels, `ctrl` zooms the graph. With `ctrl` it holds Ctrl across a frame the same way `key` does.
- The zoom-in chord is `key ctrl+equal`, not `ctrl+plus`: xdotool's `plus` keysym needs Shift, and the app binds `Key::Equals` alongside `Plus`.
- Ctrl+C, Ctrl+X and Ctrl+V (with or without Shift) become clipboard events in egui-winit before egui sees a key, so no app binding on them can fire — do not test one.

## Dependencies

`xvfb`, `xdotool`, `imagemagick` (for `import`); `python3-pil` optional, for pixel measurement. The script names them if any are missing.

## References

- WCAG contrast thresholds and the rest of the visual bar: `.claude/rules/gui.md`.
- Scenario flags: `dmm-gui --help` lists the flags; the `--mock-mode` values come from `MockMode::ALL` in `crates/dmm-lib/src/mock/mod.rs`, and passing an invalid one makes dmm-gui print the valid list. Two mock-only flags hidden from `--help`, `--mock-clock-preseed <SECS>` and `--mock-clock-scale <FACTOR>`, start a run with history instead of waiting for it; pin `--mock-mode` alongside. Details: `docs/development.md`, Headless GUI checks.
