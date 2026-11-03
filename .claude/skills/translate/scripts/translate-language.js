export const meta = {
  name: 'translate-language',
  description: 'Draft one language from its glossary and style sheet, check it, review it on screenshots and CLI output, and fix what the review finds',
  phases: [
    { title: 'Draft', detail: 'six agents, one per file group, glossary pinned' },
    { title: 'Review', detail: 'GUI screenshots + CLI/library text, MQM-style' },
    { title: 'Fix', detail: 'apply findings, re-run the checks' },
  ],
}

// args: {repo, lang, name, glossary?, writeModel?} — see the translate skill.
const REPO = args.repo
const LANG = args.lang
const NAME = args.name
// The glossary and style sheet: committed under i18n/<lang>/ by default.
const G = args.glossary || `${REPO}/i18n/${LANG}`
// Drafting and fixing may run on a lighter model; the reviewers inherit the session's.
const WRITE = args.writeModel ? { model: args.writeModel } : {}

const COMMON = `
You are translating the dmm-tools apps (a Rust desktop app dmm-gui and CLI dmm-cli for digital multimeters) into ${NAME} (locale id \`${LANG}\`). The repo is ${REPO}. Work only inside the files you are given; never edit Rust code, en-US files, docs, other languages, or anything else, and never commit, stash or push (the orchestrator commits). Scratch files go under ${REPO}/references/l10n/${LANG}/work/ (gitignored), never only in /tmp.

Read first, and follow exactly:
- ${G}/STYLE.md — the style sheet for ${NAME}. Section "0. Rulings" is what applies (they are provisional, but they are the rules for this draft): register, punctuation and spacing, quotes, plurals, key names, badges, AC/DC.
- ${G}/glossary.tsv — use its translation for every term it lists, as its notes say.
- ${REPO}/CONTRIBUTING.md section "Translations".
- For decisions already taken in another language, you may look at a reviewed language's files (\`reviewed: true\` in ${REPO}/crates/dmm-shared/src/l10n/lang.rs; French was the first): structure, which texts stay as notation, how a message was split. Never translate from it; translate from the English.

How to translate a file \`i18n/en-US/<path>\`:
- Write \`${REPO}/i18n/${LANG}/<path>\` (create directories as needed). Start from a copy of the en-US file: keep every comment line as it is (English comments are fine), the IDs, attribute names (.hover, .a11y, …), \`{ $variables }\`, \`{ -terms }\`, message references, selector expressions and their variant keys. Translate only the text.
- Translate every message and attribute, the specification text in lib.ftl included (table-, note-, input-, band- messages; optional for shipping, but draft them). A message whose text is notation the language writes the same way (a unit, an abbreviation the meter prints, a flag like HOLD) keeps its English text.
- Read the \`#\` comments above each message and the \`##\`/\`###\` group and file comments: they say where the text shows, what each variable holds, width budgets, and whether a library label is inserted.
- Plurals: follow STYLE §0. Keep a selector's \`*[other]\` default. Never invent categories the language doesn't have; a numeric key like \`[0]\` is allowed where the style sheet asks for it.
- Keep verbatim: \`code spans\` in backticks, \`--options\` and their values, command lines, device ids (ut61eplus, mock, auto…), file names and extensions, URLs, environment variable names, the meter's own key and menu names, and every number with its unit in spec notes (the app falls back to English for a note whose numbers change). Numbers keep the English decimal point and no thousands separator.
- Fluent syntax: a continuation line is indented; a line of text must not start with \`[\`, \`*\` or \`.\` (write \`{"["}\` etc.); a literal \`{\` or \`}\` in text is \`{"{"}\` / \`{"}"}\`. Keep the en-US file's line structure for multi-line messages (activation steps keep their numbering, one step per line).
- Short labels: stay within about 1.5× the English width; the top bar and chips are tight. For Chinese, count each Han character as two Latin ones.

When your files are written, check them:
  cd ${REPO} && cargo test -q -p dmm-shared translations 2>&1 | grep -E '^${LANG}/|panicked|test result'
It lists problems as \`${LANG}/<path>: …\`. Fix every problem in YOUR files (ignore other files' lines: other agents are writing them at the same time). Re-run until none of your files is listed. Also run
  cd ${REPO} && cargo test -q -p dmm-shared long_labels -- --ignored --nocapture 2>&1 | grep '^${LANG}/'
and shorten what you can among your files' entries.
Cargo may wait on a lock while another agent builds: that's expected, just wait.

Return a short report: the files written, message count per file, any message you were unsure about (ID + your choice + the alternative), and glossary gaps you hit (an English term the glossary lacks, with the translation you chose).`

