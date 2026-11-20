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
mode-a-ac-peak = A AC crête
# Used by: UT171A/B/C
mode-a-ac-plus-dc = A AC+DC
# Used by: UT171A/B/C, UT181A
mode-a-dc = A DC
# Used by: UT181A
mode-a-dc-ac-plus-dc = A DC AC+DC
# Used by: UT181A
mode-a-dc-peak = A DC crête
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
mode-ac-power-a = Puissance AC A
# Used by: 9 meters
mode-ac-power-ah = Puissance AC Ah
# Used by: 9 meters
mode-ac-power-v = Puissance AC V
# Used by: 9 meters
mode-ac-power-va = Puissance AC VA
# Used by: 9 meters
mode-ac-power-w = Puissance AC W
# Used by: 9 meters
mode-ac-power-wh = Puissance AC Wh
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
mode-angle = Angle
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
mode-auto = Auto
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-auto-v = Auto V
# Used by: 47 meters
mode-capacitance = Capacité
# Used by: UT8803
mode-capacitance-d = Capacité D
# Used by: UT8803
mode-capacitance-r = Capacité R
# Used by: 8 meters
mode-clamp-ac-a = Pince AC A
# Used by: 8 meters
mode-clamp-ac-plus-dc-a = Pince AC+DC A
# Used by: 8 meters
mode-clamp-dc-a = Pince DC A
# Used by: 8 meters
mode-clamp-lpf-a = Pince LPF A
# Used by: 9 meters
mode-compass = Boussole
# Used by: 46 meters
mode-continuity = Continuité
# Used by: UT181A
mode-continuity-open = Continuité (circuit ouvert)
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
mode-dc-power-a = Puissance DC A
# Used by: 9 meters
mode-dc-power-ah = Puissance DC Ah
# Used by: 9 meters
mode-dc-power-v = Puissance DC V
# Used by: 9 meters
mode-dc-power-va = Puissance DC VA
# Used by: 9 meters
mode-dc-power-w = Puissance DC W
# Used by: 9 meters
mode-dc-power-wh = Puissance DC Wh
# Used by: 28 meters
mode-dc-ua = DC µA
# Used by: 121GW
mode-dc-uva = DC µVA
# Used by: 46 meters
mode-diode = Diode
# Used by: UT181A
mode-diode-alarm = Alarme diode
# Used by: 44 meters
mode-duty-pct = Rapport cyclique %
# Used by: BM829s/BM827s/BM822s/BM821s
mode-ef = EF
# Used by: BM788BT/BM787BT
mode-ef-h = EF-H
# Used by: BM788BT/BM787BT
mode-ef-l = EF-L
# Used by: 9 meters
mode-frequency = Fréquence
# Used by: 9 meters
mode-hv-ac-v = HV AC V
# Used by: 9 meters
mode-hv-dc-v = HV DC V
# Used by: UT8803
mode-inductance = Inductance
# Used by: UT8803
mode-inductance-q = Inductance Q
# Used by: UT8803
mode-inductance-r = Inductance R
# Used by: 22 meters
mode-inrush = Courant d’appel
# Used by: BM788BT/BM787BT
mode-line-hz = Hz secteur
# Used by: 8 meters
mode-live = Phase
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
mode-logic-hz = Hz logique
# Used by: 9 meters
mode-motor = Moteur
# Used by: 9 meters
mode-motor-1-2-3 = Moteur 1-2-3
# Used by: 9 meters
mode-motor-3-2-1 = Moteur 3-2-1
# Used by: 20 meters
mode-ncv = NCV
# Used by: UT71A, UT71B, UT71C/D/E, UT804, VC920/VC940/VC960
mode-power = Puissance
# Used by: 9 meters
mode-power-a = Puissance A
# Used by: 9 meters
mode-power-ah = Puissance Ah
# Used by: 9 meters
mode-power-v = Puissance V
# Used by: 9 meters
mode-power-va = Puissance VA
# Used by: 9 meters
mode-power-w = Puissance W
# Used by: 9 meters
mode-power-wh = Puissance Wh
# Used by: 9 meters
mode-power-factor = Facteur de puissance
# Used by: 121GW, UT181A
mode-pulse-width = Largeur d’impulsion
# Used by: BM525s/BM521s
mode-recall = Lecture mémoire
# Used by: UT8802, UT8803
mode-scr = SCR
# Used by: 9 meters
mode-scope = Oscilloscope
# Used by: 9 meters
mode-solar = Solaire
# Used by: UT171A/B/C
mode-square-wave = Signal carré
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
mode-temperature = Température
# Used by: 9 meters
mode-time = Temps
# Used by: 9 meters
mode-usb-power-a = Puissance USB A
# Used by: 9 meters
mode-usb-power-ah = Puissance USB Ah
# Used by: 9 meters
mode-usb-power-v = Puissance USB V
# Used by: 9 meters
mode-usb-power-va = Puissance USB VA
# Used by: 9 meters
mode-usb-power-w = Puissance USB W
# Used by: 9 meters
mode-usb-power-wh = Puissance USB Wh
# Used by: UT171A/B/C, UT181A
mode-v-ac = V AC
# Used by: UT181A
mode-v-ac-hz = V AC Hz
# Used by: UT181A
mode-v-ac-lpf = V AC LPF
# Used by: UT181A
mode-v-ac-peak = V AC crête
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
mode-v-dc-peak = V DC crête
# Used by: UT171A/B/C
mode-vfc = VFC
# Used by: BM788BT/BM787BT, BM869s/BM867s
mode-vfd-ac-v = VFD AC V
# Used by: BM788BT/BM787BT, BM869s/BM867s
mode-vfd-hz = VFD Hz
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-dbm-reference = Référence dBm
# Used by: 25 meters
mode-hfe = hFE
# Used by: UT171A/B/C, UT181A
mode-ma-ac = mA AC
# Used by: UT181A
mode-ma-ac-hz = mA AC Hz
# Used by: UT181A
mode-ma-ac-peak = mA AC crête
# Used by: UT171A/B/C
mode-ma-ac-plus-dc = mA AC+DC
# Used by: UT171A/B/C, UT181A
mode-ma-dc = mA DC
# Used by: UT181A
mode-ma-dc-ac-plus-dc = mA DC AC+DC
# Used by: UT181A
mode-ma-dc-peak = mA DC crête
# Used by: UT171A/B/C, UT181A
mode-mv-ac = mV AC
# Used by: UT181A
mode-mv-ac-ac-plus-dc = mV AC AC+DC
# Used by: UT181A
mode-mv-ac-hz = mV AC Hz
# Used by: UT181A
mode-mv-ac-peak = mV AC crête
# Used by: UT171A/B/C
mode-mv-ac-plus-dc = mV AC+DC
# Used by: UT171A/B/C, UT181A
mode-mv-dc = mV DC
# Used by: UT181A
mode-mv-dc-peak = mV DC crête
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
mode-ua-ac-peak = µA AC crête
# Used by: UT171A/B/C
mode-ua-ac-plus-dc = µA AC+DC
# Used by: UT171A/B/C, UT181A
mode-ua-dc = µA DC
# Used by: UT181A
mode-ua-dc-ac-plus-dc = µA DC AC+DC
# Used by: UT181A
mode-ua-dc-peak = µA DC crête

