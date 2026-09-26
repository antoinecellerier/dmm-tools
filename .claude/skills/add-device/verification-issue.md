# Verification issues

The template is the newest one: `gh issue view 28` (ZOTEK). Copy its sections, not an older issue's. How many issues and what they link: `docs/adding-devices.md`, Phase 7.

- **Title:** "Help wanted: <brand> <models> verification (real hardware needed)", naming every model a reader might search for — "and other meters" tells a first-time reader nothing. Label `help wanted`.
- **"What needs verification"** is the `--list-steps --format md` output, pasted. The "Not covered by a capture step" list below it holds `list` output and the auto-detect check (`--device auto info` → `Detected: …`).
- **Every platform.** Each Linux-only command (`lsusb`, udev, `bluetoothctl`) gets its Windows and macOS equivalent, or the text says what to look for in the system settings.
- **Nothing unsafe** in the issue's own prose either: no step touches mains with the leads or probes an outlet.
- **Dev builds:** "Use the newest build in the [dev build listing](https://github.com/antoinecellerier/dmm-tools/releases?q=prerelease%3Atrue)", then the build-from-source line (Linux needs `libdbus-1-dev` for Bluetooth). Not a `dev-<sha>` — it expires before a reporter arrives.
- **Links** go to manufacturer-owned pages, never a file-sharing mirror.
