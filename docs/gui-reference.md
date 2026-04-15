# dmm-gui — GUI Reference

<!-- Keep this file in sync with the GUI. If you add, remove, or change
     features, panels, or controls, update the relevant section here in the
     same commit. -->

## Name

**dmm-gui** — real-time graphing multimeter display for UNI-T and Voltcraft meters

## Synopsis

```
dmm-gui [OPTIONS]
```

## Description

A desktop GUI for live measurement display, time-series graphing, recording,
and remote control of UNI-T and Voltcraft multimeters.

The Settings panel includes a **Device** selector populated from the
device registry with all supported models (UT61E+, UT61B+, UT61D+,
UT161B/D/E, UT8802, UT8803, UT803, UT804, UT171A/B/C, UT181A,
Voltcraft VC-880, Voltcraft VC650BT, Voltcraft VC-890) and a **Mock (simulated)**
option. Each model selects the correct protocol tables (e.g., UT61B+
uses different mode/range mappings than UT61E+). The selection persists
across sessions and requires a reconnect to take effect. When connected
to an experimental (unverified) protocol, an orange **EXPERIMENTAL**
badge appears in the top bar. Clicking it opens the device's
verification issue on GitHub where you can report feedback.

The **Mock (simulated)** device generates synthetic measurements without
hardware, cycling through DC V, AC V, Ohms, Capacitance, Hz,
Temperature, DC mA, Overload, and NCV modes. When Mock is selected, a
**Mock mode** row appears in Settings with choices: **Auto (cycle)**
(default) or a specific mode (dcv, acv, ohm, cap, hz, temp, dcma,
ohm-ol, ncv). Selecting a specific mode pins the mock to that
measurement type indefinitely. Remote control buttons (HOLD, REL,
RANGE, etc.) respond to toggle flags. The SELECT button advances to
the next mode regardless of the auto-cycle setting.

![Wide layout — live measurement with graph, statistics, recording, and minimap](../assets/gui-wide-layout.png)

## Top Bar

The top bar contains:

- **App name and version** — click the version label to open the "What's
  New" changelog popup. On release upgrades, this popup opens automatically
  on first launch.
- **Connect / Disconnect** button
- **Pause / Resume** button — freezes data capture without disconnecting.
  Pauses longer than the gap threshold produce gap markers on the graph.
- **Clear** button — resets graph history and statistics (does not affect
  recording)
- **Connection status** — colored dot (green = connected, orange =
  reconnecting/paused, gray = disconnected) with device name
- **Settings gear** (right side) — opens the settings panel
- **Help link** — opens the project page

Toast notifications appear in the top-right corner (e.g. CSV export
success/failure) and expire after 4 seconds.

## Reading Display

![Reading display with HOLD and REL flags active, and remote control buttons](../assets/gui-reading-controls.png)

- Primary value in large monospace font, using the meter's raw 7-character
  display string for stable width (no jitter between readings)
- Unit shown adjacent (e.g. "V", "mV", "kΩ")
- Mode and range label below in smaller text
- Active flags shown as colored badges:
  - **AUTO** — auto-range active
  - **HOLD** — display frozen on meter
  - **REL** — relative/delta mode
  - **MIN**, **MAX** — min/max recording active
  - **LOW BAT** — low battery warning (orange)
- Overload ("OL") rendered in warning red

## Remote Control

