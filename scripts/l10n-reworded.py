#!/usr/bin/env python3
"""List the English messages reworded under the same ID since a revision.

    python3 scripts/l10n-reworded.py [<rev>]

<rev> defaults to the latest v* tag. A translation is matched to its English
by ID alone, so the tests count a reworded message as translated; this list
is what to read at the release step, to find the rewordings whose meaning
changed. Messages and attributes are matched by ID across i18n/en-US/, so a
message moved between files is not listed, nor one added or removed.
"""

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
ENGLISH = "i18n/en-US"

ENTRY = re.compile(r"^(-?[A-Za-z][\w-]*)\s*=\s?(.*)$")
ATTRIBUTE = re.compile(r"^\s+\.([A-Za-z][\w-]*)\s*=\s?(.*)$")


def git(*args):
    return subprocess.run(
        ["git", *args], cwd=REPO, capture_output=True, text=True, encoding="utf-8"
    )


def parse(source):
    """{id or id.attribute: text} of an .ftl source, whitespace collapsed
    and comments left out, as a comment is no change of wording."""
    found = {}
    message = key = None
    for line in source.splitlines():
        if line.startswith("#"):
            continue
        entry, attribute = ENTRY.match(line), ATTRIBUTE.match(line)
        if entry:
            message = key = entry.group(1)
            found[key] = entry.group(2)
        elif attribute and message:
            key = f"{message}.{attribute.group(1)}"
            found[key] = attribute.group(2)
        elif key and line.strip():
            found[key] += " " + line.strip()
    return {k: " ".join(v.split()) for k, v in found.items()}


def merge(sources):
    """{id: set of its texts} over several files: the CLI and the GUI may
    each define an ID."""
    merged = {}
    for source in sources:
        for key, text in parse(source).items():
            merged.setdefault(key, set()).add(text)
    return merged


def at_revision(rev):
    names = git("ls-tree", "-r", "--name-only", rev, "--", ENGLISH).stdout.split()
    return merge(
        git("show", f"{rev}:{name}").stdout for name in names if name.endswith(".ftl")
    )


def main():
    if len(sys.argv) > 2:
        sys.exit(__doc__)
    if len(sys.argv) == 2:
        rev = sys.argv[1]
    else:
        rev = git("describe", "--tags", "--abbrev=0", "--match", "v*").stdout.strip()
    if not rev or git("rev-parse", "--verify", "-q", f"{rev}^{{commit}}").returncode:
        sys.exit(f"no such revision: {rev or '(no v* tag)'}")

    old = at_revision(rev)
    if not old:
        print(f"{rev} has no {ENGLISH}: nothing to compare")
        return
    new = merge(p.read_text(encoding="utf-8") for p in (REPO / ENGLISH).rglob("*.ftl"))
    reworded = sorted(k for k in old.keys() & new.keys() if old[k] != new[k])
    for key in reworded:
        print(key)
        for text in sorted(old[key] - new[key]):
            print(f"  - {text}")
        for text in sorted(new[key] - old[key]):
            print(f"  + {text}")
    print(f"{len(reworded)} reworded since {rev}")


if __name__ == "__main__":
    main()
