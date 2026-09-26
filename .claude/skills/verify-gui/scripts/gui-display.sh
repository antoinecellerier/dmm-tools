#!/usr/bin/env bash
# Drive dmm-gui on a private Xvfb display for headless visual checks.
#
# Never touches the live desktop: every child process gets DISPLAY=:N with
# WAYLAND_DISPLAY removed, so winit cannot pick the Wayland backend and open a
# window on the user's screen. The caller's own DISPLAY/WAYLAND_DISPLAY are
# never modified.
set -euo pipefail
# Byte semantics for every [0-9] below: digit ranges are locale-collated, so
# under a UTF-8 locale Arabic-Indic digits and '²' pass an ASCII-looking class.
export LC_ALL=C

# No /tmp fallback: a predictable, world-writable path could be pre-planted.
STATE="${VERIFY_GUI_STATE:-${XDG_RUNTIME_DIR:?verify-gui needs XDG_RUNTIME_DIR}/verify-gui}"
CONFIG="$STATE/config" # private XDG_CONFIG_HOME so the user's settings.json is untouched
LOG="$STATE/gui.log"
# Root window WxHxDEPTH. 'start' reuses a running Xvfb, so change this only
# after a 'stop'. The default holds the default dmm-gui window at 1x with margin.
GEOMETRY="${VERIFY_GUI_GEOMETRY:-1600x1000x24}"
DISPLAY_MIN=99          # :0 and :1 belong to the user's real session
DISPLAY_MAX=110         # give up rather than wander into unknown displays
XVFB_TRIES=50           # x 0.1s = 5s for the X server to accept clients
WINDOW_TIMEOUT=20       # seconds for the window to map (cold debug start)
FIRST_FRAMES=3          # seconds for the mock device to connect and draw samples
CHORD_HOLD=0.3          # seconds: longer than one egui frame, shorter than key repeat
RESIZE_TIMEOUT=5        # seconds: --sync hangs if the app resizes back before the first poll
GUI_EXIT_TRIES=30       # x 0.1s = 3s to exit on SIGTERM before SIGKILL
MIN_PNG_BYTES=1024      # smaller means a truncated or failed capture
MIN_COLORS=100          # a real frame has many colours; a flat fill has one

die() { echo "verify-gui: $*" >&2; exit 1; }

