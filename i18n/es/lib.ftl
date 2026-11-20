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
mode-ac-power-a = Potencia AC A
# Used by: 9 meters
mode-ac-power-ah = Potencia AC Ah
# Used by: 9 meters
mode-ac-power-v = Potencia AC V
# Used by: 9 meters
mode-ac-power-va = Potencia AC VA
# Used by: 9 meters
mode-ac-power-w = Potencia AC W
# Used by: 9 meters
mode-ac-power-wh = Potencia AC Wh
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
mode-angle = Ángulo
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
mode-auto = Auto
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-auto-v = Auto V
# Used by: 47 meters
mode-capacitance = Capacitancia
# Used by: UT8803
mode-capacitance-d = Capacitancia D
# Used by: UT8803
mode-capacitance-r = Capacitancia R
# Used by: 8 meters
mode-clamp-ac-a = Pinza AC A
# Used by: 8 meters
mode-clamp-ac-plus-dc-a = Pinza AC+DC A
# Used by: 8 meters
mode-clamp-dc-a = Pinza DC A
# Used by: 8 meters
mode-clamp-lpf-a = Pinza LPF A
# Used by: 9 meters
mode-compass = Brújula
# Used by: 46 meters
mode-continuity = Continuidad
# Used by: UT181A
mode-continuity-open = Continuidad (circuito abierto)
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
mode-dc-power-a = Potencia DC A
# Used by: 9 meters
mode-dc-power-ah = Potencia DC Ah
# Used by: 9 meters
mode-dc-power-v = Potencia DC V
# Used by: 9 meters
mode-dc-power-va = Potencia DC VA
# Used by: 9 meters
mode-dc-power-w = Potencia DC W
# Used by: 9 meters
mode-dc-power-wh = Potencia DC Wh
# Used by: 28 meters
mode-dc-ua = DC µA
# Used by: 121GW
mode-dc-uva = DC µVA
# Used by: 46 meters
mode-diode = Diodo
# Used by: UT181A
mode-diode-alarm = Alarma de diodo
# Used by: 44 meters
mode-duty-pct = Ciclo de trabajo %
# Used by: BM829s/BM827s/BM822s/BM821s
mode-ef = EF
# Used by: BM788BT/BM787BT
mode-ef-h = EF-H
# Used by: BM788BT/BM787BT
mode-ef-l = EF-L
# Used by: 9 meters
mode-frequency = Frecuencia
# Used by: 9 meters
mode-hv-ac-v = HV AC V
# Used by: 9 meters
mode-hv-dc-v = HV DC V
# Used by: UT8803
mode-inductance = Inductancia
# Used by: UT8803
mode-inductance-q = Inductancia Q
# Used by: UT8803
mode-inductance-r = Inductancia R
# Used by: 22 meters
mode-inrush = Arranque
# Used by: BM788BT/BM787BT
mode-line-hz = Hz de red
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
mode-logic-hz = Hz lógico
# Used by: 9 meters
mode-motor = Motor
# Used by: 9 meters
mode-motor-1-2-3 = Motor 1-2-3
# Used by: 9 meters
mode-motor-3-2-1 = Motor 3-2-1
# Used by: 20 meters
mode-ncv = NCV
# Used by: UT71A, UT71B, UT71C/D/E, UT804, VC920/VC940/VC960
mode-power = Potencia
# Used by: 9 meters
mode-power-a = Potencia A
# Used by: 9 meters
mode-power-ah = Potencia Ah
# Used by: 9 meters
mode-power-v = Potencia V
# Used by: 9 meters
mode-power-va = Potencia VA
# Used by: 9 meters
mode-power-w = Potencia W
# Used by: 9 meters
mode-power-wh = Potencia Wh
# Used by: 9 meters
mode-power-factor = Factor de potencia
# Used by: 121GW, UT181A
mode-pulse-width = Ancho de pulso
# Used by: BM525s/BM521s
mode-recall = Recuperar
# Used by: UT8802, UT8803
mode-scr = SCR
# Used by: 9 meters
mode-scope = Osciloscopio
# Used by: 9 meters
mode-solar = Solar
# Used by: UT171A/B/C
mode-square-wave = Onda cuadrada
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
mode-time = Tiempo
# Used by: 9 meters
mode-usb-power-a = Potencia USB A
# Used by: 9 meters
mode-usb-power-ah = Potencia USB Ah
# Used by: 9 meters
mode-usb-power-v = Potencia USB V
# Used by: 9 meters
mode-usb-power-va = Potencia USB VA
# Used by: 9 meters
mode-usb-power-w = Potencia USB W
# Used by: 9 meters
mode-usb-power-wh = Potencia USB Wh
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
mode-dbm-reference = Referencia de dBm
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
range-duty = Ciclo de trabajo
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
aux-angle = Ángulo
# Used by: UT171A/B/C
aux-aux = Aux
# Used by: UT181A
aux-average = Promedio
# Used by: 121GW
aux-battery = Batería
# Used by: 121GW
aux-burden-voltage = Tensión de carga
# Used by: UT181A
aux-comp-high = COMP alto
# Used by: UT181A
aux-comp-low = COMP bajo
# Used by: 10 meters
aux-capacitance = Capacitancia
# Used by: UT202BT
aux-clamp-ac-a = Pinza AC A
# Used by: UT202BT
aux-clamp-lpf-a = Pinza LPF A
# Used by: 9 meters
aux-compass = Brújula
# Used by: 10 meters
aux-continuity = Continuidad
# Used by: 121GW
aux-current = Corriente
# Used by: 12 meters
aux-dc = DC
# Used by: 9 meters
aux-dc-a = DC A
# Used by: 10 meters
aux-dc-v = DC V
# Used by: 9 meters
aux-diode = Diodo
# Used by: 10 meters
aux-duty = Ciclo de trabajo
# Used by: 9 meters
aux-frequency = Frecuencia
# Used by: 9 meters
aux-hv-ac-v = HV AC V
# Used by: 9 meters
aux-hv-dc-v = HV DC V
# Used by: UT181A
aux-impedance = Impedancia
# Used by: UT202BT
aux-inrush = Arranque
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
aux-pulse-width = Ancho de pulso
# Used by: UT181A, VC871
aux-reference = Referencia
# Used by: 9 meters
aux-solar = Solar
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, Mock UT61E+, UT181A
aux-t1 = T1
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s, Mock UT61E+, UT181A
aux-t2 = T2
# Used by: UT202BT
aux-temperature = Temperatura
# Used by: 9 meters
aux-time = Tiempo
# Used by: 121GW, UT181A
aux-voltage = Tensión
# Used by: 9 meters
aux-hfe = hFE

