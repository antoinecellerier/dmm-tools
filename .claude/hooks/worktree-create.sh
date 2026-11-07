#!/bin/bash
# WorktreeCreate: makes the worktree Claude Code asks for (--worktree,
# EnterWorktree, `isolation: worktree` subagents) and seeds its target/ from
# the main checkout's build, so it reuses every dependency build instead of
# spending about 15 minutes rebuilding them ("Build speed & disk usage" in
# docs/development.md).
#
# Claude Code reads the last stdout line as the worktree's path; everything
# else goes to stderr. Seeding is best-effort: if any step of it fails, the
# worktree still comes back, with an empty target/.
set -euo pipefail

# git reads these over any -C.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR

input=$(cat)
field() { # field NAME: a string field of the JSON input, or empty
  if command -v jq >/dev/null; then
    jq -r --arg k "$1" '.[$k] // empty' <<<"$input"
  else
    python3 -I -c 'import json,sys; print(json.loads(sys.argv[1]).get(sys.argv[2]) or "")' "$input" "$1"
  fi
}
name=$(field name)
cwd=$(field cwd)

# The name becomes a path and a branch: a slug only.
if ! [[ "$name" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || [[ "$name" == *..* ]] \
  || [[ "$name" == *.lock ]] || [[ "$name" == *. ]]; then
  echo "worktree-create: refusing worktree name '$name'" >&2
  exit 1
fi

# The main checkout: the first entry `git worktree list` gives, found from
# the project root rather than the input's cwd, which could sit in a nested
# clone. A bare repository has no checkout to seed from.
# The whole run fits a 270 s budget, inside the 300 s timeout in
# .claude/settings.json; seeding gives up first, so the worktree still
# comes back.
deadline=$((SECONDS + 270))
left() { echo $((deadline - SECONDS)); }

project=${CLAUDE_PROJECT_DIR:-$cwd}
common=$(git -C "$project" rev-parse --path-format=absolute --git-common-dir)
# Parallel creations take turns from here to `worktree add`: `worktree
# list` fails on a worktree another call is still adding.
if command -v flock >/dev/null; then
  exec 7>>"$common/claude-worktree-create.lock"
  flock -w 60 7 || { echo "worktree-create: another creation held the lock for a minute" >&2; exit 1; }
fi
list=$(git -C "$project" worktree list --porcelain)
root=$(sed -n '1s/^worktree //p' <<<"$list")
if [ ! -d "$root" ] || sed -n '2,/^$/p' <<<"$list" | grep -qx bare; then
  echo "worktree-create: no main checkout found from $project" >&2
  exit 1
fi
dir="$root/.claude/worktrees/$name"
branch="worktree-$name"

# An existing worktree of this repository by that name is reopened, as
# Claude Code does without this hook.
if [ -e "$dir" ]; then
  if [ "$(git -C "$dir" rev-parse --show-toplevel 2>/dev/null)" = "$dir" ] &&
    [ "$(git -C "$dir" rev-parse --path-format=absolute --git-common-dir)" = "$common" ]; then
    rm -rf "$dir/target.seed"
    echo "$dir"
    exit 0
  fi
  echo "worktree-create: $dir exists and is not a worktree of this repository" >&2
  exit 1
fi

# The hook always branches from the session's HEAD when the session runs in
# this repository (unpushed commits come along), the main checkout's
# otherwise.
base=$(git -C "$root" rev-parse HEAD)
if [ -n "$cwd" ] &&
  [ "$(git -C "$cwd" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" = "$common" ]; then
  base=$(git -C "$cwd" rev-parse HEAD)
fi
mkdir -p "$root/.claude/worktrees"
# A directory deleted without `git worktree prune` stays registered, and
# add refuses it unless forced; force only when git marks that very path
# prunable.
force=()
if awk -v d="worktree $dir" '$0 == d { f = 1; next } /^worktree / { f = 0 } f && /^prunable/ { p = 1 } END { exit !p }' \
  <<<"$list"; then
  force=(-f)
fi
# A branch left by an earlier worktree of that name restarts from the base,
# unless it holds commits of its own.
if git -C "$root" show-ref --verify --quiet "refs/heads/$branch"; then
  if git -C "$root" merge-base --is-ancestor "$branch" "$base"; then
    git -C "$root" worktree add "${force[@]}" -B "$branch" "$dir" "$base" >&2
  else
    echo "worktree-create: $branch holds commits of its own, checking it out as it is" >&2
    git -C "$root" worktree add "${force[@]}" "$dir" "$branch" >&2
  fi
else
  git -C "$root" worktree add "${force[@]}" -b "$branch" "$dir" "$base" >&2
fi
exec 7>&-

# The copy and the clean happen in target.seed, which cargo never reads;
# only a cleaned copy becomes target/, so a kill part way leaves nothing
# cargo would use.
seed() {
  local src="$root/target/debug" tmp="$dir/target.seed" need_kb free_kb
  [ -d "$src" ] || { echo "worktree-create: no $src to seed from" >&2; return 0; }
  command -v flock >/dev/null || { echo "worktree-create: no flock, not seeding" >&2; return 0; }
  # Parallel subagents seed one at a time. A seed takes well under 90 s; a
  # turn that would come later than that before the deadline isn't waited
  # for.
  local turn=$(( $(left) - 90 ))
  [ "$turn" -gt 0 ] || { echo "worktree-create: no time left to seed" >&2; return 1; }
  exec 8>>"$root/.claude/worktrees/.seed-lock"
  flock -w "$turn" 8 || { echo "worktree-create: other seeds took the time, not seeding" >&2; return 1; }
  # Cargo holds .cargo-lock shared for a whole build: holding it
  # exclusively waits out a running build and holds off new ones, so no
  # half-written artifact gets copied behind a fresh fingerprint.
  exec 9>>"$src/.cargo-lock"
  flock -w 20 9 || { echo "worktree-create: the main checkout is building, not seeding" >&2; return 1; }
  # Checked under the lock, so parallel seeds can't each count the same
  # free space. Commands below don't inherit the locks: if this hook is
  # killed, nothing keeps holding them.
  need_kb=$(timeout 10 du -sk "$src" 8>&- 9>&- | cut -f1)
  free_kb=$(df -Pk "$root" | awk 'NR == 2 { print $4 }')
  if ! [[ "$need_kb" =~ ^[0-9]+$ && "$free_kb" =~ ^[0-9]+$ ]] ||
    [ "$free_kb" -lt $(( need_kb + 10 * 1024 * 1024 )) ]; then
    echo "worktree-create: not enough free space to seed" >&2
    return 1
  fi
  mkdir -p "$tmp"
  cp "$root/target/CACHEDIR.TAG" "$tmp/" || return 1 # cargo clean wants it
  [ "$(left)" -gt 45 ] || { echo "worktree-create: no time left to seed" >&2; return 1; }
  timeout "$(( $(left) - 35 ))" cp -a "$src" "$tmp/" 8>&- 9>&- || return 1
  exec 9>&- 8>&-
  # Our own crates must rebuild from this worktree's sources: cargo names
  # them by their path in the workspace and judges them fresh by file times,
  # so a copied build could pass for this checkout's. Both manifests, so a
  # crate only the main checkout has goes too.
  local manifest
  for manifest in "$dir/Cargo.toml" "$root/Cargo.toml"; do
    timeout 15 cargo clean -q --offline --locked --workspace \
      --manifest-path "$manifest" --target-dir "$tmp" >&2 || return 1
  done
  mv -T "$tmp" "$dir/target"
}
seed || echo "worktree-create: not seeding" >&2
exec 9>&- 8>&-
rm -rf "$dir/target.seed"

echo "$dir"
