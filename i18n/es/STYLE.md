# Spanish style sheet (es), draft

How the Spanish translation is written. It goes with `glossary.tsv` in this folder: where the two disagree, the glossary wins for a term and this sheet wins for form. `cargo test -p dmm-shared translations` checks what can be checked mechanically ([development guide](../../docs/development.md#translations)).

One Spanish serves every Spanish system locale: the app matches by language subtag, so `es-ES`, `es-MX`, `es-AR` and `es-419` all read these files. Write a neutral international Spanish that reads naturally in Spain and in Latin America (§2, "One Spanish for every region").

**Status: draft, no native reviewer yet.** Every ruling in §0 is provisional: it is the recommended choice, with the alternative in `SUMMARY.md`. Rulings marked *(decided)* were made for every language before this draft and are not open.

## 0. Rulings (all provisional unless marked decided)

1. **«tú», never «usted», «vos» or «vosotros».** Buttons, chips, menu items, checkboxes and action hovers use the infinitive («Conectar», «Exportar…», «Restablecer zoom», «Congelar la lectura actual»). Notices, error advice and activation steps use the tú imperative («Comprueba…», «Enciende el multímetro», «Mantén presionado SELECT…»). Both Microsoft guides recommend «tú» (MS-SG-ES PDF p.34, MS-SG-MX PDF p.32) and the infinitive for what the user tells the program (MS-SG-ES PDF p.34, MS-SG-MX PDF p.31). Every maker manual uses «usted»: SUMMARY question 1.
2. **Neutral vocabulary where Spain and Latin America differ:** «equipo» for the computer (never «ordenador», «computadora» or «PC», whose gender differs); «mouse» (Spain: «ratón»); «archivo», never «fichero»; «Configuración» for Settings (both guides). §2 lists the rest.
3. **«presionar» for pressing a key or button** («presiona», «mantén presionado»), «hacer clic en» for the mouse. Alternative «pulsar».
4. **Sentence case** in every label: «Restablecer zoom», «Contraste alto», «Teclado y mouse», «Ayuda y comentarios» (MS-SG-ES PDF p.25, MS-SG-MX PDF p.23). Capitals keep their accents (MS-SG-ES PDF p.26).
5. **Opening ¿ and ¡ always**; no space before : ; ? ! (MS-SG-ES PDF p.35, MS-SG-MX PDF p.32). Colons inside notation stay as the English writes them («Y:fijo», «Media=», «∫:»); captions are «Mín.:», «Máx.:», «Media:».
6. **Quotation marks “ ”** (U+201C/U+201D), ‘ ’ inside them (MS-SG-ES PDF p.38, MS-SG-MX PDF p.35). Straight `'…'` around command lines and Windows device names stay as they are. Alternative « » (the Spanish Academies' first choice, which both guides mention and set aside).
7. *(decided)* **English decimal point everywhere**, typed numbers in prose included («0.6 s», «3.3»); **no thousands separator** («1000 V», «60000»).
8. **U+00A0 between a number and its unit, and before %, in prose you write** («2 s», «15 minutos», «1 %»). Notation follows the English as written («10%–100%», `Duty %`, `{ $percent }%` in the histogram).
9. *(decided)* **Meter flags stay as printed** (HOLD, REL, AUTO, LOW BAT, LEAD ERR, HV!, VOID…); their spoken forms are Spanish («retención», «rango automático», «batería baja»).
10. **The app's own badges** are translated, in capitals with accents: ESCALA, LÍM. SUP., LÍM. INF., EN VIVO, VERIFICADO, PARCIALMENTE VERIFICADO (fallback VERIF. PARCIAL), EXPERIMENTAL.
11. *(decided)* **AC/DC/AC+DC stay as printed** in mode labels and notes. Table titles and prose use full words: «Tensión continua», «Tensión alterna», «Tensión alterna + continua». **Never «CA»/«CC»** («c.a.»/«c.c.»), though Fluke, Amprobe and Truper print them.
12. *(decided)* **Glossary entries are full terms**, no optional words in parentheses. A parenthesis in an entry is part of the text («ganancia de corriente (hFE)», «potencia aparente (VA)»).
13. **Terms:** «tensión» (not «voltaje»); «exactitud» (not «precisión»); «lectura» for a reading, «medición» for the act; «rango» for a range; «dígitos» in accuracy and residual notes, «cuentas» for display counts; «botón» for a meter key, «tecla» for a computer key; «EN VIVO» / «en vivo» for live.
14. **Key names** (`shortcuts-key-*`): Ctrl, Mayús, Alt, Espacio, Esc, Izquierda, Derecha, Inicio, Fin, Más, Menos. In prose, Microsoft's full names where room allows: «Flecha arriba», «Flecha abajo», «Re Pág», «Av Pág», «barra espaciadora» (MS-SG-ES PDF p.55–56, MS-SG-MX PDF p.51–52).
15. **Plurals:** fluent-bundle gives Spanish `one` (exactly 1) and `other` (0 included): «0 muestras», «1 muestra», «2 muestras». Write `[one]` and `*[other]` only (§10).
16. **Nothing agrees with the user's gender:** no adjective or participle refers to the user (MS-SG-ES PDF p.17, MS-SG-MX PDF p.14). Rephrase impersonally or with «tú» verbs.
17. **Simple tenses:** the pretérito perfecto simple for what just happened («Se detectó…», «No se pudo…»), as MS-SG-ES PDF p.11 («si ya permitiste», not «si ya has permitido») and PDF p.39–40 ask; the -ra subjunctive («si el multímetro no respondiera»), which every region uses (MS-SG-MX PDF p.37).
18. **Terms with no approved source** (the rows marked so in `glossary.tsv`, listed in `SUMMARY.md`): take the glossary's value as drafted until a reviewer rules.
19. **OS names.** Windows, the same in Spain and Mexico: «Administrador de dispositivos», «Dispositivos de interfaz humana», «Otros dispositivos» (MS-T; MS-Support es-es and es-mx). macOS differs by region: «Ajustes del Sistema» (Spain) and «Configuración del Sistema» (Latin America), then «Privacidad y seguridad > Monitorización de entrada» and «… > Bluetooth» in both (Apple-ES, Apple-MX). Name both: «Ajustes del Sistema / Configuración del Sistema > Privacidad y seguridad > Monitorización de entrada». 'USB Input Device' stays **unverified** ('Dispositivo de entrada USB'): its message carries a `# UNVERIFIED Windows name` comment until someone checks a Spanish Windows.
20. **Spoken unit names** (`reading-unit-*`, read aloud only): the Academies' forms, «voltio», «amperio», «ohmio», «hercio», «faradio», «siemens», «vatio», «voltamperio», «henrio», «segundo», «por ciento»; prefixes «pico», «nano», «micro», «mili», «kilo», «mega», «giga», joined whole with no contraction («kiloohmio», «megaohmio»). The plural comes from the number as displayed: singular for exactly 1 («1.000 voltio»), plural otherwise («1.5 voltios», «0.000 voltios»). Alternative: the international names Mexico's standards use («volt», «ampere», «ohm», «watt»).

## 1. Sources

Every rule and term cites one of these. PDF page numbers are the PDF's own page index (as `pdftotext` counts pages), not the number printed on the page; in the two Microsoft guides they coincide. The files are in `references/l10n/es/`, kept out of the repo; `SOURCE.txt` there gives each URL.

| Key | Document | File |
|---|---|---|
| Fluke 80 Series V | Fluke 80 Series V Multimeters, Manual de uso, Rev. 3, 3/25 (52 pages) | `fluke-80-series-v-manual-es.pdf` |
| Fluke 287/289 | Fluke 287/289 True-rms Digital Multimeters, Manual de uso, Rev. 2, 3/09 (94 pages; logging, PC link) | `fluke-287-289-manual-es.pdf` |
| Fluke 117 | Fluke 110/113/114/115/117 True-rms Multimeter, Manual de uso (22 pages) | `fluke-110-117-manual-es.pdf` |
| Fluke 3000 FC | Fluke 3000 FC Wireless Multimeter, Manual de uso, Rev. 1, 9/16 (46 pages; pairing with a phone app) | `fluke-3000fc-manual-es.pdf` |
| UNI-T UT197 | UNI-T UT197 Manual de usuario (41 pages; Bluetooth app section on PDF p.38–39) | `uni-t-ut197-user-manual-es.pdf` |
| UNI-T UT195 | UNI-T UT195E/M/DS instrucciones de empleo (22 pages) | `uni-t-ut195-user-manual-es.pdf` |
| Amprobe AM-500 | Amprobe AM-500 User Manual / Manual de uso (71 pages; Spanish from PDF p.47) | `amprobe-am-500-manual-en-fr-es.pdf` |
| Truper MUT-39 | Truper Multímetro digital MUT-39 (10402), Instructivo (32 pages; Mexico) | `truper-10402-MUT-39-instructivo.pdf` |
| Truper MUT-202 | Truper Multímetro digital de gancho MUT-202 (10404), Instructivo (32 pages; Mexico) | `truper-10404-MUT-202-instructivo.pdf` |
| MS-SG-ES | Microsoft Spanish (Spain) Localization Style Guide (63 pages) | `microsoft-spa-esp-styleguide.pdf` |
| MS-SG-MX | Microsoft Spanish (Mexico) Localization Style Guide (62 pages) | `microsoft-spa-mex-styleguide.pdf` |
| MS-T | Microsoft Terminology Collection, `SPANISH.tbx`: one Spanish set, every entry marked for all Spanish-speaking regions (no separate es-ES / es-MX files) | `microsoft-terminology-SPANISH.tbx.gz` |
| Apple-ES, Apple-MX | Apple support, Mac User Guide in Spanish for Spain and for Mexico: mchl4cedafb6 (Monitorización de entrada), mchl211c911f (Privacidad y seguridad) | `os-names/apple-mac-help-*-es-es.html`, `*-es-mx.html` |
| MS-Support | Microsoft support, es-es and es-mx: touchscreen page (Administrador de dispositivos > Dispositivos de interfaz humana), Bluetooth page (Otros dispositivos) | `os-names/ms-*.html` |
| IEV | IEC Electropedia | electropedia.org returned 403 to every fetch (curl and WebFetch, 2026-11-01). No IEV term is cited. |
| egui | epaint 0.37 and its bundled fonts | Ubuntu-Light covers U+00A0–U+024F (á é í ó ú ü ñ ¿ ¡ « »), – —, “ ” ‘ ’ and …; see the French sheet §4 for how egui breaks lines |

Not used: other apps' translations, vendor app string files, community translations and machine translation.

The maker manuals are uneven. Fluke's are the most careful and the most consistent, and they read as Latin American Spanish («la PC», «presione»); Truper's are Mexican («multímetro de gancho», «perilla», «la terminal»). UNI-T's UT197 is a loose translation («medidor», «RMS real», «voltaje de AC»), and its UT195 manual is unusable («presión eléctrica» for voltage): the glossary cites UT195 only where it rejects a term. The Microsoft guides give the conventions (register, punctuation, error wording, key names); where the two guides differ, this sheet takes the form both regions read (§2).

## 2. Register and voice

- **Address the user as «tú»** (MS-SG-ES PDF p.34, MS-SG-MX PDF p.32: "the informal second person singular pronoun “tú” is recommended"). Never «usted», never the Rioplatense «vos», never «el usuario» (MS-SG-ES PDF p.33). The app speaks to one user: no plural address is needed; if one ever is, «ustedes», not «vosotros» (MS-SG-ES PDF p.34 gives «ustedes» as the neutral form). Leave out «tú» where the verb already says it.
- **Buttons, menu items, checkboxes, chips and shortcut-list actions:** the infinitive, or a noun where English uses one: «Conectar», «Exportar…», «Restablecer zoom», «Siempre visible», «Activar o desactivar la grabación». MS-SG-ES PDF p.34 and MS-SG-MX PDF p.31: "when the user is telling the program … what to do, the infinitive is used"; their UI examples «Guardar como», «Cancelar», «Seleccionar todo» (MS-SG-ES PDF p.25, p.56–59). Within one list, one form throughout.
- **Hovers that name what a control does** use the infinitive and stay short: «Abrir la conexión USB con el multímetro seleccionado (Ctrl+O)», «Congelar la lectura actual» (Fluke 287/289 PDF p.15 «Congela la lectura actual en la pantalla»).
- **Instructions in notices, steps and sentences** use the tú imperative: «Comprueba que esté seleccionado el dispositivo correcto en Configuración (⚙)», «Enciende el multímetro», «Haz clic en “Conectar” después de resolver el problema». MS-SG-ES PDF p.12 «Fíjate si está bien y prueba otra vez», PDF p.15 «haz clic en Cancelar y termina lo que tengas que hacer». Imperatives to watch: «mantén» (not «mantiene»), «haz», «pon», «sal».
- **Activation steps** (lib.ftl `activation-*`) are numbered imperatives: «1. Conecta el cable USB al multímetro», «2. Enciende el multímetro», «3. Mantén presionado SELECT…». The makers number steps the same way, in «usted» (Fluke 80 Series V PDF p.16 «1. Conecte un termopar tipo K a los terminales COM y…»).
- **Plain, everyday words** (MS-SG-ES PDF p.11–12, MS-SG-MX PDF p.7): «pedir», not «solicitar»; «dar», not «proporcionar»; «pero», not «sin embargo»; «sobre», not «acerca de»; «con», not «junto con»; «después», not «a continuación»; «usar», not «utilizar». For an argument or value the user gives on the command line (clap's "provided", "supplied", "given"), «indicar». No «Por favor» (the guides' examples drop it, MS-SG-ES PDF p.12).
- **Tenses** (MS-SG-ES PDF p.39–40, MS-SG-MX PDF p.37–38): simple present; the pretérito perfecto simple for what has happened («Se detectó el UT61E+», «No se pudo guardar»), which reads naturally in both regions, where the compound «se ha detectado» reads as Spain only. The future only for what really comes later. The -ra subjunctive («cantara»), never -se (MS-SG-MX PDF p.37; Spain reads -ra too).
- **«comprobar» for "check"** (MS-SG-ES PDF p.45, MS-SG-MX PDF p.44); «verificar» only for "is it true": the support levels («verificado»).
- **Possessives:** fewer than English (MS-SG-ES PDF p.23, MS-SG-MX PDF p.21): "Check your meter" → «Comprueba el multímetro».
- **No pronoun for the meter that changes by region:** «lo» for a masculine direct object, never «le» (no leísmo, MS-SG-MX PDF p.32): «Enciéndelo», «ponlo».
- **Error messages** (MS-SG-ES PDF p.48–52, MS-SG-MX PDF p.46–50):
  - The problem and what to do are separate sentences, joined by a period, not a colon or semicolon (MS-SG-ES PDF p.48). Drop exclamation marks.
  - Nominal forms in short messages, the verb "to be" left out: «Disco lleno.», «Comando no disponible.», «Dispositivo especificado no válido.» (MS-SG-ES PDF p.48–49).
  - "Cannot …" / "Unable to …" / "Could not …" → «No se puede …» + infinitive; «No se pudo …» when it happened in the past (PDF p.50).
  - "Failed to …" / "… failed" → «Error al …», «Error durante …», «Error en …» (Error + preposition), never «fallo»/«falló» (PDF p.50): «Error al importar: …», «Error al exportar: …»; with a subject in mid-sentence, «… no se pudo …»: «La importación no se pudo completar» (PDF p.51).
  - "… occurred" → left out, never «ha ocurrido» (PDF p.51).
  - "Not enough …" → «… insuficiente» (PDF p.51).
  - "… is not available" → «… no disponible» (PDF p.51).
  - "… not found" → «No se encuentra …» (PDF p.51–52): «No se encuentra ningún cable USB», «No se encuentra el adaptador».
  - Inserted values: find out what the placeable holds and build around it (PDF p.52).
- **Reasons the app builds into a sentence** (`input-*`, `read-error-*`, `lib-errors` details) stay lowercase with no full stop, as the en-US comments ask. Those comments override the guides' period-separator rule.

### One Spanish for every region

Where the Spain and Mexico guides, or the manuals, pick different words, use a word both regions read, or rephrase:

| English | Use | Not | Why |
|---|---|---|---|
| computer | equipo («en este equipo») | ordenador (Spain), computadora (Latin America), el/la PC | MS-T gives «equipo»; MS-SG-ES PDF p.45 and MS-SG-MX PDF p.43 ban each other's word; «PC» is masculine in Spain, feminine in Latin America (MS-SG-ES PDF p.21, MS-SG-MX PDF p.19) |
| mouse | mouse (los mouse) | ratón | MS-SG-MX PDF p.44; the larger share of readers; SUMMARY question 3 |
| file | archivo | fichero | both guides; MS-T |
| Settings | Configuración | Ajustes | both guides (MS-SG-ES PDF p.13, MS-SG-MX PDF p.10); Apple Spain's «Ajustes» is macOS only |
| press | presionar | pulsar, oprimir | both guides write «Presiona Aceptar» (MS-SG-ES PDF p.49, MS-SG-MX PDF p.47); SUMMARY question 4 |
| icon | icono | ícono | both guides write «icono» (MS-SG-ES PDF p.40); Fluke 3000 FC PDF p.9 |
| clamp meter | pinza | gancho | «gancho» is Mexican (Truper MUT-202 PDF p.1) |
| graph | gráfico | gráfica | MS-T «chart» → «gráfico» |
| video, cost, take | (not used) | vídeo/video, coste/costo, coger | MS-SG-MX PDF p.43: «coger» is offensive in Latin America; use «tomar» |
| subjunctive | -ra | -se | MS-SG-MX PDF p.37 |
| recent past | se detectó | se ha detectado | §2 Tenses |

## 3. Capitalization

- **Sentence case everywhere** (MS-SG-ES PDF p.25, MS-SG-MX PDF p.23): only the first word and proper nouns take a capital, in buttons, menus, dialog titles and headings alike. English Title Case loses its capitals: "Reset Zoom" → «Restablecer zoom», "Keyboard & Mouse" → «Teclado y mouse», "High Contrast" → «Contraste alto», "What's New" → «Novedades».
- **OS names keep the OS's own capitals:** «Administrador de dispositivos», «Ajustes del Sistema», «Configuración del Sistema», «Privacidad y seguridad», «Monitorización de entrada».
- **Capitals keep their accents** (MS-SG-ES PDF p.26, MS-SG-MX PDF p.24, "mandatory"): «LÍM. SUP.», «ÁREA», «Área visible». Every Spanish letter is in Latin-1, which the GUI's bundled faces cover.
- **Key names:** follow the source's casing for abbreviated keys («Ctrl+Mayús+A», MS-SG-ES PDF p.26); a key spelled out takes an initial capital only («tecla Control»).
- **The meter's annunciators and key names stay in capitals as printed** (§7). The app's own badges, which imitate them, are translated and kept in capitals (§0 item 10).
- **Text after a colon** starts lowercase unless it is a full quotation or a proper name.
- **Acronyms** take no periods and no plural «s» (MS-SG-ES PDF p.21, MS-SG-MX PDF p.19: «los PC»): «los LED», «varios CSV».
- **Language names** in running text are lowercase («el español», MS-SG-ES PDF p.26). The Language list shows each language's name in itself as a fixed entry («Español»), so leave it.

## 4. Punctuation and spaces

- **¿ and ¡ open every question and exclamation** (MS-SG-ES PDF p.35, MS-SG-MX PDF p.32): «¿Descartar las muestras sin exportar?», «¿Ya transmite y sigue sin reconocerse?». In a sentence that only ends in a question, the ¿ goes where the question starts: «Si ya transmite, ¿sigue sin reconocerse?».
- **No space before : ; ? !**, and none inside quotation marks or parentheses (MS-SG-ES PDF p.38–39). The French sheet's no-break-space rules do not apply.
- **No-break space (U+00A0)** between a number you type and its unit (MS-SG-ES PDF p.31, MS-SG-MX PDF p.37) and before % (§5) in prose. Nowhere else. egui keeps U+00A0 together and may break at U+202F, so never use U+202F.
- **Quotation marks:** “ ” with ‘ ’ inside (§0 item 6; MS-SG-ES PDF p.38: "curly or smart quotes will be used in normal text"). Translate the English's "…" and “…” to “…”. A comma or period after a quotation goes outside the closing mark: «“Exportar…” lo guarda.».
- **Names of on-screen controls:** follow the English. Where it quotes a name ("Tick "Look for Bluetooth devices""), quote it with “ ” («Activa “Buscar dispositivos Bluetooth”»); where it doesn't (Settings, Export…, Record), add nothing. MS-SG-ES PDF p.38 follows the source's quotes («pulsa "Mostrar id. disponibles"»). A button named as a sentence's subject is quoted *(decided)*.
- **Ellipsis:** the single character … (U+2026), straight after the word with no space (MS-SG-ES PDF p.37), including where the English has "..." («Reconectando…»). MS-SG-ES PDF p.38 drops the ellipsis when a sentence refers to a command (“Exportar”, not “Exportar…”): do so in sentences, keep it on the button itself.
- **Dashes:** the en dash – for ranges, with no spaces («204–206», MS-SG-ES PDF p.37), and as the minus sign. The em dash — (raya) only around an aside, opened and closed (MS-SG-ES PDF p.37: "only … to emphasize an isolated element"). The English uses " — " to join two parts of a hover or toast ("Halt acquisition — stops recording and graph updates"): use a colon where it introduces an explanation («Detener la adquisición: se detienen la grabación y la actualización del gráfico»), a period between two sentences, as the guides do in error messages (MS-SG-ES PDF p.48).
- **&** becomes «y», screen-reader text included: "Clear graph & statistics" → «Borrar el gráfico y las estadísticas». The "~" in spec ranges (`10V~36V`) is notation: keep it.
- **Comma:** none before «y», «o», «ni» closing a series: «tensión, corriente y resistencia».
- **Semicolon:** rare in UI text; spec notes that list conditions with ";" keep it, unspaced. The screen-reader separator `{"; "}` stays `{"; "}`.
- **Bulleted lists** (MS-SG-ES PDF p.35–36): items that are full sentences start with a capital and end with a period; fragments take no period.
- **No double space after a period** (MS-SG-ES PDF p.38).

## 5. Numbers

**The app keeps the English decimal point in every number it formats itself** *(decided)*: readings, statistics, durations (`1.5 h`), sample counts (`1.2K`, `500K`), memory (`≈194 MB`), percentages in the histogram and zoom chips (`12.5%`, `150%`). `docs/localization-design.md` ("Numbers keep their point") gives the reasons. Spanish readers meet both conventions at home: Mexico, Central America and the Caribbean write a decimal point, Spain and most of South America a comma (Fluke's Spanish manuals print «0,001 V», Fluke 117 PDF p.6; UNI-T's print «1.5V», UT197 PDF p.40).

1. **Write the numbers you type in prose with a point too**: «en menos de 0.6 s», «Ib0 ≈1.8µA». **Typed examples must keep the point**: the hover `(e.g. 3.3, 5, 12)` (graph-ref-values) shows what to type, and «3,3» would be read as two values.
2. **Spec text:** the spec check accepts a decimal comma («0,6» matches 0.6), but point 1 holds there too: the same panel shows the library's figures with points.
3. **No thousands separator, ever.** `1000 V`, `40000`, as the English does. The spec check rejects «1 000» and «1.000» reads as one in this app (`docs/localization-design.md`, "Spec numbers survive translation").
4. **Integers in prose:** MS-SG-ES PDF p.31 spells out short numbers in non-technical text and keeps figures in technical text; PDF p.32: "In software localization, usually the source usage is followed". Keep digits wherever the English has them, and always before a unit ("5 minutes" → «5 minutos»).
5. **Percent:** U+00A0 before % in sentences and hovers you write («1 %», «{ $percent } %»), as most maker manuals space it (Fluke 3000 FC PDF p.42 «2 % de lectura»; Amprobe AM-500 PDF p.61 «≤ 75 %»; UNI-T UT197 PDF p.8 «0 % a 80 %»; Truper writes «80% H.R.», MUT-202 PDF p.3). In notation, follow the English: spec bands and notes («10%–100%»), the zoom and histogram figures that arrive formatted, and `Duty %`. SUMMARY question 13.
6. **Version numbers** keep their periods: «Versión 0.10.0» (MS-SG-ES PDF p.42).
7. **Gaps the app leaves (for the developers):** `MB`/`KiB` and the `K` suffix arrive preformatted in English inside Spanish sentences. Spanish writes MB and KB too, so these read correctly. Leave the placeables as they are.

## 6. Units and AC/DC

- **Unit symbols never change** *(decided)*, except in the spoken reading's unit names (§0 ruling 20): V, mV, A, mA, µA, Ω, kΩ, MΩ, Hz, F, °C, °F, W, VA, Wh, Ah, dBm, dBV, s, ms, h, V·s, mAh. They are symbols, not abbreviations: no period, no plural «s» (MS-SG-ES PDF p.20–21).
- **Space between a number and its unit:** follow the English as written in notation and spec text (`10MΩ`, `≈3V`); in prose you write, put U+00A0 («2 s», «10 MΩ»).
- **The meter's AC/DC labels stay as the meter prints them** *(decided)*: `DC V`, `V AC`, `AC mA`, `AC+DC`, `ACV`, `DCV`, `LoZ V~`.
- **In prose and table titles, spell the words out** *(decided)*: «tensión continua» (Fluke 117 PDF p.6), «tensión alterna» (Fluke 3000 FC PDF p.39), «corriente continua / alterna» (Amprobe AM-500 PDF p.51), «tensión alterna + continua» for AC+DC.
- **No «CA» / «CC»** («c.a.» / «c.c.»). Fluke and Amprobe write «tensión de CC», «V CA» (Fluke 80 Series V PDF p.7, Amprobe AM-500 PDF p.62), Truper «c.a.»/«c.c.» (MUT-39 PDF p.2), while UNI-T keeps AC/DC (UT197 PDF p.7 «AC (Corriente alterna) DC (Corriente continua)»). Keep AC/DC in labels and notes, «AC+DC» included: a note "AC+DC: add 1%+35 digits" would otherwise put «CA+CC» beside a mode line that says `AC+DC`.
- **«tensión», not «voltaje»** in prose and titles (Fluke 80 Series V, Fluke 117, Fluke 3000 FC, Amprobe AM-500); Fluke 287/289 and UNI-T write «voltaje». SUMMARY question 5.
- **True RMS:** «verdadero valor eficaz» (Fluke 80 Series V PDF p.13, Fluke 3000 FC PDF p.5), or keep «True RMS» in notes as Truper does (MUT-202 PDF p.7). Never «RMS real» (UT197 PDF p.4, MUT-39 PDF p.7: a calque). "rms" after a number (`250V rms`) is notation: keep it.

## 7. What the meter prints stays as it prints it *(decided)*

The Spanish manuals print every annunciator and key name exactly as it appears on the meter and explain it in Spanish beside it: «Retención de valores en pantalla (HOLD)» (Fluke 117 PDF p.3), «MAX, MIN, AVG (promedio)» (Fluke 80 Series V PDF p.8), «presione brevemente el botón RANGE» (UNI-T UT197 PDF p.13), «Mantenga presionado SELECT» (UT197 PDF p.12), «AUTO Rango automático» (Truper MUT-39 PDF p.4), «APO Apagado automático» (Truper MUT-202 PDF p.4), «La pantalla muestra LEAD durante un segundo» (Fluke 3000 FC PDF p.9).

- **Status flags** (`lib-names.ftl`) stay: HOLD, REL, AUTO, MIN, MAX, AVG, LOW BAT, HV!, P-MAX, P-MIN, LEAD ERR, COMP, REC, LoZ, VOID. The spoken forms (`reading-spoken-*`) are Spanish: «retención», «relativo», «rango automático», «mínimo», «máximo», «promedio», «pico mínimo», «pico máximo», «batería baja», «advertencia de tensión peligrosa», «error de conexión de los cables de prueba», «comparación», «grabación», «baja impedancia», «lectura no válida».
- **Key names, display text and menu paths in activation steps and hovers**, as the `lib.ftl` comments require: SELECT, SEL, ZERO/ᛒ, Hz/Duty△/ᛒ, Tab⇌, △/ᛒ, 1ms PEAK, MAX MIN, SEND, RS232, PC, BLE, SETUP -> Communication -> ON, “Org”, dSAPO, APO.oF, BT, ((D)), "the S icon". Translate the words around them: «Mantén presionado el botón Hz/Duty△/ᛒ hasta que aparezca el icono ᛒ».
- **Mode qualifiers:** keep LPF and REL (`LPF { $mode }`, `{ $mode } REL`). Translate "Peak": «Pico { $mode }» / «{ $mode } pico».
- **Brands, model names and trademarks** stay: UNI-T, Voltcraft, ZOTEK, ZOYI, BSIDE, ANENG, EEVblog, Brymen, OWON, the part numbers, AutoCheck™. "Mock UT61E+" → «UT61E+ simulado».

## 8. Command lines, options, files and keys

- **Verbatim** *(decided)*: every `--flag` and its values, `dmm-cli`, `dmm-gui`, subcommands (`list`, `read`, `capture`, `get`, `set`), the quoted command lines (`'dmm-cli list'`, `--device 'ut61e+'`), device and step ids, setting ids as the CLI takes them (`hold`, `rel`, `minmax`, `peak`, `auto`, `system`), settings-file keys (`device_family`, `mock_mode`, `language`, `bluetooth`), environment variables (`RUST_LOG`, `NO_COLOR`, `LANGUAGE`, `LC_ALL`, `LANG`), file names and patterns (`capture-<device>.yaml`), CSV column names and JSON keys, the backquoted text in `read-error-*`, RFC 3339, the password `0000`. The translation check fails a message that loses a `code` span or an `--option`.
- **clap's words** are translated too *(decided)* (`cli/help.ftl`, `cli/args.ftl`): «Uso:», «Comandos:», «Argumentos:», «Opciones:», «[predeterminado: …]», «[valores posibles: …]», «Valores posibles:», «Mostrar la ayuda», «Mostrar la ayuda (más detalles con '--help')», «Mostrar la versión», «error:», «sugerencia:». Sources: MS-T «argument» → «argumento», «command» → «comando», «version» → «versión», «default» → «predeterminado» (MS-SG-ES PDF p.45: never «por defecto»), «tip» → «sugerencia»; «Uso» and «subcomando» have no MS-T entry (MS-T «usage» → «utilización», too heavy for a heading). Value names in capitals (`<FORMAT>`) and the usage line's syntax stay as clap writes them; in errors, names stay in their straight quotes, as typed.
- **Windows and macOS names** must match what a Spanish system shows: §0 item 19. `'CP2110 USB to UART Bridge'` is the device's own name and stays English.
- **Computer key names** are messages of their own *(decided)* (`shortcuts-key-*`): §0 item 14. The guides name Shift «Mayúsculas» in their key table and «Mayús» in every shortcut (MS-SG-ES PDF p.26 «Ctrl+Mayús+A», p.56 «Mayús+F1»), and keep «Ctrl» in shortcuts though the table says «Control» (PDF p.55, p.58 «Ctrl+Mayús+Esc»). Home and End are «Inicio» and «Fin», Page Up/Down «Re Pág»/«Av Pág», the arrows «Flecha izquierda/derecha/arriba/abajo», Spacebar «Barra espaciadora» (PDF p.55–56). The key column needs short names, hence «Espacio», «Izquierda», «Derecha»; prose uses the full names where room allows. Letters, digits, F1–F12 and macOS's ⌘ ⇧ ⌥ symbols stay as they are.
- **Gestures** (the `.gesture` column): «Arrastrar», «Mayús + arrastrar», «Ctrl + rueda del mouse», «Rueda del mouse», «Doble clic», «Clic (cursores activados)», «Clic derecho», «Arrastrar en el minimapa» (MS-T «drag» → «arrastrar», «double-click» → «hacer doble clic», «right-click» → «hacer clic con el botón derecho», «scroll wheel» → «rueda del mouse»).

## 9. Length budget

Spanish runs about 20–30 % longer than English. `cargo test -p dmm-shared long_labels -- --ignored --nocapture` lists short labels that grow past 1.5×. Watch most closely, in order:

1. **The top bar** (one row on a wide window): «Conectar», «Desconectar», «▶ Reanudar», «⏸ Pausar», «Borrar», «Reconectando…», the status, the badges, «Ayuda y comentarios» (19 for 15), and the update link. «PARCIALMENTE VERIFICADO» is 23 characters for 15 (1.53×): the longest; fallback «VERIF. PARCIAL».
2. **Graph chips** (tight rows): «Media», «Mín./Máx.», «Histograma», «Ref.», «Disparos», «Cursores», «EN VIVO», «Y:fijo», «Y:auto», «Restablecer zoom» (16 for 10: 1.6×; fallback «Restablecer»), «Trazar:», «Mostrar:».
3. **The mode line under the reading** wraps on a narrow window: «Ciclo de trabajo %» is the risky one; keep `Duty %` if it wraps.
4. **The Statistics column** right-aligns to the widest of «Mín.:», «Máx.:», «Media:», «σ:».
5. **Settings chips** wrap; their captions end in «:».

When room is short, abbreviate as MS-SG-ES PDF p.19–20 describes: keep the letters in order, cut at least two letters, end on a consonant (never a vowel) and with a period, keep accents, and never add an «s» to a unit or symbol. Use the guides' list where it has the word: «máx.», «mín.», «ref.», «aprox.», «izqda.», «dcha.» (MS-SG-ES PDF p.20); others: «Res.», «Exact.», «Lím.», «Sup.», «Inf.», «Verif.». Never drop a word the user needs to act. In a hover, a sentence may grow.

## 10. Agreement around placeables

Fluent can't tell you the gender or number of what a placeable holds, and Spanish contracts prepositions with the article («de» + «el» = «del», «a» + «el» = «al»), so a bare placeable after «de» or «a» forces a guess. Build sentences in which nothing has to agree with it (MS-SG-ES PDF p.52: find out what replaces the placeholder and build the sentence around it):

- **Put a noun in front:** «el multímetro { $model }», «del multímetro { $model }»; «en modo { $mode }»; «en el rango { $range }»; «la curva { $series }»; «el valor secundario { $series } ({ $unit })»; «el ajuste { $setting }»; «el botón { $key }».
- **Agree with that noun, never with the placeable:** «El multímetro está ahora en modo { $choice }»; «La compatibilidad con el multímetro { $model } está verificada».
- **Use a label-colon form for status lines:** «Rango: { $range } (manual)», «Alarma activada: { $limits }».
- **`$link`, `$side`, `$state`, `$saved`, `$case` and the other selectors** have one variant per value; make each agree with its own noun: [usb] «El cable USB está conectado», [adapter] «El adaptador Bluetooth está conectado», [radio] «El enlace Bluetooth está conectado».
- **`$setting` in `connection-switch-failed`** is a lowercase CLI id (hold, rel, minmax, peak): «Error al cambiar el ajuste { $setting }: { $error }».
- **Inserted reasons** (`$reason`, `$error`, `$detail`) may be Spanish or English (from the OS or a parser). Put them after a colon, never inside a sentence that expects a gender.
- **Nothing agrees with the user** (§0 item 16): «Se detectó el multímetro», not «Estás conectado».
- **Gender of the words this app uses most:** el multímetro, el dispositivo, el adaptador, el cable, el enlace, el modo, el rango, el ajuste, el botón, el marcador, el cursor, el búfer, el gráfico, el eje; la lectura, la muestra, la medición, la conexión, la grabación, la curva, la serie, la alarma, la escala, la tensión, la corriente. «terminal» is masculine in Spain and often feminine in Mexico (glossary): avoid an article on it.

**Plurals.** CLDR 37's rule for Spanish, which fluent-bundle applies (`intl_pluralrules` 7.0.2, `rules.rs`), puts exactly 1 in `one` and everything else, 0 included, in `other`. That matches Spanish: «0 muestras», «1 muestra», «2 muestras». Write `[one]` and `*[other]` only, as the translation check requires. Newer CLDR adds `many` for Spanish (exact millions in compact notation); if the rules ever gain it, give it the `*[other]` text. Where the English has a numeric `[0]` variant (`args-too-few-values`, `args-wrong-number-of-values`), give it the plural text: «solo se indicaron 0». Unit symbols never take an «s»; words do («3 segundos», «2 pitidos»).

## 11. Fluent specifics

- **Never change** message IDs, attribute names (`.hover`, `.a11y`, `.hover-disabled`, `.wayland`, `.gesture`, `.about`, `.help`, `.value-replay`), variable names (`$keys`, `$model`…) or selector keys (`[one]`, `[usb]`, `[yes]`, `[big-meter]`…). Keep the `*` default on the same variant. Translate every variant's text.
- **You may** reorder placeables, drop a placeable the Spanish doesn't need, add a selector on a variable that exists, or turn a plain message into one with variants. Nothing in the data gives gender, so don't invent gender selectors.
- **Message references stay references:** `{ project-link }`, `{ device-auto }`, `{ bluetooth-setting }`, `{ error-bluetooth-reset-gesture }`. Translate the referenced message.
- **String literals** (`{" "}`, `{", "}`, `{"  "}`) are layout: keep their meaning.
- **Multi-line messages:** keep the indentation and the line breaks. The notices are hard-wrapped for a narrow column (connection.ftl), and the activation steps are one step per line. A wrapped notice line may move a word or two if the Spanish runs longer.
- **`.a11y` text** is read aloud, never drawn: write it as speech, units in words where the comments ask («grados C»), «y» for «&», «más» for «+», «aproximadamente» for «~» and «≈» (MS-SG-ES PDF p.19, MS-SG-MX PDF p.16), no abbreviations.
- **Trailing and leading spaces are dropped** by Fluent; write a needed one as `{" "}`. U+00A0 is ordinary text to Fluent and survives next to a placeable: type it literally.
- **Text never translated:** the `###`/`#` comments, and `settings-language-no-chinese-font`, which its comment says is always shown in English.

## 12. Spec text (lib.ftl: bands, inputs, notes): optional polish *(decided)*

- Keep every number, sign, ±, ≤, ≥, <, >, ≈ and ~, and each unit symbol right after its number. Numbers as in the English (§5). The spec check compares them and falls back to English on any mismatch.
- Keep the exact sense: comparisons, ranges, «típico» / «típ.», «nominal», «aproximadamente» / «aprox.», «al menos», «máx.». The check can't catch a flipped comparison, and the panel shows the English on hover wherever the text differs.
- Use the makers' wording: «No especificado»; «de la lectura» (accuracy); «dígitos» (accuracy and residual digits), «cuentas» (display counts); «protección contra sobrecargas»; «impedancia de entrada»; «factor de cresta»; «coeficiente de temperatura»; «cables de prueba en cortocircuito»; «circuito abierto»; «sensibilidad»; «tensión de carga» (burden voltage); «umbral»; «respuesta de frecuencia»; «solo como referencia». See the glossary for each source.
- Table titles keep the manual's numbering and letters: «(1) Tensión alterna», «A. Tensión continua», «K. Temperatura 1-1. Grados Celsius».

## 13. Words to avoid

| Avoid | Use | Why |
|---|---|---|
| usted, vos, vosotros, el usuario | tú | MS-SG-ES PDF p.33–34, MS-SG-MX PDF p.31–32 |
| ordenador, computadora, el PC / la PC | equipo | regional words; PC changes gender (§2) |
| ratón | mouse | MS-SG-MX PDF p.44; SUMMARY question 3 |
| fichero | archivo | both guides; MS-T |
| voltaje | tensión | SUMMARY question 5 |
| capacidad (for capacitance) | capacitancia | every maker manual; «capacidad» is a fuse's breaking capacity (UT197 PDF p.8) |
| RMS real, RMS verdadero | verdadero valor eficaz / True RMS | calque (UT197, MUT-39) |
| CA / CC, c.a. / c.c. | AC / DC as printed | consistency with the mode labels (§6) |
| medidor (the meter) | multímetro | one word throughout; Fluke 117 and UT197 mix both |
| presión eléctrica | tensión | UT195's machine-like Spanish |
| conteos, recuentos | cuentas | one word for display counts (glossary) |
| app, info | aplicación, información | MS-SG-ES PDF p.8, MS-SG-MX PDF p.8 |
| por defecto | predeterminado | MS-SG-ES PDF p.45, MS-SG-MX PDF p.43 |
| atajo | método abreviado | MS-SG-ES PDF p.44, MS-SG-MX PDF p.43 |
| fallo, falló | error, no se pudo | MS-SG-ES PDF p.45, p.50 |
| ignorar (for ignore, dismiss) | omitir, pasar por alto, cerrar | false friend (MS-SG-ES PDF p.46) |
| remover, reporte, checar | quitar / extraer, informe, comprobar | MS-SG-ES PDF p.44–45 |
| coger | tomar | offensive in Latin America (MS-SG-MX PDF p.43) |
| ocurrencia (occurrence) | repetición, caso | MS-SG-ES PDF p.47 |
| ha ocurrido, ocurrió (in errors) | (leave out) | MS-SG-ES PDF p.51 |
| Por favor | (leave out) | MS-SG-ES PDF p.12 |
| digitalizar, escanear (Bluetooth scan) | buscar | MS-T «scan» → «digitalizar» is for paper |
| movimiento panorámico (pan) | desplazar | camera sense |
| desencadenador (graph trigger) | disparo | MS-T's workflow sense |
| sobre (envelope) | envolvente | MS-T's is postal |
| le (direct object, leísmo) | lo | MS-SG-MX PDF p.32 |
| cantase (-se subjunctive) | cantara | MS-SG-MX PDF p.37 |