## On and off, for a meter setting offered as a choice.

# Used by: 13 meters
toggle-off = desactivado
# Used by: 13 meters
toggle-on = activado

## Meter keys named in words for what they do. A name printed on a key is
## not listed here: it stays as printed.

# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-auto-function = Función automática
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-capacitance = Capacitancia
# Used by: ZT-300AB, ZT-5566SE, ZT-5BQ
key-current = Corriente
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-diode-continuity = Diodo / continuidad

# Used by: VC891, VC915
key-hover-1-khz-low-pass-filter = Filtro de paso bajo de 1 kHz
# Used by: 121GW
key-hover-capture-1-ms-peaks-in-ac = Capturar picos de 1 ms en AC V
# Used by: OW67B, VC871
key-hover-change-what-the-displays-show = Cambiar lo que muestran las pantallas
# Used by: VC915, VC925 PV
key-hover-compare-the-reading-with-the-set = Comparar la lectura con los límites definidos
# Used by: CMS061, CMS101
key-hover-enter-max-min-click-again-to = Entrar en el modo MAX/MIN; haz clic de nuevo para salir

## Meter key hovers: how to do on the meter what a remote button does.
## Keep the meter's key names as it prints them.

# Used by: 121GW
key-hover-hold-the-meter-s-rel-key = Mantener presionado el botón REL del multímetro (filtro de paso bajo de 1 kHz)
# Used by: CMS061, CMS101
key-hover-measure-inrush-current-click-again-to = Medir la corriente de arranque; haz clic de nuevo para salir
# Used by: UT171A/B/C
key-hover-peak-hold-in-ac-v-and = Retención de picos en AC V y A: haz clic para activarla o desactivarla
# Used by: OW18B/OW16B, OW18E
key-hover-press-the-meter-s-hz-duty = Presionar brevemente el botón Hz/Duty△/ᛒ del multímetro
# Used by: CM2100B
key-hover-press-the-meter-s-zero-key = Presionar brevemente el botón ZERO/ᛒ del multímetro
# Used by: OW18B/OW16B, OW18E
key-hover-rel-in-ac-and-hz-steps = REL; en AC y en Hz, alterna entre Hz y Duty
# Used by: OW67B, OW69B, VC871, VC891, VC915, VC925 PV
key-hover-read-a-4-20-ma-loop = Leer un lazo de 4-20 mA como porcentaje
# Used by: ZT-5566SE
key-hover-record-maximum-and-minimum-readings = Registrar las lecturas máxima y mínima
# Used by: UT171A/B/C
key-hover-record-maximum-average-and-minimum-readings-2 = Registrar las lecturas máxima, promedio y mínima: haz clic para pasar de una a otra y en × para salir
# Used by: UT181A
key-hover-record-maximum-average-and-minimum-readings = Registrar las lecturas máxima, promedio y mínima: haz clic en × para salir
# Used by: VC-880, VC-890, VC650BT
key-hover-record-maximum-minimum-and-average-readings = Registrar las lecturas máxima, mínima y promedio: haz clic para pasar de una a otra y en × para salir
# Used by: 121GW
key-hover-record-minimum-maximum-and-average-readings = Registrar las lecturas mínima, máxima y promedio: haz clic para pasar de una a otra y en × para salir
# Used by: UT171A/B/C, UT181A
key-hover-save-the-reading-to-the-meter = Guardar la lectura en la memoria del multímetro
# Used by: CMS061, CMS101
key-hover-send-select-switches-v-dc-to = Enviar Select; cambia de V,DC a V,AC
# Used by: VC915
key-hover-show-3-phase-rotation-direction = Mostrar el sentido de rotación de las tres fases
# Used by: VC915
key-hover-show-vac-or-vdc-on-the = Mostrar VAC o VDC en la pantalla secundaria
# Used by: 121GW
key-hover-step-through-hold-and-autohold = Alternar entre HOLD y AutoHOLD
# Used by: VC915
key-hover-swap-hz-pct-and-voltage = Intercambiar Hz/% y tensión
# Used by: VC925 PV
key-hover-swap-the-main-and-second-displays = Intercambiar las pantallas principal y secundaria
# Used by: OW69B
key-hover-switch-the-low-pass-filter-on = Activar o desactivar el filtro de paso bajo
# Used by: UT171A/B/C
key-hover-take-the-reading-as-the-reference = Tomar la lectura como referencia: haz clic en × para salir
# Used by: OW18B/OW16B, OW18E
key-hover-toggle-the-backlight-and-an-ow18 = Encender o apagar la luz de fondo y, en un OW18, la linterna
# Used by: VC871, VC891, VC915, VC925 PV
key-hover-zero-the-reading-to-exit = Poner la lectura a cero: haz clic en × para salir

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
    1. Fija el cable BU-86X en el puerto óptico PC-Comm de la parte trasera del multímetro y conéctalo al equipo
    2. Enciende el multímetro con el selector en una función de tensión
    Nota: el multímetro se apaga después de unos 17 minutos de inactividad en un BM869s o BM867s, y de 30 minutos en los demás. Mantén presionado SELECT mientras lo enciendes para desactivar el apagado automático.
