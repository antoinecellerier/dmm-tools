# Japanese style sheet (ja), draft

How the Japanese translation is written. It goes with `glossary.tsv` in this folder: where the two disagree, the glossary wins for a term and this sheet wins for form. `cargo test -p dmm-shared translations` checks what can be checked mechanically ([development guide](../../docs/development.md#translations)).

**Status: draft, 2026-11-02.** No native reviewer has read it. Every ruling in §0 marked *provisional* is the drafter's recommendation until the user or a ja reviewer rules on it. `SUMMARY.md` lists the questions to look at first.

## 0. Rulings

The short list. Where a section below gives the background, this list is what applies. Items without *provisional* are the decisions made for every language.

1. *Provisional.* No pronoun: leave "you" and "we" out (MS-SG p.9, p.22–23); never «あなた», never «弊社». Five levels, as MS-SG p.42–43 assigns them:
   - **Buttons, menu items, chips, tabs, headings, field labels, list items and shortcut-list actions:** noun phrases (体言止め): «接続», «エクスポート…», «ズームのリセット», «記録の切り替え».
   - **Checkboxes:** plain form (である調): «起動時に自動接続する», «Bluetooth デバイスを探す».
   - **Hovers that name an action:** plain form, no «。»: «現在の測定値を固定する». A hover written as sentences takes です・ます and «。».
   - **Notices, toasts, errors and steps:** です・ます, «。» at the end; a request is «～してください。». Activation steps are numbered «～します。» sentences.
   - **dmm-cli:** command and option descriptions in the plain form, like MS-SG's command list (p.58, «ヘルプを表示する»); runtime lines (status, notes, errors) in です・ます.
2. *Provisional.* Punctuation follows MS-SG p.24–31: full-width «、» and «。» in Japanese sentences; half-width `:` `?` `!` `( )` `[ ]` `/` `+`. A colon has no space before it and one after it when text follows («エラー: …»).
3. *Provisional.* One half-width space (U+0020) between Japanese and Latin letters or digits (MS-SG p.32: «Word を使用», «第 3 章», «Shift キー»), with MS-SG's exceptions (p.32–34): none next to «、» «。», none inside brackets and quotes, none around `/`, none before a closing `:` `?` `!` `…`, none between a number and `%` or `°`. Never U+00A0.
4. Numbers keep the English point everywhere, with no thousands separator (this departs from MS-SG p.24 «2,000 ページ»). Digits are half-width (MS-SG p.22). Counters (§5): «件» for readings, samples and records; «個» for markers and files; «回» / «回目» for times and attempts.
5. *Provisional.* Katakana long vowels follow MS-SG p.35–39: «ー» for -er, -or, -ar and for words of fewer than four counted characters, with MS-SG's exceptions (コネクタ, トランジスタ, プロセッサ, メモリ). So «マルチメーター», «フィルター», «アダプター», «バッファー», «マーカー», «ファクター», «モーター», «デューティー», although Sanwa and Kyoritsu print «マルチメータ», «フィルタ», «クレストファクタ».
6. *Provisional.* An on-screen control or setting named in a sentence is set in half-width brackets, as MS-SG p.28 and p.61 ask, even where the English has no quotes: «[接続] をクリックしてください», «[設定] (⚙) で», «[Bluetooth デバイスを探す] チェック ボックスをオンにします». This covers the English's quoted button names (a button named as a sentence's subject is quoted in every language).
7. *Provisional.* Other quoted text takes «「」»: text the user types, text from a file, what the meter reports or shows on its display («「Org」と表示されます», as Sanwa PC7000 JP PDF p.7 «「dSAPO」» and KEW1062 JP PDF p.11 «「ＯＬ」表示»). No «“ ”» in Japanese text. The `'…'` around command lines and Windows device names stays as it is (MS-SG p.54).
8. Meter flags stay as printed (HOLD, REL, AUTO, LOW BAT, LEAD ERR, HV!, VOID…); their spoken forms are Japanese.
9. *Provisional.* The app's own badges are translated: «検証済み», «一部検証済み», «試験的», «スケール», «上限超過», «下限未満», «ライブ». Japanese has no capitals; the badge's style marks it.
10. AC/DC/AC+DC stay as printed in mode labels and notes. Table titles and prose use full words: «直流電圧», «交流電流», and *(provisional)* «交流+直流電圧» for AC+DC, in the English's order (the makers write «直流+交流», Sanwa PC7000 JP PDF p.2).
11. Glossary entries are full terms, with no optional words in parentheses. A parenthesis in an entry is part of the text («トランジスタの電流増幅率 (hFE)», «皮相電力 (VA)»). A unit symbol the English gives next to a term stays.
12. Unit symbols never change: `°C` stays two characters, never «℃» (U+2103) as the manuals print it; `µ` stays U+00B5.
13. *Provisional.* Key terms: reading «測定値»; range «レンジ»; accuracy «確度»; resolution «分解能»; digits/counts «カウント»; specifications «仕様»; meter «マルチメーター»; dial «ファンクションスイッチ»; test lead «テストリード»; display «ディスプレイ»; graph «グラフ»; trace «トレース»; recording «記録»; replay «リプレイ»; simulated «シミュレーション»; scale «スケール»; alarm «アラーム»; OL «オーバーレンジ»; support (of a meter) «対応»; machine-translated «自動翻訳».
14. *Provisional.* Key names (`shortcuts-key-*`, MS-SG p.51, p.56–57): Ctrl, Shift, Alt, Space, Esc, Home, End stay; Left/Right → «←» / «→»; Plus/Minus → «プラス» / «マイナス» (no approved source). In prose a Latin key name takes « キー» after a space («Esc キー», «Space キー»), an arrow takes «キー» with none («←キー»); PgUp/PgDn → «PageUp» / «PageDown». Text read aloud says «左右方向キー», «上下方向キー».
15. Plurals: Japanese has only `other` (`intl_pluralrules` 7.0.2, CLDR 37: `ja` → `PluralCategory::OTHER`), and the translation check asks for exactly that form. Keep one `*[other]` variant, or drop the select and write the sentence once, as long as every variable of the English still appears (§10).
16. Windows and macOS names. «デバイス マネージャー» (MS-T), «ヒューマン インターフェイス デバイス» (Microsoft's ja-jp touchscreen page), «ほかのデバイス» (Microsoft's ja-jp Bluetooth page, in Device Manager); macOS «システム設定 > プライバシーとセキュリティ > 入力監視» and «… > Bluetooth» (Apple's ja-jp pages), written with the app's brackets: «[システム設定] > [プライバシーとセキュリティ] > [入力監視]». **Unverified:** 'USB Input Device' «USB 入力デバイス»: its message carries a `# UNVERIFIED Windows name` comment until someone checks a Japanese Windows.
17. *Provisional.* Spoken unit names (`reading-unit-*`, read aloud only): «ボルト», «アンペア», «オーム», «ヘルツ», «ファラド», «ジーメンス», «ワット», «ボルトアンペア», «ヘンリー», «秒», «パーセント»; prefixes «ピコ», «ナノ», «マイクロ», «ミリ», «キロ», «メガ», «ギガ», joined whole («ミリボルト», «キロオーム», «メガオーム»: not Fluke's «メグオーム»). °C/°F spoken «摂氏度» / «華氏度». The number comes first: «50.0 パーセント». No plural select.
18. *Provisional.* Ellipsis: U+2026 «…», as the English mostly writes it, straight after the last character («エクスポート…»); where the English has "...", write «…» too. MS-SG p.26 writes three periods («変更...»); the app keeps one character for all languages. Progress labels take «～中…» (MS-SG p.41): «再接続中…», «スキャン中…», «マルチメーターを検出中…».
19. *Provisional.* Terms without an approved source take the glossary's value as drafted, marked there "no approved source found"; the reviewer decides first: burden voltage, inrush current's label, Live (live wire), COMP High/Low, the badges, Colorblind, time window, Visible, Plus/Minus key names.

## 1. Sources

Every rule and term cites one of these. PDF page numbers are the PDF's own page index. The `references/` paths are the maintainer's copies, kept out of the repo; the URLs find them again (`references/l10n/ja/SOURCE.txt`).

| Key | Document | Where |
|---|---|---|
| CD772, CD800a, PC7000, PC710, PC720M, PC773, PM3 | Sanwa Electric Instrument (三和電気計器) デジタルマルチメータ 取扱説明書, Japanese. CD800a and PM3 are one-page folded sheets (PDF p.1) | `references/l10n/ja/manuals/sanwa-*_JP.pdf`, from overseas.sanwa-meter.co.jp/pdf/manual/digital_multimeters/ |
| KEW1062, KEW1052, KEW MATE 2000A | Kyoritsu (共立電気計器) KEW 1061/1062, KEW 1051/1052, KEW MATE 2000A/2001A 取扱説明書 | `references/l10n/ja/manuals/kew-*.pdf`, from kew-ltd.co.jp/download/manual/ |
| Fluke 80V, Fluke 289 | Fluke 80 Series V and 287/289 ユーザーズ・マニュアル (日本語) | `references/l10n/ja/manuals/fluke-*.pdf`, from fluke.com/ja-jp product pages |
| MS-SG | Microsoft Japanese Localization Style Guide, 71 pages (printed page = PDF page) | https://aka.ms/japanese-styleguide (`references/l10n/ja/microsoft-jpn-jpn-styleguide.pdf`) |
| MS-T | Microsoft Terminology Collection, JAPANESE.tbx | MicrosoftTermCollection.zip (`references/l10n/ja/microsoft-terminology-JAPANESE.tbx.gz`) |
| Apple | Apple support, Mac User Guide in Japanese | https://support.apple.com/ja-jp/guide/mac-help/mchl211c911f/mac, https://support.apple.com/ja-jp/guide/mac-help/mchl4cedafb6/mac |
| MS support | Microsoft support in Japanese | Enable and disable a touchscreen in Windows; Fix Bluetooth problems in Windows (support.microsoft.com/ja-jp) |
| egui | epaint 0.37.0 source (`text/unicode.rs`) | for how the GUI breaks Japanese lines |

**Unreachable.** Hioki's manual downloads answered 403 and Custom's site did not answer; neither is cited. No Japanese manual from UNI-T or OWON was found.

**Not used:** other apps' translations, vendor app string files and app manuals, community translations, machine translation.

The makers disagree on several words, and the glossary picks one and names the others: «テストリード» (Sanwa, Fluke) or «測定コード» (KEW); «表示器» (Sanwa), «表示部» (KEW) or «ディスプレイ» (Fluke); «静電容量» or «キャパシタンス» (KEW); «導通チェック» or «導通試験» (Fluke); «キー» (KEW) or «ボタン» (Sanwa); «デューティー比» or «デューティ比».

## 2. Register and voice

- **No pronoun.** MS-SG p.9 and p.22–23: Japanese leaves out "you" and "we"; «あなた» may sound rude and «弊社» is for formal text only. Rewrite around the subject instead: "You can change…" → «～を変更できます».
- **The levels (ruling 1).** MS-SG p.42–43: です・ます for explanatory text in windows, messages and status bars; である調 for checkboxes and option buttons («パスワードを記憶させる», p.23); noun phrases for menu titles, commands, buttons, tabs, labels and list items («変更箇所の表示», p.43). Keep one level within a list (p.43). A noun phrase is simple: «～の～», never «～の～の～» (p.43); use the plain form where a noun phrase would be unclear («アイテムを開く»).
- **Hovers** name the action in a few words, in the plain form without «。»: «選択したマルチメーターへの USB 接続を開く (Ctrl+O)», «現在の測定値を固定する». A hover that explains in sentences (the update check, the buffer size) uses です・ます.
- **Instructions** in notices and steps: «～してください。» (MS-SG p.42). For check, see and try, MS-SG p.45 asks for the politer forms: «ご確認ください», «ご覧ください», «もう一度お試しください». No other honorific or humble forms (p.44): «お使いいただく», «～していただきますと» are too much.
- **Questions** asking the user what to do: «～しますか?», never «～してもよろしいですか?» (MS-SG p.46): «エクスポートされていないサンプルを破棄しますか?»
- **Everyday words** (MS-SG p.7–9): «できます» not «が可能です»; «お勧めします» not «推奨します»; «使用できません» not «無効です» for invalid input; «～すると» not «～することにより»; «もう一度» not «再度»; «詳しい» not «詳細な»; «～されていません» not «未～» («エクスポートされていない», not «未エクスポート»). Avoid «～させる» (p.39–40). Use passive voice when the app does something by itself (p.43–44): «デバイスが検出されます».
- **Must, should, may** (MS-SG p.41–42): «～する必要があります»; «～する場合があります».
- **Error messages** (MS-SG p.52–53): です・ます; "Cannot …" → «～できません»; "Failed to …" / "… failed" → «～できませんでした» («インポートできませんでした: …»); "Cannot find" / "not found" → «～が見つかりません»; "… is not available" → «～は使用できません». No «すみません»; "Sorry" is «申し訳ございません» where the English has it (the app does not).
- **Reasons the app builds into a sentence** (`input-*`, `read-error-*`, `lib-errors` details) end with no «。», as the en-US comments ask. A reason that is a clause still takes です・ます («ファイルが空です»); one that is a noun phrase stays one («不明なデバイス: { $device }»).
- **Click and select:** follow the English: "click" → «クリック», "pick"/"select" → «選択». MS-SG p.15 prefers «選択» for accessibility, and «選ぶ» reads more naturally inside a longer sentence (p.8).

## 3. Characters and Latin text

- **Kana and kanji** per the 常用漢字表 and 送り仮名の付け方 (MS-SG p.18); counters read «か», «こ» in hiragana or kanji: «3 か月», «5 個», never «ヶ» (p.18).
- **Full-width katakana, half-width Latin letters and digits** (MS-SG p.18, p.22). Never half-width katakana, never full-width Latin («ＯＬ», «Ｖ») even where the manuals print them.
- **Katakana compounds** (MS-SG p.19): a space where the English has one («ダイアログ ボックス», «マウス ホイール», «タイトル バー»), none where it has a hyphen («ダブルクリック»), a space before «バー» («メニュー バー»). MS-T's spelling wins where it differs from MS-SG's examples («チェック ボックス»). In a tight label the space may go (p.16–17: «ユーザーインターフェイス»).
- **Long vowels (ruling 5)** per MS-SG p.35–39 and its exception list. The meter manuals follow the older JIS habit of dropping the final «ー» (Sanwa «マルチメータ», «モータ»; KEW «フィルタ»); the app follows MS-SG, as Fluke's Japanese manuals do («マルチメーター», 289 JP PDF p.11).
- **No capitals in Japanese.** English Title Case and all-caps badges have nothing to map to: "Reset Zoom" → «ズームのリセット», "PARTLY VERIFIED" → «一部検証済み».
- **Latin text keeps its case**: acronyms in capitals (USB, HID, CSV, JSON, LCD, NCV), untranslated (MS-SG p.17); product and brand names as written.
- **Middle dot «・»** only in names (MS-SG p.31); not in technical compounds («クレストファクター», not «クレスト・ファクター» as Fluke prints).
- **Language names**: the Language list shows each language in itself; «日本語» is fixed.

## 4. Punctuation and spaces

### Which width (ruling 2)

MS-SG p.32: Japanese symbols and punctuation are full-width, symbols used worldwide are half-width.

| Full-width | Half-width |
|---|---|
| `、` `。` `「」` | `:` `?` `!` `( )` `[ ]` `/` `+` `-` `…` `%` |

- **Full stop and comma:** «。» and «、» in Japanese sentences (MS-SG p.24). The half-width comma only in a list of Latin values as the English writes it («3.3, 5, 12») and between Latin items (p.24). Never «，» or «．» (KEW's manuals use them).
- **Colon:** half-width, no space before, one after when text follows: «最小値:», «デバイス:», «エラー: { $reason }» (MS-SG p.25, p.33, p.54 «データベース: ‘%s’»). Notation keeps the English exactly, no space: `Mean={ $value }` → `平均値={ $value }`, `Readings=` → `測定値=`, `ΔT=`, `σ=`, `∫:`, `Y:Fixed` → `Y:固定`, `{ $ratio }:1`, and anything the meter prints (`SETUP -> Communication -> ON`).
- **Question and exclamation marks:** half-width, no space before (MS-SG p.30, p.33). Drop «!» where it sounds aggressive (p.30).
- **Parentheses:** half-width; a space between them and Japanese or Latin text outside, none inside (MS-SG p.27–28, p.33–34: «列 A (タイトル)», «[View] (表示) をクリック»). None between a parenthesis and «。» «、».
- **Ellipsis (ruling 18):** U+2026, straight after the last character (MS-SG p.33 «その他...» takes no space).
- **Em dash:** do not use it (MS-SG p.26). The English uses it a lot in hovers and toasts ("Halt acquisition — stops recording and graph updates"): rephrase with «。», «、», a colon or parentheses: «データ取得を一時停止 (記録とグラフの更新を停止)».
- **En dash:** do not use it unless necessary (MS-SG p.25). Ranges the app writes («A–B») are notation: keep them.
- **Wave dash «～»:** never in the app's text (MS-SG p.26: it breaks builds, and ranges become «から» or a half-width hyphen): "between 0 and 99 mm" → «0 から 99 mm». The spec text's own `10V~36V`, `50Hz ~ 60Hz` is notation: keep it as written.
- **&** becomes «と» (or «、»), also for screen readers (MS-SG p.15, p.29): "Keyboard & Mouse" → «キーボードとマウス»; "Clear graph & statistics" → «グラフと統計のクリア».
- **Slash:** half-width, no spaces (MS-SG p.30, p.32): «オン/オフ».
- **Plus in shortcuts:** half-width, no spaces in the UI («Ctrl+O», MS-SG p.39, p.57). The gesture column writes `Shift + drag` with spaces: keep the English's spacing there.

### Brackets and quotes (rulings 6 and 7)

- **«[ ]» for the names of on-screen controls and settings** whenever a message names them (MS-SG p.28, p.61): «[設定] (⚙) で正しいデバイスが選択されていることを確認してください». After a button's name, no «ボタン» unless needed (p.61); a checkbox takes «チェック ボックス» after it; a menu may drop «メニュー» (p.16, p.63). No space inside the brackets (p.33), one outside.
- **«「」» for text to type, text from a file and what the meter shows** (MS-SG p.28–29 for text to enter and text in a file; Sanwa PC7000 JP PDF p.7 «「dSAPO」», KEW1062 JP PDF p.11 «「ＯＬ」表示» for display text): «「{ $text }」は数値ではありません», «マルチメーターは「{ $reported }」と報告しています».
- **«『』»** only for book and document titles (MS-SG p.29); the app has none.
- **«“ ”»:** not used. MS-SG p.27 sets on-screen text in them; the meter makers use «「」» for that, and one quote mark is simpler.
- The straight `'…'` around command lines and Windows device names stays as it is (MS-SG p.54: quotes around a placeholder stay). The backquotes in `read-error-*` stay too.
- Keys take no brackets (MS-SG p.56).

### Spaces between scripts (ruling 3)

- One half-width space between a Japanese character and a Latin letter or digit, either side: «マルチメーター UT61E+ が接続されました», «3 回目», «500K 件のサンプル» (MS-SG p.32). The meter manuals often leave it out («Kタイプ»): print layout, not software.
- **None** next to «、» and «。», inside brackets or quotes, around `/`, before a closing `:` `?` `!` `…`, and between a digit and `%` or `°` (MS-SG p.32–34). Between a digit and a unit symbol, follow the English as written (§6).
- **None between two Japanese characters** (MS-SG p.33), except inside katakana compounds (§3).
- **Use U+0020.** egui breaks Japanese lines between any two kana or kanji anyway; U+00A0 would only glue a Latin run to the Japanese around it.
- **Placeables:** a space next to a placeable that holds a number, a model name, a path, a key name, a unit or a mode label (most mode labels are Latin: `DC V`, `Duty %`); none next to one that holds a whole Japanese sentence (`$warning`, `$steps`), or where punctuation already separates them.

### How egui breaks Japanese lines

epaint 0.37 (`text/unicode.rs:49–71`) treats U+3040–U+30FF and the CJK ideograph blocks as breakable before any character, and keeps the characters of its list off a row's start: «、» «。» «」» «』» «ー», the small kana («ッ», «ャ», «ァ»…), «・», «…», and the half-width `)` `:` `?` `!` `%`. Punctuation stays off a line start, but a bug in epaint 0.37 (`text_layout.rs`, the `pre_cjk` break) lets «ー» and the small kana start one after kana or kanji («マルチメ» / «ーター»); nothing in the text can prevent it. Opening «「» and «[» are not held to the next character: avoid long bracketed names at the end of a wrapped notice line.

## 5. Numbers

**The app keeps the English decimal point in every number it formats itself**, as for every language (`docs/localization-design.md`, "Numbers keep their point"). Japanese uses the point anyway (MS-SG p.27 «1.5 インチ»).

1. **Half-width Arabic digits wherever the English has digits** (MS-SG p.22), kanji numerals only in fixed words («もう一度», «一部», «一時的», p.22).
2. **No thousands separator:** write `1000 V`, `40000`, `500K` as the English does. The spec check rejects regrouped numbers. This departs from MS-SG p.24 and from the manuals («87,328 件», Sanwa PC720M JP PDF p.8). Never «万» grouping.
3. **Typed examples keep the point and the half-width commas:** the hover `(e.g. 3.3, 5, 12)` shows what to type, and the reference-values field reads commas as separators: «(例: 3.3, 5, 12)».
4. **Counters** (no approved list; MS-SG p.18 shows «5個»): «件» for readings, samples, records and log entries («{ $count } 件»); «個» for markers, files and other countable things; «回» for times, beeps, presses and crossings; «回目» for an ordinal («{ $attempt } 回目»); «秒», «分» for durations in words. A space between the number and the counter (MS-SG p.32 «第 3 章»): «{ $samples } 件のサンプル».
5. **Gaps the app leaves (for the developers, not the translators):** `MB`/`KiB` arrive preformatted, which reads fine in Japanese; the `K` suffix (`500K`) reads as English; time units in `≈194 MB, about 14 h` and the axis's `1m 5s` stay English symbols. Leave the placeables as they are.

**Percent:** no space: «10%», «{ $percent }%» (MS-SG p.34). The spec's `10%–100%` is notation: keep it.

## 6. Units and AC/DC

- **Unit symbols never change,** except in the spoken reading's unit names (ruling 17): V, mV, A, mA, µA, Ω, kΩ, MΩ, Hz, F, °C, °F, W, VA, Wh, Ah, dBm, dBV, s, ms, h, V·s, mAh. Keep `°C`/`°F` as the app writes them; the manuals' «℃» would fail the spec check and look different from the reading.
- **Space between a number and its unit:** follow the English as written; spec text keeps the manual's own spacing (`10MΩ`), which the spec check ignores. MS-SG p.34 puts a space before a unit («3 kg»), which matches the English's prose.
- **Time words in prose:** where the English writes words ("17 minutes", "2 seconds"), write «17 分», «2 秒»; where it writes a symbol, keep the symbol.
- **The meter's AC/DC labels stay as the meter prints them:** `DC V`, `V AC`, `AC mA`, `AC+DC`, `ACV`, `DCV`, `LoZ V~`. The Japanese manuals do the same in their tables («DCV直流電圧», Sanwa CD772 JP PDF p.25).
- **In prose and table titles, spell the words out:** «直流電圧», «交流電圧», «直流電流», «交流電流» (KEW1062 JP PDF p.9), and «交流+直流電圧» for AC+DC (ruling 10).
- **True RMS:** «真の実効値», or «真の実効値 (True RMS)» once in a note (Sanwa PC7000 JP PDF p.4); "rms" after a number (`250V rms`) is notation: keep it.
- **OL** is «オーバーレンジ» in words (KEW1062 JP PDF p.11 «オーバーレンジ表示 …「ＯＬ」表示»); «過負荷» is for overload *protection* only.

## 7. What the meter prints stays as it prints it

The Japanese manuals print every annunciator and key name exactly as on the meter and explain it in Japanese beside it: «HOLD（データホールド）ボタン», «RANGE（レンジホールド）ボタン» (Sanwa CD800a JP PDF p.1), «SELECT (PRINT)キー» (KEW1062 JP PDF p.22), «表示部に“MIN” “MAX” “AVG”が表示されます» (KEW1062 JP PDF p.24), «オーバーレンジ表示 …「ＯＬ」表示» (KEW1062 JP PDF p.11).

So:
- **Status flags** (`lib-names.ftl`) stay as they are: HOLD, REL, AUTO, MIN, MAX, AVG, LOW BAT, HV!, P-MAX, P-MIN, LEAD ERR, COMP, REC, LoZ, VOID. Their spoken forms (`reading-spoken-*`) are Japanese: «データホールド», «オートレンジ», «電池消耗», «テストリード誤挿入», «高電圧警告».
- **Key names, display text and menu paths** in activation steps and hovers stay as printed: SELECT, SEL, ZERO/ᛒ, Hz/Duty△/ᛒ, Tab⇌, △/ᛒ, 1ms PEAK, MAX MIN, SEND, RS232, PC, BLE, SETUP -> Communication -> ON, «Org», dSAPO, APO.oF, BT, ((D)). Translate around them in the manuals' words: «X キーを押します» (press), «X を長押しします» or «X を 1 秒以上押します» (hold; Sanwa PC7000 JP PDF p.6 defines a short press as «押す» and a long one as «1 秒以上押す»), «X を押しながら電源を入れます» (KEW1062 JP PDF p.43), «ディスプレイに ᛒ が表示されるまで». "key" → «キー», "button" → «ボタン», as the English says.
- **Mode qualifiers:** keep LPF and REL (`qualifier-lpf-mode = LPF { $mode }`, `qualifier-mode-rel = { $mode } REL`). "Peak" → «ピーク»: «{ $mode } ピーク».
- **Brands, model names and trademarks** unchanged: UNI-T, Voltcraft, ZOTEK, ZOYI, BSIDE, ANENG, EEVblog, Brymen, OWON, part numbers, AutoCheck™. "Mock UT61E+" → «UT61E+ (シミュレーション)».

## 8. Command lines, options, files and keys

- **Verbatim**, as for fr: every `--flag` and its values, `dmm-cli`, `dmm-gui`, subcommands, quoted command lines, device and step ids, setting ids, settings-file keys, environment variables, file names and patterns, CSV column names and JSON keys, the backquoted text in `read-error-*`, RFC 3339, the password `0000`. The translation check fails a message that loses a `code` span or an `--option`. Put a space between them and the Japanese around them (§4).
- **clap's words** are translated too (`shared.ftl`): «使用法:», «コマンド:», «引数:», «オプション:», «[既定値: { $value }]», «[指定可能な値: { $values }]», «エラー:», «ヒント:». Value names in capitals (`<FORMAT>`) and the usage line's syntax stay as clap writes them; in errors, keep the names in their straight quotes, as typed. CLI headings in capitals («CONFIGURATION:») become plain headings: «構成:», «環境変数:».
- **Windows and macOS names** must match what a Japanese system shows (ruling 16). 'CP2110 USB to UART Bridge' is the device's own name and stays English.
- **Computer key names** are messages of their own (`shortcuts-key-*`): ruling 14. MS-SG p.56 capitalises each word and joins them («PageUp», «CapsLock»); p.16 lets a tight label drop «キー». Letters, digits, F1–F12 and macOS's ⌘ ⇧ ⌥ stay.

## 9. Length budget

Each kana or kanji is about as wide as two Latin letters. Kanji words come out narrower than the English («接続» against "Connect"), katakana loanwords wider («エクスポート…» against "Export…").

- `cargo test -p dmm-shared long_labels -- --ignored --nocapture` counts characters and will under-report Japanese width: count a kana, kanji or full-width mark as 2 when reading it, and judge on screen (verify-gui with a Japanese font installed, §13).
- Watch the places fr watched: the top bar (Connect, Disconnect, Resume, Pause, Clear, Reconnecting…, the badges, Help & Feedback, the update link), the graph chips, the mode line («デューティー比 %» is the risky one: fallback «Duty %»), the Statistics column, the Settings chips.
- **Shorten** by choosing the kanji word over the katakana one where both are sourced, by dropping «キー» after a key name or the space inside a katakana compound (MS-SG p.16–17), never by cutting a word or adding a period.
- **The terminal:** a Japanese character takes two cells. `cli/help.ftl`'s CONFIGURATION/ENVIRONMENT lines aren't wrapped by clap; keep them under about 40 Japanese characters.

## 10. Placeables, word order and plurals

Japanese has no gender, number or article to agree, and its word order puts the verb last. What matters instead:

- **Reorder freely** (MS-SG p.54: placeables may move when they can be told apart): "{ $series } in { $unit } on a right axis" → «右側の軸に { $series } ({ $unit })».
- **A noun with a model name:** «マルチメーター { $model }» reads better than a bare model number: «マルチメーター { $model } への対応は検証済みです».
- **Particles after a Latin placeable:** a space before the particle («{ $model } が», «{ $file } から»).
- **Numbers take a counter** (§5): «{ $count } 件の測定値», never «{ $count } 測定値».
- **Status lines in label: value form:** «レンジ: { $range } (マニュアル)», «アラーム オン: { $limits }».
- **`$link`, `$side`, `$state` and the other selectors** have one variant per value: translate each in full: [usb] «USB ケーブルが接続されています», [adapter] «Bluetooth アダプターが接続されています», [radio] «Bluetooth リンクが確立されています».
- **`$setting` in `connection-switch-failed`** is a lowercase CLI id (hold, rel, minmax, peak): «{ $setting } を切り替えられませんでした: { $error }».
- **Inserted reasons** (`$reason`, `$error`, `$detail`) may be Japanese, or English from the OS or a parser: put them after a colon, never in the middle of a sentence.

**Plurals (ruling 15).** `intl_pluralrules` gives Japanese only `other` (`rules.rs`, cardinal `ja` → `PluralCategory::OTHER`), and `plural_forms("ja")` in `crates/dmm-shared/src/l10n/translations.rs` will return {other}. So a plural select keeps one variant: `{ $samples -> *[other] エクスポートされていないサンプル { $samples } 件が破棄されます。 }`, or, simpler, drop the select and write the sentence once, as long as every variable of the English still appears in the text (the check compares variables). Nested selects in `export-import.ftl` collapse the same way; `args-too-few-values`'s `[0]` variant goes too. Selectors that are not plurals (`[usb]`, `[yes]`, `[big-meter]`…) keep all their variants.

## 11. Fluent specifics

- **Never change** message IDs, attribute names (`.hover`, `.a11y`, `.hover-disabled`, `.wayland`, `.gesture`, `.about`, `.help`, `.value-replay`), variable names or non-plural selector keys. Keep the `*` default on the same variant. Translate every variant's text.
- **You may** reorder placeables, drop a plural select (§10), or add a selector on a variable that exists.
- **Message references stay references:** `{ project-link }`, `{ device-auto }`, `{ bluetooth-setting }`, `{ error-bluetooth-reset-gesture }`.
- **String literals** are layout; adapt them to Japanese punctuation: `reading-spoken-separator` `{", "}` → `{"、"}`; `graph-plot-a11y-also-separator` `{", "}` → `{"、"}`; `graph-histogram-a11y-separator` `{", "}` → `{"、"}`; `graph-histogram-a11y-interval-separator` `{"; "}` → `{"。"}`; `graph-plot-a11y-separator` `{" "}` (between sentences) → `{""}`, since a sentence ending in «。» needs no space. The two leading spaces in `connect-not-identified-name-it` (`{"  "}`) are CLI indentation: keep them.
- **Multi-line messages:** keep the indentation and the line breaks; one activation step per line. Japanese lines come out shorter; re-wrap a notice only if a line would be very short.
- **`.a11y` text** is read aloud: units in words where the comments ask («摂氏度»), «と» for «&», arrow keys as «左右方向キー», no symbols a screen reader may misread (MS-SG p.15 names & + ~).
- **Trailing and leading spaces** are dropped by Fluent; write `{" "}` where one matters.
- **Never translated:** the `#` comments, and `settings-language-no-japanese-font` and `settings-language-no-chinese-font`, which are always shown in English.

## 12. Spec text (lib.ftl: bands, inputs, notes)

Optional for shipping (`translations::optional()`), as for every language.

- Keep every number, sign, ±, ≤, ≥, <, >, ≈ and ~, and each unit symbol right after its number. The spec check falls back to English on any mismatch.
- Keep the exact sense: comparisons, ranges, «標準» / «TYP.», «定格», «約», «以上», «以下», «最大». The check can't catch a flipped comparison.
- Use the manuals' wording: «規定されず» / «規定なし» (Sanwa PC710 JP PDF p.29); «読み取り値» in accuracy, «%rdg» in a formula (Sanwa CD772 JP PDF p.25); «カウント» for digits («10 カウントを加算», «残留 ≤10 カウント»); «レンジの 10%–100%» (Fluke 80V JP PDF p.43); «過負荷保護»; «入力インピーダンス»; «クレストファクター»; «温度係数»; «開放電圧»; «参考値» (no approved source).
- Table titles keep the manual's numbering and letters: «(1) 交流電圧», «A. 直流電圧», «K. 温度 1-1. 摂氏».

## 13. Glyphs and fonts

The GUI draws Japanese with a system font chosen for the `ja` locale (`crates/dmm-gui/src/app/language.rs`, `font_locale`, which passes the language id through), so Han characters take Japanese shapes. Latin text and the meter's symbols still come from the bundled fonts (Ubuntu-Light, Hack), so ᛒ, △, ⇌, Ω, µ, σ, ∫, ≈, …, ⏸/▶ need nothing new.

Characters this sheet asks for beyond Latin: hiragana and katakana (U+3040–U+30FF), kanji of the 常用漢字表, «、» «。» «「» «」» (U+3001, U+3002, U+300C, U+300D), «ー» (U+30FC). Noto Sans CJK JP covers them all. The review screenshots were taken with Noto Sans CJK JP installed.

When no system font draws Japanese, the GUI shows `settings-language-no-japanese-font`, in English (`script_probe` looks for «あ»).

## 14. Words to avoid

| Avoid | Use | Why |
|---|---|---|
| あなた, 弊社 | no pronoun | MS-SG p.22–23 |
| 本器 (the meter) | マルチメーター | the manuals' "this instrument"; the app talks about a meter it is connected to |
| マルチメータ, フィルタ, アダプタ, ドライバ | マルチメーター, フィルター, アダプター, ドライバー | MS-SG p.35 long-vowel rule (ruling 5) |
| 範囲 (measurement range) | レンジ | every maker; 範囲 is MS-T's generic range |
| 精度, 正確性 | 確度 | every maker's spec tables; MS-T accuracy => 正確性 is the general sense |
| 解像度 | 分解能 | 解像度 is screen resolution |
| 過負荷 (for OL) | オーバーレンジ | KEW1062 JP PDF p.11; 過負荷 is overload protection |
| キャパシタンス | 静電容量 | Sanwa and Fluke; KEW alone says キャパシタンス |
| 測定コード | テストリード | Sanwa and Fluke; KEW alone says 測定コード |
| レコーディング, 録画 | 記録 | MS-T recording is audio/video |
| サポート (a meter's support) | 対応 | MS-T support is customer support |
| パン (pan) | スクロール | MS-T pan is the hand tool |
| ライブ (live wire) | 活線 | ライブ is the graph's LIVE chip |
| 閾値 | しきい値 | MS-T; kana form |
| 既に | すでに | kana form, one spelling across the files (workflow review, 2026-11-02) |
| モデル (a meter's model) | 機種 | Sanwa CD772 JP PDF p.23; モデル is MS-T's generic model |
| 未エクスポート, 未保存 | エクスポートされていない, 保存されていない | MS-SG p.9 |
| が可能です, 推奨します, 再度 | できます, お勧めします, もう一度 | MS-SG p.7–8 |
| ～させる | the plain verb | MS-SG p.39–40 |
| ～してもよろしいですか? | ～しますか? | MS-SG p.46 |
| ～ (wave dash) | から, or `-` | MS-SG p.26 |
| — (em dash) | 。、: or ( ) | MS-SG p.26 |
| “ ” | 「」 or [ ] | ruling 7 |
| ℃, ℉, μ (U+03BC) | °C, °F, µ (U+00B5) | the app's own unit text; spec check |
| 2,000 / 万 grouping | 2000 | spec check; consistency with readings |
| full-width Latin and digits (ＯＬ, ３) | half-width | MS-SG p.18, p.22 |

## 15. How Japanese differs from English

The patterns of English that a native review of this translation corrected most often, as principles for any new text.

1. English prepositions (on, at, in, of, per) name a relation loosely; do not map them to 上で, に, にある or あたり. State the actual relation with a verb or a Japanese frame.
2. Drop English's small pointing and emphasis words (own, itself, there, then, that, already). Japanese leaves what is clear unsaid; where the word carries meaning, state that meaning directly.
3. Build detail into the sentence instead of appending it: what English adds in parentheses, after a dash or in a follow-up sentence becomes a modifier before the noun or a linked clause.
4. Turn noun phrases into verb clauses. Chains joined by の or への and nominalised actions read stiff; a 「～を～する」 clause or a relative clause reads natural.
5. Check that the subject can do what the predicate says. English lets a thing stand for its owner (a code "sends frames", a value "is partial"); Japanese names the real actor.
6. Do not link clauses with a colon or end one clipped. Write out the condition or the verb: 「～場合は」, 「～すると」, 「～を指定する」.
7. Put place and means first and the object right before the verb. "Use X to do Y" becomes 「X で Y する」, naming the real action instead of 使用する; a contrast comes before the point (A ではなく B).
8. A modifier attaches to the nearest thing it can: reorder or split whenever it could reach the wrong noun, or a purpose like 「～には」 could extend over a later clause.
