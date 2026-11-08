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
mode-a-ac-peak = A AC crête
# Used by: UT171A/B/C
mode-a-ac-plus-dc = A AC+DC
# Used by: UT171A/B/C, UT181A
mode-a-dc = A DC
# Used by: UT181A
mode-a-dc-ac-plus-dc = A DC AC+DC
# Used by: UT181A
mode-a-dc-peak = A DC crête
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
# Used by: 9 meters
mode-angle = Angle
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
mode-auto = Auto
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-auto-v = Auto V
# Used by: 46 meters
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
# Used by: 45 meters
mode-continuity = Continuité
# Used by: UT181A
mode-continuity-open = Continuité (circuit ouvert)
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
# Used by: 27 meters
mode-dc-ua = DC µA
# Used by: 121GW
mode-dc-uva = DC µVA
# Used by: 45 meters
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
# Used by: 8 meters
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
# Used by: UT71A/B, UT71C/D/E, UT804, VC920/VC940/VC960
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
# Used by: UT71A/B, UT71C/D/E, UT803, UT804, VC920/VC940/VC960
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
key-hover-record-maximum-average-and-minimum-readings-2 = Enregistrer les valeurs maximale, moyenne et minimale : cliquez pour passer à la suivante, × pour quitter
# Used by: UT181A
key-hover-record-maximum-average-and-minimum-readings = Enregistrer les valeurs maximale, moyenne et minimale : × pour quitter
# Used by: VC-880, VC-890, VC650BT
key-hover-record-maximum-minimum-and-average-readings = Enregistrer les valeurs maximale, minimale et moyenne : cliquez pour passer à la suivante, × pour quitter
# Used by: 121GW
key-hover-record-minimum-maximum-and-average-readings = Enregistrer les valeurs minimale, maximale et moyenne : cliquez pour passer à la suivante, × pour quitter
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
key-hover-take-the-reading-as-the-reference = Prendre la mesure actuelle comme référence : × pour quitter
# Used by: OW18B/OW16B, OW18E
key-hover-toggle-the-backlight-and-an-ow18 = Allumer ou éteindre le rétroéclairage et, sur un OW18, la lampe torche
# Used by: VC871, VC891, VC915, VC925 PV
key-hover-zero-the-reading-to-exit = Remettre la mesure à zéro : × pour quitter

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
# Used by: UT71A/B, UT71C/D/E, VC920/VC940/VC960
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
    1. Déconnectez le multimètre de toute application mobile
    2. Tournez le commutateur rotatif sur n’importe quelle fonction sauf Auto V/LoZ
    3. Maintenez la touche Δ enfoncée pendant au moins une seconde, jusqu’à ce que ((D)) s’affiche à l’écran
    Remarque : le multimètre s’éteint après 15 à 30 minutes d’inactivité. Maintenez la touche SELECT enfoncée en allumant le multimètre pour désactiver l’arrêt automatique (dSAPO s’affiche).
    Si le multimètre est introuvable ou si son mot de passe de connexion a été modifié, maintenez la touche Hz enfoncée en tournant le commutateur de OFF à la position capacité en moins de 0.6 s : le multimètre affiche « Org », et son mot de passe de connexion et son nom Bluetooth reviennent aux réglages d’usine.
# Used by: OW69B
activation-ow69b =
    1. Déconnectez le multimètre de toute application mobile
    2. Allumez le multimètre
    3. Maintenez la touche < enfoncée (BLE inscrit au-dessus) jusqu’à ce que l’icône Bluetooth s’affiche
# Used by: OW67B
activation-ow67b =
    1. Déconnectez le multimètre de toute application mobile
    2. Allumez le multimètre
    3. Maintenez la touche < enfoncée (BLE inscrit au-dessus) jusqu’à ce que l’icône Bluetooth s’affiche
    Remarque : le Bluetooth se désactive après 5 minutes d’inactivité. Maintenez de nouveau la touche < enfoncée pour le réactiver.
# Used by: VC915, VC925 PV
activation-vc915 =
    1. Déconnectez le multimètre de toute application mobile
    2. Allumez le multimètre
    3. Maintenez le bouton BLE enfoncé pendant environ 2 secondes, jusqu’à ce que le multimètre émette un bip sonore et que le symbole Bluetooth s’affiche