const GROUPS = [
  { key: 'shared-lib-small', files: ['shared.ftl', 'lib-names.ftl', 'lib-alarm.ftl', 'lib-errors.ftl'],
    note: 'shared.ftl is the help, setup and version text both apps print, with the words clap puts around --help (Usage:, Commands:, Options:, [default: …], [possible values: …], Print help…) and clap\'s command-line errors (names typed by the user stay in their straight quotes). Windows and macOS setting names: use STYLE §0 (Apple and Microsoft public docs are acceptable sources for these names); put a `# UNVERIFIED Windows name` comment above each message quoting a Windows name no approved source confirms, as the other languages do. lib-names.ftl holds flags (stay as the meter prints them), link names and stability badges (the app\'s own badges translate, STYLE §0). lib-errors.ftl renders library errors: reasons are lowercase with no final full stop, as the comments ask.' },
  { key: 'lib', files: ['lib.ftl'],
    note: 'lib.ftl (1374 lines) is generated from the library: modes, mode qualifiers ({ $mode } placement), ranges, aux values, toggles, meter keys and their hovers (hovers name an action: STYLE register for hovers), simulated meters\' model names, activation steps, spec table titles, input/protection strings and spec notes. Modes made of words are translated; notation (DC V, AC+DC, LPF, REL, hFE) stays. Table titles use full words for AC/DC (STYLE §0). Spec notes keep every number and unit exactly. Activation steps keep the meter\'s own menu and key names (SETUP, SELECT, MAX MIN, SEND…).' },
  { key: 'gui-1', files: ['gui/top-bar.ftl', 'gui/connection.ftl', 'gui/settings.ftl', 'gui/colors.ftl', 'gui/recording.ftl'],
    note: 'The top bar must stay one row on a wide window. settings.ftl: the Language row label follows its en-US comment exactly (the language\'s word for Language, then "(Language)"); the machine-translated notice messages apply to this language, which ships unreviewed, so word them carefully. Theme names stay English (comments say so). Button names quoted as a sentence\'s subject (recording hint, discard toast) keep their quotes, in the language\'s quote marks.' },
  { key: 'gui-2', files: ['gui/export-import.ftl', 'gui/graph.ftl', 'gui/stats-markers.ftl'],
    note: 'graph.ftl has tight chip rows (Show:, Y:Fixed/Auto, histogram, cursor read-outs): notation stays compact. stats-markers.ftl: statistics captions per STYLE and the glossary (Min, Max, Mean, σ, Readings). File-dialog filter names are short.' },
  { key: 'gui-3', files: ['gui/shortcuts.ftl', 'gui/reading.ftl', 'gui/alarm-scale.ftl', 'gui/specs-remote.ftl', 'gui/desktop.ftl', 'gui/command-line.ftl', 'gui-help.ftl'],
    note: 'shortcuts.ftl: the shortcut list (actions per STYLE register) and the shortcuts-key-* key names (STYLE §0: the names printed on keyboards in this language; the shortcut column and every hover naming a shortcut use them), and the note under the list, which must name keys exactly as those messages do. reading.ftl includes the screen-reader (spoken) forms of the reading and of flags: write them as speech. alarm-scale.ftl: alarm and scale fields and their input errors. desktop.ftl: one short line for the Linux desktop entry. command-line.ftl and gui-help.ftl are dmm-gui --help (gui-help.ftl generated from clap doc comments, as cli-help.ftl is) and a reason a command-line value was refused: keep option names, values and lists verbatim.' },
  { key: 'cli', files: ['cli/command.ftl', 'cli/common.ftl', 'cli/connect.ftl', 'cli/debug.ftl', 'cli/help.ftl', 'cli/info.ftl', 'cli/list.ftl', 'cli/read.ftl', 'cli/settings.ftl', 'cli-help.ftl'],
    note: 'dmm-cli\'s stderr prose, its --help and the words around it. cli/help.ftl is the help dmm-cli builds as it runs (the device list\'s first line, CONFIGURATION, ENVIRONMENT). cli-help.ftl is generated from clap doc comments with path-based IDs: translate .help/.long-help text, keep option names, possible values, [auto, ut61eplus, …] lists, `code` and paths verbatim. Terminal output has no width budget but keep lines readable.' },
]