## Mode qualifiers: a word around a mode's name for a low-pass filter, a
## held peak or a relative reading, around any of the modes above. $mode is the
## mode, already translated: put it where your language puts it.

# Used by: 9 meters
# $mode is the mode, already translated.
qualifier-lpf-mode = LPF { $mode }
# Used by: 9 meters
# $mode is the mode, already translated.
qualifier-peak-mode = Crête { $mode }
# Used by: UT181A
# $mode is the mode, already translated.
qualifier-mode-rel = { $mode } REL
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
# $mode is the mode, already translated.
qualifier-mode-peak = { $mode } crête

## Ranges: beside the reading and in the range menu. Ranges that are a
## number and a unit only are not listed.

# Used by: UT161E, UT61E+
range-1000beta = 1000β
# Used by: 12 meters
range-auto = Auto
# Used by: 7 meters
range-cont = Cont
# Used by: 7 meters
range-diode = Diode
# Used by: 7 meters
range-duty = Rapport cyclique
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
aux-absolute = Absolu
# Used by: 9 meters
aux-angle = Angle
# Used by: UT171A/B/C
aux-aux = Aux
# Used by: UT181A
aux-average = Moyenne
# Used by: 121GW
aux-battery = Pile
# Used by: 121GW
aux-burden-voltage = Chute de tension
# Used by: UT181A
aux-comp-high = COMP haut
# Used by: UT181A
aux-comp-low = COMP bas
# Used by: 10 meters
aux-capacitance = Capacité
# Used by: UT202BT
aux-clamp-ac-a = Pince AC A
# Used by: UT202BT
aux-clamp-lpf-a = Pince LPF A
# Used by: 9 meters
aux-compass = Boussole
# Used by: 10 meters
aux-continuity = Continuité
# Used by: 121GW
aux-current = Courant
# Used by: 12 meters
aux-dc = DC
# Used by: 9 meters
aux-dc-a = DC A
# Used by: 10 meters
aux-dc-v = DC V
# Used by: 9 meters
aux-diode = Diode
# Used by: 10 meters
aux-duty = Rapport cyclique
# Used by: 9 meters
aux-frequency = Fréquence
# Used by: 9 meters
aux-hv-ac-v = HV AC V
# Used by: 9 meters
aux-hv-dc-v = HV DC V
# Used by: UT181A
aux-impedance = Impédance
# Used by: UT202BT
aux-inrush = Courant d’appel
# Used by: 121GW
aux-internal-temperature = Température interne
# Used by: UT202BT
aux-lpf-v = LPF V
# Used by: VC871
aux-live = Phase
# Used by: UT181A
aux-max = Max
# Used by: UT181A
aux-min = Min
# Used by: 9 meters
aux-motor = Moteur
# Used by: CMS061, CMS101
aux-ncv = NCV
# Used by: 9 meters
aux-pf = PF
# Used by: UT181A
aux-peak-min = Crête min.
# Used by: Mock UT61E+, UT181A
aux-period = Période
# Used by: UT171A/B/C
aux-pulse-width = Largeur d’impulsion
# Used by: UT181A, VC871
aux-reference = Référence
# Used by: 9 meters
aux-solar = Solaire
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, Mock UT61E+, UT181A
aux-t1 = T1
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s, Mock UT61E+, UT181A
aux-t2 = T2
# Used by: UT202BT
aux-temperature = Température
# Used by: 9 meters
aux-time = Temps
# Used by: 121GW, UT181A
aux-voltage = Tension
# Used by: 9 meters
aux-hfe = hFE