# Used by: VC871, VC891
activation-vc871 =
    1. Déconnectez le multimètre de toute application mobile
    2. Allumez le multimètre
    3. Maintenez le bouton BLE enfoncé pendant environ 2 secondes, jusqu’à ce que le multimètre émette un bip sonore et que le symbole Bluetooth s’affiche en haut à gauche
    Remarque : le Bluetooth est désactivé à chaque mise en marche du multimètre. Maintenez alors de nouveau le bouton BLE enfoncé.
# Used by: OW18B/OW16B, OW18E
activation-ow18b =
    1. Déconnectez le multimètre de toute application mobile
    2. Allumez le multimètre
    3. Maintenez la touche Hz/Duty△/ᛒ enfoncée jusqu’à ce que l’icône ᛒ s’affiche
    Remarque : après 10 minutes d’inactivité, le multimètre émet deux bips sonores et le Bluetooth se désactive. Maintenez de nouveau la touche Hz/Duty△/ᛒ enfoncée pour le réactiver.
# Used by: OW65B
activation-ow65b =
    1. Déconnectez le multimètre de toute application mobile
    2. Allumez le multimètre
    3. Maintenez la touche SETUP enfoncée (BLE inscrit au-dessus) jusqu’à ce que l’icône Bluetooth s’affiche
    Remarque : maintenez de nouveau la touche SETUP enfoncée pour désactiver le Bluetooth.
# Used by: CMS061, CMS101
activation-cms061 =
    1. Déconnectez le multimètre de toute application mobile
    2. Allumez le multimètre
    3. Maintenez la touche Tab⇌ enfoncée (BLE inscrit au-dessus) jusqu’à ce que l’icône Bluetooth s’affiche
    Remarque : le multimètre ne s’éteint pas de lui-même tant que le Bluetooth est activé.
# Used by: CM2100B
activation-cm2100b =
    1. Déconnectez le multimètre de toute application mobile
    2. Allumez le multimètre
    3. Maintenez la touche ZERO/ᛒ enfoncée pendant environ 2 secondes, jusqu’à ce que l’icône ᛒ s’affiche en haut à gauche de l’écran
    Remarque : le Bluetooth se désactive après 5 minutes d’inactivité, ou quand vous maintenez de nouveau la touche ZERO/ᛒ enfoncée. Le multimètre s’éteint après environ 15 minutes d’inactivité. Maintenez la touche SELECT enfoncée en allumant le multimètre pour désactiver l’arrêt automatique jusqu’à la prochaine mise hors tension.
# Used by: B33
activation-b33 =
    1. Déconnectez le multimètre de toute application mobile
    2. Allumez le multimètre
    3. Maintenez la touche △/ᛒ enfoncée jusqu’à ce que l’icône ᛒ s’affiche
    Remarque : après 10 minutes d’inactivité, le multimètre émet deux bips sonores et le Bluetooth se désactive. Le Bluetooth se désactive aussi à la fin d’un enregistrement hors ligne. Maintenez de nouveau la touche △/ᛒ enfoncée pour le réactiver.
# Used by: B35T+, B41T+
activation-b35t-plus =
    1. Déconnectez le multimètre de toute application mobile
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
    Remarque : le multimètre s’éteint après 30 minutes. Réglez APO.oF dans SETUP pour désactiver l’arrêt automatique.
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

## Specification table names, in the Specifications panel, as the
## meter's manual titles them. Keep the manual's numbering: (1), A.

