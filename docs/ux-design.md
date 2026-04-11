# UX Design

## Design Principles

1. **Clean and modern** — minimal chrome, subtle separators, generous but efficient use of space
2. **High information density** — no wasted space; every pixel earns its place
3. **Readable** — large primary reading, clear hierarchy, good contrast in both themes
4. **Configurable** — users choose which panels are visible via a settings gear menu
5. **Responsive** — adapts between wide (side-by-side) and narrow (stacked) layouts

## CLI Interface

### Subcommands

- `ut61eplus list` — enumerate connected devices
- `ut61eplus info` — connect and print device name (queried from meter)
- `ut61eplus read` — continuous measurement reading with `--format` (text/csv/json), `--output`, `--interval-ms`
- `ut61eplus command` — send button presses: hold, min-max, exit-min-max, rel, range, auto, select, select2, light, peak-min-max, exit-peak
- `ut61eplus debug` — raw hex dump mode for protocol development
- `ut61eplus capture` — guided protocol capture wizard for bug reports. YAML output with raw bytes, structured flags, user screen confirmations. Supports `--steps` filter, auto-resume, and freeform captures.

### GUI Command-Line Options

The GUI accepts `--device`, `--theme`, and `--mock-mode` flags (via `clap`,
consistent with the CLI). These override saved settings for the current
session only. The settings panel shows which values are overridden (e.g.,
"UT181A (--device)"). Clicking a different value in the panel clears the
override and persists the user's choice. See `docs/gui-reference.md` for
the full options table.

## GUI Layout

### Theme

- Supports light and dark mode, toggled via settings
- Default: dark
- Connected status: green indicator dot + device name (e.g., "UT61E+")
- Disconnected/error: grey indicator dot
- Reconnecting: orange indicator dot

### Color Palette

- Three curated presets: Default (warm), High Contrast (bold), Colorblind-safe (blue/orange/purple)
- All 18 base colors customizable per-theme via UI color pickers or JSON overrides
- Colors are split: UI chrome (3), graph (9), status indicators (5), minimap (1)
- Derived colors auto-track their base (cursor dim/delta, minimap line, live indicator, recording warning, button hover/active)
- UI chrome colors (background, text, button) modify egui Visuals — plot grid and axis labels follow automatically
- Preset selection and per-color overrides persist to `settings.json`

### Top Bar

Compact toolbar row: app title, Connect/Disconnect button, Pause/Resume button (freezes capture without disconnecting — pauses >gap threshold show gap markers), Clear button (resets graph/stats), connection status with device name and colored dot, settings gear icon (right-aligned).

### Settings Panel

Toggled by the gear icon. Contains:

- **Theme:** Dark / Light
- **Colors:** Default / High Contrast / Colorblind preset selector. Collapsible "Customize colors" section with per-color edit buttons.
- **Panels:** Show/hide Graph, Statistics, Recording, Specifications
- **Auto-connect on start:** default on
- **Show device name on connect (beeps):** default on — queries device name via protocol, which causes the meter to beep
- **Sample interval:** 0ms (fastest, ~10 Hz), 100ms, 200ms, 300ms, 500ms, 1000ms, 2000ms. Requires reconnect to take effect.
- **Zoom:** UI scale selector (30%-300%, Firefox-style non-linear levels) + keyboard shortcuts (Ctrl+/-, Ctrl+0 to reset). 100% = OS default scale. Persists across sessions.

Settings persist to `~/.config/ut61eplus/settings.json`.

### Responsive Layout

Threshold at ~900px available width:

**Wide (≥ 900px):** Two-column layout with resizable panels.
- Left column (resizable, 180-400px): reading display, remote control buttons, mode/range/flags, specifications panel, statistics panel
- Right column: graph toolbar + main graph + minimap, drag separator, recording panel
- Graph/recording split resizable via drag handle

**Narrow (< 900px):** Single-column stack.
- Reading (compact single line for mode/flags)
- Specifications (compact inline)
- Statistics (compact line + visible window stats)
- Graph (toolbar + main + minimap)
- Recording (resizable via drag handle)

**Big meter mode (graph + recording both hidden):** Single centered display.
- Reading, buttons, specs (inline), and stats scale to fill available space
- Font size computed from both available width and height using cached measured text ratios
- Buttons and stats scale proportionally with the reading
- Quick toggle via **⊞** button (near remote controls) or **Ctrl+B** — temporarily hides all panels without changing saved settings
- Useful as a large bench-meter display or for presentations

### Reading Display