# Used by: UT8802, UT8803
activation-ut8802 =
    1. Conecta el cable USB al multímetro
    2. Enciende el multímetro
# Used by: UT171A/B/C
activation-ut171 =
    1. Conecta el cable USB al multímetro
    2. Enciende el multímetro
    3. Ve a SETUP -> Communication -> ON
# Used by: UT181A
activation-ut181a =
    1. Conecta el cable USB al multímetro
    2. Enciende el multímetro
    3. Ve a SETUP -> Communication -> ON
    Nota: este ajuste se restablece cada vez que se apaga el multímetro.
# Used by: UT71A, UT71B, UT71C/D/E, VC920/VC940/VC960
activation-ut71ab =
    1. Conecta el cable USB al multímetro
    2. Enciende el multímetro
    3. Mantén presionado MAX MIN durante 1 s, o presiona SEND en un UT71A; la pantalla muestra SEND
    Nota: EXIT desactiva SEND.
# Used by: UT803
activation-ut803 =
    1. Conecta el cable USB al multímetro
    2. Enciende el multímetro
    3. Presiona RS232; la pantalla muestra RS232
# Used by: UT804
activation-ut804 =
    1. Conecta el cable USB al multímetro
    2. Enciende el multímetro
    3. Presiona SEND; la pantalla muestra SEND
    Nota: HOLD pausa el envío de datos y EXIT desactiva SEND.
