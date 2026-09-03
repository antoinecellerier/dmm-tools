---
name: verify-gui
description: >-
  Runs dmm-gui headless on a private Xvfb display against the mock device to
  take screenshots, do visual checks, measure contrast, and exercise keyboard
  shortcuts and clicks — without ever opening a window on the live desktop.
  Use for any GUI change that needs to be seen or driven: a screenshot, a
  theme or contrast check, a layout check, or a shortcut/click test.
paths:
  - "crates/dmm-gui/**"
allowed-tools: Bash(${CLAUDE_SKILL_DIR}/scripts/gui-display.sh *)
---

# Headless dmm-gui verification

## Safety

- MUST launch dmm-gui only through `scripts/gui-display.sh`. Never `cargo run -p dmm-gui`, never `target/debug/dmm-gui` directly: winit 0.30 ignores `WINIT_UNIX_BACKEND` and opens on Wayland whenever `WAYLAND_DISPLAY` is set, putting the window on the user's screen.
- MUST NOT run `xdotool`, `import`, or any other input or capture tool against the user's display (`:0`, `:1`). On GNOME this raises a "Remote Desktop — Allow Remote Interaction" prompt.
- MUST run `stop` when finished, including after a failure.

## Workflow

1. `start` — bring up the private display.
2. `run [dmm-gui args…]` — build and launch; prints `WID=<window id>` and the log path.
3. `key <chord>` / `click <x> <y>` — drive the window.
4. `shot <out.png>` — capture the private display.
5. View the PNG with the Read tool, or sample pixels with python3 + PIL to compute contrast numerically. Write screenshots to the session scratchpad directory.
6. `stop` — kill dmm-gui and the display.

## Commands

Run the script rather than reading it:

```sh
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh start
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh run --device mock
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh run --mock-mode ohms
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh key ctrl+o
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh click 125 12
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh shot <dir>/before.png
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh status
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh stop
${CLAUDE_SKILL_DIR}/scripts/gui-display.sh selftest
```

`run` defaults to `--device mock`, refuses any other `--device` unless the user has approved real hardware (`VERIFY_GUI_ALLOW_HW=1`), uses a private `XDG_CONFIG_HOME` and `XDG_DATA_HOME` so the user's `settings.json` and desktop entries are untouched, and waits for the window plus the first frames. `shot` writes plain `.png` paths only. Every subcommand exits non-zero with a message naming the log when something fails — a missing window means the app died or drew elsewhere, so read the log before retrying.

## Input behaviour

- `key` takes xdotool keysym names joined by `+`: `ctrl+o`, `space`, `question`, `bracketleft`, `Home`. `click` coordinates are window-relative pixels at 1×.
- `key` holds each modifier down across a frame and releases it after: egui reads its modifier snapshot when the frame runs, so a chord released within a millisecond can arrive with no modifiers.
- Ctrl+C, Ctrl+X and Ctrl+V (with or without Shift) become clipboard events in egui-winit before egui sees a key, so no app binding on them can fire — do not test one.

## Dependencies

`xvfb`, `xdotool`, `imagemagick` (for `import`); `python3-pil` optional, for pixel measurement. The script names them if any are missing.

## References

- WCAG contrast thresholds and the rest of the visual bar: `.claude/rules/gui.md`.
- Scenario flags: `dmm-gui --help` lists the flags; the `--mock-mode` values come from `MockMode::ALL` in `crates/dmm-lib/src/mock.rs`, and passing an invalid one makes dmm-gui print the valid list.
