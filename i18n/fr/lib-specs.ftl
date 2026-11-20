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
band-4000-counts = (4000 points)
# Used by: VC920/VC940/VC960
band-30-100-pct-of-mr = 30 - 100% de la plage
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
band-in-rel-state = En mode REL
# Used by: UT181A, UT804
# In: (9) Capacitance; H. Capacitance
band-not-specified-2 = Non spécifié
# Used by: UT181A
# In: (1) AC Voltage; (3) AC Voltage + DC Voltage
band-only-for-reference = À titre de référence uniquement
# Used by: CMS061, CMS101
# In: Duty Cycle (%)
band-typical-vrms-1-v-f-100hz = Typique : Vrms=1 V, f=100Hz

## Inputs: a table's input impedance or overload protection.

# Used by: UT803
# In: 6 tables
input-250v-rms = 250V rms
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
input-current-fuse-10a-1000v-voltage-1000v = Courant : fusible 10A 1000V ; tension : 1000V
# Used by: UT803
# In: K. Transistor
input-fuse-200ma-250v-fuse-500ma-125v = Fusible 200mA 250V ; fusible 500mA 125V

## Notes, under a table. Keep the exact sense, comparisons and qualifiers
## such as "typical" included.

# Used by: VC920/VC940/VC960
# In: Power
note-0-1150w-continuous-1150w-2500w-max = 0–1150W en continu ; >1150W–2500W 10s max., puis 15min de pause
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-10a-continuous-10a-to-20a-30s = 10A en continu ; >10A à 20A : 30s max., 5 min de refroidissement
# Used by: VC920/VC940/VC960
# In: AC / A; DC / A
note-10a-0-5a-continuous-5a-10a = 10A : 0–5A en continu ; >5A–10A 10s max., puis 15min de pause
# Used by: UT181A
# In: (13) Pulse Width
note-10hz-200khz-pulse-width-2us-depends = 10Hz–200kHz : largeur d’impulsion >2µs, selon la fréquence du signal
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
note-20a-30s-on-then-10min-off = 20A : 30s de mesure, puis 10min d’arrêt ; non spécifié au-delà de 10A
# Used by: UT804
# In: E. Resistance
note-400ohm-accuracy-plus-test-lead-open = 400Ω : précision + valeur des cordons de mesure en circuit ouvert
# Used by: UT71B, UT71C/D/E
# In: K. 4~20 mA loop current (UT71B/C/D/E)
note-4ma-shows-0-pct-20ma-shows-2 = À 4mA, l’écran affiche 0% ; à 20mA, 100%
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms)
note-60-00mohm-tempco-0-6-accuracy = 60.00MΩ : coefficient de température 0.6× précision/°C @(0°C ~ 18°C ou 28°C ~ 45°C)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-600ohm-ref-typical-acmv-42-22 = Réf. 600Ω, typique : ACmV -42.22 à -02.22dBm, ACV -17.78 à 62.22dBm
# Used by: UT181A
# In: (7) Resistance
note-60mohm-humidity-50-pct = 60MΩ : humidité <50%
# Used by: UT181A
# In: (2) DC Voltage
note-60mv-use-rel-to-compensate-bias = 60mV : utilisez REL pour compenser la tension de décalage
# Used by: UT803
# In: H. Capacitance
note-6nf-60nf-600nf-subtract-open-circuit = 6nF, 60nF, 600nF : soustrayez la capacité des cordons de mesure en circuit ouvert
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-lo-20ma-shows-hi = <4mA : l’écran affiche LO ; >20mA : HI
# Used by: UT71B, UT71C/D/E
# In: K. 4~20 mA loop current (UT71B/C/D/E)
note-4ma-display-shows-lo = <4mA : l’écran affiche LO
# Used by: UT71B, UT71C/D/E
# In: K. 4~20 mA loop current (UT71B/C/D/E)
note-20ma-display-shows-hi = >20mA : l’écran affiche HI
# Used by: 7 meters
# In: AC Current; DC Current
note-5a-max-10s-rest-15-min = >5A : 10s max., repos ≥15 min
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-ac-plus-dc-range-accuracy-plus = AC+DC : précision de la plage + 1%
# Used by: UT803
# In: J. Temperature
note-above-230degc-use-the-rod-type = Au-dessus de 230°C : utilisez la sonde à tige
# Used by: UT181A
# In: (10) Temperature
note-accessory-point-contact-probe-below-230degc = Sonde à contact fournie en accessoire : en dessous de 230°C uniquement
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-accuracy-valid-1-pct-100-pct-2 = Précision valable sur 1%–100% de la plage (1kHz–10kHz : 10%–100%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-accuracy-with-film-capacitor-or-better = Précision donnée avec un condensateur à film ou de meilleure qualité
# Used by: CMS061, CMS101
# In: Capacitance(F); Resistance(Ω)
note-allow-for-the-test-leads-own = Tenez compte de la résistance et de la réactance propres des cordons de mesure
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-audible-threshold-between-20ohm-and-300ohm = Seuil du bip sonore : entre 20Ω et 300Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-bm821s-specified-from-10-pct-to = BM821s : spécifié de 10% à 100% de la plage
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-bar-segments-beep-tones-proportional-to = Segments du bargraphe et bips sonores proportionnels à l’intensité du champ
# Used by: Mock ZT-5B, ZT-5B
# In: Continuity
note-beeps-50ohm = Bip sonore <50Ω
# Used by: UT71A, UT71B, UT71C/D/E
# In: F. Continuity Test
note-beeps-continuously-40ohm = Bip continu ≤40Ω
# Used by: UT71A, UT71B, UT71C/D/E
# In: B. AC Voltage (AC+DC measurement is available)
note-below-100khz-accuracy-guaranteed-10-pct = En dessous de 100kHz : précision garantie sur 10%–100%
# Used by: UT181A
# In: (15) Diode Test
note-brief-beep-on-a-normal-junction = Bip bref sur une jonction normale, continu en cas de court-circuit
# Used by: 7 meters
# In: Continuity and Diode
note-continuity-audio-visual-alarm-50ohm-no = Continuité : alarme sonore et visuelle <50Ω ; pas de bip ≥70Ω
# Used by: VC920/VC940/VC960
# In: Ω
note-continuity-beeps-50ohm = Continuité : bip sonore <50Ω
# Used by: UT803
# In: B. AC Voltage
note-crest-factor-3-0-1000v-1 = Facteur de crête 3.0 (1000V : 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage; DC AC & AC+DC AC Voltage
note-crest-factor-2-1-at-full = Facteur de crête < 2:1 à pleine échelle, < 4:1 à mi-échelle
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-crest-factor-up-to-3-0 = Facteur de crête jusqu’à 3.0 (1000V : 1.5)
# Used by: VC920/VC940/VC960
# In: AC / A; AC / V
note-crest-factor-3 = Facteur de crête ≤3
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Crest mode (Instantaneous Peak Hold)
note-crest-specified-accuracy-adds-250-digits = Crête : précision spécifiée + 250 points pour les variations > 1.0 ms
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-detects-50-60hz-antenna-at-the = Détecte les champs à 50/60Hz ; antenne à l’extrémité supérieure du multimètre
# Used by: 7 meters
# In: Continuity and Diode
note-diode-one-beep-0-12v-2v = Diode : un bip 0.12V–2V (normal), bip long <0.12V (court-circuit)
# Used by: 7 meters
# In: Continuity and Diode
note-diode-open-circuit-3v = Diode : circuit ouvert ≈3V
# Used by: UT181A
# In: (9) Capacitance
note-display-digits-6000 = Affichage : 6000 points
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-duty-cycle-specified-for-5hz-10khz = Rapport cyclique spécifié pour 5Hz ~ 10kHz
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-duty-square-waves-only-1vpp-20vpp = Rapport cyclique : signaux carrés uniquement, 1Vpp–20Vpp, ≤10kHz, 10.0%–90.0%
# Used by: VC920/VC940/VC960
# In: Ω
note-fixed-at-4000-counts = Affichage fixé à 4000 points
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-for-ac-voltage-shorted-leads-residual = En tension alternative, cordons court-circuités : résidu ≤200 points
# Used by: Mock ZT-5B, ZT-5B
# In: AC Voltage (V); AC current (A); AC current (mA)
note-frequency-response-40hz-1khz = Réponse en fréquence 40Hz–1kHz
# Used by: UT804
# In: G. Diode Test
note-good-silicon-junction-0-5v-0 = Jonction silicium saine : 0.5V–0.8V
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-hold-sel-ncv-for-ncv = Appui long sur SEL/NCV pour la fonction NCV
# Used by: UT181A
# In: (8) Conductance
note-humidity-50-pct = Humidité <50%
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-1mhz-600mv-20v = Entrée Hz >100kHz–1MHz : 600mV–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-40mhz-1v-20v = Entrée Hz >1MHz–40MHz : 1V–20V rms ; >40MHz non spécifié
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Transistor Magnification
note-ib0-1-8ua-vce-2-5v = Ib0 ≈1.8µA, Vce ≈2.5V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV); AutoCheck™ (DCV)
note-impedance-rises-within-a-fraction-of = L’impédance augmente en une fraction de seconde au-dessus de 50V (typ.)
# Used by: UT804
# In: K. Temperature 1-1. Degrees Celsius; K. Temperature 1-2. Fahrenheit
note-included-k-type-point-contact-probe = Sonde à contact de type K fournie : en dessous de 230°C uniquement
# Used by: UT803
# In: J. Temperature
note-included-point-contact-probe-below-230degc = Sonde à contact fournie : en dessous de 230°C uniquement
# Used by: UT181A
# In: (11) Frequency
note-input-30mhz-not-specified = Entrée >30MHz : non spécifiée
# Used by: UT161D, UT61D+
# In: Temperature
note-k-type-thermocouple-only = Thermocouple de type K uniquement
# Used by: UT181A
# In: (10) Temperature
note-k-type-thermocouple-two-channels-via = Thermocouple de type K ; deux voies sur les connecteurs de température
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-lo-z-threshold-plus-1-5vdc = Seuil Lo-Z : > +1.5VDC ou < -1.0VDC nominal
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-lo-z-threshold-3vac-50-60hz = Seuil Lo-Z : > 3VAC (50/60Hz) nominal
# Used by: UT803
# In: K. Transistor
note-max-1000beta = Max. 1000β
# Used by: UT161D, UT61D+
# In: Temperature
note-measured-temperature-should-stay-below-230degc = La température mesurée doit rester inférieure à 230°C/446°F
# Used by: UT71A, UT71B, UT71C/D/E
# In: F. Continuity Test
note-no-beep-60ohm = Pas de bip >60Ω
# Used by: 7 meters
# In: AC Current; AC Voltage
note-non-sine-add-4-pct-crest = Non sinusoïdal : ajoutez 4% (facteur de crête 1–2), 5% (2–2.5), 7% (2.5–3)
# Used by: UT181A
# In: (14) Continuity Test
note-open-alarm-beeps-50ohm-silent-10ohm = Alarme de circuit ouvert : bip >50Ω, silencieux <10Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms); Ohms
note-open-circuit-1-2vdc-1-0vdc = Circuit ouvert < 1.2VDC (< 1.0VDC sur 60.00MΩ)
# Used by: UT181A
# In: (15) Diode Test
note-open-circuit-3v-forward-drop-up = Circuit ouvert ≈3V ; tension directe jusqu’à ≈3V
# Used by: Mock ZT-5B, ZT-5B
# In: AC current (A); DC current (A)
note-over-2a-measure-for-less-than = Au-delà de 2A : mesurez pendant moins de 3s
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
note-power-factor-input-0-000-1 = Facteur de puissance en entrée : 0.000–1.000
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-range-accuracy-depend-on-acmv-acv = La plage et la précision dépendent de la fonction ACmV/ACV et de l’impédance de référence
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-ranges-2-2uf-add-10-digits = Plages ≤2.2µF : ajoutez 10 points quand la précision est ≤3%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Transistor Magnification
note-reading-for-reference-only = Mesure à titre de référence uniquement
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Record mode
note-record-specified-accuracy-adds-10-digits = Enregistrement : précision spécifiée + 10 points pour les variations > 100 ms
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-a-ma-picks = Cordon rouge sur la borne A mA : sélection du courant
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-vohm-picks-v = Cordon rouge sur la borne VΩ : sélection de V au-dessus de 0.8V, sinon de Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-response-time-100us = Temps de réponse < 100µs
# Used by: 7 meters
# In: Resistance
note-result-reading-shorted-lead-resistance = Résultat = mesure − résistance des cordons court-circuités
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-sel-ncv-continuity-diode-cap-hz = SEL/NCV : continuité/diode, capacité, Hz, °C
# Used by: VC920/VC940/VC960
# In: Frequency
note-sensitivity-10hz-40mhz-200mv-40mhz-not = Sensibilité 10Hz–40MHz : 200mV ; >40MHz non spécifiée
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-sensitivity-2-5vp-square-wave-for = Sensibilité : signal carré 2.5Vp, pour les familles logiques 3V et 5V
# Used by: UT181A
# In: (14) Continuity Test
note-short-alarm-beeps-10ohm-silent-50ohm = Alarme de court-circuit : bip <10Ω, silencieux >50Ω
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-shorted-input-residual-under-30-digits = Entrée court-circuitée : résidu inférieur à ~30 points, sans effet sur la précision
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-shorted-leads-80-digit-residual-accuracy = Cordons court-circuités : résidu de 80 points, sans effet sur la précision
# Used by: CMS061, CMS101
# In: On-Off
note-shows-ol-above-200-0ohm = L’écran affiche OL au-delà de 200.0Ω
# Used by: CMS061, CMS101
# In: Frequency (Hz)
note-signal-1hz-20mhz-1v-rms = Signal : 1Hz–20MHz, ≥ 1V rms
# Used by: VC920/VC940/VC960
# In: Temperature
note-supplied-wire-thermocouple-max-plus-230degc = Thermocouple filaire fourni : +230°C max.
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-test-current-0-4ma-typical = Courant de test 0.4mA typique
# Used by: VC920/VC940/VC960
# In: Diode test
note-test-voltage-max-2-8v-test = Tension de test max. 2.8V ; courant de test ≈1mA
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-then-typically-18kohm-100v-125kohm-300v = Puis typiquement 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 460kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-then-typically-18kohm-100v-125kohm-300v-2 = Puis typiquement 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 500kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-tolerance-10v-36v-23v-83v-59v = Tolérance : 10V~36V, 23V~83V, 59V~165V, 124V~330V, > 250V
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10 = True RMS et précision valables sur 10%–100% de la plage
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage
note-true-rms-from-10-pct-to = True RMS de 10% à 100% de la plage ; BM821s : mesure de la valeur moyenne
# Used by: UT71A, UT71B, UT71C/D/E
# In: B. AC Voltage (AC+DC measurement is available)
note-true-rms-valid-10-pct-100-3 = True RMS valable sur 10%–100% de la plage ; facteur de crête ≤3.0 (1000V : 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Temperature
note-type-k-thermocouple-range-accuracy-not = Plage et précision du thermocouple de type K non incluses
# Used by: CMS061, CMS101
# In: Duty Cycle (%)
note-typical-waveform-square = Forme d’onde typique : carrée
# Used by: CMS061, CMS101
# In: Frequency (Hz)
note-typical-waveform-square-or-sine = Forme d’onde typique : carrée ou sinusoïdale
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-typical-20v-55v-110v-220v-440v = Typique : - 20V, -- 55V, --- 110V, ---- 220V, ----- 440V
# Used by: UT804
# In: J. Duty Cycle
note-valid-10-pct-90-pct-of = Valable sur 10%–90% de la plage, 5Hz–2kHz uniquement
# Used by: UT803
# In: K. Transistor
note-vce-2-2v-bo-10ua = Vce ≈2.2V, bo ≈10µA
# Used by: VC920/VC940/VC960
# In: Power
note-voltage-190v-250v-ac = Tension 190V–250V AC
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
note-voltage-input-50v-250v-ac = Tension d’entrée 50V–250V AC
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
note-voltage-input-impedance-10mohm = Impédance d’entrée en tension ≈10MΩ
# Used by: UT181A
# In: (12) Duty Cycle; (13) Pulse Width
note-when-rise-time-1us-signals-center = Quand le temps de montée est <1µs, les signaux sont centrés sur le niveau de déclenchement
# Used by: UT803
# In: G. Diode Testst
note-working-current-1ma = Courant de fonctionnement ≈1mA
# Used by: VC920/VC940/VC960
# In: AC / A; AC / V
note-true-rms-accuracy-valid-10-pct = Précision True RMS valable sur 10%–100% de la plage
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-true-rms-accuracy-valid-10-pct-2 = True RMS ; précision valable sur 10%–100% de la plage
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current
note-ua-ranges-min-30ua = Plages µA : min. 30µA
# Used by: UT803
# In: C. DC Current; D. AC Current
note-5a-range-continuous-5a-range-10s = Plage ≤5A : en continu ; plage >5A : ≤10s à la fois, à ≥15min d’intervalle
