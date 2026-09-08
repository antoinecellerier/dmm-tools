#!/usr/bin/env bash
# BROWSER target for dmm-gui under verify-gui. The app's hyperlinks go through
# the webbrowser crate, which tries $BROWSER before xdg-open; without this the
# URL reaches the user's real browser even though the window is on the private
# display. Record the URL where the caller asked and report success.
printf 'verify-gui: blocked browser open: %s\n' "$*" >>"${VERIFY_GUI_LOG:-/dev/stderr}"
exit 0
