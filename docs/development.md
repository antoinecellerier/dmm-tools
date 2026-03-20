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

## Adding a New Device Model (same protocol family)

To add a new meter that shares an existing protocol (e.g. another UT61x variant):

1. Create `crates/ut61eplus-lib/src/protocol/ut61eplus/tables/new_model.rs`
2. Implement the `DeviceTable` trait with mode/range tables for the new model
3. Register it in `protocol/ut61eplus/tables/mod.rs`

## Adding a New Protocol Family

To add support for a device family with a different wire protocol:

1. Create `crates/ut61eplus-lib/src/protocol/newfamily/mod.rs`
2. Implement the `Protocol` trait (`init`, `request_measurement`, `send_command`, `get_name`, `profile`, `capture_steps`)
3. Add a variant to the `DeviceFamily` enum in `protocol/mod.rs` and update `Display`
4. Add the match arm in `open_device()` in `lib.rs`
5. Add a `SelectableDevice` entry in `protocol/registry.rs` with a factory function
6. Create research docs in `docs/research/newfamily/` documenting the wire protocol
7. Set `Stability::Experimental` in the `DeviceProfile` until verified against real hardware

That's it — the CLI and GUI automatically pick up new devices from the registry.
No app code changes needed. The `--device` help text, GUI device selector, activation
instructions, and protocol dispatch all come from the registry entry.

**Example: adding a hypothetical UT99X family**

```rust
// 1. In protocol/mod.rs — add the variant:
pub enum DeviceFamily {
    // ...existing variants...
    Ut99x,
}

// 2. In protocol/registry.rs — add factory + entry:
fn new_ut99x() -> Box<dyn Protocol> {
    Box::new(Ut99xProtocol::new())
}

// Add to DEVICES slice:
SelectableDevice {
    id: "ut99x",
    display_name: "UT99X",
    aliases: &["ut99xa", "ut99xb"],
    requires_hardware: true,
    activation_instructions: "1. Connect USB\n2. Turn on meter",
    family: DeviceFamily::Ut99x,
    new_protocol: new_ut99x,
},

// 3. In lib.rs — add the match arm in open_device():
DeviceFamily::Ut99x => Box::new(Ut99xProtocol::new()),
```

The `Protocol` trait is object-safe and `Send`, so the new family works automatically
with `Dmm<T>`, the CLI, and the GUI.

## Golden File Tests

Golden file tests verify measurement parsing against known-good byte sequences.
Each `.yaml` file in `crates/ut61eplus-lib/tests/golden/{family}/` uses the same
format as capture YAML samples (`raw_hex`, `mode`, `value`, `unit`, `range_label`,
`flags`). This means you can copy a sample directly from a capture report into a
golden file.

To add a golden test:

1. Run `ut61eplus --device <family> capture` and complete the steps
2. Open the capture YAML and find a sample with known-good values
3. Copy the sample fields into a new `.yaml` file in `tests/golden/{family}/`
4. Run `cargo test --workspace` to verify

Golden tests run as part of the standard test suite. They are the primary
regression safety net for protocol parsing — add them whenever you verify
a new mode/range/flag combination against real hardware.

## Release Process

1. Set the release version in root `Cargo.toml` (workspace inherits it), e.g. `version = "0.2.0"`
2. Commit: `git commit -am "Release v0.2.0"`
3. Tag: `git tag v0.2.0 && git push && git push origin v0.2.0`
4. The `release.yml` GitHub Actions workflow builds Linux and Windows binaries and creates a GitHub Release automatically
5. Bump to the next dev version: set `version = "0.3.0-dev"` in `Cargo.toml`, commit, and push

## Shell Completions

Generate completions for your shell:

```sh
ut61eplus completions bash > ~/.local/share/bash-completion/completions/ut61eplus
ut61eplus completions zsh > ~/.zfunc/_ut61eplus
ut61eplus completions fish > ~/.config/fish/completions/ut61eplus.fish
ut61eplus completions powershell >> $PROFILE
```
