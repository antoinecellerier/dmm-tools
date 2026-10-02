# UX Design

The principles behind the CLI and the GUI, and the reasons for the choices a
user meets. What each command and control does is in the
[CLI reference](cli-reference.md) and the [GUI reference](gui-reference.md).
Code paths below are under `crates/dmm-gui/src/`.

## Design Principles

1. **Clean and modern** — minimal chrome, subtle separators, generous but efficient use of space
2. **High information density** — no wasted space; every pixel earns its place
3. **Readable** — large primary reading, clear hierarchy, good contrast in both themes
4. **Configurable** — users choose which panels are visible via a settings gear menu
5. **Responsive** — adapts between wide (side-by-side) and narrow (stacked) layouts

## Command line

**Command-line values last one session.** GUI options (parsed with `clap`, as
in the CLI) never reach `settings.json`: `--device auto` detects without
saving, where Auto-detect in the panel saves what it finds.

**Overrides are marked.** A session-only value has to be told from a saved
one.

## GUI layout

**One width decides the layout.** The reading column sits beside the graph in
a wide window and stacks above it in a narrow one (Responsive). The
threshold is `WIDE_LAYOUT_MIN_WIDTH` in `app/meter_fit.rs`.

**Columns scroll like a page.** A short window scrolls each column rather than
cropping it; the top bar stays pinned; the wheel scrolls and Ctrl+wheel zooms
the graph. Users know these gestures from other apps, unlike nested scrollers.

**News floats as a toast.** A transient message sits over the top-right
corner, so every layout shows it, minimal mode included, and a narrow top bar
neither clips it nor grows to hold it.

**The window's position is left to the OS.** Wayland ignores it, and a monitor
unplugged since would leave the window off-screen. Fullscreen isn't restored
either: it is a mode the user steps into.

**Big-meter sizes are not kept.** The mode itself isn't, and a small readout
window would cramp the next launch's full layout (`App::track_layout` in
`app/layout.rs`).

**Zoom steps like a browser's.** Ctrl+Plus, Ctrl+Minus and Ctrl+0 walk
non-linear levels (`ZOOM_LEVELS` in `app/appearance.rs`); 100% is the OS scale.

**Big meter and minimal modes.** With the graph and recording hidden the
reading fills the window, for a bench display or a presentation; minimal mode
drops the top bar and buttons too. **⊞** and Ctrl+B leave saved panels alone,
so closing the app keeps the configured layout.

**Nothing below a window-sized reading.** In big meter and minimal mode a
connection problem replaces the reading's placeholder with its title, and the
steps go in its hover text.

## Reading display

**The meter's own digits.** The reading is the meter's display string in
monospace at a fixed width, so digits and a minus sign coming and going never
shift it (`format_display_raw` in `display/text.rs`).

**Only a working control looks like one.** The mode and range labels become
dropdowns only where the meter can be switched and offers a choice; elsewhere
they stay plain labels.

**A key list is a list of presses.** A meter that lists function keys (ZOTEK)
gets a **Meter keys** dropdown captioned as presses. Every pick presses, the
marked key too, since a key can cycle within its function.

**A button promises a press.** The remote buttons mirror the meter's front
panel and send a raw press, so their labels and tooltips promise no more;
naming a destination is the readout dropdowns' job.

**SCALE comes last.** Its badge follows the meter's own: it is the app's
state, not the meter's, and among the reported flags it would be misattributed.

## Scale

**Scale is set apart from the meter's buttons.** A vertical rule separates it:
they drive the meter, and Scale changes nothing on it. Without the boundary it
would suggest the meter knows the factor; a line break serves when it wraps.

**Scale is always on screen.** While disconnected it keeps a line of its own,
so an active scale can always be turned off.

**Scale commits on Apply.** Apply or Enter commits, never a keystroke: a
half-typed number would clear the graph and statistics on every character.

