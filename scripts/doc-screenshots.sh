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
# against the committed file and leaves that file alone when there is none;
# Xvfb is not guaranteed to render identically across driver versions, so a
# small delta is still possible — look at the PNGs before committing them.
set -euo pipefail

# Byte semantics for the [0-9] classes below, and a stable number format in
# whatever the GUI prints into a picture.
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUI="$ROOT/.claude/skills/verify-gui/scripts/gui-display.sh"
ASSETS="$ROOT/assets"
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
# Where the refresh's two 1 mA crossings sit in the 30 s window ending at
# 178 s. A click snaps to the nearest sample and the samples are 100 ms apart,
# about 4 px here, so each x is nudged off the crossing onto the sample the
# picture wants: A onto the rising edge's first reading above 1 mA (149.18 s)
# and B onto the first idle reading after the fall (170.07 s), far enough from
# the plot's right edge for its readout to fit.
CURSOR_A_X=668; CURSOR_B_X=1565; CURSOR_Y=741
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
# The seconds field right of the Min/Max chip, which the envelope averages
# over. A whole-window 60 s band shows the boot's peak and floor; the 1 s
# default just shadows the trace.
ENVELOPE_FIELD_X=732; ENVELOPE_FIELD_Y=123
# The hero's view is picked off the minimap: a click at 16.5 s of the 160 s
# session (the strip starts at x 505, about 8.66 px/s) centres the 30s window
# on the boot, so it runs from 1.6 s to 31.6 s with the 30s chip still lit.
HERO_MINIMAP_X=648; HERO_MINIMAP_Y=925
# Its cursors sit on the idle floor either side of the boot, so both level
# lines lie on 0 mA rather than across the plot: A on the last reading before
# the rise (2.376 s) and B on the first after the fall (29.995 s). A sample is
# about 4 px wide in this window.
HERO_CURSOR_A_X=649; HERO_CURSOR_B_X=1834; HERO_CURSOR_Y=700
# The same click in the narrow window's minimap, which starts at x 20 and
# packs the session into about 5.98 px/s.
NARROW_MINIMAP_X=119; NARROW_MINIMAP_Y=925

# asset written -> the function that stages it.
SCENES=(
	"gui-wide-layout.png scene_wide"
	"gui-narrow-layout.png scene_narrow"
	"gui-reading-controls.png scene_reading_controls"
	"gui-graph-overlays.png scene_overlays"
	"gui-graph-triggers.png scene_overlays"
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
	"$GUI" run --replay "$REPLAYS/$replay.replay" \
		--mock-clock-preseed "$preseed" --mock-clock-scale "$CLOCK_SCALE" "$@" >/dev/null
}

# The app on Auto-detect with nothing to detect, for the pictures that must
# show the settings and the help a user meets before any meter answers. The
# grant is what lets --device name something other than the mock gui-display
# would otherwise pass; the guard every caller runs first has already made
# sure there is no meter to open.
launch_without_meter() {
	VERIFY_GUI_ALLOW_HW=1 "$GUI" run --device auto >/dev/null
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
	listing="$(cd "$ROOT" && cargo run -q -p dmm-cli -- list 2>&1 || true)"
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
key() {
	"$GUI" key "$1" >/dev/null
	sleep 0.5
}

click() { "$GUI" click "$1" "$2" >/dev/null; sleep 0.5; }

park() { click "$PARK_X" "$PARK_Y"; }

# shot <out.png> [crop]
shot() {
	local out="$1" crop="${2:-}"
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
	sleep 1
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
# [ picks the 30s preset and the minimap click moves the window back onto the
# boot; M adds the window's mean and C the cursors that span the boot.
scene_wide() {
	write_settings
	launch dcma-boot-refresh 160.5
	key bracketleft
	click "$HERO_MINIMAP_X" "$HERO_MINIMAP_Y"
	key m
	key c
	click "$HERO_CURSOR_A_X" "$HERO_CURSOR_Y"
	click "$HERO_CURSOR_B_X" "$HERO_CURSOR_Y"
	park
	capture gui-wide-layout.png
}

# The hero's session and window in a window too narrow for two columns. The
# mean stays but the cursors do not: the plot is too short for their readouts
# to find a corner off the trace.
scene_narrow() {
	write_settings
	launch dcma-boot-refresh 160.5
	"$GUI" resize 1000 1280
	sleep 1
	key bracketleft
	click "$NARROW_MINIMAP_X" "$NARROW_MINIMAP_Y"
	key m
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

# Two graph pictures from one launch: the overlays that read the last refresh
# cycle off the live view, then the ones that mark the boot sequence, picked
# out of the session's history. They are separate because triggers and cursors
# land on the same two crossings, and because only one of them can be live.
#
# [ steps the window from 1m down to 30s, M draws the mean and C arms the
# cursors. Then the same keys take those off and raise the second picture's:
# R turns the reference lines on and puts the caret in their field, Escape
# gives the keyboard back to the graph, X adds the envelope over the window
# its field is set to, ] steps back to 1m and Home jumps to the start of the
# session, which leaves live mode and puts the minimap's brackets over the
# boot. The trigger markers are on by default and show as soon as there is a
# reference line to cross.
scene_overlays() {
	write_settings
	launch dcma-boot-refresh 178
	key bracketleft
	key m
	key c
	click "$CURSOR_A_X" "$CURSOR_Y"
	click "$CURSOR_B_X" "$CURSOR_Y"
	park
	capture gui-graph-overlays.png "$GRAPH_CROP"
	key c # off, which drops both cursors
	key m
	key r
	key 1
	key Return
	key Escape
	key x
	click "$ENVELOPE_FIELD_X" "$ENVELOPE_FIELD_Y"
	key ctrl+a
	key 6
	key 0
	key Return
	key Escape
	key bracketright
	key Home
	capture gui-graph-triggers.png "$GRAPH_CROP"
}

# Big meter on a UT181A frame, which carries sub-values: the reading scaled to
# the window with its frequency and period under it, the mode line, the remote
# buttons and the corner button that leaves the mode.
scene_big_meter() {
	local geometry
	write_settings '{"device_family": "ut181a"}'
	launch ut181a-vac-hz 60
	key ctrl+b
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
	key ctrl+b
	key ctrl+b
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

# One graph picture per colour preset. Same keys as the overlays scene minus
# the cursors, so each shows the palette on a trace, a mean line, a reference
# line and its trigger markers.
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
		launch dcma-boot-refresh 175
		key bracketleft
		key m
		key r
		key 1
		key Return
		key Escape
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
	# the meter…".
	sleep 20
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
