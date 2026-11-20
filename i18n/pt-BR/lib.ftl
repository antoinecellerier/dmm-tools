### Text the library shows: the labels of readings and of the choices a meter
### offers, meter keys and the steps that switch a meter's data transmission on.
### The specification tables' text is in lib-specs.ftl.
###
### Generated from dmm-lib by `UPDATE_L10N=1 cargo test -p dmm-shared lib_ftl`.
### Entries are only ever added. The English of each one is what the library
### shows and is how the app finds it: edit it only to follow a change in the
### library, keeping the ID to keep its translations.
###
### Numbers and unit symbols stay as they are in every language. Labels made of
### nothing else (200mV, kΩ, °C) are not listed: they always show as they are.
### Nor are modes made of a qualifier around another mode (LPF AC V): the
### qualifier is listed once.


## Measurement modes: beside the reading, in the mode menu, the graph
## and the statistics. Keep each as short as the meter prints it; you may keep
## abbreviations such as DC V, LPF or REL as they are.

# Used by: UT171A/B/C
mode-600a-ac = 600A AC
# Used by: UT171A/B/C
mode-600a-dc = 600A DC
# Used by: UT171A/B/C, UT181A
mode-a-ac = A AC
# Used by: UT181A
mode-a-ac-hz = A AC Hz
# Used by: UT181A
mode-a-ac-peak = A AC pico
# Used by: UT171A/B/C
mode-a-ac-plus-dc = A AC+DC
# Used by: UT171A/B/C, UT181A
mode-a-dc = A DC
# Used by: UT181A
mode-a-dc-ac-plus-dc = A DC AC+DC
# Used by: UT181A
mode-a-dc-peak = A DC pico
# Used by: 43 meters
mode-ac-a = AC A
# Used by: 45 meters
mode-ac-v = AC V
# Used by: Mock UT61E+
mode-ac-v-hz = AC V Hz
# Used by: 121GW
mode-ac-va = AC VA
# Used by: 28 meters
mode-ac-ma = AC mA
# Used by: 24 meters
mode-ac-mv = AC mV
# Used by: 121GW
mode-ac-mva = AC mVA
# Used by: 9 meters
mode-ac-power-a = Potência AC A
# Used by: 9 meters
mode-ac-power-ah = Potência AC Ah
# Used by: 9 meters
mode-ac-power-v = Potência AC V
# Used by: 9 meters
mode-ac-power-va = Potência AC VA
# Used by: 9 meters
mode-ac-power-w = Potência AC W
# Used by: 9 meters
mode-ac-power-wh = Potência AC Wh
# Used by: 27 meters
mode-ac-ua = AC µA
# Used by: 121GW
mode-ac-uva = AC µVA
# Used by: 17 meters
mode-ac-plus-dc-a = AC+DC A
# Used by: 30 meters
mode-ac-plus-dc-v = AC+DC V
# Used by: 9 meters
mode-ac-plus-dc-ma = AC+DC mA
# Used by: 9 meters
mode-ac-plus-dc-mv = AC+DC mV
# Used by: 9 meters
mode-ac-plus-dc-ua = AC+DC µA
# Used by: VC-880, VC-890, VC650BT
mode-acv-lpf = ACV LPF
# Used by: 9 meters
mode-angle = Ângulo
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
mode-auto = Auto
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-auto-v = Auto V
# Used by: 47 meters
mode-capacitance = Capacitância
# Used by: UT8803
mode-capacitance-d = Capacitância D
# Used by: UT8803
mode-capacitance-r = Capacitância R
# Used by: 8 meters
mode-clamp-ac-a = Alicate AC A
# Used by: 8 meters
mode-clamp-ac-plus-dc-a = Alicate AC+DC A
# Used by: 8 meters
mode-clamp-dc-a = Alicate DC A
# Used by: 8 meters
mode-clamp-lpf-a = Alicate LPF A
# Used by: 9 meters
mode-compass = Bússola
# Used by: 46 meters
mode-continuity = Continuidade
# Used by: UT181A
mode-continuity-open = Continuidade (circuito aberto)
# Used by: 43 meters
mode-dc-a = DC A
# Used by: 45 meters
mode-dc-v = DC V
# Used by: 121GW
mode-dc-va = DC VA
# Used by: 29 meters
mode-dc-ma = DC mA
# Used by: 28 meters
mode-dc-mv = DC mV
# Used by: 121GW
mode-dc-mva = DC mVA
# Used by: 9 meters
mode-dc-power-a = Potência DC A
# Used by: 9 meters
mode-dc-power-ah = Potência DC Ah
# Used by: 9 meters
mode-dc-power-v = Potência DC V
# Used by: 9 meters
mode-dc-power-va = Potência DC VA
# Used by: 9 meters
mode-dc-power-w = Potência DC W
# Used by: 9 meters
mode-dc-power-wh = Potência DC Wh
# Used by: 28 meters
mode-dc-ua = DC µA
# Used by: 121GW
mode-dc-uva = DC µVA
# Used by: 46 meters
mode-diode = Diodo
# Used by: UT181A
mode-diode-alarm = Alarme de diodo
# Used by: 44 meters
mode-duty-pct = Duty %
# Used by: BM829s/BM827s/BM822s/BM821s
mode-ef = EF
# Used by: BM788BT/BM787BT
mode-ef-h = EF-H
# Used by: BM788BT/BM787BT
mode-ef-l = EF-L
# Used by: 9 meters
mode-frequency = Frequência
# Used by: 9 meters
mode-hv-ac-v = HV AC V
# Used by: 9 meters
mode-hv-dc-v = HV DC V
# Used by: UT8803
mode-inductance = Indutância
# Used by: UT8803
mode-inductance-q = Indutância Q
# Used by: UT8803
mode-inductance-r = Indutância R
# Used by: 22 meters
mode-inrush = Corrente de partida
# Used by: BM788BT/BM787BT
mode-line-hz = Hz da rede
# Used by: 8 meters
mode-live = Fase
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-loz-ac-v = LoZ AC V
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-loz-dc-v = LoZ DC V
# Used by: 9 meters
mode-loz-v = LoZ V
# Used by: UT171A/B/C
mode-loz-v-2 = LoZ V~
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-loz-ohm = LoZ Ω
# Used by: BM788BT/BM787BT
mode-logic-hz = Hz de nível lógico
# Used by: 9 meters
mode-motor = Motor
# Used by: 9 meters
mode-motor-1-2-3 = Motor 1-2-3
# Used by: 9 meters
mode-motor-3-2-1 = Motor 3-2-1
# Used by: 20 meters
mode-ncv = NCV
# Used by: UT71A, UT71B, UT71C/D/E, UT804, VC920/VC940/VC960
mode-power = Potência
# Used by: 9 meters
mode-power-a = Potência A
# Used by: 9 meters
mode-power-ah = Potência Ah
# Used by: 9 meters
mode-power-v = Potência V
# Used by: 9 meters
mode-power-va = Potência VA
# Used by: 9 meters
mode-power-w = Potência W
# Used by: 9 meters
mode-power-wh = Potência Wh
# Used by: 9 meters
mode-power-factor = Fator de potência
# Used by: 121GW, UT181A
mode-pulse-width = Largura de pulso
# Used by: BM525s/BM521s
mode-recall = Recuperação
# Used by: UT8802, UT8803
mode-scr = SCR
# Used by: 9 meters
mode-scope = Osciloscópio
# Used by: 9 meters
mode-solar = Solar
# Used by: UT171A/B/C
mode-square-wave = Onda quadrada
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1 = T1
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1-t2 = T1-T2
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t2 = T2
# Used by: Mock UT61E+
mode-temp-degc = Temp °C
# Used by: Mock UT61E+
mode-temp-degc-t1-t2 = Temp °C T1 (T2)
# Used by: Mock UT61E+
mode-temp-degc-t1-t2-2 = Temp °C T1-T2
# Used by: Mock UT61E+
mode-temp-degc-t2-t1 = Temp °C T2-T1
# Used by: UT71A, UT71B, UT71C/D/E, UT803, UT804, VC920/VC940/VC960
mode-temperature = Temperatura
# Used by: 9 meters
mode-time = Tempo
# Used by: 9 meters
mode-usb-power-a = Potência USB A
# Used by: 9 meters
mode-usb-power-ah = Potência USB Ah
# Used by: 9 meters
mode-usb-power-v = Potência USB V
# Used by: 9 meters
mode-usb-power-va = Potência USB VA
# Used by: 9 meters
mode-usb-power-w = Potência USB W
# Used by: 9 meters
mode-usb-power-wh = Potência USB Wh
# Used by: UT171A/B/C, UT181A
mode-v-ac = V AC
# Used by: UT181A
mode-v-ac-hz = V AC Hz
# Used by: UT181A
mode-v-ac-lpf = V AC LPF
# Used by: UT181A
mode-v-ac-peak = V AC pico
# Used by: UT181A
mode-v-ac-dbv = V AC dBV
# Used by: UT181A
mode-v-ac-dbm = V AC dBm
# Used by: UT171A/B/C
mode-v-ac-plus-dc = V AC+DC
# Used by: UT171A/B/C, UT181A
mode-v-dc = V DC
# Used by: UT181A
mode-v-dc-ac-plus-dc = V DC AC+DC
# Used by: UT181A
mode-v-dc-peak = V DC pico
# Used by: UT171A/B/C
mode-vfc = VFC
# Used by: BM788BT/BM787BT, BM869s/BM867s
mode-vfd-ac-v = VFD AC V
# Used by: BM788BT/BM787BT, BM869s/BM867s
mode-vfd-hz = VFD Hz
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-dbm-reference = Referência dBm
# Used by: 25 meters
mode-hfe = hFE
# Used by: UT171A/B/C, UT181A
mode-ma-ac = mA AC
# Used by: UT181A
mode-ma-ac-hz = mA AC Hz
# Used by: UT181A
mode-ma-ac-peak = mA AC pico
# Used by: UT171A/B/C
mode-ma-ac-plus-dc = mA AC+DC
# Used by: UT171A/B/C, UT181A
mode-ma-dc = mA DC
# Used by: UT181A
mode-ma-dc-ac-plus-dc = mA DC AC+DC
# Used by: UT181A
mode-ma-dc-peak = mA DC pico
# Used by: UT171A/B/C, UT181A
mode-mv-ac = mV AC
# Used by: UT181A
mode-mv-ac-ac-plus-dc = mV AC AC+DC
# Used by: UT181A
mode-mv-ac-hz = mV AC Hz
# Used by: UT181A
mode-mv-ac-peak = mV AC pico
# Used by: UT171A/B/C
mode-mv-ac-plus-dc = mV AC+DC
# Used by: UT171A/B/C, UT181A
mode-mv-dc = mV DC
# Used by: UT181A
mode-mv-dc-peak = mV DC pico
# Used by: UT181A
mode-degc-t1-t2 = °C T1-T2
# Used by: UT181A
mode-degc-t2 = °C T2
# Used by: UT181A
mode-degc-t2-t1 = °C T2-T1
# Used by: UT181A
mode-degf-t1-t2 = °F T1-T2
# Used by: UT181A
mode-degf-t2 = °F T2
# Used by: UT181A
mode-degf-t2-t1 = °F T2-T1
# Used by: UT171A/B/C, UT181A
mode-ua-ac = µA AC
# Used by: UT181A
mode-ua-ac-hz = µA AC Hz
# Used by: UT181A
mode-ua-ac-peak = µA AC pico
# Used by: UT171A/B/C
mode-ua-ac-plus-dc = µA AC+DC
# Used by: UT171A/B/C, UT181A
mode-ua-dc = µA DC
# Used by: UT181A
mode-ua-dc-ac-plus-dc = µA DC AC+DC
# Used by: UT181A
mode-ua-dc-peak = µA DC pico