- Primary value uses meter's raw 7-char display string in monospace font for stable formatting
- Unit adjacent in monospace ("V")
- Mode, range label, and active flags below
- Flags shown as subtle colored badges: AUTO, HOLD, REL, MIN, MAX
- Low battery warning shown as orange "LOW BAT" badge

### Remote Control Buttons

Row of buttons below the reading (only shown when connected and receiving data):
- **HOLD, REL, RANGE, AUTO, MIN/MAX, PEAK** — highlight blue when the corresponding protocol flag is active
- **SELECT** — cycles sub-modes (no toggle state, mode change visible in reading)
- **LIGHT** — toggles backlight (no protocol feedback for state)

### Specifications Panel

Shows per-range electrical specifications from the device manual:
- Resolution, accuracy (with multiple frequency bands for AC), input impedance, notes
- "Manual" hyperlink to manufacturer's product page when `manual_url` is configured
- Adapts to each layout: full panel (wide), inline summary (big meter), compact line (narrow)
- Data cached per mode+range — re-looked up only on mode/range changes, zero per-frame allocations
- Coverage: UT61E+, UT61B+, UT61D+, UT161 family, Mock. Other devices show manual link only.

### Connection Help

Shown when connection fails:
- **USB adapter not found:** udev rule instructions, prompt to click Connect
- **No response from meter:** "Waiting for meter..." animation during timeouts, then step-by-step USB enable instructions (insert module, turn on, long press USB/Hz button)

### Graph Panel

Three components stacked vertically:

**Toolbar:**
- Time window presets: 5s, 10s, 30s, 1m, 5m, 10m
- LIVE toggle button (green when active)
- Y:Auto / Y:Fixed toggle — in fixed mode, shows min/max text input fields. Switching to fixed snapshots current auto range unless user previously edited values.
- **Mean** toggle — dashed horizontal line at visible window average, labeled with value
- **Min/Max** toggle — sliding window envelope (configurable width in seconds), dashed boundary lines showing value range
- **Ref** toggle — one or more horizontal reference lines at user-specified values (comma/space/semicolon separated), each labeled. When active, optional **Triggers** toggle shows diamond markers at threshold crossings.
- **Cursors** toggle — click graph to place cursor A then B (snaps to nearest data point). Draws vertical + horizontal lines at each cursor. Labels show time and value. Toolbar displays ΔT and ΔV between cursors.

**Main graph:**
- `egui_plot` time series with auto-scaling Y axis (10% padding)
- Y axis tick labels include unit (e.g. "1.0 mV" not "1.0"), X axis labels include unit ("10 s", "1 m")
- Crosshair tooltip shows time and value with units
- In LIVE mode: auto-scrolls to latest data, drag/zoom disabled
- In browse mode (click LIVE to toggle, or click minimap): drag to pan X, scroll wheel to zoom X (centered on cursor). Y auto-scales to visible data.
- Scroll while in LIVE mode exits to browse mode
- Double-click to return to LIVE mode
- Disconnect gaps shown as dashed red vertical line pairs
- Consistent line color across reconnects
- Timeline is continuous across disconnects (data not cleared on reconnect)

**Minimap:**
- Custom-painted thin strip showing full capture history
- Viewport indicator as [ ] bracket markers (thick blue lines)
- Click/drag to navigate: moves main graph viewport to clicked time
- Clicking near the latest data re-enables LIVE mode
- Time axis labels with smart interval selection

**History:** ~10,000 points (VecDeque, oldest dropped). Mode changes clear the graph (incompatible units).

### Statistics Panel

- Min, Max, Avg values in monospace with right-aligned fixed-width formatting
- Sample count
- Reset button clears stats
- Stats persist across reconnects (use Clear button for full reset)
- In wide layout: also shows visible window stats (min/max/avg for current graph interval)

### Recording Panel

- Record/Stop toggle button
- Export CSV button (opens file dialog on separate thread — no UI freeze)
- Shows sample count and duration while recording
- Records to in-memory buffer, exported on demand
- Scrollable sample log showing recent samples (timestamp, value, unit, flags) in monospace. Auto-scrolls to bottom, caps at last 500 entries.

### Accessibility

- All colors are theme-aware — darker variants on light backgrounds, brighter on dark
- WCAG 2.1 AA contrast ratios verified: ≥4.5:1 for text, ≥3:1 for graphical elements
- Minimum font size 11pt throughout (WCAG recommends ≥12px)
- Flag badges use bold text in addition to color for non-color distinction
- Status dot uses text label alongside color indicator
- Graph overlays use distinct line styles (solid, dashed-dense, dashed-loose) in addition to color
- `NO_COLOR=1` env var disables CLI color output
