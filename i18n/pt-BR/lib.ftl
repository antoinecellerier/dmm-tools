### Text the library shows: the labels of readings and of the choices a meter
### offers, meter keys, the steps that switch a meter's data transmission on,
### and the specification tables.
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
# Used by: 42 meters
mode-ac-a = AC A
# Used by: 44 meters
mode-ac-v = AC V
# Used by: Mock UT61E+
mode-ac-v-hz = AC V Hz
# Used by: 121GW
mode-ac-va = AC VA
# Used by: 27 meters
mode-ac-ma = AC mA
# Used by: 23 meters
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
# Used by: 26 meters
mode-ac-ua = AC µA
# Used by: 121GW
mode-ac-uva = AC µVA
# Used by: 16 meters
mode-ac-plus-dc-a = AC+DC A
# Used by: 29 meters
mode-ac-plus-dc-v = AC+DC V
# Used by: 8 meters
mode-ac-plus-dc-ma = AC+DC mA
# Used by: 8 meters
mode-ac-plus-dc-mv = AC+DC mV
# Used by: 8 meters
mode-ac-plus-dc-ua = AC+DC µA
# Used by: VC-880, VC-890, VC650BT
mode-acv-lpf = ACV LPF
# Used by: UT803
mode-adp = ADP
# Used by: 9 meters
mode-angle = Ângulo
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
mode-auto = Auto
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-auto-v = Auto V
# Used by: 46 meters
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
# Used by: 45 meters
mode-continuity = Continuidade
# Used by: UT181A
mode-continuity-open = Continuidade (circuito aberto)
# Used by: 42 meters
mode-dc-a = DC A
# Used by: 44 meters
mode-dc-v = DC V
# Used by: 121GW
mode-dc-va = DC VA
# Used by: 28 meters
mode-dc-ma = DC mA
# Used by: 27 meters
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
# Used by: 27 meters
mode-dc-ua = DC µA
# Used by: 121GW
mode-dc-uva = DC µVA
# Used by: 45 meters
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
# Used by: 8 meters
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
# Used by: UT71A/B, UT71C/D/E, UT804, VC920/VC940/VC960
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
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1 = T1
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1-t2 = T1-T2
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t2 = T2
# Used by: UT803
mode-tachometer = Tacômetro
# Used by: Mock UT61E+
mode-temp-degc = Temp °C
# Used by: Mock UT61E+
mode-temp-degc-t1-t2 = Temp °C T1 (T2)
# Used by: Mock UT61E+
mode-temp-degc-t1-t2-2 = Temp °C T1-T2
# Used by: Mock UT61E+
mode-temp-degc-t2-t1 = Temp °C T2-T1
# Used by: UT71A/B, UT71C/D/E, UT803, UT804, VC920/VC940/VC960
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

## Meter keys: the remote panel's buttons, named after the meter's keys.
## Keep a name the meter prints on its key (SEL, ZERO) as it is.

# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-auto-function = Função automática
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-capacitance = Capacitância
# Used by: ZT-300AB, ZT-5566SE, ZT-5BQ
key-current = Corrente
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-diode-continuity = Diodo / continuidade
# Used by: 9 meters
key-hz-duty = Hz/Duty
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-ncv = NCV
# Used by: CM2100B, Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-zero = ZERO

## Meter key hovers: how to do on the meter what a remote button does.
## Keep the meter's key names as it prints them.

# Used by: 121GW
key-hover-hold-the-meter-s-rel-key = Manter a tecla REL do multímetro pressionada (filtro passa-baixa de 1 kHz)
# Used by: OW18B/OW16B, OW18E
key-hover-press-the-meter-s-hz-duty = Pressionar brevemente a tecla Hz/Duty△/ᛒ do multímetro
# Used by: CM2100B
key-hover-press-the-meter-s-zero-key = Pressionar brevemente a tecla ZERO/ᛒ do multímetro

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
    Observação: o multímetro desliga após cerca de 17 minutos de inatividade em um BM869s ou BM867s, e após 30 minutos nos outros; para desativar o desligamento automático, mantenha a tecla SELECT pressionada ao ligar o multímetro.
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
# Used by: UT71A/B, UT71C/D/E, VC920/VC940/VC960
activation-ut71ab =
    1. Conecte o cabo USB ao multímetro
    2. Ligue o multímetro
    3. Mantenha a tecla MAX MIN pressionada por 1 s, ou pressione SEND em um UT71A; o display exibe SEND
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
    1. Desconecte o multímetro de qualquer aplicativo de celular
    2. Gire a chave seletora para qualquer função, exceto Auto V/LoZ
    3. Mantenha a tecla Δ pressionada por um segundo ou mais, até ((D)) aparecer no display
    Observação: o multímetro desliga após 15 a 30 minutos de inatividade; para desativar o desligamento automático, mantenha a tecla SELECT pressionada ao ligar o multímetro (dSAPO aparece).
    Se o multímetro não for encontrado ou se a senha de conexão tiver sido alterada, mantenha a tecla Hz pressionada enquanto gira a chave seletora de OFF para capacitância em menos de 0.6 s: o multímetro exibe “Org”, e a senha de conexão e o nome Bluetooth voltam às configurações de fábrica.