## Mode qualifiers: a word around a mode's name for a low-pass filter, a
## held peak or a relative reading, around any of the modes above. $mode is the
## mode, already translated: put it where your language puts it.

# Used by: 9 meters
# $mode is the mode, already translated.
qualifier-lpf-mode = LPF { $mode }
# Used by: 9 meters
# $mode is the mode, already translated.
qualifier-peak-mode = Pico { $mode }
# Used by: UT181A
# $mode is the mode, already translated.
qualifier-mode-rel = { $mode } REL
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
# $mode is the mode, already translated.
qualifier-mode-peak = { $mode } pico

## Ranges: beside the reading and in the range menu. Ranges that are a
## number and a unit only are not listed.

# Used by: UT161E, UT61E+
range-1000beta = 1000β
# Used by: 12 meters
range-auto = Auto
# Used by: 7 meters
range-cont = Cont
# Used by: 7 meters
range-diode = Diodo
# Used by: 7 meters
range-duty = Duty
# Used by: Mock UT61E+, UT161E, UT61E+
range-temp = Temp

## Sub-value labels: the meter's secondary readings beside the main one,
## in the graph's legend and in the statistics. Keep them short: each sits next
## to a number.

# Used by: 15 meters
aux-ac = AC
# Used by: 10 meters
aux-ac-a = AC A
# Used by: 10 meters
aux-ac-v = AC V
# Used by: 9 meters
aux-ac-plus-dc-v = AC+DC V
# Used by: UT181A
aux-absolute = Absoluto
# Used by: 9 meters
aux-angle = Ângulo
# Used by: UT171A/B/C
aux-aux = Aux
# Used by: UT181A
aux-average = Média
# Used by: 121GW
aux-battery = Bateria
# Used by: 121GW
aux-burden-voltage = Queda de tensão
# Used by: UT181A
aux-comp-high = COMP alto
# Used by: UT181A
aux-comp-low = COMP baixo
# Used by: 10 meters
aux-capacitance = Capacitância
# Used by: UT202BT
aux-clamp-ac-a = Alicate AC A
# Used by: UT202BT
aux-clamp-lpf-a = Alicate LPF A
# Used by: 9 meters
aux-compass = Bússola
# Used by: 10 meters
aux-continuity = Continuidade
# Used by: 121GW
aux-current = Corrente
# Used by: 12 meters
aux-dc = DC
# Used by: 9 meters
aux-dc-a = DC A
# Used by: 10 meters
aux-dc-v = DC V
# Used by: 9 meters
aux-diode = Diodo
# Used by: 10 meters
aux-duty = Duty
# Used by: 9 meters
aux-frequency = Frequência
# Used by: 9 meters
aux-hv-ac-v = HV AC V
# Used by: 9 meters
aux-hv-dc-v = HV DC V
# Used by: UT181A
aux-impedance = Impedância
# Used by: UT202BT
aux-inrush = Corrente de partida
# Used by: 121GW
aux-internal-temperature = Temperatura interna
# Used by: UT202BT
aux-lpf-v = LPF V
# Used by: VC871
aux-live = Fase
# Used by: UT181A
aux-max = Máx.
# Used by: UT181A
aux-min = Mín.
# Used by: 9 meters
aux-motor = Motor
# Used by: CMS061, CMS101
aux-ncv = NCV
# Used by: 9 meters
aux-pf = PF
# Used by: UT181A
aux-peak-min = Pico mín.
# Used by: Mock UT61E+, UT181A
aux-period = Período
# Used by: UT171A/B/C
aux-pulse-width = Largura de pulso
# Used by: UT181A, VC871
aux-reference = Referência
# Used by: 9 meters
aux-solar = Solar
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, Mock UT61E+, UT181A
aux-t1 = T1
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s, Mock UT61E+, UT181A
aux-t2 = T2
# Used by: UT202BT
aux-temperature = Temperatura
# Used by: 9 meters
aux-time = Tempo
# Used by: 121GW, UT181A
aux-voltage = Tensão
# Used by: 9 meters
aux-hfe = hFE

