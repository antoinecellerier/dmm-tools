#!/usr/bin/env python3
"""Turn a native-sweep.js run into files, a review page and edits.

  native-sweep.py extract <journal.jsonl> <out-dir> [--prior <journal.jsonl>]...
  native-sweep.py page <out-dir> <lang> <page.html>
  native-sweep.py apply <out-dir> <lang>

extract writes <out-dir>/<lang>/{found,confirmed,rejected,dropped}.json and
prints the yield. Dropped: a finding whose text is not in its message, one that
loses a placeable, code or option, one that changes nothing, and one that
restores text a --prior review replaced.
"""
import argparse
import collections
import difflib
import html
import json
import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[4]
SPACES = "   "


def squash(s):
    return " ".join(s.split())


def read_journal(path):
    labels, results = {}, {}
    for line in pathlib.Path(path).read_text(encoding="utf-8").splitlines():
        entry = json.loads(line)
        if "label" in entry:
            labels[entry["key"]] = entry["label"]
        elif "result" in entry:
            result = entry["result"]
            if isinstance(result, str):
                try:
                    result = json.loads(result)
                except ValueError:
                    continue
            results[labels.get(entry["key"], "?")] = result
    return results


def sweep_findings(results):
    """Every finding of a sweep's journal, with the id the workflow gave it."""
    found = []
    for label, result in results.items():
        if label.startswith("find:") and isinstance(result, dict):
            _, lang, group = label.split(":")
            for i, f in enumerate(result["findings"]):
                found.append({**f, "id": f"{label}#{i}", "lang": lang, "group": group})
    return found


def verdicts_by_id(results):
    return {
        v["id"]: v
        for label, result in results.items()
        if label.startswith("confirm:") and isinstance(result, dict)
        for v in result["verdicts"]
    }


def prior_changes(paths):
    """(file, where) -> texts earlier reviews replaced."""
    prior = collections.defaultdict(list)
    for path in paths:
        results = read_journal(path)
        verdicts = verdicts_by_id(results)
        if verdicts:
            kept = {}
            for f in sweep_findings(results):
                kept.setdefault((f["current"], f["suggestion"]), []).append(f)
            ruled = {
                key: any(verdicts.get(f["id"], {}).get("verdict") in ("confirm", "revise") for f in group)
                for key, group in kept.items()
            }
            for key, group in kept.items():
                for f in group if ruled[key] else []:
                    prior[(f["file"], f["where"])].append(squash(f["current"]))
        else:
            for result in results.values():
                if isinstance(result, dict):
                    for f in result.get("findings", []):
                        prior[(f["file"], f["where"])].append(squash(f["current"]))
    return prior


def message_span(text, where):
    """Character range of a Fluent message, narrowed to its attribute and variant if named."""
    m = re.match(r"-?[\w-]+", where)
    if not m:
        return None
    lines = text.splitlines(keepends=True)
    start = next((i for i, l in enumerate(lines) if re.match(re.escape(m.group()) + r"\s*=", l)), None)
    if start is None:
        return None
    end = start + 1
    while end < len(lines) and (not lines[end].strip() or lines[end][0].isspace()):
        end += 1
    attr = re.search(r"\.([\w-]+)", where)
    if attr:
        at = next((i for i in range(start, end) if re.match(r"\s+\." + re.escape(attr.group(1)) + r"\s*=", lines[i])), None)
        if at is not None:
            start = at
            end = next((i for i in range(at + 1, end) if re.match(r"\s+\.[\w-]+\s*=", lines[i])), end)
    variant = re.search(r"#([\w-]+)", where)
    if variant:
        at = next((i for i in range(start, end) if re.match(r"\s*\*?\[" + re.escape(variant.group(1)) + r"\]", lines[i])), None)
        if at is not None:
            start = at
            end = next((i for i in range(at + 1, end) if re.match(r"\s*(\*?\[[\w-]+\]|\})", lines[i])), end)
    offset = sum(map(len, lines[:start]))
    return offset, offset + sum(map(len, lines[start:end]))


def matches(text, span, current):
    pattern = r"\s+".join(re.escape(token) for token in current.split())
    return [m.span() for m in re.finditer(pattern, text[span[0]:span[1]])] if pattern else []


def locate(text, file, where, current):
    """The range of current in its message or README line, or None. A README line that doesn't hold it falls back on the file."""
    if file.endswith(".md"):
        whole = (0, len(text))
        line = re.match(r"\d+", where)
        if line:
            lines = text.splitlines(keepends=True)
            n = int(line.group()) - 1
            if 0 <= n < len(lines):
                at = sum(map(len, lines[:n]))
                span = (at, at + len(lines[n]))
                if matches(text, span, current):
                    return span
        return whole
    return message_span(text, where)


def protected(s):
    tokens = [squash(t) for t in re.findall(r"\{[^{}]*\}", s)]
    tokens += re.findall(r"`[^`]*`", s)
    tokens += re.findall(r"--[a-z][\w-]*", s)
    return collections.Counter(tokens)


