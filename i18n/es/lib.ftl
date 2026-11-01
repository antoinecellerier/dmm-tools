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
mode-angle = Ángulo
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
mode-auto = Auto
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-auto-v = Auto V
# Used by: 46 meters
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
# Used by: 45 meters
mode-continuity = Continuidad
# Used by: UT181A
mode-continuity-open = Continuidad (circuito abierto)
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
# Used by: 27 meters
mode-dc-ua = DC µA
# Used by: 121GW
mode-dc-uva = DC µVA
# Used by: 45 meters
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
# Used by: 8 meters
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
# Used by: UT71A/B, UT71C/D/E, UT804, VC920/VC940/VC960
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
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1 = T1
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1-t2 = T1-T2
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t2 = T2
# Used by: UT803
mode-tachometer = Tacómetro
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

## Meter keys: the remote panel's buttons, named after the meter's keys.
## Keep a name the meter prints on its key (SEL, ZERO) as it is.

# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-auto-function = Función automática
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-capacitance = Capacitancia
# Used by: ZT-300AB, ZT-5566SE, ZT-5BQ
key-current = Corriente
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-diode-continuity = Diodo / continuidad
# Used by: 9 meters
key-hz-duty = Hz/Duty
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-ncv = NCV
# Used by: CM2100B, Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-zero = ZERO

## Meter key hovers: how to do on the meter what a remote button does.
## Keep the meter's key names as it prints them.

# Used by: 121GW
key-hover-hold-the-meter-s-rel-key = Mantener presionado el botón REL del multímetro (filtro de paso bajo de 1 kHz)
# Used by: OW18B/OW16B, OW18E
key-hover-press-the-meter-s-hz-duty = Presionar brevemente el botón Hz/Duty△/ᛒ del multímetro
# Used by: CM2100B
key-hover-press-the-meter-s-zero-key = Presionar brevemente el botón ZERO/ᛒ del multímetro

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
    2. Enciende el multímetro en una función de tensión
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
# Used by: UT71A/B, UT71C/D/E, VC920/VC940/VC960
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
    Nota: HOLD pausa los datos y EXIT desactiva SEND.
# Used by: VC-880, VC-890, VC650BT
activation-vc650bt =
    1. Conecta el cable USB al multímetro
    2. Enciende el multímetro
    3. Presiona el botón PC del multímetro
# Used by: BM788BT/BM787BT
activation-bm78xbt =
    1. Desconecta el multímetro de cualquier aplicación del teléfono
    2. Gira el selector a cualquier función excepto Auto V/LoZ
    3. Mantén presionado Δ durante un segundo o más, hasta que aparezca ((D)) en la pantalla
    Nota: el multímetro se apaga después de 15 a 30 minutos de inactividad. Mantén presionado SELECT mientras lo enciendes para desactivar el apagado automático (aparece dSAPO).
    Si no se encuentra el multímetro o se cambió su contraseña de conexión, mantén presionado el botón Hz mientras giras el selector de OFF a capacitancia en menos de 0.6 s: el multímetro muestra “Org”, y la contraseña de conexión y el nombre Bluetooth vuelven a los valores de fábrica.
# Used by: OW69B
activation-ow69b =
    1. Desconecta el multímetro de cualquier aplicación del teléfono
    2. Enciende el multímetro
    3. Mantén presionado el botón < (con BLE impreso encima) hasta que aparezca el icono Bluetooth
# Used by: OW67B
activation-ow67b =
    1. Desconecta el multímetro de cualquier aplicación del teléfono
    2. Enciende el multímetro
    3. Mantén presionado el botón < (con BLE impreso encima) hasta que aparezca el icono Bluetooth
    Nota: el Bluetooth se desactiva después de 5 minutos de inactividad. Vuelve a mantener presionado < para reactivarlo.
# Used by: VC915, VC925 PV
activation-vc915 =
    1. Desconecta el multímetro de cualquier aplicación del teléfono
    2. Enciende el multímetro
    3. Mantén presionado el botón BLE unos 2 segundos, hasta que el multímetro emita un pitido y aparezca el símbolo Bluetooth
# Used by: VC871, VC891
activation-vc871 =
    1. Desconecta el multímetro de cualquier aplicación del teléfono
    2. Enciende el multímetro
    3. Mantén presionado el botón BLE unos 2 segundos, hasta que el multímetro emita un pitido y aparezca el símbolo Bluetooth arriba a la izquierda
    Nota: el Bluetooth queda desactivado cada vez que se enciende el multímetro. Vuelve entonces a mantener presionado BLE.
