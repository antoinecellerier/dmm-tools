#!/usr/bin/env python3
"""Mechanical steps of the spec-transcription pipeline.

  check  T.json                       validate a transcription's shape
  diff   A.json B.json --count N      blind double-entry diff + sanity flags -> JSON on stdout
  merge  A.json resolutions.json DIR  apply adjudicated values -> DIR/verified.json, DIR/provenance.json
  compare verified.json dump.json [--notes app-notes.json]
                                      code (dump_specs --format json) vs verified transcription;
                                      --notes checks the code's notes against reviewed short notes

Transcription shape (what transcribers write, and what `dump_specs --format json` emits):
  {model, source, conditions, tables: [{table, page, input_impedance|null,
   overload_protection|null, notes: [str], ranges: [{range, resolution,
   accuracy: [{freq_range|null, accuracy|null}], input_impedance?,
   overload_protection?, comment?}]}]}

Everything is compared on a flattened form: one record per (table, range) with
the effective impedance/overload (row override, else the table's), plus
table-level records (page, notes). Values are normalised for comparison only.
"""
import json
import re
import sys

SEP = " / "

TABLE_KEYS = {"table", "page", "input_impedance", "overload_protection", "notes", "ranges", "comment"}
RANGE_KEYS = {"range", "resolution", "accuracy", "input_impedance", "overload_protection", "comment"}
BAND_KEYS = {"freq_range", "accuracy", "comment"}
TOP_KEYS = {"model", "source", "conditions", "tables"}


def die(msg):
    print(f"error: {msg}", file=sys.stderr)
    sys.exit(2)


