# Development

## Setup

After cloning, turn on the git hooks:

```sh
git config core.hooksPath git-hooks
```

`pre-commit` runs `cargo fmt --check`, `cargo clippy`, and `cargo test` before each commit. `commit-msg` rejects a `Claude-Session:` trailer in the message; `Co-Authored-By` trailers are fine.

On Linux the build needs `libudev-dev` for hidapi and `libdbus-1-dev` for the Bluetooth transport (`systemd-devel` and `dbus-devel` on Fedora); see [`setup.md`](setup.md) for the full list. `cargo check -p dmm-lib --no-default-features` builds the library without Bluetooth, and `cargo check -p dmm-gui --no-default-features` the GUI without its update check; CI runs both.

Shell completions are in the [CLI reference](cli-reference.md#dmm-cli-completions) and the [GUI reference](gui-reference.md#command-line-options).

## Running tests

```sh
cargo test --workspace
```

No test needs a meter connected. A few are `#[ignore]`d: the timing-sensitive
ones, which measure that a cost does not grow with the session and are meant
for a release build, and one that reaches api.github.com, whose comment says
how to run it alone. `cargo test --release -p dmm-gui -- --ignored` runs them
all.

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

## Adding device support

[`adding-devices.md`](adding-devices.md) is the end-to-end guide, from discovery and reverse engineering through the code steps to hardware verification. Assistants run it through the `add-device` skill (`.claude/skills/add-device/SKILL.md`).

## Verifying specification data

The `dump_specs` example prints each device's `Protocol::spec_sheet`
(resolution, accuracy, input impedance, notes) for side-by-side comparison
with the PDF manuals in `references/`. Devices without spec data are skipped
unless named.

```sh
# Dump all devices
cargo run -p dmm-lib --example dump_specs

# Dump a specific device
cargo run -p dmm-lib --example dump_specs -- ut61b+

# Multiple devices
cargo run -p dmm-lib --example dump_specs -- ut61eplus ut61d+
```

The default text output lists every mode and range, as the GUI's
Specifications panel shows them; pipe it to `less` or a file.

`--format json <id>` prints one device's tables in the shape of a manual
transcription, to diff against a verified transcription.

`--format html <id>` prints a review sheet: each table laid out as the manual
prints it, beside the render of its manual page. Render the pages first and
point `--pages-dir` at them. The page paths are written as given, so give
them relative to where the sheet is saved:

```sh
# PDF pages FIRST to LAST become pages/p-NN.png
pdftoppm -r 200 -png -f FIRST -l LAST manual.pdf pages/p
cargo run -p dmm-lib --example dump_specs -- --format html \
  --pages-dir pages --marks marks.json <id> > sheet.html
```

`--marks` is optional: a JSON file of notes that highlights the cells it
names, such as a transcription's `provenance.json`. Its format and the JSON
shape are in the header of `crates/dmm-lib/examples/dump_specs.rs`.

## Triaging a capture report

`dmm-cli triage <report>` reads a report a `capture` run wrote, with this
build's parser and no meter. It takes the meter from the report's
`device_id`; a v0.6.0 report has none, so name it, as in
`dmm-cli --device <id> triage <report>`. A plan run's expectations need its
plan file, passed as `--plan`.

It prints four sections:
- a header: the run's tier, sessions, truncation and diagnostics;
- findings, one per line, tagged by kind;
- each step's distinct readings;
- the stats: flags never or always set, and the modes, units and ranges
  seen.

The kinds of finding:
- `[error]`, `[timeout]` and `[attention]` repeat what the capture filed.
- `[lcd]` sets the operator's text beside this build's reading, and tags a
  difference in the unit alone.
- `[stale]` is a step whose first sample shows the state the step before
  it ended in.
- `[reparse]` counts samples this build reads differently from the build
  that wrote the report. The details follow the findings.
- `[unrecognised]` is data the parser reported as unknown.

A hardware address is masked wherever it appears. The step summary carries
no times, so diffing two reports' output compares the same steps across
runs.

## Golden file tests

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

To see how the apps show a fixture's frame, open it as a recording:
`dmm-gui --replay crates/dmm-lib/tests/golden/<id>/<case>.yaml` (or
`dmm-cli read --replay …`) plays that one frame from the meter the directory
names, so a meter nobody here owns can still be looked at.

## Generated doc tables

Both device tables sit between `<!-- devices:start -->` and `<!-- devices:end -->`
markers, and a `dmm-cli` test guards each. The one in `docs/cli-reference.md` is
rendered from the registry: after changing an entry or the `DEVICES` order run
`UPDATE_DOCS=1 cargo test -p dmm-cli` to rewrite it rather than editing it by
hand, or the test fails with a diff. The one in `README.md` is hand-written on
purpose — the test only checks that every protocol family and every
verification issue still appears in it, one issue per row at most.

## Doc screenshots and snippets

Both come from the same recorded meter sessions, so a reader sees one bench
run rather than a simulation. Record a session once with
`dmm-cli read --format replay -o <name>.replay`, commit it under
`assets/replays/`, and point a snippet or a scene at it.

The command-output blocks in `README.md` and `docs/cli-reference.md` sit
between `<!-- snippet via=… -->` and `<!-- /snippet -->`, with the commands
themselves in the opening marker. A `dmm-cli` test runs each one and compares
what it prints with the block. `via=mock[:<mode>]` runs the command against
the mock device, and `via=<name>.replay` against
`assets/replays/<name>.replay`. The block shows the command as a user would
type it, without either flag. A plain `cargo test -p dmm-cli` fails with a
diff; `UPDATE_DOCS=1 cargo test -p dmm-cli` rewrites it.

The GUI pictures come from `scripts/doc-screenshots.sh all`, which stages one
scene per picture on the [headless display](#headless-gui-checks): a
recording, the settings the picture needs, and the keys, clicks and window
size it is taken at. `list` prints the pictures, and naming one takes only its
scene. A rerun stages the same frame, and each capture prints how many pixels
it differs from the committed file. X rendering is not identical across driver
versions, so look at each changed PNG before committing it. The two settings
pictures and the connection-help one need the USB cable out; with a cable
plugged in they skip themselves with a message. The script's header and scene
comments carry the rest.

`cargo test -p dmm-gui --test doc_screenshots` fails when a doc shows a
picture no scene writes, or a scene writes one no doc shows.

What each recording in `assets/replays/` holds is listed beside `REPLAYS` in
`scripts/doc-screenshots.sh`; a snippet names its own in its `via=` marker.

## Headless GUI checks

`.claude/skills/verify-gui/scripts/gui-display.sh` runs `dmm-gui` against the
mock device on a private Xvfb display. Screenshots, contrast measurements and
keyboard or click tests never touch your live desktop session or your
`settings.json`. It needs `xvfb`, `xdotool` and `imagemagick` (plus
`python3-pil` for pixel measurement). Check the setup with:

```sh
.claude/skills/verify-gui/scripts/gui-display.sh selftest
```

The subcommands and what each one waits for are in the `verify-gui` skill
(`.claude/skills/verify-gui/SKILL.md`). Always finish with `stop`.

## Hidden flags

These contributor flags are left out of `--help`.

**Report triage.** `dmm-cli triage <report>`; see [Triaging a capture
report](#triaging-a-capture-report).

**Session clock.** `--mock-clock-preseed <SECS>` hands out that many seconds
of session time instantly, and `--mock-clock-scale <FACTOR>` runs what follows
at `FACTOR` times real speed. They let a run start with history rather than
wait for it. Both apply to the mock only: in `dmm-gui` either one implies
`--device mock`, and an explicit hardware `--device` is refused.

```sh
.claude/skills/verify-gui/scripts/gui-display.sh run --mock-mode dcv --mock-clock-preseed 90
```

The first `shot` then shows 90 s of readings. Pin a mock mode as above: the
auto-cycling mock changes scenario on its own, and the graph restarts on each
change. The burst is spent once per process, so a reconnect does not replay
it. `dmm-cli read` takes the same two flags, with `--device mock` spelled out.
`--replay <FILE>` takes them in place of `--mock-mode`, and opens the
recording at the instant the preseed names.

**Replay without waiting.** `dmm-cli read --replay <FILE> --mock-clock-scale
max` plays every frame at its recorded timestamp as fast as it decodes, and
ends with the file: a day-long recording converts in seconds. The public
`--import` ([CLI reference](cli-reference.md#dmm-cli-read)) is this for a
replay file, and also reads CSV and JSON exports. `max` needs `--replay` and
refuses `--mock-clock-preseed`, and the GUI refuses `max`, since a session
would race through the whole file.

**Update notice.** A local build never checks GitHub (see [Shared build
matrix](#shared-build-matrix)). `dmm-gui --update-notice <TAG>` shows the top
bar's link and the **Check for new versions** row for a tag such as `v0.8.0`
or `dev-f5ff045`, without a request and without writing the cache. The link
follows that row, which is saved to the real `settings.json` like any other
setting: untick it and the link goes, in this build and in a downloaded one
reading the same file.

## AI-assisted development

`CLAUDE.md` holds the project context and rules an assistant works by.
`.claude/rules/` holds the path-scoped rules that load when their files are
touched, and `.claude/skills/` the on-demand workflows; `CLAUDE.md` lists
both. Before changing a family's protocol code, read its
`docs/research/<family>/reverse-engineered-protocol.md`.

## Maintaining

### Release process

1. Reread `## Unreleased` in `CHANGELOG.md` against `.claude/rules/changelog.md` and reorder it: entries by importance, sections in the rule's order
2. Rename the heading to the version. If the release has a theme, add a short tagline stating what it changes in scope or intent: `## v0.2.0 — Multi-Device Protocol Support`. Open the section with a one- or two-sentence summary of the intent and the main areas touched. Close it with the `**Full Changelog**` compare link, as the existing entries do
3. Set the release version in root `Cargo.toml` (workspace inherits it), e.g. `version = "0.3.0"`
4. Update `Cargo.lock`: `cargo update --workspace`
5. With the USB cable unplugged — three scenes skip themselves otherwise — regenerate the GUI pictures with `scripts/doc-screenshots.sh all`, review the deltas and the PNGs, and commit the ones whose changelog entry changed what they show
6. Run `scripts/package-docs.py <dir>` (needs `pandoc`): a dead relative link in a shipped doc fails the tagged build
7. Commit: `git commit -am "Release v0.3.0"`
8. Push the release commit and wait for CI to go green: `git push`
9. Tag and push the tag, which publishes the release: `git tag v0.3.0 && git push origin v0.3.0`
10. The `release.yml` workflow takes over from the tag:
    - It builds binaries for all supported platforms: Linux x86_64/ARM, Windows x86_64/ARM, macOS ARM/Intel.
    - It creates a GitHub Release titled `v0.3.0 — <tagline>`, or just `v0.3.0` without one, with the changelog entry as the body. A `[@user](https://github.com/user)` credit becomes an `@user` mention there, and only there.
    - It fails before building anything if the tag does not match the workspace `version` in `Cargo.toml`, and fails if `CHANGELOG.md` has no `## v0.3.0` heading.
11. Bump to the next dev version: set `version = "0.4.0-dev"` in `Cargo.toml`, run `cargo update --workspace`, put an empty `## Unreleased` back above the released heading (the nightly notes append that section), commit, and push

### GitHub Actions workflows

Four workflows in `.github/workflows/`. None of them need touching to work on
the crates:

- `ci.yml` — fmt, clippy, tests and the dependency policy on every push and pull
  request, plus a three-target build so platform-specific breakage shows up
  early.
- `build-matrix.yml` — the six-target release build, called by the three others.
- `release.yml` — runs on a `v*` tag, see [Release process](#release-process).
  Dispatched by hand against a branch it builds all six targets and publishes
  nothing; against a tag it overwrites that release's assets and notes.
- `dev-build.yml` — the nightly prerelease.

#### Linting the workflows

`ci.yml` runs [actionlint](https://github.com/rhysd/actionlint) over
`.github/workflows/`. To run it locally, install `actionlint` and `shellcheck` —
without shellcheck it silently skips the bash inside `run:` blocks:

```sh
actionlint
```

#### Dependency policy

`ci.yml` runs [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) over the
dependency graph: RustSec advisories, the licence allow-list in `deny.toml`,
wildcard version requirements and unknown registries. Locally:

```sh
cargo install --locked cargo-deny
cargo deny check
```

The allow-list holds exactly the SPDX ids the graph needs today, so a new
dependency on an unlisted licence fails the check — read the licence, decide
whether it belongs in a GPL-3.0-or-later binary, and only then add the id,
both there and in `scripts/third-party-licenses.toml`'s `accepted` list. The
check covers the six targets the release ships; a new target in
`build-matrix.yml` goes into the `targets` lists of both files too.

Every release archive carries `THIRD-PARTY-LICENSES.html`, the licence texts
and copyright notices of the code compiled into the binaries. `build-matrix.yml`
generates its body with [cargo-about](https://github.com/EmbarkStudios/cargo-about)
from `scripts/third-party-licenses.toml` and the `.hbs` template beside it, and
`scripts/package-docs.py` renders it in the user docs' style. A crate's
declared licence can leave out what it compiles in (vendored C, fonts,
generated tables), so the `.toml` clarifies those crates with checksummed
licence files. A crate update that changes one of those files fails the build:
re-read it, then update the entry. Locally, with a cargo-about binary from its
releases:

```sh
cargo about generate --workspace --locked --fail -c scripts/third-party-licenses.toml \
    scripts/third-party-licenses.hbs -o /tmp/third-party.html
scripts/package-docs.py /tmp/package-docs /tmp/third-party.html
```

`.github/dependabot.yml` opens weekly update pull requests for the Cargo and
Actions dependencies, with minor and patch bumps grouped into one PR per
ecosystem and majors left on their own.

#### Shared build matrix

`ci.yml`, `release.yml` and `dev-build.yml` all call `build-matrix.yml`, so a
nightly dev build exercises the same packaging path a release does, and the
targets CI builds cannot drift from the ones a release ships. Its inputs are
described at the top of the file. CI builds the three-target `subset: ci`
without uploading, and is the only caller setting `cache: true`: the 10 GB
repository cache is worth more to pull-request turnaround than to unattended
builds.

`dev-build.yml` and a tagged `release.yml` run pass `published: true`, which
builds with `DMM_PUBLISHED_BUILD=1`. Only those binaries check GitHub for a
newer release: any other build's commit matches no published one, so it would
read every nightly as newer. To try the real request locally, build with
`DMM_PUBLISHED_BUILD=1`. It caches its answer in `update-check.json` beside
`settings.json`; delete that file to ask again within the day.

The archives carry only the user docs, as Markdown and as HTML, with LICENSE,
THIRD-PARTY-LICENSES.html and the images the docs show. `scripts/package-docs.py` assembles them once per run for
every archive. Relative links to any other doc become GitHub links at the built
commit, and a link to a missing file fails the build. A new user doc goes into
its `DOCS` list. To look at the result locally (needs `pandoc`):

```sh
scripts/package-docs.py /tmp/package-docs
```

#### Dev builds

`dev-build.yml` publishes a prerelease from `main` every night, skipping the run
when `main` has not moved. Each build gets its own immutable `dev-<short sha>`
tag — tags are never moved — and all but the newest few dev releases
(`KEEP_DEV_RELEASES` in `dev-build.yml`) are deleted automatically, tag
included. Nothing here needs doing by hand; do not
create or edit `dev-*` tags yourself. Trigger one early with
`gh workflow run dev-build.yml` (add `-f force=true` to rebuild a commit that
already has a dev release).

The prerelease body comes from `.github/dev-release-notes.md` with the
`## Unreleased` changelog section appended, which is another reason to keep that
section current as changes land. Keep each paragraph in that template on a
single line, however long: GitHub renders a single newline in a release body as
a hard line break, so wrapped prose comes out broken mid-sentence. `CHANGELOG.md`
is written the same way for the same reason.
