# Brazilian Portuguese glossary and style sheet: what a reviewer should rule on first

No native reviewer has seen this draft. Each row gives the draft's choice (applied in `glossary.tsv` and `STYLE.md`) and the alternative. Most important first.

| # | Question | Draft choice | Alternative | Source / reason |
|---|---|---|---|---|
| 1 | Register and verb form | «você»; buttons, chips, menu items, action hovers in the infinitive («Exportar…», «Congelar a leitura atual»); notices, errors' advice and activation steps in the você imperative («Verifique…», «Gire a chave seletora…») | Imperative on buttons too («Exporte…») | MS-SG PDF p.28, p.49–51 (infinitive UI: «Cancelar», «Salvar como»); p.15, p.17 (imperative: «Tente novamente», «clique em Cancelar»); Minipa ET-1649BT PDF p.14, p.22 |
| 2 | Capitalization of labels | Sentence case everywhere («Redefinir zoom», «Alto contraste», «Teclado e mouse») | Copy the English capitals in UI labels («Redefinir Zoom»), as Microsoft does | MS-SG PDF p.28 follows the source in UI strings "for localization simplicity", but sentence case for errors; the English mixes Title Case and sentence case, so copying it keeps the mix |
| 3 | A reading | «leitura» (f.); «valor medido» for the number; «medição» for the act | «medida» (MS-T measurement → medida) | Minipa ET-1649BT PDF p.10 «Congelamento de Leitura», p.24 «% da leitura»; ET-2587 PDF p.8 |
| 4 | range | «faixa» | «escala» (ET-2517A/2615A PDF p.6 once), «intervalo» (MS-T, a data range) | Minipa ET-1649BT PDF p.12, p.24 table heading «Faixa» |
| 5 | The meter's display | «display» (m.) | «visor» (MS-T display → visor) | Every Minipa manual: ET-1649BT PDF p.10, p.14 |
| 6 | duty cycle / `Duty %` | Keep «Duty Cycle» and `Duty %` as Minipa does | «ciclo de trabalho» (textbook word, no approved source) | Minipa ET-1649BT PDF p.19, ET-2587 PDF p.2, ET-2517A/2615A PDF p.22 |
| 7 | Zero in plurals | Add a `[0]` variant with the plural text where a count can be 0 («0 amostras») | Accept the `[one]` text for 0 («0 amostra»), or rephrase («nenhuma amostra») | fluent-bundle's pt rule (CLDR 37) puts 0 in `one`; the translation check ignores number keys |
| 8 | Bluetooth link (a meter with Bluetooth built in) | «conexão Bluetooth» (f.) | «Bluetooth integrado» (m.), which avoids reusing the word for "connection" | Minipa ET-1649BT PDF p.22 «Conexão Bluetooth», PDF p.4 «Módulo Bluetooth integrado» |
| 9 | Key names in the shortcut column | Ctrl, Shift, Alt, Espaço, Esc, Esquerda, Direita, Home, End, Mais, Menos | MS's full names: «Barra de espaços», «Seta para a esquerda/direita» (long in the key column) | MS-SG PDF p.47–48 (keeps Alt, Shift, Esc, Home, End in English) |
| 10 | Quotation marks | “ ” with ‘ ’ inside | none (« » is European Portuguese and French) | MS-SG PDF p.34; Minipa ET-1649BT PDF p.4 |
| 11 | % spacing | No space: «50%», «{ $percent }%» | Space before %, as ET-2517A/2615A PDF p.20 writes «10 % a 100 %» | Most Minipa figures unspaced (ET-1649BT PDF p.24–25) |
| 12 | The app's badges | ESCALA, LIM. SUP., LIM. INF., AO VIVO, VERIFICADO, PARCIALMENTE VERIFICADO (23 for 15 characters), EXPERIMENTAL | «VERIF. PARCIAL» if the top bar overflows | App text, not LCD text |
| 13 | accuracy | «precisão» | «exatidão» (MS-T) | Every Minipa manual: ET-1649BT PDF p.24 |
| 14 | digits / counts | «dígitos» in accuracy and residual-reading notes; «contagens» for display counts («6000 contagens») | «D» / «d» as Minipa's tables abbreviate | Minipa ET-1649BT PDF p.23–24; ET-2517A/2615A PDF p.19–20 |
| 15 | Live (live-wire detection mode) | «Fase» | «Fio vivo» (Minipa's words) | Minipa ET-2517A/2615A PDF p.12 «conexão viva», «tensões fase a neutro» |
| 16 | offset (--offset, scaling) | «deslocamento» | «offset», as Minipa keeps it | MS-T offset → Deslocamento; Minipa ET-2517A/2615A PDF p.5 |
| 17 | Gender toward the user | Rephrase so nothing agrees with the user («Conexão estabelecida») | Generic masculine («Você está conectado»), as the French sheet does | MS-SG PDF p.19–20 |
| 18 | Colorblind preset | «Daltonismo» | «Para daltônicos» | No approved source; MS-SG PDF p.21 "focus on people, not disabilities" |
| 19 | inrush current | «corrente de partida» | keep «Inrush» as clamp meters print it | No approved source |
| 20 | Pause / Resume, Dismiss, Record | «Pausar» / «Continuar»; «Ignorar»; «Gravar» | «Retomar»; «Fechar»; «Registrar» (Minipa's REC is «registro», kept for the meter's memory) | MS-T; Minipa ET-2517A/2615A PDF p.16 «Pausar/Continuar/Parar», PDF p.13 «modo de registro» |
| 21 | Short press | «pressionar brevemente» | «pressionar momentaneamente» (ET-2517A/2615A PDF p.7) | Minipa ET-1649BT PDF p.20; its «pressione levemente» (PDF p.12) means lightly |
| 22 | Terms with no approved source (24 rows) | temperatura interna; ganho do transistor (hFE); VFD; VFC; indutância; potência aparente (VA); fator de potência / PF; corrente de partida; período; absoluto; função secundária; VOID / inválido; apenas para referência; reconectar / Reconectando…; identificar-se; verificado / parcialmente verificado / experimental; curva (trace); retícula (crosshair); lacuna (data gap); área visível (viewport); Visível; histerese; Daltonismo; moldura da janela (window decorations) | | Partly sourced, worth a look too: the Motor/Solar/Tacômetro/Bússola/Ângulo row, «texto secundário» (weak text), «deslocar» (pan), «alicate» (Minipa web category only) |

Also open:

- **OS names.** Windows «Gerenciador de Dispositivos» is from MS-T, «Dispositivos de Interface Humana» and «Outros dispositivos» from Microsoft's pt-BR support pages, and the macOS path («Ajustes do Sistema > Privacidade e Segurança > Monitoração de Entrada») from Apple's. 'USB Input Device' («Dispositivo de Entrada USB») is unverified: it needs a Brazilian Windows.
- **Electropedia** refused every fetch (403), so no IEV term backs any row.
- **Code gaps (not translation):** `MB`/`KiB`/`K` arrive preformatted (fine in pt-BR, which writes MB too); `$keys` shortcuts come from egui in English.