## On and off, for a meter setting offered as a choice.

# Used by: 13 meters
toggle-off = desativado
# Used by: 13 meters
toggle-on = ativado

## Meter keys named in words for what they do. A name printed on a key is
## not listed here: it stays as printed.

# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-auto-function = Função automática
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-capacitance = Capacitância
# Used by: ZT-300AB, ZT-5566SE, ZT-5BQ
key-current = Corrente
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-diode-continuity = Diodo / continuidade

## Meter key hovers: what a remote button does, or how to do it on the
## meter. Keep the meter's key names as it prints them.

# Used by: VC891, VC915
key-hover-1-khz-low-pass-filter = Filtro passa-baixa de 1 kHz
# Used by: 121GW
key-hover-capture-1-ms-peaks-in-ac = Capturar picos de 1 ms no modo AC V
# Used by: OW67B, VC871
key-hover-change-what-the-displays-show = Alterar o que os displays exibem
# Used by: VC915, VC925 PV
key-hover-compare-the-reading-with-the-set = Comparar a leitura com os limites definidos
# Used by: CMS061, CMS101
key-hover-enter-max-min-click-again-to = Entrar no modo MAX/MIN; clique novamente para sair
# Used by: 121GW
key-hover-hold-the-meter-s-rel-key = Manter a tecla REL do multímetro pressionada (filtro passa-baixa de 1 kHz)
# Used by: CMS061, CMS101
key-hover-measure-inrush-current-click-again-to = Medir a corrente de partida; clique novamente para sair
# Used by: UT171A/B/C
key-hover-peak-hold-in-ac-v-and = Captura de picos em AC V e A: clique para entrar no modo ou sair dele
# Used by: OW18B/OW16B, OW18E
key-hover-press-the-meter-s-hz-duty = Pressionar brevemente a tecla Hz/Duty△/ᛒ do multímetro
# Used by: CM2100B
key-hover-press-the-meter-s-zero-key = Pressionar brevemente a tecla ZERO/ᛒ do multímetro
# Used by: OW18B/OW16B, OW18E
key-hover-rel-in-ac-and-hz-steps = REL; nos modos AC e Hz, alterna entre Hz e Duty
# Used by: OW67B, OW69B, VC871, VC891, VC915, VC925 PV
key-hover-read-a-4-20-ma-loop = Ler uma corrente de loop 4-20 mA em porcentagem
# Used by: ZT-5566SE
key-hover-record-maximum-and-minimum-readings = Registrar as leituras máxima e mínima
# Used by: UT171A/B/C
key-hover-record-maximum-average-and-minimum-readings-2 = Registrar as leituras máxima, média e mínima: clique para alternar entre elas, × para sair
# Used by: UT181A
key-hover-record-maximum-average-and-minimum-readings = Registrar as leituras máxima, média e mínima: × para sair
# Used by: VC-880, VC-890, VC650BT
key-hover-record-maximum-minimum-and-average-readings = Registrar as leituras máxima, mínima e média: clique para alternar entre elas, × para sair
# Used by: 121GW
key-hover-record-minimum-maximum-and-average-readings = Registrar as leituras mínima, máxima e média: clique para alternar entre elas, × para sair
# Used by: UT171A/B/C, UT181A
key-hover-save-the-reading-to-the-meter = Salvar a leitura na memória do multímetro
# Used by: CMS061, CMS101
key-hover-send-select-switches-v-dc-to = Enviar o comando Select: o multímetro muda de V,DC para V,AC
# Used by: VC915
key-hover-show-3-phase-rotation-direction = Mostrar o sentido de rotação das fases em redes trifásicas
# Used by: VC915
key-hover-show-vac-or-vdc-on-the = Mostrar VAC ou VDC no segundo display
# Used by: 121GW
key-hover-step-through-hold-and-autohold = Alternar entre HOLD e AutoHOLD
# Used by: VC915
key-hover-swap-hz-pct-and-voltage = Trocar as posições de Hz/% e da tensão
# Used by: VC925 PV
key-hover-swap-the-main-and-second-displays = Trocar o conteúdo do display principal com o do segundo display
# Used by: OW69B
key-hover-switch-the-low-pass-filter-on = Ativar ou desativar o filtro passa-baixa
# Used by: UT171A/B/C
key-hover-take-the-reading-as-the-reference = Usar a leitura como referência: × para sair
# Used by: OW18B/OW16B, OW18E
key-hover-toggle-the-backlight-and-an-ow18 = Ligar ou desligar a luz de fundo e, no OW18, a lanterna
# Used by: VC871, VC891, VC915, VC925 PV
key-hover-zero-the-reading-to-exit = Zerar a leitura: × para sair