# Used by: OW18B/OW16B, OW18E
activation-ow18b =
    1. Desconecta el multímetro de cualquier aplicación del teléfono
    2. Enciende el multímetro
    3. Mantén presionado el botón Hz/Duty△/ᛒ hasta que aparezca el icono ᛒ
    Nota: el Bluetooth se desactiva después de 10 minutos de inactividad, tras dos pitidos. Vuelve a mantener presionado Hz/Duty△/ᛒ para reactivarlo.
# Used by: OW65B
activation-ow65b =
    1. Desconecta el multímetro de cualquier aplicación del teléfono
    2. Enciende el multímetro
    3. Mantén presionado el botón SETUP (con BLE impreso encima) hasta que aparezca el icono Bluetooth
    Nota: si vuelves a mantener presionado SETUP, el Bluetooth se desactiva.
# Used by: CMS061, CMS101
activation-cms061 =
    1. Desconecta el multímetro de cualquier aplicación del teléfono
    2. Enciende el multímetro
    3. Mantén presionado el botón Tab⇌ (con BLE impreso encima) hasta que aparezca el icono Bluetooth
    Nota: el multímetro no se apaga automáticamente mientras el Bluetooth esté activado.
# Used by: CM2100B
activation-cm2100b =
    1. Desconecta el multímetro de cualquier aplicación del teléfono
    2. Enciende el multímetro
    3. Mantén presionado el botón ZERO/ᛒ unos 2 segundos, hasta que aparezca el icono ᛒ arriba a la izquierda de la pantalla
    Nota: el Bluetooth se desactiva después de 5 minutos de inactividad o al volver a mantener presionado ZERO/ᛒ. El multímetro se apaga después de unos 15 minutos de inactividad. Mantén presionado SELECT mientras lo enciendes para desactivar el apagado automático hasta la próxima vez que se apague.
# Used by: B33
activation-b33 =
    1. Desconecta el multímetro de cualquier aplicación del teléfono
    2. Enciende el multímetro
    3. Mantén presionado el botón △/ᛒ hasta que aparezca el icono ᛒ
    Nota: el Bluetooth se desactiva después de 10 minutos de inactividad, tras dos pitidos, y al terminar una grabación sin conexión. Vuelve a mantener presionado △/ᛒ para reactivarlo.
# Used by: B35T+, B41T+
activation-b35t-plus =
    1. Desconecta el multímetro de cualquier aplicación del teléfono
    2. Enciende el multímetro
    3. Mantén presionado el botón △/ᛒ hasta que aparezca el icono ᛒ
    Nota: el Bluetooth se desactiva después de 10 minutos de inactividad, tras dos pitidos. Vuelve a mantener presionado △/ᛒ para reactivarlo.
# Used by: ZT-5B
activation-zt5b =
    1. Mantén presionado el botón rojo durante 2 segundos para encender el multímetro
    2. Presiona brevemente el botón rojo; aparece el símbolo Bluetooth
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
    2. Mantén presionado el botón Hz% durante 2 segundos; aparece el símbolo Bluetooth
    Nota: el multímetro se apaga después de 15 minutos de inactividad. Mantén presionado SEL mientras lo enciendes para desactivar el apagado automático.
# Used by: UT60BT
activation-ut60bt =
    1. Enciende el multímetro
    2. Mantén presionado SEL hasta que aparezca el símbolo Bluetooth
# Used by: ZT-5566SE
activation-zt5566se =
    1. Enciende el multímetro
    2. Presiona POWER para activar el Bluetooth; el símbolo Bluetooth parpadea
    Nota: el manual documenta el Bluetooth solo para el altavoz del multímetro. Las lecturas transmitidas por Bluetooth no están verificadas.
# Used by: ZT-5BQ
activation-zt5bq =
    1. Enciende el multímetro
    2. Presiona Power y Hz a la vez; aparece el símbolo Bluetooth
    Nota: el multímetro se apaga después de 15 minutos de inactividad. Mantén presionado Hz/NCV mientras lo enciendes para desactivar el apagado automático.
# Used by: UT202BT
activation-ut202bt =
    1. Enciende el multímetro
    2. Presiona brevemente el botón Bluetooth; el símbolo Bluetooth parpadea
    Nota: el Bluetooth se desactiva automáticamente después de 5 minutos sin conexión.
# Used by: Mock UT61E+, Mock ZT-5B
activation-mock = No requiere configuración: es un dispositivo simulado.

## Specification table names, in the Specifications panel, as the
## meter's manual titles them. Keep the manual's numbering: (1), A.

