# Contributing

## Bug reports

When filing a bug report, please include:

1. What you expected vs. what happened
2. Your meter model (printed on the meter, e.g. "UT61E+")
3. OS and Rust version (`rustc --version`)
4. A protocol capture (see below) — this is the single most useful thing you can attach

## Protocol captures

The protocol was reverse-engineered, not documented by UNI-T. Captures from real devices are essential for finding bugs and adding support for new models.

### Running a capture

The built-in capture wizard walks you through each measurement mode and flag:

```sh
cargo run --bin ut61eplus -- capture
```

It will:
- Guide you to set specific modes on the meter (DC V, AC V, ohms, etc.)
- Record raw bytes and parsed values for each step
- Ask you to confirm what the meter's LCD actually shows
- Save everything to a YAML file (e.g. `capture-ut61e+.yaml`)

You can run a partial capture if you only want specific steps:

```sh
# List available steps
cargo run --bin ut61eplus -- capture --list-steps

# Run only specific steps
cargo run --bin ut61eplus -- capture --steps dcmv,temp,duty
```

Captures auto-save after each step, so you can interrupt and resume later.

### What to do with the capture

Attach the YAML file to your GitHub issue. If your meter is an unsupported model, the capture is especially valuable — the tool will flag this automatically.

### Other models

If you have a UNI-T meter that uses the same CP2110 USB adapter (e.g. UT61B+, UT61D+), we'd love captures from it. The capture wizard will note that it's an unknown model and prompt you to complete as many steps as possible.

## Code changes

1. Fork and create a feature branch
2. Make sure `cargo clippy --workspace -- -D warnings` and `cargo test --workspace` pass
3. Include tests for new functionality
4. For protocol changes: verify against a real device (`RUST_LOG=ut61eplus_lib=trace cargo run --bin ut61eplus -- debug`)
5. Open a pull request with a description of what and why
