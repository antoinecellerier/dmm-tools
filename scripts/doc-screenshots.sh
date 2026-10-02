#!/usr/bin/env bash
# Regenerate the dmm-gui pictures in assets/ from the bench recordings.
#
# Every scene drives dmm-gui through the verify-gui skill's gui-display.sh, on
# a private Xvfb display with a private XDG_CONFIG_HOME — the user's desktop,
# browser and settings.json are never touched. The meter readings come from
# assets/replays/, so a picture shows a real session rather than the mock.
#
# Usage:
#   scripts/doc-screenshots.sh list          # the asset each scene writes
#   scripts/doc-screenshots.sh all           # every scene
#   scripts/doc-screenshots.sh gui-settings.png [...]   # the scenes that
#                                                       # write these pictures
#
# Session time is frozen at each scene's preseed instant (see `launch`), so a
# rerun stages exactly the same frame. Each capture prints the pixel difference
# against the committed file and leaves that file alone when there is none,
# since a rewritten PNG carries a new timestamp. Xvfb is not guaranteed to
# render identically across driver versions, so a small delta is still
# possible — look at the PNGs before committing them.
set -euo pipefail

# Byte semantics for the [0-9] classes below, and a stable number format in
# whatever the GUI prints into a picture.
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUI="$ROOT/.claude/skills/verify-gui/scripts/gui-display.sh"
ASSETS="$ROOT/assets"
# The bench recordings, shared with the doc snippets, which name theirs in
# their `via=` marker:
#   dcma-boot-refresh     an e-paper thermometer booting and refreshing, on a
#                         fixed 220 mA range
#   ohm                   a flat 4.649 kΩ on AUTO
#   dcmv-hold-rel         DC mV with HOLD, then REL
#   ut181a-vac-hz         one frame from a UT181A golden fixture, repeated
#   acdcv-cell            a UT61E+ in AC+DC V across a 1.6 V cell, with a lead
#                         lifted twice
#   dcv-steps             a bench supply stepped and ramped 2.9–9.3 V; unused
#   dcma-boot-refresh-autorange
#                         the thermometer cycle on AUTO, with 22 ↔ 220 mA hops
#                         and an OL blip at each boot and refresh; unused
#   ut181a-temp-t1-t2     a UT181A frame with two temperature sub-values;
#                         unused
REPLAYS="$ASSETS/replays"

# A state dir of our own so a verify-gui session in another terminal keeps its
# display and its settings.json.
export VERIFY_GUI_STATE="${XDG_RUNTIME_DIR:?doc-screenshots needs XDG_RUNTIME_DIR}/doc-screenshots"
# 960x640 logical, the app's default window size, at 2x: the window fills the
# root exactly, so a full-window shot is 1920x1280 with crisp text.
ROOT_GEOMETRY=1920x1280x24
# The meter-mode scenes shape the window themselves and need one wider than
# the default root. Xvfb takes its size at start, so those scenes stop the
# display first.
METER_GEOMETRY=2560x1440x24
export VERIFY_GUI_GEOMETRY="$ROOT_GEOMETRY"
export WINIT_X11_SCALE_FACTOR=2
# Session time per second of real time once the preseed burst has been handed
# out: slow enough that a scene's keys and clicks all land on the same frame.
CLOCK_SCALE=0.001
CONFIG_DIR="$VERIFY_GUI_STATE/config/dmm-tools"