def read_file(cache, file):
    if file not in cache:
        path = REPO / file
        cache[file] = path.read_text(encoding="utf-8") if path.is_file() else None
    return cache[file]


def restores_prior(prior, location, current, final):
    final, current = squash(final), squash(current)
    for old in prior.get(location, []):
        if old in final and old not in current:
            return True
        if final in old and final not in current:
            return True
    return False


def extract(args):
    results = read_journal(args.journal)
    verdicts = verdicts_by_id(results)
    prior = prior_changes(args.prior)
    out = pathlib.Path(args.out_dir)
    groups = collections.defaultdict(list)
    for f in sweep_findings(results):
        groups[(f["lang"], f["current"], f["suggestion"])].append(f)
    cache = {}
    per_lang = collections.defaultdict(lambda: collections.defaultdict(list))
    yield_rows = collections.Counter()
    for (lang, current, suggestion), members in groups.items():
        verdict = next((verdicts[f["id"]] for f in members if f["id"] in verdicts), None)
        first = members[0]
        item = {
            "id": first["id"],
            "group": first["group"],
            "locations": [{"file": f["file"], "where": f["where"]} for f in members],
            "english": first["english"],
            "current": current,
            "suggestion": suggestion,
            "reader_why": first["why"],
        }
        for f in members:
            yield_rows[(lang, f["group"], "found")] += 1
        per_lang[lang]["found"].append({**item, "verdict": verdict})
        tally = (lang, first["group"])
        if verdict is None:
            per_lang[lang]["dropped"].append({**item, "reason": "no verdict"})
            yield_rows[(*tally, "dropped")] += 1
            continue
        if verdict["verdict"] == "reject":
            per_lang[lang]["rejected"].append({**item, "why": verdict["why"]})
            yield_rows[(*tally, "rejected")] += 1
            continue
        final = verdict.get("final_text") or suggestion if verdict["verdict"] == "revise" else suggestion
        placed = []
        for location in item["locations"]:
            text = read_file(cache, location["file"])
            span = locate(text, location["file"], location["where"], current) if text is not None else None
            if span and matches(text, span, current):
                placed.append(location)
        reason = None
        if not placed:
            reason = "text not found in its message"
        elif squash(final) == squash(current):
            reason = "changes nothing"
        elif protected(final) != protected(current):
            reason = "placeable, code or option differs"
        elif any(restores_prior(prior, (loc["file"], loc["where"]), current, final) for loc in placed):
            reason = "restores text an earlier review replaced"
        if reason:
            per_lang[lang]["dropped"].append({**item, "final": final, "reason": reason})
            yield_rows[(*tally, "dropped")] += 1
            continue
        missing = len(item["locations"]) - len(placed)
        item = {**item, "locations": placed, "final": final, "verdict": verdict["verdict"], "severity": verdict["severity"], "note": verdict["why"]}
        if missing:
            item["unlocated"] = missing
        per_lang[lang]["confirmed"].append(item)
        yield_rows[(*tally, "revised" if verdict["verdict"] == "revise" else "confirmed")] += 1
    for lang, buckets in per_lang.items():
        (out / lang).mkdir(parents=True, exist_ok=True)
        for name in ("found", "confirmed", "rejected", "dropped"):
            payload = json.dumps(buckets[name], ensure_ascii=False, indent=1)
            (out / lang / f"{name}.json").write_text(payload + "\n", encoding="utf-8")
    columns = ("found", "confirmed", "revised", "rejected", "dropped")
    print("lang\tgroup\t" + "\t".join(columns))
    for lang, group in sorted({key[:2] for key in yield_rows}):
        print(f"{lang}\t{group}\t" + "\t".join(str(yield_rows[(lang, group, c)]) for c in columns))
    for lang, buckets in sorted(per_lang.items()):
        reasons = collections.Counter(d["reason"] for d in buckets["dropped"])
        print(f"{lang}: dropped {dict(reasons)}")
    if not verdicts:
        print("warning: the journal holds no verdicts")


def marked(old, new):
    """HTML of new with the words that differ from old highlighted."""
    a, b = re.findall(r"\s+|\S+", old), re.findall(r"\s+|\S+", new)
    parts = []
    for op, _, _, j1, j2 in difflib.SequenceMatcher(None, a, b, autojunk=False).get_opcodes():
        chunk = html.escape("".join(b[j1:j2]))
        parts.append(f"<mark>{chunk}</mark>" if op != "equal" and chunk.strip() else chunk)
    return "".join(parts)


