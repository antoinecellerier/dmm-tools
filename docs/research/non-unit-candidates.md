# Non-UNI-T Device Candidate Analysis

Research conducted 2026-03-20. Evaluating viable non-UNI-T devices for adding support,
prioritizing community adoption data and implementation feasibility within the existing
crate architecture.

## Summary Ranking

| Rank | Device | Label | Infrastructure | Protocol Docs | Community |
|------|--------|-------|---------------|---------------|-----------|
| 1 | Voltcraft VC880/VC650BT | Easiest | None (CP2110 reuse) | Full (pylablib) | Modest |
| 2 | Brymen BM869 | Most popular | New HID transport | Manufacturer docs + sigrok | Strong |
| 3 | OWON XDM1041 | Popular bench | SCPI + USB TMC | Official manual | Good |
| 4 | Owon B35T+/OW18E | Popular BLE | BLE transport (`btleplug`) | Community RE | Good |

**Recommendation:** VC880/VC650BT for exercising the `adding-devices.md` workflow with minimal infrastructure risk. Brymen BM869 for maximum community impact if willing to invest in a new transport backend.

---

## 1. Voltcraft VC880 / VC650BT

**The lowest-friction candidate.** Reuses existing CP2110 transport and AB CD BE16 framing with zero infrastructure changes.

### Community Adoption
- **EEVBlog:** 1 dedicated thread ("VOLTCRAFT VC650BT multimeter"), 10 replies. Niche — participants noted "very hard to find much on Voltcraft."
- **pylablib:** Complete `VC880` Python driver (MIT-licensed). Tested with VC-880. Docs say it "might also work with VC650T."
- **Conrad:** Published "Protocol Rev 2.4 VC650BT DESKTOP DMM" specification (PDF behind navigation page, not fully retrieved).
- **sigrok:** VC-890 listed as "planned" with `conn=hid/cp2110`. VC-880/VC650BT have no sigrok driver.

### Technical Profile
- **Transport:** CP2110 HID-to-UART (VID `0x10C4`, PID `0xEA80`) — identical to UT61E+
- **Protocol:** AB CD header, BE16 checksum, 33-byte measurement payload, streaming (no trigger)
- **Chipset:** ES51966A (Cyrustek) + MSP430F5418 + BU9799KV LCD + CP2110
- **Modes:** 19 (DC/AC V, mV, Hz, duty, resistance, diode, continuity, cap, temp C/F, uA/mA/A DC/AC, LPF)
- **Commands:** Autorange enable (0x47), disable (0x46)
- **Counts:** 40,000
- **Activation:** User must press `PC` button on meter

### Implementation Complexity: Low
- Reuses `Cp2110Transport` as-is
- Reuses `extract_frame_abcd_be16()` as-is
- New `Protocol` impl following UT8803 pattern
- No new dependencies
- Two devices covered by one implementation

### Reference Implementations
| Project | Stars | Language | Quality |
|---------|-------|----------|---------|
| pylablib `VC880` class | (part of large lib) | Python | Complete, working, MIT |

### Why Lowest-Friction
1. Same CP2110 bridge — zero transport work
2. Same AB CD framing — frame extraction reuses directly
3. pylablib provides complete byte-level protocol documentation
4. Two devices for one implementation
5. 19 modes with streaming and commands — enough complexity to exercise the full workflow

---

## 2. Brymen BM869

**Highest community engagement.** Professional-grade meter frequently compared to Fluke 289. But requires a new transport backend.

### Community Adoption
- **EEVBlog:** 7+ dedicated threads — most of any candidate. Topics: vs Fluke 289, industrial electrician reviews, data logging, vs Hioki DT4282
- **sigrok:** Fully supported driver (written by Aurelien Jacobs, in libsigrok)
- **Manufacturer:** Brymen provided protocol documentation
- **Market position:** 50,000 counts (500k DC), CAT IV 1000V, dual display

### Technical Profile
- **Transport:** IR-to-USB via Brymen BU-86X cable (USB HID)
- **Protocol:** USB/HID, proprietary segment-based encoding
- **Counts:** 50,000 (500,000 DC)
- **CAT rating:** CAT IV 1000V

### Implementation Complexity: High
- Requires **new `Transport` implementation** for the BU-86X HID cable
- New protocol family from scratch
- IR optical link adds a physical layer concern (cable compatibility)
- Protocol is manufacturer-documented + sigrok driver available, so RE effort is minimal

### Reference Implementations
| Project | Stars | Language | Quality |
|---------|-------|----------|---------|
| sigrok `brymen-bm86x` | (part of libsigrok) | C | Production-quality, well-tested |

### Why Not Recommended First
- New HID transport backend is significant infrastructure work
- IR optical cable is an additional hardware requirement
- Better suited as a second non-UNI-T device after the transport abstraction is proven with a simpler case

---

