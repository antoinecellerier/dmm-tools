export const meta = {
  name: 'spec-transcription',
  description: 'Blind double transcription of manual spec tables, diff, adjudication and cross-source check per meter',
  phases: [
    { title: 'Transcribe', detail: 'two blind transcriptions per model (Opus, Sonnet)' },
    { title: 'Adjudicate', detail: 'diff, resolve each disagreement from the page image, cross-check, merge' },
  ],
}

const SHAPE = `{
  "model": "<model>",
  "source": "<manual path> <revision as printed>",
  "conditions": "<accuracy conditions, verbatim>",
  "tables": [
    {
      "table": "<section title as printed, e.g. 'A. DC Voltage'>",
      "page": <PDF page number, int>,
      "input_impedance": "<text>" | null,
      "overload_protection": "<text>" | null,
      "notes": ["<remark verbatim>", ...],
      "ranges": [
        {
          "range": "<range as printed, e.g. '400mV'>",
          "resolution": "<as printed, e.g. '0.01mV'>" | null,
          "accuracy": [ { "freq_range": "<qualifier>" | null, "accuracy": "<e.g. '0.05%+5'>" | null } ],
          "input_impedance": "<only when rows differ>",
          "overload_protection": "<only when rows differ>",
          "comment": "<merged-cell spans, unreadable notes; optional>"
        }
      ]
    }
  ]
}`

const RULES = `Transcription rules:
1. Structure (which value belongs to which range/band) comes ONLY from the page images. Look at the cell borders: a value in a cell spanning several rows applies to each of those rows. Expand it into every row it spans and say so in that row's "comment" (e.g. "accuracy cell merged across 4V–400V").
2. Transcribe every detailed accuracy table (sections A, B, C, …) in manual order. Do not transcribe the summary/"basic specifications" table or the general specifications, but read them for remarks that apply to a table.
3. Values verbatim as printed, with only these normalisations: accuracy written as "0.3%+2" (drop ± and the parentheses); printed words such as "Not Specified" stay as the accuracy text; use the Unicode glyphs µ, Ω, °, ≤, ≥, ~ as printed.
4. "freq_range" on an accuracy band holds whatever qualifies that figure: a frequency band (e.g. "40Hz–1kHz"; en dash where the manual prints a dash, "~" where it prints ~), a sub-range (temperature spans), or a printed condition (e.g. "under REL mode"). null when the figure has no qualifier.
5. When several printed rows are sub-divisions of ONE meter range (AC bandwidth sub-rows, temperature spans), record ONE range entry with one accuracy band per sub-row. If a sub-row has a resolution of its own, keep the range's first resolution and describe the others in "comment".
6. input_impedance / overload_protection: at table level when one value covers every row of the table (merged or repeated); when rows differ, set the table level to null and put the value on each row.
7. Remarks, footnotes and markers (a), b), *, "Remarks", notes under a table, remarks continuing on the next page) must be resolved: put each remark's text verbatim in the table's "notes". A remark applying to one row only is prefixed with that row's range, e.g. "600Ω: + test lead short circuit resistance". Do not paraphrase. A letter that is part of a label (e.g. "Input scope (a)") is not a footnote — check before treating it as one.
8. Anything you cannot read with certainty even after zooming: null, plus "comment": "unreadable: <what>" on that row. Never infer a value from neighbouring cells, other models, or what would "make sense".
9. "page" is the PDF page number (the NN of the render p-NN.png), not the printed page number.`

function tools(m, who) {
  return `Inputs:
- Page renders (200 dpi PNG) of the manual's spec section: ${m.renders}/p-NN.png, NN = PDF page, for pages ${m.pages}. View them with the Read tool.
- The PDF: ${m.manual}. To zoom, render a region at higher resolution, e.g. \`pdftoppm -r 400 -f NN -l NN -png -x X -y Y -W W -H H ${m.manual} ${m.scratch}/zoom-${who}/pNN\` (X/Y/W/H in 400-dpi pixels), or \`magick ${m.renders}/p-NN.png -crop WxH+X+Y -resize 200% ${m.scratch}/zoom-${who}/c.png\`, then Read the result. Put scratch images only under ${m.scratch}/zoom-${who}/ (mkdir -p it).
- Optional text layer: \`pdftotext -layout -f NN -l NN ${m.manual} -\`. It DROPS ±, Ω, ≤, ≥, ° and misaligns merged rows: use it only to confirm digits you have already located in the image, never for structure.`
}

const SUMMARY = {
  type: 'object',
  properties: {
    tables: { type: 'integer' },
    rows: { type: 'integer' },
    unreadable: { type: 'array', items: { type: 'string' } },
    remarks: { type: 'string' },
  },
  required: ['tables', 'rows', 'unreadable', 'remarks'],
}