# Used by: OW69B
activation-ow69b =
    1. Desconecte o multímetro de qualquer aplicativo de celular
    2. Ligue o multímetro
    3. Mantenha pressionada a tecla < (com BLE impresso acima dela) até o ícone do Bluetooth aparecer
# Used by: OW67B
activation-ow67b =
    1. Desconecte o multímetro de qualquer aplicativo de celular
    2. Ligue o multímetro
    3. Mantenha pressionada a tecla < (com BLE impresso acima dela) até o ícone do Bluetooth aparecer
    Observação: o Bluetooth desliga após 5 minutos de inatividade; mantenha a tecla < pressionada de novo para religá-lo.
# Used by: VC915, VC925 PV
activation-vc915 =
    1. Desconecte o multímetro de qualquer aplicativo de celular
    2. Ligue o multímetro
    3. Mantenha a tecla BLE pressionada por cerca de 2 segundos, até o multímetro emitir um bip e o símbolo do Bluetooth aparecer
# Used by: VC871, VC891
activation-vc871 =
    1. Desconecte o multímetro de qualquer aplicativo de celular
    2. Ligue o multímetro
    3. Mantenha a tecla BLE pressionada por cerca de 2 segundos, até o multímetro emitir um bip e o símbolo do Bluetooth aparecer no canto superior esquerdo
    Observação: o Bluetooth fica desligado sempre que o multímetro é ligado; a cada vez, mantenha a tecla BLE pressionada de novo.
# Used by: OW18B/OW16B, OW18E
activation-ow18b =
    1. Desconecte o multímetro de qualquer aplicativo de celular
    2. Ligue o multímetro
    3. Mantenha a tecla Hz/Duty△/ᛒ pressionada até o ícone ᛒ aparecer
    Observação: após 10 minutos de inatividade, o multímetro emite dois bipes e desliga o Bluetooth; mantenha a tecla Hz/Duty△/ᛒ pressionada de novo para religá-lo.
# Used by: OW65B
activation-ow65b =
    1. Desconecte o multímetro de qualquer aplicativo de celular
    2. Ligue o multímetro
    3. Mantenha pressionada a tecla SETUP (com BLE impresso acima dela) até o ícone do Bluetooth aparecer
    Observação: para desligar o Bluetooth, mantenha a tecla SETUP pressionada de novo.
# Used by: CMS061, CMS101
activation-cms061 =
    1. Desconecte o multímetro de qualquer aplicativo de celular
    2. Ligue o multímetro
    3. Mantenha pressionada a tecla Tab⇌ (com BLE impresso acima dela) até o ícone do Bluetooth aparecer
    Observação: o multímetro não se desliga sozinho enquanto o Bluetooth está ligado.
# Used by: CM2100B
activation-cm2100b =
    1. Desconecte o multímetro de qualquer aplicativo de celular
    2. Ligue o multímetro
    3. Mantenha a tecla ZERO/ᛒ pressionada por cerca de 2 segundos, até o ícone ᛒ aparecer no canto superior esquerdo do display
    Observação: o Bluetooth desliga após 5 minutos de inatividade ou quando a tecla ZERO/ᛒ é mantida pressionada de novo. O multímetro desliga após cerca de 15 minutos de inatividade; para desativar o desligamento automático até que o multímetro seja desligado de novo, mantenha a tecla SELECT pressionada ao ligá-lo.
# Used by: B33
activation-b33 =
    1. Desconecte o multímetro de qualquer aplicativo de celular
    2. Ligue o multímetro
    3. Mantenha a tecla △/ᛒ pressionada até o ícone ᛒ aparecer
    Observação: após 10 minutos de inatividade, o multímetro emite dois bipes e desliga o Bluetooth, que também desliga ao fim de um registro offline; mantenha a tecla △/ᛒ pressionada de novo para religá-lo.
# Used by: B35T+, B41T+
activation-b35t-plus =
    1. Desconecte o multímetro de qualquer aplicativo de celular
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
    Observação: o manual documenta o Bluetooth apenas para o alto-falante; as leituras recebidas via Bluetooth não foram verificadas.
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
activation-mock = Nenhuma configuração necessária: este é um dispositivo simulado.

## Specification table names, in the Specifications panel, as the
## meter's manual titles them. Keep the manual's numbering: (1), A.