## On and off, for a meter setting offered as a choice.

# Used by: 13 meters
toggle-off = désactivé
# Used by: 13 meters
toggle-on = activé

## Meter keys named in words for what they do. A name printed on a key is
## not listed here: it stays as printed.

# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-auto-function = Fonction auto
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-capacitance = Capacité
# Used by: ZT-300AB, ZT-5566SE, ZT-5BQ
key-current = Courant
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-diode-continuity = Diode / continuité

## Meter key hovers: what a remote button does, or how to do it on the
## meter. Keep the meter's key names as it prints them.

# Used by: VC891, VC915
key-hover-1-khz-low-pass-filter = Filtre passe-bas 1 kHz
# Used by: 121GW
key-hover-capture-1-ms-peaks-in-ac = Capturer les crêtes de 1 ms en mode AC V
# Used by: OW67B, VC871
key-hover-change-what-the-displays-show = Changer le contenu des écrans
# Used by: VC915, VC925 PV
key-hover-compare-the-reading-with-the-set = Comparer la mesure aux limites définies
# Used by: CMS061, CMS101
key-hover-enter-max-min-click-again-to = Passer en mode MAX/MIN : cliquez de nouveau pour le quitter

## Meter key hovers: how to do on the meter what a remote button does.
## Keep the meter's key names as it prints them.

