# Brazilian Portuguese style sheet (pt-BR), draft

How the Brazilian Portuguese translation is written. It goes with `glossary.tsv` in this folder: where the two disagree, the glossary wins for a term and this sheet wins for form. `cargo test -p dmm-shared translations` checks what can be checked mechanically ([development guide](../../docs/development.md#translations)).

**Status: draft, no native reviewer yet.** Every ruling in §0 is provisional: it is the recommended choice, with the alternative in `SUMMARY.md`. Rulings marked *(decided)* were made for every language before this draft and are not open.

## 0. Rulings (all provisional unless marked decided)

1. **«você», never «tu».** Buttons, chips, menu items, checkboxes and action hovers use the infinitive («Conectar», «Exportar…», «Redefinir zoom», «Congelar a leitura atual»). Notices, error advice and activation steps use the você imperative («Verifique…», «Ligue o multímetro», «Gire a chave seletora…»).
2. **Sentence case** in every label, Title Case dropped («Redefinir zoom», «Alto contraste», «Teclado e mouse», «Ajuda e sugestões»). Microsoft's pt-BR guide copies the English capitals in UI strings (MS-SG PDF p.28); this draft does not (§3).
3. **No space before : ; ! ?**, ever (unlike French). Colons inside notation stay unspaced as in English («Y:fixo», «Média=», «∫:»); captions are «Mín.:», «Máx.:», «Média:».
4. **Quotation marks “ ”** (U+201C/U+201D), ‘ ’ inside them. Never « » (pt-PT and French use them; Brazilian text does not). Straight `'…'` around command lines and Windows device names stay as they are.
5. *(decided)* **English decimal point everywhere**, typed numbers in prose included («0.6 s», «3.3»); **no thousands separator** («1000 V», «60000», never «60.000», which reads as sixty here).
6. **No space before %**: «50%», «{ $percent }%», as in English and in most of Minipa's figures («±(0,5% + 3D)», ET-1649BT PDF p.24; «±10%», PDF p.25). ET-2517A/2615A PDF p.20 spaces it («de 10 % a 100 % da faixa»).
7. *(decided)* **Meter flags stay as printed** (HOLD, REL, AUTO, LOW BAT, LEAD ERR, HV!, VOID…); their spoken forms are Portuguese («congelamento da leitura», «faixa automática», «bateria fraca»).
8. **The app's own badges** are translated, in capitals with accents: ESCALA, LIM. SUP., LIM. INF., AO VIVO, VERIFICADO, PARCIALMENTE VERIFICADO, EXPERIMENTAL.
9. *(decided)* **AC/DC/AC+DC stay as printed** in mode labels and notes. Table titles and prose use full words: «Tensão contínua», «Tensão alternada», «Tensão alternada + contínua». Never «CC»/«CA» (Minipa writes AC/DC too: ET-1649BT PDF p.8 «Corrente Contínua (DC)»).
10. *(decided)* **Glossary entries are full terms**, no optional words in parentheses. A parenthesis in an entry is part of the text («ganho do transistor (hFE)», «potência aparente (VA)»).
11. **Plurals:** the plural rules fluent-bundle uses give Portuguese `one` for **0 and 1**. Brazilian Portuguese puts zero in the plural («0 amostras»). Where a count can be 0, add a numeric `[0]` variant with the plural text (§10); the translation check ignores numeric keys.
12. **Gender-neutral toward the user:** no adjective or participle agrees with the user. Rephrase («Conexão estabelecida», not «Você está conectado»), as MS-SG PDF p.19–20 asks.
13. **Key names** (`shortcuts-key-*`): Ctrl, Shift, Alt, Espaço, Esc, Esquerda, Direita, Home, End, Mais, Menos. In prose, MS's full names: «Seta para cima», «Seta para a esquerda», «Barra de espaços», «Page Up» (MS-SG PDF p.47–49).
14. **Terms ruled without an approved source** (24 rows, listed in `SUMMARY.md`): take the glossary's value as drafted until a reviewer rules.
15. **OS names.** Windows: «Gerenciador de Dispositivos» (MS-T), «Dispositivos de Interface Humana» and «Outros dispositivos» (Microsoft's pt-BR support pages). macOS: «Ajustes do Sistema > Privacidade e Segurança > Monitoração de Entrada» and «… > Bluetooth» (Apple's pt-BR support pages). 'USB Input Device' stays **unverified** («Dispositivo de Entrada USB»): its message carries a `# UNVERIFIED Windows name` comment until someone checks a Brazilian Windows.
16. **Spoken unit names** (`reading-unit-*`, read aloud only): «volt», «ampère», «ohm», «hertz», «farad», «siemens», «watt», «volt-ampère», «henry» (plural «henrys»), «segundo», «por cento»; prefixes «pico», «nano», «micro», «mili», «quilo», «mega», «giga», joined whole with no contraction («quiloohm», «megaohm»). The plural comes from the number as displayed, by CLDR's rule: singular below 2 («1.5 volt», «0.000 volt»), plural from 2 («2.0 volts»). No `[0]` variant: a Fluent number key never matches a decimal such as 0.000.

## 1. Sources

Every rule and term cites one of these. PDF page numbers are the PDF's own page index (as `pdftotext` counts pages), not the number printed on the page. The files are in this folder's parent, `references/l10n/pt-BR/` (kept out of the repo); `SOURCE.txt` there gives each URL.

| Key | Document | File |
|---|---|---|
| Minipa ET-1649BT | Minipa ET-1649BT Manual de Instruções, Multímetro Digital com Bluetooth, rev. 03, 18/12/2025 (36 pages, PT/EN/ES; PT on PDF p.1–32) | `minipa-ET-1649BT-1103-BR.pdf` |
| Minipa ET-2517A/2615A | Minipa ET-2517A / ET-2615A Manual de Instruções, ver. 02, 12/10/2011 (25 pages; a Brymen BM820s-series rebrand, the BU-86X cable) | `minipa-ET-2517A-2615A-1102-BR.pdf` |
| Minipa ET-2587 | Minipa ET-2587 Manual de Instruções (32 pages) | `minipa-ET-2587-1100-BR.pdf` |
| Minipa ET-2990 | Minipa ET-2990 Manual de Instruções (38 pages; loop calibrator multimeter) | `minipa-ET-2990-1103-BR.pdf` |
| MS-SG | Microsoft Portuguese (Brazil) Localization Style Guide (56 pages, 2024) | `microsoft-por-bra-styleguide.pdf` |
| MS-T | Microsoft Terminology Collection, `PORTUGUESE (BRAZIL).tbx`, entries marked BRA | `microsoft-terminology-PORTUGUESE-BRAZIL.tbx.gz` |
| Apple | Apple support, Mac User Guide in Brazilian Portuguese: mchl4cedafb6 (Monitoração de Entrada), mchl211c911f (Privacidade e Segurança) | `apple-mac-help-*-pt-br.html` |
| IEV | IEC Electropedia | electropedia.org returned 403 to every fetch (curl and WebFetch, 2026-11-01). No IEV term is cited. |
| egui | epaint 0.37 and its bundled fonts | for how the GUI draws text; see the French sheet §4 |

Not used: other apps' translations, vendor app string files, community translations and machine translation. No UNI-T or Brymen manual in Portuguese was found; Minipa's are the maker manuals in this language.

The four Minipa manuals are uneven too: the older ones write «freqüência» (pre-2009 spelling), «curto circuitadas» without its hyphen, «60.000 contagens» with a thousands point, and keep English terms («Duty Cycle», «True RMS», «offset», «Autorange»). The glossary takes their electrical vocabulary and says where it departs from them.

## 2. Register and voice

- **Address the user as «você»** (MS-SG PDF p.15 «Address the user directly using second person pronoun», p.31 «address the user as "você" instead of … "usuário"»). Never «tu», never «o usuário», never the passive to avoid «você» (MS-SG PDF p.31). Omit «você» where the verb already says it: «Se quiser…», not «Se você quiser…» every time (MS-SG PDF p.31 «avoid redundancy of pronouns»).
- **Buttons, menu items, checkboxes, chips and shortcut-list actions:** the infinitive, or a noun where English uses one: «Conectar», «Exportar…», «Redefinir zoom», «Sempre visível», «Alternar a gravação». MS-SG's UI examples: PDF p.28 «Adicionar a Contatos», p.49–51 «Cancelar», «Salvar como», «Copiar», «Selecionar Tudo». Within one list, keep one form (all infinitives or all nouns).
- **Hovers that name what a control does** use the infinitive and stay short: «Abrir a conexão USB com o multímetro selecionado (Ctrl+O)», «Congelar a leitura atual».
- **Instructions in notices, steps and sentences** use the você imperative (3rd-person form): «Verifique se o dispositivo certo está selecionado em Configurações (⚙)», «Ligue o multímetro». MS-SG PDF p.15 «Tente novamente», p.17 «clique em Cancelar», p.35 «Verifique se o .NET Framework foi instalado corretamente». Minipa words its steps the same way: ET-1649BT PDF p.14 «1. Gire a chave seletora…», PDF p.22 «1. Ligue o instrumento 2. Mantenha pressionada a tecla “SEL”».
- **Activation steps** (lib.ftl `activation-*`) are numbered imperatives: «1. Conecte o cabo USB ao multímetro», «2. Ligue o multímetro», «3. Mantenha a tecla SELECT pressionada…».
- **Plain, everyday words** (MS-SG PDF p.7–9): «poder», not «ser capaz de»; «seguinte», not «subsequente»; «veja», not «consulte»; drop «quaisquer». No «Por favor» in UI text (Minipa uses it; MS voice is direct: PDF p.15).
- **Tenses** (MS-SG PDF p.35–36): simple present and pretérito perfeito; avoid the future unless the action really is future; never the gerund for a future action («estaremos verificando»). A status in progress («Reconectando…», «Procurando…») is fine. Avoid stiff subjunctives: "Make sure …" → «Verifique se …», not «Certifique-se de que … esteja» (MS-SG PDF p.35).
- **Possessives:** fewer than English (MS-SG PDF p.26): "Check your meter" → «Verifique o multímetro».
- **Error messages** (MS-SG PDF p.42–45), sentence case always (PDF p.28, p.42):
  - "Cannot …" / "Could not …" → «Não é possível …» + infinitive; «Não foi possível …» when the failure happened in the past (PDF p.44).
  - "Failed to …" / "… failed" → «Falha ao …» («Falha ao importar: …», «Falha ao exportar: …»); with a subject in mid-sentence, «… não pôde …» (PDF p.44).
  - "… not found" / "Cannot find" → «… não encontrado» (PDF p.43 «Arquivo não encontrado»): «Nenhum cabo USB encontrado», «Nenhum multímetro encontrado via USB ou Bluetooth».
  - "… is not available" → «… não disponível», verb dropped (PDF p.45).
  - "Not enough …" → «… insuficiente» (PDF p.45). "… occurred" → leave out (PDF p.44).
- **Reasons the app builds into a sentence** (`input-*`, `read-error-*`, `lib-errors` details) stay lowercase with no full stop, as the en-US comments ask.

## 3. Capitalization

- **Sentence case everywhere** *(provisional; SUMMARY question 2)*: only the first word and proper nouns take a capital. "Reset Zoom" → «Redefinir zoom», "Keyboard & Mouse" → «Teclado e mouse», "High Contrast" → «Alto contraste», "What's New" → «Novidades». Reasons: Brazilian orthography lowercases common nouns, and MS-SG PDF p.27 applies it everywhere except UI strings; its UI exception (PDF p.28, "the source (English) capitalization is followed … for localization simplicity") exists for strings translated out of context, which these are not (every message has a comment); and the English here mixes "Reset Zoom" with "Always on top", so copying it would carry the mix over. Error messages are sentence case under either rule (MS-SG PDF p.28).
- **Proper names of the OS keep their own capitals:** «Gerenciador de Dispositivos», «Ajustes do Sistema», «Privacidade e Segurança», «Monitoração de Entrada».
- **Capitals keep their accents:** «VERIFICADO», «LIM. INF.», «AÇÃO», «Não». Every pt-BR letter (á â ã à ç é ê í ó ô õ ú) is in Latin-1, which the GUI's bundled faces cover.
- **The meter's annunciators and key names stay in capitals as printed** (§7). The app's own badges, which imitate them, are translated and kept in capitals (§0 item 8).
- **Text after a colon** starts lowercase unless it is a full quotation or a proper name (MS-SG PDF p.33 «OBSERVAÇÃO: esta compra…»).
- **Acronyms** take no periods. MS-SG PDF p.23: acronyms take a plural «s» («os PCs»); here, write it only where the English has one («os LEDs»), never on a unit or flag.
- **Language names** in running text are lowercase («o português»). The Language list shows each language's name in itself as a fixed entry, so leave it.

## 4. Punctuation and spaces

- **No space before : ; ! ?** and none inside quotation marks or parentheses (MS-SG PDF p.34). The French sheet's no-break-space rules do not apply.
- **No-break space (U+00A0)** only where a line must not break: between a number you type and its unit in prose («2 s», «15 minutos») (MS-SG PDF p.29–30), and nowhere else. Notation and spec text follow the English spacing as written. egui keeps U+00A0 together; it may break at U+202F, so never use U+202F.
- **Quotation marks:** “ ” with ‘ ’ inside (§0 item 4). MS-SG PDF p.34 says to follow the source's quotes; its note on curly quotes is cut off mid-sentence, and Minipa prints “ ” (ET-1649BT PDF p.4 «os tópicos “Informações de Segurança”»). Translate the English's "…" and “…” to “…”.
- **Names of on-screen controls:** follow the English. Where it quotes a name ("Tick "Look for Bluetooth devices""), quote it with “ ” («Marque “Procurar dispositivos Bluetooth”»); where it doesn't (Settings, Export…, Record), add nothing (MS-SG PDF p.34 example «toque em "Mostrar IDs disponíveis"»).
- **Comma:** none before «e», «ou», «nem» closing a series (MS-SG PDF p.32–33): "mail, contacts, and tasks" → «emails, contatos e tarefas».
- **Ellipsis:** the single character … (U+2026), straight after the word: «Exportar…», «Reconectando…» (MS-SG PDF p.34: follow the source).
- **Dashes** (MS-SG PDF p.33): the en dash – for ranges and the minus sign; the em dash — only to set off an aside that the sentence doesn't need. The English uses "—" a lot in hovers and toasts ("Halt acquisition — stops recording and graph updates"): use a colon where it introduces an explanation («Suspender a aquisição: para a gravação e as atualizações do gráfico»), keep the em dash, spaced, where it sets off an aside.
- **&** becomes «e», screen-reader text included (MS-SG PDF p.22): "Clear graph & statistics" → «Limpar o gráfico e as estatísticas». The "~" in spec ranges (`10V~36V`) is notation: keep it.
- **Semicolon:** rare in UI text; spec notes that list conditions with ";" keep it, unspaced. The screen-reader separator `{"; "}` stays `{"; "}`.
- **Parentheses:** no space inside (MS-SG PDF p.34).
- **No double space after a period** (MS-SG PDF p.34).

## 5. Numbers

**The app keeps the English decimal point in every number it formats itself** *(decided)*: readings, statistics, durations (`1.5 h`), counts (`1.2K`, `500K`), memory (`≈194 MB`), percentages (`12.5%`). `docs/localization-design.md` ("Numbers keep their point") gives the reasons. Brazilian convention is the decimal comma (Minipa ET-1649BT PDF p.24 «9,999mV»), but Brazilian meter readers meet the point too: Minipa's own ET-2517A/2615A manual prints its specifications with points (PDF p.19–21 «9.999V», «0.15 x»).

1. **Write the numbers you type in prose with a point too**: «em menos de 0.6 s», «Ib0 ≈1.8µA». **Typed examples must keep the point**: the hover `(e.g. 3.3, 5, 12)` (graph-ref-values) shows what to type, and «3,3» would be read as two values.
2. **Spec text:** the spec check accepts a decimal comma («0,6» matches 0.6), but point 1 holds there too: the same panel shows the library's figures with points.
3. **No thousands separator, ever.** `1000 V`, `40000`. A thousands point («60.000», as ET-2587 PDF p.20 writes) reads as sixty in this app; a thousands space is rejected by the spec check (`docs/localization-design.md`, "Spec numbers survive translation").
4. **Integers in prose:** MS-SG PDF p.30 spells out one to ten in non-technical text and keeps numerals in technical text and labels. Keep digits wherever the English has them, and always before a unit ("5 minutes" → «5 minutos», "2 s" → «2 s»). Version numbers keep their periods: «Versão 0.10.0» (MS-SG PDF p.38).
5. **Gaps the app leaves (for the developers):** `MB`/`KiB` and the `K` suffix arrive preformatted in English inside Portuguese sentences. Brazilian Portuguese also writes MB and KB, so these read correctly. Leave the placeables as they are.

## 6. Units and AC/DC

- **Unit symbols never change** *(decided)*, except in the spoken reading's unit names (§0 ruling 16): V, mV, A, mA, µA, Ω, kΩ, MΩ, Hz, F, °C, °F, W, VA, Wh, Ah, dBm, dBV, s, ms, h, V·s, mAh. No plural «s» on a symbol (MS-SG PDF p.23).
- **Space between a number and its unit:** follow the English as written in notation and spec text (`10MΩ`, `≈3V`); in prose you write, put U+00A0 («2 s», «10 MΩ») (MS-SG PDF p.30).
- **The meter's AC/DC labels stay as the meter prints them** *(decided)*: `DC V`, `V AC`, `AC mA`, `AC+DC`, `ACV`, `DCV`, `LoZ V~`.
- **In prose and table titles, spell the words out** *(decided)*: «tensão contínua», «tensão alternada», «corrente contínua / alternada» (Minipa ET-1649BT PDF p.8), «tensão alternada + contínua» for AC+DC.
- **No «CC» / «CA».** Minipa writes AC/DC in every manual found (ET-1649BT PDF p.3 «Medida de Tensão AC/DC»; ET-2517A/2615A PDF p.3). A note "AC+DC: add 1%+35 digits" keeps AC+DC beside a mode line that says `AC+DC`.
- **True RMS:** keep «True RMS» as Minipa does (ET-2517A/2615A PDF p.5, ET-1649BT PDF p.23). "rms" after a number (`250V rms`) is notation: keep it.

## 7. What the meter prints stays as it prints it *(decided)*

Minipa prints every annunciator and key name as it appears on the meter and explains it in Portuguese beside it: «exibindo o símbolo “AUTO”» (ET-1649BT PDF p.12), «pressione a tecla “RANGE”» (PDF p.14), «Pressione a tecla HOLD momentaneamente» (ET-2517A/2615A PDF p.13), «se o display exibir “OL”, indica que a faixa foi excedida» (ET-1649BT PDF p.14), «Os indicadores “R” e “MAX MIN AVG*” do LCD acendem» (ET-2517A/2615A PDF p.13).

- **Status flags** (`lib-names.ftl`) stay: HOLD, REL, AUTO, MIN, MAX, AVG, LOW BAT, HV!, P-MAX, P-MIN, LEAD ERR, COMP, REC, LoZ, VOID. The spoken forms (`reading-spoken-*`) are Portuguese: «congelamento da leitura», «faixa automática», «bateria fraca», «alta tensão», «erro de conexão das pontas de prova», «leitura inválida».
- **Key names, display text and menu paths in activation steps and hovers**, as the `lib.ftl` comments require: SELECT, SEL, ZERO/ᛒ, Hz/Duty△/ᛒ, Tab⇌, △/ᛒ, 1ms PEAK, MAX MIN, SEND, RS232, PC, BLE, SETUP -> Communication -> ON, “Org”, dSAPO, APO.oF, BT, ((D)). Translate the words around them: «Mantenha a tecla Hz/Duty△/ᛒ pressionada até o ícone ᛒ aparecer».
- **Mode qualifiers:** keep LPF and REL (`LPF { $mode }`, `{ $mode } REL`). Translate "Peak": «Pico { $mode }» / «{ $mode } pico».
- **Brands, model names and trademarks** stay: UNI-T, Voltcraft, ZOTEK, ZOYI, BSIDE, ANENG, EEVblog, Brymen, OWON, Minipa, the part numbers, AutoCheck™. "Mock UT61E+" → «UT61E+ simulado».

## 8. Command lines, options, files and keys

- **Verbatim** *(decided)*: every `--flag` and its values, `dmm-cli`, `dmm-gui`, subcommands, quoted command lines (`'dmm-cli list'`, `--device 'ut61e+'`), device and step ids, setting ids as the CLI takes them (`hold`, `rel`, `minmax`…), settings-file keys, environment variables (`RUST_LOG`, `LANGUAGE`, `LC_ALL`, `LANG`), file names and patterns, CSV column names and JSON keys, the backquoted text in `read-error-*`, RFC 3339, the password `0000`. The translation check fails a message that loses a `code` span or an `--option`.
- **clap's words** (headings, `[padrão: …]`, `[valores possíveis: …]`, the built-in flags, command-line errors) are translated too *(decided)*: «Uso:», «Comandos:», «Argumentos:», «Opções:», «Exibir ajuda», «Exibir versão» (MS-T «usage» → «uso», «argument» → «argumento», «default» → «padrão», «subcommand» → «subcomando»). Value names in capitals (`<FORMAT>`) and the usage line's syntax stay as clap writes them; in errors, names stay in their straight quotes, as typed.
- **Windows and macOS names** must match what a Brazilian system shows: §0 item 15. `'CP2110 USB to UART Bridge'` is the device's own name and stays English.
- **Computer key names** are messages of their own *(decided)* (`shortcuts-key-*`): §0 item 13. ABNT2 keyboards print Ctrl, Alt, Shift (or ⇧), Esc, Home, End; MS-SG PDF p.47–48 keeps Alt, Shift, Esc, Home, End, Page Up/Down in English, writes «Barra de espaços» and «Seta para a esquerda», and keeps «Ctrl» inside shortcuts (PDF p.50 «Ctrl+Esc») though its key table says «Control». The key column needs short names, hence «Espaço», «Esquerda», «Direita»; prose uses MS's full names. Letters, digits, F1–F12 and macOS's ⌘ ⇧ ⌥ symbols stay as they are.
- **Gestures:** «Ctrl + roda de rolagem», «Shift + arrastar», «clicar com o botão direito», «clicar duas vezes» (MS-T).

## 9. Length budget

Brazilian Portuguese runs longer than English. `cargo test -p dmm-shared long_labels -- --ignored --nocapture` lists short labels that grow past 1.5×. Watch most closely, in order:

1. **The top bar** (one row on a wide window): «Conectar», «Desconectar», «Continuar», «Pausar», «Limpar», «Reconectando…», the badges, «Ajuda e sugestões» (17 for 15). «PARCIALMENTE VERIFICADO» is 23 characters for 15 (1.53×): the longest; fallback «VERIF. PARCIAL».
2. **Graph chips:** «Média», «Mín./Máx.», «Histograma», «Ref.», «Gatilhos», «Cursores», «AO VIVO», «Y:fixo», «Y:auto», «Redefinir zoom», «Plotar:», «Mostrar:».
3. **The mode line under the reading** wraps on a narrow window: keep `Duty %` as printed.
4. **The Statistics column** right-aligns to the widest of «Mín.:», «Máx.:», «Média:», «σ:».
5. **Settings chips** wrap; their captions end in «:».

When room is short, abbreviate as MS-SG PDF p.22–23 describes: end on a consonant, keep accents, end with a period, and never add an «s» to a unit or symbol (Máx., Mín., Ref., Lim., Sup., Inf., Qtde.). Never drop a word the user needs to act. In a hover, a sentence may grow.

## 10. Agreement around placeables

Fluent can't tell you the gender or number of what a placeable holds, and Portuguese contracts prepositions with articles («de» + «o» = «do», «em» + «a» = «na»), so a bare placeable after «de» or «em» forces a guess. Build sentences in which nothing has to agree with it (MS-SG PDF p.26–27 asks for an article before a placeholder product name; a noun in front gives the article something to agree with):

- **Put a noun in front:** «o multímetro { $model }», «do multímetro { $model }»; «no modo { $mode }»; «na faixa { $range }»; «a curva { $series }»; «o valor secundário { $series } ({ $unit })»; «a configuração { $setting }»; «a tecla { $key }».
- **Agree with that noun, never with the placeable:** «O multímetro está agora no modo { $choice }»; «O suporte ao multímetro { $model } está verificado».
- **Use a label-colon form for status lines:** «Faixa: { $range } (manual)», «Alarme ativado: { $limits }».
- **`$link`, `$side`, `$state`, `$saved`, `$case` and the other selectors** have one variant per value; make each agree with its own noun: [usb] «O cabo USB está conectado», [adapter] «O adaptador Bluetooth está conectado», [radio] «A conexão Bluetooth está estabelecida».
- **`$setting` in `connection-switch-failed`** is a lowercase CLI id (hold, rel, minmax, peak): «Falha ao alterar a configuração { $setting }: { $error }».
- **Inserted reasons** (`$reason`, `$error`, `$detail`) may be Portuguese or English (from the OS or a parser). Put them after a colon, never inside a sentence that expects a gender.
- **Nothing agrees with the user** (§0 item 12; MS-SG PDF p.19–20): use a noun phrase or an impersonal form.
- **Crase:** «à» where «a» + feminine article meet: «à faixa», «à rede elétrica»; none before a masculine noun or a verb.

**Plurals.** CLDR 37's rule for Portuguese, which fluent-bundle applies to pt-BR, puts integers 0 and 1 in `one` and everything else in `other` (`intl_pluralrules` 7.0.2, `rules.rs`). The app passes integers (`Arg::Int`), except to the spoken unit names (§0 ruling 16). Write `[one]` and `*[other]` as the check requires; where the count can be 0, add a `[0]` variant with the plural text, since Brazilian Portuguese says «0 amostras», «nenhuma amostra»:

```
{ $count ->
    [0] { $count } amostras
    [one] { $count } amostra
   *[other] { $count } amostras
}
```

`[0]` is a number literal: Fluent matches it before the plural category, and the translation check ignores number keys (`translations.rs`, `walk_expression`). Unit symbols never take an «s»; words do («3 segundos», «2 bipes»).

## 11. Fluent specifics

- **Never change** message IDs, attribute names (`.hover`, `.a11y`, `.hover-disabled`, `.wayland`, `.gesture`, `.about`, `.help`, `.value-replay`), variable names or selector keys (`[one]`, `[usb]`, `[yes]`, `[big-meter]`…). Keep the `*` default on the same variant. Translate every variant's text.
- **You may** reorder placeables, drop one the Portuguese doesn't need, add a selector on a variable that exists (including a `[0]` number variant, §10), or turn a plain message into one with variants. Nothing in the data gives gender, so don't invent gender selectors.
- **Message references stay references:** `{ project-link }`, `{ device-auto }`, `{ bluetooth-setting }`, `{ error-bluetooth-reset-gesture }`. Translate the referenced message.
- **String literals** (`{" "}`, `{", "}`, `{"  "}`) are layout: keep their meaning.
- **Multi-line messages:** keep the indentation and line breaks. A wrapped notice line may move a word or two if the Portuguese runs longer.
- **`.a11y` text** is read aloud: write it as speech, units in words where the comments ask («graus C»), «e» for «&», «mais» for «+», «cerca de» for «~» (MS-SG PDF p.22), no abbreviations.
- **Trailing and leading spaces are dropped** by Fluent; write a needed one as `{" "}`.
- **Text never translated:** the `###`/`#` comments, and `settings-language-no-chinese-font`, which its comment says is always shown in English.

## 12. Spec text (lib.ftl: bands, inputs, notes) — optional polish *(decided)*

- Keep every number, sign, ±, ≤, ≥, <, >, ≈ and ~, and each unit symbol right after its number. Numbers as in the English (§5). The spec check falls back to English on any mismatch.
- Keep the exact sense: comparisons, ranges, «típico» / «típ.», «nominal», «aproximadamente» / «aprox.», «pelo menos», «máx.». The check can't catch a flipped comparison.
- Use Minipa's wording: «Não especificado»; «da leitura» (accuracy); «dígitos» (digits/counts in accuracy), «contagens» (display counts); «proteção de sobrecarga»; «impedância de entrada»; «fator de crista»; «coeficiente de temperatura»; «pontas de prova em curto-circuito»; «tensão de circuito aberto»; «sensibilidade»; «queda de tensão» (burden voltage); «limiar». See the glossary for each source.
- Table titles keep the manual's numbering and letters: «(1) Tensão alternada», «A. Tensão contínua», «K. Temperatura 1-1. Graus Celsius».

## 13. Words to avoid

| Avoid | Use | Why |
|---|---|---|
| ficheiro, ecrã, rato, utilizador, definições, aplicação (app), transferir (download), registo, contacto, facto | arquivo, tela, mouse, (você), configurações, aplicativo, baixar, registro, contato, fato | European Portuguese |
| voltagem | tensão | colloquial; no maker manual uses it |
| capacidade (for capacitance) | capacitância | «capacidade» is a battery's or fuse's capacity (ET-1649BT PDF p.15) |
| freqüência, conseqüência | frequência, consequência | pre-2009 spelling (older Minipa manuals) |
| CC / CA | DC / AC as printed | consistency with the mode labels (§6) |
| usuário (addressing the reader) | você, or an impersonal form | MS-SG PDF p.31 |
| ser capaz de, subsequente, quaisquer, consulte | poder, seguinte, (omit), veja | MS-SG PDF p.7–8 |
| estaremos verificando (future gerund) | verificaremos / verifique | MS-SG PDF p.36 |
| digitalizar, varredura (Bluetooth scan) | procurar | MS-T scan → digitalizar is for paper |
| aplicar panorâmica (pan) | deslocar | camera sense |
| leitura (replay) | reprodução | «leitura» is a reading |
| pressionar levemente (a meter key) | pressionar brevemente | «levemente» describes force, not duration (ET-1649BT PDF p.12 vs p.20) |
| visor (the meter's display) | display | Minipa's word throughout; see SUMMARY question 6 |
| medidor (the meter) | multímetro | «medidor» alone suggests an energy meter |
| Por favor (in UI text) | — | MS voice is direct (MS-SG PDF p.15) |
