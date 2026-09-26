---
name: spec-data
description: >-
  Adds or re-checks a meter's specification data (resolution, accuracy bands,
  input impedance, overload protection, notes) from its manual: two blind
  transcriptions from rendered PDF pages, adjudication of every disagreement,
  a cross-check against the manufacturer's product page and datasheet,
  keyed-row spec tables with coverage tests, a dump_specs JSON compare and an
  HTML review sheet the user checks against the manual. Use when a meter's
  Specifications panel shows only the Manual link, when adding spec tables for
  a new or existing device, or when re-verifying spec tables against a manual.
---

# Spec data from a manual

CLAUDE.md applies throughout: never fabricate a value, and read tables from the rendered page, never from extracted text. `pdftotext` loses merged cells, band sub-rows and footnotes, and fonts often drop ±, Ω, ≤ and °. The manual is the source; datasheets and product pages only cross-check it. The code layout and test rules are in `docs/adding-devices.md` ("Specification data"); the `dump_specs` flags are in `docs/development.md`.

Tools: `python3`, `pdftoppm`/`pdftotext` (poppler-utils), `magick` (ImageMagick), `cargo`.

Copy this checklist and tick it off per meter:

```
Spec data: <meter>
- [ ] 1. Sources saved under references/<family>/, spec pages located
- [ ] 2. Pages rendered to references/specs-review/pages/<id>/
- [ ] 3. Transcription workflow run; both transcribers returned tables
- [ ] 4. UNKNOWN cells read by the user; parser-affecting conflicts decided
- [ ] 5. Short notes drafted and reviewed
- [ ] 6. Tables implemented; mutation-proven tests pass
- [ ] 7. specs_tool.py compare prints "match"; review sheet sent to the user
- [ ] 8. Independent review done; GUI screenshots checked
```

## 1. Sources

