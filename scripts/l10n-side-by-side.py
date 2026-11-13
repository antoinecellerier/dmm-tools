#!/usr/bin/env python3
"""Write one HTML page that sets a language's translation beside its English.

    python3 scripts/l10n-side-by-side.py pt-BR pt-BR.html

Every file under i18n/en-US/, the GUI's first, under its group headings: each
message and each attribute on a row of its own, with the English's comment
under it. Messages the language lacks are marked, the optional ones (the
specification text in lib.ftl and capture's text, which show in English
until translated) apart from the rest. Text is shown as the file writes it,
placeables and selectors included. For reading a language through; edits go
in the files themselves.
"""

import csv
import html
import re
import subprocess
import sys
import urllib.parse
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
I18N = REPO / "i18n"
ENGLISH = "en-US"

ENTRY = re.compile(r"^(-?[A-Za-z][\w-]*)\s*=\s?(.*)$")
ATTRIBUTE = re.compile(r"^\s+\.([A-Za-z][\w-]*)\s*=\s?(.*)$")


def parse(path):
    """The entries of an .ftl file, in order: ("group", text) for a `##`
    comment, or ("message", id, value, comment, [(attribute, value)]).
    Values keep their Fluent source; continuation lines are dedented."""
    entries, comment, group = [], [], []
    current = target = None

    def close_group():
        if group:
            entries.append(("group", " ".join(group)))
            group.clear()

    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("###"):
            current = None
        elif line.startswith("##"):
            current = None
            group.append(line[2:].strip())
        elif line.startswith("#"):
            close_group()
            current = None
            comment.append(line[1:].strip())
        elif m := ENTRY.match(line):
            close_group()
            current = ["message", m.group(1), [m.group(2)], " ".join(comment), []]
            target = current[2]
            comment = []
            entries.append(current)
        elif current and line[:1] in (" ", "\t"):
            if m := ATTRIBUTE.match(line):
                current[4].append((m.group(1), [m.group(2)]))
                target = current[4][-1][1]
            else:
                target.append(line)
        elif not line.strip():
            close_group()
            comment = []
            if current:
                target.append("")
    close_group()
    out = []
    for entry in entries:
        if entry[0] == "group":
            out.append(entry)
            continue
        _, id_, value, note, attributes = entry
        out.append(
            ("message", id_, text(value), note, [(a, text(v)) for a, v in attributes])
        )
    return out


def lines_of(path):
    """The line each message (`id`) and attribute (`id.name`) of `path`
    starts on, counting from 1."""
    out, current = {}, None
    if not path.exists():
        return out
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if m := ENTRY.match(line):
            current = m.group(1)
            out[current] = number
        elif current and (m := ATTRIBUTE.match(line)):
            out[f"{current}.{m.group(1)}"] = number
        elif line.startswith("#"):
            current = None
    return out


def open_link(path, line, icon, title):
    """An icon in a cell's corner that opens `path` at `line` in VS Code
    (its `vscode://file` handler), which works on the machine that wrote
    the page."""
    target = urllib.parse.quote(path.resolve().as_posix().lstrip("/"), safe="/:")
    return (
        f'<a class="open" href="vscode://file/{target}:{line}" '
        f'title="{html.escape(title)}" aria-label="{html.escape(title)}">{icon}</a>'
    )


def text(lines):
    """A value's lines as one block: the first as written, the rest
    dedented together, trailing blank lines dropped."""
    while lines and not lines[-1].strip():
        lines = lines[:-1]
    if not lines:
        return ""
    first, rest = lines[0], lines[1:]
    indents = [len(line) - len(line.lstrip()) for line in rest if line.strip()]
    cut = min(indents, default=0)
    body = [line[cut:] for line in rest]
    return "\n".join(([first] if first else []) + body)


def optional(path, key):
    """Whether `key` of `path` may be left out of a shipped language, as the
    translations check (crates/dmm-shared/src/l10n/translations.rs) says."""
    if path == "cli/capture.ftl":
        return True
    return path == "lib.ftl" and key.split("-", 1)[0] in ("note", "input", "band")