# Used by: 121GW
key-hover-hold-the-meter-s-rel-key = Maintenir la touche REL du multimètre enfoncée (filtre passe-bas 1 kHz)
# Used by: CMS061, CMS101
key-hover-measure-inrush-current-click-again-to = Mesurer le courant d’appel : cliquez de nouveau pour quitter ce mode
# Used by: UT171A/B/C
key-hover-peak-hold-in-ac-v-and = Maintien de crête en AC V et AC A : cliquez pour entrer dans ce mode ou en sortir
# Used by: OW18B/OW16B, OW18E
key-hover-press-the-meter-s-hz-duty = Appuyer brièvement sur la touche Hz/Duty△/ᛒ du multimètre
# Used by: CM2100B
key-hover-press-the-meter-s-zero-key = Appuyer brièvement sur la touche ZERO/ᛒ du multimètre
# Used by: OW18B/OW16B, OW18E
key-hover-rel-in-ac-and-hz-steps = Activer REL ou, en AC et en Hz, alterner entre Hz et rapport cyclique
# Used by: OW67B, OW69B, VC871, VC891, VC915, VC925 PV
key-hover-read-a-4-20-ma-loop = Afficher le courant de boucle 4-20 mA en pourcentage
# Used by: ZT-5566SE
key-hover-record-maximum-and-minimum-readings = Enregistrer les valeurs maximale et minimale
# Used by: UT171A/B/C
key-hover-record-maximum-average-and-minimum-readings-2 = Enregistrer les valeurs maximale, moyenne et minimale : cliquez pour passer à la suivante ou sur × pour quitter
# Used by: UT181A
key-hover-record-maximum-average-and-minimum-readings = Enregistrer les valeurs maximale, moyenne et minimale : cliquez sur × pour quitter
# Used by: VC-880, VC-890, VC650BT
key-hover-record-maximum-minimum-and-average-readings = Enregistrer les valeurs maximale, minimale et moyenne : cliquez pour passer à la suivante ou sur × pour quitter
# Used by: 121GW
key-hover-record-minimum-maximum-and-average-readings = Enregistrer les valeurs minimale, maximale et moyenne : cliquez pour passer à la suivante ou sur × pour quitter
# Used by: UT171A/B/C, UT181A
key-hover-save-the-reading-to-the-meter = Enregistrer la mesure dans la mémoire du multimètre
# Used by: CMS061, CMS101
key-hover-send-select-switches-v-dc-to = Envoyer Select pour passer de V,DC à V,AC
# Used by: VC915
key-hover-show-3-phase-rotation-direction = Afficher le sens de rotation triphasé
# Used by: VC915
key-hover-show-vac-or-vdc-on-the = Afficher VAC ou VDC sur l’écran secondaire
# Used by: 121GW
key-hover-step-through-hold-and-autohold = Parcourir les modes HOLD et AutoHOLD
# Used by: VC915
key-hover-swap-hz-pct-and-voltage = Intervertir Hz/% et la tension
# Used by: VC925 PV
key-hover-swap-the-main-and-second-displays = Intervertir les écrans principal et secondaire
# Used by: OW69B
key-hover-switch-the-low-pass-filter-on = Activer ou désactiver le filtre passe-bas
# Used by: UT171A/B/C
key-hover-take-the-reading-as-the-reference = Prendre la mesure actuelle comme référence : cliquez sur × pour quitter
# Used by: OW18B/OW16B, OW18E
key-hover-toggle-the-backlight-and-an-ow18 = Allumer ou éteindre le rétroéclairage et, sur un OW18, la lampe torche
# Used by: VC871, VC891, VC915, VC925 PV
key-hover-zero-the-reading-to-exit = Remettre la mesure à zéro : cliquez sur × pour quitter

## Model names with words in them: the simulated meters. Keep the part
## number as it is.

# Used by: Mock UT61E+
model-mock-ut61e-plus = UT61E+ simulé
# Used by: Mock ZT-5B
model-mock-zt-5b = ZT-5B simulé

## Activation steps: how to switch a meter's data transmission on, shown
## when a connection gets no data. Keep each step on its own line, and the
## meter's key names, menu items and display text as the meter prints them.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
activation-bm52x =
    1. Fixez le câble BU-86X sur le port optique PC-Comm à l’arrière du multimètre et branchez-le
    2. Allumez le multimètre en position tension
    Remarque : le multimètre s’éteint après environ 17 minutes d’inactivité sur un BM869s ou un BM867s, 30 minutes sur les autres. Maintenez la touche SELECT enfoncée en allumant le multimètre pour désactiver l’arrêt automatique.
# Used by: UT8802, UT8803
activation-ut8802 =
    1. Branchez le câble USB sur le multimètre
    2. Allumez le multimètre
# Used by: UT171A/B/C
activation-ut171 =
    1. Branchez le câble USB sur le multimètre
    2. Allumez le multimètre
    3. Allez dans SETUP -> Communication -> ON
# Used by: UT181A
activation-ut181a =
    1. Branchez le câble USB sur le multimètre
    2. Allumez le multimètre
    3. Allez dans SETUP -> Communication -> ON
    Remarque : ce réglage se réinitialise à chaque mise hors tension.
# Used by: UT71A, UT71B, UT71C/D/E, VC920/VC940/VC960
activation-ut71ab =
    1. Branchez le câble USB sur le multimètre
    2. Allumez le multimètre
    3. Maintenez la touche MAX MIN enfoncée pendant 1 s ou, sur un UT71A, appuyez sur SEND : l’écran affiche SEND
    Remarque : la touche EXIT désactive SEND.