- Save the manual, the datasheet or flyer, and the product page under `references/<family>/` (gitignored).
  - For UNI-T, uni-trendus.com (UNI-T's US site) is much faster than meters./instruments.uni-trend.com, whose download buttons are token-gated.
  - Ask the user to download large files: a killed `curl` leaves a partial PDF behind.
- Save a product page's spec table as raw text (`curl` plus a tag strip) in `references/<family>/specs-work/`. Never use WebFetch for figures: its summarising model can change digits.
- Find the spec pages with `pdftotext` and grep. For an outline-only PDF with no text layer, build a contact sheet instead: `pdftoppm -r 30` then `magick montage -label '%t'`. Datasheets are usually outline-only.
- Record every source and its revision in `docs/research/<family>/reverse-engineering-approach.md`.

## 2. Render

```bash
pdftoppm -r 200 -png -f FIRST -l LAST manual.pdf references/specs-review/pages/<id>/p
```

Render the datasheet's spec page too. The PNG names carry the PDF page number, which is what `page` means everywhere below.

## 3. Transcribe (workflow)

Run `Workflow({scriptPath: ".claude/skills/spec-data/scripts/spec-transcription.js", args: {models: [<one entry per meter>]}})`:

| Field | Meaning |
|---|---|
| `id`, `name` | Registry id and display name |
| `count` | Display counts, for the resolution sanity check (6000, 40000, 60000) |
| `manual`, `pages` | PDF path; a prose string naming the spec pages and layout hazards, e.g. `"39-48 (A-K); no text layer"` |
| `renders`, `scratch` | The page PNGs; a scratch dir for zoom crops, outside `references/` (the session scratchpad) |
| `work` | `references/<family>/specs-work`, where every result lands |
| `tool` | Absolute path of `.claude/skills/spec-data/scripts/specs_tool.py` |
| `cross` | Prose naming each cross-check source and what it carries |
| `skipA` | Optional: reuse `transcription-a.json` and rerun only B and the adjudication |

For each meter the workflow runs:
- transcriber A (Opus) and transcriber B (Sonnet), blind to each other; a different model means their misreads rarely coincide;
- `specs_tool.py diff` with sanity flags;
- an Opus adjudicator, which zooms into every disputed cell, compares the result with the cross-check sources, and writes `verified.json` and `provenance.json`.

Don't swap in Fable: it's expensive, and this work is mechanical.

Check both transcriber summaries before you trust the result. A transcriber once took a relayed chat message about downloads as a reason to wait, and returned 0 tables. If one side is missing, rerun only that side with `skipA`.

## 4. Settle with the user

- **UNKNOWN cells:** the user reads them, all in one message. For each cell give the file, the PDF and printed page, the table, row and column, and a zoomed crop path. Record the value with provenance "read by user".
- **Manual-internal conflicts that touch the parser** (e.g. Table 2-3 labels a range 750V where the spec table says 1000V): ask the user. The fix is its own commit.
- **Cross-source mismatches:** where one matters, write a one-line code comment and keep the manual's value.

## 5. Short notes

A delegated Opus agent drafts `references/<family>/specs-work/app-notes.json` (`{table title: [notes]}`) and `app-notes-review.md` (verbatim to short, dropped, unclear). Rules:
- Keep every figure, limit and condition.
- Drop anything a field already shows (impedance, overload), and boilerplate.
- Fix spelling.
- Aim for notes under ~60 characters, at most 4 per table.
- Never compute a derived figure.
- Keep genuinely unclear remarks close to the manual's wording, and list them for the user.

Review the draft before it goes into code.

## 6. Implement

Delegate to an Opus agent, one meter per commit (`lib: <model> readings carry the manual's specs`), following `docs/adding-devices.md`:

- **Structure:**
  - keyed rows (`ModeSpecs`/`RangeSpec` in `specs.rs`), with `ALL` in manual order and one `table()` match;
  - the lookup gets the whole `Measurement`, and re-decodes `raw_payload` when coupling or duty sits outside the mode and range bytes.
- **Values:**
  - copy them exactly from `verified.json`, generated by a script;
  - normalise only typesetting slips (stray spaces, digit commas, the diameter sign as `ø`), each with a `// Printed "…"` comment;
  - write fuse ratings as `Fuse <current> <voltage>` (`Fuse 1A 240V`), with the printed text in a `// Printed "…"` comment, diameter sign as `ø`; `verified.json` holds the same form, its provenance the printed text;
  - notes are exactly `app-notes.json`.
- **Rows:**
  - a reading gets a row only where the manual prints an accuracy for it;
  - where the manual gives only a rule ("add (1%+35 digits) for AC+DC"), the reading gets the table's mode spec and no row;
  - readings the manual doesn't cover (Peak, LPF, dB, T1−T2, RPM) go on `NO_SPEC` with a reason.
- **Tests,** each proven by re-applying a mutation and watching it fail (swap two tables, two range bytes, continuity↔diode, °C↔°F):
  - coverage: every accepted reading resolves to a row, to mode spec only, or to `NO_SPEC`, and every table and list entry is reached;
  - the exact range label, or the full scale;
  - the AC/DC/AC+DC table against the reading's coupling;
  - the resolution unit (`protocol::test_support::unit_family`);
  - golden `resolution` fields;
  - one replay through `Dmm::request_measurement`.
- **Docs:** the coverage lines in `docs/gui-reference.md` and `docs/ux-design.md`, the sources in the research approach doc, and a CHANGELOG GUI entry ("The Specifications panel covers the …"). The tables need no hardware check of their own (`.claude/rules/protocol.md` exempts them), but a newly added meter gets them only after its first hardware capture (`docs/adding-devices.md`, Specification data).

## 7. Verify

1. Compare the code with the verified transcription, and loop until it matches:
   ```bash
   cargo run -q -p dmm-lib --example dump_specs -- --format json <id> > /tmp/dump.json
   python3 .claude/skills/spec-data/scripts/specs_tool.py compare <work>/verified.json /tmp/dump.json --notes <work>/app-notes.json
   ```
   It prints `match: N tables, M rows`, or one line per mismatch. On a mismatch, fix the Rust table and run it again.
2. Build the review sheet. Run it exactly like this: the image links are written relative to `references/specs-review/`, so the sheet must be written there.
   ```bash
   (cd references/specs-review && cargo run -q -p dmm-lib --example dump_specs -- --format html --pages-dir pages/<id> --marks ../<family>/specs-work/provenance.json <id> > <id>.html)
   ```
   Give the user the path. The sheet shows each manual table beside its page render (continuation pages included), with cells coloured by provenance. The user's corrections go into the Rust table and `verified.json`; then run step 1 again.
3. Have a fresh Opus reviewer check committed objects only, without editing:
   - the mapping: which reading picks which row;
   - every row against the renders;
   - whether the tests catch a swapped table or range byte.
4. Take `verify-gui` screenshots of the Specifications panel for each new kind of reading: one with a row, and one with mode spec only.

## Scripts

- **`.claude/skills/spec-data/scripts/specs_tool.py`** — run it:
  - `check T.json`: shape check;
  - `diff A.json B.json --count N`: the disagreement list, as JSON;
  - `merge A.json resolutions.json DIR`: writes `verified.json` and `provenance.json`;
  - `compare verified.json dump.json [--notes app-notes.json]`: `match`, or the mismatches, with exit status 1.

  The transcription shape is in its docstring.
- **`.claude/skills/spec-data/scripts/spec-transcription.js`** — the workflow for step 3; run it through the Workflow tool, don't read it into context.
- **`evals.json`** — three scenarios to re-test the skill after editing it.