phase('Draft')
const drafts = await parallel(GROUPS.map(g => () => agent(
  `${COMMON}\n\nYour files (each en-US file under ${REPO}/i18n/en-US/ → the same path under ${REPO}/i18n/${LANG}/):\n${g.files.map(f => '- ' + f).join('\n')}\n\nNotes for these files: ${g.note}`,
  { label: `draft:${LANG}:${g.key}`, phase: 'Draft', ...WRITE })))
const draftReports = GROUPS.map((g, i) => `### ${g.key}\n${drafts[i] ?? '(agent failed: files may be missing)'}`).join('\n\n')

const FINDINGS = {
  type: 'object',
  properties: {
    findings: { type: 'array', items: { type: 'object', properties: {
      file: { type: 'string', description: `path under i18n/${LANG}/` },
      id: { type: 'string', description: 'message id, with .attribute if any' },
      severity: { type: 'string', enum: ['critical', 'major', 'minor'] },
      category: { type: 'string', description: 'MQM: accuracy, fluency, terminology, style, locale-convention, layout (truncation/overflow/wrap), untranslated' },
      issue: { type: 'string' },
      suggestion: { type: 'string', description: 'the corrected text for that message or attribute' },
    }, required: ['file', 'id', 'severity', 'category', 'issue', 'suggestion'] } },
    notes: { type: 'string', description: 'anything not tied to one message: layout problems needing code, untestable areas, screenshots taken (absolute paths)' },
  },
  required: ['findings', 'notes'],
}

const REVIEW_COMMON = `
You are a fresh-context reviewer of a ${NAME} (${LANG}) translation of the dmm-tools apps (dmm-gui desktop app, dmm-cli CLI for digital multimeters) in ${REPO}. The translation is in ${REPO}/i18n/${LANG}/, the English reference in ${REPO}/i18n/en-US/. Do NOT edit any file under i18n/ or crates/ and never commit or push: report findings only. Judge against ${G}/STYLE.md (section 0 "Rulings" applies) and ${G}/glossary.tsv. Report MQM-style: critical (wrong meaning that could mislead about a measurement or an action, broken text), major (wrong term vs glossary, unidiomatic or confusing text, untranslated text, text cut off or overflowing), minor (style, punctuation, consistency). Each finding gives the corrected text. Don't report the cross-language decisions (English decimal point, meter flags kept as printed, theme names English, value names in capitals like <FORMAT>), nor text that stays English by design ("Stays English" in ${REPO}/docs/localization-design.md: logs, read's stdout data lines, set tokens, capture, What's New). Put scratch files and screenshots under ${REPO}/references/l10n/${LANG}/review/ (gitignored).`

