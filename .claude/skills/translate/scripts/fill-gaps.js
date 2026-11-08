export const meta = {
  name: 'fill-translation-gaps',
  description: 'Translate the messages each shipped language lacks, review them against the English and the glossary, and fix what the review finds',
  phases: [
    { title: 'Draft', detail: 'one agent per language, missing messages only' },
    { title: 'Review', detail: 'the added messages, MQM-style' },
    { title: 'Fix', detail: 'apply findings, re-run the checks' },
    { title: 'Register', detail: 'the ftl! lines of the new files, one agent for all languages' },
  ],
}

// args: {repo, langs: [{lang, name}], writeModel?, optional?} — see the translate skill.
const REPO = args.repo
const WRITE = args.writeModel ? { model: args.writeModel } : {}
// With optional, the optional text too (spec notes, capture's own text).
const GAPS = args.optional ? 'shipped_languages_have_the_optional_text' : 'shipped_languages_are_complete'

// Every agent stays in the checkout it was given: another run's text would
// steer a draft or a review without anyone seeing it.
const STAY = lang => `Read nothing outside ${REPO}, and under its references/ only your own scratch files and the approved sources that references/l10n/${lang}/SOURCE.txt lists.`

const common = (lang, name) => `
You are bringing the ${name} (locale id \`${lang}\`) translation of the dmm-tools apps (dmm-gui desktop app, dmm-cli CLI for digital multimeters) up to date with its English, in ${REPO}. Work only in ${REPO}/i18n/${lang}/; never edit Rust code, en-US files, docs, other languages or anything else, and never commit, stash or push. Scratch files go under ${REPO}/references/l10n/${lang}/work/ (gitignored). ${STAY(lang)}

Read first, and follow: ${REPO}/i18n/${lang}/STYLE.md (section 0 "Rulings", and the section on how ${name} differs from English) and ${REPO}/i18n/${lang}/glossary.tsv, and ${REPO}/CONTRIBUTING.md section "Translations". Read the existing ${lang} file around each message you add, so the new text uses the words the language already uses for the same things.`

const FINDINGS = {
  type: 'object',
  properties: {
    findings: { type: 'array', items: { type: 'object', properties: {
      file: { type: 'string' },
      id: { type: 'string', description: 'message id, with .attribute if any' },
      severity: { type: 'string', enum: ['critical', 'major', 'minor'] },
      issue: { type: 'string' },
      suggestion: { type: 'string', description: 'the corrected text' },
    }, required: ['file', 'id', 'severity', 'issue', 'suggestion'] } },
    notes: { type: 'string' },
  },
  required: ['findings', 'notes'],
}

const results = await pipeline(
  args.langs,
  ({ lang, name }) => agent(`${common(lang, name)}

List what ${lang} lacks:
  cd ${REPO} && cargo test -q -p dmm-shared ${GAPS} -- --ignored 2>&1 | grep '^${lang}/'
For each missing message, add its translation to the same file under i18n/${lang}/, at the place the en-US file has it (same order, same comments copied from en-US). A missing file: copy it from en-US and translate it; a later step compiles it in. Translate the text only: keep IDs, attributes, { $variables }, selectors and their keys, \`code\`, --options, command lines, numbers and units as they are.

Write each message as a native ${name} technical writer would: render what it tells the reader, and build the sentence the way ${name} builds it rather than following the English one. A glossary term covers the sense its row describes: where the English word is used in another sense, use the word ${name} has for that sense and report it as a glossary gap. Where the message's comment, or a search for its ID under ${REPO}/crates/, shows it is printed after or inside other text, write it so the whole line reads correctly, punctuation included.

When the drafts are done, reread each one alone for native feel: calques of English syntax, word order and what each modifier attaches to, prepositions or particles, noun chains, verbs that do not go with their object, a register that sounds translated. Revise what you find. Then run
  cd ${REPO} && cargo test -q -p dmm-shared translations 2>&1 | grep -E '^${lang}/|test result'
and the completeness command above until neither lists ${lang}. Return the IDs you added, with any you were unsure about (your choice and the alternative).`,
    { label: `draft:${lang}`, phase: 'Draft', ...WRITE }),
  (draft, { lang, name }) => agent(`${STAY(lang)}

You review, without editing anything, the ${name} (${lang}) messages just added to ${REPO}/i18n/${lang}/ (\`cd ${REPO} && git diff -- i18n/${lang}/\` shows them, and \`git ls-files --others -- i18n/${lang}/\` lists any new file, to read whole; the drafter's report follows). Compare each with its English in ${REPO}/i18n/en-US/, against ${REPO}/i18n/${lang}/STYLE.md section 0 and glossary.tsv, and against how the rest of the ${lang} files word the same things. Read each one again for native feel alone, as a native ${name} technical writer: calques of English syntax, word order, prepositions or particles, noun chains, a register that sounds translated. Severity: critical (wrong meaning, broken text), major (glossary term not used, unidiomatic, inconsistent with the rest of the file), minor (style, punctuation). Give the corrected text for each.

Drafter's report:
${draft ?? '(the drafter failed: report that in notes)'}`,
    { label: `review:${lang}`, phase: 'Review', schema: FINDINGS }),
  (review, { lang, name }) => !review || review.findings.length === 0
    ? { lang, fixed: 0, notes: review ? review.notes : '(review failed)' }
    : agent(`${common(lang, name)}

Apply these review findings to the messages they name (critical and major always; minor unless it contradicts STYLE section 0 or the glossary, then say why), and change nothing else. Re-run \`cd ${REPO} && cargo test -q -p dmm-shared translations\` until it lists nothing for ${lang}, and \`cd ${REPO} && cargo test -q -p dmm-shared ${GAPS} -- --ignored 2>&1 | grep '^${lang}/'\` until it prints nothing. Return what you applied and skipped.

${JSON.stringify(review.findings, null, 1)}

Reviewer notes: ${review.notes}`,
      { label: `fix:${lang}`, phase: 'Fix', ...WRITE }).then(fix => ({ lang, review, fix })),
)

// One agent, after every language: the lists of ftl! lines are shared files.
// It reads what is on disk, as a language whose drafter died leaves no result.
const register = await agent(`The translations of ${REPO}/i18n/ have just been filled in, and any new .ftl file still has to be compiled in. List them:
  cd ${REPO} && cargo test -q --workspace messages_match_their_uses 2>&1 | grep 'not compiled in'
For each, add a \`dmm_shared::ftl!("<locale>", "<path>")\` line to the list that holds the en-US one (in crates/dmm-gui/src/l10n.rs, crates/dmm-cli/src/l10n.rs or crates/dmm-shared/src/), among that locale's lines at the position the file has in the other locales' lines. A list that holds only the en-US line gets one line per locale in the order the languages have elsewhere in that file; run cargo fmt on what you edit. Edit nothing but those lists, and never commit, stash or push. Repeat the command until it prints nothing, then run
  cd ${REPO} && cargo test -q -p dmm-shared ${GAPS} -- --ignored 2>&1 | grep -E '^(${args.langs.map(l => l.lang).join('|')})/'
and return the lines you added, then every language that command still lists, with what it lists: their drafter or fixer failed, and their files are partial.`,
  { label: 'register', phase: 'Register', ...WRITE })

return { results, register }