def load(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def norm(v):
    """Comparison form: case, whitespace, ±, parentheses, look-alike glyphs."""
    if v is None:
        return None
    s = str(v)
    s = s.replace("μ", "µ")  # Greek mu -> micro sign
    s = s.replace("Ω", "Ω")  # ohm sign -> Greek Omega
    s = s.replace("℃", "°C").replace("℉", "°F")
    s = re.sub(r"[–—−~～]", "-", s)
    s = s.replace("≤", "<=").replace("≥", ">=")
    s = s.replace("±", "")
    s = re.sub("[\u03c6\u03a6\u03d5\u2300\u00f8]", "\u00f8", s)  # φ Φ ϕ ⌀ ø: the fuse diameter sign
    s = re.sub(r"[()\s]", "", s)
    s = re.sub(r"(?<=\d),(?=\d{3})", "", s)  # digit grouping: 1,000V is 1000V
    return s.lower()


def check(t, path="<transcription>"):
    errs = []
    if not isinstance(t, dict):
        die(f"{path}: top level is not an object")
    for k in t:
        if k not in TOP_KEYS:
            errs.append(f"unknown top-level key {k!r}")
    if not isinstance(t.get("tables"), list) or not t["tables"]:
        errs.append("tables missing or empty")
    seen = set()
    for ti, tab in enumerate(t.get("tables") or []):
        where = f"tables[{ti}]"
        for k in tab:
            if k not in TABLE_KEYS:
                errs.append(f"{where}: unknown key {k!r}")
        if not isinstance(tab.get("table"), str) or not tab["table"]:
            errs.append(f"{where}: table title missing")
        # null only in a code dump of a table with no page recorded (UT61+).
        if tab.get("page") is not None and not isinstance(tab.get("page"), int):
            errs.append(f"{where}: page must be an int (PDF page) or null")
        if not isinstance(tab.get("notes", []), list):
            errs.append(f"{where}: notes must be a list")
        for ri, r in enumerate(tab.get("ranges") or []):
            w = f"{where}.ranges[{ri}]"
            for k in r:
                if k not in RANGE_KEYS:
                    errs.append(f"{w}: unknown key {k!r}")
            if "range" not in r:
                errs.append(f"{w}: range missing")
            if not isinstance(r.get("accuracy"), list):
                errs.append(f"{w}: accuracy must be a list")
            for bi, b in enumerate(r.get("accuracy") or []):
                for k in b:
                    if k not in BAND_KEYS:
                        errs.append(f"{w}.accuracy[{bi}]: unknown key {k!r}")
            key = (norm(tab.get("table")), norm(r.get("range")))
            if key in seen:
                errs.append(f"{w}: duplicate row {tab.get('table')}{SEP}{r.get('range')}")
            seen.add(key)
    return errs


def flatten(t):
    """-> (tables {ntable: rec}, rows {(ntable, nrange): rec}) keeping original spellings."""
    tables, rows = {}, {}
    for tab in t["tables"]:
        nt = norm(tab["table"])
        rec = tables.setdefault(nt, {"table": tab["table"], "page": tab.get("page"), "notes": []})
        for n in tab.get("notes", []):
            if n not in rec["notes"]:
                rec["notes"].append(n)
        for r in tab.get("ranges", []):
            rows[(nt, norm(r["range"]))] = {
                "table": tab["table"],
                "range": r["range"],
                "resolution": r.get("resolution"),
                "accuracy": [
                    {"freq_range": b.get("freq_range"), "accuracy": b.get("accuracy")}
                    for b in r.get("accuracy", [])
                ],
                "input_impedance": r.get("input_impedance", tab.get("input_impedance")),
                "overload_protection": r.get("overload_protection", tab.get("overload_protection")),
            }
    return tables, rows


def norm_acc(bands):
    return [(norm(b["freq_range"]), norm(b["accuracy"])) for b in bands]


PREFIX = {"p": 1e-12, "n": 1e-9, "µ": 1e-6, "u": 1e-6, "m": 1e-3, "": 1.0, "k": 1e3, "M": 1e6, "G": 1e9}
QTY = re.compile(r"^\s*([0-9]*\.?[0-9]+)\s*([pnuµμmkMG]?)(V|A|Ω|Ω|F|Hz|S)\s*$")


def quantity(s):
    if not isinstance(s, str):
        return None
    m = QTY.match(s.replace("μ", "µ"))
    if not m:
        return None
    unit = m.group(3).replace("Ω", "Ω")
    return float(m.group(1)) * PREFIX[m.group(2)], unit


ACC_OK = re.compile(r"^\d+(\.\d+)?%(\+\d+)?$")


def sanity(row, count):
    """Flags on a single row: implausible resolution/range ratio, odd accuracy text."""
    flags = []
    rq, sq = quantity(row["range"]), quantity(row["resolution"])
    if rq and sq and rq[1] == sq[1] and sq[0] > 0 and count:
        ratio = rq[0] / sq[0]
        # A range shows at most `count` steps; 1.0001 absorbs float error.
        # Top ranges legitimately show fewer (a 40000-count meter's 1000V
        # range reads 1000.0, 10000 steps), so only flag a ratio below a
        # twentieth of the count, which points at a misread decade.
        if ratio > count * 1.0001 or ratio < count / 20:
            flags.append(("resolution", f"range/resolution = {ratio:g}, the meter counts {count}"))
    for i, b in enumerate(row["accuracy"]):
        a = b["accuracy"]
        if a is None:
            flags.append((f"accuracy[{i}]", "accuracy is null (unreadable?)"))
        elif not ACC_OK.match(norm(a) or ""):
            flags.append((f"accuracy[{i}]", f"accuracy {a!r} is not of the form 0.3%+2"))
    if row["resolution"] is None:
        flags.append(("resolution", "resolution is null (unreadable?)"))
    return flags


def cmd_diff(a_path, b_path, count):
    a, b = load(a_path), load(b_path)
    for p, t in ((a_path, a), (b_path, b)):
        errs = check(t, p)
        if errs:
            die(f"{p} fails the shape check:\n  " + "\n  ".join(errs))
    at, ar = flatten(a)
    bt, br = flatten(b)
    items = []

    def add(key, field, va, vb, why):
        items.append({"key": key, "field": field, "a": va, "b": vb, "why": why})

    for nt in sorted(set(at) | set(bt), key=lambda k: (at.get(k) or bt.get(k))["table"]):
        ta, tb = at.get(nt), bt.get(nt)
        key = (ta or tb)["table"]
        if ta is None or tb is None:
            add(key, "table", ta, tb, "table present in only one transcription")
            continue
        if ta["page"] != tb["page"]:
            add(key, "page", ta["page"], tb["page"], "page differs")
        if sorted(map(norm, ta["notes"])) != sorted(map(norm, tb["notes"])):
            add(key, "notes", ta["notes"], tb["notes"], "notes differ")
    order = list(ar) + [k for k in br if k not in ar]
    for rk in order:
        ra, rb = ar.get(rk), br.get(rk)
        key = SEP.join(((ra or rb)["table"], (ra or rb)["range"]))
        if ra is None or rb is None:
            add(key, "row", ra, rb, "row present in only one transcription")
            continue
        for f in ("resolution", "input_impedance", "overload_protection"):
            if norm(ra[f]) != norm(rb[f]):
                add(key, f, ra[f], rb[f], f"{f} differs")
        if norm_acc(ra["accuracy"]) != norm_acc(rb["accuracy"]):
            add(key, "accuracy", ra["accuracy"], rb["accuracy"], "accuracy bands differ")
    disputed = {(i["key"], i["field"]) for i in items}
    for rk, ra in ar.items():
        rb = br.get(rk)
        if rb is None:
            continue
        key = SEP.join((ra["table"], ra["range"]))
        for field, why in sanity(ra, count):
            f = "accuracy" if field.startswith("accuracy") else field
            if (key, f) not in disputed:
                add(key, f, ra[f], rb[f], f"both agree, but flagged: {why}")
                disputed.add((key, f))
    out = {
        "tables": len(set(at) | set(bt)),
        "rows_a": len(ar),
        "rows_b": len(br),
        "items": items,
    }
    json.dump(out, sys.stdout, ensure_ascii=False, indent=1)
    print()


# Verdicts: a/b/other replace the value with `value`; confirmed (a flagged
# value that is right), unknown (nobody could read it) and cross-source (a
# mismatch with the product page/datasheet, manual kept) only annotate.
APPLY = {"a", "b", "other"}
KEEP = {"confirmed", "unknown", "cross-source"}


def cmd_merge(a_path, res_path, out_dir):
    a = load(a_path)
    errs = check(a, a_path)
    if errs:
        die(f"{a_path} fails the shape check:\n  " + "\n  ".join(errs))
    at, ar = flatten(a)
    res = load(res_path)
    tables = {v["table"]: dict(v) for v in at.values()}
    rows = {SEP.join((v["table"], v["range"])): dict(v) for v in ar.values()}
    # Title/range spellings from A; resolutions refer to the diff's keys.
    prov = {}
    for r in res:
        key, field, verdict = r["key"], r["field"], r["verdict"]
        mark = f"{key}{SEP}{field}"
        if verdict not in APPLY | KEEP:
            die(f"{mark}: verdict {verdict!r} is not one of {sorted(APPLY | KEEP)}")
        if mark in prov:
            prov[mark]["evidence"] += f" | {verdict}: {r.get('evidence', '')}"
        else:
            prov[mark] = {"status": verdict, "evidence": r.get("evidence", "")}
        if verdict in KEEP:
            continue
        val = r.get("value")
        if field == "table":
            if val is None:
                tables.pop(key, None)
                for k in [k for k in rows if rows[k]["table"] == key]:
                    rows.pop(k)
            else:
                t2, r2 = flatten({"tables": [val]})
                tables[key] = next(iter(t2.values()))
                for v in r2.values():
                    rows[SEP.join((v["table"], v["range"]))] = v
        elif field in ("page", "notes"):
            if key not in tables:
                die(f"resolution for unknown table {key!r}")
            tables[key][field] = val
        elif field == "row":
            if val is None:
                rows.pop(key, None)
            else:
                rows[key] = val
        else:
            if key not in rows:
                die(f"resolution for unknown row {key!r}")
            rows[key][field] = val
    verified = {
        "model": a.get("model"),
        "source": a.get("source"),
        "conditions": a.get("conditions"),
        "tables": list(tables.values()),
        "rows": list(rows.values()),
    }
    with open(f"{out_dir}/verified.json", "w", encoding="utf-8") as f:
        json.dump(verified, f, ensure_ascii=False, indent=1)
        f.write("\n")
    with open(f"{out_dir}/provenance.json", "w", encoding="utf-8") as f:
        json.dump(prov, f, ensure_ascii=False, indent=1)
        f.write("\n")
    unknown = [k for k, v in prov.items() if v["status"] == "unknown"]
    print(json.dumps({"tables": len(tables), "rows": len(rows), "resolutions": len(res),
                      "unknown": unknown}, ensure_ascii=False, indent=1))


def cmd_compare(v_path, dump_path, notes_path=None):
    """verified.json (flat) vs a dump_specs JSON (nested). Exit 1 on any mismatch.

    With notes_path, the code's notes are compared against that reviewed file
    ({table title: [short app note, ...]}, every table listed) instead of the
    verbatim remarks, which stay in verified.json as the record of the manual.
    """
    v = load(v_path)
    app_notes = load(notes_path) if notes_path else None
    d = load(dump_path)
    errs = check(d, dump_path)
    if errs:
        die(f"{dump_path} fails the shape check:\n  " + "\n  ".join(errs))
    dt, dr = flatten(d)
    vt = {norm(t["table"]): t for t in v["tables"]}
    vr = {(norm(r["table"]), norm(r["range"])): r for r in v["rows"]}
    out = []
    for nt in sorted(set(vt) | set(dt)):
        a, b = vt.get(nt), dt.get(nt)
        name = (a or b)["table"]
        if a is None or b is None:
            out.append(f"{name}: table only in {'code' if a is None else 'verified'}")
            continue
        if a.get("page") != b.get("page"):
            out.append(f"{name}: page verified {a.get('page')} code {b.get('page')}")
        if app_notes is not None:
            want = next((n for t, n in app_notes.items() if norm(t) == nt), None)
            if want is None:
                out.append(f"{name}: table missing from {notes_path}")
            elif sorted(map(norm, want)) != sorted(map(norm, b["notes"])):
                out.append(f"{name}: notes\n    app-notes {want}\n    code      {b['notes']}")
        elif sorted(map(norm, a["notes"])) != sorted(map(norm, b["notes"])):
            out.append(f"{name}: notes\n    verified {a['notes']}\n    code     {b['notes']}")
    if app_notes is not None:
        for t in app_notes:
            if norm(t) not in vt:
                out.append(f"{t}: in {notes_path} but not a verified table")
    for rk in list(vr) + [k for k in dr if k not in vr]:
        a, b = vr.get(rk), dr.get(rk)
        name = SEP.join(((a or b)["table"], (a or b)["range"]))
        if a is None or b is None:
            out.append(f"{name}: row only in {'code' if a is None else 'verified'}")
            continue
        for f in ("resolution", "input_impedance", "overload_protection"):
            if norm(a.get(f)) != norm(b.get(f)):
                out.append(f"{name}: {f} verified {a.get(f)!r} code {b.get(f)!r}")
        if norm_acc(a["accuracy"]) != norm_acc(b["accuracy"]):
            out.append(f"{name}: accuracy\n    verified {a['accuracy']}\n    code     {b['accuracy']}")
    if out:
        print("\n".join(out))
        print(f"{len(out)} mismatch(es)")
        sys.exit(1)
    print(f"match: {len(vt)} tables, {len(vr)} rows")


def main(argv):
    if len(argv) >= 3 and argv[1] == "check":
        errs = check(load(argv[2]), argv[2])
        if errs:
            print("\n".join(errs))
            sys.exit(1)
        t, r = flatten(load(argv[2]))
        print(f"ok: {len(t)} tables, {len(r)} rows")
    elif len(argv) == 6 and argv[1] == "diff" and argv[4] == "--count":
        cmd_diff(argv[2], argv[3], int(argv[5]))
    elif len(argv) == 5 and argv[1] == "merge":
        cmd_merge(argv[2], argv[3], argv[4])
    elif len(argv) == 4 and argv[1] == "compare":
        cmd_compare(argv[2], argv[3])
    elif len(argv) == 6 and argv[1] == "compare" and argv[4] == "--notes":
        cmd_compare(argv[2], argv[3], argv[5])
    else:
        print(__doc__)
        sys.exit(2)


if __name__ == "__main__":
    main(sys.argv)
