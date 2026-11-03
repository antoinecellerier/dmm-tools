#!/bin/bash
# Cloud sessions only: what a fresh container lacks for `cargo build` and the
# commit hooks (docs/development.md). Local checkouts follow that guide.
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "${CLAUDE_PROJECT_DIR:-.}"

# The pre-commit (fmt, clippy, tests) and commit-msg hooks, copied as they
# are at session start rather than followed through core.hooksPath: a branch
# checked out later could otherwise run its own git-hooks/post-checkout.
cp git-hooks/pre-commit git-hooks/commit-msg "$(git rev-parse --git-path hooks)/"

# hidapi needs libudev, the Bluetooth transport libdbus; both via pkg-config.
missing=()
for pkg in pkg-config libudev-dev libdbus-1-dev; do
  status=$(dpkg-query -W -f='${db:Status-Status}' "$pkg" 2>/dev/null || true)
  [ "$status" = installed ] || missing+=("$pkg")
done
if [ "${#missing[@]}" -gt 0 ]; then
  apt-get update -qq
  DEBIAN_FRONTEND=noninteractive apt-get install -y -qq "${missing[@]}" >/dev/null
fi