phase('Review')
const reviews = await parallel([
  () => agent(`${REVIEW_COMMON}

Your part: the GUI, on screen. Use the verify-gui skill (${REPO}/.claude/skills/verify-gui/SKILL.md) — read it fully; launch dmm-gui ONLY through its gui-display.sh and run \`stop\` at the end, even after a failure. Load the files under review over the built-in ones: write a wrapper script under ${REPO}/references/l10n/${LANG}/review/ that exports DMM_TOOLS_L10N_DIR=${REPO}/i18n and VERIFY_GUI_STATE=$XDG_RUNTIME_DIR/verify-gui-${LANG}-review, then calls gui-display.sh with its arguments (an env prefix on the command would prompt; the script avoids that). Put {"language": "${LANG}"} in config/dmm-tools/settings.json under that state directory before \`run\`, and delete that private state directory at the end. Confirm the window is in ${NAME} in the first shot.${LANG.startsWith('zh') ? ' Chinese draws with a system font (`fc-list :lang=zh` names the one installed; with none, stop and say so): check that no character shows as a box or the replacement glyph, and that glyphs arrive (they may take a few frames after start).' : ''}
Cover, dark and light themes, at 1600 px wide and at 700 px wide (resize): the main window with a live mock (--device mock, try --mock-mode dcv and ohm), the settings panel (open it, scroll; the Language row and its notice), the colours/themes section, the shortcut list (key question), the graph with its toolbar and right-click menu, statistics and markers, alarm and scale fields, recording panel, export/import, the specifications and remote-key panels (--device mock-zt5b; golden fixtures in crates/dmm-lib/tests/golden/ pass with --replay), connection notices only if no real hardware is attached: real cables and meters may be plugged into this machine, and picking a real meter in the device dropdown makes the app open it. So never pick anything but the Simulated meters in the dropdown and never pass a real --device, unless \`cd ${REPO} && cargo run -q -p dmm-cli -- --lang en-US --no-bluetooth list\` shows no cable at all — then you may pick a USB meter (with "Look for Bluetooth devices" off) to see the no-cable notice; otherwise skip that notice and say so in notes. Look for: truncated or overflowing text, a wrapped top bar, wrong or awkward wording, English left over (text that isn't in the ${LANG} files), wrong terms. Look up each message you flag in i18n/${LANG}/ to give its file and id. List the important screenshots in notes.`,
    { label: `review:${LANG}:gui`, phase: 'Review', schema: FINDINGS }),
  () => agent(`${REVIEW_COMMON}

Your part: the text that isn't on the GUI's main screens — dmm-cli output, help and errors, the shared help/setup text, library errors, lib.ftl (modes, ranges, activation steps, spec notes), and a read-through of every ${LANG} file for accuracy against its English. Run the CLI in ${NAME} by loading the files: \`cd ${REPO} && DMM_TOOLS_L10N_DIR=${REPO}/i18n cargo run -q -p dmm-cli -- --lang ${LANG} <args>\` with e.g. \`--help\`, \`-h\`, \`read --help\`, \`list\`, \`read --device mock --count 5\`, \`info --device mock\`, \`get\`/\`set\` help and errors, a missing command, an unknown option (\`read --fromat csv\`), a bad value (\`read --format xml\`), \`read --device mock-zt5b --count 3\`. Never pass a real device id (only mock*). Also run \`cargo test -q -p dmm-shared translations\` and \`cargo test -q -p dmm-shared long_labels -- --ignored --nocapture\` and report any ${LANG} problem. Read lib.ftl fully: modes stay short, qualifiers place { $mode } naturally, activation steps are correct with the meter's key names kept, spec notes keep every number. Judge the drafters' own doubts too:\n\n${draftReports}`,
    { label: `review:${LANG}:text`, phase: 'Review', schema: FINDINGS }),
])
const findings = reviews.filter(Boolean).flatMap(r => r.findings)
const reviewNotes = reviews.map((r, i) => `${i === 0 ? 'GUI' : 'text'} reviewer notes: ${r ? r.notes : '(failed)'}`).join('\n')
log(`${LANG}: ${findings.length} findings: ${findings.filter(f => f.severity === 'critical').length} critical, ${findings.filter(f => f.severity === 'major').length} major`)

phase('Fix')
const fix = await agent(`${COMMON}

You are now the fixer, for all of ${REPO}/i18n/${LANG}/. Two reviewers reported the findings below (JSON). Apply each critical and major finding, and each minor one unless it contradicts STYLE §0 or the glossary (then skip it and say why). Where two findings conflict, or a finding contradicts the glossary, keep the glossary and report it. Don't change messages nobody flagged. Then run the checks from the instructions above over ALL ${LANG} files (all of them are yours now) until \`translations\` reports nothing for ${LANG}.

Findings:
${JSON.stringify(findings, null, 1)}

${reviewNotes}

Return: what you applied (count), what you skipped and why, the final check result, and any reviewer note that needs code (layout) rather than a translation.`,
  { label: `fix:${LANG}`, phase: 'Fix', ...WRITE })

return { lang: LANG, draftReports, findings, reviewNotes, fix }