## Model names with words in them: the simulated meters. Keep the part
## number as it is.

# Used by: Mock UT61E+
model-mock-ut61e-plus = UT61E+ simulado
# Used by: Mock ZT-5B
model-mock-zt-5b = ZT-5B simulado

## Activation steps: how to switch a meter's data transmission on, shown
## when a connection gets no data. Keep each step on its own line, and the
## meter's key names, menu items and display text as the meter prints them.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
activation-bm52x =
    1. Encaixe o cabo BU-86X na porta óptica PC-Comm, na parte traseira do multímetro, e conecte-o ao computador
    2. Ligue o multímetro em uma função de tensão
    Observação: o multímetro desliga após cerca de 17 minutos de inatividade no BM869s e no BM867s, e após 30 minutos nos demais; para desativar o desligamento automático, mantenha a tecla SELECT pressionada ao ligar o multímetro.
# Used by: UT8802, UT8803
activation-ut8802 =
    1. Conecte o cabo USB ao multímetro
    2. Ligue o multímetro
# Used by: UT171A/B/C
activation-ut171 =
    1. Conecte o cabo USB ao multímetro
    2. Ligue o multímetro
    3. Acesse SETUP -> Communication -> ON
# Used by: UT181A
activation-ut181a =
    1. Conecte o cabo USB ao multímetro
    2. Ligue o multímetro
    3. Acesse SETUP -> Communication -> ON
    Observação: esta configuração é redefinida sempre que o multímetro é desligado.