**Applying a scale resets the graph.** The graph, statistics and integral no
longer describe the same quantity, so they restart. The recording carries on,
as it does on **Clear**.

## Graph

**The toolbar reads view, series, overlays.** The boxed **Plot:** and
**Show:** groups get a row between the time window and the analysis toggles:
inline, nothing told the controls apart (`show_toolbar` in `graph/toolbar.rs`).

**Each unit gets its own axis.** A sub-value in another unit is drawn
against a Y axis of its own on the right, up to four units; a fifth's chip
waits for one to be hidden. The right axes label the plotted unit's
gridlines in round steps of their own, since egui_plot gives every axis one
transform and one grid (`graph/axes.rs`); a 2.5 step labels every other
gridline, which gives a trace the height between a 2 and a 5 step. Aligned
by construction, several share one column, each gridline's values stacked:
one label's width rather than one per unit, so they fit far narrower
windows before the column is dropped.
Where lines of two units cross means nothing, so the analysis tools and the
minimap stay on the plotted series, which **Plot:** picks, and with a right
axis the hover lists every series at its time rather than one height. Stacked
lanes, one per unit, were the alternative: no false crossings, but every lane
shorter ([issue #5](https://github.com/antoinecellerier/dmm-tools/issues/5)).
A change of mode or unit clears the graph: the old and new scales are
incompatible.

**The plot key is a key.** It toggles nothing: egui_plot's legend loses its
show/hide state while the view is pinned every frame (`paint_plot_key` in
`graph/render.rs`), so the **Show:** chips are the control.

**Session choices are never saved, hidden traces are.** A scale or plotted
series restored silently at the next launch would corrupt readings or plot a
sub-value the user doesn't suspect; a plotted series lasts while the meter
sends it. Hiding a trace loses no reading and its chip still lists it, unlit,
so a **Show:** click is remembered by label (`Settings::hidden_series`) — but
not the main reading's own chip, which would hide it on the next meter.

## Recording and export

**Import… comes first.** Placed before Record, it leaves Record, Export…,
Discard and the sample count together as the recording's controls. An import
replaces the session, so it asks through Record's discard prompt.

**Export… saves the graph when nothing is recorded.** The graph and a
recording share one store, so a reading both hold is paid for once (`Recording`
in `recording.rs`), and the samples on screen can be saved without Record.

**The format is picked before the dialog.** The dialog returns a path but not
the file type picked, and GTK keeps the name's extension when the filter
changes (`ExportFormat` in `app/export.rs`).

## Accessibility

**Never colour alone.** Flag badges are bold, the status dot has its text, an
imported session's ring shape says no meter is attached, a toast carries a
glyph, overlay traces differ by dash pattern, and with a right axis the plot
key and every tick name their unit. **Graph lines: Solid** gives up the dash
patterns for those who don't need them, on data lines only: the mean,
reference, envelope, cursor, marker and data-loss lines keep theirs, which
are what tell them from data and from each other.

**11 pt floor.** egui's small text style ships at 9 pt, so it is raised to
11 pt once at startup rather than avoided per call site (`SMALL_TEXT_SIZE` in
`app/appearance.rs`).

## Colour and contrast

**WCAG 2.1 AA in every preset.** Text clears 4.5:1 and graphical elements
3:1, in both themes: every colour has a dark and a light variant, and tests in
`theme.rs` check each preset.

**Secondary text has its own colour.** egui's 60% dimming falls under 4.5:1,
so secondary text takes a per-preset colour clearing it on the panel, the frame
fill and the text-edit background. A user's own pick is theirs to keep above it.

**Dark text is lifted.** Dark primary text is gray(180), not egui's gray(140):
beside 140 no dimmer tone clears 4.5:1, and 180 keeps two distinct tiers
(ratios beside `PRESET_DEFAULT` in `theme.rs`).

**The stock presets stay egui's own.** Text and Accent reach egui's own
painting only once customized, and only High Contrast pins Border (3:1), so the
presets follow egui's defaults as egui changes them.
