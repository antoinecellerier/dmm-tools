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

## Linting

```sh
cargo clippy --workspace -- -D warnings
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
2. Implement the `DeviceTable` trait with mode/range tables
3. Register in the family's `tables/mod.rs`
4. Add `SelectableDevice` entry in `protocol/registry.rs`

**New protocol family:**

1. Create `crates/dmm-lib/src/protocol/newfamily/mod.rs`
2. Implement the `Protocol` trait (`init`, `request_measurement`, `send_command`, `get_name`, `profile`, `capture_steps`)
3. Add variant to `DeviceFamily` enum in `protocol/mod.rs`
4. Add match arm in `open_device()` in `lib.rs`
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
Each `.yaml` file in `crates/dmm-lib/tests/golden/{family}/` uses the same
format as capture YAML samples (`raw_hex`, `mode`, `value`, `unit`, `range_label`,
`flags`). This means you can copy a sample directly from a capture report into a
golden file.

To add a golden test:

1. Run `dmm-cli --device <family> capture` and complete the steps
2. Open the capture YAML and find a sample with known-good values
3. Copy the sample fields into a new `.yaml` file in `tests/golden/{family}/`
4. Run `cargo test --workspace` to verify

Golden tests run as part of the standard test suite. They are the primary
regression safety net for protocol parsing — add them whenever you verify
a new mode/range/flag combination against real hardware.

## Release Process

1. Write the release entry in `CHANGELOG.md` (see existing entries for format). If the release has a theme, put a short tagline in the heading — `## v0.2.0 — Multi-Device Protocol Support` — stating what it changes in scope or intent, and open the section with a one- or two-sentence summary of the intent and main areas touched
2. Set the release version in root `Cargo.toml` (workspace inherits it), e.g. `version = "0.3.0"`
3. Update `Cargo.lock`: `cargo update --workspace`
4. Update the README screenshot if the GUI has changed
5. Commit: `git commit -am "Release v0.3.0"`
6. Tag and push: `git tag v0.3.0 && git push && git push origin v0.3.0`
7. The `release.yml` GitHub Actions workflow builds binaries for all supported platforms (Linux x86_64/ARM, Windows x86_64/ARM, macOS ARM/Intel) and creates a GitHub Release with the changelog entry as the body, titled `v0.3.0 — <tagline>` (or just `v0.3.0` without one). The workflow fails if `CHANGELOG.md` has no `## v0.3.0` heading
8. Bump to the next dev version: set `version = "0.4.0-dev"` in `Cargo.toml`, run `cargo update --workspace`, commit, and push

## Shell Completions

Generate completions for your shell:

```sh
dmm-cli completions bash > ~/.local/share/bash-completion/completions/dmm-cli
dmm-cli completions zsh > ~/.zfunc/_dmm-cli
dmm-cli completions fish > ~/.config/fish/completions/dmm-cli.fish
dmm-cli completions powershell >> $PROFILE
```

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
- Run `cargo clippy --workspace -- -D warnings` and `cargo test --workspace`
  before committing

The `docs/research/` directories contain per-family reverse engineering notes
that provide essential context for protocol work. The assistant should read
the relevant `reverse-engineered-protocol.md` before modifying protocol code.