require() {
	local t missing=()
	for t in "$@"; do command -v "$t" >/dev/null 2>&1 || missing+=("$t"); done
	((${#missing[@]} == 0)) ||
		die "missing tool(s): ${missing[*]} — ask the user to run: sudo apt install xvfb xdotool imagemagick python3-pil"
}

state_get() { if [ -f "$STATE/$1" ]; then cat "$STATE/$1"; fi; }
# A pid read from a state file is data: it must be a positive integer (never
# -1 or 0, which would signal every process) and, where we kill, still be the
# program we started rather than a reused pid.
alive() { [[ "${1:-}" =~ ^[1-9][0-9]*$ ]] && kill -0 "$1" 2>/dev/null; }
alive_as() { alive "${1:-}" && [ "$(cat "/proc/$1/comm" 2>/dev/null)" = "$2" ]; }

need_display() {
	local d
	d="$(state_get display)"
	[ -n "$d" ] && alive_as "$(state_get xvfb.pid)" Xvfb || die "no private display — run 'start' first"
	# Only a display this script could have started: a tampered state file must
	# never point input or capture at the user's real session.
	[[ "$d" =~ ^:[0-9]+$ ]] && ((${d#:} >= DISPLAY_MIN && ${d#:} <= DISPLAY_MAX)) || die "refusing display '$d'"
	echo "$d"
}

need_wid() {
	local w
	w="$(state_get wid)"
	[ -n "$w" ] && alive_as "$(state_get gui.pid)" dmm-gui || die "no running dmm-gui — run 'run' first"
	# The id reaches xdotool as an argument: a tampered state file must not smuggle flags.
	[[ "$w" =~ ^[0-9]+$ ]] || die "refusing window id '$w'"
	echo "$w"
}

# Run a command against the private display only.
onx() {
	local d
	d="$(need_display)" || exit 1 # a die inside the substitution must not leave DISPLAY empty
	env -u WAYLAND_DISPLAY DISPLAY="$d" "$@"
}

kill_gui() {
	local pid i
	pid="$(state_get gui.pid)"
	if alive_as "$pid" dmm-gui; then
		kill "$pid" 2>/dev/null || true
		for ((i = 0; i < GUI_EXIT_TRIES; i++)); do
			alive_as "$pid" dmm-gui || break
			sleep 0.1
		done
		alive_as "$pid" dmm-gui && kill -9 "$pid" 2>/dev/null || true
	fi
	rm -f "$STATE/gui.pid" "$STATE/wid"
}

cmd_start() {
	require Xvfb xdotool
	# Bounded to five digits per axis: a typo must not ask Xvfb for gigabytes.
	[[ "$GEOMETRY" =~ ^[1-9][0-9]{0,4}x[1-9][0-9]{0,4}x(8|16|24)$ ]] ||
		die "VERIFY_GUI_GEOMETRY must be WxHxDEPTH with depth 8, 16 or 24 (got '$GEOMETRY')"
	mkdir -p -m 700 "$STATE" "$CONFIG"
	[ -O "$STATE" ] && [ ! -L "$STATE" ] || die "state dir $STATE is not ours"
	chmod 700 "$STATE"
	local disp pid n i
	disp="$(state_get display)"
	pid="$(state_get xvfb.pid)"
	if [ -n "$disp" ] && alive_as "$pid" Xvfb; then
		echo "reusing private display $disp (Xvfb pid $pid)"
		return 0
	fi
	disp=""
	for ((n = DISPLAY_MIN; n <= DISPLAY_MAX; n++)); do
		if [ ! -e "/tmp/.X$n-lock" ]; then
			disp=":$n"
			break
		fi
	done
	[ -n "$disp" ] || die "no free display between :$DISPLAY_MIN and :$DISPLAY_MAX"
	env -u WAYLAND_DISPLAY Xvfb "$disp" -screen 0 "$GEOMETRY" -nolisten tcp >"$STATE/xvfb.log" 2>&1 &
	pid=$!
	echo "$disp" >"$STATE/display"
	echo "$pid" >"$STATE/xvfb.pid"
	for ((i = 0; i < XVFB_TRIES; i++)); do
		# If our Xvfb lost the display to another server, the probe below would
		# answer for that server and every later step would target it.
		alive "$pid" || {
			rm -f "$STATE/display" "$STATE/xvfb.pid"
			die "Xvfb on $disp exited (display taken?); see $STATE/xvfb.log"
		}
		if env -u WAYLAND_DISPLAY DISPLAY="$disp" xdotool getdisplaygeometry >/dev/null 2>&1; then
			echo "started private display $disp (Xvfb pid $pid, ${GEOMETRY%x*})"
			return 0
		fi
		sleep 0.1
	done
	rm -f "$STATE/display" "$STATE/xvfb.pid"
	die "Xvfb on $disp did not come up; see $STATE/xvfb.log"
}

cmd_run() {
	require xdotool cargo
	local root disp pid wid i dev="" replay=0
	root="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel)}"
	[ -f "$root/Cargo.toml" ] || die "no Cargo.toml in $root"
	local script_dir
	script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
	[ -x "$script_dir/blocked-browser.sh" ] || die "missing $script_dir/blocked-browser.sh"
	# webbrowser splits $BROWSER on ':' and whitespace; a path containing either
	# would silently fall through to xdg-open and the user's real browser.
	case "$script_dir" in *[:[:space:]]*) die "skill path must not contain ':' or spaces: $script_dir" ;; esac
	local args=("$@")
	for ((i = 0; i < ${#args[@]}; i++)); do
		case "${args[i]}" in
		--device) dev="${args[i + 1]:-}" ;;
		--device=*) dev="${args[i]#--device=}" ;;
		# A recording names its own meter and opens no cable, so it neither
		# takes the --device mock default nor counts as hardware.
		--replay | --replay=*) replay=1 ;;
		esac
	done
	if [ -z "$dev" ] && [ "$replay" = 0 ]; then
		args=(--device mock ${args[@]+"${args[@]}"})
	elif [ -n "$dev" ] && [ "${VERIFY_GUI_ALLOW_HW:-0}" != 1 ]; then
		# Real meters need the user's go-ahead (CLAUDE.md); this grant is prompt-free.
		# `mock` and `mock-*` are simulated: a registry test keeps any name with
		# that prefix off every entry that needs hardware.
		case "$dev" in
		mock | mock-*) ;;
		*) die "--device $dev would open real hardware; ask the user, then set VERIFY_GUI_ALLOW_HW=1" ;;
		esac
	fi
	cmd_start
	kill_gui
	(cd "$root" && cargo build -q -p dmm-gui) || die "cargo build -p dmm-gui failed"
	disp="$(need_display)"
	# BROWSER keeps the app's hyperlinks off the user's real browser: the
	# webbrowser crate tries it before xdg-open; the stub logs the URL instead.
	# A dead session-bus address keeps the CSV save dialog (rfd, via the
	# xdg-desktop-portal) and AccessKit off the user's desktop; unsetting the
	# variable is not enough because zbus falls back to $XDG_RUNTIME_DIR/bus.
	# GDK_BACKEND=x11 pins that fallback (GTK) to DISPLAY: with WAYLAND_DISPLAY
	# unset, GTK still connects to the default wayland-0 socket, i.e. the user's
	# compositor. The log is opened for append so the stub's lines survive.
	: >"$LOG"
	env -u WAYLAND_DISPLAY DISPLAY="$disp" XDG_SESSION_TYPE=x11 LIBGL_ALWAYS_SOFTWARE=1 \
		GDK_BACKEND=x11 XDG_CONFIG_HOME="$CONFIG" XDG_DATA_HOME="$CONFIG/data" \
		RUST_LOG=dmm_gui=info BROWSER="$script_dir/blocked-browser.sh" VERIFY_GUI_LOG="$LOG" \
		DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent/verify-gui-no-bus \
		"$root/target/debug/dmm-gui" "${args[@]}" >>"$LOG" 2>&1 &
	pid=$!
	echo "$pid" >"$STATE/gui.pid"
	wid="$(env -u WAYLAND_DISPLAY DISPLAY="$disp" timeout "$WINDOW_TIMEOUT" \
		xdotool search --sync --onlyvisible --pid "$pid" 2>/dev/null | head -1 || true)"
	if [ -z "$wid" ]; then
		kill_gui
		die "no window on $disp within ${WINDOW_TIMEOUT}s — dmm-gui died or drew elsewhere; log: $LOG"
	fi
	echo "$wid" >"$STATE/wid"
	sleep "$FIRST_FRAMES"
	echo "launched dmm-gui ${args[*]} on $disp (pid $pid)"
	echo "WID=$wid"
	echo "log: $LOG"
}

cmd_shot() {
	require import
	local out="${1:-}" size
	[ -n "$out" ] || die "usage: shot <out.png>"
	# A plain .png path only: ImageMagick would take "txt:/path" or a ".json"
	# suffix as a coder and write any file the user can, with no prompt.
	case "$out" in
	-* | *:*) die "shot needs a plain .png path" ;;
	*.png) ;;
	*) die "shot writes .png only" ;;
	esac
	[ ! -L "$out" ] || die "refusing symlink $out"
	onx import -window root "$out" || die "import failed — is the private display up?"
	[ -f "$out" ] || die "no screenshot written to $out"
	size="$(stat -c %s "$out")"
	[ "$size" -ge "$MIN_PNG_BYTES" ] ||
		die "screenshot $out is ${size}B (< ${MIN_PNG_BYTES}B) — capture failed"
	echo "wrote $out (${size} bytes)"
}

