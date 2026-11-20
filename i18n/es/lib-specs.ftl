### Text of the specification tables: accuracy bands, input ratings and notes,
### from each meter's manual, as the Specifications panel shows them.
###
### Optional: a message left out shows in English.
###
### Generated from dmm-lib by `UPDATE_L10N=1 cargo test -p dmm-shared lib_ftl`.
### Entries are only ever added. The English of each one is what the library
### shows and is how the app finds it: edit it only to follow a change in the
### library, keeping the ID to keep its translations.
###
### Keep every number, sign and ± exactly; a decimal comma is fine (0,6). A
### translation that changes, drops or adds a number is shown in English. Text
### made of numbers and unit symbols only is not listed: it always shows as it
### is.


## Accuracy bands, beside an accuracy: the condition it holds under, or an
## accuracy the manual gives in words.

# Used by: VC920/VC940/VC960
band-4000-counts = (4000 cuentas)
# Used by: VC920/VC940/VC960
band-30-100-pct-of-mr = 30 - 100% del rango
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
band-3d-khz-plus-2d = 3d/kHz+2d
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Voltage
band-40hz-100hz-lpf = 40Hz~100Hz (LPF)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
band-dc-50hz-60hz = DC, 50Hz ~ 60Hz
# Used by: UT181A
# In: (7) Resistance
band-in-rel-state = En modo REL
# Used by: UT181A, UT804
# In: (9) Capacitance; H. Capacitance
band-not-specified-2 = No especificado
# Used by: UT181A
# In: (1) AC Voltage; (3) AC Voltage + DC Voltage
band-only-for-reference = Solo como referencia
# Used by: CMS061, CMS101
# In: Duty Cycle (%)
band-typical-vrms-1-v-f-100hz = Típico: Vrms=1 V, f=100Hz

## Inputs: a table's input impedance or overload protection.

# Used by: UT803
# In: 6 tables
input-250v-rms = 250V rms
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
input-current-fuse-10a-1000v-voltage-1000v = Corriente: fusible 10A 1000V; tensión: 1000V
# Used by: UT803
# In: K. Transistor
input-fuse-200ma-250v-fuse-500ma-125v = Fusible 200mA 250V; fusible 500mA 125V

## Notes, under a table. Keep the exact sense, comparisons and qualifiers
## such as "typical" included.