# Used by: UT181A
table-1-ac-voltage = (1) Tensão alternada
# Used by: UT181A
table-10-temperature = (10) Temperatura
# Used by: UT181A
table-11-frequency = (11) Frequência
# Used by: UT181A
table-12-duty-cycle = (12) Duty cycle
# Used by: UT181A
table-13-pulse-width = (13) Largura de pulso
# Used by: UT181A
table-14-continuity-test = (14) Teste de continuidade
# Used by: UT181A
table-15-diode-test = (15) Teste de diodo
# Used by: UT181A
table-2-dc-voltage = (2) Tensão contínua
# Used by: UT181A
table-3-ac-voltage-plus-dc-voltage = (3) Tensão alternada + tensão contínua
# Used by: UT181A
table-4-ac-current = (4) Corrente alternada
# Used by: UT181A
table-5-dc-current = (5) Corrente contínua
# Used by: UT181A
table-6-ac-current-plus-dc-current = (6) Corrente alternada + corrente contínua
# Used by: UT181A
table-7-resistance = (7) Resistência
# Used by: UT181A
table-8-conductance = (8) Condutância
# Used by: UT181A
table-9-capacitance = (9) Capacitância
# Used by: UT803, UT804
table-a-dc-voltage = A. Tensão contínua
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-ac-ac-plus-dc-current = Corrente alternada e alternada + contínua
# Used by: 7 meters
table-ac-current = Corrente alternada
# Used by: 9 meters
table-ac-voltage = Tensão alternada
# Used by: Mock ZT-5B, ZT-5B
table-ac-voltage-v = Tensão alternada (V)
# Used by: Mock ZT-5B, ZT-5B
table-ac-current-a = Corrente alternada (A)
# Used by: Mock ZT-5B, ZT-5B
table-ac-current-ma = Corrente alternada (mA)
# Used by: Mock UT61E+, UT161E, UT61E+
table-ac-plus-dc-voltage = Tensão alternada + contínua
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-audible-continuity-tester = Teste de continuidade audível
# Used by: Mock ZT-5B, ZT-5B
table-auto = Auto
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-acv = AutoCheck™ (ACV)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-dcv = AutoCheck™ (DCV)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-ohms = AutoCheck™ (Ohms)
# Used by: UT803
table-b-ac-voltage = B. Tensão alternada
# Used by: UT804
table-b-ac-voltage-ac-plus-dc = B. Tensão alternada (medição alternada + contínua disponível)
# Used by: UT803, UT804
table-c-dc-current = C. Corrente contínua
# Used by: 11 meters
table-capacitance = Capacitância
# Used by: Mock ZT-5B, ZT-5B
table-continuity = Continuidade
# Used by: 7 meters
table-continuity-and-diode = Continuidade e diodo
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-crest-mode-instantaneous-peak-hold = Modo de crista (retenção de pico instantâneo)
# Used by: UT803
table-d-ac-current = D. Corrente alternada
# Used by: UT804
table-d-ac-current-ac-plus-dc = D. Corrente alternada (medição alternada + contínua disponível)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-dc-ac-ac-plus-dc-ac = Tensão contínua, alternada e alternada + contínua
# Used by: 9 meters
table-dc-current = Corrente contínua
# Used by: Mock ZT-5B, ZT-5B
table-dc-voltage-v = Tensão contínua (V)
# Used by: 9 meters
table-dc-voltage = Tensão contínua
# Used by: Mock ZT-5B, ZT-5B
table-dc-current-a = Corrente contínua (A)
# Used by: Mock ZT-5B, ZT-5B
table-dc-current-ma = Corrente contínua (mA)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-diode-tester = Teste de diodo
# Used by: UT803, UT804
table-e-resistance = E. Resistência
# Used by: UT803, UT804
table-f-continuity-test = F. Teste de continuidade
# Used by: Mock ZT-5B, ZT-5B
table-frequency = Frequência
# Used by: 7 meters
table-frequency-duty-ratio = Frequência/duty cycle
# Used by: UT804
table-g-diode-test = G. Teste de diodo
# Used by: UT803
table-g-diode-testst = G. Teste de diodo
# Used by: UT803, UT804
table-h-capacitance = H. Capacitância
# Used by: UT803, UT804
table-i-frequency = I. Frequência
# Used by: UT804
table-j-duty-cycle = J. Duty cycle
# Used by: UT803
table-j-temperature = J. Temperatura
# Used by: UT804
table-k-temperature-1-1-degrees-celsius = K. Temperatura 1-1. Graus Celsius
# Used by: UT804
table-k-temperature-1-2-fahrenheit = K. Temperatura 1-2. Fahrenheit
# Used by: UT803
table-k-transistor = K. Transistor
# Used by: UT804
table-l-4-20-ma-loop-current = L. Corrente de loop 4~20 mA
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-line-level-frequency-hz = Frequência de nível de rede (~Hz)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-logic-level-frequency-hz-duty-cycle = Frequência de nível lógico (⎍Hz) e duty cycle (D%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-non-contact-ef-detection = Detecção de campo elétrico sem contato
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-ohms = Resistência
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-record-mode = Modo de registro
# Used by: 9 meters
table-resistance = Resistência
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, Mock ZT-5B, UT161D, UT61D+, ZT-5B
table-temperature = Temperatura
# Used by: Mock UT61E+, UT161E, UT61E+
table-transistor-magnification = Ganho do transistor (hFE)

## Specification accuracy bands, beside an accuracy in the
## Specifications panel: the condition it holds under, or an accuracy the manual
## gives in words. Keep every number, sign and ± exactly; a decimal comma is
## fine (0,6). A translation that changes, drops or adds a number is shown in
## English. Bands that are numbers and units only are not listed.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
band-3d-khz-plus-2d = 3d/kHz+2d
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Voltage
band-40hz-100hz-lpf = 40Hz~100Hz (LPF)
# Used by: UT803
# In: B. AC Voltage
band-50khz-100khz = >50kHZ–100kHz
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
band-dc-50hz-60hz = DC, 50Hz ~ 60Hz
# Used by: UT181A
# In: (7) Resistance
band-in-rel-state = No modo REL
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); I. Frequency
band-not-specified = Não especificado
# Used by: UT181A, UT804
# In: (9) Capacitance; H. Capacitance
band-not-specified-2 = Não especificado
# Used by: UT181A
# In: (1) AC Voltage; (3) AC Voltage + DC Voltage
band-only-for-reference = Apenas para referência
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage; DC AC & AC+DC AC Voltage
band-unspec-d = Não especif.
# Used by: UT804
# In: A. DC Voltage
band-under-rel-mode = no modo REL