function transcribePrompt(m, who) {
  const order = who === 'b'
    ? '\nWork through the pages in REVERSE order (last spec page first) while reading, but write the tables in manual order.'
    : ''
  return `You are transcribing the electrical specification tables of the UNI-T ${m.name} multimeter from its user manual into JSON. This data ships in an app: exactness matters more than speed, and nothing may be guessed. Work alone; do not look for other transcriptions (other files under ${m.work} are off limits).${order}

${tools(m, who)}

${RULES}

Output: write the JSON to ${m.work}/transcription-${who}.json in exactly this shape (no other keys):
${SHAPE}
Then run \`python3 ${m.tool} check ${m.work}/transcription-${who}.json\` and fix until it prints "ok".

Return: the number of tables and rows, the rows you marked unreadable, and one line of remarks (e.g. layout surprises).`
}

const ADJ = {
  type: 'object',
  properties: {
    verdicts: { type: 'object', properties: {}, additionalProperties: { type: 'integer' } },
    unknown: { type: 'array', items: { type: 'string' } },
    cross_source: { type: 'array', items: { type: 'string' } },
    merge_output: { type: 'string' },
  },
  required: ['verdicts', 'unknown', 'cross_source', 'merge_output'],
}

function adjudicatePrompt(m) {
  return `You adjudicate two blind transcriptions of the UNI-T ${m.name} manual's specification tables, then cross-check the result. Exactness matters more than speed; nothing may be guessed.

${tools(m, 'adj')}

${RULES}

Steps:
1. Run \`python3 ${m.tool} diff ${m.work}/transcription-a.json ${m.work}/transcription-b.json --count ${m.count} > ${m.work}/diff.json\` and read it. Each item is a (key, field) where the transcriptions disagree, or where both agree but a mechanical check flagged the value.
2. For EVERY item, look at the page image (zoom into the exact cell) and decide a verdict:
   - "a" or "b": that transcription is right; "value" is that transcription's value for the field, copied exactly.
   - "other": both are wrong; "value" is the correct value, following the rules above.
   - "confirmed": a flagged value both agree on is right as printed (no value).
   - "unknown": you cannot read it with certainty even zoomed (no value). Say precisely where it is: page, table, row, column.
   Value shapes by field: resolution / input_impedance / overload_protection: string or null; accuracy: the full band list [{"freq_range", "accuracy"}]; notes: the full list of strings; page: int; row: the whole range entry (with its effective input_impedance and overload_protection) or null to delete it; table: the whole table object (transcription shape) or null to delete it.
   "evidence" says what you saw and where, e.g. "p60 zoomed: 400V row, 1kHz–10kHz sub-row reads ±(1.2%+40)".
3. Only after step 2, cross-check against these secondary sources: ${m.cross}
   Compare only what a secondary source actually carries (the set of ranges, basic/best accuracy per function, resolution). For each mismatch add an item {"key": "<table>" or "<table> / <range>", "field": "<field>", "verdict": "cross-source", "evidence": "<source> says X; manual table says Y"}. Never change a manual value because of a secondary source. Manual-internal conflicts you notice (e.g. a range labelled differently in the spec table and elsewhere in the manual) go in as "cross-source" items too, with both page references.
4. Write every item as a JSON list of {"key", "field", "verdict", "value"?, "evidence"} to ${m.work}/resolutions.json (keys and fields exactly as in diff.json for step-2 items) and run \`python3 ${m.tool} merge ${m.work}/transcription-a.json ${m.work}/resolutions.json ${m.work}\`. Fix and re-run until it succeeds; it writes verified.json and provenance.json.

Return: the count per verdict, the unknown items (each with page/table/row/column), the cross-source items (one line each), and the merge command's output.`
}

const models = args.models
const results = await pipeline(
  models,
  (m) => parallel([
    () => m.skipA
      ? Promise.resolve({ tables: -1, rows: -1, unreadable: [], remarks: 'reused the existing transcription-a.json' })
      : agent(transcribePrompt(m, 'a'), { label: `${m.id}:transcribe-a`, phase: 'Transcribe', schema: SUMMARY, model: 'opus' }),
    () => agent(transcribePrompt(m, 'b'), { label: `${m.id}:transcribe-b`, phase: 'Transcribe', schema: SUMMARY, model: 'sonnet' }),
  ]),
  (pair, m) => {
    if (!pair[0] || !pair[1]) {
      log(`${m.id}: a transcriber failed; skipping adjudication`)
      return { id: m.id, a: pair[0], b: pair[1], adj: null }
    }
    return agent(adjudicatePrompt(m), { label: `${m.id}:adjudicate`, phase: 'Adjudicate', schema: ADJ, model: 'opus' })
      .then((adj) => ({ id: m.id, a: pair[0], b: pair[1], adj }))
  },
)
return results