cmd_key() {
	require xdotool
	local chord="${1:-}" wid key m i
	[ -n "$chord" ] || die "usage: key <chord>   e.g. ctrl+shift+c, ctrl+o, space"
	[[ "$chord" =~ ^[A-Za-z0-9_]+(\+[A-Za-z0-9_]+)*$ ]] || die "chord must be keysyms joined by '+', e.g. ctrl+shift+c"
	wid="$(need_wid)"
	local parts=() mods=() cmd=()
	IFS='+' read -r -a parts <<<"$chord"
	key="${parts[-1]}"
	for m in "${parts[@]:0:${#parts[@]}-1}"; do
		case "${m,,}" in
		ctrl | control) mods+=(ctrl) ;;
		shift) mods+=(shift) ;;
		alt) mods+=(alt) ;;
		super | meta | cmd) mods+=(super) ;;
		*) die "unknown modifier '$m' in '$chord' (ctrl, shift, alt, super)" ;;
		esac
	done
	onx xdotool windowactivate --sync "$wid" >/dev/null 2>&1 || true # no WM on Xvfb; focus is what matters
	onx xdotool windowfocus --sync "$wid" || die "could not focus window $wid"
	# Hold the modifiers across a frame: egui reads its modifier snapshot when the
	# frame runs, so a chord released within a millisecond can arrive bare.
	for m in ${mods[@]+"${mods[@]}"}; do cmd+=(keydown "$m"); done
	cmd+=(key "$key" sleep "$CHORD_HOLD")
	for ((i = ${#mods[@]} - 1; i >= 0; i--)); do cmd+=(keyup "${mods[i]}"); done
	onx xdotool "${cmd[@]}" || die "xdotool failed to send '$chord'"
	echo "sent $chord to window $wid"
}

cmd_click() {
	require xdotool
	local x="${1:-}" y="${2:-}" wid
	[[ "$x" =~ ^[0-9]+$ ]] && [[ "$y" =~ ^[0-9]+$ ]] || die "usage: click <x> <y>   (window-relative pixels)"
	wid="$(need_wid)"
	# Hold the button across a frame, as a hand does: a press and release that
	# land in the same egui frame register as a click but never as the button
	# being down, which is what the minimap pans on.
	onx xdotool mousemove --window "$wid" "$x" "$y" mousedown 1 sleep "$CHORD_HOLD" mouseup 1 || {
		# A chain that failed mid-way can leave the button held, silently turning
		# every later click on this display into a drag.
		onx xdotool mouseup 1 >/dev/null 2>&1 || true
		die "click at $x,$y failed"
	}
	echo "clicked $x,$y in window $wid"
}

cmd_wheel() {
	require xdotool
	local x="${1:-}" y="${2:-}" dir="${3:-down}" mod="${4:-}" wid btn
	local usage="usage: wheel <x> <y> [up|down] [ctrl]   (window-relative pixels)"
	[[ "$x" =~ ^[0-9]+$ ]] && [[ "$y" =~ ^[0-9]+$ ]] || die "$usage"
	case "$dir" in
	up) btn=4 ;;
	down) btn=5 ;;
	*) die "$usage — direction is 'up' or 'down'" ;;
	esac
	[ -z "$mod" ] || [ "$mod" = ctrl ] || die "$usage — the only modifier is 'ctrl'"
	wid="$(need_wid)"
	onx xdotool mousemove --window "$wid" "$x" "$y" || die "could not move the pointer to $x,$y"
	if [ "$mod" = ctrl ]; then
		# Hold Ctrl across a frame as cmd_key does: egui reads its modifier snapshot
		# when the frame runs, so a chord released within a millisecond arrives bare.
		onx xdotool keydown ctrl sleep "$CHORD_HOLD" click "$btn" sleep "$CHORD_HOLD" keyup ctrl || {
			# A chain that failed mid-way can leave Ctrl held, silently tainting
			# every later key, click and shot on this display.
			onx xdotool keyup ctrl >/dev/null 2>&1 || true
			die "ctrl+wheel $dir at $x,$y failed"
		}
	else
		onx xdotool click "$btn" || die "wheel $dir at $x,$y failed"
	fi
	echo "sent ${mod:+ctrl+}wheel $dir at $x,$y in window $wid"
}