# Used by: UT181A
table-1-ac-voltage = (1) Tensión alterna
# Used by: UT181A
table-10-temperature = (10) Temperatura
# Used by: UT181A
table-11-frequency = (11) Frecuencia
# Used by: UT181A
table-12-duty-cycle = (12) Ciclo de trabajo
# Used by: UT181A
table-13-pulse-width = (13) Ancho de pulso
# Used by: UT181A
table-14-continuity-test = (14) Prueba de continuidad
# Used by: UT181A
table-15-diode-test = (15) Prueba de diodos
# Used by: UT181A
table-2-dc-voltage = (2) Tensión continua
# Used by: UT181A
table-3-ac-voltage-plus-dc-voltage = (3) Tensión alterna + tensión continua
# Used by: UT181A
table-4-ac-current = (4) Corriente alterna
# Used by: UT181A
table-5-dc-current = (5) Corriente continua
# Used by: UT181A
table-6-ac-current-plus-dc-current = (6) Corriente alterna + corriente continua
# Used by: UT181A
table-7-resistance = (7) Resistencia
# Used by: UT181A
table-8-conductance = (8) Conductancia
# Used by: UT181A
table-9-capacitance = (9) Capacitancia
# Used by: UT803, UT804
table-a-dc-voltage = A. Tensión continua
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-ac-ac-plus-dc-current = Corriente alterna y alterna + continua
# Used by: 7 meters
table-ac-current = Corriente alterna
# Used by: 9 meters
table-ac-voltage = Tensión alterna
# Used by: Mock ZT-5B, ZT-5B
table-ac-voltage-v = Tensión alterna (V)
# Used by: Mock ZT-5B, ZT-5B
table-ac-current-a = Corriente alterna (A)
# Used by: Mock ZT-5B, ZT-5B
table-ac-current-ma = Corriente alterna (mA)
# Used by: Mock UT61E+, UT161E, UT61E+
table-ac-plus-dc-voltage = Tensión alterna + continua
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-audible-continuity-tester = Comprobador de continuidad audible
# Used by: Mock ZT-5B, ZT-5B
table-auto = Auto
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-acv = AutoCheck™ (ACV)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-dcv = AutoCheck™ (DCV)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-ohms = AutoCheck™ (Ohms)
# Used by: UT803
table-b-ac-voltage = B. Tensión alterna
# Used by: UT804
table-b-ac-voltage-ac-plus-dc = B. Tensión alterna (medición de alterna + continua disponible)
# Used by: UT803, UT804
table-c-dc-current = C. Corriente continua
# Used by: 11 meters
table-capacitance = Capacitancia
# Used by: Mock ZT-5B, ZT-5B
table-continuity = Continuidad
# Used by: 7 meters
table-continuity-and-diode = Continuidad y diodo
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-crest-mode-instantaneous-peak-hold = Modo cresta (retención de pico instantáneo)
# Used by: UT803
table-d-ac-current = D. Corriente alterna
# Used by: UT804
table-d-ac-current-ac-plus-dc = D. Corriente alterna (medición de alterna + continua disponible)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-dc-ac-ac-plus-dc-ac = Tensión continua, alterna y alterna + continua
# Used by: 9 meters
table-dc-current = Corriente continua
# Used by: Mock ZT-5B, ZT-5B
table-dc-voltage-v = Tensión continua (V)
# Used by: 9 meters
table-dc-voltage = Tensión continua
# Used by: Mock ZT-5B, ZT-5B
table-dc-current-a = Corriente continua (A)
# Used by: Mock ZT-5B, ZT-5B
table-dc-current-ma = Corriente continua (mA)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-diode-tester = Comprobador de diodos
# Used by: UT803, UT804
table-e-resistance = E. Resistencia
# Used by: UT803, UT804
table-f-continuity-test = F. Prueba de continuidad
# Used by: Mock ZT-5B, ZT-5B
table-frequency = Frecuencia
# Used by: 7 meters
table-frequency-duty-ratio = Frecuencia/ciclo de trabajo
# Used by: UT804
table-g-diode-test = G. Prueba de diodos
# Used by: UT803
table-g-diode-testst = G. Prueba de diodos
# Used by: UT803, UT804
table-h-capacitance = H. Capacitancia
# Used by: UT803, UT804
table-i-frequency = I. Frecuencia
# Used by: UT804
table-j-duty-cycle = J. Ciclo de trabajo
# Used by: UT803
table-j-temperature = J. Temperatura
# Used by: UT804
table-k-temperature-1-1-degrees-celsius = K. Temperatura 1-1. Grados Celsius
# Used by: UT804
table-k-temperature-1-2-fahrenheit = K. Temperatura 1-2. Fahrenheit
# Used by: UT803
table-k-transistor = K. Transistor
# Used by: UT804
table-l-4-20-ma-loop-current = L. Corriente de lazo de 4~20 mA
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-line-level-frequency-hz = Frecuencia a nivel de red (~Hz)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-logic-level-frequency-hz-duty-cycle = Frecuencia a nivel lógico (⎍Hz) y ciclo de trabajo (D%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-non-contact-ef-detection = Detección de campo eléctrico (EF) sin contacto
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-ohms = Resistencia
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-record-mode = Modo de grabación
# Used by: 9 meters
table-resistance = Resistencia
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, Mock ZT-5B, UT161D, UT61D+, ZT-5B
table-temperature = Temperatura
# Used by: Mock UT61E+, UT161E, UT61E+
table-transistor-magnification = Ganancia de corriente del transistor (hFE)

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
band-in-rel-state = En modo REL
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); I. Frequency
band-not-specified = No especificado
# Used by: UT181A, UT804
# In: (9) Capacitance; H. Capacitance
band-not-specified-2 = No especificado
# Used by: UT181A
# In: (1) AC Voltage; (3) AC Voltage + DC Voltage
band-only-for-reference = Solo como referencia
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage; DC AC & AC+DC AC Voltage
band-unspec-d = No espec.
# Used by: UT804
# In: A. DC Voltage
band-under-rel-mode = en modo REL

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
input-about-10mohm = Aprox. 10MΩ
# Used by: 7 meters
# In: DC Voltage
input-about-1gohm = Aprox. 1GΩ
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
input-approx-10mohm = Aprox. 10MΩ
# Used by: UT803, UT804
# In: A. DC Voltage; B. AC Voltage
input-around-10mohm = Aprox. 10MΩ
# Used by: UT804
# In: A. DC Voltage
input-around-2-5gohm = Aprox. 2.5GΩ
# Used by: UT803
# In: A. DC Voltage; B. AC Voltage
input-around-3000mohm = Aprox. > 3000MΩ
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current; Line Level Frequency (~Hz)
input-fuse-0-44a-1000v = Fusible 0.44A 1000V
# Used by: UT804
# In: C. DC Current; D. AC Current (AC+DC measurement is available); L. 4~20 mA loop current
input-fuse-0-5a-250v = Fusible 0.5A 250V
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
input-fuse-0-8a-1000v = Fusible 0.8A 1000V
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
input-fuse-10a-1000v = Fusible 10A 1000V
# Used by: Mock UT61E+, UT61B+, UT61D+, UT61E+
# In: AC Current; DC Current
input-fuse-10a-240v = Fusible 10A 240V
# Used by: UT803, UT804
# In: C. DC Current; D. AC Current; D. AC Current (AC+DC measurement is available)
input-fuse-10a-250v = Fusible 10A 250V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, UT161B, UT161D, UT161E
# In: 4 tables
input-fuse-11a-1000v = Fusible 11A 1000V
# Used by: Mock UT61E+, UT61B+, UT61D+, UT61E+
# In: AC Current; DC Current
input-fuse-1a-240v = Fusible 1A 240V
# Used by: UT803
# In: K. Transistor
input-fuse-200ma-250v-fuse-500ma-125v = Fusible 200mA 250V; fusible 500mA 125V
# Used by: UT803
# In: C. DC Current; D. AC Current
input-fuse-500ma-125v = Fusible 500mA 125V
# Used by: UT161B, UT161D, UT161E
# In: AC Current; DC Current
input-fuse-600ma-1000v = Fusible 600mA 1000V
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
note-10a-continuous-10a-to-20a-30s = 10A continuo; >10A a 20A: 30s máx., 5 min de enfriamiento
# Used by: UT804
# In: C. DC Current; D. AC Current (AC+DC measurement is available)
note-10a-5a-continuous-5a-10a-10s = 10A: ≤5A continuo; >5A–10A ≤10s cada vez, separados por >15min
# Used by: UT181A
# In: (13) Pulse Width
note-10hz-200khz-pulse-width-2us-depends = 10Hz–200kHz: ancho de pulso >2µs, depende de la frecuencia de la señal
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
note-20a-30s-on-then-10min-off = 20A: 30s de medición, luego 10min de pausa; no especificado por encima de 10A
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-220mf-range-measurement-takes-20s = Rango de 220mF: la medición tarda ≈20s
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-25-00mf-manual-range-not-specified = 25.00mF, rango manual: no especificado por debajo de 5.4mF
# Used by: UT804
# In: E. Resistance
note-400ohm-accuracy-plus-test-lead-open = 400Ω: exactitud + valor de los cables de prueba en circuito abierto
# Used by: UT804
# In: H. Capacitance
note-40nf-accuracy-plus-open-circuit-test = 40nF: exactitud + capacitancia de los cables de prueba en circuito abierto
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-0-pct-20ma-shows = 4mA muestra 0%, 20mA muestra 100%
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-6-000mf-manual-range-not-specified = 6.000mF, rango manual: no especificado por debajo de 0.54mF
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms)
note-60-00mohm-tempco-0-6-accuracy = 60.00MΩ: coeficiente de temperatura 0.6× exactitud/°C @(0°C ~ 18°C o 28°C ~ 45°C)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-60-00mv-600-0mv-1khz-20khz = 60.00mV, 600.0mV, 1kHz ~ 20kHz: especificado del 30% al 100% del rango
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage
note-60-00mv-600-0mv-3khz-20khz = 60.00mV, 600.0mV, 3kHz ~ 20kHz: especificado del 30% al 100% del rango
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-600-0uf-manual-range-not-specified = 600.0µF, rango manual: no especificado por debajo de 50.0µF
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-600ohm-ref-typical-acmv-42-22 = Ref. 600Ω, típico: ACmV -42.22 a -02.22dBm, ACV -17.78 a 62.22dBm
# Used by: UT803
# In: E. Resistance
note-600ohm-accuracy-plus-test-lead-short = 600Ω: exactitud + resistencia de los cables de prueba en cortocircuito
# Used by: UT181A
# In: (7) Resistance
note-60mohm-humidity-50-pct = 60MΩ: humedad <50%
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Capacitance
note-60mf-range-measurement-takes-20s = Rango de 60mF: la medición tarda ≈20s
# Used by: UT181A
# In: (2) DC Voltage
note-60mv-use-rel-to-compensate-bias = 60mV: usar REL para compensar la tensión de polarización
# Used by: UT803
# In: H. Capacitance
note-6nf-60nf-600nf-subtract-open-circuit = 6nF, 60nF, 600nF: restar la capacitancia de los cables de prueba en circuito abierto
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-9-999v-1khz-20khz-specified-for = 9.999V, 1kHz ~ 20kHz: especificado solo para 1kHz ~ 15kHz
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage
note-9-999v-3khz-20khz-specified-for = 9.999V, 3kHz ~ 20kHz: especificado solo para 3kHz ~ 15kHz
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-lo-20ma-shows-hi = <4mA muestra LO, >20mA muestra HI
# Used by: 7 meters
# In: AC Current; DC Current
note-5a-max-10s-rest-15-min = >5A: máx. 10s, reposo ≥15 min
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-10-00a-15-00-3 = AC 10.00A: 15.00 ~ 3.000kHz, sensibilidad 6A (valor eficaz de onda sinusoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-6-000a-15-00-3 = AC 6.000A: 15.00 ~ 3.000kHz, sensibilidad 4A (valor eficaz de onda sinusoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-60-00ma-15-00-3 = AC 60.00mA: 15.00 ~ 3.000kHz, sensibilidad 40mA (valor eficaz de onda sinusoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-60-00mv-15-00-50 = AC 60.00mV: 15.00 ~ 50.00kHz, sensibilidad 40mV (valor eficaz de onda sinusoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0ma-15-00-3 = AC 600.0mA: 15.00 ~ 3.000kHz, sensibilidad 60mA (valor eficaz de onda sinusoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0mv-15-00-50 = AC 600.0mV: 15.00 ~ 50.00kHz, sensibilidad 60mV (valor eficaz de onda sinusoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0ua-15-00-3 = AC 600.0µA: 15.00 ~ 3.000kHz, sensibilidad 200µA (valor eficaz de onda sinusoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-6000ua-15-00-3-000khz = AC 6000µA: 15.00 ~ 3.000kHz, sensibilidad 600µA (valor eficaz de onda sinusoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-9-999v-15-00-10 = AC 9.999V: 15.00 ~ 10.00kHz, sensibilidad 2.5V (valor eficaz de onda sinusoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-99-99v-15-00-10 = AC 99.99V: 15.00 ~ 10.00kHz, sensibilidad 25V (valor eficaz de onda sinusoidal)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-999-9v-15-00-10 = AC 999.9V: 15.00 ~ 10.00kHz, sensibilidad 100V (valor eficaz de onda sinusoidal)
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-ac-plus-dc-add-1-pct = AC+DC: sumar 1%+35 dígitos a la exactitud
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-ac-plus-dc-range-accuracy-plus = AC+DC: exactitud del rango + 1%
# Used by: UT804
# In: K. Temperature 1-1. Degrees Celsius; K. Temperature 1-2. Fahrenheit
note-above-230degc-use-the-rod-contact = Por encima de 230°C: usar la sonda de contacto de varilla
# Used by: UT803
# In: J. Temperature
note-above-230degc-use-the-rod-type = Por encima de 230°C: usar la sonda de varilla
# Used by: UT181A
# In: (10) Temperature
note-accessory-point-contact-probe-below-230degc = Sonda de contacto puntual accesoria: solo por debajo de 230°C
# Used by: 7 meters
# In: 4 tables
note-accuracy-valid-1-pct-100-pct = Exactitud válida en el 1%–100% del rango
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-accuracy-valid-1-pct-100-pct-2 = Exactitud válida en el 1%–100% del rango (1kHz–10kHz: 10%–100%)
# Used by: UT161D, UT61D+
# In: AC Current
note-accuracy-valid-40hz-1khz-1-pct = Exactitud válida en 40Hz–1kHz, 1%–100% del rango (600.0µA: 5%–100%)
# Used by: UT161D, UT61D+
# In: AC Voltage
note-accuracy-valid-40hz-1khz-1-pct-2 = Exactitud válida en 40Hz–1kHz, 1%–100% del rango (60mV: 2%–100%)
# Used by: UT161B, UT61B+
# In: AC Current
note-accuracy-valid-40hz-500hz-1-pct = Exactitud válida en 40Hz–500Hz, 1%–100% del rango (600.0µA: 5%–100%)
# Used by: UT161B, UT61B+
# In: AC Voltage
note-accuracy-valid-40hz-500hz-1-pct-2 = Exactitud válida en 40Hz–500Hz, 1%–100% del rango (60mV: 2%–100%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-accuracy-with-film-capacitor-or-better = Exactitud con condensador de película o mejor
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-audible-threshold-between-20ohm-and-300ohm = Umbral audible: entre 20Ω y 300Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-bm821s-specified-from-10-pct-to = BM821s: especificado del 10% al 100% del rango
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-bar-segments-beep-tones-proportional-to = Segmentos de barra y tonos de pitido proporcionales a la intensidad del campo
# Used by: Mock ZT-5B, ZT-5B
# In: Continuity
note-beeps-50ohm = Pitido <50Ω
# Used by: UT803
# In: F. Continuity Test
note-beeps-continuously-10ohm-silent-30ohm = Pitido continuo ≤10Ω; silencio >30Ω
# Used by: UT804
# In: F. Continuity Test
note-beeps-continuously-10ohm-silent-50ohm = Pitido continuo ≤10Ω; silencio >50Ω
# Used by: UT181A
# In: (15) Diode Test
note-brief-beep-on-a-normal-junction = Pitido breve en una unión normal, continuo si está en cortocircuito
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-0-02v-a = Tensión de carga 0.02V/A
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-0-08mv-ua = Tensión de carga 0.08mV/µA
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-2-1mv-ma = Tensión de carga 2.1mV/mA
# Used by: 7 meters
# In: Continuity and Diode
note-continuity-audio-visual-alarm-50ohm-no = Continuidad: alarma sonora/visual <50Ω; sin pitido ≥70Ω
# Used by: UT803
# In: B. AC Voltage
note-crest-factor-3-0-1000v-1 = Factor de cresta 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage; DC AC & AC+DC AC Voltage
note-crest-factor-2-1-at-full = Factor de cresta < 2:1 a fondo de escala, < 4:1 a media escala
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-crest-factor-up-to-3-0 = Factor de cresta hasta 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Crest mode (Instantaneous Peak Hold)
note-crest-specified-accuracy-adds-250-digits = Cresta: la exactitud especificada suma 250 dígitos para cambios > 1.0 ms
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-detects-50-60hz-antenna-at-the = Detecta 50/60Hz; antena en el extremo superior del multímetro
# Used by: 7 meters
# In: Continuity and Diode
note-diode-one-beep-0-12v-2v = Diodo: un pitido 0.12V–2V (normal), pitido largo <0.12V (cortocircuito)
# Used by: 7 meters
# In: Continuity and Diode
note-diode-open-circuit-3v = Diodo: circuito abierto ≈3V
# Used by: UT181A
# In: (9) Capacitance
note-display-digits-6000 = Cuentas de la pantalla: 6000
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-duty-cycle-specified-for-5hz-10khz = Ciclo de trabajo especificado para 5Hz ~ 10kHz
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-duty-square-waves-only-1vpp-20vpp = Ciclo de trabajo: solo ondas cuadradas, 1Vpp–20Vpp, ≤10kHz, 10.0%–90.0%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-for-ac-voltage-shorted-leads-residual = Para tensión alterna, cables de prueba en cortocircuito: lectura residual ≤200 dígitos
# Used by: Mock ZT-5B, ZT-5B
# In: AC Voltage (V); AC current (A); AC current (mA)
note-frequency-response-40hz-1khz = Respuesta de frecuencia 40Hz–1kHz
# Used by: UT804
# In: G. Diode Test
note-good-silicon-junction-0-5v-0 = Unión de silicio en buen estado: 0.5V–0.8V
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-hold-sel-ncv-for-ncv = Mantener presionado SEL/NCV para NCV
# Used by: UT181A
# In: (8) Conductance
note-humidity-50-pct = Humedad <50%
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-1mhz-600mv-20v = Entrada Hz >100kHz–1MHz: 600mV–20V rms
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-1v-20v-rms = Entrada Hz >1MHz: 1V–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-40mhz-1v-20v = Entrada Hz >1MHz–40MHz: 1V–20V rms; >40MHz no especificado
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-200mv-20v-rms = Entrada Hz ≤100kHz: 200mV–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Transistor Magnification
note-ib0-1-8ua-vce-2-5v = Ib0 ≈1.8µA, Vce ≈2.5V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV); AutoCheck™ (DCV)
note-impedance-rises-within-a-fraction-of = La impedancia aumenta en una fracción de segundo por encima de 50V (típ.)
# Used by: UT804
# In: K. Temperature 1-1. Degrees Celsius; K. Temperature 1-2. Fahrenheit
note-included-k-type-point-contact-probe = Sonda de contacto puntual tipo K incluida: solo por debajo de 230°C
# Used by: UT803
# In: J. Temperature
note-included-point-contact-probe-below-230degc = Sonda de contacto puntual incluida: solo por debajo de 230°C
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
note-input-30mhz-not-specified = Entrada >30MHz: no especificado
# Used by: UT804
# In: I. Frequency; J. Duty Cycle
note-input-at-zero-dc-level-40mhz = Entrada con nivel DC cero; >40MHz no especificado
# Used by: UT803
# In: I. Frequency
note-input-at-zero-dc-level-50mhz = Entrada con nivel DC cero; >50MHz no especificado
# Used by: UT161D, UT61D+
# In: Temperature
note-k-type-thermocouple-only = Solo termopar tipo K
# Used by: UT181A
# In: (10) Temperature
note-k-type-thermocouple-two-channels-via = Termopar tipo K; dos canales mediante los conectores de temperatura
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-lo-z-threshold-plus-1-5vdc = Umbral Lo-Z: > +1.5VDC o < -1.0VDC nominal
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-lo-z-threshold-3vac-50-60hz = Umbral Lo-Z: > 3VAC (50/60Hz) nominal
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
note-measured-temperature-should-stay-below-230degc = La temperatura medida debe mantenerse por debajo de 230°C/446°F
# Used by: 7 meters
# In: AC Current; AC Voltage
note-non-sine-add-4-pct-crest = No sinusoidal: sumar 4% (factor de cresta 1–2), 5% (2–2.5), 7% (2.5–3)
# Used by: UT181A
# In: (15) Diode Test
note-normal-silicon-junction-0-5v-0 = Unión de silicio normal: 0.5V–0.8V
# Used by: UT181A
# In: (14) Continuity Test
note-open-alarm-beeps-50ohm-silent-10ohm = Alarma de circuito abierto: pitido >50Ω, silencio <10Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms); Ohms
note-open-circuit-1-2vdc-1-0vdc = Circuito abierto < 1.2VDC (< 1.0VDC en 60.00MΩ)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-open-circuit-3-5vdc = Circuito abierto < 3.5VDC
# Used by: UT803
# In: F. Continuity Test
note-open-circuit-1-2v = Circuito abierto ≈-1.2V
# Used by: UT804
# In: F. Continuity Test
note-open-circuit-1-2v-2 = Circuito abierto ≈1.2V
# Used by: 7 meters
# In: Resistance
note-open-circuit-1v = Circuito abierto ≈1V
# Used by: UT803
# In: G. Diode Testst
note-open-circuit-2-7v = Circuito abierto ≈2.7V
# Used by: UT804
# In: G. Diode Test
note-open-circuit-2-8v = Circuito abierto ≈2.8V
# Used by: UT181A
# In: (14) Continuity Test
note-open-circuit-3v = Circuito abierto ≈3V
# Used by: UT181A
# In: (15) Diode Test
note-open-circuit-3v-forward-drop-up = Circuito abierto ≈3V; tensión directa hasta ≈3V
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; DC Current
note-open-circuit-residual-10-digits = Circuito abierto: lectura residual ≤10 dígitos
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; DC Current
note-open-circuit-residual-5-digits = Circuito abierto: lectura residual ≤5 dígitos
# Used by: Mock ZT-5B, ZT-5B
# In: AC current (A); DC current (A)
note-over-2a-measure-for-less-than = Más de 2A: medir durante menos de 3s
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-range-accuracy-depend-on-acmv-acv = El rango y la exactitud dependen de ACmV/ACV y de la impedancia de referencia
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-ranges-2-2uf-add-10-digits = Rangos ≤2.2µF: sumar 10 dígitos cuando la exactitud es ≤3%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Transistor Magnification
note-reading-for-reference-only = Lectura solo como referencia
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Record mode
note-record-specified-accuracy-adds-10-digits = Grabación: la exactitud especificada suma 10 dígitos para cambios > 100 ms
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-a-ma-picks = Cable rojo en A mA: selecciona corriente
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-vohm-picks-v = Cable rojo en VΩ: selecciona V por encima de 0.8V; si no, Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-response-time-100us = Tiempo de respuesta < 100µs
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Capacitance
note-result-reading-open-lead-capacitance-1uf = Resultado = lectura − capacitancia de los cables abiertos; ≤1µF: usar REL
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-result-reading-open-lead-capacitance-22nf = Resultado = lectura − capacitancia de los cables abiertos; ≤22nF: usar REL
# Used by: 7 meters
# In: Resistance
note-result-reading-shorted-lead-resistance = Resultado = lectura − resistencia de los cables en cortocircuito
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-sel-ncv-continuity-diode-cap-hz = SEL/NCV: continuidad/diodo, capacitancia, Hz, °C
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-sensitivity-2-5vp-square-wave-for = Sensibilidad: onda cuadrada de 2.5Vp, para familias lógicas de 3V y 5V
# Used by: UT181A
# In: (14) Continuity Test
note-short-alarm-beeps-10ohm-silent-50ohm = Alarma de cortocircuito: pitido <10Ω, silencio >50Ω
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-shorted-input-residual-under-30-digits = Entrada en cortocircuito: lectura residual inferior a ~30 dígitos, sin efecto en la exactitud
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-shorted-leads-80-digit-residual-accuracy = Cables de prueba en cortocircuito: lectura residual de 80 dígitos, sin efecto en la exactitud
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Voltage
note-shorted-leads-residual-10-digits = Cables de prueba en cortocircuito: lectura residual ≤10 dígitos
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Voltage
note-shorted-leads-residual-3-digits = Cables de prueba en cortocircuito: lectura residual ≤3 dígitos
# Used by: 7 meters
# In: DC Voltage
note-shorted-leads-residual-5-digits = Cables de prueba en cortocircuito: lectura residual ≤5 dígitos
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-test-current-0-4ma-typical = Corriente de prueba 0.4mA típica
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-then-typically-18kohm-100v-125kohm-300v = Después, típicamente 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 460kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-then-typically-18kohm-100v-125kohm-300v-2 = Después, típicamente 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 500kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-tolerance-10v-36v-23v-83v-59v = Tolerancia: 10V~36V, 23V~83V, 59V~165V, 124V~330V, > 250V
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10 = True RMS y exactitud válidos en el 10%–100% del rango
# Used by: UT804
# In: D. AC Current (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10-2 = True RMS y exactitud válidos en el 10%–100% del rango; factor de cresta hasta 3.0
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage
note-true-rms-from-10-pct-to = True RMS del 10% al 100% del rango; BM821s: respuesta al valor medio
# Used by: UT181A
# In: 4 tables
note-true-rms-valid-10-pct-100 = True RMS válido en el 10%–100% del rango
# Used by: UT803
# In: B. AC Voltage
note-true-rms-valid-10-pct-95 = True RMS válido en el 10%–95% del rango
# Used by: UT803
# In: D. AC Current
note-true-rms-valid-10-pct-95-2 = True RMS válido en el 10%–95% del rango; factor de cresta 3.0
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-true-rms-accuracy-valid-10-pct = True RMS; exactitud válida en el 10%–100% del rango
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-2-0 = True RMS; factor de cresta ≤2.0 a 10000, ≤1 a 22000 cuentas
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-3-0 = True RMS; factor de cresta ≤3.0 a 3000, ≤1.5 a 6000 cuentas
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Temperature
note-type-k-thermocouple-range-accuracy-not = Rango y exactitud del termopar tipo K no incluidos
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-typical-20v-55v-110v-220v-440v = Típico: - 20V, -- 55V, --- 110V, ---- 220V, ----- 440V
# Used by: UT804
# In: J. Duty Cycle
note-valid-10-pct-90-pct-of = Válido en el 10%–90% del rango, solo 5Hz–2kHz
# Used by: UT803
# In: K. Transistor
note-vce-2-2v-bo-10ua = Vce ≈2.2V, bo ≈10µA
# Used by: UT181A
# In: (12) Duty Cycle; (13) Pulse Width
note-when-rise-time-1us-signals-center = Con tiempo de subida <1µs, las señales se centran en el nivel de disparo
# Used by: UT803
# In: G. Diode Testst
note-working-current-1ma = Corriente de trabajo ≈1mA
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current
note-ua-ranges-min-30ua = Rangos de µA: mín. 30µA
# Used by: UT803
# In: C. DC Current; D. AC Current
note-5a-range-continuous-5a-range-10s = Rango ≤5A: continuo; rango >5A: ≤10s cada vez, separados por ≥15min
