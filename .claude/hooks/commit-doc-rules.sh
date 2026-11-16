#!/bin/sh
# PreToolUse hook on `git commit`: name the rule files that cover the
# Markdown files about to be committed.
#
# A path-scoped rule in .claude/rules/ loads only when a file it covers is
# read or edited with the file tools. A doc changed by a script, or by a
# subagent, reaches the commit with its rules never seen; this puts them in
# front of the assistant at the one point every change passes. It never
# blocks or fails: anything unexpected ends in silence and exit 0.
# settings.json runs it for `git commit` calls only.
#
# Markdown only: code is always read before it is changed, so its rules load.
#
# Of a rule's frontmatter it reads the form the rule files use, an indented
# `- "glob"` list under `paths:`, with `*` and `**` as the only wildcards.

exec 2>/dev/null
input=$(cat)
HOOK_INPUT=$input python3 -I - <<'PY' 2>/dev/null || exit 0
import json, os, re, subprocess, sys
from pathlib import Path

# The most docs listed by name; a commit touching more gets a count.
MAX_LISTED = 20
# A name is shown only if it is made of these: what reaches the assistant
# must read as a path, whatever an unpacked archive left in the tree.
PLAIN = re.compile(r"[\w./ +@-]+")


def checkout():
    """The checkout the command runs in: a worktree's own, not the session's."""
    try:
        cwd = json.loads(os.environ.get("HOOK_INPUT", ""))["cwd"]
    except (KeyError, TypeError, ValueError):
        cwd = os.environ.get("CLAUDE_PROJECT_DIR", ".")
    top = subprocess.run(
        ("git", "-C", cwd, "rev-parse", "--show-toplevel"),
        capture_output=True,
        timeout=4,
    ).stdout
    return Path(os.fsdecode(top).rstrip("\n"))


def changed_docs(top):
    """Markdown files added or modified, staged or not: `git add … && git
    commit` stages inside the command this hook runs ahead of. Read without
    taking the index lock, so a git process beside it is left alone."""
    out = subprocess.run(
        ("git", "-C", str(top), "--no-optional-locks", "status", "--porcelain")
        + ("-z", "--no-renames", "--untracked-files=all"),
        capture_output=True,
        timeout=8,
    ).stdout
    docs = []
    for entry in os.fsdecode(out).split("\0"):
        state, path = entry[:2], entry[3:]
        if path.endswith(".md") and "D" not in state and PLAIN.fullmatch(path):
            docs.append(path)
    return sorted(docs)


def patterns(rule):
    """The `paths:` globs in a rule file's frontmatter."""
    lines = rule.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "---":
        return []
    found, inside = [], False
    for line in lines[1:]:
        if line == "---":
            break
        if line.startswith("paths:"):
            inside = True
        elif inside and (m := re.match(r'\s+-\s+"?([^"]+)"?\s*$', line)):
            found.append(m.group(1))
        else:
            inside = False
    return found


def matcher(glob):
    """`**` crosses directories, `*` stays inside one."""
    parts = re.split(r"(\*\*|\*)", glob)
    return re.compile(
        "".join(
            ".*" if p == "**" else "[^/]*" if p == "*" else re.escape(p)
            for p in parts
        )
    )


def main():
    top = checkout()
    docs = changed_docs(top)
    if not docs:
        return
    rules = []
    for rule in sorted((top / ".claude/rules").glob("*.md")):
        if rule.is_file() and PLAIN.fullmatch(rule.name):
            rules.append((rule.name, [matcher(g) for g in patterns(rule)]))
    covered = {}
    for doc in docs:
        found = [
            f".claude/rules/{name}"
            for name, globs in rules
            if any(g.fullmatch(doc) for g in globs)
        ]
        if found:
            covered[doc] = found
    if not covered:
        return

    # The ask first: a long list is cut from the end.
    lines = [
        "This commit changes docs that have rule files. Before committing: "
        "Read any of these rule files not yet read in this session, then "
        "judge each changed section whole, as it will stand after the commit "
        "and against the sections around it, not only the lines this commit "
        "adds (`git diff origin/main -- <file>` shows what the unpushed work "
        "has added to it so far). Cut or move what the rules exclude first."
    ]
    listed = list(covered.items())[:MAX_LISTED]
    lines += [f"- {doc}: {', '.join(found)}" for doc, found in listed]
    if len(covered) > MAX_LISTED:
        lines.append(f"- and {len(covered) - MAX_LISTED} more changed docs")
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "additionalContext": "\n".join(lines),
                }
            }
        )
    )


try:
    main()
except Exception:
    pass
PY
exit 0
