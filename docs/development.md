# Development

## Setup

After cloning, turn on the git hooks:

```sh
git config core.hooksPath git-hooks
```

`pre-commit` runs `cargo fmt --check`, `cargo clippy`, and `cargo test` before each commit. `commit-msg` rejects a `Claude-Session:` trailer in the message; `Co-Authored-By` trailers are fine.

On Linux the build needs `libudev-dev` for hidapi and `libdbus-1-dev` for the Bluetooth transport (`systemd-devel` and `dbus-devel` on Fedora); see [`setup.md`](setup.md) for the full list. `cargo check -p dmm-lib --no-default-features` builds the library without Bluetooth, which CI also runs.

## Running Tests

```sh
cargo test --workspace
```

All tests use `MockTransport` and run without hardware connected.

Tests that would otherwise wait drive session time instead of elapsing it:
build a `Clock::manual()`, hand it to the `Dmm` (and to `MockProtocol` where
the mock's waveform matters), then call `clock.advance(d)` and assert on
`clock.now()`. That is how the stream's pacing tests and the mock's
time-travel tests stay deterministic and finish instantly.

## Linting

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

## Build artifacts & disk usage

Cargo does not garbage-collect `target/` — old hash-suffixed artifacts in
`target/debug/deps` accumulate indefinitely. To reclaim space, run:

```sh
cargo clean                 # nuke target/ entirely (forces a full rebuild)
# or, to keep recent/live artifacts:
cargo install cargo-sweep   # one-time
cargo sweep --installed     # drop artifacts from old toolchains
cargo sweep --time 7        # drop artifacts not used in 7 days
```

The embedded git hash (`GIT_HASH`, shown in the GUI about line, `dmm-cli
--version`, and capture `tool_version`) is only baked in for **release** builds.
Debug builds use the constant `dev` so the binary's compile-time identity stays
fixed across commits — otherwise every commit would mint a fresh `*-<hash>`
binary in `target/debug/deps` and balloon disk usage. The release pipeline always
builds `--release`, so distributed binaries still carry the real commit hash. See
`crates/dmm-gui/build.rs` / `crates/dmm-cli/build.rs`.

## Adding Device Support

See **[`adding-devices.md`](adding-devices.md)** for the complete end-to-end guide covering discovery, reverse engineering, implementation, testing, and verification.

### Quick reference: implementation steps

**New device model (same protocol family):**

1. Create `crates/dmm-lib/src/protocol/<family>/tables/new_model.rs`
2. Implement `ModeTables`: one `entry` match returning the range table per mode (`DeviceTable` is derived), and give the model's spec tables a `SpecModel` variant. Spec tables sit in the family module: `ut61eplus/specs/`, `ut80x/specs_ut803.rs` and `specs_ut804.rs`, `ut181a/specs.rs`.
3. Register in the family's `tables/mod.rs`
4. Add a `SelectableDevice` entry in the family's `devices.rs` and list it in `DEVICES` in `protocol/registry.rs`, which sets the picker order

**New protocol family:**

1. Create `crates/dmm-lib/src/protocol/newfamily/mod.rs`
2. Implement the `Protocol` trait (`init`, `request_measurement`, `send_command`, `get_name`, `profile`, `capture_steps`)
3. Add variant to `DeviceFamily` enum in `protocol/mod.rs`
4. Name the links the family is seen on in `preferred_transports()` in `lib.rs` — its USB cables (the transports themselves are in `KNOWN_TRANSPORTS`), and `BLUETOOTH` if a UT-D07B carries it
5. Add a `SelectableDevice` entry in `protocol/newfamily/devices.rs` (`bluetooth_only` and the `bluetooth_names` it advertises, for a meter with the radio built in and no cable) and list it in `DEVICES` in `protocol/registry.rs`
6. Create research docs in `docs/research/newfamily/`
7. Mark as experimental until verified against real hardware (the CLI prints a warning for every model short of `Stability::Verified`)

The CLI and GUI automatically pick up new devices from the registry — no app code changes needed.

## Verifying Specification Data

The `dump_specs` example prints all per-device specification data (resolution,
accuracy, input impedance, notes) in formatted tables for side-by-side
comparison with the PDF manuals in `references/`. It reads each device's
`Protocol::spec_sheet`, and skips devices that have none unless named.

```sh
# Dump all devices
cargo run -p dmm-lib --example dump_specs

# Dump a specific device
cargo run -p dmm-lib --example dump_specs -- ut61b+

# Multiple devices
cargo run -p dmm-lib --example dump_specs -- ut61eplus ut61d+
```

Pipe to `less` or redirect to a file for easier comparison. The output
enumerates every mode and range for each device, showing exactly what the
GUI specifications panel will display.

`--format json <id>` prints one device's tables in the shape of a manual
transcription (`{model, tables: [{table, page, input_impedance,
overload_protection, notes, ranges: [{range, resolution, accuracy}]}]}`),
which is what gets diffed against a verified transcription.

`--format html <id>` prints a review sheet: each table laid out as the manual
prints it, beside the render of its manual page. Render the pages first, then
point `--pages-dir` at them; the page paths are written as given, so give
them relative to where the sheet is saved:

```sh
# PDF pages FIRST to LAST become pages/p-NN.png
pdftoppm -r 200 -png -f FIRST -l LAST manual.pdf pages/p
cargo run -p dmm-lib --example dump_specs -- --format html \
  --pages-dir pages --marks marks.json <id> > sheet.html
```

`--marks` is optional: a JSON object mapping `"<table> / <range> / <field>"`
(or `"<table> / <field>"`) to a note, where the field is `resolution`,
`accuracy`, `input_impedance`, `overload_protection`, `row` or `range`,
`notes`, `page` or `table`. A note may also be a `{status, evidence}` object,
the shape of a transcription's provenance file, which colours the cell by
status. Marked cells are highlighted with the note on hover, and marks that
match no cell are listed, collapsed, at the top of the sheet.

## Golden File Tests

Golden file tests verify measurement parsing against known-good byte sequences.
Each subdirectory of `crates/dmm-lib/tests/golden/` is named after a registry
device id (`ut61eplus`, `ut804`, …), and its `.yaml` files are parsed by that
device's `Protocol::parse_payload`. They use the same format as capture YAML
samples (`raw_hex`, `mode`, `value`, `unit`, `range_label`, `flags`), so you can
copy a sample directly from a capture report into a golden file.

To add a golden test:

1. Run `dmm-cli --device <id> capture` and complete the steps
2. Open the capture YAML and find a sample with known-good values
3. Copy the sample fields into a new `.yaml` file in `tests/golden/<id>/`
4. Run `cargo test --workspace` to verify

Golden tests run as part of the standard test suite. They are the primary
regression safety net for protocol parsing — add them whenever you verify
a new mode/range/flag combination against real hardware. Fixtures come
only from captures: a hand-built payload belongs in the parser's unit
tests, so a family has no golden directory until its first hardware run.

## Generated Doc Tables

Both device tables sit between `<!-- devices:start -->` and `<!-- devices:end -->`
markers, and a `dmm-cli` test guards each. The one in `docs/cli-reference.md` is
rendered from the registry: after changing an entry or the `DEVICES` order run
`UPDATE_DOCS=1 cargo test -p dmm-cli` to rewrite it rather than editing it by
hand, or the test fails with a diff. The one in `README.md` is hand-written on
purpose — the test only checks that every protocol family and every
verification issue still appears in it.

## Doc screenshots and snippets

Both come from the same recorded meter sessions, so a reader sees one bench
run rather than a simulation. Record a session once with
`dmm-cli read --format replay -o <name>.replay`, commit it under
`assets/replays/`, and point a snippet or a scene at it.

The command-output blocks in `README.md` and `docs/cli-reference.md` sit
between `<!-- snippet via=… -->` and `<!-- /snippet -->`, with the commands
themselves in the opening marker; a `dmm-cli` test runs each one and compares
what it prints with the block. `via=mock[:<mode>]` runs the command against the
mock device and `via=<name>.replay` against `assets/replays/<name>.replay`, so
a snippet that shows a meter's readings needs its recording committed there
first. The block shows the command as a user would type it, without either
flag. A plain `cargo test -p dmm-cli` fails with a diff;
`UPDATE_DOCS=1 cargo test -p dmm-cli` rewrites it.

The GUI pictures come from `scripts/doc-screenshots.sh all`, which stages one
scene per picture on the verify-gui skill's private display — a recording, the
settings the picture needs, and the keys, clicks and window size it is taken
at. `list` prints the pictures and naming one takes only its scene. A scene
hands out its recording's first seconds in one burst and then runs session
time at a thousandth of real time, so the window, the trace and the click
coordinates stay put however long its keys and clicks take, and a rerun stages
the same frame. Every capture prints how many pixels it differs from the
committed file, and leaves that file alone when none do, since a rewritten PNG
carries a new timestamp. X rendering is not identical across driver versions,
so look at each changed PNG before committing it. Three scenes
need the USB cable out and skip themselves with a message while one is plugged
in: the two settings pictures, so the panel shows its defaults, and the
connection-help one, which is the failed connection.

`cargo test -p dmm-gui --test doc_screenshots` fails when a doc shows a
picture no scene writes, or a scene writes one no doc shows.

`assets/replays/` holds the recordings. `dcma-boot-refresh.replay` is 185 s of
a low-power e-paper thermometer on a fixed 220 mA range — boot at 2.5–30 s
peaking at 108.5 mA, refreshes at 89 s and 149–170 s, a 0.02 mA idle floor —
and backs the README's text `read` block and the wide, narrow, graph, minimal
meter and palette pictures; `ohm.replay` is 23 s of a flat 4.649 kΩ on AUTO,
behind the README's JSON block. `dcmv-hold-rel.replay` is 39 s of DC mV with
HOLD from 10.4 s and REL from 26 s, for the reading controls.
`ut181a-vac-hz.replay` is a single frame rebuilt from a UT181A golden fixture,
behind the CSV block in `docs/cli-reference.md` and the big meter picture.
`dcv-steps.replay` (61 s of a bench supply stepped and ramped 2.9–9.3 V),
`dcma-boot-refresh-autorange.replay` (the same thermometer cycle with AUTO
ranging — 22 ↔ 220 mA hops and an OL blip at each boot and refresh) and
`ut181a-temp-t1-t2.replay` (a UT181A frame carrying two temperature
sub-values) are kept for future use.

## Shell Completions

Generate completions for your shell:

```sh
dmm-cli completions bash > ~/.local/share/bash-completion/completions/dmm-cli
dmm-cli completions zsh > ~/.zfunc/_dmm-cli
dmm-cli completions fish > ~/.config/fish/completions/dmm-cli.fish
dmm-cli completions powershell >> $PROFILE
```

## Release Process

1. Reread `## Unreleased` in `CHANGELOG.md` against `.claude/rules/changelog.md` and reorder it: entries by importance, sections in the rule's order
2. Rename the heading to the version. If the release has a theme, put a short tagline in it — `## v0.2.0 — Multi-Device Protocol Support` — stating what it changes in scope or intent; open the section with a one- or two-sentence summary of the intent and main areas touched, and close it with the `**Full Changelog**` compare link (see existing entries)
3. Set the release version in root `Cargo.toml` (workspace inherits it), e.g. `version = "0.3.0"`
4. Update `Cargo.lock`: `cargo update --workspace`
5. With the USB cable unplugged — three scenes skip themselves otherwise — regenerate the GUI pictures with `scripts/doc-screenshots.sh all`, review the deltas and the PNGs, and commit the ones whose changelog entry changed what they show
6. Run `scripts/package-docs.py <dir>` (needs `pandoc`): a dead relative link in a shipped doc fails the tagged build
7. Commit: `git commit -am "Release v0.3.0"`
8. Push the release commit — **confirm with the maintainer first** — and wait for CI to go green: `git push`
9. Tag and push the tag — **confirm with the maintainer again**, this publishes the release: `git tag v0.3.0 && git push origin v0.3.0`
10. The `release.yml` GitHub Actions workflow builds binaries for all supported platforms (Linux x86_64/ARM, Windows x86_64/ARM, macOS ARM/Intel) and creates a GitHub Release with the changelog entry as the body (a `[@user](https://github.com/user)` credit becomes an `@user` mention there, and only there), titled `v0.3.0 — <tagline>` (or just `v0.3.0` without one). The workflow fails before it builds anything if the tag does not match the workspace `version` in `Cargo.toml`, and fails if `CHANGELOG.md` has no `## v0.3.0` heading
11. Bump to the next dev version: set `version = "0.4.0-dev"` in `Cargo.toml`, run `cargo update --workspace`, put an empty `## Unreleased` back above the released heading (the nightly notes append that section), commit, and push — with the maintainer's OK, as for every push

## GitHub Actions workflows

Four workflows in `.github/workflows/`. None of them need touching to work on
the crates:

- `ci.yml` — fmt, clippy, tests and the dependency policy on every push and pull
  request, plus a three-target build so platform-specific breakage shows up
  early.
- `build-matrix.yml` — the six-target release build, called by the three others.
- `release.yml` — runs on a `v*` tag, see [Release Process](#release-process).
  Dispatched by hand against a branch it builds all six targets and publishes
  nothing; against a tag it overwrites that release's assets and notes.
- `dev-build.yml` — the nightly prerelease.

### Linting the workflows

`ci.yml` runs [actionlint](https://github.com/rhysd/actionlint) over
`.github/workflows/`. To run it locally, install `actionlint` and `shellcheck` —
without shellcheck it silently skips the bash inside `run:` blocks:

```sh
actionlint
```

### Dependency policy

`ci.yml` runs [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) over the
dependency graph: RustSec advisories, the licence allow-list in `deny.toml`,
wildcard version requirements and unknown registries. Locally:

```sh
cargo install --locked cargo-deny
cargo deny check
```

The allow-list holds exactly the SPDX ids the graph needs today, so a new
dependency on an unlisted licence fails the check — read the licence, decide
whether it belongs in a GPL-3.0-or-later binary, and only then add the id.

`.github/dependabot.yml` opens weekly update pull requests for the Cargo and
Actions dependencies, with minor and patch bumps grouped into one PR per
ecosystem and majors left on their own.

### Shared build matrix

`ci.yml`, `release.yml` and `dev-build.yml` all call `build-matrix.yml`, so a
nightly dev build exercises the same packaging path a release does — a break
shows up the next morning rather than at tag time — and the targets CI builds
cannot drift from the ones a release ships. CI passes `subset: ci` for the
cheaper three-target build, leaves `upload-artifacts` off, and is the only
caller setting `cache: true`: the 10 GB repository cache is worth more to
pull-request turnaround than to the unattended release and nightly builds.

The archives carry only the user docs, as Markdown and as HTML, with LICENSE and
the images they show. `scripts/package-docs.py` assembles them once per run for
every archive. Relative links to any other doc become GitHub links at the built
commit, and a link to a missing file fails the build. A new user doc goes into
its `DOCS` list. To look at the result locally (needs `pandoc`):

```sh
scripts/package-docs.py /tmp/package-docs
```

### Dev builds

`dev-build.yml` publishes a prerelease from `main` every night, skipping the run
when `main` has not moved. Each build gets its own immutable `dev-<short sha>`
tag — tags are never moved — and all but the newest seven dev releases are
deleted automatically, tag included. Nothing here needs doing by hand; do not
create or edit `dev-*` tags yourself. Trigger one early with
`gh workflow run dev-build.yml` (add `-f force=true` to rebuild a commit that
already has a dev release).

The prerelease body comes from `.github/dev-release-notes.md` with the
`## Unreleased` changelog section appended, which is another reason to keep that
section current as changes land. Keep each paragraph in that template on a
single line, however long: GitHub renders a single newline in a release body as
a hard line break, so wrapped prose comes out broken mid-sentence. `CHANGELOG.md`
is written the same way for the same reason.

## AI-Assisted Development

This project uses a `CLAUDE.md` file in the repo root to provide persistent
context and guidelines to AI coding assistants (Claude Code, Cursor, etc.).
It covers:

- Project structure and module responsibilities
- Build, test, and lint commands
- Engineering standards (error handling, logging, protocol correctness,
  commit discipline, GUI design, review checklist)
- Clean-room reverse engineering rules
- Documentation expectations

When using an AI assistant on this codebase, it will automatically pick up
these guidelines. Key points the assistant should follow:

- **Protocol changes must be verified against real hardware** — unit tests
  alone are not sufficient
- **Never fabricate specification data** — mark unknown values as missing
- **Physical device interaction requires user confirmation** — the assistant
  should describe the required setup and wait, not drive through steps
- Run `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`
  before committing

The `docs/research/` directories contain per-family reverse engineering notes
that provide essential context for protocol work. The assistant should read
the relevant `reverse-engineered-protocol.md` before modifying protocol code.

Alongside `CLAUDE.md`, `.claude/rules/` holds path-scoped rules that load
when their files are touched: `protocol.md` (`crates/dmm-lib/`), `gui.md`
(`crates/dmm-gui/`), `changelog.md` (`CHANGELOG.md`), `research-docs.md`
(`docs/research/`, `docs/architecture.md`), and for the user docs
`docs-user-facing.md`, `reference-docs.md` (the CLI and GUI references),
`device-catalog.md` (`docs/supported-devices.md`) and `readme.md`.
`.claude/skills/` holds on-demand checklists: `add-device` (new-meter
onboarding), `spec-data` (transcribing a manual's spec tables),
`issue-replies` (issue triage and GitHub reply drafting) and `verify-gui`
(headless GUI checks, below).

### Headless GUI checks

`.claude/skills/verify-gui/scripts/gui-display.sh` runs `dmm-gui` against the
mock device on a private Xvfb display, so screenshots, contrast measurements and
keyboard/click tests never touch your live desktop session or your
`settings.json`. Check the setup with:

```sh
.claude/skills/verify-gui/scripts/gui-display.sh selftest
```

It needs `xvfb`, `xdotool` and `imagemagick` (plus `python3-pil` for pixel
measurement). `start`, `run`, `key`, `click`, `wheel`, `resize`, `shot`, `status`
and `stop` are the individual steps; always finish with `stop`.

`wheel <x> <y> [up|down] [ctrl]` sends one wheel tick at window-relative
coordinates, and `resize <width> <height>` reshapes the window for small-window
checks — there is no window manager on the private display, so the app's minimum
size is not enforced, but the app re-grows a window below its own computed
minimum and `resize` prints the size it settled on. `VERIFY_GUI_GEOMETRY`
(default `1600x1000x24`) sets the root window size; `start` reuses a running
Xvfb, so `stop` before changing it.

Session clock: two hidden `dmm-gui` flags let a run start with history rather
than wait for it. `--mock-clock-preseed <SECS>` hands out that many seconds of
session time instantly, and `--mock-clock-scale <FACTOR>` runs what follows at
`FACTOR` times real speed. Both apply to the mock only — either one implies
`--device mock`, and an explicit hardware `--device` is refused — and neither
appears in `--help`.

```sh
.claude/skills/verify-gui/scripts/gui-display.sh run --mock-mode dcv --mock-clock-preseed 90
```

The first `shot` after the script's own first-frames wait then shows 90 s of
readings. Pin a mock mode as above: the auto-cycling mock changes scenario on
its own schedule and the graph re-anchors on each change, so an unpinned
preseed leaves only the last scenario's history on screen. The burst is spent
once per process, so a reconnect does not replay it. `dmm-cli read` takes the
same two flags, with `--device mock` spelled out, and `run --replay <FILE>`
takes them in place of `--mock-mode`, opening a recorded session at the instant
the preseed names.
