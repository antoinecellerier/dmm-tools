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
dmm-cli capture

# For non-UT61E+ meters, specify the device family
dmm-cli --device ut8803 capture
```

It will:
- Guide you to set specific modes on the meter (DC V, AC V, ohms, etc.)
- Record raw bytes and parsed values for each step
- Ask you to confirm what the meter's LCD actually shows
- Save everything to a YAML file (e.g. `capture-ut61e+.yaml`)

You can run a partial capture if you only want specific steps:

```sh
# List available steps
dmm-cli capture --list-steps

# Run only specific steps
dmm-cli capture --steps dcmv,temp,duty
```

Captures auto-save after each step, so you can interrupt and resume later.

### What to do with the capture

Attach the YAML file to your GitHub issue. If your meter is an unsupported model, the capture is especially valuable — the tool will flag this automatically.

### Other models

If you have any of the [supported meters](docs/supported-devices.md), we'd love captures — especially for experimental (unverified) devices. Use `--device <family>` to select your meter model, e.g. `dmm-cli --device ut171 capture`. See the [supported devices table](README.md#supported-devices) for verification issue links.

## Unconfirmed platform testing

macOS ARM (Apple Silicon), Linux ARM (Raspberry Pi 3B+), and Windows ARM have been confirmed working. The following platforms have builds but haven't been verified yet:

- **macOS Intel** — [issue #2](https://github.com/antoinecellerier/dmm-tools/issues/2)

If you have one of these platforms and a supported meter:

1. Download the appropriate build from [Releases](https://github.com/antoinecellerier/dmm-tools/releases), or build from source: `cargo build --workspace`
2. Plug in the USB adapter and run `dmm-cli list`
3. If the device is found, try `dmm-cli read` and `dmm-gui`
4. Comment on the relevant issue with your results — include your OS version, device, meter model, and whether readings were correct

Even "it works" is valuable. If something doesn't work, the output of `RUST_LOG=dmm_lib=trace dmm-cli read` will help us debug.

## Code changes

1. Fork and create a feature branch
2. Make sure `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace` pass (the pre-commit hook runs all three — see [development guide](docs/development.md))
3. Include tests for new functionality
4. For protocol changes: verify against a real device (`RUST_LOG=dmm_lib=trace dmm-cli debug`)
5. Open a pull request with a description of what and why
