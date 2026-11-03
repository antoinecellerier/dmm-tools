# Simplified Chinese style sheet (zh-CN) — DRAFT

How the Simplified Chinese translation is written. It goes with `glossary.tsv` in this folder: where the two disagree, the glossary wins for a term and this sheet wins for form. `cargo test -p dmm-shared translations` checks what can be checked mechanically ([development guide](../../docs/development.md#translations)).

**Status: draft, 2026-11-01.** No native reviewer has read it. Every ruling in §0 is the drafter's recommendation and is marked *provisional* until a zh-CN reviewer confirms it. `SUMMARY.md` lists the questions to look at first.

## 0. Rulings (all provisional)

The short list. Where a section below gives the background, this list is what applies.

1. *Provisional.* Address the user as «你», and leave the pronoun out wherever the sentence reads without it (MS-SG p.27 §4.1.12). Never «您». Buttons, chips and menu items are bare verbs or nouns («导出…», «重置缩放»); instructions are plain imperatives, «请» only where a step would sound curt without it.
2. *Provisional.* Punctuation follows MS-SG's **software** convention (p.28): half-width `:` `!` `?` `( )` `…` in UI and CLI text; full-width `，` `。` `；` `、` in sentences; full-width `“ ”` around quoted text that holds Chinese or a placeable, half-width `" "` around Latin-only text.
3. *Provisional.* A half-width space between Han characters and Latin letters, digits or a half-width bracket (MS-SG examples p.11, p.19, p.29, p.35, p.51: «连接到 Internet 后», «发布到 Web 上», «1 TB 的», «按 Ctrl»). None next to full-width punctuation. Placeables follow MS-SG p.45: a space beside one that holds Latin text or a number (`万用表 { $model }`), none beside one that holds translated Chinese; where a placeable can hold either, no space (MS-SG: "If uncertain with the string, please remove space").
4. Numbers keep the English point everywhere; no thousands separator, and no «万»/«亿» grouping (`500K`, `40000` stay as written).
5. *Provisional.* No space before `%` («10%», MS-SG p.30 «10 ~ 20%»).
6. Meter flags stay as printed (HOLD, REL, AUTO, LOW BAT, LEAD ERR, HV!, VOID…); their spoken forms are Chinese.
7. *Provisional.* The app's own badges are translated: «已验证», «部分验证», «实验性», «换算», «超上限», «超下限», «实时». Chinese has no capitals; the badge's own style marks it.
8. AC/DC/AC+DC stay as printed in mode labels and notes. Table titles and prose use full words: «直流电压», «交流电流», and *(provisional)* «交流+直流电压» for AC+DC. Never «交直流» for AC+DC: UNI-T uses it for "AC or DC".
9. Glossary entries are full terms, with no optional words in parentheses. A parenthesis in an entry is part of the text («三极管放大倍数 (hFE)», «视在功率 (VA)»).
10. A unit symbol the English gives next to a term stays. Unit symbols never change: «°C» stays two characters, never «℃» (U+2103) as the manuals print it.
11. *Provisional.* Key names (`shortcuts-key-*`, MS-SG p.50–51): Ctrl, Shift, Alt, Esc, Home, End stay; Space → «空格键»; Left/Right → «向左键»/«向右键»; Plus/Minus → «加号»/«减号». In prose also «向上键»/«向下键», «Page Up»/«Page Down». Never add «键» after Ctrl, Shift or Alt.
12. *Provisional.* Names of on-screen controls go in full-width quotes whenever a message names them, even where the English doesn't quote them (MS-SG p.21 §4.1.7 b): «在“设置”(⚙) 中», «单击“连接”». The meter's keys and the computer's keys take no quotes: «按 SELECT 键», «按 Esc».
13. *Provisional.* Key terms: reading «读数»; range «量程»; accuracy «准确度»; resolution «分辨力»; digits/counts «字»; specifications «技术指标»; graph «图表»; trace «曲线»; recording «记录»; replay «回放»; simulated «仿真»; scale/scaling «换算»; alarm «报警»; OL «超量程».
14. *Provisional.* Terms without an approved source take the glossary's value as drafted, marked there "no approved source found"; the reviewer decides inrush current, burden voltage, hysteresis, Colorblind, Live (live wire) and the Bluetooth link first.
15. Windows and macOS names. «设备管理器» is MS-T's, «其他设备» Microsoft's zh-cn Bluetooth support page's, and macOS «系统设置 > 隐私与安全 > 输入监控» and «… > 蓝牙» Apple's zh-cn support pages'. Two are **unverified**: 'USB Input Device' «USB 输入设备», and the HID group «人体学输入设备», the name Chinese Windows' Device Manager is believed to show; Microsoft's zh-cn touchscreen support page and MS-T write «人机接口设备». Their messages carry a `# UNVERIFIED Windows name` comment until someone checks a Chinese Windows.
16. *Provisional.* Spoken unit names (`reading-unit-*`, read aloud only) take the short names: «伏», «安», «欧», «赫兹», «法», «西门子», «瓦», «伏安», «亨», «秒», and «百分比» for %; prefixes «皮», «纳», «微», «毫», «千», «兆», «吉», joined to the name («毫伏», «千欧», «兆赫兹»). No plural select. A number of percent is said first, «百分之 50.0» (`reading-spoken-amount`); «百分比» is left to where no number goes before it, as in a mode's name or after «超量程».

## 1. Sources

Every rule and term cites one of these. PDF page numbers are the PDF's own page index. Where a PDF page is a two-page spread, **L**/**R** names its left or right half, and the number printed on that page follows in parentheses: «UT61E+ CN PDF p.7R (p.12)». The `references/` paths are the maintainer's copies, kept out of the repo; the URLs find them again (`references/l10n/zh-CN/SOURCE.txt`).

| Key | Document | Where |
|---|---|---|
| UT61E+ | UNI-T UT61B+/UT61D+/UT61E+ 使用说明书 (中英文合订本), Chinese on PDF p.1–19, spreads | `references/l10n/zh-CN/uni-t-ut61bplus-dplus-eplus-manual-cn.pdf` |
| UT181A | UNI-T UT181A 使用说明书 (中英文), one page per PDF page, printed = PDF − 4 | `references/l10n/zh-CN/uni-t-ut181a-manual-cn-en.pdf` |
| UT171 | UNI-T UT171A/B/C 使用说明书, spreads | `references/l10n/zh-CN/uni-t-ut171abc-manual-cn.pdf` |
| UT195 | UNI-T UT195E/UT195M/UT195DS 使用说明书, spreads | `references/l10n/zh-CN/uni-t-ut195e-m-ds-manual-cn.pdf` |
| UT71 | UNI-T UT71A/B/C/D/E 使用说明书 (text layer) | `references/ut71/manual/ut71abcde-user-manual-cn.pdf` |
| UT8805A, UT8805N, UT8806 | UNI-T bench DMM user manuals (text layer) | `references/ut8805/manuals/UT8805A_user_manual_V1.01.pdf`, `UT8805N_user_manual_V3.0.pdf`, `UT8806_user_manual_V1.02.pdf` |
| MS-SG | Microsoft Chinese (Simplified) Localization Style Guide, 58 pages | https://aka.ms/chinese-simplified-styleguide (`references/l10n/zh-CN/microsoft-zho-chn-styleguide.pdf`) |
| MS-T | Microsoft Terminology Collection, CHINESE (SIMPLIFIED).tbx | MicrosoftTermCollection.zip (`references/l10n/zh-CN/microsoft-terminology-CHINESE-SIMPLIFIED.tbx.gz`) |
| egui | epaint 0.37.0 source (`text/unicode.rs`, `text/text_layout.rs`) | for how the GUI breaks Chinese lines |

**Unreachable.** IEC Electropedia returned 403 (as it did for fr); no IEV entry is cited. Apple's zh-cn support page mchl211c911f was read later, for the macOS names (§0 item 15).

**Not used:** other apps' translations, vendor app string files and decompiled resources, community translations, machine translation.

The UNI-T handheld manuals are image-only: every citation from them was read off a rendered page. They are uneven: UT61E+ writes both «准确度» and «精度», and «波峰因素» beside UT8805A's «波峰因数»; UT171 writes «付小显» for «副小显». The glossary picks one and says which it rejects.

## 2. Register and voice

- **«你», mostly unsaid.** MS-SG p.27: "If it's needed, use 你 instead of 您", and omit pronouns where the meaning holds. UT195's manual says «您» (PDF p.4R) and UT61E+'s «你» (PDF p.8L): both exist; the app follows MS-SG.
- **Buttons, menu items, checkboxes, chips and shortcut-list actions** are short verb phrases or nouns, no «请»: «连接», «导出…», «重置缩放», «窗口置顶», «切换记录». One list, one form (all verbs or all nouns).
- **Hovers that name what a control does** are verb phrases: «打开到所选万用表的 USB 连接 (Ctrl+O)», «保持当前读数». A hover names the action in a few words.
- **Instructions in notices and steps** are imperatives: «检查“设置”(⚙) 中是否选择了正确的设备», «打开万用表». Add «请» only where the bare verb would sound curt, as MS-SG's error example does (p.9 «密码有误，请再试一次»). MS-SG p.15: say «选择» for *select*; for *click* use «单击» (p.11 «可单击“取消”»), never «点击».
- **Activation steps** (lib.ftl `activation-*`) are numbered imperatives in the manuals' words: «1. 将 USB 连接线插入万用表», «2. 将旋钮转到任意功能档», «3. 按住 SELECT 键的同时开机». See §7.
- **Everyday words** (MS-SG p.6, p.20–21): «如果» not «倘若/若»; «只是» not «仅是»; «但» not «但是/不过»; «到» not «至» («添加到»).
- **Progressive operations** take «正在…» (MS-SG p.39): «正在重新连接…», «正在扫描…», «正在检测万用表…». Not «…中».
- **Error messages** (MS-SG p.44): "Cannot / Unable to / Could not …" → «无法…»; "… is not available" → «无法使用…»; "not found" → «找不到…» (MS-SG p.10 «找不到下载的文件»); "Failed to import" / "Import failed" → «导入失败». Short and plain; no «对不起».
- **Avoid stacked «的»** (MS-SG p.19) and long modifier chains before a noun (p.25–26); split the sentence instead.
- **Reasons the app builds into a sentence** (`input-*`, `read-error-*`, `lib-errors` details) end with no «。», as the en-US comments ask.

## 3. Script and Latin text

- **Simplified characters only**, GB 2312 / GB 18030 forms. Avoid Taiwan usage seen in some UNI-T bench manuals («二极体» → «二极管», «吋» → «英寸»).
- **No capitalization in Chinese.** English Title Case and all-caps badges have nothing to map to: "Reset Zoom" → «重置缩放», "PARTLY VERIFIED" → «部分验证».
- **Latin text keeps its case**: acronyms upper case (USB, HID, CSV, JSON, LCD, NCV) without a plural «s» (MS-SG p.18); product and brand names as written.
- **Language names**: the Language list shows each language in itself; «简体中文» is fixed.
- **Book titles** take «《》» (MS-SG p.37); the app has none today.

## 4. Punctuation and spaces

### Which width (ruling 2)

MS-SG p.28 gives two conventions. For **software**:

| Half-width | Full-width |
|---|---|
| `:` `!` `?` `( )` `< >` `[ ]` `…` `-` | `，` `；` `。` `、` `《》` |

For documentation it uses full-width `：` `；` `！` `？` `（）` and `“”`. The app's strings are software, so:

- **Colon:** half-width `:` then a space (MS-SG p.29 «错误: %1»), no space before a full-width mark or a `“`: «错误: “{ $file }”». Captions: «最小值:», «最大值:», «平均值:», «σ:», «设备:», «绘制:», «显示:». Notation keeps the English exactly, no space: `Mean={ $value }` → `平均值={ $value }`, `Readings=` → `读数=`, `ΔT=`, `σ=`, `∫:`, `Y:Fixed` → `Y:固定`, `{ $ratio }:1`, and anything the meter prints (`SETUP -> Communication -> ON`).
- **Comma, full stop, semicolon, enumeration comma:** full-width `，` `。` `；` `、`. Lists of parallel items take `、` (MS-SG p.28–29 «Word、Excel 和 PowerPoint»), the last joined by «和».
- **Parentheses:** half-width `( )`; no space between them and Han characters outside, one space between them and Latin letters or digits outside (MS-SG p.34–35): «我的发送服务器(SMTP)要求验证» but «Type (如 string 或 Guid)». In practice the app's parentheses mostly hold Latin («(Ctrl+O)», «(6V)»), so they take a space before.
- **Question and exclamation marks:** half-width, then a space before the next character (MS-SG p.33, p.37).
- **Ellipsis:** U+2026 `…`, as the English has it (MS-SG p.31 «正在上载…»); straight after the last character: «导出…», «正在重新连接…». Never `……`.
- **Em dash:** never (MS-SG p.31: it looks like «一»). The English uses it a lot in hovers and toasts ("Halt acquisition — stops recording and graph updates"): rephrase with `：`, `，` or parentheses: «停止采集: 记录和图表都停止更新». Where a pause must stay, an en dash with spaces ` – ` (MS-SG p.30).
- **Wave dash:** ranges may use `~` with spaces as MS-SG p.30 shows («10 ~ 20%»); the spec text's own `10V~36V`, `50Hz ~ 60Hz` is notation: keep it as written. The manuals' full-width `～` is not used.
- **&** becomes «和» (MS-SG p.15, also for screen readers): "Keyboard & Mouse" → «键盘和鼠标».
- **Slash:** half-width, no spaces (MS-SG p.36): «显示/隐藏».

### Quotes (ruling 12)

- Text containing Chinese or a placeable: full-width `“ ”` (MS-SG p.32 «未找到程序集“{0}”»). Latin-only text: half-width `" "` with a space outside them (MS-SG p.32 «使用 "out" 参数»). Nested quotes: `‘ ’` inside `“ ”` (MS-SG p.33).
- **Names of on-screen controls** are quoted with `“ ”` wherever a message names them (MS-SG p.21): «单击“连接”», «勾选“查找蓝牙设备”». The app's text has no bold, and an unquoted «设置» mid-sentence reads as a verb.
- The `‘…’` and `'…'` around command lines and Windows device names stay as they are (§8).
- Keys take no quotes (MS-SG p.22): «按 Enter 键», «按 SELECT 键».

### Spaces between scripts (ruling 3)

- One half-width space between a Han character and a Latin letter or digit, either side: «万用表 UT61E+ 已连接», «第 3 次尝试», «500K 个样本». MS-SG's own examples do this throughout; the UNI-T manuals don't («按SELECT键»), which is print layout, not software.
- **No** space between Han and full-width punctuation, between a digit and its unit symbol where the English has none (`10MΩ`), or inside notation the English writes tight.
- **Use U+0020**, not U+00A0: egui breaks Chinese text between any two Han characters anyway (below), and a non-breaking space would only glue Latin runs to the Chinese around them.
- **Placeables** (MS-SG p.45): a space next to a placeable that holds a number, a model name, a path, a key name or a unit; none next to one that holds translated Chinese (`$reason` in Chinese, a translated mode label). When a placeable can be either, leave the space out, as MS-SG says.

### How egui breaks Chinese lines

epaint 0.37 (`text_layout.rs:1394–1431`, `unicode.rs:49–71`) may break before any Han character or kana, and at whitespace other than U+00A0. It never starts a row with a closing mark from its list, which includes `，` `。` `、` `：` `；` `？` `！` `）` `”` `’` `》` `…` and the half-width `)` `:` `!` `?`. So full-width punctuation never strands at a line start, and nothing special is needed. Opening marks (`“` `（` `《`) are not held to the next character; avoid long quoted names at the end of a wrapped notice line.

## 5. Numbers

**The app keeps the English decimal point in every number it formats itself**, as for every language (`docs/localization-design.md`, "Numbers keep their point"). Chinese uses the point anyway (MS-SG p.31 «123.45»), so this costs nothing.

1. **Digits wherever the English has digits** (MS-SG p.26), and always with a unit or a measure word. Where the English spells a small number out, use Chinese numerals: "two beeps" → «两声蜂鸣» (`两` before a measure word, not `二`).
2. **No thousands separator, no «万»/«亿»:** write `1000 V`, `40000`, `500K` as the English does. The spec check rejects regrouped numbers. Never «6千», «4万».
3. **Typed examples keep the point:** the hover `(e.g. 3.3, 5, 12)` shows what to type; keep the half-width commas between the values there, because the reference-values field reads them as separators: «(例如 3.3, 5, 12)».
4. **Measure words** (MS-SG p.45, p.26 «两位研究人员», not «2 个研究人员»): pick the specific one from the glossary: samples, readings, markers, files, digits «个»; records and log entries «条»; attempts and times «次»; beeps «声»; the meter «台». A number before a noun always takes one: «{ $samples } 个样本».
5. **Ordinals:** «第 { $attempt } 次尝试».
6. **Gaps the app leaves (for the developers, not the translators):** `MB`/`KiB` arrive preformatted, which is fine in Chinese (MS-SG p.18 keeps MB and GB); the `K` suffix (`500K`) reads as English; time units in `≈194 MB, about 14 h` and the axis's `1m 5s` stay English symbols. Leave the placeables as they are.

**Percent:** no space: «10%», «{ $percent }%». The spec's `10%–100%` is notation: keep it.

## 6. Units and AC/DC

- **Unit symbols never change,** except in the spoken reading's unit names (§0 ruling 16): V, mV, A, mA, µA, Ω, kΩ, MΩ, Hz, F, °C, °F, W, VA, Wh, Ah, dBm, dBV, s, ms, h, V·s, mAh. Keep `°C`/`°F` as the app writes them (degree sign + letter); the manuals' single-character «℃» and «℉» would fail the spec check and look different from the reading.
- **Space between a number and its unit:** follow the English as written; spec text keeps the manual's own spacing (`10MΩ`), which the spec check ignores.
- **Time words in prose:** where the English writes words ("17 minutes", "2 seconds"), write «17 分钟», «2 秒»; where it writes a symbol, keep the symbol.
- **The meter's AC/DC labels stay as the meter prints them:** `DC V`, `V AC`, `AC mA`, `AC+DC`, `ACV`, `DCV`, `LoZ V~`. The UNI-T manuals do the same in their tables («LoZ ACV 600.0V», UT61E+ PDF p.15R).
- **In prose and table titles, spell the words out:** «直流电压», «交流电压», «直流电流», «交流电流» (UT61E+ PDF p.8L, p.8R, p.18R), and «交流+直流电压» for AC+DC (ruling 8). **«交直流» means AC *or* DC** in UNI-T's Chinese («交直流电压测量», UT61E+ PDF p.7L), so it never stands for AC+DC.
- **True RMS:** «真有效值», or «真有效值 (True RMS)» once in a note; «True RMS» alone is fine where the meter prints it. "rms" after a number (`250V rms`) is notation: keep it.
- **OL** is «超量程» in words (UT71 PDF p.13 «OL 超量程提示符»); «过载» is for overload *protection* only.

## 7. What the meter prints stays as it prints it

UNI-T's Chinese manuals print every annunciator and key name exactly as on the meter and explain it beside it: «HOLD 数据保持键», «REL 相对测试键», «RANGE 量程切换键», «LIGHT 背光开关键» (UT71 PDF p.11), «AUTO 自动量程提示符», «OL 超量程提示符» (UT71 PDF p.13), «短按 SELECT 键» (UT61E+ PDF p.8L), «LoZ 交流低阻抗提示符» (UT61E+ PDF p.6R).

So:
- **Status flags** (`lib-names.ftl`) stay as they are: HOLD, REL, AUTO, MIN, MAX, AVG, LOW BAT, HV!, P-MAX, P-MIN, LEAD ERR, COMP, REC, LoZ, VOID. The spoken forms (`reading-spoken-*`) are Chinese: «数据保持», «自动量程», «电池电量不足», «表笔插错», «高压警示».
- **Key names, display text and menu paths** in activation steps and hovers stay as printed: SELECT, SEL, ZERO/ᛒ, Hz/Duty△/ᛒ, Tab⇌, △/ᛒ, 1ms PEAK, MAX MIN, SEND, RS232, PC, BLE, SETUP -> Communication -> ON, "Org", dSAPO, APO.oF, BT, ((D)). Translate around them in the manuals' words: «按 X 键» (press), «短按 X 键», «长按 X 键» (UT61E+ PDF p.7R defines both: <2 s and >2 s), «按住 X 键的同时开机», «直到显示屏上显示 ᛒ».
- **Mode qualifiers:** keep LPF and REL (`qualifier-lpf-mode = LPF { $mode }`, `qualifier-mode-rel = { $mode } REL`). "Peak" → «峰值»: «{ $mode } 峰值».
- **Brands, model names and trademarks** unchanged: UNI-T, Voltcraft, ZOTEK, ZOYI, BSIDE, ANENG, EEVblog, Brymen, OWON, part numbers, AutoCheck™. UNI-T's Chinese brand «优利德» appears on its covers; keep «UNI-T» as the app writes it. "Mock UT61E+" → «UT61E+ 仿真».

## 8. Command lines, options, files and keys

- **Verbatim**, as for fr: every `--flag` and its values, `dmm-cli`, `dmm-gui`, subcommands, quoted command lines, device and step ids, setting ids, settings-file keys, environment variables, file names and patterns, CSV column names and JSON keys, the backquoted text in `read-error-*`, RFC 3339, the password `0000`. The translation check fails a message that loses a `code` span or an `--option`. Put a space between them and the Chinese around them (§4).
- **clap's words** are translated too (`shared.ftl`): «用法:», «命令:», «参数:», «选项:», «[默认值: { $value }]», «[可能的值: { $values }]». Value names in capitals (`<FORMAT>`) and the usage line's syntax stay as clap writes them; in errors, keep the names in their straight quotes, as typed. CLI headings in capitals («CONFIGURATION:») become plain Chinese headings: «配置:», «环境变量:».
- **Windows and macOS names** must match what a Chinese system shows (§0 item 15). 'CP2110 USB to UART Bridge' is the device's own name and stays English.
- **Computer key names** are messages of their own (`shortcuts-key-*`): §0 item 11. Prose names keys the same way: «按 Esc», «Shift + 拖动», «用向上键/向下键滚动». Letters, digits, F1–F12 and macOS's ⌘ ⇧ ⌥ stay.

## 9. Length budget

Chinese has fewer characters than English but each Han character is about as wide as two Latin letters (about one em). So:

- `cargo test -p dmm-shared long_labels -- --ignored --nocapture` counts characters and will under-report Chinese width: count a Han character or full-width mark as 2 when reading it, and judge on screen (verify-gui with a CJK font installed).
- Most labels come out narrower than the English («连接» against “Connect”). Watch the places fr watched: the top bar, the graph chips, the mode line, the Statistics column, the Settings chips.
- **The terminal:** a Han character takes two cells. `cli/help.ftl`'s CONFIGURATION/ENVIRONMENT lines aren't wrapped by clap; keep them under about 40 Han characters. *Code gap:* several CLI columns pad with Rust's `{:<width$}`, which counts characters, so translated text in them will misalign (e.g. `crates/dmm-cli/src/format.rs:271` sub-value labels, `cmd/settings.rs:565` setting names, `cli.rs:633`); check whether those cells are translated before shipping.
- Chinese doesn't abbreviate (MS-SG p.16): shorten by choosing a shorter word («标准差», not «标准偏差»), never by cutting a word.

## 10. Placeables, word order and measure words

Chinese has no gender, number or article to agree, so a placeable can sit anywhere. What matters instead:

- **Word order:** modifiers come before the noun, and long ones read badly (MS-SG p.25). Move placeables to where Chinese wants them; Fluent allows reordering.
- **A noun in front of a model name** still helps: «万用表 { $model }» reads better than a bare model number, and avoids «{ $model } 的支持». «对万用表 { $model } 的支持已验证».
- **Numbers take a measure word** (§5): «{ $count } 个读数», never «{ $count } 读数».
- **Status lines in label: value form:** «量程: { $range } (手动)», «报警已启用: { $limits }».
- **`$link`, `$side`, `$state` and the other selectors** have one variant per value: translate each in full: [usb] «USB 连接线已连接», [adapter] «蓝牙适配器已连接», [radio] «蓝牙链路已建立».
- **`$setting` in `connection-switch-failed`** is a lowercase CLI id (hold, rel, minmax, peak): «切换 { $setting } 失败: { $error }».
- **Inserted reasons** (`$reason`, `$error`, `$detail`) may be Chinese, or English from the OS or a parser: put them after a colon, never in the middle of a sentence.

**Plurals.** CLDR gives Chinese only `other`, and the translation check asks for exactly the forms the plural rules give (`plural_forms("zh-CN")` = {other}, `crates/dmm-shared/src/l10n/translations.rs`). So a plural select keeps one variant: `{ $samples -> *[other] 将丢弃 { $samples } 个未导出的样本。 }`, or, simpler, drop the select and write the sentence once, as long as every variable of the English still appears in the text (the check compares variables). Nested selects in `export-import.ftl` collapse the same way. Selectors that are not plurals (`[usb]`, `[yes]`, `[big-meter]`…) keep all their variants.

## 11. Fluent specifics

- **Never change** message IDs, attribute names (`.hover`, `.a11y`, `.hover-disabled`, `.wayland`, `.gesture`, `.about`, `.help`, `.value-replay`), variable names or non-plural selector keys. Keep the `*` default on the same variant. Translate every variant's text.
- **You may** reorder placeables, drop a plural select (§10), or add a selector on a variable that exists.
- **Message references stay references:** `{ project-link }`, `{ device-auto }`, `{ bluetooth-setting }`, `{ error-bluetooth-reset-gesture }`.
- **String literals** are layout; adapt them to Chinese punctuation: `reading-spoken-separator` `{", "}` → `{"，"}`; `graph-plot-a11y-also-separator` `{", "}` → `{"、"}`; `graph-histogram-a11y-separator` `{", "}` → `{"，"}`; `graph-histogram-a11y-interval-separator` `{"; "}` → `{"；"}`; `graph-plot-a11y-separator` `{" "}` (between sentences) → `{""}`, since a sentence ending in «。» needs no space. The two leading spaces in `connect-not-identified-name-it` (`{"  "}`) are CLI indentation: keep them.
- **Multi-line messages:** keep the indentation and the line breaks; one activation step per line. Chinese lines come out shorter; re-wrap a notice only if a line would be very short.
- **`.a11y` text** is read aloud: units in words where the comments ask («摄氏度»), «和» for «&», no symbols a screen reader may misread (MS-SG p.15 names & + ~).
- **Trailing and leading spaces** are dropped by Fluent; write `{" "}` where one matters.
- **Never translated:** the `#` comments, and `settings-language-no-chinese-font`, which is always shown in English (a user without a Chinese font couldn't read it).

## 12. Spec text (lib.ftl: bands, inputs, notes)

Optional for shipping (`translations::optional()`), as for every language.

- Keep every number, sign, ±, ≤, ≥, <, >, ≈ and ~, and each unit symbol right after its number. The spec check falls back to English on any mismatch.
- Keep the exact sense: comparisons, ranges, «典型值»/«典型», «标称», «约», «至少», «最大». The check can't catch a flipped comparison.
- Use the manuals' wording: «未指定» (UT71 PDF p.22); «读数» in accuracy («±(a%读数+b)», UT61E+ PDF p.15L); «字» for digits («加 10 个字», «剩余读数 ≤10 个字», UT61E+ PDF p.15L, p.17R); «量程的 10%–100%» (UT61E+ PDF p.16L); «过载保护»; «输入阻抗»; «波峰因数»; «温度系数»; «表笔短路»; «开路电压»; «仅供参考».
- Table titles keep the manual's numbering and letters: «(1) 交流电压», «A. 直流电压», «K. 温度 1-1. 摄氏度».

## 13. Glyphs and fonts

The GUI draws Chinese with a system font chosen for `zh-Hans` (`crates/dmm-gui/src/app/language.rs`, `font_locale`), and shows the English notice `settings-language-no-chinese-font` when none is found. Latin text and the meter's symbols still come from the bundled fonts (Ubuntu-Light, Hack), so ᛒ, △, ⇌, Ω, µ, σ, ∫, ≈ and ⏸/▶ need nothing new.

Characters this sheet asks for beyond Latin: Han characters of GB 2312, the full-width marks `，` `。` `；` `、` `：` (U+FF0C, U+3002, U+FF1B, U+3001, U+FF1A), `“ ”` `‘ ’` (U+201C/D, U+2018/9). Noto Sans CJK SC covers all of them; this was **not checked on this machine**, which has no CJK font installed (`fc-list :lang=zh-cn` is empty). Check under verify-gui with Noto Sans CJK SC installed before shipping. Avoid rare or extension-block characters (CJK Ext. B and beyond): none appears in the glossary.

## 14. Words to avoid

| Avoid | Use | Why |
|---|---|---|
| 您 | 你, or no pronoun | MS-SG p.27 |
| 点击 | 单击 / 选择 | MS-SG p.11, p.15 |
| 仪表, 仪器 (for the meter) | 万用表 | one word throughout; 仪器 only for a bench instrument in its own text |
| 计量 (meter) | 万用表 | MS-T meter => 计量 is the billing sense |
| 范围 (measurement range) | 量程 | every manual; 范围 is MS-T's generic range |
| 精度 | 准确度 | UT61E+ mixes them; tables and MS-T say 准确度 |
| 分辨率 (in specs) | 分辨力 | 分辨率 is also screen resolution |
| 过载 (for OL) | 超量程 | UT71 PDF p.13; 过载 is overload protection |
| 交直流 (for AC+DC) | 交流+直流 / AC+DC | UNI-T uses 交直流 for "AC or DC" |
| 波峰因素 | 波峰因数 | misprint variant in UT61E+ |
| 二极体 | 二极管 | Taiwan usage in UT8806 |
| 模拟 (simulated meter) | 仿真 | 模拟 is "analog" in meters (模拟条) |
| 缩放 (scaling) | 换算 | 缩放 is zoom (MS-T) |
| 录制 (recording) | 记录 | MS-T recording => 录制 is audio/video |
| 闹钟 (alarm) | 报警 | MS-T alarm => 闹钟 is an alarm clock |
| 播放 / 重播 (replay) | 回放 | media / broadcast senses |
| 召回率 / 回顾 (Recall) | 数据回读 | MS-T recall is the statistics sense |
| 跟踪 / 迹 (trace) | 曲线 | MS-T trace is the debugging sense |
| ℃, ℉ | °C, °F | the app's own unit text; spec check |
| 万, 亿, 千 grouping | digits as written | spec check, consistency with readings |
| em dash — | ：，( ) or – | MS-SG p.31 |
| 倘若, 若要, 仅是 | 如果, 要, 只是 | MS-SG p.6 |