# Used by: UT71A, UT71B, UT71C/D/E, VC920/VC940/VC960
activation-ut71ab =
    1. Conecte o cabo USB ao multímetro
    2. Ligue o multímetro
    3. Mantenha a tecla MAX MIN pressionada por 1 s, ou pressione SEND no UT71A; o display exibe SEND
    Observação: a tecla EXIT desativa o SEND.
# Used by: UT803
activation-ut803 =
    1. Conecte o cabo USB ao multímetro
    2. Ligue o multímetro
    3. Pressione RS232; o display exibe RS232
# Used by: UT804
activation-ut804 =
    1. Conecte o cabo USB ao multímetro
    2. Ligue o multímetro
    3. Pressione SEND; o display exibe SEND
    Observação: HOLD pausa o envio dos dados e EXIT desativa o SEND.
# Used by: VC-880, VC-890, VC650BT
activation-vc650bt =
    1. Conecte o cabo USB ao multímetro
    2. Ligue o multímetro
    3. Pressione a tecla PC do multímetro
# Used by: BM788BT/BM787BT
activation-bm78xbt =
    1. Desconecte o multímetro de qualquer outro aplicativo
    2. Gire a chave seletora para qualquer função, exceto Auto V/LoZ
    3. Mantenha a tecla Δ pressionada por um segundo ou mais, até ((D)) aparecer no display
    Observação: o multímetro desliga após 15 a 30 minutos de inatividade; para desativar o desligamento automático, mantenha a tecla SELECT pressionada ao ligar o multímetro (dSAPO aparece).
    Se o multímetro não for encontrado ou se a senha de conexão tiver sido alterada, mantenha a tecla Hz pressionada enquanto gira a chave seletora de OFF para capacitância em menos de 0.6 s: o multímetro exibe “Org”, e a senha de conexão e o nome Bluetooth voltam às configurações de fábrica.