# Used by: UT181A
table-1-ac-voltage = (1) Tension alternative
# Used by: UT181A
table-10-temperature = (10) Température
# Used by: UT181A
table-11-frequency = (11) Fréquence
# Used by: UT181A
table-12-duty-cycle = (12) Rapport cyclique
# Used by: UT181A
table-13-pulse-width = (13) Largeur d’impulsion
# Used by: UT181A
table-14-continuity-test = (14) Test de continuité
# Used by: UT181A
table-15-diode-test = (15) Test de diode
# Used by: UT181A
table-2-dc-voltage = (2) Tension continue
# Used by: UT181A
table-3-ac-voltage-plus-dc-voltage = (3) Tension alternative + tension continue
# Used by: UT181A
table-4-ac-current = (4) Courant alternatif
# Used by: UT181A
table-5-dc-current = (5) Courant continu
# Used by: UT181A
table-6-ac-current-plus-dc-current = (6) Courant alternatif + courant continu
# Used by: UT181A
table-7-resistance = (7) Résistance
# Used by: UT181A
table-8-conductance = (8) Conductance
# Used by: UT181A
table-9-capacitance = (9) Capacité
# Used by: UT803, UT804
table-a-dc-voltage = A. Tension continue
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-ac-ac-plus-dc-current = Courant alternatif et alternatif + continu
# Used by: 7 meters
table-ac-current = Courant alternatif
# Used by: 9 meters
table-ac-voltage = Tension alternative
# Used by: Mock ZT-5B, ZT-5B
table-ac-voltage-v = Tension alternative (V)
# Used by: Mock ZT-5B, ZT-5B
table-ac-current-a = Courant alternatif (A)
# Used by: Mock ZT-5B, ZT-5B
table-ac-current-ma = Courant alternatif (mA)
# Used by: Mock UT61E+, UT161E, UT61E+
table-ac-plus-dc-voltage = Tension alternative + continue
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-audible-continuity-tester = Testeur de continuité acoustique
# Used by: Mock ZT-5B, ZT-5B
table-auto = Auto
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-acv = AutoCheck™ (ACV)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-dcv = AutoCheck™ (DCV)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-ohms = AutoCheck™ (Ohms)
# Used by: UT803
table-b-ac-voltage = B. Tension alternative
# Used by: UT804
table-b-ac-voltage-ac-plus-dc = B. Tension alternative (mesure alternative + continue disponible)
# Used by: UT803, UT804
table-c-dc-current = C. Courant continu
# Used by: 11 meters
table-capacitance = Capacité
# Used by: Mock ZT-5B, ZT-5B
table-continuity = Continuité
# Used by: 7 meters
table-continuity-and-diode = Continuité et diode
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-crest-mode-instantaneous-peak-hold = Mode crête (maintien de la crête instantanée)
# Used by: UT803
table-d-ac-current = D. Courant alternatif
# Used by: UT804
table-d-ac-current-ac-plus-dc = D. Courant alternatif (mesure alternative + continue disponible)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-dc-ac-ac-plus-dc-ac = Tension continue, alternative et alternative + continue
# Used by: 9 meters
table-dc-current = Courant continu
# Used by: Mock ZT-5B, ZT-5B
table-dc-voltage-v = Tension continue (V)
# Used by: 9 meters
table-dc-voltage = Tension continue
# Used by: Mock ZT-5B, ZT-5B
table-dc-current-a = Courant continu (A)
# Used by: Mock ZT-5B, ZT-5B
table-dc-current-ma = Courant continu (mA)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-diode-tester = Testeur de diode
# Used by: UT803, UT804
table-e-resistance = E. Résistance
# Used by: UT803, UT804
table-f-continuity-test = F. Test de continuité
# Used by: Mock ZT-5B, ZT-5B
table-frequency = Fréquence
# Used by: 7 meters
table-frequency-duty-ratio = Fréquence/rapport cyclique
# Used by: UT804
table-g-diode-test = G. Test de diode
# Used by: UT803
table-g-diode-testst = G. Test de diode
# Used by: UT803, UT804
table-h-capacitance = H. Capacité
# Used by: UT803, UT804
table-i-frequency = I. Fréquence
# Used by: UT804
table-j-duty-cycle = J. Rapport cyclique
# Used by: UT803
table-j-temperature = J. Température
# Used by: UT804
table-k-temperature-1-1-degrees-celsius = K. Température 1-1. Degrés Celsius
# Used by: UT804
table-k-temperature-1-2-fahrenheit = K. Température 1-2. Fahrenheit
# Used by: UT803
table-k-transistor = K. Transistor
# Used by: UT804
table-l-4-20-ma-loop-current = L. Courant de boucle 4~20 mA
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-line-level-frequency-hz = Fréquence de niveau secteur (~Hz)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-logic-level-frequency-hz-duty-cycle = Fréquence de niveau logique (⎍Hz) et rapport cyclique (D%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-non-contact-ef-detection = Détection EF sans contact
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-ohms = Résistance
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-record-mode = Mode enregistrement
# Used by: 9 meters
table-resistance = Résistance
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, Mock ZT-5B, UT161D, UT61D+, ZT-5B
table-temperature = Température
# Used by: Mock UT61E+, UT161E, UT61E+
table-transistor-magnification = Gain en courant du transistor (hFE)

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
band-in-rel-state = En mode REL
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); I. Frequency
band-not-specified = Non spécifié
# Used by: UT181A, UT804
# In: (9) Capacitance; H. Capacitance
band-not-specified-2 = Non spécifié
# Used by: UT181A
# In: (1) AC Voltage; (3) AC Voltage + DC Voltage
band-only-for-reference = À titre de référence uniquement
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage; DC AC & AC+DC AC Voltage
band-unspec-d = Non spécifié
# Used by: UT804
# In: A. DC Voltage
band-under-rel-mode = en mode REL

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
input-about-10mohm = Environ 10MΩ
# Used by: 7 meters
# In: DC Voltage
input-about-1gohm = Environ 1GΩ
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
input-approx-10mohm = Environ 10MΩ
# Used by: UT803, UT804
# In: A. DC Voltage; B. AC Voltage
input-around-10mohm = Environ 10MΩ
# Used by: UT804
# In: A. DC Voltage
input-around-2-5gohm = Environ 2.5GΩ
# Used by: UT803
# In: A. DC Voltage; B. AC Voltage
input-around-3000mohm = Environ > 3000MΩ
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
input-fuse-200ma-250v-fuse-500ma-125v = Fusible 200mA 250V ; fusible 500mA 125V
# Used by: UT803
# In: C. DC Current; D. AC Current
input-fuse-500ma-125v = Fusible 500mA 125V
# Used by: UT161B, UT161D, UT161E
# In: AC Current; DC Current
input-fuse-600ma-1000v = Fusible 600mA 1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
input-initially-approx-3-0kohm-150pf-nominal = Au départ environ 3.0kΩ, 150pF nominal
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
input-initially-approx-3-0kohm-165pf-nominal = Au départ environ 3.0kΩ, 165pF nominal