def glossary(path):
    """The rows of a language's glossary.tsv (english, translation, part of
    speech, context, source, notes): the term, its translation, and the
    notes with the part of speech and context dimmed. The source column is
    for whoever drafts the glossary and is left out."""
    if not path.exists():
        return []
    with open(path, encoding="utf-8") as f:
        lines = list(csv.reader(f, delimiter="\t"))
    rows = []
    for cells in lines[1:]:
        cells = (cells + [""] * 6)[:6]
        english, translation, pos, context, _, notes = (c.strip() for c in cells)
        if not english:
            continue
        about = " · ".join(p for p in (pos, context) if p and p != "—")
        about_html = f'<div class="note">{html.escape(about)}</div>' if about else ""
        rows.append(
            f'<tr><td class="en">{html.escape(english)}</td>'
            f'<td class="tr">{html.escape(translation)}</td>'
            f'<td class="en">{html.escape(notes)}{about_html}</td></tr>'
        )
    return rows


def reading_order(path):
    """What most users meet first: the GUI, the help both apps print, the
    meter's labels, dmm-cli, its --help, and capture's optional text last."""
    if path.startswith("gui/"):
        rank = 0
    elif path == "shared.ftl":
        rank = 1
    elif path.startswith("lib"):
        rank = 2
    elif path == "cli/capture.ftl":
        rank = 5
    elif path.startswith("cli/"):
        rank = 3
    else:
        rank = 4
    return (rank, path)


def status(required_missing, optional_untranslated, text_):
    """`text_` coloured, and marked, by the worst gap it counts: a required
    message missing (✖, red), only optional ones untranslated (○, orange),
    or none (✔, green)."""
    if required_missing:
        kind, mark = "short", "✖"
    elif optional_untranslated:
        kind, mark = "gaps", "○"
    else:
        kind, mark = "ok", "✔"
    return f'<span class="status {kind}">{mark} {html.escape(text_)}</span>'


# A placeable that is a variable, a message or term reference, or a quoted
# literal: `{ $high }`, `{ -app-name }`, `{ "  " }`. A selector's braces
# hold `->` and its variants, and stay as text.
PLACEABLE = re.compile(r'\{\s*(?:\$[\w-]+|-?[A-Za-z][\w-]*(?:\.[\w-]+)?|"(?:[^"\\]|\\.)*")\s*\}')


def show(value):
    """`value` as HTML, placeables dimmed."""
    escaped = html.escape(value, quote=False)
    return PLACEABLE.sub(lambda m: f'<span class="ph">{m.group(0)}</span>', escaped)


def prose_words(value):
    """The words of prose in a message: what is left once placeables,
    selector syntax, variant keys, `code`, --options and addresses are
    taken out."""
    value = re.sub(r"\{[^{}]*->", " ", value)
    value = re.sub(r"\*?\[[^\]]*\]", " ", value)
    value = re.sub(r"\{[^{}]*\}", " ", value)
    value = re.sub(r"`[^`]*`|--[a-z][\w-]*|https?://\S+", " ", value)
    return re.findall(r"[A-Za-z]*[a-z]{2,}[A-Za-z]*", value)


def looks_untranslated(english, translation, note):
    """Whether a translation identical to its English is most likely one
    nobody translated: three words of prose or more rarely read the same in
    another language, where a label ("AC V", "HOLD", "Bluetooth") often
    does. A message whose comment says it needs no translation is left
    alone."""
    return (
        translation == english
        and len(prose_words(english)) >= 3
        and "no need to translate" not in note
    )


