export const meta = {
  name: 'native-review',
  description: 'Review shipped translations (app files or README copies) for accuracy and native feel, then apply the fixes per language',
  phases: [
    { title: 'Review', detail: 'per language and file group: Sonnet + Opus accuracy, Opus native feel' },
    { title: 'Fix', detail: 'one fixer per language; languages the user reads are reported, not applied' },
  ],
}

// args: {repo, langs: [{lang, name}], target?: 'app' | 'readme', accuracy?: boolean,
//        readLangs?: [lang], writeModel?} — see the translate skill, "Native review".
const REPO = args.repo
const TARGET = args.target || 'app'
const ACCURACY = args.accuracy !== false
const READ = new Set(args.readLangs || [])
const WRITE = args.writeModel ? { model: args.writeModel } : {}

// What one reviewer reads: the language's files of one group, beside their English.
const GROUPS = TARGET === 'readme'
  ? [{ key: 'readme', files: lang => [`README.${lang}.md`], english: 'README.md' }]
  : [
      { key: 'gui', files: lang => [`i18n/${lang}/gui/*.ftl`], english: 'i18n/en-US/gui/*.ftl' },
      { key: 'cli', files: lang => [`i18n/${lang}/cli/*.ftl`, `i18n/${lang}/cli-help.ftl`], english: 'i18n/en-US/cli/*.ftl, i18n/en-US/cli-help.ftl' },
      { key: 'shared', files: lang => ['shared.ftl', 'lib-names.ftl', 'lib-alarm.ftl', 'lib-errors.ftl'].map(f => `i18n/${lang}/${f}`), english: 'the same files under i18n/en-US/' },
      { key: 'lib', files: lang => [`i18n/${lang}/lib.ftl`], english: 'i18n/en-US/lib.ftl' },
    ]

const FINDINGS = {
  type: 'object',
  properties: {
    findings: { type: 'array', items: { type: 'object', properties: {
      file: { type: 'string', description: 'path from the repo root' },
      where: { type: 'string', description: 'message id with .attribute if any, or the line number for a README' },
      kind: { type: 'string', enum: ['accuracy', 'terms', 'native', 'structure'] },
      severity: { type: 'string', enum: ['critical', 'major', 'minor'] },
      current: { type: 'string' },
      suggestion: { type: 'string', description: 'the corrected text, whole' },
      why: { type: 'string', description: 'a few words' },
    }, required: ['file', 'where', 'kind', 'severity', 'current', 'suggestion', 'why'] } },
    notes: { type: 'string' },
  },
  required: ['findings', 'notes'],
}