def page(args):
    items = json.loads((pathlib.Path(args.out_dir) / args.lang / "confirmed.json").read_text(encoding="utf-8"))
    rank = {"major": 0, "minor": 1}
    items.sort(key=lambda i: (rank.get(i["severity"], 2), i["locations"][0]["file"]))
    rows = []
    for n, item in enumerate(items, 1):
        where = "<br>".join(html.escape(f'{l["file"]} {l["where"]}') for l in item["locations"])
        rows.append(
            f'<tr id="r{n}"><td class="n">{n}<br><span class="sev {item["severity"]}">{item["severity"]}</span></td>'
            f'<td><div class="where">{where}</div><div class="en">{html.escape(item["english"])}</div></td>'
            f'<td>{html.escape(item["current"])}</td><td>{marked(item["current"], item["final"])}</td>'
            f'<td class="note">{html.escape(item["note"])}</td></tr>'
        )
    counts = collections.Counter(i["severity"] for i in items)
    glance = "".join(
        f'<li><a href="#r{n}">{n}</a> {marked(i["current"], i["final"])}</li>' for n, i in enumerate(items, 1) if i["severity"] == "major"
    )
    style = """
:root{color-scheme:light dark;--bg:#fff;--fg:#1a1a1a;--muted:#667;--line:#ddd;--mark:#ffe08a}
@media (prefers-color-scheme:dark){:root{--bg:#16181c;--fg:#e6e6e6;--muted:#99a;--line:#333;--mark:#6b5310}}
body{background:var(--bg);color:var(--fg);font:15px/1.45 system-ui,sans-serif;margin:16px}
table{border-collapse:collapse;width:100%}td,th{border-top:1px solid var(--line);padding:8px;vertical-align:top;text-align:left}
.where,.en,.note{color:var(--muted);font-size:13px}.where{font-family:monospace;word-break:break-all}.en{margin-top:6px}
.n{white-space:nowrap}.sev.major{font-weight:700}mark{background:var(--mark);color:inherit}
"""
    body = (
        f"<!doctype html><meta charset=utf-8><title>Native sweep {html.escape(args.lang)}</title><style>{style}</style>"
        f"<h1>Native sweep: {html.escape(args.lang)}</h1><p>{len(items)} proposed changes: {counts['major']} major, {counts['minor']} minor.</p>"
        + (f"<h2>Major, at a glance</h2><ul>{glance}</ul>" if glance else "")
        + "<h2>All</h2><table><tr><th>#</th><th>Where / English</th><th>Current</th><th>Proposed</th><th>Confirmer's note</th></tr>"
        + "".join(rows)
        + "</table>"
    )
    pathlib.Path(args.page).write_text(body, encoding="utf-8")
    print(f"{len(items)} items: {args.page}")


def adopt_spaces(new, text):
    """Give new the file's own spaces around punctuation, such as French non-breaking ones."""
    for punct, pattern, replace in [(p, f"([{SPACES}]){re.escape(p)}", lambda s, p: f"{s}{p}") for p in ":;?!»"] + [("«", f"«([{SPACES}])", lambda s, p: f"{p}{s}")]:
        used = collections.Counter(re.findall(pattern, text)).most_common(1)
        if used and used[0][0] != " ":
            new = new.replace(replace(" ", punct), replace(used[0][0], punct))
    return new


def apply_one(text, location, item):
    """The text with the edit made, or the reason it could not be."""
    span = locate(text, location["file"], location["where"], item["current"])
    if not span:
        return None, "message not found"
    found = matches(text, span, item["current"])
    if len(found) != 1:
        return None, f"{len(found)} matches"
    begin, end = span[0] + found[0][0], span[0] + found[0][1]
    old = text[begin:end]
    new_lines = [line.strip() for line in item["final"].split("\n")]
    if len(new_lines) - 1 != old.count("\n"):
        return None, "line count differs"
    indent = re.search(r"\n([ \t]*)", old)
    new = ("\n" + indent.group(1) if indent else "\n").join(new_lines)
    return text[:begin] + adopt_spaces(new, text) + text[end:], None


def apply_edits(args):
    out = pathlib.Path(args.out_dir) / args.lang
    items = json.loads((out / "confirmed.json").read_text(encoding="utf-8"))
    cache, unplaced, applied = {}, [], 0
    for item in items:
        for location in item["locations"]:
            text = read_file(cache, location["file"])
            result, reason = apply_one(text, location, item)
            if result is None:
                unplaced.append({"id": item["id"], **location, "current": item["current"], "final": item["final"], "reason": reason})
            else:
                cache[location["file"]] = result
                applied += 1
    for file, text in cache.items():
        (REPO / file).write_text(text, encoding="utf-8")
    (out / "unplaced.json").write_text(json.dumps(unplaced, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"{args.lang}: applied {applied}, unplaced {len(unplaced)}")
    for u in unplaced:
        print(f'  {u["file"]} {u["where"]}: {u["reason"]}')


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(required=True)
    p = commands.add_parser("extract")
    p.add_argument("journal")
    p.add_argument("out_dir")
    p.add_argument("--prior", action="append", default=[])
    p.set_defaults(run=extract)
    p = commands.add_parser("page")
    p.add_argument("out_dir")
    p.add_argument("lang")
    p.add_argument("page")
    p.set_defaults(run=page)
    p = commands.add_parser("apply")
    p.add_argument("out_dir")
    p.add_argument("lang")
    p.set_defaults(run=apply_edits)
    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
