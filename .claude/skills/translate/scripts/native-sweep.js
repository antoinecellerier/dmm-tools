export const meta = {
  name: 'native-sweep',
  description: 'Two-stage native-feel sweep of shipped translations: one reader per file group finds, a confirmer rules on each finding. Read-only',
  phases: [
    { title: 'Find', detail: 'one reader per language and file group' },
    { title: 'Confirm', detail: 'findings in chunks per language, default reject' },
  ],
}

// args: {repo, langs: [{lang, name}], findModel?, confirmModel?, targets?: ['app', 'readme']}
// See the translate skill, "Native review". The script returns counts only: the
// findings and verdicts stay in the run's journal.jsonl, for native-sweep.py.
const REPO = args.repo
const FIND_MODEL = args.findModel || 'opus'
const CONFIRM_MODEL = args.confirmModel || 'opus'
const TARGETS = new Set(args.targets || ['app', 'readme'])
const CHUNK = 50

const inDir = (lang, names) => names.map(f => `i18n/${lang}/${f}`)
const APP = [
  { key: 'gui-a', files: lang => inDir(lang, [...['top-bar', 'connection', 'reading', 'recording', 'settings', 'shortcuts', 'desktop', 'command-line', 'specs-remote'].map(f => `gui/${f}.ftl`), 'gui-help.ftl']) },
  { key: 'gui-b', files: lang => inDir(lang, ['graph', 'colors', 'export-import', 'alarm-scale', 'stats-markers'].map(f => `gui/${f}.ftl`)) },
  { key: 'cli', files: lang => [`i18n/${lang}/cli/*.ftl`, `i18n/${lang}/cli-help.ftl`] },
  { key: 'shared', files: lang => inDir(lang, ['shared.ftl', 'lib-names.ftl', 'lib-alarm.ftl', 'lib-errors.ftl']) },
  { key: 'lib', files: lang => inDir(lang, ['lib.ftl']) },
]
const README = { key: 'readme', readme: true, files: lang => [`README.${lang}.md`] }
const GROUPS = [...(TARGETS.has('app') ? APP : []), ...(TARGETS.has('readme') ? [README] : [])]

const FINDINGS = {
  type: 'object',
  properties: {
    findings: { type: 'array', items: { type: 'object', properties: {
      file: { type: 'string', description: 'path from the repo root' },
      where: { type: 'string', description: 'id[.attr][#variant] for a Fluent message; the line number for a README' },
      severity: { type: 'string', enum: ['major', 'minor'] },
      current: { type: 'string', description: 'exactly one message value, attribute or variant, or one README sentence, copied from the file; no joins or ellipses' },
      english: { type: 'string', description: 'the same text in the English file' },
      suggestion: { type: 'string', description: 'the corrected text, covering exactly what current covers' },
      why: { type: 'string', description: 'a few words' },
    }, required: ['file', 'where', 'severity', 'current', 'english', 'suggestion', 'why'] } },
    notes: { type: 'string' },
  },
  required: ['findings', 'notes'],
}

const VERDICTS = {
  type: 'object',
  properties: {
    verdicts: { type: 'array', items: { type: 'object', properties: {
      id: { type: 'string' },
      verdict: { type: 'string', enum: ['confirm', 'revise', 'reject'] },
      final_text: { type: 'string', description: 'for revise only: the better text, covering exactly what current covers' },
      severity: { type: 'string', enum: ['major', 'minor'] },
      why: { type: 'string', description: 'a few words' },
    }, required: ['id', 'verdict', 'severity', 'why'] } },
  },
  required: ['verdicts'],
}

const sections = name => `"Rulings" (section 0) and "How ${name} differs from English"`

const house = (lang, g) => g.readme
  ? `House rules, which a change must not break: ${REPO}/i18n/${lang}/STYLE.md, ${REPO}/i18n/${lang}/glossary.tsv and ${REPO}/.claude/rules/readme.md ("Translated copies"). The first line (the hash comment), the language row and the generated CLI sample stay as they are.`
  : `House rules, which a change must not break: ${REPO}/i18n/${lang}/STYLE.md and ${REPO}/i18n/${lang}/glossary.tsv; text that stays English by design ("Stays English" in ${REPO}/docs/localization-design.md); notation kept as the meter prints it; { $variables }, \`code\`, --options, numbers and units as written.`

const PLATFORM = `A term a platform names in its own interface (GitHub's issue, release, pull request; a Windows or macOS setting) takes the platform's own word in the language: GitHub Docs (docs.github.com/<lang>/), Microsoft's and Apple's support pages. No other web source, no other translations, no machine-translation services.`

const NATIVE_ASK = name => `Look only at native feel: where a native ${name} technical writer would have phrased it differently. Calques of English syntax, awkward prepositions, particles or articles, a missing elision, English word order carried over, long modifier or noun chains, odd collocations, a register that sounds translated. Example from a review of the French README: « les pinces multimètres de UNI-T » was a calque a native writes « des marques UNI-T ». Only what a native reader would actually notice. Meaning and terminology are out of scope. If the text reads natively, say so and list nothing.`