# Used by: OW69B
activation-ow69b =
    1. Desconecte o multímetro de qualquer outro aplicativo
    2. Ligue o multímetro
    3. Mantenha pressionada a tecla < (com BLE impresso acima dela) até o ícone do Bluetooth aparecer
# Used by: OW67B
activation-ow67b =
    1. Desconecte o multímetro de qualquer outro aplicativo
    2. Ligue o multímetro
    3. Mantenha pressionada a tecla < (com BLE impresso acima dela) até o ícone do Bluetooth aparecer
    Observação: o Bluetooth desliga após 5 minutos de inatividade; mantenha a tecla < pressionada de novo para religá-lo.
# Used by: VC915, VC925 PV
activation-vc915 =
    1. Desconecte o multímetro de qualquer outro aplicativo
    2. Ligue o multímetro
    3. Mantenha a tecla BLE pressionada por cerca de 2 segundos, até o multímetro emitir um bip e o símbolo do Bluetooth aparecer
# Used by: VC871, VC891
activation-vc871 =
    1. Desconecte o multímetro de qualquer outro aplicativo
    2. Ligue o multímetro
    3. Mantenha a tecla BLE pressionada por cerca de 2 segundos, até o multímetro emitir um bip e o símbolo do Bluetooth aparecer no canto superior esquerdo
    Observação: o Bluetooth vem desligado sempre que o multímetro é ligado; para ativá-lo, mantenha a tecla BLE pressionada de novo.
# Used by: OW18B/OW16B, OW18E
activation-ow18b =
    1. Desconecte o multímetro de qualquer outro aplicativo
    2. Ligue o multímetro
    3. Mantenha a tecla Hz/Duty△/ᛒ pressionada até o ícone ᛒ aparecer
    Observação: após 10 minutos de inatividade, o multímetro emite dois bipes e desliga o Bluetooth; mantenha a tecla Hz/Duty△/ᛒ pressionada de novo para religá-lo.
# Used by: OW65B
activation-ow65b =
    1. Desconecte o multímetro de qualquer outro aplicativo
    2. Ligue o multímetro
    3. Mantenha pressionada a tecla SETUP (com BLE impresso acima dela) até o ícone do Bluetooth aparecer
    Observação: para desligar o Bluetooth, mantenha a tecla SETUP pressionada de novo.