def page(lang):
    english_dir, lang_dir = I18N / ENGLISH, I18N / lang
    paths = sorted(
        (p.relative_to(english_dir).as_posix() for p in english_dir.rglob("*.ftl")),
        key=reading_order,
    )
    sections, index = [], []
    # keys, translated, required missing, optional, optional untranslated
    totals = [0] * 7
    for path in paths:
        english = parse(english_dir / path)
        theirs_path = lang_dir / path
        theirs = {}
        if theirs_path.exists():
            for entry in parse(theirs_path):
                if entry[0] == "message":
                    _, id_, value, _, attributes = entry
                    if value:
                        theirs[id_] = value
                    for name, attribute in attributes:
                        theirs[f"{id_}.{name}"] = attribute
        # keys, translated, required missing, optional, optional untranslated,
        # same as English, only in the translation
        rows, counts, seen = [], [0] * 7, set()
        english_lines, their_lines = lines_of(english_dir / path), lines_of(theirs_path)
        # Where a missing message belongs in the translation: after the last
        # one before it, in the English order, that the translation has.
        insert_at = [1]

        def row(key, english_text, note):
            counts[0] += 1
            is_optional = optional(path, key)
            counts[3] += is_optional
            translation = theirs.get(key)
            classes = ["opt"] if is_optional else []
            if key in their_lines:
                insert_at[0] = their_lines[key]
                edit = open_link(theirs_path, their_lines[key], "✎", f"Edit the {lang} in VS Code")
            elif theirs_path.exists():
                edit = open_link(theirs_path, insert_at[0], "✎", f"Add it to the {lang} file in VS Code, where it belongs")
            else:
                edit = ""
            seen.add(key)
            if translation is not None and looks_untranslated(english_text, translation, note):
                if is_optional:
                    counts[4] += 1
                    classes.append("optgap")
                    tag = '<div class="tag optional">same as English, looks untranslated</div>'
                else:
                    counts[2] += 1
                    classes.append("miss")
                    tag = '<div class="tag missing">same as English, looks untranslated</div>'
                cell = f'<td class="tr">{edit}{show(translation)}{tag}</td>'
            elif translation is not None:
                counts[1] += 1
                same = ""
                if translation == english_text:
                    counts[5] += 1
                    classes.append("same")
                    same = '<div class="tag same">same as English</div>'
                cell = f'<td class="tr">{edit}{show(translation)}{same}</td>'
            elif is_optional:
                counts[4] += 1
                classes.append("optgap")
                cell = f'<td class="tr gap">{edit}<span class="tag optional">not translated, shows in English</span></td>'
            else:
                counts[2] += 1
                classes.append("miss")
                cell = f'<td class="tr gap">{edit}<span class="tag missing">missing</span></td>'
            note_html = f'<div class="note">{html.escape(note)}</div>' if note else ""
            label = '<div class="tag optional">optional</div>' if is_optional else ""
            source = (
                open_link(english_dir / path, english_lines[key], "↗", "Open the English in VS Code")
                if key in english_lines
                else ""
            )
            class_attr = f' class="{" ".join(classes)}"' if classes else ""
            rows.append(
                f'<tr id="{html.escape(path)}:{html.escape(key)}"{class_attr}>'
                f'<td class="id">{html.escape(key)}{label}</td>'
                f'<td class="en">{source}{show(english_text)}{note_html}</td>{cell}</tr>'
            )

        for entry in english:
            if entry[0] == "group":
                rows.append(f'<tr class="group"><td colspan="3">{html.escape(entry[1])}</td></tr>')
                continue
            _, id_, value, note, attributes = entry
            if value:
                row(id_, value, note)
            for i, (name, attribute) in enumerate(attributes):
                row(f"{id_}.{name}", attribute, "" if value or i else note)
        # What only the translation has: left behind when the English was
        # removed or renamed. The translations check fails on it too.
        for key, translation in theirs.items():
            if key in seen:
                continue
            counts[6] += 1
            edit = open_link(theirs_path, their_lines.get(key, 1), "✎", f"Edit the {lang} in VS Code")
            rows.append(
                f'<tr class="miss"><td class="id">{html.escape(key)}</td>'
                f'<td class="en gap"><span class="tag missing">not in English: delete it, or match the English ID</span></td>'
                f'<td class="tr">{edit}{show(translation)}</td></tr>'
            )
        for i in range(len(totals)):
            totals[i] += counts[i]
        anchor = html.escape(path)
        detail = []
        if counts[2]:
            detail.append(f"{counts[2]} missing")
        if counts[6]:
            detail.append(f"{counts[6]} not in English")
        if counts[0] and counts[3] == counts[0]:
            detail.append("all optional")
            if counts[4]:
                detail.append(f"{counts[4]} not translated")
        elif counts[3]:
            detail.append(f"{counts[3]} optional")
            if counts[4]:
                detail.append(f"{counts[4]} of them not translated")
        text_ = ", ".join([f"{counts[1]}/{counts[0]}"] + detail)
        if counts[5]:
            text_ += f", {counts[5]} same as English"
        index.append(f'<li><a href="#{anchor}">{anchor}</a> {status(counts[2] + counts[6], counts[4], text_)}</li>')
        sections.append(
            f'<section><h2 id="{anchor}">{anchor}</h2>'
            f'<table><thead><tr><th>ID</th><th>{ENGLISH}</th><th>{html.escape(lang)}</th></tr></thead>'
            f'<tbody>{"".join(rows)}</tbody></table></section>'
        )
    terms = glossary(lang_dir / "glossary.tsv")
    if terms:
        index.append(f'<li><a href="#glossary">Glossary</a> <span class="count">{len(terms)} terms</span></li>')
        sections.append(
            '<section><h2 id="glossary">Glossary</h2>'
            f'<table><thead><tr><th>{ENGLISH}</th><th>{html.escape(lang)}</th><th>Notes</th></tr></thead>'
            f'<tbody>{"".join(terms)}</tbody></table></section>'
        )
    try:
        commit = subprocess.run(
            ["git", "-C", str(REPO), "rev-parse", "--short", "HEAD"],
            capture_output=True, text=True, check=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        commit = "unknown"
    summary = f"{totals[1]} of {totals[0]} messages and attributes translated"
    if totals[2]:
        summary += f", {totals[2]} required missing"
    if totals[4]:
        summary += f", {totals[4]} optional not translated"
    if totals[6]:
        summary += f", {totals[6]} not in English"
    if totals[5]:
        summary += f", {totals[5]} same as English"
    summary = status(totals[2] + totals[6], totals[4], summary)
    return f"""<!doctype html>
<html lang="{html.escape(lang)}"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(lang)} beside {ENGLISH}</title>
<style>
:root {{ --bg: #fff; --fg: #1d1d1f; --muted: #6b6b70; --line: #e3e3e6; --head: #f4f4f6;
  --ph: #6b4fa0; --missing: #b3261e; --missing-bg: #fdecea; --optional: #8a6100; --link: #0b57d0; --ok: #1e7d32; --same-bg: #eef2f8; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg: #17171a; --fg: #e8e8ea; --muted: #9a9aa2;
  --line: #2e2e33; --head: #222226; --ph: #b9a2e8; --missing: #ff8a80; --missing-bg: #3a1d1b; --optional: #e6b84d; --link: #8ab4f8; --ok: #81c995; --same-bg: #1f2530; }} }}
body {{ background: var(--bg); color: var(--fg); font: 15px/1.45 system-ui, sans-serif; margin: 16px; }}
h1 {{ font-size: 1.4em; margin: 0 0 4px; }} h2 {{ font-size: 1.1em; margin: 32px 0 8px; }}
.meta, .count, .note {{ color: var(--muted); }} .meta {{ margin: 0 0 16px; }}
ul.index {{ columns: 3 14em; padding-left: 1.2em; }}
table {{ border-collapse: collapse; width: 100%; table-layout: fixed; }}
th {{ text-align: left; background: var(--head); }}
th, td {{ border-bottom: 1px solid var(--line); padding: 6px 8px; vertical-align: top; }}
th:first-child {{ width: 20%; }}
td.en, td.tr {{ white-space: pre-wrap; overflow-wrap: anywhere; }}
td.id {{ font: 12px/1.4 ui-monospace, monospace; color: var(--muted); overflow-wrap: anywhere; }}
tr.group td {{ background: var(--head); color: var(--muted); font-style: italic; }}
.note {{ font-size: 12px; margin-top: 4px; white-space: normal; }}
.ph {{ color: var(--ph); }}
.tag {{ font-size: 12px; font-weight: 600; }} .missing {{ color: var(--missing); }} .optional {{ color: var(--optional); }}
td.id .tag {{ font-family: system-ui, sans-serif; margin-top: 2px; }}
tr.opt td:first-child {{ box-shadow: inset 3px 0 0 var(--optional); }}
tr.miss td {{ background: var(--missing-bg); }}
tr.same td {{ background: var(--same-bg); }}
.filters button {{ font: inherit; font-size: 13px; color: var(--fg); background: var(--head); border: 1px solid var(--line); border-radius: 4px; padding: 2px 8px; margin: 0 4px 4px 0; cursor: pointer; }}
.filters button[aria-pressed="true"] {{ border-color: var(--link); color: var(--link); }}
[hidden] {{ display: none !important; }}
.tag.same {{ color: var(--muted); font-weight: 500; margin-top: 2px; }}
a {{ color: var(--link); }}
td.en, td.tr {{ position: relative; padding-right: 26px; }}
a.open {{ position: absolute; top: 5px; right: 6px; text-decoration: none; opacity: .45; font-size: 14px; }}
a.open:hover, a.open:focus {{ opacity: 1; }}
.status.ok {{ color: var(--ok); }} .status.gaps {{ color: var(--optional); }} .status.short {{ color: var(--missing); font-weight: 600; }}
@media (max-width: 720px) {{
  thead {{ display: none; }} table, tbody, tr, td {{ display: block; width: auto; }}
  tr {{ border-bottom: 1px solid var(--line); padding: 6px 0; }} td {{ border: 0; padding: 2px 0; }}
}}
</style></head><body>
<h1>{html.escape(lang)} beside {ENGLISH}</h1>
<p class="meta">{summary}. From commit {html.escape(commit)}, by scripts/l10n-side-by-side.py.</p>
<p class="meta">Messages marked <span class="tag optional">optional</span>, with a coloured edge, may stay untranslated: they show in English. Every other message must be translated before the language ships. <span class="tag same">Same as English</span> marks a translation identical to its English, often right for the meter's own labels; one with three words of prose or more is marked <span class="tag missing">looks untranslated</span> and counted as missing.</p>
<p class="filters" hidden>Show:
<button type="button" data-f="all" aria-pressed="true">All</button>
<button type="button" data-f="miss">Missing ({totals[2] + totals[6]})</button>
<button type="button" data-f="optgap">Optional untranslated ({totals[4]})</button>
<button type="button" data-f="same">Same as English ({totals[5]})</button></p>
<ul class="index">{"".join(index)}</ul>
{"".join(sections)}
<script>
// Show only the rows in one state, and only the files that have some.
const bar = document.querySelector(".filters");
bar.hidden = false;
const buttons = bar.querySelectorAll("button");
for (const button of buttons) {{
  button.addEventListener("click", () => {{
    const f = button.dataset.f;
    for (const section of document.querySelectorAll("section")) {{
      let any = false;
      for (const tr of section.querySelectorAll("tbody tr")) {{
        const show = f === "all" || tr.classList.contains(f);
        tr.hidden = !show;
        any ||= show;
      }}
      section.hidden = !any;
    }}
    for (const b of buttons) b.setAttribute("aria-pressed", String(b === button));
  }});
}}
// A filter given in the address, such as ?show=same, applies on load.
const initial = new URLSearchParams(location.search).get("show");
for (const button of buttons) if (button.dataset.f === initial) button.click();
</script>
</body></html>
"""


def main():
    if len(sys.argv) not in (2, 3) or sys.argv[1] in ("-h", "--help"):
        sys.exit(__doc__.strip())
    lang = sys.argv[1]
    if lang == ENGLISH or not (I18N / lang).is_dir():
        known = sorted(p.name for p in I18N.iterdir() if p.is_dir() and p.name != ENGLISH)
        sys.exit(f"no translation called {lang!r} under i18n/; there are: {', '.join(known)}")
    # Written as UTF-8 by the script, not through a shell's `>`: Windows
    # would encode redirected output in its code page, which lacks ✔ or «.
    if len(sys.argv) == 3:
        Path(sys.argv[2]).write_text(page(lang), encoding="utf-8")
    else:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stdout.write(page(lang))


if __name__ == "__main__":
    main()
