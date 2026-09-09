# Development

## Setup

After cloning, install the pre-commit hooks:

```sh
ln -sf ../../git-hooks/pre-commit .git/hooks/pre-commit
```

This runs `cargo fmt --check`, `cargo clippy`, and `cargo test` before each commit.

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
2. Implement `ModeTables`: one `entry` match returning the ranges and specs per mode (`DeviceTable` is derived)
3. Register in the family's `tables/mod.rs`
4. Add `SelectableDevice` entry in `protocol/registry.rs`

**New protocol family:**

1. Create `crates/dmm-lib/src/protocol/newfamily/mod.rs`
2. Implement the `Protocol` trait (`init`, `request_measurement`, `send_command`, `get_name`, `profile`, `capture_steps`)
3. Add variant to `DeviceFamily` enum in `protocol/mod.rs`
4. Name the family's USB cable in `preferred_transports()` in `lib.rs` (the transports themselves are in `KNOWN_TRANSPORTS`)
5. Add `SelectableDevice` entry in `protocol/registry.rs`
6. Create research docs in `docs/research/newfamily/`
7. Mark as experimental until verified against real hardware (the CLI prints a warning for non-UT61E+ families)

The CLI and GUI automatically pick up new devices from the registry — no app code changes needed.

## Verifying Specification Data

The `dump_specs` example prints all per-device specification data (resolution,
accuracy, input impedance, notes) in formatted tables for side-by-side
comparison with the PDF manuals in `references/`.

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
rendered from the registry: after changing `registry.rs` run
`UPDATE_DOCS=1 cargo test -p dmm-cli` to rewrite it rather than editing it by
hand, or the test fails with a diff. The one in `README.md` is hand-written on
purpose — the test only checks that every protocol family and every
verification issue still appears in it.

## Shell Completions

Generate completions for your shell:

```sh
dmm-cli completions bash > ~/.local/share/bash-completion/completions/dmm-cli
dmm-cli completions zsh > ~/.zfunc/_dmm-cli
dmm-cli completions fish > ~/.config/fish/completions/dmm-cli.fish
dmm-cli completions powershell >> $PROFILE
```

## Release Process

1. Write the release entry in `CHANGELOG.md` (see existing entries for format). If the release has a theme, put a short tagline in the heading — `## v0.2.0 — Multi-Device Protocol Support` — stating what it changes in scope or intent, and open the section with a one- or two-sentence summary of the intent and main areas touched
2. Set the release version in root `Cargo.toml` (workspace inherits it), e.g. `version = "0.3.0"`
3. Update `Cargo.lock`: `cargo update --workspace`
4. Update the README screenshot if the GUI has changed
5. Commit: `git commit -am "Release v0.3.0"`
6. Tag and push — **confirm with the maintainer first**, this publishes the release: `git tag v0.3.0 && git push && git push origin v0.3.0`
7. The `release.yml` GitHub Actions workflow builds binaries for all supported platforms (Linux x86_64/ARM, Windows x86_64/ARM, macOS ARM/Intel) and creates a GitHub Release with the changelog entry as the body, titled `v0.3.0 — <tagline>` (or just `v0.3.0` without one). The workflow fails before it builds anything if the tag does not match the workspace `version` in `Cargo.toml`, and fails if `CHANGELOG.md` has no `## v0.3.0` heading
8. Bump to the next dev version: set `version = "0.4.0-dev"` in `Cargo.toml`, run `cargo update --workspace`, commit, and push

## GitHub Actions workflows

Four workflows in `.github/workflows/`. None of them need touching to work on
the crates:

- `ci.yml` — fmt, clippy, tests and the dependency policy on every push and pull
  request, plus a three-target build so platform-specific breakage shows up
  early.
- `build-matrix.yml` — the six-target release build, called by the three others.
- `release.yml` — runs on a `v*` tag, see [Release Process](#release-process).
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
cheaper three-target build, leaves `upload-artifacts` off so a push or pull
request compiles the binaries and keeps nothing, and is the only caller setting
`cache: true`: the 10 GB repository cache is worth more to pull-request
turnaround than to the unattended release and nightly builds.

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
when their files are touched (`protocol.md` for `crates/dmm-lib/`, `gui.md`
for `crates/dmm-gui/`, `changelog.md` for `CHANGELOG.md`), and
`.claude/skills/` holds on-demand checklists: `add-device` (new-meter
onboarding), `issue-replies` (issue triage and GitHub reply drafting) and
`verify-gui` (headless GUI checks, below).

### Headless GUI checks

`.claude/skills/verify-gui/scripts/gui-display.sh` runs `dmm-gui` against the
mock device on a private Xvfb display, so screenshots, contrast measurements and
keyboard/click tests never touch your live desktop session or your
`settings.json`. Check the setup with:

```sh
.claude/skills/verify-gui/scripts/gui-display.sh selftest
```

It needs `xvfb`, `xdotool` and `imagemagick` (plus `python3-pil` for pixel
measurement). `start`, `run`, `key`, `click`, `shot`, `status` and `stop` are the
individual steps; always finish with `stop`.

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
same two flags, with `--device mock` spelled out.
