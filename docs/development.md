# Development

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

## Adding a New Device Model

1. Create `crates/ut61eplus-lib/src/tables/new_model.rs`
2. Implement the `DeviceTable` trait with mode/range tables for the new model
3. Register it in `tables/mod.rs`

## Release Process

1. Update version in root `Cargo.toml` (workspace inherits it)
2. Run full check: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings`
3. Tag the release: `git tag -a v0.x.y -m "Release v0.x.y"`
4. Build release binaries: `cargo build --workspace --release`
5. Binaries are in `target/release/ut61eplus` (CLI) and `target/release/ut61eplus-gui` (GUI)