## Specification notes, under a table in the Specifications panel, from
## the meter's manual. Keep the exact sense, comparisons and qualifiers such as
## "typical" included. Keep every number, sign and ± exactly; a decimal comma
## is fine (0,6). A translation that changes, drops or adds a number is shown
## in English.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-10a-continuous-10a-to-20a-30s = 10A en continu ; >10A à 20A : 30s max., 5 min de refroidissement
# Used by: UT804
# In: C. DC Current; D. AC Current (AC+DC measurement is available)
note-10a-5a-continuous-5a-10a-10s = 10A : ≤5A en continu ; >5A–10A ≤10s à la fois, à >15min d’intervalle
# Used by: UT181A
# In: (13) Pulse Width
note-10hz-200khz-pulse-width-2us-depends = 10Hz–200kHz : largeur d’impulsion >2µs, selon la fréquence du signal
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
note-20a-30s-on-then-10min-off = 20A : 30s de mesure, puis 10min d’arrêt ; non spécifié au-delà de 10A
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-220mf-range-measurement-takes-20s = Plage 220mF : la mesure prend ≈20s
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-25-00mf-manual-range-not-specified = 25.00mF, plage manuelle : non spécifié en dessous de 5.4mF
# Used by: UT804
# In: E. Resistance
note-400ohm-accuracy-plus-test-lead-open = 400Ω : précision + valeur des cordons de mesure en circuit ouvert
# Used by: UT804
# In: H. Capacitance
note-40nf-accuracy-plus-open-circuit-test = 40nF : précision + capacité des cordons de mesure en circuit ouvert
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-0-pct-20ma-shows = À 4mA, l’écran affiche 0% ; à 20mA, 100%
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-6-000mf-manual-range-not-specified = 6.000mF, plage manuelle : non spécifié en dessous de 0.54mF
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms)
note-60-00mohm-tempco-0-6-accuracy = 60.00MΩ : coefficient de température 0.6× précision/°C @(0°C ~ 18°C ou 28°C ~ 45°C)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-60-00mv-600-0mv-1khz-20khz = 60.00mV, 600.0mV, 1kHz ~ 20kHz : spécifié de 30% à 100% de la plage
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage
note-60-00mv-600-0mv-3khz-20khz = 60.00mV, 600.0mV, 3kHz ~ 20kHz : spécifié de 30% à 100% de la plage
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-600-0uf-manual-range-not-specified = 600.0µF, plage manuelle : non spécifié en dessous de 50.0µF
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-600ohm-ref-typical-acmv-42-22 = Réf. 600Ω, typique : ACmV -42.22 à -02.22dBm, ACV -17.78 à 62.22dBm
# Used by: UT803
# In: E. Resistance
note-600ohm-accuracy-plus-test-lead-short = 600Ω : précision + résistance des cordons de mesure court-circuités
# Used by: UT181A
# In: (7) Resistance
note-60mohm-humidity-50-pct = 60MΩ : humidité <50%
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Capacitance
note-60mf-range-measurement-takes-20s = Plage 60mF : la mesure prend ≈20s
# Used by: UT181A
# In: (2) DC Voltage
note-60mv-use-rel-to-compensate-bias = 60mV : utilisez REL pour compenser la tension de décalage
# Used by: UT803
# In: H. Capacitance
note-6nf-60nf-600nf-subtract-open-circuit = 6nF, 60nF, 600nF : soustrayez la capacité des cordons de mesure en circuit ouvert
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-9-999v-1khz-20khz-specified-for = 9.999V, 1kHz ~ 20kHz : spécifié pour 1kHz ~ 15kHz uniquement
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage
note-9-999v-3khz-20khz-specified-for = 9.999V, 3kHz ~ 20kHz : spécifié pour 3kHz ~ 15kHz uniquement
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-lo-20ma-shows-hi = <4mA : l’écran affiche LO ; >20mA : HI
# Used by: 7 meters
# In: AC Current; DC Current
note-5a-max-10s-rest-15-min = >5A : 10s max., repos ≥15 min
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-10-00a-15-00-3 = AC 10.00A : 15.00 ~ 3.000kHz, sensibilité 6A (valeur efficace sinusoïdale)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-6-000a-15-00-3 = AC 6.000A : 15.00 ~ 3.000kHz, sensibilité 4A (valeur efficace sinusoïdale)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-60-00ma-15-00-3 = AC 60.00mA : 15.00 ~ 3.000kHz, sensibilité 40mA (valeur efficace sinusoïdale)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-60-00mv-15-00-50 = AC 60.00mV : 15.00 ~ 50.00kHz, sensibilité 40mV (valeur efficace sinusoïdale)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0ma-15-00-3 = AC 600.0mA : 15.00 ~ 3.000kHz, sensibilité 60mA (valeur efficace sinusoïdale)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0mv-15-00-50 = AC 600.0mV : 15.00 ~ 50.00kHz, sensibilité 60mV (valeur efficace sinusoïdale)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0ua-15-00-3 = AC 600.0µA : 15.00 ~ 3.000kHz, sensibilité 200µA (valeur efficace sinusoïdale)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-6000ua-15-00-3-000khz = AC 6000µA : 15.00 ~ 3.000kHz, sensibilité 600µA (valeur efficace sinusoïdale)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-9-999v-15-00-10 = AC 9.999V : 15.00 ~ 10.00kHz, sensibilité 2.5V (valeur efficace sinusoïdale)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-99-99v-15-00-10 = AC 99.99V : 15.00 ~ 10.00kHz, sensibilité 25V (valeur efficace sinusoïdale)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-999-9v-15-00-10 = AC 999.9V : 15.00 ~ 10.00kHz, sensibilité 100V (valeur efficace sinusoïdale)
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-ac-plus-dc-add-1-pct = AC+DC : ajoutez 1%+35 points à la précision
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-ac-plus-dc-range-accuracy-plus = AC+DC : précision de la plage + 1%
# Used by: UT804
# In: K. Temperature 1-1. Degrees Celsius; K. Temperature 1-2. Fahrenheit
note-above-230degc-use-the-rod-contact = Au-dessus de 230°C : utilisez la sonde de contact à tige
# Used by: UT803
# In: J. Temperature
note-above-230degc-use-the-rod-type = Au-dessus de 230°C : utilisez la sonde à tige
# Used by: UT181A
# In: (10) Temperature
note-accessory-point-contact-probe-below-230degc = Sonde à contact fournie en accessoire : en dessous de 230°C uniquement
# Used by: 7 meters
# In: 4 tables
note-accuracy-valid-1-pct-100-pct = Précision valable sur 1%–100% de la plage
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-accuracy-valid-1-pct-100-pct-2 = Précision valable sur 1%–100% de la plage (1kHz–10kHz : 10%–100%)
# Used by: UT161D, UT61D+
# In: AC Current
note-accuracy-valid-40hz-1khz-1-pct = Précision valable sur 40Hz–1kHz, 1%–100% de la plage (600.0µA : 5%–100%)
# Used by: UT161D, UT61D+
# In: AC Voltage
note-accuracy-valid-40hz-1khz-1-pct-2 = Précision valable sur 40Hz–1kHz, 1%–100% de la plage (60mV : 2%–100%)
# Used by: UT161B, UT61B+
# In: AC Current
note-accuracy-valid-40hz-500hz-1-pct = Précision valable sur 40Hz–500Hz, 1%–100% de la plage (600.0µA : 5%–100%)
# Used by: UT161B, UT61B+
# In: AC Voltage
note-accuracy-valid-40hz-500hz-1-pct-2 = Précision valable sur 40Hz–500Hz, 1%–100% de la plage (60mV : 2%–100%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-accuracy-with-film-capacitor-or-better = Précision donnée avec un condensateur à film ou de meilleure qualité
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-audible-threshold-between-20ohm-and-300ohm = Seuil sonore : entre 20Ω et 300Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-bm821s-specified-from-10-pct-to = BM821s : spécifié de 10% à 100% de la plage
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-bar-segments-beep-tones-proportional-to = Segments du bargraphe et bips sonores proportionnels à l’intensité du champ
# Used by: Mock ZT-5B, ZT-5B
# In: Continuity
note-beeps-50ohm = Bip sonore <50Ω
# Used by: UT803
# In: F. Continuity Test
note-beeps-continuously-10ohm-silent-30ohm = Bip continu ≤10Ω ; silencieux >30Ω
# Used by: UT804
# In: F. Continuity Test
note-beeps-continuously-10ohm-silent-50ohm = Bip continu ≤10Ω ; silencieux >50Ω
# Used by: UT181A
# In: (15) Diode Test
note-brief-beep-on-a-normal-junction = Bip bref sur une jonction normale, continu en cas de court-circuit
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-0-02v-a = Chute de tension 0.02V/A
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-0-08mv-ua = Chute de tension 0.08mV/µA
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-2-1mv-ma = Chute de tension 2.1mV/mA
# Used by: 7 meters
# In: Continuity and Diode
note-continuity-audio-visual-alarm-50ohm-no = Continuité : alarme sonore et visuelle <50Ω ; pas de bip ≥70Ω
# Used by: UT803
# In: B. AC Voltage
note-crest-factor-3-0-1000v-1 = Facteur de crête 3.0 (1000V : 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage; DC AC & AC+DC AC Voltage
note-crest-factor-2-1-at-full = Facteur de crête < 2:1 à pleine échelle, < 4:1 à mi-échelle
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-crest-factor-up-to-3-0 = Facteur de crête jusqu’à 3.0 (1000V : 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Crest mode (Instantaneous Peak Hold)
note-crest-specified-accuracy-adds-250-digits = Crête : précision spécifiée + 250 points pour les variations > 1.0 ms
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-detects-50-60hz-antenna-at-the = Détecte 50/60Hz ; antenne à l’extrémité supérieure du multimètre
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
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-1v-20v-rms = Entrée Hz >1MHz : 1V–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-40mhz-1v-20v = Entrée Hz >1MHz–40MHz : 1V–20V rms ; >40MHz non spécifié
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-200mv-20v-rms = Entrée Hz ≤100kHz : 200mV–20V rms
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
# Used by: UT803
# In: I. Frequency
note-input-10hz-1mhz-150mv-30v-rms = Entrée 10Hz–1MHz : 150mV–30V rms
# Used by: UT181A
# In: (11) Frequency
note-input-10hz-30mhz-600mv-30v-rms = Entrée 10Hz–30MHz : 600mV–30V rms
# Used by: UT804
# In: I. Frequency; J. Duty Cycle
note-input-10hz-40mhz-200mv-30v-rms = Entrée 10Hz–40MHz : 200mV–30V rms
# Used by: UT803
# In: I. Frequency
note-input-10mhz-50mhz-600mv-30v-rms = Entrée >10MHz–50MHz : 600mV–30V rms
# Used by: UT803
# In: I. Frequency
note-input-1mhz-10mhz-300mv-30v-rms = Entrée >1MHz–10MHz : 300mV–30V rms
# Used by: UT181A
# In: (11) Frequency
note-input-30mhz-not-specified = Entrée >30MHz : non spécifiée
# Used by: UT804
# In: I. Frequency; J. Duty Cycle
note-input-at-zero-dc-level-40mhz = Entrée à niveau DC nul ; >40MHz non spécifié
# Used by: UT803
# In: I. Frequency
note-input-at-zero-dc-level-50mhz = Entrée à niveau DC nul ; >50MHz non spécifié
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
# Used by: Mock ZT-5B, ZT-5B
# In: Frequency
note-max-10mhz = Max. 10MHz
# Used by: Mock ZT-5B, ZT-5B
# In: Resistance
note-max-40mohm = Max. 40MΩ
# Used by: Mock ZT-5B, ZT-5B
# In: Capacitance
note-max-4mf = Max. 4mF
# Used by: Mock ZT-5B, ZT-5B
# In: AC Voltage (V); DC VOLTAGE (V)
note-max-600v = Max. 600V
# Used by: Mock ZT-5B, ZT-5B
# In: 4 tables
note-max-9-999a = Max. 9.999A
# Used by: UT161D, UT61D+
# In: Temperature
note-measured-temperature-should-stay-below-230degc = La température mesurée doit rester inférieure à 230°C/446°F
# Used by: 7 meters
# In: AC Current; AC Voltage
note-non-sine-add-4-pct-crest = Non sinusoïdal : ajoutez 4% (facteur de crête 1–2), 5% (2–2.5), 7% (2.5–3)
# Used by: UT181A
# In: (15) Diode Test
note-normal-silicon-junction-0-5v-0 = Jonction silicium normale : 0.5V–0.8V
# Used by: UT181A
# In: (14) Continuity Test
note-open-alarm-beeps-50ohm-silent-10ohm = Alarme de circuit ouvert : bip >50Ω, silencieux <10Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms); Ohms
note-open-circuit-1-2vdc-1-0vdc = Circuit ouvert < 1.2VDC (< 1.0VDC sur 60.00MΩ)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-open-circuit-3-5vdc = Circuit ouvert < 3.5VDC
# Used by: UT803
# In: F. Continuity Test
note-open-circuit-1-2v = Circuit ouvert ≈-1.2V
# Used by: UT804
# In: F. Continuity Test
note-open-circuit-1-2v-2 = Circuit ouvert ≈1.2V
# Used by: 7 meters
# In: Resistance
note-open-circuit-1v = Circuit ouvert ≈1V
# Used by: UT803
# In: G. Diode Testst
note-open-circuit-2-7v = Circuit ouvert ≈2.7V
# Used by: UT804
# In: G. Diode Test
note-open-circuit-2-8v = Circuit ouvert ≈2.8V
# Used by: UT181A
# In: (14) Continuity Test
note-open-circuit-3v = Circuit ouvert ≈3V
# Used by: UT181A
# In: (15) Diode Test
note-open-circuit-3v-forward-drop-up = Circuit ouvert ≈3V ; tension directe jusqu’à ≈3V
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; DC Current
note-open-circuit-residual-10-digits = Circuit ouvert : résidu ≤10 points
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; DC Current
note-open-circuit-residual-5-digits = Circuit ouvert : résidu ≤5 points
# Used by: Mock ZT-5B, ZT-5B
# In: AC current (A); DC current (A)
note-over-2a-measure-for-less-than = Au-delà de 2A : mesurez pendant moins de 3s
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
note-red-lead-in-a-ma-picks = Cordon rouge sur la borne A mA : sélectionne le courant
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-vohm-picks-v = Cordon rouge sur la borne VΩ : sélectionne V au-dessus de 0.8V, sinon Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-response-time-100us = Temps de réponse < 100µs
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Capacitance
note-result-reading-open-lead-capacitance-1uf = Résultat = mesure − capacité des cordons en circuit ouvert ; ≤1µF : utilisez REL
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-result-reading-open-lead-capacitance-22nf = Résultat = mesure − capacité des cordons en circuit ouvert ; ≤22nF : utilisez REL
# Used by: 7 meters
# In: Resistance
note-result-reading-shorted-lead-resistance = Résultat = mesure − résistance des cordons court-circuités
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-sel-ncv-continuity-diode-cap-hz = SEL/NCV : continuité/diode, capacité, Hz, °C
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
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Voltage
note-shorted-leads-residual-10-digits = Cordons court-circuités : résidu ≤10 points
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Voltage
note-shorted-leads-residual-3-digits = Cordons court-circuités : résidu ≤3 points
# Used by: 7 meters
# In: DC Voltage
note-shorted-leads-residual-5-digits = Cordons court-circuités : résidu ≤5 points
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-test-current-0-4ma-typical = Courant de test 0.4mA typique
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
# Used by: UT804
# In: D. AC Current (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10-2 = True RMS et précision valables sur 10%–100% de la plage ; facteur de crête jusqu’à 3.0
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage
note-true-rms-from-10-pct-to = True RMS de 10% à 100% de la plage ; BM821s : mesure de la valeur moyenne
# Used by: UT181A
# In: 4 tables
note-true-rms-valid-10-pct-100 = True RMS valable sur 10%–100% de la plage
# Used by: UT803
# In: B. AC Voltage
note-true-rms-valid-10-pct-95 = True RMS valable sur 10%–95% de la plage
# Used by: UT803
# In: D. AC Current
note-true-rms-valid-10-pct-95-2 = True RMS valable sur 10%–95% de la plage ; facteur de crête 3.0
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-true-rms-accuracy-valid-10-pct = True RMS ; précision valable sur 10%–100% de la plage
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-2-0 = True RMS ; facteur de crête ≤2.0 à 10000, ≤1 à 22000 points
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-3-0 = True RMS ; facteur de crête ≤3.0 à 3000, ≤1.5 à 6000 points
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Temperature
note-type-k-thermocouple-range-accuracy-not = Plage et précision du thermocouple de type K non incluses
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-typical-20v-55v-110v-220v-440v = Typique : - 20V, -- 55V, --- 110V, ---- 220V, ----- 440V
# Used by: UT804
# In: J. Duty Cycle
note-valid-10-pct-90-pct-of = Valable sur 10%–90% de la plage, 5Hz–2kHz uniquement
# Used by: UT803
# In: K. Transistor
note-vce-2-2v-bo-10ua = Vce ≈2.2V, bo ≈10µA
# Used by: UT181A
# In: (12) Duty Cycle; (13) Pulse Width
note-when-rise-time-1us-signals-center = Quand le temps de montée est <1µs, les signaux sont centrés sur le niveau de déclenchement
# Used by: UT803
# In: G. Diode Testst
note-working-current-1ma = Courant de fonctionnement ≈1mA
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current
note-ua-ranges-min-30ua = Plages µA : min. 30µA
# Used by: UT803
# In: C. DC Current; D. AC Current
note-5a-range-continuous-5a-range-10s = Plage ≤5A : en continu ; plage >5A : ≤10s à la fois, à ≥15min d’intervalle
