# Japanese glossary and style sheet: what a reviewer should rule on first

Draft of 2026-11-02. No native reviewer has read it, so every answer below is the drafter's provisional choice. Sources are in `STYLE.md` §1. Glossary rows cite document and PDF page. MS-SG is Microsoft's Japanese style guide and MS-T is Microsoft Terminology.

## Form (STYLE.md §0)

| # | Question | Draft choice | Alternative | Source / reason |
|---|---|---|---|---|
| 1 | Register | No pronoun. です・ます for notices, errors and steps ("～してください。"). Noun phrases for buttons, menus, chips and labels. Plain form for checkboxes and for hovers that name an action. dmm-cli option help in plain form; runtime lines in です・ます | です・ます everywhere, hovers and CLI help included | MS-SG p.22–23 (pronouns), p.42–43 (levels), p.58 (command list "ヘルプを表示する") |
| 2 | Punctuation width | Full-width 、。「」 in Japanese sentences; half-width `: ? ! ( ) [ ] / …` | Full-width ：（）？！ throughout, as the meter manuals print | MS-SG p.24–32 |
| 3 | Space between Japanese and Latin/digits | Yes, one U+0020 ("Word を", "第 3 章", "Esc キー"), with MS-SG's exceptions | No spaces, as the manuals print ("Kタイプ") | MS-SG p.32–34 |
| 4 | Katakana long vowel | MS rule: マルチメーター, フィルター, アダプター, ファクター, モーター, デューティー; exceptions コネクタ, トランジスタ, メモリ | The Japanese makers' JIS habit: マルチメータ, フィルタ, クレストファクタ (Sanwa, KEW) | MS-SG p.35–39; Fluke JP writes マルチメーター |
| 5 | Naming on-screen controls | Half-width brackets always: "[接続] をクリック", "[設定] (⚙) で", even where the English has no quotes | 「」 around them, as Apple's ja pages do (「システム設定」) | MS-SG p.28, p.61 |
| 6 | Other quoted text (typed text, meter display text, what the meter reports) | 「」 ("「Org」と表示されます"); no “ ” | “ ” for on-screen text, as MS-SG p.27 says | Sanwa PC7000 JP PDF p.7 「dSAPO」; KEW1062 JP PDF p.11 「ＯＬ」 |
| 7 | Ellipsis and progress labels | U+2026 "…", as the English mostly writes it; "～中…" ("再接続中…") | Three periods "..." per MS-SG; "～しています…" | MS-SG p.26, p.41 |
| 8 | Key names (`shortcuts-key-*`) | Ctrl, Shift, Alt, Space, Esc, Home, End kept; Left/Right → ←/→; Plus/Minus → プラス/マイナス; in prose "Esc キー", "←キー" | Spelled-out 左方向キー/右方向キー in the key column too; "+"/"-" symbols for Plus/Minus | MS-SG p.51, p.56–57; Plus/Minus unsourced |
| 9 | AC+DC in titles and prose | 交流+直流電圧 (the English's order) | 直流+交流電圧, the makers' order | Sanwa PC7000 JP PDF p.2; KEW1062 JP PDF p.6 "DC+AC" |
| 10 | The app's badges | 検証済み, 一部検証済み, 試験的, スケール, 上限超過, 下限未満, ライブ | Keep the English capitals | No capitals in Japanese; app text, not LCD text |
| 11 | Counters | 件 (readings, samples), 個 (markers, files), 回 / 回目 (times, attempts) | 個 for samples too | MS-SG p.18 ("5個"); Sanwa PC720M JP PDF p.8 "87,328 件" |
| 12 | Spoken units | ボルト, アンペア, オーム, ヘルツ, ファラド, ジーメンス, ワット, ボルトアンペア, ヘンリー, 秒, パーセント; prefixes joined whole (キロオーム, メガオーム); 摂氏度/華氏度 | Fluke 80V's "メグオーム" | Fluke 80V JP PDF p.12; Sanwa PC7000 JP PDF p.3 |

## Terms

| # | English | Draft choice | Alternative | Source / reason |
|---|---|---|---|---|
| 13 | reading | 測定値 | 読み取り値 (kept for "of reading" in accuracy) | KEW1062 JP PDF p.36; Sanwa CD772 JP PDF p.25 "rdg：読み取り値" |
| 14 | range / accuracy / resolution | レンジ / 確度 / 分解能 | 範囲 / 正確性 / 解像度 (MS-T, generic senses) | Every maker, e.g. Sanwa CD772 JP PDF p.3, KEW1062 JP PDF p.6, p.13 |
| 15 | digits/counts in accuracy | カウント ("10 カウントを加算") | dgt as the formulas print | Fluke 80V JP PDF p.15; Sanwa CD772 JP PDF p.25 |
| 16 | OL / overload | オーバーレンジ for OL; 過負荷 only for overload protection | レンジオーバー (Sanwa) | KEW1062 JP PDF p.11; Sanwa PC7000 JP PDF p.33 |
| 17 | dial / test lead / display | ファンクションスイッチ / テストリード / ディスプレイ | ロータリー･スイッチ (Fluke) / 測定コード (KEW) / 表示器 (Sanwa) | Sanwa PC7000 JP PDF p.2; Sanwa CD772 JP PDF p.3; Fluke 80V JP PDF p.7 |
| 18 | key / button on the meter | As the English says: key → キー, button → ボタン | One word throughout | KEW1062 JP PDF p.22 (キー); Sanwa CD772 JP PDF p.7 (ボタン) |
| 19 | capacitance / duty cycle | 静電容量 / デューティー比 ("Duty %" → "デューティー比 %", fallback "Duty %") | キャパシタンス (KEW) / デューティ比 (Sanwa PC7000, KEW) | Sanwa CD772 JP PDF p.3; Sanwa CD800a JP PDF p.1 |
| 20 | support (of a meter by the app) | 対応 | サポート (MS-T, customer support) | KEW1062 JP PDF p.9 "通信対応" |
| 21 | recording / replay / simulated | 記録 / リプレイ / シミュレーション ("UT61E+ (シミュレーション)") | レコーディング (MS-T, audio) / 再生 (MS-T playback) / 模擬 | KEW1062 JP PDF p.24; MS-T |
| 22 | trace (a series' line) | トレース | 線 (MS-T line) | MS-T trace (also the debugging sense) |
| 23 | machine-translated | 自動翻訳 | 機械翻訳 (the everyday word) | MS-T machine translation => 自動翻訳 |
| 24 | scale / scaling | スケール (chip, badge) / スケーリング (prose); scale factor 倍率 | スケール ファクター | MS-T scale, scaling, scale factor |

## Terms with no approved source (the drafter's guess)

| English | Draft | Alternatives |
|---|---|---|
| burden voltage | バーデン電圧 | 電流測定時の電圧降下 |
| Live (live-wire detection mode) | 活線 | 充電部 |
| high/low limit; HI/LO LIMIT badges | 上限 / 下限; 上限超過 / 下限未満 | 下限超過, 下限割れ |
| COMP High / Low | COMP 上 / COMP 下 | COMP ハイ / ロー |
| Colorblind (preset) | 色覚サポート | 色覚多様性 |
| Plus / Minus key names | プラス / マイナス | + / - |
| others | internal temperature 内部温度, inductance インダクタンス, apparent power 皮相電力 (VA), power factor 力率, absolute 絶対値, for reference only 参考値, in Bluetooth range 通信範囲内, identify 識別, verified/partly verified/experimental 検証済み/一部検証済み/試験的, time window 時間幅, crosshair 十字線, data gap 欠落, statistics 統計, Visible 表示範囲, hysteresis ヒステリシス, weak text 補助テキスト, volt-ampere ボルトアンペア, henry ヘンリー, Compass/Scope コンパス/オシロスコープ | |

## Also open

- **Unverified Windows name:** 'USB Input Device' is drafted as USB 入力デバイス (from MS-T input device). Check it on a Japanese Windows. ヒューマン インターフェイス デバイス and ほかのデバイス come from Microsoft's ja-jp support pages. The macOS names (システム設定 > プライバシーとセキュリティ > 入力監視 / Bluetooth) come from Apple's ja-jp pages.
- **Unreachable makers:** Hioki's manual downloads answered 403 and Custom's site did not answer. A reviewer with a Hioki manual could confirm the terms the draft takes from Sanwa and Kyoritsu.
- **Plurals:** ja has only `other`. The draft drops plural selects where every variable still appears (STYLE §10). Confirm that reads well in the nested `export-import.ftl` prompts.
