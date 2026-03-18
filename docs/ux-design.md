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

## GUI Layout

### Theme

- Supports light and dark mode, toggled via settings
- Default: dark
- Connected status: green indicator dot + device name (e.g., "UT61E+")
- Disconnected/error: grey indicator dot
- Reconnecting: orange indicator dot

### Top Bar

Compact toolbar row: app title, Connect/Disconnect button, Clear button (resets graph/stats), connection status with device name, settings gear icon (right-aligned).

### Settings Panel

Toggled by the gear icon. Contains:

- **Theme:** Dark / Light
- **Panels:** Show/hide Graph, Statistics, Recording
- **Auto-connect on start:** default on
- **Show device name on connect (beeps):** default on — queries device name via protocol, which causes the meter to beep
- **Sample interval:** 0ms (fastest, ~10 Hz), 100ms, 200ms, 300ms, 500ms, 1000ms, 2000ms. Requires reconnect to take effect.
- **Zoom:** UI scale selector (30%-300%, Firefox-style non-linear levels) + keyboard shortcuts (Ctrl+/-, Ctrl+0 to reset). 100% = OS default scale. Persists across sessions.

Settings persist to `~/.config/ut61eplus/settings.json`.

### Responsive Layout

Threshold at ~900px available width:

**Wide (≥ 900px):** Two-column layout.
- Left column (fixed ~220px): reading display, remote control buttons, mode/range/flags, statistics panel
- Right column (remaining width): graph toolbar + main graph + minimap, recording bar

**Narrow (< 900px):** Single-column stack.
- Reading (compact single line for mode/flags)
- Statistics (compact line)
- Graph (toolbar + main + minimap)
- Recording (single-line toolbar)

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

**Main graph:**
- `egui_plot` time series with auto-scaling Y axis (10% padding)
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