# Used by: VC-880, VC-890, VC650BT
activation-vc650bt =
    1. Conecta el cable USB al multímetro
    2. Enciende el multímetro
    3. Presiona el botón PC del multímetro
# Used by: BM788BT/BM787BT
activation-bm78xbt =
    1. Desconecta el multímetro de cualquier otra aplicación
    2. Gira el selector a cualquier función excepto Auto V/LoZ
    3. Mantén presionado Δ durante un segundo o más, hasta que aparezca ((D)) en la pantalla
    Nota: el multímetro se apaga después de 15 a 30 minutos de inactividad. Mantén presionado SELECT mientras lo enciendes para desactivar el apagado automático (aparece dSAPO).
    Si no se encuentra el multímetro o se cambió su contraseña de conexión, mantén presionado el botón Hz mientras giras el selector de OFF a capacitancia en menos de 0.6 s: el multímetro muestra “Org”, y la contraseña de conexión y el nombre de Bluetooth vuelven a los valores de fábrica.
# Used by: OW69B
activation-ow69b =
    1. Desconecta el multímetro de cualquier otra aplicación
    2. Enciende el multímetro
    3. Mantén presionado el botón < (con BLE impreso encima) hasta que aparezca el icono de Bluetooth
# Used by: OW67B
activation-ow67b =
    1. Desconecta el multímetro de cualquier otra aplicación
    2. Enciende el multímetro
    3. Mantén presionado el botón < (con BLE impreso encima) hasta que aparezca el icono de Bluetooth
    Nota: el Bluetooth se desactiva después de 5 minutos de inactividad. Vuelve a mantener presionado < para reactivarlo.
# Used by: VC915, VC925 PV
activation-vc915 =
    1. Desconecta el multímetro de cualquier otra aplicación
    2. Enciende el multímetro
    3. Mantén presionado el botón BLE unos 2 segundos, hasta que el multímetro emita un pitido y aparezca el símbolo de Bluetooth
# Used by: VC871, VC891
activation-vc871 =
    1. Desconecta el multímetro de cualquier otra aplicación
    2. Enciende el multímetro
    3. Mantén presionado el botón BLE unos 2 segundos, hasta que el multímetro emita un pitido y aparezca el símbolo de Bluetooth arriba a la izquierda
    Nota: el Bluetooth queda desactivado cada vez que se enciende el multímetro. Después de encenderlo, vuelve a mantener presionado BLE.
# Used by: OW18B/OW16B, OW18E
activation-ow18b =
    1. Desconecta el multímetro de cualquier otra aplicación
    2. Enciende el multímetro
    3. Mantén presionado el botón Hz/Duty△/ᛒ hasta que aparezca el icono ᛒ
    Nota: el Bluetooth se desactiva con dos pitidos después de 10 minutos de inactividad. Vuelve a mantener presionado Hz/Duty△/ᛒ para reactivarlo.
# Used by: OW65B
activation-ow65b =
    1. Desconecta el multímetro de cualquier otra aplicación
    2. Enciende el multímetro
    3. Mantén presionado el botón SETUP (con BLE impreso encima) hasta que aparezca el icono de Bluetooth
    Nota: si vuelves a mantener presionado SETUP, el Bluetooth se desactiva.
# Used by: CMS061, CMS101
activation-cms061 =
    1. Desconecta el multímetro de cualquier otra aplicación
    2. Enciende el multímetro
    3. Mantén presionado el botón Tab⇌ (con BLE impreso encima) hasta que aparezca el icono de Bluetooth
    Nota: el multímetro no se apaga automáticamente mientras el Bluetooth esté activado.