# Used by: VC920/VC940/VC960
# In: Power
note-0-1150w-continuous-1150w-2500w-max = 0–1150W continuo; >1150W–2500W máx. 10s y después 15min de pausa
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-10a-continuous-10a-to-20a-30s = 10A continuo; >10A a 20A: 30s máx., 5 min de enfriamiento
# Used by: VC920/VC940/VC960
# In: AC / A; DC / A
note-10a-0-5a-continuous-5a-10a = 10A: 0–5A continuo; >5A–10A máx. 10s y después 15min de pausa
# Used by: UT181A
# In: (13) Pulse Width
note-10hz-200khz-pulse-width-2us-depends = 10Hz–200kHz: ancho de pulso >2µs, depende de la frecuencia de la señal
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
note-20a-30s-on-then-10min-off = 20A: 30s de medición, luego 10min de pausa; no especificado por encima de 10A
# Used by: UT804
# In: E. Resistance
note-400ohm-accuracy-plus-test-lead-open = 400Ω: exactitud + valor de los cables de prueba en circuito abierto
# Used by: UT71B, UT71C/D/E
# In: K. 4~20 mA loop current (UT71B/C/D/E)
note-4ma-shows-0-pct-20ma-shows-2 = Con 4mA muestra 0%; con 20mA, 100%
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms)
note-60-00mohm-tempco-0-6-accuracy = 60.00MΩ: coeficiente de temperatura 0.6× exactitud/°C @(0°C ~ 18°C o 28°C ~ 45°C)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-600ohm-ref-typical-acmv-42-22 = Ref. 600Ω, típico: ACmV -42.22 a -02.22dBm, ACV -17.78 a 62.22dBm
# Used by: UT181A
# In: (7) Resistance
note-60mohm-humidity-50-pct = 60MΩ: humedad <50%
# Used by: UT181A
# In: (2) DC Voltage
note-60mv-use-rel-to-compensate-bias = 60mV: usar REL para compensar la tensión de polarización
# Used by: UT803
# In: H. Capacitance
note-6nf-60nf-600nf-subtract-open-circuit = 6nF, 60nF, 600nF: restar la capacitancia de los cables de prueba en circuito abierto
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-lo-20ma-shows-hi = Con <4mA muestra LO; con >20mA, HI
# Used by: UT71B, UT71C/D/E
# In: K. 4~20 mA loop current (UT71B/C/D/E)
note-4ma-display-shows-lo = <4mA: la pantalla muestra LO
# Used by: UT71B, UT71C/D/E
# In: K. 4~20 mA loop current (UT71B/C/D/E)
note-20ma-display-shows-hi = >20mA: la pantalla muestra HI
# Used by: 7 meters
# In: AC Current; DC Current
note-5a-max-10s-rest-15-min = >5A: máx. 10s, reposo ≥15 min
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-ac-plus-dc-range-accuracy-plus = AC+DC: exactitud del rango + 1%
# Used by: UT803
# In: J. Temperature
note-above-230degc-use-the-rod-type = Por encima de 230°C: usar la sonda de varilla
# Used by: UT181A
# In: (10) Temperature
note-accessory-point-contact-probe-below-230degc = Sonda de contacto puntual (accesorio): solo por debajo de 230°C
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-accuracy-valid-1-pct-100-pct-2 = Exactitud válida del 1% al 100% del rango (1kHz–10kHz: 10%–100%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-accuracy-with-film-capacitor-or-better = Exactitud con un condensador de película o de mejor calidad
# Used by: CMS061, CMS101
# In: Capacitance(F); Resistance(Ω)
note-allow-for-the-test-leads-own = Tener en cuenta la resistencia y la reactancia propias de los cables de prueba
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-audible-threshold-between-20ohm-and-300ohm = Umbral del pitido: entre 20Ω y 300Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-bm821s-specified-from-10-pct-to = BM821s: especificado del 10% al 100% del rango
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-bar-segments-beep-tones-proportional-to = Segmentos de la barra y pitidos proporcionales a la intensidad del campo
# Used by: Mock ZT-5B, ZT-5B
# In: Continuity
note-beeps-50ohm = Pitido <50Ω
# Used by: UT71A, UT71B, UT71C/D/E
# In: F. Continuity Test
note-beeps-continuously-40ohm = Pitido continuo ≤40Ω
# Used by: UT71A, UT71B, UT71C/D/E
# In: B. AC Voltage (AC+DC measurement is available)
note-below-100khz-accuracy-guaranteed-10-pct = Por debajo de 100kHz: exactitud garantizada del 10% al 100%
# Used by: UT181A
# In: (15) Diode Test
note-brief-beep-on-a-normal-junction = Pitido breve en una unión normal, continuo si está en cortocircuito
# Used by: 7 meters
# In: Continuity and Diode
note-continuity-audio-visual-alarm-50ohm-no = Continuidad: alarma sonora/visual <50Ω; sin pitido ≥70Ω
# Used by: VC920/VC940/VC960
# In: Ω
note-continuity-beeps-50ohm = Continuidad: pitido <50Ω
# Used by: UT803
# In: B. AC Voltage
note-crest-factor-3-0-1000v-1 = Factor de cresta 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage; DC AC & AC+DC AC Voltage
note-crest-factor-2-1-at-full = Factor de cresta < 2:1 a fondo de escala, < 4:1 a media escala
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-crest-factor-up-to-3-0 = Factor de cresta hasta 3.0 (1000V: 1.5)
# Used by: VC920/VC940/VC960
# In: AC / A; AC / V
note-crest-factor-3 = Factor de cresta ≤3
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Crest mode (Instantaneous Peak Hold)
note-crest-specified-accuracy-adds-250-digits = Cresta: sumar 250 dígitos a la exactitud especificada para cambios > 1.0 ms
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
note-display-digits-6000 = Pantalla de 6000 cuentas
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-duty-cycle-specified-for-5hz-10khz = Ciclo de trabajo especificado para 5Hz ~ 10kHz
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-duty-square-waves-only-1vpp-20vpp = Ciclo de trabajo: solo ondas cuadradas, 1Vpp–20Vpp, ≤10kHz, 10.0%–90.0%
# Used by: VC920/VC940/VC960
# In: Ω
note-fixed-at-4000-counts = Fijo en 4000 cuentas
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
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-40mhz-1v-20v = Entrada Hz >1MHz–40MHz: 1V–20V rms; >40MHz no especificado
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
# Used by: UT181A
# In: (11) Frequency
note-input-30mhz-not-specified = Entrada >30MHz: no especificado
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
# Used by: UT161D, UT61D+
# In: Temperature
note-measured-temperature-should-stay-below-230degc = La temperatura medida debe mantenerse por debajo de 230°C/446°F
# Used by: UT71A, UT71B, UT71C/D/E
# In: F. Continuity Test
note-no-beep-60ohm = Sin pitido >60Ω
# Used by: 7 meters
# In: AC Current; AC Voltage
note-non-sine-add-4-pct-crest = No sinusoidal: sumar 4% (factor de cresta 1–2), 5% (2–2.5), 7% (2.5–3)
# Used by: UT181A
# In: (14) Continuity Test
note-open-alarm-beeps-50ohm-silent-10ohm = Alarma de circuito abierto: pitido >50Ω, silencio <10Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms); Ohms
note-open-circuit-1-2vdc-1-0vdc = Circuito abierto < 1.2VDC (< 1.0VDC en 60.00MΩ)
# Used by: UT181A
# In: (15) Diode Test
note-open-circuit-3v-forward-drop-up = Circuito abierto ≈3V; tensión directa hasta ≈3V
# Used by: Mock ZT-5B, ZT-5B
# In: AC current (A); DC current (A)
note-over-2a-measure-for-less-than = Más de 2A: medir durante menos de 3s
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
note-power-factor-input-0-000-1 = Factor de potencia de entrada 0.000–1.000
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
note-record-specified-accuracy-adds-10-digits = Grabación: sumar 10 dígitos a la exactitud especificada para cambios > 100 ms
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-a-ma-picks = Con el cable rojo en A mA, el multímetro selecciona la corriente
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-vohm-picks-v = Con el cable rojo en VΩ, el multímetro selecciona V por encima de 0.8V; si no, Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-response-time-100us = Tiempo de respuesta < 100µs
# Used by: 7 meters
# In: Resistance
note-result-reading-shorted-lead-resistance = Resultado = lectura − resistencia de los cables en cortocircuito
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-sel-ncv-continuity-diode-cap-hz = SEL/NCV: continuidad/diodo, capacitancia, Hz, °C
# Used by: VC920/VC940/VC960
# In: Frequency
note-sensitivity-10hz-40mhz-200mv-40mhz-not = Sensibilidad 10Hz–40MHz: 200mV; >40MHz no especificada
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
# Used by: CMS061, CMS101
# In: On-Off
note-shows-ol-above-200-0ohm = Muestra OL por encima de 200.0Ω
# Used by: CMS061, CMS101
# In: Frequency (Hz)
note-signal-1hz-20mhz-1v-rms = Señal: 1Hz–20MHz, ≥ 1V rms
# Used by: VC920/VC940/VC960
# In: Temperature
note-supplied-wire-thermocouple-max-plus-230degc = Termopar de hilo incluido: máx. +230°C
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-test-current-0-4ma-typical = Corriente de prueba típica: 0.4mA
# Used by: VC920/VC940/VC960
# In: Diode test
note-test-voltage-max-2-8v-test = Tensión de prueba máx. 2.8V; corriente de prueba ≈1mA
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-then-typically-18kohm-100v-125kohm-300v = Después, valores típicos: 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 460kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-then-typically-18kohm-100v-125kohm-300v-2 = Después, valores típicos: 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 500kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-tolerance-10v-36v-23v-83v-59v = Tolerancia: 10V~36V, 23V~83V, 59V~165V, 124V~330V, > 250V
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10 = True RMS y exactitud válidos del 10% al 100% del rango
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage
note-true-rms-from-10-pct-to = True RMS del 10% al 100% del rango; BM821s: respuesta al valor medio
# Used by: UT71A, UT71B, UT71C/D/E
# In: B. AC Voltage (AC+DC measurement is available)
note-true-rms-valid-10-pct-100-3 = True RMS válido del 10% al 100% del rango; factor de cresta ≤3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Temperature
note-type-k-thermocouple-range-accuracy-not = No se incluyen el rango ni la exactitud del termopar tipo K
# Used by: CMS061, CMS101
# In: Duty Cycle (%)
note-typical-waveform-square = Forma de onda típica: cuadrada
# Used by: CMS061, CMS101
# In: Frequency (Hz)
note-typical-waveform-square-or-sine = Forma de onda típica: cuadrada o sinusoidal
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-typical-20v-55v-110v-220v-440v = Típico: - 20V, -- 55V, --- 110V, ---- 220V, ----- 440V
# Used by: UT804
# In: J. Duty Cycle
note-valid-10-pct-90-pct-of = Válido del 10% al 90% del rango, solo de 5Hz a 2kHz
# Used by: UT803
# In: K. Transistor
note-vce-2-2v-bo-10ua = Vce ≈2.2V, bo ≈10µA
# Used by: VC920/VC940/VC960
# In: Power
note-voltage-190v-250v-ac = Tensión 190V–250V AC
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
note-voltage-input-50v-250v-ac = Tensión de entrada 50V–250V AC
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
note-voltage-input-impedance-10mohm = Impedancia de entrada de tensión ≈10MΩ
# Used by: UT181A
# In: (12) Duty Cycle; (13) Pulse Width
note-when-rise-time-1us-signals-center = Con tiempo de subida <1µs, las señales se centran en el nivel de disparo
# Used by: UT803
# In: G. Diode Testst
note-working-current-1ma = Corriente de trabajo ≈1mA
# Used by: VC920/VC940/VC960
# In: AC / A; AC / V
note-true-rms-accuracy-valid-10-pct = Exactitud True RMS válida del 10% al 100% del rango
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-true-rms-accuracy-valid-10-pct-2 = True RMS; exactitud válida en el 10%–100% del rango
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current
note-ua-ranges-min-30ua = Rangos de µA: mín. 30µA
# Used by: UT803
# In: C. DC Current; D. AC Current
note-5a-range-continuous-5a-range-10s = Rango ≤5A: continuo; rango >5A: ≤10s cada vez, con pausas de ≥15min
