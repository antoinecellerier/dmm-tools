# Draft reply for antoinecellerier/dmm-tools#5

Status: pending — waiting for user approval before posting.

---

Thanks for reporting this! We looked into this and have good news: UNI-T's own "updated" UT181A PC software (V1.05) ships with **both** `SLABHIDtoUART.dll` (CP2110) and `CH9329DLL.dll`, so the CH9329 cable is officially supported by UNI-T. The [UT-D09 is listed on UNI-T's site](https://meters.uni-trend.com/product/ut-d-series-2/).

From analyzing the vendor software, both cables feed into the same protocol handler — the measurement protocol over UART is identical, only the USB transport layer differs.

**We've added CH9329 support** and would love your help testing it. Here's the quick version:

```sh
git clone https://github.com/antoinecellerier/dmm-tools.git
cd dmm-tools
cargo build --release

# Linux: install udev rule
sudo cp udev/99-dmm-tools.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
# (unplug and replug the cable)

# Verify detection
./target/release/dmm-cli list

# Try reading measurements (with full debug logging)
RUST_LOG=dmm_lib=trace ./target/release/dmm-cli --device ut181a debug --count 5
```

There's a detailed test plan with troubleshooting for different failure modes in [`docs/research/ut181/ch9329-test-plan.md`](https://github.com/antoinecellerier/dmm-tools/blob/main/docs/research/ut181/ch9329-test-plan.md).

There are two things that might need tweaking based on your hardware:
1. **HID interface selection** — if the CH9329 enumerates as a composite device (keyboard + mouse + custom HID), we might open the wrong interface. The test plan covers how to diagnose this.
2. **UART configuration** — the CH9329 might need explicit baud rate init before data flows. We're starting without it (assuming factory-configured). The TRACE logs will show if writes go out but nothing comes back.

A few other questions:
- Is this a cable that came bundled with the UT181A, or purchased separately?
- Where/when was it purchased? (Trying to understand if this is a recent production change.)
- Does UNI-T's official "DMM" PC software work with this cable?