cmd_resize() {
	require xdotool
	local w="${1:-}" h="${2:-}" wid geom gw gh
	[[ "$w" =~ ^[1-9][0-9]*$ ]] && [[ "$h" =~ ^[1-9][0-9]*$ ]] ||
		die "usage: resize <width> <height>   (positive integers, pixels at 1x)"
	wid="$(need_wid)"
	onx timeout "$RESIZE_TIMEOUT" xdotool windowsize --sync "$wid" "$w" "$h" ||
		die "could not resize window $wid to ${w}x${h}"
	# No WM on Xvfb, so the app's MinInnerSize hint is not enforced; give the app a
	# frame to re-grow a window below its own minimum and report what it settled on.
	sleep "$CHORD_HOLD"
	geom="$(onx xdotool getwindowgeometry --shell "$wid")" || die "could not read the geometry of window $wid"
	gw="$(printf '%s\n' "$geom" | sed -n 's/^WIDTH=\([0-9]\{1,\}\)$/\1/p')"
	gh="$(printf '%s\n' "$geom" | sed -n 's/^HEIGHT=\([0-9]\{1,\}\)$/\1/p')"
	[ -n "$gw" ] && [ -n "$gh" ] || die "xdotool reported no size for window $wid"
	echo "window $wid is ${gw}x${gh} (asked ${w}x${h})"
}