# Used by: CM2100B
activation-cm2100b =
    1. Desconecta el multímetro de cualquier otra aplicación
    2. Enciende el multímetro
    3. Mantén presionado el botón ZERO/ᛒ unos 2 segundos, hasta que aparezca el icono ᛒ arriba a la izquierda de la pantalla
    Nota: el Bluetooth se desactiva después de 5 minutos de inactividad o al volver a mantener presionado ZERO/ᛒ. El multímetro se apaga después de unos 15 minutos de inactividad. Mantén presionado SELECT mientras lo enciendes para desactivar el apagado automático hasta la próxima vez que se apague.
# Used by: B33
activation-b33 =
    1. Desconecta el multímetro de cualquier otra aplicación
    2. Enciende el multímetro
    3. Mantén presionado el botón △/ᛒ hasta que aparezca el icono ᛒ
    Nota: el Bluetooth se desactiva con dos pitidos después de 10 minutos de inactividad, y también al terminar una grabación sin conexión. Vuelve a mantener presionado △/ᛒ para reactivarlo.
# Used by: B35T+, B41T+
activation-b35t-plus =
    1. Desconecta el multímetro de cualquier otra aplicación
    2. Enciende el multímetro
    3. Mantén presionado el botón △/ᛒ hasta que aparezca el icono ᛒ
    Nota: el Bluetooth se desactiva con dos pitidos después de 10 minutos de inactividad. Vuelve a mantener presionado △/ᛒ para reactivarlo.
# Used by: ZT-5B
activation-zt5b =
    1. Mantén presionado el botón rojo durante 2 segundos para encender el multímetro
    2. Presiona brevemente el botón rojo; aparece el símbolo de Bluetooth
    Nota: el multímetro se apaga después de 15 minutos de inactividad. Presiona NCV antes de encenderlo para desactivar el apagado automático.
# Used by: UT161B, UT161D, UT161E, UT61B+, UT61D+, UT61E+
activation-ut161b =
    1. Inserta el módulo USB en el multímetro
    2. Enciende el multímetro
    3. Mantén presionado el botón USB/Hz
    4. Aparece el icono S en la pantalla LCD
# Used by: 121GW
activation-121gw =
    1. Gira el selector a cualquier función
    2. Mantén presionado 1ms PEAK hasta que aparezca BT en la pantalla
    Nota: el multímetro se apaga después de 30 minutos. Configura APO.oF en SETUP para desactivar el apagado automático.
# Used by: ZT-300AB
activation-zt300ab =
    1. Enciende el multímetro
    2. Mantén presionado el botón Hz% durante 2 segundos; aparece el símbolo de Bluetooth
    Nota: el multímetro se apaga después de 15 minutos de inactividad. Mantén presionado SEL mientras lo enciendes para desactivar el apagado automático.
# Used by: UT60BT
activation-ut60bt =
    1. Enciende el multímetro
    2. Mantén presionado SEL hasta que aparezca el símbolo de Bluetooth
# Used by: ZT-5566SE
activation-zt5566se =
    1. Enciende el multímetro
    2. Presiona POWER para activar el Bluetooth; el símbolo de Bluetooth parpadea
    Nota: el manual documenta el Bluetooth solo para el altavoz del multímetro. Las lecturas transmitidas por Bluetooth no están verificadas.
# Used by: ZT-5BQ
activation-zt5bq =
    1. Enciende el multímetro
    2. Presiona Power y Hz a la vez; aparece el símbolo de Bluetooth
    Nota: el multímetro se apaga después de 15 minutos de inactividad. Mantén presionado Hz/NCV mientras lo enciendes para desactivar el apagado automático.
# Used by: UT202BT
activation-ut202bt =
    1. Enciende el multímetro
    2. Presiona brevemente el botón de Bluetooth; el símbolo de Bluetooth parpadea
    Nota: el Bluetooth se desactiva automáticamente después de 5 minutos sin conexión.
# Used by: Mock UT61E+, Mock ZT-5B
activation-mock = No requiere configuración: es un dispositivo simulado.