## 3. OWON XDM1041

**Popular bench DMM with official SCPI documentation.** But SCPI is a fundamentally different protocol paradigm.

### Community Adoption
- **GitHub:** 63 stars (TheHWcave/OWON-XDM1041) — Python3 recording utility with complete SCPI command list
- **sigrok:** XDM2041 listed on wiki
- **Documentation:** Official programming manual available from OWON (XDM1000 series)
- **Market:** Popular budget bench DMM, widely available on Amazon

### Technical Profile
- **Transport:** SCPI over USB (or LAN). XDM2041 also supports RS232
- **Protocol:** Standard SCPI text-based commands (ASCII `*IDN?`, `MEAS:VOLT:DC?`, etc.)
- **Interface:** USB TMC (Test & Measurement Class)

### Implementation Complexity: Medium-High
- SCPI is text-based — completely different from the binary framing used by all current devices
- Would need USB TMC transport or CDC serial transport
- `Protocol` trait assumes binary frames, not text command/response pairs
- Official documentation eliminates RE effort entirely

### Reference Implementations
| Project | Stars | Language | Quality |
|---------|-------|----------|---------|
| TheHWcave/OWON-XDM1041 | 63 | Python | Complete SCPI command coverage |
| martin-bochum/Multimeter | — | Python | Multi-DMM control (XDM3041/3051) via USB/TCP/RS232 |

### Why Not Recommended First
- SCPI is a different paradigm — "feels like a different project"
- Would require rethinking the `Protocol` trait abstraction
- Better suited as a future expansion if the project grows to support bench instrument protocols

---

## 4. Owon B35T+ / OW18E

**Popular budget BLE meters with very simple protocol.** But requires adding Bluetooth transport.

### Community Adoption
- **EEVBlog:** 4 threads (teardown, AvE review, Linux BLE client, OW18E app issues)
- **GitHub:** 68 stars total across repos
  - sercona/Owon-Multimeters: 34 stars, 4 forks (B35T+, B41T+, CM2100B, OW18E)
  - DeanCording/owonb35: 34 stars, 5 forks (last updated 2018)
- **Hackaday:** Project for B35T Bluetooth data logging
- **YouTube:** AvE reviewed the OWON B35

### Technical Profile
- **Transport:** Bluetooth Low Energy (BLE) — no USB at all
- **Protocol:** 3 uint16 values per measurement packet
  - uint16 #1: function/scale/decimal places
  - uint16 #2: reading type (hold, delta, autorange, battery, min/max)
  - uint16 #3: measurement value (signed magnitude)
- **Example packet:** `24 f0 04 00 00 00`

### Implementation Complexity: High
- Requires **new BLE transport** (`btleplug` crate) — a major new dependency
- Protocol itself is trivial (3 integers)
- BLE introduces platform-specific concerns (Linux BlueZ, macOS CoreBluetooth, Windows WinRT)

### Reference Implementations
| Project | Stars | Language | Quality |
|---------|-------|----------|---------|
| sercona/Owon-Multimeters | 34 | Python | Multi-device BLE, maintained |
| DeanCording/owonb35 | 34 | C | Stable but unmaintained (2018) |
| JayTee42/ow18b | — | C | Linux BLE |

### Why Not Recommended First
- BLE is a completely new transport type and dependency
- Platform-specific BLE stack adds cross-platform complexity
- Simple protocol doesn't exercise the adding-devices workflow thoroughly

---

## Other Devices Investigated (Not Viable)

| Device | Transport | Why Excluded |
|--------|-----------|-------------|
| Victor 70C/86C | Unmarked SO-20 HID chip (not CP2110) | Unknown bridge chip, no docs |
| Tenma 72-7730/7732 | CH9325 / HE2325U (UT-D04 cable) | UNI-T rebrands, not "non-UNI-T" |
| Voltcraft VC-870 | CH9325 (UT-D04 cable) | Not CP2110, different transport |
| Voltcraft VC-890 | CP2110 (`10C4:EA80`) | Protocol completely undocumented, sigrok "planned" only |

**Voltcraft VC-890 note:** Uses CP2110 and has ES51997P + EFM32 MCU. Would reuse transport, but lacks any protocol documentation or community implementation. Much harder than VC880/VC650BT. Could become viable if sigrok adds support.

## Decision Criteria

The ranking weighs these factors for the specific goal of exercising the `adding-devices.md` workflow:

1. **Infrastructure reuse** (highest weight) — Does it use CP2110? Does framing match? New dependencies?
2. **Protocol documentation** — Is the wire protocol fully known? From what source?
3. **Workflow coverage** — Does it exercise enough of the adding-devices phases to be a meaningful test?
4. **Community adoption** — Are there users who would benefit? Reference implementations to validate against?

For a different goal (e.g., maximizing user impact), Brymen BM869 would rank first despite higher implementation cost.