## Specification inputs: a table's input impedance or overload
## protection. Keep every number, sign and ± exactly; a decimal comma is fine
## (0,6). A translation that changes, drops or adds a number is shown in
## English.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: 11 tables
input-1000v-dc-ac-rms = 1000V DC/AC rms
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: 4 tables
input-10mohm-50pf-nominal = 10MΩ, 50pF nominal
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage; DC AC & AC+DC AC Voltage; DC Voltage
input-10mohm-80pf-nominal = 10MΩ, 80pF nominal
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: 6 tables
input-1100v-dc-ac-rms = 1100V DC/AC rms
# Used by: UT803
# In: 6 tables
input-250v-rms = 250V rms
# Used by: 8 meters
# In: 6 tables
input-about-10mohm = Cerca de 10MΩ
# Used by: 7 meters
# In: DC Voltage
input-about-1gohm = Cerca de 1GΩ
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
input-approx-10mohm = Aprox. 10MΩ
# Used by: UT803, UT804
# In: A. DC Voltage; B. AC Voltage
input-around-10mohm = Cerca de 10MΩ
# Used by: UT804
# In: A. DC Voltage
input-around-2-5gohm = Cerca de 2.5GΩ
# Used by: UT803
# In: A. DC Voltage; B. AC Voltage
input-around-3000mohm = Cerca de > 3000MΩ
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current; Line Level Frequency (~Hz)
input-fuse-0-44a-1000v = Fusível 0.44A 1000V
# Used by: UT804
# In: C. DC Current; D. AC Current (AC+DC measurement is available); L. 4~20 mA loop current
input-fuse-0-5a-250v = Fusível 0.5A 250V
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
input-fuse-0-8a-1000v = Fusível 0.8A 1000V
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
input-fuse-10a-1000v = Fusível 10A 1000V
# Used by: Mock UT61E+, UT61B+, UT61D+, UT61E+
# In: AC Current; DC Current
input-fuse-10a-240v = Fusível 10A 240V
# Used by: UT803, UT804
# In: C. DC Current; D. AC Current; D. AC Current (AC+DC measurement is available)
input-fuse-10a-250v = Fusível 10A 250V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, UT161B, UT161D, UT161E
# In: 4 tables
input-fuse-11a-1000v = Fusível 11A 1000V
# Used by: Mock UT61E+, UT61B+, UT61D+, UT61E+
# In: AC Current; DC Current
input-fuse-1a-240v = Fusível 1A 240V
# Used by: UT803
# In: K. Transistor
input-fuse-200ma-250v-fuse-500ma-125v = Fusível 200mA 250V; fusível 500mA 125V
# Used by: UT803
# In: C. DC Current; D. AC Current
input-fuse-500ma-125v = Fusível 500mA 125V
# Used by: UT161B, UT161D, UT161E
# In: AC Current; DC Current
input-fuse-600ma-1000v = Fusível 600mA 1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
input-initially-approx-3-0kohm-150pf-nominal = Inicialmente aprox. 3.0kΩ, 150pF nominal
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
input-initially-approx-3-0kohm-165pf-nominal = Inicialmente aprox. 3.0kΩ, 165pF nominal