# Pixel geometry, measured once from a full-size shot of the wide layout at
# 1920x1280. Every coordinate is in physical pixels from the window's corner,
# which is what gui-display.sh's click and crop take.
# Left column, from under the top bar (its rule is at y 44) to the rule above
# "Specifications" (y 330): the reading, its mode/range line and flags, the
# remote-control buttons, Scale. The column ends at the sidebar rule, x 478.
READING_CROP="478x282+0+46"
# Graph column: toolbar, main plot and minimap, between the sidebar rule and
# the window's right edge, stopping above the graph/recording rule at y 1033.
GRAPH_CROP="1428x983+492+48"
# One palette tile: the toolbar's chips, whole — the last of them, Reset Zoom,
# ends at x 1198 — and the plot as far as the trace's spike, without the
# minimap. Narrow enough that the four still read side by side in a table.
THEME_CROP="720x760+492+48"
# The top bar and the whole settings panel — its closing rule is at y 610 —
# over a band of the reading and graph below, enough to place the panel in the
# window without carrying its full height.
SETTINGS_CROP="1920x960+0+0"
# The colour rows of the settings panel plus a swatch picker opened at the
# panel's right edge, where it covers no other setting.
COLOR_CROP="1920x638+0+58"
# The left column down to the rule under the connection help (y 889): the
# no-reading dashes, then the help's title and a section per link, which is
# the whole subject — the rest of the window is an empty graph.
HELP_CROP="478x843+0+46"
GEAR_X=1884; GEAR_Y=22              # the settings gear, right end of the top bar
CUSTOMIZE_X=150; CUSTOMIZE_Y=166    # the "Customize colors" collapsing header
# The Graph row's last swatch, Crosshair: its picker opens against the right
# edge instead of over the rows beside it.
CROSSHAIR_SWATCH_X=1578; CROSSHAIR_SWATCH_Y=281
# Empty left-column space, below the statistics and below the narrow layout's
# recording hint: clicking here moves the pointer off the plot without
# activating anything, so no crosshair tooltip lands in the picture.
PARK_X=240; PARK_Y=1240
# Big meter takes one window, sized so nothing wraps out of it. Minimal mode
# has two shapes, one picture each: the mode and range controls beside the
# reading in a wide, short window, and under it in a narrow one.
BIG_METER_W=900; BIG_METER_H=640
MINIMAL_WIDE_W=1200; MINIMAL_WIDE_H=200
MINIMAL_NARROW_W=420; MINIMAL_NARROW_H=240
# Each graph scene's state, as the `# view:` line `staged` appends: times in
# seconds from the recording's first frame, which is where the app puts them
# back. The markers' offsets are in the scenes themselves.
#
# The hero: the 30 s window over the boot, the mean, and cursors on the idle
# floor either side of it so both level lines lie on 0 mA rather than across
# the plot — A on the last reading before the rise (2.376 s), B on the first
# after the fall (29.995 s). Its markers name the boot's stages: its first
# reading above 0 mA, the dip after the plateau as the radio goes off, and the
# fall to the draw of the e-Paper display refreshing.
HERO_VIEW='# view: {"window":30.0,"start":1.62,"mean":true,"cursors":{"a":2.376,"b":29.995}}'
# The hero's session and window in a window too narrow for two columns, the
# mean without the cursors.
NARROW_VIEW='# view: {"window":30.0,"start":1.06,"mean":true}'
# The last refresh cycle off the live view: the mean, and cursors on the
# refresh's two 1 mA crossings — A on the rising edge's first reading above
# 1 mA (149.182 s), B on an idle reading after the fall (170.071 s), far
# enough from the plot's right edge for its readout to fit.
OVERLAYS_VIEW='# view: {"window":30.0,"mean":true,"cursors":{"a":149.182,"b":170.071}}'
# The boot picked out of the session's history: the 1m window at its start,
# a reference line at 1 mA — whose trigger markers are on by default — and a
# whole-window 60 s envelope showing the boot's peak and floor, where the 1 s
# default just shadows the trace.
TRIGGERS_VIEW='# view: {"window":60.0,"start":0.0,"envelope":60.0,"references":[1.0],"references_shown":true}'
# One per colour preset, live: the palette on a trace, a mean line, a
# reference line and its trigger markers.
THEMES_VIEW='# view: {"window":30.0,"mean":true,"references":[1.0],"references_shown":true}'

# asset written -> the function that stages it.
SCENES=(
	"gui-wide-layout.png scene_wide"
	"gui-narrow-layout.png scene_narrow"
	"gui-reading-controls.png scene_reading_controls"
	"gui-graph-overlays.png scene_overlays"
	"gui-graph-triggers.png scene_overlays"
	"gui-graph-series.png scene_series"
	"gui-big-meter.png scene_big_meter"
	"gui-minimal-meter-wide.png scene_minimal_meter"
	"gui-minimal-meter-narrow.png scene_minimal_meter"
	"gui-settings.png scene_settings"
	"gui-theme-dark.png scene_themes"
	"gui-theme-light.png scene_themes"
	"gui-theme-high-contrast.png scene_themes"
	"gui-theme-colorblind.png scene_themes"
	"gui-color-customization.png scene_color_customization"
	"gui-connection-help.png scene_connection_help"
)

