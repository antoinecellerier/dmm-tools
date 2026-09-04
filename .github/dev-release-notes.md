> [!WARNING]
> **This is an automated development build, not a release.** It is built from
> the latest commit on `main`, and may be broken. For normal use, download the
> [latest stable release](__REPO__/releases/latest).

Built from commit [`__SHA__`](__REPO__/commit/__SHA__) on __DATE__.

[All changes since __PREV__](__REPO__/compare/__PREV__...__SHA__)

## Reporting a problem

Please include the output of `dmm-cli --version` — on this build it ends with
`(__SHA__)` — so the report points at the exact commit it came from.
[Open an issue](__REPO__/issues/new).

## Before you run it

- **macOS** — the binaries are unsigned, so Gatekeeper blocks the first launch.
  Right-click the binary and choose *Open*, or run
  `xattr -d com.apple.quarantine dmm-cli dmm-gui`.
- **Windows** — SmartScreen warns about unsigned binaries. Choose *More info*,
  then *Run anyway*.
- **Linux** — built on current Ubuntu, linking GTK3 and libxkbcommon
  dynamically. On an older distribution `dmm-gui` may refuse to start; build
  from source there.

## Changes since the last release