# Used by: UT803
activation-ut803 =
    1. Branchez le câble USB sur le multimètre
    2. Allumez le multimètre
    3. Appuyez sur RS232 : l’écran affiche RS232
# Used by: UT804
activation-ut804 =
    1. Branchez le câble USB sur le multimètre
    2. Allumez le multimètre
    3. Appuyez sur SEND : l’écran affiche SEND
    Remarque : HOLD suspend l’envoi des données et EXIT désactive SEND.
# Used by: VC-880, VC-890, VC650BT
activation-vc650bt =
    1. Branchez le câble USB sur le multimètre
    2. Allumez le multimètre
    3. Appuyez sur le bouton PC du multimètre
# Used by: BM788BT/BM787BT
activation-bm78xbt =
    1. Déconnectez le multimètre de toute autre application
    2. Tournez le commutateur rotatif sur n’importe quelle fonction sauf Auto V/LoZ
    3. Maintenez la touche Δ enfoncée pendant au moins une seconde, jusqu’à ce que ((D)) s’affiche à l’écran
    Remarque : le multimètre s’éteint après 15 à 30 minutes d’inactivité. Maintenez la touche SELECT enfoncée en allumant le multimètre pour désactiver l’arrêt automatique (dSAPO s’affiche).
    Si le multimètre est introuvable ou si son mot de passe de connexion a été modifié, maintenez la touche Hz enfoncée en tournant le commutateur de la position OFF à la position capacité en moins de 0.6 s : le multimètre affiche « Org », et son mot de passe de connexion et son nom Bluetooth reprennent leurs réglages d’usine.
# Used by: OW69B
activation-ow69b =
    1. Déconnectez le multimètre de toute autre application
    2. Allumez le multimètre
    3. Maintenez la touche < enfoncée (BLE inscrit au-dessus) jusqu’à ce que l’icône Bluetooth s’affiche
# Used by: OW67B
activation-ow67b =
    1. Déconnectez le multimètre de toute autre application
    2. Allumez le multimètre
    3. Maintenez la touche < enfoncée (BLE inscrit au-dessus) jusqu’à ce que l’icône Bluetooth s’affiche
    Remarque : le Bluetooth se désactive après 5 minutes d’inactivité. Maintenez de nouveau la touche < enfoncée pour le réactiver.
# Used by: VC915, VC925 PV
activation-vc915 =
    1. Déconnectez le multimètre de toute autre application
    2. Allumez le multimètre
    3. Maintenez le bouton BLE enfoncé pendant environ 2 secondes, jusqu’à ce que le multimètre émette un bip sonore et que le symbole Bluetooth s’affiche
# Used by: VC871, VC891
activation-vc871 =
    1. Déconnectez le multimètre de toute autre application
    2. Allumez le multimètre
    3. Maintenez le bouton BLE enfoncé pendant environ 2 secondes, jusqu’à ce que le multimètre émette un bip sonore et que le symbole Bluetooth s’affiche en haut à gauche
    Remarque : le Bluetooth est désactivé à chaque mise en marche du multimètre. Maintenez alors de nouveau le bouton BLE enfoncé.
# Used by: OW18B/OW16B, OW18E
activation-ow18b =
    1. Déconnectez le multimètre de toute autre application
    2. Allumez le multimètre
    3. Maintenez la touche Hz/Duty△/ᛒ enfoncée jusqu’à ce que l’icône ᛒ s’affiche
    Remarque : après 10 minutes d’inactivité, le multimètre émet deux bips sonores et le Bluetooth se désactive. Maintenez de nouveau la touche Hz/Duty△/ᛒ enfoncée pour le réactiver.
# Used by: OW65B
activation-ow65b =
    1. Déconnectez le multimètre de toute autre application
    2. Allumez le multimètre
    3. Maintenez la touche SETUP enfoncée (BLE inscrit au-dessus) jusqu’à ce que l’icône Bluetooth s’affiche
    Remarque : maintenez de nouveau la touche SETUP enfoncée pour désactiver le Bluetooth.
# Used by: CMS061, CMS101
activation-cms061 =
    1. Déconnectez le multimètre de toute autre application
    2. Allumez le multimètre
    3. Maintenez la touche Tab⇌ enfoncée (BLE inscrit au-dessus) jusqu’à ce que l’icône Bluetooth s’affiche
    Remarque : le multimètre ne s’éteint pas de lui-même tant que le Bluetooth est activé.