## Specification notes, under a table in the Specifications panel, from
## the meter's manual. Keep the exact sense, comparisons and qualifiers such as
## "typical" included. Keep every number, sign and ± exactly; a decimal comma
## is fine (0,6). A translation that changes, drops or adds a number is shown
## in English.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-10a-continuous-10a-to-20a-30s = 10A contínuo; >10A a 20A: 30s máx., 5 min de resfriamento
# Used by: UT804
# In: C. DC Current; D. AC Current (AC+DC measurement is available)
note-10a-5a-continuous-5a-10a-10s = 10A: ≤5A contínuo; >5A–10A ≤10s por vez, com intervalo >15min
# Used by: UT181A
# In: (13) Pulse Width
note-10hz-200khz-pulse-width-2us-depends = 10Hz–200kHz: largura de pulso >2µs, depende da frequência do sinal
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
note-20a-30s-on-then-10min-off = 20A: 30s de medição, depois 10min de pausa; não especificado acima de 10A
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-220mf-range-measurement-takes-20s = Faixa de 220mF: a medição leva ≈20s
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-25-00mf-manual-range-not-specified = 25.00mF, faixa manual: não especificado abaixo de 5.4mF
# Used by: UT804
# In: E. Resistance
note-400ohm-accuracy-plus-test-lead-open = 400Ω: precisão + valor das pontas de prova em circuito aberto
# Used by: UT804
# In: H. Capacitance
note-40nf-accuracy-plus-open-circuit-test = 40nF: precisão + capacitância das pontas de prova em circuito aberto
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-0-pct-20ma-shows = 4mA corresponde a 0%, 20mA a 100%
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-6-000mf-manual-range-not-specified = 6.000mF, faixa manual: não especificado abaixo de 0.54mF
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms)
note-60-00mohm-tempco-0-6-accuracy = 60.00MΩ: coeficiente de temperatura 0.6× precisão/°C @(0°C ~ 18°C ou 28°C ~ 45°C)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-60-00mv-600-0mv-1khz-20khz = 60.00mV, 600.0mV, 1kHz ~ 20kHz: especificado de 30% a 100% da faixa
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage
note-60-00mv-600-0mv-3khz-20khz = 60.00mV, 600.0mV, 3kHz ~ 20kHz: especificado de 30% a 100% da faixa
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-600-0uf-manual-range-not-specified = 600.0µF, faixa manual: não especificado abaixo de 50.0µF
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-600ohm-ref-typical-acmv-42-22 = Ref. 600Ω, típico: ACmV -42.22 a -02.22dBm, ACV -17.78 a 62.22dBm
# Used by: UT803
# In: E. Resistance
note-600ohm-accuracy-plus-test-lead-short = 600Ω: precisão + resistência das pontas de prova em curto-circuito
# Used by: UT181A
# In: (7) Resistance
note-60mohm-humidity-50-pct = 60MΩ: umidade <50%
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Capacitance
note-60mf-range-measurement-takes-20s = Faixa de 60mF: a medição leva ≈20s
# Used by: UT181A
# In: (2) DC Voltage
note-60mv-use-rel-to-compensate-bias = 60mV: use REL para compensar a tensão de polarização
# Used by: UT803
# In: H. Capacitance
note-6nf-60nf-600nf-subtract-open-circuit = 6nF, 60nF, 600nF: subtraia a capacitância das pontas de prova em circuito aberto
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-9-999v-1khz-20khz-specified-for = 9.999V, 1kHz ~ 20kHz: especificado apenas para 1kHz ~ 15kHz
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage
note-9-999v-3khz-20khz-specified-for = 9.999V, 3kHz ~ 20kHz: especificado apenas para 3kHz ~ 15kHz
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-lo-20ma-shows-hi = <4mA: exibe LO; >20mA: exibe HI
# Used by: 7 meters
# In: AC Current; DC Current
note-5a-max-10s-rest-15-min = >5A: máx. 10s, repouso ≥15 min
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-10-00a-15-00-3 = AC 10.00A: 15.00 ~ 3.000kHz, sensibilidade 6A (RMS senoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-6-000a-15-00-3 = AC 6.000A: 15.00 ~ 3.000kHz, sensibilidade 4A (RMS senoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-60-00ma-15-00-3 = AC 60.00mA: 15.00 ~ 3.000kHz, sensibilidade 40mA (RMS senoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-60-00mv-15-00-50 = AC 60.00mV: 15.00 ~ 50.00kHz, sensibilidade 40mV (RMS senoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0ma-15-00-3 = AC 600.0mA: 15.00 ~ 3.000kHz, sensibilidade 60mA (RMS senoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0mv-15-00-50 = AC 600.0mV: 15.00 ~ 50.00kHz, sensibilidade 60mV (RMS senoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0ua-15-00-3 = AC 600.0µA: 15.00 ~ 3.000kHz, sensibilidade 200µA (RMS senoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-6000ua-15-00-3-000khz = AC 6000µA: 15.00 ~ 3.000kHz, sensibilidade 600µA (RMS senoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-9-999v-15-00-10 = AC 9.999V: 15.00 ~ 10.00kHz, sensibilidade 2.5V (RMS senoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-99-99v-15-00-10 = AC 99.99V: 15.00 ~ 10.00kHz, sensibilidade 25V (RMS senoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-999-9v-15-00-10 = AC 999.9V: 15.00 ~ 10.00kHz, sensibilidade 100V (RMS senoidal)
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-ac-plus-dc-add-1-pct = AC+DC: acrescente 1%+35 dígitos à precisão
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-ac-plus-dc-range-accuracy-plus = AC+DC: precisão da faixa + 1%
# Used by: UT804
# In: K. Temperature 1-1. Degrees Celsius; K. Temperature 1-2. Fahrenheit
note-above-230degc-use-the-rod-contact = Acima de 230°C: use o sensor de contato tipo haste
# Used by: UT803
# In: J. Temperature
note-above-230degc-use-the-rod-type = Acima de 230°C: use o sensor tipo haste
# Used by: UT181A
# In: (10) Temperature
note-accessory-point-contact-probe-below-230degc = Sensor acessório de contato pontual: apenas abaixo de 230°C
# Used by: 7 meters
# In: 4 tables
note-accuracy-valid-1-pct-100-pct = Precisão válida para 1%–100% da faixa
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-accuracy-valid-1-pct-100-pct-2 = Precisão válida para 1%–100% da faixa (1kHz–10kHz: 10%–100%)
# Used by: UT161D, UT61D+
# In: AC Current
note-accuracy-valid-40hz-1khz-1-pct = Precisão válida para 40Hz–1kHz, 1%–100% da faixa (600.0µA: 5%–100%)
# Used by: UT161D, UT61D+
# In: AC Voltage
note-accuracy-valid-40hz-1khz-1-pct-2 = Precisão válida para 40Hz–1kHz, 1%–100% da faixa (60mV: 2%–100%)
# Used by: UT161B, UT61B+
# In: AC Current
note-accuracy-valid-40hz-500hz-1-pct = Precisão válida para 40Hz–500Hz, 1%–100% da faixa (600.0µA: 5%–100%)
# Used by: UT161B, UT61B+
# In: AC Voltage
note-accuracy-valid-40hz-500hz-1-pct-2 = Precisão válida para 40Hz–500Hz, 1%–100% da faixa (60mV: 2%–100%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-accuracy-with-film-capacitor-or-better = Precisão medida com capacitor de filme ou de melhor qualidade
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-audible-threshold-between-20ohm-and-300ohm = Limiar audível: entre 20Ω e 300Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-bm821s-specified-from-10-pct-to = BM821s: especificado de 10% a 100% da faixa
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-bar-segments-beep-tones-proportional-to = Segmentos da barra e tons de bip proporcionais à intensidade do campo
# Used by: Mock ZT-5B, ZT-5B
# In: Continuity
note-beeps-50ohm = Bip <50Ω
# Used by: UT803
# In: F. Continuity Test
note-beeps-continuously-10ohm-silent-30ohm = Bip contínuo ≤10Ω; silencioso >30Ω
# Used by: UT804
# In: F. Continuity Test
note-beeps-continuously-10ohm-silent-50ohm = Bip contínuo ≤10Ω; silencioso >50Ω
# Used by: UT181A
# In: (15) Diode Test
note-brief-beep-on-a-normal-junction = Bip breve em uma junção normal, contínuo se em curto
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-0-02v-a = Queda de tensão 0.02V/A
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-0-08mv-ua = Queda de tensão 0.08mV/µA
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-2-1mv-ma = Queda de tensão 2.1mV/mA
# Used by: 7 meters
# In: Continuity and Diode
note-continuity-audio-visual-alarm-50ohm-no = Continuidade: alarme sonoro/visual <50Ω; sem bip ≥70Ω
# Used by: UT803
# In: B. AC Voltage
note-crest-factor-3-0-1000v-1 = Fator de crista 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage; DC AC & AC+DC AC Voltage
note-crest-factor-2-1-at-full = Fator de crista < 2:1 em fundo de escala, < 4:1 em meia escala
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-crest-factor-up-to-3-0 = Fator de crista até 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Crest mode (Instantaneous Peak Hold)
note-crest-specified-accuracy-adds-250-digits = Crista: acrescente 250 dígitos à precisão especificada para variações > 1.0 ms
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-detects-50-60hz-antenna-at-the = Detecta 50/60Hz; antena na extremidade superior do multímetro
# Used by: 7 meters
# In: Continuity and Diode
note-diode-one-beep-0-12v-2v = Diodo: um bip 0.12V–2V (normal), bip longo <0.12V (curto)
# Used by: 7 meters
# In: Continuity and Diode
note-diode-open-circuit-3v = Diodo: circuito aberto ≈3V
# Used by: UT181A
# In: (9) Capacitance
note-display-digits-6000 = Display: 6000 contagens
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-duty-cycle-specified-for-5hz-10khz = Duty cycle especificado para 5Hz ~ 10kHz
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-duty-square-waves-only-1vpp-20vpp = Duty: apenas ondas quadradas, 1Vpp–20Vpp, ≤10kHz, 10.0%–90.0%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-for-ac-voltage-shorted-leads-residual = Em tensão alternada, pontas de prova em curto: residual ≤200 dígitos
# Used by: Mock ZT-5B, ZT-5B
# In: AC Voltage (V); AC current (A); AC current (mA)
note-frequency-response-40hz-1khz = Resposta de frequência 40Hz–1kHz
# Used by: UT804
# In: G. Diode Test
note-good-silicon-junction-0-5v-0 = Junção de silício em bom estado: 0.5V–0.8V
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-hold-sel-ncv-for-ncv = Mantenha a tecla SEL/NCV pressionada para selecionar NCV
# Used by: UT181A
# In: (8) Conductance
note-humidity-50-pct = Umidade <50%
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-1mhz-600mv-20v = Entrada Hz >100kHz–1MHz: 600mV–20V rms
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-1v-20v-rms = Entrada Hz >1MHz: 1V–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-40mhz-1v-20v = Entrada Hz >1MHz–40MHz: 1V–20V rms; >40MHz não especificado
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-200mv-20v-rms = Entrada Hz ≤100kHz: 200mV–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Transistor Magnification
note-ib0-1-8ua-vce-2-5v = Ib0 ≈1.8µA, Vce ≈2.5V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV); AutoCheck™ (DCV)
note-impedance-rises-within-a-fraction-of = A impedância sobe em uma fração de segundo acima de 50V (típ.)
# Used by: UT804
# In: K. Temperature 1-1. Degrees Celsius; K. Temperature 1-2. Fahrenheit
note-included-k-type-point-contact-probe = Sensor de contato pontual tipo K incluso: apenas abaixo de 230°C
# Used by: UT803
# In: J. Temperature
note-included-point-contact-probe-below-230degc = Sensor de contato pontual incluso: apenas abaixo de 230°C
# Used by: UT803
# In: I. Frequency
note-input-10hz-1mhz-150mv-30v-rms = Entrada 10Hz–1MHz: 150mV–30V rms
# Used by: UT181A
# In: (11) Frequency
note-input-10hz-30mhz-600mv-30v-rms = Entrada 10Hz–30MHz: 600mV–30V rms
# Used by: UT804
# In: I. Frequency; J. Duty Cycle
note-input-10hz-40mhz-200mv-30v-rms = Entrada 10Hz–40MHz: 200mV–30V rms
# Used by: UT803
# In: I. Frequency
note-input-10mhz-50mhz-600mv-30v-rms = Entrada >10MHz–50MHz: 600mV–30V rms
# Used by: UT803
# In: I. Frequency
note-input-1mhz-10mhz-300mv-30v-rms = Entrada >1MHz–10MHz: 300mV–30V rms
# Used by: UT181A
# In: (11) Frequency
note-input-30mhz-not-specified = Entrada >30MHz: não especificada
# Used by: UT804
# In: I. Frequency; J. Duty Cycle
note-input-at-zero-dc-level-40mhz = Entrada com nível DC zero; >40MHz não especificado
# Used by: UT803
# In: I. Frequency
note-input-at-zero-dc-level-50mhz = Entrada com nível DC zero; >50MHz não especificado
# Used by: UT161D, UT61D+
# In: Temperature
note-k-type-thermocouple-only = Apenas termopar tipo K
# Used by: UT181A
# In: (10) Temperature
note-k-type-thermocouple-two-channels-via = Termopar tipo K; dois canais pelos conectores de temperatura
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-lo-z-threshold-plus-1-5vdc = Limiar Lo-Z: > +1.5VDC ou < -1.0VDC nominal
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-lo-z-threshold-3vac-50-60hz = Limiar Lo-Z: > 3VAC (50/60Hz) nominal
# Used by: UT803
# In: K. Transistor
note-max-1000beta = Máx. 1000β
# Used by: Mock ZT-5B, ZT-5B
# In: Frequency
note-max-10mhz = Máx. 10MHz
# Used by: Mock ZT-5B, ZT-5B
# In: Resistance
note-max-40mohm = Máx. 40MΩ
# Used by: Mock ZT-5B, ZT-5B
# In: Capacitance
note-max-4mf = Máx. 4mF
# Used by: Mock ZT-5B, ZT-5B
# In: AC Voltage (V); DC VOLTAGE (V)
note-max-600v = Máx. 600V
# Used by: Mock ZT-5B, ZT-5B
# In: 4 tables
note-max-9-999a = Máx. 9.999A
# Used by: UT161D, UT61D+
# In: Temperature
note-measured-temperature-should-stay-below-230degc = A temperatura medida deve ficar abaixo de 230°C/446°F
# Used by: 7 meters
# In: AC Current; AC Voltage
note-non-sine-add-4-pct-crest = Não senoidal: acrescente 4% (fator de crista 1–2), 5% (2–2.5), 7% (2.5–3)
# Used by: UT181A
# In: (15) Diode Test
note-normal-silicon-junction-0-5v-0 = Junção de silício normal: 0.5V–0.8V
# Used by: UT181A
# In: (14) Continuity Test
note-open-alarm-beeps-50ohm-silent-10ohm = Alarme de circuito aberto: bip >50Ω, silencioso <10Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms); Ohms
note-open-circuit-1-2vdc-1-0vdc = Circuito aberto < 1.2VDC (< 1.0VDC na faixa de 60.00MΩ)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-open-circuit-3-5vdc = Circuito aberto < 3.5VDC
# Used by: UT803
# In: F. Continuity Test
note-open-circuit-1-2v = Circuito aberto ≈-1.2V
# Used by: UT804
# In: F. Continuity Test
note-open-circuit-1-2v-2 = Circuito aberto ≈1.2V
# Used by: 7 meters
# In: Resistance
note-open-circuit-1v = Circuito aberto ≈1V
# Used by: UT803
# In: G. Diode Testst
note-open-circuit-2-7v = Circuito aberto ≈2.7V
# Used by: UT804
# In: G. Diode Test
note-open-circuit-2-8v = Circuito aberto ≈2.8V
# Used by: UT181A
# In: (14) Continuity Test
note-open-circuit-3v = Circuito aberto ≈3V
# Used by: UT181A
# In: (15) Diode Test
note-open-circuit-3v-forward-drop-up = Circuito aberto ≈3V; queda de tensão direta até ≈3V
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; DC Current
note-open-circuit-residual-10-digits = Circuito aberto: residual ≤10 dígitos
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; DC Current
note-open-circuit-residual-5-digits = Circuito aberto: residual ≤5 dígitos
# Used by: Mock ZT-5B, ZT-5B
# In: AC current (A); DC current (A)
note-over-2a-measure-for-less-than = Acima de 2A: meça por menos de 3s
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-range-accuracy-depend-on-acmv-acv = A faixa e a precisão dependem de ACmV/ACV e da impedância de referência
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-ranges-2-2uf-add-10-digits = Faixas ≤2.2µF: acrescente 10 dígitos quando a precisão for ≤3%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Transistor Magnification
note-reading-for-reference-only = Leitura apenas para referência
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Record mode
note-record-specified-accuracy-adds-10-digits = Registro: acrescente 10 dígitos à precisão especificada para variações > 100 ms
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-a-ma-picks = Ponta de prova vermelha no terminal A mA: seleciona corrente
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-vohm-picks-v = Ponta de prova vermelha no terminal VΩ: seleciona V acima de 0.8V, senão Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-response-time-100us = Tempo de resposta < 100µs
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Capacitance
note-result-reading-open-lead-capacitance-1uf = Resultado = leitura − capacitância das pontas de prova abertas; ≤1µF: use REL
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-result-reading-open-lead-capacitance-22nf = Resultado = leitura − capacitância das pontas de prova abertas; ≤22nF: use REL
# Used by: 7 meters
# In: Resistance
note-result-reading-shorted-lead-resistance = Resultado = leitura − resistência das pontas de prova em curto
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-sel-ncv-continuity-diode-cap-hz = SEL/NCV: continuidade/diodo, capacitância, Hz, °C
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-sensitivity-2-5vp-square-wave-for = Sensibilidade: onda quadrada 2.5Vp, para famílias lógicas de 3V e 5V
# Used by: UT181A
# In: (14) Continuity Test
note-short-alarm-beeps-10ohm-silent-50ohm = Alarme de curto-circuito: bip <10Ω, silencioso >50Ω
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-shorted-input-residual-under-30-digits = Entrada em curto: residual abaixo de ~30 dígitos, sem efeito na precisão
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-shorted-leads-80-digit-residual-accuracy = Pontas de prova em curto: residual de 80 dígitos, sem efeito na precisão
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Voltage
note-shorted-leads-residual-10-digits = Pontas de prova em curto: residual ≤10 dígitos
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Voltage
note-shorted-leads-residual-3-digits = Pontas de prova em curto: residual ≤3 dígitos
# Used by: 7 meters
# In: DC Voltage
note-shorted-leads-residual-5-digits = Pontas de prova em curto: residual ≤5 dígitos
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-test-current-0-4ma-typical = Corrente de teste 0.4mA típica
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-then-typically-18kohm-100v-125kohm-300v = Depois, tipicamente 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 460kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-then-typically-18kohm-100v-125kohm-300v-2 = Depois, tipicamente 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 500kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-tolerance-10v-36v-23v-83v-59v = Tolerância: 10V~36V, 23V~83V, 59V~165V, 124V~330V, > 250V
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10 = True RMS e precisão válidos para 10%–100% da faixa
# Used by: UT804
# In: D. AC Current (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10-2 = True RMS e precisão válidos para 10%–100% da faixa; fator de crista até 3.0
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage
note-true-rms-from-10-pct-to = True RMS de 10% a 100% da faixa; BM821s: resposta ao valor médio
# Used by: UT181A
# In: 4 tables
note-true-rms-valid-10-pct-100 = True RMS válido para 10%–100% da faixa
# Used by: UT803
# In: B. AC Voltage
note-true-rms-valid-10-pct-95 = True RMS válido para 10%–95% da faixa
# Used by: UT803
# In: D. AC Current
note-true-rms-valid-10-pct-95-2 = True RMS válido para 10%–95% da faixa; fator de crista 3.0
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-true-rms-accuracy-valid-10-pct = True RMS; precisão válida para 10%–100% da faixa
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-2-0 = True RMS; fator de crista ≤2.0 em 10000, ≤1 em 22000 contagens
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-3-0 = True RMS; fator de crista ≤3.0 em 3000, ≤1.5 em 6000 contagens
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Temperature
note-type-k-thermocouple-range-accuracy-not = Faixa e precisão do termopar tipo K não incluídas
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-typical-20v-55v-110v-220v-440v = Típico: - 20V, -- 55V, --- 110V, ---- 220V, ----- 440V
# Used by: UT804
# In: J. Duty Cycle
note-valid-10-pct-90-pct-of = Válido para 10%–90% da faixa, apenas 5Hz–2kHz
# Used by: UT803
# In: K. Transistor
note-vce-2-2v-bo-10ua = Vce ≈2.2V, bo ≈10µA
# Used by: UT181A
# In: (12) Duty Cycle; (13) Pulse Width
note-when-rise-time-1us-signals-center = Com tempo de subida <1µs, os sinais se centralizam no nível de disparo
# Used by: UT803
# In: G. Diode Testst
note-working-current-1ma = Corrente de operação ≈1mA
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current
note-ua-ranges-min-30ua = Faixas de µA: mín. 30µA
# Used by: UT803
# In: C. DC Current; D. AC Current
note-5a-range-continuous-5a-range-10s = Faixa ≤5A: contínuo; faixa >5A: ≤10s por vez, com intervalo ≥15min