const house = (lang) => `House rules, which a finding must not break: ${REPO}/i18n/${lang}/STYLE.md (section 0 "Rulings" applies) and ${REPO}/i18n/${lang}/glossary.tsv${TARGET === 'readme' ? `, and ${REPO}/.claude/rules/readme.md ("Translated copies")` : `; text that stays English by design ("Stays English" in ${REPO}/docs/localization-design.md); notation kept as the meter prints it; { $variables }, \`code\`, --options, numbers and units as written. A label in a chip row, the top bar or a statistics caption has a width budget (comments say so): a longer wording is a finding only if it still fits`}. A term a platform names in its own interface (GitHub's issue, release, pull request; a Windows or macOS setting) takes the platform's own word in the language: GitHub Docs (docs.github.com/<lang>/), Microsoft's and Apple's support pages. You may fetch those; no other web source, no other translations, no machine-translation services.`

const intro = (lang, name, g) => `Read-only review: never edit, commit or stash anything; report findings only. Repo: ${REPO} (dmm-tools: the dmm-gui desktop app and the dmm-cli command-line tool for digital multimeters). Under review: the ${name} (${lang}) ${TARGET === 'readme' ? 'translation of the README' : 'translation of the apps'}, files ${g.files(lang).join(', ')}, beside their English in ${g.english}. Read every file of yours in full.`

const ACCURACY_ASK = `Look for, most important first: meaning that differs from the English (a claim or instruction added, lost or changed); terms that stray from the glossary or from how the rest of the language's files word the same thing; text left in English that should not be; broken structure (a lost variable, a link or anchor that no longer matches, a reordered list). Each finding gives the corrected text whole. Say so in notes if nothing needs changing.`

const NATIVE_ASK = (name) => `Ignore meaning and terms (others check those). Look only at native feel: where a native ${name} technical writer would have phrased it differently. Calques of English syntax, awkward prepositions, particles or articles, a missing elision, English word order carried over, long modifier or noun chains, odd collocations, a register that sounds translated. Example from a review of the French README: « les pinces multimètres de UNI-T » was a calque a native writes « des marques UNI-T ». Only what a native reader would actually notice; each finding gives the corrected text whole. If the text reads natively, say so and list nothing.`

const reviewers = (lang, name) => GROUPS.flatMap(g => [
  ...(ACCURACY ? [
    { g, label: `accuracy-sonnet:${lang}:${g.key}`, model: 'sonnet', ask: ACCURACY_ASK },
    { g, label: `accuracy-opus:${lang}:${g.key}`, model: 'opus', ask: ACCURACY_ASK },
  ] : []),
  { g, label: `native:${lang}:${g.key}`, model: 'opus', ask: NATIVE_ASK(name) },
])

const results = await pipeline(
  args.langs,
  ({ lang, name }) => parallel(reviewers(lang, name).map(r => () =>
    agent(`${intro(lang, name, r.g)}\n\n${house(lang)}\n\n${r.ask}`,
      { label: r.label, phase: 'Review', model: r.model, schema: FINDINGS })
      .then(res => res && { reviewer: r.label, ...res }))),
  (reviews, { lang, name }) => {
    const found = reviews.filter(Boolean)
    const findings = found.flatMap(r => r.findings.map(f => ({ ...f, reviewer: r.reviewer })))
    const notes = found.map(r => `${r.reviewer}: ${r.notes}`).join('\n')
    log(`${lang}: ${findings.length} findings from ${found.length}/${reviews.length} reviewers`)
    if (findings.length === 0 || READ.has(lang)) return { lang, findings, notes, fix: READ.has(lang) ? '(a language the user reads: findings reported, not applied)' : '(nothing to fix)' }
    return agent(`You are the fixer for the ${name} (${lang}) ${TARGET === 'readme' ? `README, ${REPO}/README.${lang}.md` : `translation, ${REPO}/i18n/${lang}/`}. Edit only those files; never commit, stash or push.

${house(lang)}

Reviewers reported the findings below. Apply each critical and major one, and each minor one unless it breaks a house rule (then skip it and say why). Where findings on the same text disagree, choose by the house rules and the English, and report the disagreement with your choice. Change nothing nobody flagged.${TARGET === 'readme' ? ' Keep the first line (the hash comment), the language row and the generated CLI sample as they are.' : ''}

Then check: ${TARGET === 'readme'
  ? `cd ${REPO} && cargo test -q -p dmm-cli --test readme_languages --test docs_snippets`
  : `cd ${REPO} && cargo test -q -p dmm-shared translations 2>&1 | grep -E '^${lang}/|test result', and cargo test -q -p dmm-shared long_labels -- --ignored --nocapture 2>&1 | grep '^${lang}/' (a label you lengthened past its budget goes back to a shorter wording)`}, until it reports nothing for your files. Cargo may wait on a lock while another fixer builds: wait.

Findings:
${JSON.stringify(findings, null, 1)}

Reviewer notes:
${notes}

Return: how many findings you applied, those you skipped and why, each disagreement and your choice, and the final check result.`,
      { label: `fix:${lang}`, phase: 'Fix', ...WRITE })
      .then(fix => ({ lang, findings, notes, fix }))
  },
)

return results