# Used by: CM2100B
activation-cm2100b =
    1. Déconnectez le multimètre de toute autre application
    2. Allumez le multimètre
    3. Maintenez la touche ZERO/ᛒ enfoncée pendant environ 2 secondes, jusqu’à ce que l’icône ᛒ s’affiche en haut à gauche de l’écran
    Remarque : le Bluetooth se désactive après 5 minutes d’inactivité, ou quand vous maintenez de nouveau la touche ZERO/ᛒ enfoncée. Le multimètre s’éteint après environ 15 minutes d’inactivité. Maintenez la touche SELECT enfoncée en allumant le multimètre pour désactiver l’arrêt automatique jusqu’à la prochaine mise hors tension.
# Used by: B33
activation-b33 =
    1. Déconnectez le multimètre de toute autre application
    2. Allumez le multimètre
    3. Maintenez la touche △/ᛒ enfoncée jusqu’à ce que l’icône ᛒ s’affiche
    Remarque : après 10 minutes d’inactivité, le multimètre émet deux bips sonores et le Bluetooth se désactive. Le Bluetooth se désactive aussi à la fin d’un enregistrement hors ligne. Maintenez de nouveau la touche △/ᛒ enfoncée pour le réactiver.
# Used by: B35T+, B41T+
activation-b35t-plus =
    1. Déconnectez le multimètre de toute autre application
    2. Allumez le multimètre
    3. Maintenez la touche △/ᛒ enfoncée jusqu’à ce que l’icône ᛒ s’affiche
    Remarque : après 10 minutes d’inactivité, le multimètre émet deux bips sonores et le Bluetooth se désactive. Maintenez de nouveau la touche △/ᛒ enfoncée pour le réactiver.
# Used by: ZT-5B
activation-zt5b =
    1. Maintenez le bouton rouge enfoncé pendant 2 secondes pour allumer le multimètre
    2. Appuyez brièvement sur le bouton rouge : le symbole Bluetooth s’affiche
    Remarque : le multimètre s’éteint après 15 minutes d’inactivité. Appuyez sur NCV avant d’allumer le multimètre pour désactiver l’arrêt automatique.
# Used by: UT161B, UT161D, UT161E, UT61B+, UT61D+, UT61E+
activation-ut161b =
    1. Insérez le module USB dans le multimètre
    2. Allumez le multimètre
    3. Appuyez longuement sur le bouton USB/Hz
    4. L’icône S s’affiche à l’écran LCD
# Used by: 121GW
activation-121gw =
    1. Tournez le commutateur rotatif sur n’importe quelle fonction
    2. Maintenez la touche 1ms PEAK enfoncée jusqu’à ce que BT s’affiche à l’écran
    Remarque : le multimètre s’éteint après 30 minutes. Sélectionnez APO.oF dans SETUP pour désactiver l’arrêt automatique.
# Used by: ZT-300AB
activation-zt300ab =
    1. Allumez le multimètre
    2. Maintenez le bouton Hz% enfoncé pendant 2 secondes : le symbole Bluetooth s’affiche
    Remarque : le multimètre s’éteint après 15 minutes d’inactivité. Maintenez la touche SEL enfoncée en allumant le multimètre pour désactiver l’arrêt automatique.
# Used by: UT60BT
activation-ut60bt =
    1. Allumez le multimètre
    2. Appuyez longuement sur SEL jusqu’à ce que le symbole Bluetooth s’affiche
# Used by: ZT-5566SE
activation-zt5566se =
    1. Allumez le multimètre
    2. Appuyez sur POWER pour activer le Bluetooth : le symbole Bluetooth clignote
    Remarque : le manuel ne décrit le Bluetooth que pour le haut-parleur du multimètre. Les mesures transmises par ce biais ne sont pas vérifiées.
# Used by: ZT-5BQ
activation-zt5bq =
    1. Allumez le multimètre
    2. Appuyez simultanément sur Power et Hz : le symbole Bluetooth s’affiche
    Remarque : le multimètre s’éteint après 15 minutes d’inactivité. Maintenez la touche Hz/NCV enfoncée en allumant le multimètre pour désactiver l’arrêt automatique.
# Used by: UT202BT
activation-ut202bt =
    1. Allumez le multimètre
    2. Appuyez brièvement sur le bouton Bluetooth : le symbole Bluetooth clignote
    Remarque : le Bluetooth se désactive de lui-même après 5 minutes sans connexion.
# Used by: Mock UT61E+, Mock ZT-5B
activation-mock = Aucune configuration requise : cet appareil est simulé.