A row of buttons shown when connected and receiving data (visible in the
[reading display screenshot above](#reading-display)):

| Button | Description |
|---|---|
| **HOLD** | Toggle hold mode |
| **REL** | Toggle relative mode |
| **RANGE** | Cycle manual range |
| **AUTO** | Return to auto-range |
| **MIN/MAX** | Click to enter or cycle MAX ↔ MIN. Shows stored value. **x** exits. |
| **PEAK** | Click to enter or cycle P-MAX ↔ P-MIN. Shows stored peak. **x** exits. |
| **SELECT** | Cycle sub-modes |
| **LIGHT** | Toggle backlight |

Buttons highlight blue when the corresponding flag is active in the current
measurement. LIGHT has no protocol feedback, so it does not highlight.

## Graph

![Graph with mean line, min/max envelope, reference lines, trigger markers, and cursors](../assets/gui-graph-overlays.png)

Three components stacked vertically: toolbar, main plot, and minimap.

### Toolbar

| Control | Description |
|---|---|
| **5s, 10s, 30s, 1m, 5m, 10m** | Time window presets |
| **LIVE** | Auto-scroll to latest data (green when active) |
| **Y:Auto / Y:Fixed** | Auto-scale Y axis, or enter fixed min/max values |
| **Reset Zoom** | Return to live follow with auto Y (enabled when the view has been zoomed or paused) |
| **Mean** | Dashed horizontal line at visible window average, labeled with value |
| **Min/Max** | Sliding-window envelope band showing value range. Window duration is configurable (default 1s). |
| **Ref** | Horizontal reference lines at user-specified values (comma/semicolon/space separated) |
| **Triggers** | (requires Ref) Diamond markers where data crosses a reference line |
| **Cursors** | Click to place cursor A, click again for cursor B. Shows ΔT, ΔV, and ∫ (integral, for current/voltage modes only). |

### Main Plot

- Time-series line plot with auto-scaling Y axis (10% padding)
- Axis labels include units (e.g. "1.0 mV", "10 s")
- Crosshair tooltip shows time and value with units
- Disconnection gaps shown as dashed red vertical line pairs
- Timeline is continuous across reconnects (data is not cleared)
- History buffer holds ~10,000 points (oldest dropped). Mode changes clear
  the graph since units are incompatible.

### Mouse Interactions

| Action | Effect |
|---|---|
| **Scroll wheel** (browse mode) | Zoom X axis centered on cursor (2s–3600s range) |
| **Scroll wheel** (live mode) | Exit live mode, jump to scrolled position |
| **Click & drag** | Pan left/right through history |
| **Shift + click & drag** | Draw a bounding box to zoom both time and value to the selected region. Release to apply; press Escape to cancel. |
| **Double-click** | Return to live mode with auto Y |
| **Click** (cursors active) | Place cursor A or B, snapping to nearest data point |

### Minimap

A thin strip below the main plot showing the full capture history.

- Bracket markers ([ ]) indicate the current viewport
- Click or drag the interior to jump to a specific time
- Drag the bracket edges to resize the viewport to an arbitrary time width
- Clicking near the end re-enables live mode

## Specifications

Shows per-range electrical specifications from the device manual, updated live
as the meter changes mode/range. Helps users understand the precision and
limitations of their current reading.

- **Resolution** — smallest increment the meter can display in the current range
- **Accuracy** — rated accuracy as ±(% of reading + counts). AC modes show
  separate accuracy for each frequency band (e.g., 40Hz–1kHz and 1kHz–10kHz).
  Temperature shows accuracy per sub-range (e.g., -40–0°C, 0–300°C).
  LPF mode shows its own accuracy (separate from AC V).
- **Input Z** — input impedance (e.g., ~10 MΩ), when applicable
- **Notes** — additional info like "True RMS", thermocouple type, fuse ratings
- **Manual** — hyperlink to the manufacturer's product page (shown whenever a
  URL is configured for the device, even without per-range spec data)

Panel visibility is controlled by the **Specifications** checkbox in Settings.
Default: on.

**Layout behavior:**

| Layout | Display style |
|---|---|
| Wide (≥ 900px) | Full panel in the left sidebar, between controls and statistics |
| Big meter | Pipe-separated inline summary, scaled with the reading |
| Narrow (< 900px) | Compact single line below the reading |

When no spec data is available (unsupported device or unrecognized mode), only
the Manual link is shown (if configured). If neither specs nor manual URL exist,
nothing renders.

**Coverage:** UT61E+, UT61B+, UT61D+, UT161B/D/E, and Mock (delegates to
UT61E+). Other devices show only the Manual link.

## Statistics

- **Min**, **Max**, **Avg** values in monospace with fixed-width formatting
- **Count** — number of samples
- **Int** — cumulative time-integral (shown only for current and voltage modes).
  For current modes, displays charge in Ah/mAh/µAh. For voltage modes, V·s.
  Uses the trapezoidal rule over the sample stream. Resets with the Reset button.
- **Reset** button — clears statistics and integral
- Stats persist across reconnects (use Clear for full reset)
- In wide layout, a second row shows **visible window stats** — min/max/avg
  computed only over the current graph viewport

## Recording

- **Record (●) / Stop (■)** toggle button
- **Export CSV** button — opens a file save dialog (runs on a background
  thread, does not freeze the UI)
- Sample counter and duration shown while recording
- Scrollable log of the last 500 samples showing timestamp, value, unit, mode,
  range, and flags
- Buffer holds up to 500K samples (~14 hours at 10 Hz). Recording
  auto-stops when the buffer is full and shows a toast notification.

**CSV format:**

```
# device: UT61E+
timestamp,mode,value,unit,range,flags
2026-03-19T10:15:30.123+01:00,DC V,3.3042,V,22V,AUTO
```

## Settings

Opened via the gear icon. Persisted to `~/.config/dmm-tools/settings.json` on Linux (XDG config dir under the `dmm-tools` project name; macOS and Windows use the equivalent platform-specific location).

| Setting | Default | Description |
|---|---|---|
| **Theme** | Dark | Dark or Light mode |
| **Colors** | Default | Color preset: Default, High Contrast, Colorblind. See [Color Customization](#color-customization) below. |
| **Show Graph** | on | Toggle graph panel visibility |
| **Show Statistics** | on | Toggle statistics panel visibility |
| **Show Recording** | on | Toggle recording panel visibility |
| **Show Specifications** | on | Toggle specifications panel visibility |
| **Auto-connect** | on | Connect to meter automatically on startup |
| **Query device name** | on | Ask meter for its name on connect (causes a beep) |
| **Sample interval** | 0 ms | Delay between measurements: 0 (fastest, ~10 Hz), 100, 200, 300, 500, 1000, 2000 ms. Requires reconnect. |
| **Device** | UT61E+ | Device family. See the description for supported models and Mock. Requires reconnect. |
| **Mock mode** | Auto (cycle) | Only shown when Device is Mock. Pins the mock to a specific measurement mode, or cycles through all modes. Requires reconnect. |
| **Zoom** | 100% | UI scale (30%–300%). Also controllable via keyboard. |
| **Always on top** | off | Keep the window above all other windows (`Ctrl+T`). On Wayland, use the title bar right-click menu or launch with `WAYLAND_DISPLAY=` to force X11. |
| **Hide window decorations** | off | Remove the title bar and window borders (`Ctrl+D`). Use Alt+drag (Linux) or the keyboard shortcut to restore. |

### Color Customization

Three color presets are available:

- **Default** — warm palette (red/pink graph line, green mean, orange cursor)
- **High Contrast** — bolder, higher-saturation colors for maximum visibility
- **Colorblind** — deuteranopia/protanopia safe palette (blue/orange/purple, avoids red-green)

Select a preset from the "Colors" row in the settings panel. Switching presets resets any per-color overrides.

**Per-color editing:** Expand "Customize colors" in the settings panel to see color swatches for all 18 base colors, grouped by category (UI, Graph, Status, Minimap). Click any swatch to open a color picker. Colors are edited for the current theme mode (dark or light) independently.

**JSON overrides:** Colors can also be edited directly in `settings.json` using hex strings:

```json
{
  "color_preset": "Default",
  "color_overrides": {
    "dark": {
      "background": "#1B1B1B",
      "graph_line": "#64C8FF"
    },
    "light": {
      "graph_line": "#0050A0"
    }
  }
}
```

Available color fields:

- **UI chrome:** `background`, `text`, `button`
- **Graph:** `graph_line`, `graph_gap`, `graph_mean`, `graph_ref`, `graph_crossing`, `graph_cursor`, `graph_envelope`, `plot_background`, `graph_crosshair`
- **Status:** `status_ok`, `status_warning`, `status_error`, `status_inactive`, `accent`
- **Minimap:** `minimap_viewport`

Format: `#RRGGBB` or `#RRGGBBAA`.

Derived colors auto-track their base: cursor dim/delta derive from cursor, minimap line from graph line, live indicator from status_ok, recording warning from status_warning. Button hover/active states derive from button. Plot grid and axis labels follow the UI chrome text color.

## Command-Line Options

All options override saved settings for the current session only — they
do not modify the persisted `settings.json`.

| Option | Description |
|--------|-------------|
| `--device <ID>` | Device family to connect to (e.g., `ut61eplus`, `ut181a`, `mock`). Run `--help` for the full list with aliases. |
| `--adapter <SERIAL_OR_PATH>` | Select a specific USB adapter when multiple are connected. Use serial number or HID device path from `dmm-cli list` output. |
| `--mock-mode <MODE>` | Pin mock device to a specific mode (only with `--device mock`). Modes: dcv, acv, ohm, cap, hz, temp, dcma, ohm-ol, ncv. |
| `--theme <THEME>` | Theme override: `dark`, `light`, or `system`. |
| `--renderer <RENDERER>` | Graphics renderer: `wgpu` (default) or `glow` (OpenGL, better compatibility on older GPUs). If wgpu fails at startup, glow is tried automatically. |
| `-V`, `--version` | Print version and exit. |
| `-h`, `--help` | Print help and exit. |

## Keyboard Shortcuts

Press `?` or click the `?` button in the top bar to open an in-app reference of keyboard shortcuts and mouse gestures.

### General

| Shortcut | Action |
|---|---|
| `Ctrl+Shift+C` | Connect / Disconnect |
| `Space` | Pause / Resume (when connected) |
| `Ctrl+L` | Clear graph & statistics |
| `Ctrl+R` | Toggle recording |
| `Ctrl+B` | Cycle big meter mode (off / full / minimal) |
| `Ctrl+T` | Toggle always on top |
| `Ctrl+D` | Toggle window decorations |
| `Ctrl+E` | Export CSV |
| `Ctrl+Plus` / `Ctrl+Minus` | Zoom in / out |
| `Ctrl+0` | Reset zoom to 100% |
| `Ctrl+Q` | Quit |
| `?` | Toggle keyboard & mouse help overlay |
| `Esc` / `Ctrl+W` | Close help overlay |

### Graph Navigation

| Shortcut | Action |
|---|---|
| `[` / `]` | Cycle to shorter / longer time window preset |
| `Left` / `Right` | Scroll view (exits live mode) |
| `Home` | Jump to start of data |
| `End` | Jump to live mode |

Graph and `Space` shortcuts are disabled when a text field (e.g. Y-axis
range, envelope window) has focus.

## Layout Modes

The layout adapts to the window size and panel visibility.

### Wide Layout (≥ 900px)

Two-column layout with a resizable left sidebar (180–400px):

- **Left column:** reading display, remote controls, connection help,
  specifications, statistics
- **Right column:** graph (top) and recording (bottom), separated by a
  draggable divider

### Narrow Layout (< 900px)

Single-column stack: reading, controls, help, specifications (compact),
statistics, graph, recording.

### Big Meter Mode

![Big meter mode — reading and statistics scaled to fill the window](../assets/gui-big-meter.png)

Activated when both graph and recording panels are hidden (via settings
or the toggle). The reading display scales to fill the available space —
useful as a bench-mount display or for presentations.

Use the **⊞** button (near the remote control buttons) or **Ctrl+B** to
quickly enter big meter mode — this temporarily hides graph, recording,
statistics, and specifications without changing your saved settings.
Press **Ctrl+B** again to enter **minimal mode**, which also hides the
top bar and command buttons, leaving only the reading and mode line.
Press **Ctrl+B** a third time to return to your normal layout.

![Minimal meter mode — reading only, no chrome](../assets/gui-minimal-meter.png)

If all panels are already hidden via settings, **⊞** restores all panels
to their defaults.

## Connection Help

Shown automatically when connection fails:

- **USB cable not found:** platform-specific instructions (Linux: udev rule
  install and `plugdev` group membership; Windows: Device Manager guidance to
  check if a driver is needed). All cable variants are detected automatically.
- **No response from meter:** animated "Waiting for meter..." indicator
  during initial timeouts, then step-by-step instructions to enable USB mode
  (insert module, turn on, long-press USB/Hz until S icon appears)

Auto-reconnection retries every 2 seconds after a disconnect.

## Accessibility

### Visual

- Theme-aware colors with WCAG 2.1 AA contrast ratios (≥4.5:1 text, ≥3:1 graphical elements). Minimum 11 pt font; status flags use bold text in addition to color so they don't rely on color alone.
- Every button, link, toggle, and setting has a hover tooltip explaining what it does — hover any control to learn it without leaving the GUI.

### Keyboard

- Every feature is reachable from the keyboard. See [Keyboard Shortcuts](#keyboard-shortcuts) for the full list.
- Tab and Shift+Tab cycle through every control in visual order. The currently focused control shows a visible outline, including on the color-picker swatches, the graph minimap, and the recording-panel resize divider.
- The `?` keyboard-shortcut help overlay and the "What's New" window both trap focus inside while open. Press Esc or Ctrl+W to close, and focus returns to the button you used to open them.

### Screen reader

Screen reader support is built on [AccessKit](https://accesskit.dev/) and exposed through each platform's native accessibility API: AT-SPI on Linux (used by [Orca](https://orca.gnome.org/)), UI Automation on Windows, and NSAccessibility on macOS. The labels described below are wired up in the code but have **not yet been walked end-to-end with a real screen reader** — verification is [tracked as an open item](verification-backlog.md). Reports of what does and doesn't come through as expected are welcome.

- Every button, toggle, text field, and custom widget has a spoken name. Icon-only buttons (Settings, Help, Min/Max exit, big-meter toggle), color swatches in the settings panel, the graph minimap, and the recording resize bar all announce what they do instead of their literal glyph or color.
- Toggle buttons like HOLD, REL, RANGE, AUTO, MIN/MAX, PEAK, and the graph's LIVE button announce whether they are currently on or off — you don't have to rely on the color change.
- The main reading updates as a polite live region: new values are spoken at natural pauses, not interrupting you.
- The graph announces a one-line summary of what it's showing: time window, Y-axis range, number of samples, whether it's following live, and the most recent reading. The summary updates whenever any of those change.
- Orca's landmark navigation jumps between the top bar (Toolbar), the main content area (Main), and the connection status region (Status).

### Known limitations

- There is no per-sample keyboard navigation inside the graph — you can't step from one data point to the next and hear each value spoken. Use the Statistics panel for min/max/average and the Recording panel's sample list for point-level readings; the sample list is a scrollable text log that screen readers read row by row.
- Graph measurement cursors (A/B) can only be placed by clicking on the plot.
- In the **Customize colors** popup, the RGBA drag-value fields use egui's default drag-value behaviour: press Enter to enter edit mode, then Up/Down to change the value. The 2D saturation/value and 1D hue gradient sliders accept arrow keys when Tab-focused (2 % step, horizontal for saturation/hue, vertical for value), but mouse drag remains the fastest way to pick a color.
- The graph plot's X and Y axes are separate Tab stops that don't show a focus ring — egui_plot allocates focusable drag responses for each axis that can't be customised from outside the crate.

## See Also

- [CLI reference](cli-reference.md) — command-line tool documentation
- [Setup guide](setup.md) — build prerequisites, udev rules, first-run
  instructions
- [Supported devices](supported-devices.md) — full compatibility list and device families