# A frame at the pictures' size under software GL, with room to spare.
FRAME_GAP=0.1
# `dmm-cli list`'s answer, taken once per run by no_meter_or_skip.
METER_LISTING=""

die() { echo "doc-screenshots: $*" >&2; exit 1; }

require() {
	local t missing=()
	for t in "$@"; do command -v "$t" >/dev/null 2>&1 || missing+=("$t"); done
	((${#missing[@]} == 0)) ||
		die "missing tool(s): ${missing[*]} — ask the user to run: sudo apt install xvfb xdotool imagemagick python3-pil"
}

# The settings every scene starts from, overridden per scene with a JSON object.
# last_seen_version is the workspace version so the What's New popup, which
# opens by itself after an upgrade, stays closed.
write_settings() {
	local overrides="${1:-}" version
	[ -n "$overrides" ] || overrides='{}'
	version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$ROOT/Cargo.toml" | head -1)"
	[ -n "$version" ] || die "no workspace version in Cargo.toml"
	mkdir -p "$CONFIG_DIR"
	python3 - "$CONFIG_DIR/settings.json" "$version" "$overrides" <<-'PY'
		import json, sys
		path, version, overrides = sys.argv[1:4]
		settings = {
		    "device_family": "ut61eplus",
		    "theme": "Dark",
		    "color_preset": "Default",
		    "show_graph": True,
		    "show_stats": True,
		    "show_recording": True,
		    "show_specs": True,
		    "auto_connect": True,
		    "query_device_name": True,
		    "sample_interval_ms": 0,
		    "zoom_pct": 100,
		    "last_seen_version": version,
		}
		settings.update(json.loads(overrides))
		with open(path, "w") as f:
		    json.dump(settings, f, indent=2)
	PY
}

# launch <replay basename> <preseed secs> [extra dmm-gui args...]
#
# gui-display.sh prepends --device mock when neither --device nor --replay is
# given; a recording names its own meter, so it launches as the session device.
# The path is absolute because the app is launched from the caller's directory.
#
# The preseed is handed out in one burst at connect; the scale then runs
# session time at a thousandth of real time, so the shot shows the preseed
# instant however long the keys and clicks before it take. Without it a scene
# drifts by the seconds it spends pressing keys, which moves the trace under
# the click coordinates and can run past the event the picture is of.
launch() {
	local replay="$1" preseed="$2"
	shift 2
	[ -f "$REPLAYS/$replay.replay" ] || die "no recording $REPLAYS/$replay.replay"
	launch_file "$REPLAYS/$replay.replay" "$preseed" "$@"
}

# launch_file <replay path> <preseed secs> [extra dmm-gui args...] — as
# launch, for a recording `staged` wrote.
launch_file() {
	local path="$1" preseed="$2"
	shift 2
	"$GUI" run --replay "$path" \
		--mock-clock-preseed "$preseed" --mock-clock-scale "$CLOCK_SCALE" "$@" >/dev/null
}

# staged <replay basename> <line>... — a copy of the recording with `# view:`
# and `# marker:` lines appended, printing its path. The app puts the view
# back once the preseed burst has played the moments it shows, and each
# marker on the first reading at or after its offset, so a scene loads its
# state instead of clicking it in. Times are seconds (view) and milliseconds
# (markers) from the recording's first frame.
staged() {
	local replay="$1" out
	shift
	[ -f "$REPLAYS/$replay.replay" ] || die "no recording $REPLAYS/$replay.replay"
	out="$TMP/$replay-$RANDOM.replay"
	cp "$REPLAYS/$replay.replay" "$out"
	printf '%s\n' "$@" >>"$out"
	echo "$out"
}

# The app on Auto-detect with nothing to detect, for the pictures that must
# show the settings and the help a user meets before any meter answers. No
# --device, so the Device row reads as a user sees it rather than marked as a
# command-line choice: the grant stops gui-display passing its mock and the
# app opens what the scene's settings.json names; the guard every caller runs
# first has already made sure there is no meter to open.
launch_without_meter() {
	VERIFY_GUI_ALLOW_HW=1 "$GUI" run >/dev/null
}

# Three pictures are of the app before any meter answers: the two settings
# ones so the panel carries its shipped defaults, Auto-connect included, and
# the help one because it is the failed connection itself. A cable on the
# bench would make all three a session with a real meter, so they skip
# instead. Nothing is opened either way.
#
# A cable it finds is listed on stdout; "No devices found." and the advice under
# it go to stderr, so both streams are read and the line matched whole. "No
# devices heard in range." means a paired Bluetooth adapter the scan missed:
# the app would still try it, so it must be switched off for these scenes.
no_meter_or_skip() {
	local listing
	# Once per run: the answer cannot change between scenes, and a list can
	# spend seconds on a Bluetooth scan.
	[ -n "$METER_LISTING" ] || METER_LISTING="$(cd "$ROOT" && cargo run -q -p dmm-cli -- list 2>&1 || true)"
	listing="$METER_LISTING"
	if printf '%s\n' "$listing" | grep -qx 'No devices heard in range.'; then
		echo "$1: a paired Bluetooth adapter is listed — make sure it is switched off"
	elif ! printf '%s\n' "$listing" | grep -qx 'No devices found.'; then
		echo "$1: skipped — unplug the USB cable and run this scene again"
		echo "  dmm-cli list said: $(printf '%s' "$listing" | head -1)"
		return 1
	fi
}

# Two keystrokes sent back to back can land in the same egui frame; a Return
# that closes a text field also needs a frame before the next key is read.
# FRAME_GAP is a frame with room to spare at this size under software GL.
key() {
	"$GUI" key "$1" >/dev/null
	sleep "$FRAME_GAP"
}

click() { "$GUI" click "$1" "$2" >/dev/null; sleep "$FRAME_GAP"; }

park() { click "$PARK_X" "$PARK_Y"; }

# shot <out.png> [crop] — once the screen has stopped changing.
shot() {
	local out="$1" crop="${2:-}"
	"$GUI" settle >/dev/null
	if [ -n "$crop" ]; then
		"$GUI" shot "$TMP/full.png" >/dev/null
		convert "$TMP/full.png" -crop "$crop" +repage "$out"
	else
		"$GUI" shot "$out" >/dev/null
	fi
}

# capture <asset> [crop] — shoot, report the delta, move into assets/ unless no
# pixel changed: a PNG carries the time it was written, so an identical picture
# would still show up as modified in git.
capture() {
	local asset="$1" crop="${2:-}"
	shot "$TMP/$asset" "$crop"
	if report "$asset" "$TMP/$asset"; then
		mv "$TMP/$asset" "$ASSETS/$asset"
	fi
}

# fit <width> <height> — resize and echo the geometry the window settled on.
#
# There is no window manager on the private display, so the app's minimum size
# is not enforced; it re-grows a window below its own minimum and gui-display
# prints what it took. The crop follows that, so the picture ends at the window
# edge and keeps the corner button that leaves the mode.
fit() {
	local geometry
	geometry="$("$GUI" resize "$1" "$2" | sed -n 's/^window [0-9]* is \([0-9]*x[0-9]*\).*/\1/p')"
	[ -n "$geometry" ] || die "resize did not report a window size"
	echo "$geometry"
}

# How far the new picture is from the committed one, for the human who reviews
# it: a mis-click or a popup left open shows up as a delta in the millions.
# Fails when not one pixel differs.
report() {
	local asset="$1" new="$2" old="$ASSETS/$1" size delta
	size="$(identify -format '%wx%h' "$new")"
	if [ ! -f "$old" ]; then
		echo "$asset: new, $size"
		return
	fi
	if [ "$(identify -format '%wx%h' "$old")" != "$size" ]; then
		echo "$asset: $(identify -format '%wx%h' "$old") -> $size, resized"
		return
	fi
	delta="$(compare -metric AE "$old" "$new" null: 2>&1 || true)"
	delta="${delta%%[^0-9]*}"
	if [ "$delta" = 0 ]; then
		echo "$asset: unchanged, $size"
		return 1
	fi
	echo "$asset: $delta px differ, $size"
}

## Scenes ####################################################################

# The hero: wide layout over the thermometer's boot, looked back on from its
# first refresh cycle. The preseed stops the session inside that cycle, so the
# reading shows its draw (4.69 mA) and the minimap holds the whole session.
# HERO_VIEW puts the 30s window back on the boot with the mean and the cursors
# that span it; three markers with notes name the boot's stages, on the graph
# and in the log.
scene_wide() {
	write_settings
	launch_file "$(staged dcma-boot-refresh "$HERO_VIEW" \
		'# marker: 2475 1 boot' \
		'# marker: 8018 2 wifi off' \
		'# marker: 9998 3 e-Paper display refreshing')" 160.5
	park
	capture gui-wide-layout.png
}

# The hero's session and window in a window too narrow for two columns. The
# mean stays but the cursors do not: the plot is too short for their readouts
# to find a corner off the trace.
scene_narrow() {
	write_settings
	launch_file "$(staged dcma-boot-refresh "$NARROW_VIEW")" 160.5
	"$GUI" resize 1000 1280 >/dev/null
	park
	capture gui-narrow-layout.png "1000x1280+0+0"
}

# The reading with a flag lit and the remote-control buttons under it, from the
# DC mV session that used HOLD and REL. The preseed is where the session
# starts and the shot lands about 3.5 s later, so 10 reads inside the
# 10.4–16.5 s hold rather than past its end.
scene_reading_controls() {
	write_settings
	launch dcmv-hold-rel 14
	park
	capture gui-reading-controls.png "$READING_CROP"
}

# Two graph pictures of one session: the overlays that read the last refresh
# cycle off the live view, then the ones that mark the boot sequence, picked
# out of the session's history. They are separate because triggers and cursors
# land on the same two crossings, and because only one of them can be live.
# Each loads its state (OVERLAYS_VIEW, TRIGGERS_VIEW) into a launch of its own.
scene_overlays() {
	write_settings
	launch_file "$(staged dcma-boot-refresh "$OVERLAYS_VIEW")" 178
	park
	capture gui-graph-overlays.png "$GRAPH_CROP"
	"$GUI" stop >/dev/null
	launch_file "$(staged dcma-boot-refresh "$TRIGGERS_VIEW")" 178
	park
	capture gui-graph-triggers.png "$GRAPH_CROP"
}

# A meter sending two parts of one reading, so the graph draws two traces:
# the UT61E+ in AC+DC V across a 1.6 V cell, DC plotted and AC beside it, with
# the series chips and the key. The preseed puts the 1m window over the second
# lead lift (65–76 s), where DC falls to nothing and AC picks up the pickup.
scene_series() {
	write_settings
	launch acdcv-cell 100
	park
	capture gui-graph-series.png "$GRAPH_CROP"
}

# Big meter on a UT181A frame, which carries sub-values: the reading scaled to
# the window with its frequency and period under it, the mode line, the remote
# buttons and the corner button that leaves the mode.
scene_big_meter() {
	local geometry
	write_settings '{"device_family": "ut181a"}'
	launch ut181a-vac-hz 60
	key ctrl+b
	# The big meter sizes its reading over a few frames, starting from where
	# the last fit ended: resizing before this one settles would start the
	# next from a different point, and a different picture.
	"$GUI" settle >/dev/null
	geometry="$(fit "$BIG_METER_W" "$BIG_METER_H")"
	capture gui-big-meter.png "$geometry+0+0"
}

# Ctrl+B again drops the top bar and the buttons: the reading and its mode
# line only, one picture per shape. A reading with no sub-values under it, so
# the wide shape has the room to put the mode line beside the value.
scene_minimal_meter() {
	local geometry
	"$GUI" stop >/dev/null
	export VERIFY_GUI_GEOMETRY="$METER_GEOMETRY"
	write_settings
	launch dcma-boot-refresh 166
	# Settled after each press, as in scene_big_meter; a fit draws frames
	# back to back, and a second press landing in one of them is lost.
	key ctrl+b
	"$GUI" settle >/dev/null
	key ctrl+b
	"$GUI" settle >/dev/null
	geometry="$(fit "$MINIMAL_WIDE_W" "$MINIMAL_WIDE_H")"
	capture gui-minimal-meter-wide.png "$geometry+0+0"
	geometry="$(fit "$MINIMAL_NARROW_W" "$MINIMAL_NARROW_H")"
	capture gui-minimal-meter-narrow.png "$geometry+0+0"
	export VERIFY_GUI_GEOMETRY="$ROOT_GEOMETRY"
}

# The settings panel as a user with no meter plugged in meets it: Auto-detect
# selected, every row live and at its default. A recording would pin the
# Device row instead.
scene_settings() {
	no_meter_or_skip gui-settings.png || return 0
	write_settings '{"device_family": "auto"}'
	launch_without_meter
	click "$GEAR_X" "$GEAR_Y"
	park
	capture gui-settings.png "$SETTINGS_CROP"
}

# One graph picture per colour preset, with THEMES_VIEW's mean and reference
# line, so each shows the palette on a trace, a mean line, a reference line
# and its trigger markers.
scene_themes() {
	local entry asset preset
	for entry in \
		'gui-theme-dark.png {"theme": "Dark", "color_preset": "Default"}' \
		'gui-theme-light.png {"theme": "Light", "color_preset": "Default"}' \
		'gui-theme-high-contrast.png {"theme": "Dark", "color_preset": "HighContrast"}' \
		'gui-theme-colorblind.png {"theme": "Dark", "color_preset": "ColorblindSafe"}'; do
		asset="${entry%% *}"
		preset="${entry#* }"
		write_settings "$preset"
		launch_file "$(staged dcma-boot-refresh "$THEMES_VIEW")" 175
		park
		capture "$asset" "$THEME_CROP"
	done
}

# Per-colour editing: the swatch grid expanded, with one swatch's picker open.
scene_color_customization() {
	no_meter_or_skip gui-color-customization.png || return 0
	write_settings '{"device_family": "auto"}'
	launch_without_meter
	click "$GEAR_X" "$GEAR_Y"
	# A click before the panel has settled misses the header.
	"$GUI" settle >/dev/null
	click "$CUSTOMIZE_X" "$CUSTOMIZE_Y"
	click "$CROSSHAIR_SWATCH_X" "$CROSSHAIR_SWATCH_Y"
	capture gui-color-customization.png "$COLOR_CROP"
}

# The help shown when nothing answers on either link.
scene_connection_help() {
	no_meter_or_skip gui-connection-help.png || return 0
	write_settings '{"device_family": "auto"}'
	launch_without_meter
	# The failure is not instant: the open path scans for a Bluetooth adapter
	# once the bus has nothing, and an adapter that answers the scan but not
	# the connect takes the transport's connect timeout on top. The help only
	# goes up once that has run out — before it, the column says "Detecting
	# the meter…" — and the app logs it as an error.
	"$GUI" wait-log "UI: error:" 60 >/dev/null
	park
	capture gui-connection-help.png "$HELP_CROP"
}

## Driver ####################################################################

scene_for() {
	local asset="$1" entry
	for entry in "${SCENES[@]}"; do
		[ "${entry%% *}" = "$asset" ] && {
			echo "${entry#* }"
			return 0
		}
	done
	return 1
}

# A scene can write more than one picture from one launch, so asking for any
# one of them runs it once and refreshes them all.
run_scenes() {
	local asset fn prev seen ran=()
	for asset in "$@"; do
		fn="$(scene_for "$asset")" || die "no scene writes '$asset' — try: $(basename "$0") list"
		seen=0
		for prev in ${ran[@]+"${ran[@]}"}; do [ "$prev" = "$fn" ] && seen=1; done
		[ "$seen" = 1 ] && continue
		ran+=("$fn")
		"$fn"
		"$GUI" stop >/dev/null
	done
}

cmd_list() {
	local entry
	for entry in "${SCENES[@]}"; do echo "${entry%% *}"; done
}

main() {
	[ $# -gt 0 ] || die "usage: $(basename "$0") {list | all | <asset.png>…}"
	if [ "$1" = list ]; then
		cmd_list
		return 0
	fi
	require Xvfb xdotool import convert compare identify python3 cargo
	[ -x "$GUI" ] || die "missing $GUI"
	TMP="$(mktemp -d "${TMPDIR:-/tmp}/doc-screenshots.XXXXXX")"
	trap 'rm -rf "$TMP"; "$GUI" stop >/dev/null 2>&1 || true' EXIT
	if [ "$1" = all ]; then
		local assets
		mapfile -t assets < <(cmd_list)
		run_scenes "${assets[@]}"
	else
		run_scenes "$@"
	fi
}

main "$@"