const findPrompt = (lang, name, g) => `Read-only review: never edit, commit or stash anything; report findings only. Repo: ${REPO} (dmm-tools: the dmm-gui desktop app and the dmm-cli command-line tool for digital multimeters). Under review: the ${name} (${lang}) text in ${g.files(lang).join(', ')}.

Read those files in full, on their own, as a native reader would. Open the matching English (${g.readme ? 'README.md' : 'the same path under i18n/en-US/'}) only for a message you are flagging. Of ${REPO}/i18n/${lang}/STYLE.md read only ${sections(name)}. Grep ${REPO}/i18n/${lang}/glossary.tsv only before proposing a change to a term. Look at nothing else: no git history, nothing under references/.

${house(lang, g)} ${PLATFORM}

${NATIVE_ASK(name)}

Reporting: one finding per message value, attribute or plural variant; where the same wording recurs, one finding per occurrence. "current" is copied from the file exactly and "suggestion" replaces exactly that much, keeping the same number of lines. In a nested selector, report the innermost variant's own text, never the sentence the variants assemble.`

const confirmPrompt = (lang, name, chunk) => `You are the confirmer in a native-feel review of the ${name} (${lang}) translation of dmm-tools, the dmm-gui desktop app and dmm-cli command-line tool for digital multimeters, in ${REPO}. A first reader proposed the changes below. Read-only: never edit, commit or stash anything.

Of ${REPO}/i18n/${lang}/STYLE.md read ${sections(name)}, "Length budget", "What the meter prints stays as it prints it" and "Spec text". Grep ${REPO}/i18n/${lang}/glossary.tsv for a term that a change touches. Open a file only when an item needs its neighbouring messages. Look at nothing under references/ and no git history. Text that stays English by design is listed under "Stays English" in ${REPO}/docs/localization-design.md. ${PLATFORM}

Rule on every item, returning its id:
- confirm: a native ${name} reader would notice the current text, and the suggestion fixes it.
- revise: the problem is real but the suggestion is not the best fix; give final_text, covering exactly what current covers, with the same number of lines.
- reject: anything else. The default is reject: a preference is not a defect. Always reject a suggestion that changes the meaning of the English, breaks a ruling or a glossary term, loses a { $variable }, \`code\` or --option, or makes a text of up to 24 English characters more than 1.5 times the English length.

Severity: major only if a native reader would stumble or take the text for a machine translation. Rate it yourself.

Items:
${JSON.stringify(chunk)}`

const results = await pipeline(
  args.langs,
  ({ lang, name }) => parallel(GROUPS.map(g => () =>
    agent(findPrompt(lang, name, g), { label: `find:${lang}:${g.key}`, phase: 'Find', model: FIND_MODEL, effort: 'medium', schema: FINDINGS })
      .then(res => ({ group: g.key, res })))),
  async (reads, { lang, name }) => {
    const failed = reads.filter(r => !r || !r.res).map((r, i) => r ? r.group : GROUPS[i].key)
    // Stamped by label and position; native-sweep.py rebuilds the same ids from the journal.
    const found = reads.filter(r => r && r.res).flatMap(r =>
      r.res.findings.map((f, i) => ({ ...f, id: `find:${lang}:${r.group}#${i}` })))
    const items = new Map()
    for (const f of found) {
      const key = `${f.current}\u0000${f.suggestion}`
      if (items.has(key)) items.get(key).also.push(`${f.file} ${f.where}`)
      else items.set(key, { id: f.id, file: f.file, where: f.where, also: [], english: f.english, current: f.current, suggestion: f.suggestion, why: f.why })
    }
    const list = [...items.values()].sort((a, b) => a.file < b.file ? -1 : a.file > b.file ? 1 : 0)
    const chunks = []
    for (let i = 0; i < list.length; i += CHUNK) chunks.push(list.slice(i, i + CHUNK))
    log(`${lang}: ${found.length} findings, ${list.length} items, ${chunks.length} chunks${failed.length ? `; readers failed: ${failed.join(', ')}` : ''}`)
    const ruled = (await parallel(chunks.map((chunk, i) => () =>
      agent(confirmPrompt(lang, name, chunk), { label: `confirm:${lang}:${i}`, phase: 'Confirm', model: CONFIRM_MODEL, effort: 'medium', schema: VERDICTS }))))
    const verdicts = ruled.filter(Boolean).flatMap(r => r.verdicts)
    const count = v => verdicts.filter(x => x.verdict === v).length
    const judged = new Set(verdicts.map(v => v.id))
    return { lang, found: found.length, items: list.length, confirmed: count('confirm'), revised: count('revise'), rejected: count('reject'), unjudged: list.filter(x => !judged.has(x.id)).length, failedReaders: failed, failedChunks: ruled.filter(r => !r).length }
  },
)

return results
