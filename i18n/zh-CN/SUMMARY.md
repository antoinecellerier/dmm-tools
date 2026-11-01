# Simplified Chinese glossary and style sheet: what a reviewer should rule on first

Draft of 2026-11-01, no native reviewer yet: every answer below is the drafter's provisional choice. Sources: `STYLE.md` §1; glossary rows cite document and PDF page.

## Form (STYLE.md §0)

| # | Question | Draft choice | Alternative | Source / reason |
|---|---|---|---|---|
| 1 | Pronoun and register | «你», left out wherever possible; bare verbs on buttons; «请» only where a step would sound curt | «您» throughout (UT195 manual) | MS-SG p.27 "use 你 instead of 您" |
| 2 | Punctuation width | MS software set: half-width `:` `!` `?` `( )` `…`; full-width `，` `。` `；` `、` and `“ ”` | Full-width throughout (`：` `（）` `！` `？`), as the UNI-T manuals and GB/T 15834 print | MS-SG p.28–37; half-width captions keep the Statistics column narrow |
| 3 | Space between Han and Latin/digits | Yes, one U+0020 («万用表 UT61E+ 已连接», «第 3 次尝试»); none next to a placeable that may hold Chinese | No spaces («按SELECT键»), as the manuals print | MS-SG examples p.11, p.19, p.29, p.51; placeholders p.45 |
| 4 | Quotes around names of on-screen controls | Always «“设置”», «单击“连接”», even where the English has none | Only where the English quotes | MS-SG p.21 §4.1.7 b |
| 5 | Key names (`shortcuts-key-*`) | Ctrl, Shift, Alt, Esc, Home, End kept; «空格键», «向左键»/«向右键», «加号»/«减号» | Keep every English key name (Chinese keyboards print them) | MS-SG p.50–51; MS-T plus/minus sign |
| 6 | AC+DC in titles and prose | «交流+直流电压»; labels and notes keep AC+DC | Keep «AC+DC电压» as UT61E+ heads its table (PDF p.16L) | «交直流» means AC or DC in UNI-T's Chinese |
| 7 | The app's badges | «已验证», «部分验证», «实验性», «换算», «超上限», «超下限», «实时» | Keep the English capitals | Chinese has no capitals; app text, not LCD text |
| 8 | `°C` vs `℃` | Keep `°C`/`°F` as the app writes them | — | Spec check; the reading shows `°C` |
| 9 | Percent spacing | «10%», no space | — | MS-SG p.30 |

## Terms

| # | English | Draft choice | Alternative | Source / reason |
|---|---|---|---|---|
| 10 | reading | 读数 | 测量值 (for the number itself) | UT61E+ PDF p.6R, p.15L; UT181A PDF p.8 |
| 11 | range | 量程 | — (never 范围 for the meter's range) | every manual, UT61E+ PDF p.15R |
| 12 | accuracy / resolution | 准确度 / 分辨力 | 精度 / 分辨率 | UT61E+ table headers PDF p.15R; MS-T accuracy => 准确度; 分辨率 is also screen resolution |
| 13 | digits/counts in accuracy | 字 («加 10 个字») | counts kept in Latin | UT61E+ PDF p.15L, p.17R; UT71 PDF p.35 |
| 14 | OL / overload | 超量程 for OL; 过载 only in overload protection | 过载 for both | UT71 PDF p.13 «OL 超量程提示符» |
| 15 | simulated / mock meter | 仿真 («UT61E+ 仿真») | 模拟 (MS-T simulation), 演示 | 模拟 also means analog in meters (UT61E+ «模拟条») |
| 16 | scale / scaling (the app's factor + offset) | 换算 | 缩放 (MS-T scaling) | 缩放 is already zoom; no approved source for 换算 |
| 17 | replay | 回放 | 重播 (MS-SG p.48 example) | 重播 is the broadcast sense |
| 18 | graph / trace / plot | 图表 / 曲线 / 绘图 | 曲线图 for plot | MS-T chart, trendline, plot |
| 19 | alarm / high & low limit | 报警 / 上限、下限 | 告警 (MS-T) | UT8805A PDF p.43–44; UT71 PDF p.13; MS-T alarm => 闹钟 is a clock |
| 20 | recording | 记录 | 录制 (MS-T, audio/video) | UT181A PDF p.23–24 |
| 21 | Always on top / machine-translated | 窗口置顶 / 机器翻译 | 前端显示 / 自动翻译 (both MS-T) | everyday desktop words vs MS-T |
| 22 | standard deviation | 标准差 | 标准偏差 (MS-T, UT8805A PDF p.6) | UT8805A PDF p.42; shorter |

## Terms with no approved source (the drafter's guess)

| English | Draft | Alternatives |
|---|---|---|
| inrush current | 浪涌电流 | 启动电流, 冲击电流, or keep «Inrush» |
| burden voltage | 负载电压 | 电流档压降 |
| hysteresis | 回差 | 迟滞 |
| Colorblind (preset) | 色盲友好 | 色觉友好 |
| Live (live-wire detection mode) | 火线 | — |
| Bluetooth link (built-in) | 蓝牙链路 | 内置蓝牙 |
| 4–20 mA loop current | 4-20 mA 环路电流 | 4-20 mA 电流环 |
| others | internal temperature 内部温度, inductance 电感, logic level 逻辑电平, sensitivity 灵敏度, nominal 标称, in Bluetooth range 在蓝牙范围内, identify 识别, crosshair 十字线, data gap 间断, Visible 可见部分, weak text 次要文本, Solar/Tachometer/Compass/Angle/Scope 太阳能/转速表/指南针/角度/示波器 | |

## Also open

- Unverified Windows names: 'USB Input Device' «USB 输入设备», and the HID group «人体学输入设备», which Microsoft's zh-cn pages and MS-T write «人机接口设备». Check on a Chinese Windows. «其他设备» and the macOS names come from Microsoft's and Apple's zh-cn support pages.
- Glyphs: the review machine has Droid Sans Fallback only; check shapes with Noto Sans CJK SC on a Chinese desktop.
- The drafter's code-gap note on CLI column padding does not apply: the padded columns are `read`'s stdout and setting ids, which stay English.
- Plurals: zh has only `other`; the draft drops plural selects where every variable still appears (STYLE §10). Confirm that reads well in the nested `export-import.ftl` prompts.
