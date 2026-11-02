# French style sheet (fr)

How the French translation is written. It goes with `glossary.tsv` in this folder: where the two disagree, the glossary wins for a term and this sheet wins for form. `cargo test -p dmm-shared translations` checks what can be checked mechanically ([development guide](../../docs/development.md#translations)).

## 0. Rulings

The short list. Where a section below gives the background, this list is what applies.

1. «vous». Buttons, chips, menu items and action hovers use the infinitive; notices and activation steps use the vous imperative.
2. U+00A0 before : ; ! ? and inside « », never U+202F (egui 0.37 breaks a line at U+202F).
3. Colons inside notation stay unspaced («Y:fixe», «Moy.=», «∫:»); real captions take U+00A0 («Min :»).
4. Numbers keep the English point everywhere, typed numbers in prose included («0.6 s»); no thousands separator.
5. U+00A0 before % in sentences; % in notation is written as the English writes it.
6. Meter flags stay as printed (HOLD, REL, LOW BAT, LEAD ERR, HV!, VOID…); their spoken forms are French.
7. The app's own badges are translated in capitals with accents: ÉCHELLE, LIM. HAUTE, LIM. BASSE, DIRECT, VÉRIFIÉ, PARTIELLEMENT VÉRIFIÉ, EXPÉRIMENTAL.
8. AC/DC/AC+DC stay as printed in mode labels and notes. Table titles and prose use full words, AC+DC included: «Tension alternative + continue». Never «V CC»/«V CA».
9. Glossary entries are full terms, with no optional words in parentheses: «maintien de l'affichage», «bip sonore», «valeur efficace sinusoïdale», «plage de mesure». A parenthesis in an entry is part of the text (an abbreviation or unit kept, as in «gain en courant (hFE)», «puissance apparente (VA)»).
10. A unit symbol the English gives next to a term stays («puissance apparente (VA)»).
11. Terms ruled without an approved source: «gain en courant (hFE)» ("Transistor Magnification" → «Gain en courant du transistor (hFE)»), «Daltonien» (Colorblind preset). The other unsourced terms take the glossary's value as drafted.
12. macOS names (Apple's French support pages): «Réglages Système > Confidentialité et sécurité > Surveillance de l’entrée», and «… > Bluetooth». Windows: «Gestionnaire de périphériques» (MS Terminology), «Périphériques d’interface utilisateur» and «Autres périphériques» (Microsoft's French support pages). The French name of 'USB Input Device' is unverified: its message carries a `# UNVERIFIED Windows name` comment until someone checks a French Windows.
13. Spoken unit names (`reading-unit-*`, read aloud only): «volt», «ampère», «ohm», «hertz», «farad», «siemens», «watt», «voltampère», «henry» (pluriel «henrys»), «seconde», «pour cent»; prefixes «pico», «nano», «micro», «milli», «kilo», «méga», «giga», joined whole with no contraction («kiloohm», «mégaohm»). Ruled by the user; ohm, watt and voltampère follow VC920 PDF p.71–72.

## 1. Sources

Every rule and term cites one of these. PDF page numbers are the PDF's own page index, not the number printed on the page. The `references/` paths are the maintainer's copies, kept out of the repo; the URLs and Conrad item numbers find them again.

| Key | Document | Where |
|---|---|---|
| VC831, VC851, VC871, VC891 | Voltcraft VC831 / VC851 / VC871 / VC891 operating instructions (DE/EN/FR/NL), Conrad items 2576864 / 2576865 / 2576867 / 2576866 | `references/owon/voltcraft/002576864ML00.pdf`, `002576865ML01.pdf`, `002576867ML01.pdf`, `002576866ML00.pdf` (FR from PDF p.89–110) |
| VC915, VC925 | Voltcraft VC915 and VC925 PV operating instructions, Conrad 3072347 / 3072348 | `references/owon/voltcraft/003072347ML00.pdf`, `003072348ML00.pdf` |
| VC880 | Voltcraft VC880 operating instructions, Conrad 124609 (FR PDF p.66–95) | `references/vc880/manuals/manual-124609-voltcraft-vc880-…pdf` |
| VC650 | Voltcraft VC650BT instructions, Conrad 124411 (FR PDF p.75–109) | `references/vc880/manuals/user-safety-instructions-124411-voltcraft-vc650bt-…pdf` |
| VC920 | Voltcraft VC920/940/960 operating instructions 2009, Conrad 123296 (FR PDF p.63–91) | `references/vc920/manuals/vc920-940-960-operating-instructions-2009-with-note-01-13-de-en-fr-nl-123296ML04.pdf` |
| Voltsoft | Voltcraft Voltsoft PC software user manual, French | `references/vc880/vendor-software/vc880-extracted/User Manual/Voltsoft User Manual - French.pdf` |
| UT197 | UNI-T UT197 Manuel d'utilisation (French) | https://meters.uni-trend.com/download/ut197-user-manual-french/ (`references/l10n/fr/`) |
| MS-SG | Microsoft French (France) localization style guide | https://aka.ms/french-france-styleguide (fra-fra-StyleGuide.pdf, 53 pages) |
| MS-T | Microsoft Terminology Collection, FRENCH.tbx, entries marked FRA | https://download.microsoft.com/download/b/2/d/b2db7a7c-8d33-47f3-b2c1-ee5e6445cf45/MicrosoftTermCollection.zip |
| IEV | IEC Electropedia | electropedia.org returned 403 to every fetch. The IEV numbers cited come from the search engine's index of those pages, so open them in a browser before relying on them. |
| Apple | Apple support, Mac User Guide in French | https://support.apple.com/fr-fr/guide/mac-help/mchl4cedafb6/mac, https://support.apple.com/fr-fr/guide/mac-help/mchl211c911f/mac |
| egui | epaint 0.37.0 source and its bundled fonts (`epaint-0.37.0`, `epaint_default_fonts-0.37.0`) | for how the GUI draws spaces and glyphs |

Not used: other apps' translations, including the vendor apps' own French string files (`references/vc880/.../french.txt`, the ZOTEK APKs' `values-fr`, Qt's `qt_fr.qm`), community translations and machine translation.

The UT197 manual's French is uneven. For example, it calls the meter a «compteur» and has a «cycle d'occupation». The glossary uses it only where Voltcraft says nothing, and says so where it rejects one of its terms.

## 2. Register and voice

- **Address the user as «vous», in the masculine singular** (MS-SG p.24 §4.1.8 «Vous êtes connecté», p.27 §4.1.13). Never «tu», and never «on» (MS-SG p.10).
- **Buttons, menu items, checkboxes, chips and shortcut-list actions:** use the infinitive, or a noun where English uses one: «Connecter», «Exporter…», «Réinitialiser le zoom», «Toujours visible», «Basculer l'enregistrement». This follows MS-SG's own UI examples (p.47–48 «Annuler», «Enregistrer sous», «Copier»). Within one list, use the same form throughout: all infinitives or all nouns (MS-SG p.29).
- **Hovers that name what a control does** (English: "Open USB connection…", "Freeze the current reading") use the infinitive and stay short: «Ouvrir la connexion USB au multimètre sélectionné (Ctrl+O)», «Figer la mesure affichée». A hover names the action in a few words.
- **Instructions in notices, steps and sentences** use the vous imperative: «Vérifiez que le bon appareil est sélectionné dans Paramètres (⚙)», «Allumez le multimètre». MS-SG p.11 «Vérifiez-la et recommencez», p.13 «cliquez sur Annuler». The Voltcraft manuals word their steps the same way: VC915 PDF p.140 «Activez le Bluetooth sur le lecteur. Appuyez sur le bouton « BLE » et maintenez-le enfoncé».
- **Activation steps** (lib.ftl `activation-*`) are numbered imperatives: «1. Branchez le câble USB sur le multimètre», «2. Allumez le multimètre», «3. Maintenez la touche SELECT enfoncée…».
- **Use everyday words** (MS-SG p.8–10): «pour», not «afin de»; «pendant», not «lors de»; «quand», not «lorsque»; «faire», not «réaliser»; «demander», not «requérir». Use the simple present and the passé composé, never the passé simple (MS-SG p.36).
- **Error messages** use MS-SG's standard phrases (p.41–42): "Cannot …" → «Impossible de …»; "Failed to …" / "… failed" → «Échec de …» («Échec de l'importation : …», «Échec de l'export : …»); "Cannot find" / "not found" → «… introuvable» («Adaptateur introuvable», «appareil introuvable»); "… is not available" → «… n'est pas disponible». For "we couldn't", MS-SG p.10 prefers «Nous n'avons pas pu» to «Impossible de»; this app rarely says "we".
- **Reasons the app builds into a sentence** (`input-*`, `read-error-*`, `lib-errors` details) stay lowercase with no full stop, as the en-US comments ask. Those comments override MS-SG's "always end an error message with a period" (p.41).

## 3. Capitalization

- **Sentence case everywhere** (MS-SG p.22 §4.1.5): only the first word and proper nouns take a capital. English Title Case loses its capitals: "Reset Zoom" → «Réinitialiser le zoom», "Keyboard & Mouse" → «Clavier et souris», "High Contrast" → «Contraste élevé», "What's New" → «Nouveautés».
- **Capitals keep their accents:** «ÉCHELLE», «VÉRIFIÉ», «EXPÉRIMENTAL», «Échec». Ubuntu-Light, the GUI's text face, has É, À, Ç, Œ and «» (checked against the bundled TTF, egui).
- **The meter's annunciators and key names stay in capitals as printed** (§7). The app's own badges, which imitate them, are translated and kept in capitals: «ÉCHELLE», «LIM. HAUTE», «DIRECT».
- **Text after a colon** starts lowercase unless it is a full quotation (MS-SG p.29).
- **Acronyms** (USB, HID, CSV, JSON, LCD, NCV) take no periods and no plural «s» (MS-SG p.20, p.26: «des PC»).
- **Language names** in running French are lowercase («le français»). The Language list shows each language's name in itself as a fixed entry («Français»), so leave the list alone.

## 4. Punctuation and spaces

### Non-breaking spaces

French puts a space before **: ; ! ?** and inside **« »** (MS-SG p.33 §Nonbreaking space; p.32 for the guillemets). MS-SG also puts one between a number and its unit, as a thousands separator (not used here, see §5) and before **%**.

**Which character: U+00A0 (NO-BREAK SPACE), everywhere: GUI, CLI and spec text.**

- MS-SG specifies U+00A0 (p.32: "CTRL+SHIFT+SPACEBAR or Alt+0160").
- **egui 0.37 wraps at U+202F.** epaint's line breaker exempts only U+00A0 from its list of break points (`text_layout.rs:1415`, `const NON_BREAKING_SPACE: char = '\u{A0}'`). U+202F is Unicode whitespace, so egui may end a line just before «:», «?» or «»». That is exactly the break a non-breaking space exists to stop.
- **egui draws U+202F as half a space, not a box.** It borrows the space glyph and halves its advance (`font_face.rs:508`, `text_layout.rs:271`), so the narrow space would *look* right. Of the bundled faces, only Hack has its own U+202F glyph.
- In a terminal, both characters take one cell. U+00A0 is the one every font has.

Imprimerie nationale typography would put the narrow U+202F before ; ! ? and inside « ». Use U+00A0 throughout until egui keeps U+202F from breaking.

**Writing it in `.ftl` files.** Fluent's syntax counts only U+0020 as blank (fluent-syntax), so a literal U+00A0 is ordinary text and survives parsing, including next to a placeable: `{ $series } : { $value }` with U+00A0 before the colon. Type it literally. The `{" "}` escape is legal Fluent but unreadable.

**Where the rule bends.** A space before the colon is still French punctuation, so keep it in compact captions: «Min :», «Max :», «Moy. :», «σ :», «Appareil :», «Tracer :», «Afficher :». The exceptions are notation, which follows the English exactly:
- `Mean={ $value }` → `Moy.={ $value }`, `Readings=` → `Mesures=`, `ΔT=`, `σ=`, `∫:` (graph-readout-*, stats-integral; their comments say "no spaces around =" and "stay as they are");
- `Y:Fixed` / `Y:Auto` → «Y:fixe», «Y:auto»: chips set in a tight row;
- ratios such as `{ $ratio }:1` (colors-issue-low-contrast);
- the meter's menu paths (`SETUP -> Communication -> ON`) and anything else the meter prints.

### Quotes, apostrophe, ellipsis, dashes

- **Quotation marks:** use « » with U+00A0 inside (MS-SG p.32). For a quote inside a quote, use “ ” (MS-SG p.33). Translate the English's "…" and “…” to « ». The `‘…’` and `'…'` around command lines and Windows device names are different: keep them as they are (§8).
- **Names of on-screen controls:** MS-SG p.33 drops quotation marks around UI names ("Click the "Delete" button" → «Cliquez sur le bouton Supprimer»). The app's text is plain, without bold, so quotes are the only marker it has. Follow the English: where it quotes a name ("Click "Connect" after resolving…", «Tick "Look for Bluetooth devices"»), quote it with « ». Where it doesn't (Settings, Export…, Record), add nothing. The Voltcraft manuals quote key names too: «appuyez sur la touche « BLE »» (VC891 PDF p.132).
- **Apostrophe:** use the typographic ’ (U+2019) in French text (MS-SG p.28). Keep straight quotes inside command lines and code.
- **Ellipsis:** always the single character … (U+2026) (MS-SG p.31), including where the English has "..." («Reconnexion…»). It goes straight after a word with no space: «Exporter…», «Importer un fichier…».
- **Em dash (—):** MS-SG p.27–31 asks for French to replace it with a comma, a colon or parentheses. The English uses it a lot in hovers and toasts ("Halt acquisition — stops recording and graph updates"). Rephrase: «Suspendre l'acquisition : arrête l'enregistrement et la mise à jour du graphique». Where a pause has to stay, use an en dash with spaces (« – »), and U+00A0 before it if the line must not break there.
- **Semicolon:** MS-SG p.27 avoids it: make two sentences, or use a comma. Spec notes that list conditions with ";" may keep it, with U+00A0 before it (` ; `). The screen-reader separator `{"; "}` (graph-histogram-a11y-interval-separator) becomes `{" ; "}` with U+00A0, or `{", "}`.
- **&** becomes «et» (MS-SG p.18, for screen readers too): "Clear graph & statistics" → «Effacer le graphique et les statistiques». The "~" in spec ranges (`10V~36V`, `50Hz ~ 60Hz`) is notation: keep it.

## 5. Numbers

**The app keeps the English decimal point in every number it formats itself**: readings, statistics, durations (`1.5 h`), sample counts (`1.2K`, `500K`), memory (`≈194 MB`), percentages in the histogram and zoom chips (`12.5%`, `150%`). `docs/localization-design.md` ("Numbers keep their point") gives the reasons: the meter's LCD shows a point; input fields parse only a point; and the reference-values field reads a comma as a *separator*, so `1,5` is two values.

That leaves French prose next to English-formatted numbers: «Conserver jusqu'à 500K échantillons, ≈194 MB, environ 14 h». So:

1. **Write the numbers you type in prose with a point too**, as the English does: «en moins de 0.6 s» (activation-bm78xbt), «Ib0 ≈1.8µA». A reading in a placeable a few words away will have a point, and two conventions in one line look broken. **Typed examples must keep the point**: the hover `(e.g. 3.3, 5, 12)` (graph-ref-values) shows the user what to type, and a French «3,3» would be read as two values.
2. **Spec text:** the spec check accepts a decimal comma («0,6» matches 0.6), but recommendation 1 still holds there. The same panel shows the library's own figures with points (`600.0mV`, `±(0.5%+5)`).
3. **No thousands separator, ever.** Write `1000 V`, `40000`, as the English does. The spec check rejects «1 000», and that note then shows in English (`docs/localization-design.md`, "Spec numbers survive translation"). This departs from MS-SG p.33 and from the Voltcraft manuals («1 000 V»).
4. **Integers in prose:** MS-SG p.26 spells out zero to nine («cinq minutes»). The English writes digits ("5 minutes", "2 s"). Keep digits wherever a unit symbol follows, and next to the meter's own figures ("17 minutes", "0.6 s", "2 seconds"). Spelling out is fine elsewhere: «deux bips».
5. **Gaps the app leaves (for the developers, not the translators):** `MB`/`KiB` come preformatted, where French writes Mo/Kio (MS-SG p.18–19 «200 Mo», «Go»); the `K` suffix for thousands (`500K`); and the time units in `≈194 MB, about 14 h`. These show in English units until Rust formats them per locale. Translators leave the placeables as they are.

**Percent.** French puts U+00A0 before % (MS-SG p.33; Voltcraft «0,5 %», VC925 PDF p.157 «5 à 100 %»):
- in sentences and hovers, put U+00A0 before % («{ $percent } %», «1 %»);
- in notation, follow the English: spec bands and notes («10%–100%», which the spec check compares ignoring spacing), the zoom and histogram figures that arrive formatted, and `Duty %`.

## 6. Units and AC/DC

- **Unit symbols never change,** except in the spoken reading's unit names (§0 ruling 13): V, mV, A, mA, µA, Ω, kΩ, MΩ, Hz, F, °C, °F, W, VA, Wh, Ah, dBm, dBV, s, ms, h, V·s, mAh. `lib.ftl` doesn't even list labels made only of units (`200mV`, `kΩ`).
- **Space between a number and its unit:** follow the English as written. Spec text keeps the manual's own spacing (`10MΩ`, `≈3V`), which the spec check ignores. In prose that you write, use U+00A0 («2 s», «10 MΩ») (MS-SG p.33).
- **The meter's AC/DC labels stay as the meter prints them:** `DC V`, `V AC`, `AC mA`, `AC+DC`, `ACV`, `DCV`, `LoZ V~`. `lib.ftl` allows this ("you may keep abbreviations such as DC V, LPF or REL"), and they match the LCD and the meter's own legends.
- **In prose and table titles, spell the words out:** «tension continue», «tension alternative», «courant continu / alternatif» (VC831 PDF p.91, VC920 PDF p.65), and «tension alternative + continue» for AC+DC.
- **No «V CC» / «V CA».** Voltcraft's French uses them in running text («1 000 V CC», VC920 PDF p.65; «tension CC», VC925 PDF p.111), but mixes them with «V DC», «mV DC» and «ACV» on the same pages. UNI-T's French keeps AC/DC (UT197). Keep the English DC/AC abbreviations in labels and notes, «AC+DC» included, and never write CC/CA. A note such as "AC+DC: add 1%+35 digits" would otherwise put «CA+CC» beside a mode line that says `AC+DC`.
- **True RMS:** in notes, «valeur efficace vraie (True RMS)», or keep «True RMS» as Voltcraft does (VC920 PDF p.65). "rms" after a number (`250V rms`) is notation: keep it.

## 7. What the meter prints stays as it prints it

The French manuals print every annunciator and key name exactly as it appears on the meter and explain it in French beside it. Examples: «(3) Indicateur HOLD actif» (VC831 PDF p.97), «12.1 RANGE», «Fonction SELECT», «Fonction MAX/MIN» (VC831 PDF p.92), «OL Overload = Surcharge» (VC891 PDF p.114), «AUTO désigne la "Sélection automatique de la plage de mesure"» (VC920 PDF p.71). Only VC925 translates a key once («Fonction PLAGE», PDF p.111).

So:
- **Status flags** (`lib-names.ftl`) stay as they are: HOLD, REL, AUTO, MIN, MAX, AVG, LOW BAT, HV!, P-MAX, P-MIN, LEAD ERR, COMP, REC, LoZ, VOID. LOW BAT, LEAD ERR, HV! and VOID are the app's text for an icon or a state, not text every meter prints; they stay too, so the flag row stays one alphabet. The spoken forms (`reading-spoken-*`) are French: «maintien», «plage automatique», «pile faible», «erreur de branchement des cordons».
- **Key names, display text and menu paths in activation steps and hovers**, as the `lib.ftl` comments require: SELECT, SEL, ZERO/ᛒ, Hz/Duty△/ᛒ, Tab⇌, △/ᛒ, 1ms PEAK, MAX MIN, SEND, RS232, PC, BLE, SETUP -> Communication -> ON, «Org», dSAPO, APO.oF, BT, ((D)), «the S icon». Translate the words around them: «Maintenez la touche Hz/Duty△/ᛒ enfoncée jusqu'à ce que l'icône ᛒ s'affiche».
- **Mode qualifiers:** keep LPF and REL (`qualifier-lpf-mode = LPF { $mode }`, `qualifier-mode-rel = { $mode } REL`). Translate "Peak": «Crête { $mode }» / «{ $mode } crête».
- **Brands, model names and trademarks:** UNI-T, Voltcraft, ZOTEK, ZOYI, BSIDE, ANENG, EEVblog, Brymen, OWON, the part numbers, AutoCheck™. "Mock UT61E+" → «UT61E+ simulé»: keep the part number, translate the word.

## 8. Command lines, options, files and keys

- **Verbatim:** every `--flag` and its values (`--device mock`, `--adapter`, `--format replay`, `--mock-mode`, `--theme`); `dmm-cli`, `dmm-gui`, subcommands (`list`, `read`, `capture`, `get`, `set`) and the quoted command lines (`'dmm-cli list'`, `--device 'ut61e+'`); device and step ids (`ut61eplus`, `dcmv,temp,duty`); setting ids as the CLI takes them (`mode`, `range`, `hold`, `rel`, `minmax`, `peak`, `auto`, `system`); settings-file keys (`device_family`, `mock_mode`, `language`, `bluetooth`); environment variables (`RUST_LOG`, `NO_COLOR`, `LANGUAGE`, `LC_ALL`, `LANG`); file names and patterns (`capture-<device>.yaml`); CSV column names and JSON keys (`timestamp`, `_metadata`); the backquoted text in `read-error-*`; RFC 3339; the password `0000`. The translation check fails a message that loses a `code` span or an `--option`.
- **clap's words** (headings, `[par défaut : …]`, the built-in flags, command-line errors) are ours too (`cli/help.ftl`, `cli/args.ftl`). Value names in capitals (`<FORMAT>`) and the usage line's syntax stay as clap writes them; in errors, keep the names in their straight quotes, as typed.
- **Windows and macOS names** in the setup help must match what the French OS shows (shared.ftl comments): §0 item 12. `'CP2110 USB to UART Bridge'` is the device's own name and stays English.
- **Computer key names** are messages of their own (`shortcuts-key-*` in `gui/shortcuts.ftl`), which the shortcut list and every hover naming a shortcut use: Ctrl, Maj, Alt, Espace, Échap, Gauche/Droite, Origine, Fin, Plus/Moins (MS-SG p.45–47). Prose names keys the same way, adding Haut/Bas and Pg préc/Pg suiv: «Maj + glisser», «appuyez sur Échap». Letters, digits, F1–F12 and macOS's ⌘ ⇧ ⌥ symbols stay as they are.

## 9. Length budget

French runs about 15–30 % longer than English. `cargo test -p dmm-shared long_labels -- --ignored --nocapture` lists short labels that grow past 1.5×. Watch most closely, in order:
1. **The top bar** (one row on a wide window): Connect, Disconnect, Resume, Pause, Clear, Reconnecting…, the status, the badges, Help & Feedback, and the update link. «Déconnecter» (11 characters, against 10) is fine; «PARTIELLEMENT VÉRIFIÉ» is 21 characters against 15 (1.4×).
2. **Graph chips** (tight rows): Mean, Min/Max, Histogram, Ref, Triggers, Cursors, LIVE, Y:Fixed, Reset Zoom, Plot:, Show:. Prefer «Moyenne», «Réf.», «Curseurs», «DIRECT».
3. **The mode line under the reading** wraps on a narrow window. Mode labels and badges belong here: «Rapport cyclique %» is the risky one: keep `Duty %` if it wraps.
4. **The Statistics column** right-aligns to the widest of Min, Max, Mean and σ. «Moyenne :» (9 characters against 5) is acceptable; «Moy. :» is the fallback.
5. **Settings chips** wrap; their captions end in « :».

When room is short, abbreviate as MS-SG p.18 describes: cut after a consonant, end with a period, and never add a plural «s» (Préc., Rés., Moy., Réf., Lim., Nb). Never drop a word the user needs to act. In a hover, a sentence may grow.

## 10. Agreement around placeables

Fluent can't tell you the gender or number of what a placeable holds. So build sentences in which nothing has to agree with it:

- **Put a noun in front:** «le multimètre { $model }» (also saves choosing between «le UT61E+» and «l'UT61E+»); «en mode { $mode }»; «sur la plage { $range }»; «la courbe { $series }»; «la valeur secondaire { $series } ({ $unit })»; «le réglage { $setting }»; «la touche { $key }».
- **Agree with that noun, never with the placeable:** «Le multimètre est maintenant en mode { $choice }», not «…est en { $choice }»; «La prise en charge du multimètre { $model } est vérifiée».
- **Use a label-colon form for status lines:** «Plage : { $range } (manuelle)», «Alarme activée : { $limits }».
- **`$link`, `$side`, `$state`, `$saved`, `$case` and the other selectors** have one variant per value. Make each variant agree with its own noun: [usb] «Le câble USB est connecté», [adapter] «L'adaptateur Bluetooth est connecté», [radio] «La liaison Bluetooth est établie».
- **`$setting` in `connection-switch-failed`** is a lowercase CLI id (hold, rel, minmax, peak). Write «Échec du changement de { $setting } : { $error }»: the id stays as it is and no article meets it.
- **Inserted lowercase reasons** (`$reason`, `$error`, `$detail`) may be French, or English from the OS or a parser (comments say which). Put them after a colon, never inside a sentence that expects a gender.
- **Participles** that refer to the user are masculine (MS-SG p.24).

**Plurals.** French `one` covers 0 and 1: «0 échantillon», «1 échantillon», «2 échantillons». The plural rules fluent-bundle uses give French only `one` and `other`, and the translation check asks for exactly the forms they give. Write `[one]` and `*[other]` only; if the rules ever gain the newer CLDR's `many` (exact millions), give it the `*[other]` text. Unit symbols never take an «s»; words do («3 secondes», «2 bips»).

## 11. Fluent specifics

- **Never change** message IDs, attribute names (`.hover`, `.a11y`, `.hover-disabled`, `.wayland`, `.gesture`, `.about`, `.help`, `.value-replay`), variable names (`$keys`, `$model`…) or selector keys (`[one]`, `[usb]`, `[yes]`, `[big-meter]`…). Keep the `*` default on the same variant. Translate every variant's text.
- **You may** reorder placeables, drop a placeable the French doesn't need, add a selector on a variable that exists, or turn a plain message into one with variants. Nothing in the data gives gender, so don't invent gender selectors.
- **Message references stay references:** `{ project-link }`, `{ device-auto }`, `{ bluetooth-setting }`, `{ error-bluetooth-reset-gesture }`. Translate the referenced message, not the reference.
- **String literals:** `{" "}`, `{", "}`, `{"  "}` (two leading spaces in `connect-not-identified-name-it`) are layout. Keep their meaning; adapt the separator where French punctuation differs.
- **Multi-line messages:** keep the indentation and the line breaks. The notices are hard-wrapped for a narrow column (connection.ftl), and the activation steps are one step per line. A wrapped notice line may move a word or two if the French runs longer.
- **`.a11y` text** is read aloud, never drawn. Write it as speech: units in words where the comments ask («degrés C»), «et» for «&», no abbreviations.
- **Trailing and leading spaces are dropped** by Fluent. Where one matters, write it as `{" "}`.
- **Text never translated:** the `###`/`#` comments (translator help, never shown), and `settings-language-no-chinese-font`, which its comment says is always shown in English.

## 12. Spec text (lib.ftl: bands, inputs, notes)

- Keep every number, sign, ±, ≤, ≥, <, >, ≈ and ~, and each unit symbol right after its number. Write numbers as in the English (§5). The spec check compares them and falls back to English on any mismatch.
- Keep the exact sense: comparisons, ranges, «typique» / «typ.», «nominal», «environ», «au moins», «max.». The check can't catch a flipped comparison, and the panel shows the English on hover wherever the text differs.
- Use the maker manuals' wording: «Non spécifié»; «de la mesure» (accuracy); «points» (digits/counts); «protection contre les surcharges»; «impédance d'entrée»; «facteur de crête»; «coefficient de température»; «cordons court-circuités»; «circuit ouvert»; «sensibilité». See the glossary for each source.
- Table titles keep the manual's numbering and letters: «(1) Tension alternative», «A. Tension continue», «K. Température 1-1. Degrés Celsius».

## 13. Words to avoid

| Avoid | Use | Why |
|---|---|---|
| compteur (for the meter) | multimètre | calque in UT197 FR |
| voltage | tension | anglicism |
| capacitance | capacité | anglicism (UT197 FR) |
| app | application | MS-SG p.7 |
| scanner / numériser (Bluetooth) | rechercher | MS-T scan => numériser is for paper |
| panoramique (pan) | faire défiler | camera sense |
| lecture (replay) | relecture | «lecture» also means a reading in some manuals |
| échantillon (colour swatch) | — (the swatches are named, not called swatches) | «échantillon» is taken by sample |
| CC / CA | DC / AC as printed | consistency with the mode labels (§6) |
| il faut, on, il y a | vous + verb | MS-SG p.10 |
