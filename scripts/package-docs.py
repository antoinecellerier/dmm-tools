#!/usr/bin/env python3
"""Assemble the docs that ship in the release archives.

Writes the user docs to OUT_DIR as Markdown and as HTML rendered by pandoc,
next to LICENSE and the images they reference, in the repository's layout so
their relative links keep working. A relative link to anything else (design
notes, the backlog, research) points at GitHub at the commit being built. A
relative link to a missing file fails the run. Each HTML page ends with a link
to the repository and to that commit.

Usage: scripts/package-docs.py OUT_DIR    (needs pandoc on PATH)
"""

import re
import shutil
import subprocess
import sys
from pathlib import Path, PurePosixPath

REPO_URL = "https://github.com/antoinecellerier/dmm-tools"

# The user-facing docs: the `paths:` of .claude/rules/docs-user-facing.md, plus
# the changelog. A new user doc goes in both places.
DOCS = [
    "README.md",
    "CHANGELOG.md",
    "CONTRIBUTING.md",
    "docs/setup.md",
    "docs/cli-reference.md",
    "docs/gui-reference.md",
    "docs/supported-devices.md",
]
SHIPPED = set(DOCS) | {"LICENSE"}
IMAGES = {".png", ".svg", ".jpg", ".jpeg", ".gif"}
CODE_BACKGROUND = "#f6f8fa"  # GitHub's
# Added to pandoc's default page style. Its template makes language-tagged
# (highlighted) code blocks transparent.
STYLE = f"""<style>
pre.sourceCode {{ background-color: {CODE_BACKGROUND}; }}
footer {{ margin-top: 3em; padding-top: 1em; border-top: 1px solid #d1d9e0;
  font-size: 85%; color: #59636e; }}
</style>"""

ROOT = Path(__file__).resolve().parent.parent
# Inline link or image target, up to an optional title or the closing paren.
LINK = re.compile(r"\]\(([^)\s]+)")
SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")


def rewrite(doc, text, out, sha, html):
    """Return doc's text with each relative link retargeted for the archive."""

    def target(match):
        link = match.group(1)
        if SCHEME.match(link) or link.startswith("#"):
            return match.group(0)
        path, hash_, frag = link.partition("#")
        rel = PurePosixPath(doc).parent / path
        resolved = (ROOT / rel).resolve()
        if not resolved.exists():
            sys.exit(f"{doc}: link to {link}, which does not exist")
        rel = resolved.relative_to(ROOT).as_posix()
        if rel in SHIPPED:
            if html and rel.endswith(".md"):
                path = path[: -len(".md")] + ".html"
        elif resolved.suffix.lower() in IMAGES:
            (out / rel).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(resolved, out / rel)
        else:
            kind = "tree" if resolved.is_dir() else "blob"
            path = f"{REPO_URL}/{kind}/{sha}/{rel}"
        return f"]({path}{hash_}{frag}"

    lines, fenced = [], False
    for line in text.splitlines(keepends=True):
        if line.lstrip().startswith("```"):
            fenced = not fenced
        lines.append(line if fenced else LINK.sub(target, line))
    return "".join(lines)


def git(*args):
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout.strip()


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__.strip())
    out = Path(sys.argv[1])
    sha, short = git("rev-parse", "HEAD"), git("rev-parse", "--short", "HEAD")
    footer = (
        f'<footer><a href="{REPO_URL}">dmm-tools on GitHub</a> · '
        f'built from <a href="{REPO_URL}/tree/{sha}">{short}</a></footer>'
    )
    out.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / "LICENSE", out / "LICENSE")
    for doc in DOCS:
        text = (ROOT / doc).read_text(encoding="utf-8")
        md, html = out / doc, (out / doc).with_suffix(".html")
        md.parent.mkdir(parents=True, exist_ok=True)
        md.write_text(rewrite(doc, text, out, sha, html=False), encoding="utf-8")
        title = next((l[2:].strip() for l in text.splitlines() if l.startswith("# ")), doc)
        subprocess.run(
            ["pandoc", "-f", "gfm", "-t", "html5", "-s",
             "--metadata", f"pagetitle={title}", "-V", "maxwidth=60em",
             "-V", "mainfont=system-ui, sans-serif", "-V", "linkcolor=#0969da",
             "-V", f"monobackgroundcolor={CODE_BACKGROUND}",
             "-V", f"header-includes={STYLE}", "-V", f"include-after={footer}",
             "-o", str(html)],
            input=rewrite(doc, text, out, sha, html=True),
            check=True, text=True, encoding="utf-8",
        )


if __name__ == "__main__":
    main()