cmd_stop() {
	kill_gui
	local pid
	pid="$(state_get xvfb.pid)"
	if alive_as "$pid" Xvfb; then
		kill "$pid" 2>/dev/null || true
		echo "stopped Xvfb (pid $pid) on $(state_get display)"
	else
		echo "nothing running"
	fi
	rm -f "$STATE/display" "$STATE/xvfb.pid"
}

cmd_status() {
	local d xp gp w
	d="$(state_get display)" xp="$(state_get xvfb.pid)"
	gp="$(state_get gui.pid)" w="$(state_get wid)"
	echo "state dir: $STATE"
	echo "display:   ${d:-none} (Xvfb pid ${xp:-none}, $(alive "$xp" && echo running || echo down))"
	echo "dmm-gui:   pid ${gp:-none} ($(alive "$gp" && echo running || echo down)), window id ${w:-none}"
	echo "log:       $LOG"
}

cmd_selftest() {
	require Xvfb xdotool import
	local png="$STATE/selftest.png" colors rc=0
	cmd_start
	cmd_run --device mock
	cmd_shot "$png"
	if python3 -c "import PIL" >/dev/null 2>&1; then
		colors="$(python3 -c 'import sys;from PIL import Image;print(len(Image.open(sys.argv[1]).convert("RGB").getcolors(1 << 24) or []))' "$png")"
	else
		colors="$(identify -format %k "$png")"
	fi
	[ "$colors" -gt "$MIN_COLORS" ] || rc=1
	cmd_stop
	[ "$rc" = 0 ] || die "screenshot has only $colors unique colours (<= $MIN_COLORS) — the window did not render"
	echo "$png: $colors unique colours"
	echo "selftest OK"
}

sub="${1:-}"
shift || true
case "$sub" in
start | run | shot | key | click | wheel | resize | stop | status | selftest) "cmd_$sub" "$@" ;;
*) die "usage: $(basename "$0") {start|run [dmm-gui args...]|shot <out.png>|key <chord>|click <x> <y>|wheel <x> <y> [up|down] [ctrl]|resize <width> <height>|stop|status|selftest}" ;;
esac