# Used by: CMS061, CMS101
activation-cms061 =
    1. Desconecte o multímetro de qualquer outro aplicativo
    2. Ligue o multímetro
    3. Mantenha pressionada a tecla Tab⇌ (com BLE impresso acima dela) até o ícone do Bluetooth aparecer
    Observação: o multímetro não se desliga sozinho enquanto o Bluetooth está ligado.
# Used by: CM2100B
activation-cm2100b =
    1. Desconecte o multímetro de qualquer outro aplicativo
    2. Ligue o multímetro
    3. Mantenha a tecla ZERO/ᛒ pressionada por cerca de 2 segundos, até o ícone ᛒ aparecer no canto superior esquerdo do display
    Observação: o Bluetooth desliga após 5 minutos de inatividade ou ao manter a tecla ZERO/ᛒ pressionada de novo. O multímetro desliga após cerca de 15 minutos de inatividade; para desativar o desligamento automático até a próxima vez que o multímetro for desligado, mantenha a tecla SELECT pressionada ao ligá-lo.
# Used by: B33
activation-b33 =
    1. Desconecte o multímetro de qualquer outro aplicativo
    2. Ligue o multímetro
    3. Mantenha a tecla △/ᛒ pressionada até o ícone ᛒ aparecer
    Observação: após 10 minutos de inatividade, o multímetro emite dois bipes e desliga o Bluetooth, que também desliga ao fim de um registro offline; mantenha a tecla △/ᛒ pressionada de novo para religá-lo.
# Used by: B35T+, B41T+
activation-b35t-plus =
    1. Desconecte o multímetro de qualquer outro aplicativo
    2. Ligue o multímetro
    3. Mantenha a tecla △/ᛒ pressionada até o ícone ᛒ aparecer
    Observação: após 10 minutos de inatividade, o multímetro emite dois bipes e desliga o Bluetooth; mantenha a tecla △/ᛒ pressionada de novo para religá-lo.
# Used by: ZT-5B
activation-zt5b =
    1. Mantenha a tecla vermelha pressionada por 2 segundos para ligar o multímetro
    2. Pressione brevemente a tecla vermelha; o símbolo do Bluetooth aparece
    Observação: o multímetro desliga após 15 minutos de inatividade; para desativar o desligamento automático, pressione NCV antes de ligar o multímetro.
# Used by: UT161B, UT161D, UT161E, UT61B+, UT61D+, UT61E+
activation-ut161b =
    1. Insira o módulo USB no multímetro
    2. Ligue o multímetro
    3. Mantenha a tecla USB/Hz pressionada
    4. O ícone S aparece no LCD
# Used by: 121GW
activation-121gw =
    1. Gire a chave seletora para qualquer função
    2. Mantenha a tecla 1ms PEAK pressionada até BT aparecer no display
    Observação: o multímetro desliga após 30 minutos; para desativar o desligamento automático, selecione APO.oF em SETUP.
# Used by: ZT-300AB
activation-zt300ab =
    1. Ligue o multímetro
    2. Mantenha a tecla Hz% pressionada por 2 segundos; o símbolo do Bluetooth aparece
    Observação: o multímetro desliga após 15 minutos de inatividade; para desativar o desligamento automático, mantenha a tecla SEL pressionada ao ligar o multímetro.
# Used by: UT60BT
activation-ut60bt =
    1. Ligue o multímetro
    2. Mantenha a tecla SEL pressionada até o símbolo do Bluetooth aparecer
# Used by: ZT-5566SE
activation-zt5566se =
    1. Ligue o multímetro
    2. Pressione POWER para ligar o Bluetooth; o símbolo do Bluetooth pisca
    Observação: o manual só menciona o Bluetooth para o alto-falante; as leituras recebidas via Bluetooth não foram verificadas.
# Used by: ZT-5BQ
activation-zt5bq =
    1. Ligue o multímetro
    2. Pressione Power e Hz ao mesmo tempo; o símbolo do Bluetooth aparece
    Observação: o multímetro desliga após 15 minutos de inatividade; para desativar o desligamento automático, mantenha a tecla Hz/NCV pressionada ao ligar o multímetro.
# Used by: UT202BT
activation-ut202bt =
    1. Ligue o multímetro
    2. Pressione brevemente a tecla Bluetooth; o símbolo do Bluetooth pisca
    Observação: o Bluetooth se desliga sozinho após 5 minutos sem conexão.
# Used by: Mock UT61E+, Mock ZT-5B
activation-mock = Não é preciso configurar nada: este é um dispositivo simulado.
