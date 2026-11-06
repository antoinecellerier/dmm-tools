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
mode-a-ac-peak = A AC Peak
# Used by: UT171A/B/C
mode-a-ac-plus-dc = A AC+DC
# Used by: UT171A/B/C, UT181A
mode-a-dc = A DC
# Used by: UT181A
mode-a-dc-ac-plus-dc = A DC AC+DC
# Used by: UT181A
mode-a-dc-peak = A DC Peak
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
# Used by: 7 meters
mode-ac-power-a = AC power A
# Used by: 7 meters
mode-ac-power-ah = AC power Ah
# Used by: 7 meters
mode-ac-power-v = AC power V
# Used by: 7 meters
mode-ac-power-va = AC power VA
# Used by: 7 meters
mode-ac-power-w = AC power W
# Used by: 7 meters
mode-ac-power-wh = AC power Wh
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
mode-angle = Angle
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
mode-auto = Auto
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-auto-v = Auto V
# Used by: 46 meters
mode-capacitance = Capacitance
# Used by: UT8803
mode-capacitance-d = Capacitance D
# Used by: UT8803
mode-capacitance-r = Capacitance R
# Used by: 8 meters
mode-clamp-ac-a = Clamp AC A
# Used by: 8 meters
mode-clamp-ac-plus-dc-a = Clamp AC+DC A
# Used by: 8 meters
mode-clamp-dc-a = Clamp DC A
# Used by: 8 meters
mode-clamp-lpf-a = Clamp LPF A
# Used by: 9 meters
mode-compass = Compass
# Used by: 45 meters
mode-continuity = Continuity
# Used by: UT181A
mode-continuity-open = Continuity (open)
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
# Used by: 7 meters
mode-dc-power-a = DC power A
# Used by: 7 meters
mode-dc-power-ah = DC power Ah
# Used by: 7 meters
mode-dc-power-v = DC power V
# Used by: 7 meters
mode-dc-power-va = DC power VA
# Used by: 7 meters
mode-dc-power-w = DC power W
# Used by: 7 meters
mode-dc-power-wh = DC power Wh
# Used by: 27 meters
mode-dc-ua = DC µA
# Used by: 121GW
mode-dc-uva = DC µVA
# Used by: 45 meters
mode-diode = Diode
# Used by: UT181A
mode-diode-alarm = Diode Alarm
# Used by: 44 meters
mode-duty-pct = Duty %
# Used by: BM829s/BM827s/BM822s/BM821s
mode-ef = EF
# Used by: BM788BT/BM787BT
mode-ef-h = EF-H
# Used by: BM788BT/BM787BT
mode-ef-l = EF-L
# Used by: 8 meters
mode-frequency = Frequency
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
mode-inrush = Inrush
# Used by: BM788BT/BM787BT
mode-line-hz = Line Hz
# Used by: 8 meters
mode-live = Live
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
mode-logic-hz = Logic Hz
# Used by: 9 meters
mode-motor = Motor
# Used by: 9 meters
mode-motor-1-2-3 = Motor 1-2-3
# Used by: 9 meters
mode-motor-3-2-1 = Motor 3-2-1
# Used by: 20 meters
mode-ncv = NCV
# Used by: UT71A/B, UT71C/D/E, UT804, VC920/VC940/VC960
mode-power = Power
# Used by: 9 meters
mode-power-a = Power A
# Used by: 9 meters
mode-power-ah = Power Ah
# Used by: 9 meters
mode-power-v = Power V
# Used by: 7 meters
mode-power-va = Power VA
# Used by: 7 meters
mode-power-w = Power W
# Used by: 9 meters
mode-power-wh = Power Wh
# Used by: 9 meters
mode-power-factor = Power factor
# Used by: 121GW, UT181A
mode-pulse-width = Pulse Width
# Used by: BM525s/BM521s
mode-recall = Recall
# Used by: UT8802, UT8803
mode-scr = SCR
# Used by: 9 meters
mode-scope = Scope
# Used by: 9 meters
mode-solar = Solar
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1 = T1
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1-t2 = T1-T2
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t2 = T2
# Used by: UT803
mode-tachometer = Tachometer
# Used by: Mock UT61E+
mode-temp-degc = Temp °C
# Used by: Mock UT61E+
mode-temp-degc-t1-t2 = Temp °C T1 (T2)
# Used by: Mock UT61E+
mode-temp-degc-t1-t2-2 = Temp °C T1-T2
# Used by: Mock UT61E+
mode-temp-degc-t2-t1 = Temp °C T2-T1
# Used by: UT71A/B, UT71C/D/E, UT803, UT804, VC920/VC940/VC960
mode-temperature = Temperature
# Used by: 9 meters
mode-time = Time
# Used by: 7 meters
mode-usb-power-a = USB power A
# Used by: 7 meters
mode-usb-power-ah = USB power Ah
# Used by: 7 meters
mode-usb-power-v = USB power V
# Used by: 7 meters
mode-usb-power-va = USB power VA
# Used by: 7 meters
mode-usb-power-w = USB power W
# Used by: 7 meters
mode-usb-power-wh = USB power Wh
# Used by: UT171A/B/C, UT181A
mode-v-ac = V AC
# Used by: UT181A
mode-v-ac-hz = V AC Hz
# Used by: UT181A
mode-v-ac-lpf = V AC LPF
# Used by: UT181A
mode-v-ac-peak = V AC Peak
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
mode-v-dc-peak = V DC Peak
# Used by: UT171A/B/C
mode-vfc = VFC
# Used by: BM788BT/BM787BT, BM869s/BM867s
mode-vfd-ac-v = VFD AC V
# Used by: BM788BT/BM787BT, BM869s/BM867s
mode-vfd-hz = VFD Hz
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-dbm-reference = dBm reference
# Used by: 25 meters
mode-hfe = hFE
# Used by: UT171A/B/C, UT181A
mode-ma-ac = mA AC
# Used by: UT181A
mode-ma-ac-hz = mA AC Hz
# Used by: UT181A
mode-ma-ac-peak = mA AC Peak
# Used by: UT171A/B/C
mode-ma-ac-plus-dc = mA AC+DC
# Used by: UT171A/B/C, UT181A
mode-ma-dc = mA DC
# Used by: UT181A
mode-ma-dc-ac-plus-dc = mA DC AC+DC
# Used by: UT181A
mode-ma-dc-peak = mA DC Peak
# Used by: UT171A/B/C, UT181A
mode-mv-ac = mV AC
# Used by: UT181A
mode-mv-ac-ac-plus-dc = mV AC AC+DC
# Used by: UT181A
mode-mv-ac-hz = mV AC Hz
# Used by: UT181A
mode-mv-ac-peak = mV AC Peak
# Used by: UT171A/B/C
mode-mv-ac-plus-dc = mV AC+DC
# Used by: UT171A/B/C, UT181A
mode-mv-dc = mV DC
# Used by: UT181A
mode-mv-dc-peak = mV DC Peak
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
mode-ua-ac-peak = µA AC Peak
# Used by: UT171A/B/C
mode-ua-ac-plus-dc = µA AC+DC
# Used by: UT171A/B/C, UT181A
mode-ua-dc = µA DC
# Used by: UT181A
mode-ua-dc-ac-plus-dc = µA DC AC+DC
# Used by: UT181A
mode-ua-dc-peak = µA DC Peak

## Mode qualifiers: a word around a mode's name for a low-pass filter, a
## held peak or a relative reading, around any of the modes above. $mode is the
## mode, already translated: put it where your language puts it.

# Used by: 9 meters
# $mode is the mode, already translated.
qualifier-lpf-mode = LPF { $mode }
# Used by: 9 meters
# $mode is the mode, already translated.
qualifier-peak-mode = Peak { $mode }
# Used by: UT181A
# $mode is the mode, already translated.
qualifier-mode-rel = { $mode } REL
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
# $mode is the mode, already translated.
qualifier-mode-peak = { $mode } peak

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
aux-absolute = Absolute
# Used by: 9 meters
aux-angle = Angle
# Used by: UT171A/B/C
aux-aux = Aux
# Used by: UT181A
aux-average = Average
# Used by: 121GW
aux-battery = Battery
# Used by: 121GW
aux-burden-voltage = Burden voltage
# Used by: UT181A
aux-comp-high = COMP High
# Used by: UT181A
aux-comp-low = COMP Low
# Used by: 10 meters
aux-capacitance = Capacitance
# Used by: UT202BT
aux-clamp-ac-a = Clamp AC A
# Used by: UT202BT
aux-clamp-lpf-a = Clamp LPF A
# Used by: 9 meters
aux-compass = Compass
# Used by: 10 meters
aux-continuity = Continuity
# Used by: 121GW
aux-current = Current
# Used by: 12 meters
aux-dc = DC
# Used by: 9 meters
aux-dc-a = DC A
# Used by: 10 meters
aux-dc-v = DC V
# Used by: 9 meters
aux-diode = Diode
# Used by: 10 meters
aux-duty = Duty
# Used by: 9 meters
aux-frequency = Frequency
# Used by: 9 meters
aux-hv-ac-v = HV AC V
# Used by: 9 meters
aux-hv-dc-v = HV DC V
# Used by: UT181A
aux-impedance = Impedance
# Used by: UT202BT
aux-inrush = Inrush
# Used by: 121GW
aux-internal-temperature = Internal temperature
# Used by: UT202BT
aux-lpf-v = LPF V
# Used by: VC871
aux-live = Live
# Used by: CMS061, CMS101, UT181A
aux-max = Max
# Used by: UT181A
aux-min = Min
# Used by: 9 meters
aux-motor = Motor
# Used by: CMS061, CMS101
aux-ncv = NCV
# Used by: 9 meters
aux-pf = PF
# Used by: UT181A
aux-peak-min = Peak Min
# Used by: Mock UT61E+, UT181A
aux-period = Period
# Used by: UT181A, VC871
aux-reference = Reference
# Used by: 9 meters
aux-solar = Solar
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, Mock UT61E+, UT181A
aux-t1 = T1
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s, Mock UT61E+, UT181A
aux-t2 = T2
# Used by: UT202BT
aux-temperature = Temperature
# Used by: 9 meters
aux-time = Time
# Used by: 121GW, UT181A
aux-voltage = Voltage
# Used by: 9 meters
aux-hfe = hFE

## On and off, for a meter setting offered as a choice.

# Used by: 13 meters
toggle-off = off
# Used by: 13 meters
toggle-on = on

## Meter keys named in words for what they do. A name printed on a key is
## not listed here: it stays as printed.

# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-auto-function = Auto function
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-capacitance = Capacitance
# Used by: ZT-300AB, ZT-5566SE, ZT-5BQ
key-current = Current
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-diode-continuity = Diode / continuity

## Meter key hovers: what a remote button does, or how to do it on the
## meter. Keep the meter's key names as it prints them.

# Used by: 121GW
key-hover-capture-1-ms-peaks-in-ac = Capture 1 ms peaks in AC V
# Used by: CMS061, CMS101
key-hover-enter-max-min-click-again-to = Enter MAX/MIN; click again to leave it
# Used by: 121GW
key-hover-hold-the-meter-s-rel-key = Hold the meter's REL key (1 kHz low-pass filter)
# Used by: CMS061, CMS101
key-hover-measure-inrush-current-click-again-to = Measure inrush current; click again to leave it
# Used by: OW18B/OW16B, OW18E
key-hover-press-the-meter-s-hz-duty = Press the meter's Hz/Duty△/ᛒ key briefly
# Used by: CM2100B
key-hover-press-the-meter-s-zero-key = Press the meter's ZERO/ᛒ key briefly
# Used by: OW18B/OW16B, OW18E
key-hover-rel-in-ac-and-hz-steps = REL; in AC and Hz, steps Hz/Duty
# Used by: OW67B, OW69B, VC871, VC891, VC915, VC925 PV
key-hover-read-a-4-20-ma-loop = Read a 4-20 mA loop as a percentage
# Used by: ZT-5566SE
key-hover-record-maximum-and-minimum-readings = Record maximum and minimum readings
# Used by: UT181A
key-hover-record-maximum-average-and-minimum-readings = Record maximum, average and minimum readings — × to exit
# Used by: VC-880, VC-890, VC650BT
key-hover-record-maximum-minimum-and-average-readings = Record maximum, minimum, and average readings — click to cycle, × to exit
# Used by: 121GW
key-hover-record-minimum-maximum-and-average-readings = Record minimum, maximum, and average readings — click to cycle, × to exit
# Used by: UT181A
key-hover-save-the-reading-to-the-meter = Save the reading to the meter's memory
# Used by: CMS061, CMS101
key-hover-send-select-effect-on-this-meter = Send Select; effect on this meter unknown
# Used by: 121GW
key-hover-step-through-hold-and-autohold = Step through HOLD and AutoHOLD
# Used by: OW18B/OW16B, OW18E
key-hover-toggle-the-backlight-and-an-ow18 = Toggle the backlight and an OW18's flashlight
# Used by: VC871, VC891, VC915, VC925 PV
key-hover-zero-the-reading-to-exit = Zero the reading — × to exit

## Model names with words in them: the simulated meters. Keep the part
## number as it is.

# Used by: Mock UT61E+
model-mock-ut61e-plus = Mock UT61E+
# Used by: Mock ZT-5B
model-mock-zt-5b = Mock ZT-5B

## Activation steps: how to switch a meter's data transmission on, shown
## when a connection gets no data. Keep each step on its own line, and the
## meter's key names, menu items and display text as the meter prints them.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
activation-bm52x =
    1. Attach the BU-86X cable to the optical PC-Comm port at the back of the meter and plug it in
    2. Turn the meter on at a voltage function
    Note: the meter switches off after about 17 minutes idle on a BM869s or BM867s, 30 minutes on the others; hold SELECT while turning it on to disable that.
# Used by: UT8802, UT8803
activation-ut8802 =
    1. Connect the USB cable to the meter
    2. Turn the meter on
# Used by: UT171A/B/C
activation-ut171 =
    1. Connect the USB cable to the meter
    2. Turn the meter on
    3. Go to SETUP -> Communication -> ON
# Used by: UT181A
activation-ut181a =
    1. Connect the USB cable to the meter
    2. Turn the meter on
    3. Go to SETUP -> Communication -> ON
    Note: this setting resets on power cycle.
# Used by: UT71A/B, UT71C/D/E, VC920/VC940/VC960
activation-ut71ab =
    1. Connect the USB cable to the meter
    2. Turn the meter on
    3. Hold MAX MIN for 1 s, or press SEND on a UT71A; the display shows SEND
    Note: EXIT turns SEND off.
# Used by: UT803
activation-ut803 =
    1. Connect the USB cable to the meter
    2. Turn the meter on
    3. Press RS232; the display shows RS232
# Used by: UT804
activation-ut804 =
    1. Connect the USB cable to the meter
    2. Turn the meter on
    3. Press SEND; the display shows SEND
    Note: HOLD pauses the data, and EXIT turns SEND off.
# Used by: VC-880, VC-890, VC650BT
activation-vc650bt =
    1. Connect the USB cable to the meter
    2. Turn the meter on
    3. Press the PC button on the meter
# Used by: BM788BT/BM787BT
activation-bm78xbt =
    1. Disconnect the meter from any phone app
    2. Turn the dial to any function but Auto V/LoZ
    3. Hold Δ for one second or more, until ((D)) shows on the display
    Note: the meter switches off after 15 to 30 minutes idle; hold SELECT while turning it on to disable that (dSAPO shows).
    If the meter is not found or its connection password was changed, hold the Hz button while turning the dial from OFF to capacitance within 0.6 s: the meter shows "Org" and its connection password and Bluetooth name are back to the factory settings.
# Used by: OW69B
activation-ow69b =
    1. Disconnect the meter from any phone app
    2. Turn the meter on
    3. Hold the < key (BLE printed above it) until the Bluetooth icon shows
# Used by: OW67B
activation-ow67b =
    1. Disconnect the meter from any phone app
    2. Turn the meter on
    3. Hold the < key (BLE printed above it) until the Bluetooth icon shows
    Note: Bluetooth switches off after 5 minutes idle; hold < again to switch it back on.
# Used by: VC915, VC925 PV
activation-vc915 =
    1. Disconnect the meter from any phone app
    2. Turn the meter on
    3. Hold the BLE button for about 2 seconds, until the meter beeps and the Bluetooth symbol shows
# Used by: VC871, VC891
activation-vc871 =
    1. Disconnect the meter from any phone app
    2. Turn the meter on
    3. Hold the BLE button for about 2 seconds, until the meter beeps and the Bluetooth symbol shows at the top left
    Note: Bluetooth is off each time the meter is switched on; hold BLE again then.
# Used by: OW18B/OW16B, OW18E
activation-ow18b =
    1. Disconnect the meter from any phone app
    2. Turn the meter on
    3. Hold the Hz/Duty△/ᛒ key until the ᛒ icon shows
    Note: Bluetooth switches off after 10 minutes idle, after two beeps; hold Hz/Duty△/ᛒ again to switch it back on.
# Used by: OW65B
activation-ow65b =
    1. Disconnect the meter from any phone app
    2. Turn the meter on
    3. Hold the SETUP key (BLE printed above it) until the Bluetooth icon shows
    Note: holding SETUP again switches Bluetooth off.
# Used by: CMS061, CMS101
activation-cms061 =
    1. Disconnect the meter from any phone app
    2. Turn the meter on
    3. Hold the Tab⇌ key (BLE printed above it) until the Bluetooth icon shows
    Note: the meter does not switch itself off while Bluetooth is on.
# Used by: CM2100B
activation-cm2100b =
    1. Disconnect the meter from any phone app
    2. Turn the meter on
    3. Hold the ZERO/ᛒ key for about 2 seconds, until the ᛒ icon shows at the top left of the display
    Note: Bluetooth switches off after 5 minutes idle, or when ZERO/ᛒ is held again. The meter switches off after about 15 minutes idle; hold SELECT while turning it on to disable that until it is next switched off.
# Used by: B33
activation-b33 =
    1. Disconnect the meter from any phone app
    2. Turn the meter on
    3. Hold the △/ᛒ key until the ᛒ icon shows
    Note: Bluetooth switches off after 10 minutes idle, after two beeps, and when an offline recording ends; hold △/ᛒ again to switch it back on.
# Used by: B35T+, B41T+
activation-b35t-plus =
    1. Disconnect the meter from any phone app
    2. Turn the meter on
    3. Hold the △/ᛒ key until the ᛒ icon shows
    Note: Bluetooth switches off after 10 minutes idle, after two beeps; hold △/ᛒ again to switch it back on.
# Used by: ZT-5B
activation-zt5b =
    1. Hold the red button for 2 seconds to turn the meter on
    2. Short press the red button; the Bluetooth symbol shows
    Note: the meter switches off after 15 minutes idle; press NCV before turning it on to disable that.
# Used by: UT161B, UT161D, UT161E, UT61B+, UT61D+, UT61E+
activation-ut161b =
    1. Insert the USB module into the meter
    2. Turn the meter on
    3. Long press the USB/Hz button
    4. The S icon appears on the LCD
# Used by: 121GW
activation-121gw =
    1. Turn the dial to any function
    2. Hold 1ms PEAK until BT shows on the display
    Note: the meter switches off after 30 minutes; set APO.oF in SETUP to disable that.
# Used by: ZT-300AB
activation-zt300ab =
    1. Turn the meter on
    2. Hold the Hz% button for 2 seconds; the Bluetooth symbol shows
    Note: the meter switches off after 15 minutes idle; hold SEL while turning it on to disable that.
# Used by: UT60BT
activation-ut60bt =
    1. Turn the meter on
    2. Long press SEL until the Bluetooth symbol shows
# Used by: ZT-5566SE
activation-zt5566se =
    1. Turn the meter on
    2. Press POWER to turn Bluetooth on; the Bluetooth symbol flashes
    Note: the manual documents Bluetooth for its speaker only; readings over it are unverified.
# Used by: ZT-5BQ
activation-zt5bq =
    1. Turn the meter on
    2. Press Power and Hz together; the Bluetooth symbol shows
    Note: the meter switches off after 15 minutes idle; hold Hz/NCV while turning it on to disable that.
# Used by: UT202BT
activation-ut202bt =
    1. Turn the meter on
    2. Short press the Bluetooth button; the Bluetooth symbol flashes
    Note: Bluetooth turns itself off after 5 minutes without a connection.
# Used by: Mock UT61E+, Mock ZT-5B
activation-mock = No setup required — this is a simulated device.

## Specification table names, in the Specifications panel, as the
## meter's manual titles them. Keep the manual's numbering: (1), A.

# Used by: UT181A
table-1-ac-voltage = (1) AC Voltage
# Used by: UT181A
table-10-temperature = (10) Temperature
# Used by: UT181A
table-11-frequency = (11) Frequency
# Used by: UT181A
table-12-duty-cycle = (12) Duty Cycle
# Used by: UT181A
table-13-pulse-width = (13) Pulse Width
# Used by: UT181A
table-14-continuity-test = (14) Continuity Test
# Used by: UT181A
table-15-diode-test = (15) Diode Test
# Used by: UT181A
table-2-dc-voltage = (2) DC Voltage
# Used by: UT181A
table-3-ac-voltage-plus-dc-voltage = (3) AC Voltage + DC Voltage
# Used by: UT181A
table-4-ac-current = (4) AC Current
# Used by: UT181A
table-5-dc-current = (5) DC Current
# Used by: UT181A
table-6-ac-current-plus-dc-current = (6) AC Current + DC Current
# Used by: UT181A
table-7-resistance = (7) Resistance
# Used by: UT181A
table-8-conductance = (8) Conductance
# Used by: UT181A
table-9-capacitance = (9) Capacitance
# Used by: UT803, UT804
table-a-dc-voltage = A. DC Voltage
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-ac-ac-plus-dc-current = AC & AC+DC Current
# Used by: 7 meters
table-ac-current = AC Current
# Used by: 9 meters
table-ac-voltage = AC Voltage
# Used by: Mock ZT-5B, ZT-5B
table-ac-voltage-v = AC Voltage (V)
# Used by: Mock ZT-5B, ZT-5B
table-ac-current-a = AC current (A)
# Used by: Mock ZT-5B, ZT-5B
table-ac-current-ma = AC current (mA)
# Used by: Mock UT61E+, UT161E, UT61E+
table-ac-plus-dc-voltage = AC+DC Voltage
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-audible-continuity-tester = Audible Continuity Tester
# Used by: Mock ZT-5B, ZT-5B
table-auto = Auto
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-acv = AutoCheck™ (ACV)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-dcv = AutoCheck™ (DCV)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-ohms = AutoCheck™ (Ohms)
# Used by: UT803
table-b-ac-voltage = B. AC Voltage
# Used by: UT804
table-b-ac-voltage-ac-plus-dc = B. AC Voltage (AC+DC measurement is available)
# Used by: UT803, UT804
table-c-dc-current = C. DC Current
# Used by: 11 meters
table-capacitance = Capacitance
# Used by: Mock ZT-5B, ZT-5B
table-continuity = Continuity
# Used by: 7 meters
table-continuity-and-diode = Continuity and Diode
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-crest-mode-instantaneous-peak-hold = Crest mode (Instantaneous Peak Hold)
# Used by: UT803
table-d-ac-current = D. AC Current
# Used by: UT804
table-d-ac-current-ac-plus-dc = D. AC Current (AC+DC measurement is available)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-dc-ac-ac-plus-dc-ac = DC AC & AC+DC AC Voltage
# Used by: 9 meters
table-dc-current = DC Current
# Used by: Mock ZT-5B, ZT-5B
table-dc-voltage-v = DC VOLTAGE (V)
# Used by: 9 meters
table-dc-voltage = DC Voltage
# Used by: Mock ZT-5B, ZT-5B
table-dc-current-a = DC current (A)
# Used by: Mock ZT-5B, ZT-5B
table-dc-current-ma = DC current (mA)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-diode-tester = Diode Tester
# Used by: UT803, UT804
table-e-resistance = E. Resistance
# Used by: UT803, UT804
table-f-continuity-test = F. Continuity Test
# Used by: Mock ZT-5B, ZT-5B
table-frequency = Frequency
# Used by: 7 meters
table-frequency-duty-ratio = Frequency/Duty Ratio
# Used by: UT804
table-g-diode-test = G. Diode Test
# Used by: UT803
table-g-diode-testst = G. Diode Testst
# Used by: UT803, UT804
table-h-capacitance = H. Capacitance
# Used by: UT803, UT804
table-i-frequency = I. Frequency
# Used by: UT804
table-j-duty-cycle = J. Duty Cycle
# Used by: UT803
table-j-temperature = J. Temperature
# Used by: UT804
table-k-temperature-1-1-degrees-celsius = K. Temperature 1-1. Degrees Celsius
# Used by: UT804
table-k-temperature-1-2-fahrenheit = K. Temperature 1-2. Fahrenheit
# Used by: UT803
table-k-transistor = K. Transistor
# Used by: UT804
table-l-4-20-ma-loop-current = L. 4~20 mA loop current
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-line-level-frequency-hz = Line Level Frequency (~Hz)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-logic-level-frequency-hz-duty-cycle = Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-non-contact-ef-detection = Non-Contact EF-Detection
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-ohms = Ohms
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-record-mode = Record mode
# Used by: 9 meters
table-resistance = Resistance
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, Mock ZT-5B, UT161D, UT61D+, ZT-5B
table-temperature = Temperature
# Used by: Mock UT61E+, UT161E, UT61E+
table-transistor-magnification = Transistor Magnification

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
band-in-rel-state = In REL state
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); I. Frequency
band-not-specified = Not Specified
# Used by: UT181A, UT804
# In: (9) Capacitance; H. Capacitance
band-not-specified-2 = Not specified
# Used by: UT181A
# In: (1) AC Voltage; (3) AC Voltage + DC Voltage
band-only-for-reference = Only for reference
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage; DC AC & AC+DC AC Voltage
band-unspec-d = Unspec’d
# Used by: UT804
# In: A. DC Voltage
band-under-rel-mode = under REL mode

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
input-about-10mohm = About 10MΩ
# Used by: 7 meters
# In: DC Voltage
input-about-1gohm = About 1GΩ
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
input-approx-10mohm = Approx 10MΩ
# Used by: UT803, UT804
# In: A. DC Voltage; B. AC Voltage
input-around-10mohm = Around 10MΩ
# Used by: UT804
# In: A. DC Voltage
input-around-2-5gohm = Around 2.5GΩ
# Used by: UT803
# In: A. DC Voltage; B. AC Voltage
input-around-3000mohm = Around > 3000MΩ
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current; Line Level Frequency (~Hz)
input-fuse-0-44a-1000v = Fuse 0.44A 1000V
# Used by: UT804
# In: C. DC Current; D. AC Current (AC+DC measurement is available); L. 4~20 mA loop current
input-fuse-0-5a-250v = Fuse 0.5A 250V
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
input-fuse-0-8a-1000v = Fuse 0.8A 1000V
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
input-fuse-10a-1000v = Fuse 10A 1000V
# Used by: Mock UT61E+, UT61B+, UT61D+, UT61E+
# In: AC Current; DC Current
input-fuse-10a-240v = Fuse 10A 240V
# Used by: UT803, UT804
# In: C. DC Current; D. AC Current; D. AC Current (AC+DC measurement is available)
input-fuse-10a-250v = Fuse 10A 250V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, UT161B, UT161D, UT161E
# In: 4 tables
input-fuse-11a-1000v = Fuse 11A 1000V
# Used by: Mock UT61E+, UT61B+, UT61D+, UT61E+
# In: AC Current; DC Current
input-fuse-1a-240v = Fuse 1A 240V
# Used by: UT803
# In: K. Transistor
input-fuse-200ma-250v-fuse-500ma-125v = Fuse 200mA 250V; Fuse 500mA 125V
# Used by: UT803
# In: C. DC Current; D. AC Current
input-fuse-500ma-125v = Fuse 500mA 125V
# Used by: UT161B, UT161D, UT161E
# In: AC Current; DC Current
input-fuse-600ma-1000v = Fuse 600mA 1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
input-initially-approx-3-0kohm-150pf-nominal = Initially approx. 3.0kΩ, 150pF nominal
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
input-initially-approx-3-0kohm-165pf-nominal = Initially approx. 3.0kΩ, 165pF nominal

## Specification notes, under a table in the Specifications panel, from
## the meter's manual. Keep the exact sense, comparisons and qualifiers such as
## "typical" included. Keep every number, sign and ± exactly; a decimal comma
## is fine (0,6). A translation that changes, drops or adds a number is shown
## in English.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-10a-continuous-10a-to-20a-30s = 10A continuous; >10A to 20A: 30s max, 5 min cool-down
# Used by: UT804
# In: C. DC Current; D. AC Current (AC+DC measurement is available)
note-10a-5a-continuous-5a-10a-10s = 10A: ≤5A continuous; >5A–10A ≤10s at a time, >15min apart
# Used by: UT181A
# In: (13) Pulse Width
note-10hz-200khz-pulse-width-2us-depends = 10Hz–200kHz: pulse width >2µs, depends on signal frequency
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
note-20a-30s-on-then-10min-off = 20A: 30s on, then 10min off; not specified above 10A
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-220mf-range-measurement-takes-20s = 220mF range: measurement takes ≈20s
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-25-00mf-manual-range-not-specified = 25.00mF, manual range: not specified below 5.4mF
# Used by: UT804
# In: E. Resistance
note-400ohm-accuracy-plus-test-lead-open = 400Ω: accuracy + test-lead open-circuit value
# Used by: UT804
# In: H. Capacitance
note-40nf-accuracy-plus-open-circuit-test = 40nF: accuracy + open-circuit test-lead capacitance
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-0-pct-20ma-shows = 4mA shows 0%, 20mA shows 100%
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-6-000mf-manual-range-not-specified = 6.000mF, manual range: not specified below 0.54mF
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms)
note-60-00mohm-tempco-0-6-accuracy = 60.00MΩ: tempco 0.6× accuracy/°C @(0°C ~ 18°C or 28°C ~ 45°C)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-60-00mv-600-0mv-1khz-20khz = 60.00mV, 600.0mV, 1kHz ~ 20kHz: specified 30% to 100% of range
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage
note-60-00mv-600-0mv-3khz-20khz = 60.00mV, 600.0mV, 3kHz ~ 20kHz: specified 30% to 100% of range
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-600-0uf-manual-range-not-specified = 600.0µF, manual range: not specified below 50.0µF
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-600ohm-ref-typical-acmv-42-22 = 600Ω ref, typical: ACmV -42.22 to -02.22dBm, ACV -17.78 to 62.22dBm
# Used by: UT803
# In: E. Resistance
note-600ohm-accuracy-plus-test-lead-short = 600Ω: accuracy + test-lead short-circuit resistance
# Used by: UT181A
# In: (7) Resistance
note-60mohm-humidity-50-pct = 60MΩ: humidity <50%
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Capacitance
note-60mf-range-measurement-takes-20s = 60mF range: measurement takes ≈20s
# Used by: UT181A
# In: (2) DC Voltage
note-60mv-use-rel-to-compensate-bias = 60mV: use REL to compensate bias voltage
# Used by: UT803
# In: H. Capacitance
note-6nf-60nf-600nf-subtract-open-circuit = 6nF, 60nF, 600nF: subtract open-circuit test-lead capacitance
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-9-999v-1khz-20khz-specified-for = 9.999V, 1kHz ~ 20kHz: specified for 1kHz ~ 15kHz only
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage
note-9-999v-3khz-20khz-specified-for = 9.999V, 3kHz ~ 20kHz: specified for 3kHz ~ 15kHz only
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-lo-20ma-shows-hi = <4mA shows LO, >20mA shows HI
# Used by: 7 meters
# In: AC Current; DC Current
note-5a-max-10s-rest-15-min = >5A: max 10s, rest ≥15 min
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-10-00a-15-00-3 = AC 10.00A: 15.00 ~ 3.000kHz, sensitivity 6A (sine RMS)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-6-000a-15-00-3 = AC 6.000A: 15.00 ~ 3.000kHz, sensitivity 4A (sine RMS)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-60-00ma-15-00-3 = AC 60.00mA: 15.00 ~ 3.000kHz, sensitivity 40mA (sine RMS)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-60-00mv-15-00-50 = AC 60.00mV: 15.00 ~ 50.00kHz, sensitivity 40mV (sine RMS)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0ma-15-00-3 = AC 600.0mA: 15.00 ~ 3.000kHz, sensitivity 60mA (sine RMS)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0mv-15-00-50 = AC 600.0mV: 15.00 ~ 50.00kHz, sensitivity 60mV (sine RMS)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0ua-15-00-3 = AC 600.0µA: 15.00 ~ 3.000kHz, sensitivity 200µA (sine RMS)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-6000ua-15-00-3-000khz = AC 6000µA: 15.00 ~ 3.000kHz, sensitivity 600µA (sine RMS)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-9-999v-15-00-10 = AC 9.999V: 15.00 ~ 10.00kHz, sensitivity 2.5V (sine RMS)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-99-99v-15-00-10 = AC 99.99V: 15.00 ~ 10.00kHz, sensitivity 25V (sine RMS)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-999-9v-15-00-10 = AC 999.9V: 15.00 ~ 10.00kHz, sensitivity 100V (sine RMS)
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-ac-plus-dc-add-1-pct = AC+DC: add 1%+35 digits to accuracy
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-ac-plus-dc-range-accuracy-plus = AC+DC: range accuracy + 1%
# Used by: UT804
# In: K. Temperature 1-1. Degrees Celsius; K. Temperature 1-2. Fahrenheit
note-above-230degc-use-the-rod-contact = Above 230°C: use the rod contact probe
# Used by: UT803
# In: J. Temperature
note-above-230degc-use-the-rod-type = Above 230°C: use the rod-type probe
# Used by: UT181A
# In: (10) Temperature
note-accessory-point-contact-probe-below-230degc = Accessory point-contact probe: below 230°C only
# Used by: 7 meters
# In: 4 tables
note-accuracy-valid-1-pct-100-pct = Accuracy valid 1%–100% of range
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-accuracy-valid-1-pct-100-pct-2 = Accuracy valid 1%–100% of range (1kHz–10kHz: 10%–100%)
# Used by: UT161D, UT61D+
# In: AC Current
note-accuracy-valid-40hz-1khz-1-pct = Accuracy valid 40Hz–1kHz, 1%–100% of range (600.0µA: 5%–100%)
# Used by: UT161D, UT61D+
# In: AC Voltage
note-accuracy-valid-40hz-1khz-1-pct-2 = Accuracy valid 40Hz–1kHz, 1%–100% of range (60mV: 2%–100%)
# Used by: UT161B, UT61B+
# In: AC Current
note-accuracy-valid-40hz-500hz-1-pct = Accuracy valid 40Hz–500Hz, 1%–100% of range (600.0µA: 5%–100%)
# Used by: UT161B, UT61B+
# In: AC Voltage
note-accuracy-valid-40hz-500hz-1-pct-2 = Accuracy valid 40Hz–500Hz, 1%–100% of range (60mV: 2%–100%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-accuracy-with-film-capacitor-or-better = Accuracy with film capacitor or better
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-audible-threshold-between-20ohm-and-300ohm = Audible threshold: between 20Ω and 300Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-bm821s-specified-from-10-pct-to = BM821s: specified from 10% to 100% of range
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-bar-segments-beep-tones-proportional-to = Bar segments & beep tones proportional to field strength
# Used by: Mock ZT-5B, ZT-5B
# In: Continuity
note-beeps-50ohm = Beeps <50Ω
# Used by: UT803
# In: F. Continuity Test
note-beeps-continuously-10ohm-silent-30ohm = Beeps continuously ≤10Ω; silent >30Ω
# Used by: UT804
# In: F. Continuity Test
note-beeps-continuously-10ohm-silent-50ohm = Beeps continuously ≤10Ω; silent >50Ω
# Used by: UT181A
# In: (15) Diode Test
note-brief-beep-on-a-normal-junction = Brief beep on a normal junction, continuous if shorted
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-0-02v-a = Burden voltage 0.02V/A
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-0-08mv-ua = Burden voltage 0.08mV/µA
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-2-1mv-ma = Burden voltage 2.1mV/mA
# Used by: 7 meters
# In: Continuity and Diode
note-continuity-audio-visual-alarm-50ohm-no = Continuity: audio/visual alarm <50Ω; no beep ≥70Ω
# Used by: UT803
# In: B. AC Voltage
note-crest-factor-3-0-1000v-1 = Crest factor 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage; DC AC & AC+DC AC Voltage
note-crest-factor-2-1-at-full = Crest factor < 2:1 at full scale, < 4:1 at half scale
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-crest-factor-up-to-3-0 = Crest factor up to 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Crest mode (Instantaneous Peak Hold)
note-crest-specified-accuracy-adds-250-digits = Crest: specified accuracy adds 250 digits for changes > 1.0 ms
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-detects-50-60hz-antenna-at-the = Detects 50/60Hz; antenna at the top end of the meter
# Used by: 7 meters
# In: Continuity and Diode
note-diode-one-beep-0-12v-2v = Diode: one beep 0.12V–2V (normal), long beep <0.12V (short)
# Used by: 7 meters
# In: Continuity and Diode
note-diode-open-circuit-3v = Diode: open circuit ≈3V
# Used by: UT181A
# In: (9) Capacitance
note-display-digits-6000 = Display digits: 6000
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-duty-cycle-specified-for-5hz-10khz = Duty cycle specified for 5Hz ~ 10kHz
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-duty-square-waves-only-1vpp-20vpp = Duty: square waves only, 1Vpp–20Vpp, ≤10kHz, 10.0%–90.0%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-for-ac-voltage-shorted-leads-residual = For AC voltage, shorted leads: residual ≤200 digits
# Used by: Mock ZT-5B, ZT-5B
# In: AC Voltage (V); AC current (A); AC current (mA)
note-frequency-response-40hz-1khz = Frequency response 40Hz–1kHz
# Used by: UT804
# In: G. Diode Test
note-good-silicon-junction-0-5v-0 = Good silicon junction: 0.5V–0.8V
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-hold-sel-ncv-for-ncv = Hold SEL/NCV for NCV
# Used by: UT181A
# In: (8) Conductance
note-humidity-50-pct = Humidity <50%
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-1mhz-600mv-20v = Hz input >100kHz–1MHz: 600mV–20V rms
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-1v-20v-rms = Hz input >1MHz: 1V–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-40mhz-1v-20v = Hz input >1MHz–40MHz: 1V–20V rms; >40MHz not specified
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-200mv-20v-rms = Hz input ≤100kHz: 200mV–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Transistor Magnification
note-ib0-1-8ua-vce-2-5v = Ib0 ≈1.8µA, Vce ≈2.5V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV); AutoCheck™ (DCV)
note-impedance-rises-within-a-fraction-of = Impedance rises within a fraction of a second above 50V (typ.)
# Used by: UT804
# In: K. Temperature 1-1. Degrees Celsius; K. Temperature 1-2. Fahrenheit
note-included-k-type-point-contact-probe = Included K-type point-contact probe: below 230°C only
# Used by: UT803
# In: J. Temperature
note-included-point-contact-probe-below-230degc = Included point-contact probe: below 230°C only
# Used by: UT803
# In: I. Frequency
note-input-10hz-1mhz-150mv-30v-rms = Input 10Hz–1MHz: 150mV–30V rms
# Used by: UT181A
# In: (11) Frequency
note-input-10hz-30mhz-600mv-30v-rms = Input 10Hz–30MHz: 600mV–30V rms
# Used by: UT804
# In: I. Frequency; J. Duty Cycle
note-input-10hz-40mhz-200mv-30v-rms = Input 10Hz–40MHz: 200mV–30V rms
# Used by: UT803
# In: I. Frequency
note-input-10mhz-50mhz-600mv-30v-rms = Input >10MHz–50MHz: 600mV–30V rms
# Used by: UT803
# In: I. Frequency
note-input-1mhz-10mhz-300mv-30v-rms = Input >1MHz–10MHz: 300mV–30V rms
# Used by: UT181A
# In: (11) Frequency
note-input-30mhz-not-specified = Input >30MHz: not specified
# Used by: UT804
# In: I. Frequency; J. Duty Cycle
note-input-at-zero-dc-level-40mhz = Input at zero DC level; >40MHz not specified
# Used by: UT803
# In: I. Frequency
note-input-at-zero-dc-level-50mhz = Input at zero DC level; >50MHz unspecified
# Used by: UT161D, UT61D+
# In: Temperature
note-k-type-thermocouple-only = K-type thermocouple only
# Used by: UT181A
# In: (10) Temperature
note-k-type-thermocouple-two-channels-via = K-type thermocouple; two channels via temperature connectors
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-lo-z-threshold-plus-1-5vdc = Lo-Z threshold: > +1.5VDC or < -1.0VDC nominal
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-lo-z-threshold-3vac-50-60hz = Lo-Z threshold: > 3VAC (50/60Hz) nominal
# Used by: UT803
# In: K. Transistor
note-max-1000beta = Max 1000β
# Used by: Mock ZT-5B, ZT-5B
# In: Frequency
note-max-10mhz = Max 10MHz
# Used by: Mock ZT-5B, ZT-5B
# In: Resistance
note-max-40mohm = Max 40MΩ
# Used by: Mock ZT-5B, ZT-5B
# In: Capacitance
note-max-4mf = Max 4mF
# Used by: Mock ZT-5B, ZT-5B
# In: AC Voltage (V); DC VOLTAGE (V)
note-max-600v = Max 600V
# Used by: Mock ZT-5B, ZT-5B
# In: 4 tables
note-max-9-999a = Max 9.999A
# Used by: UT161D, UT61D+
# In: Temperature
note-measured-temperature-should-stay-below-230degc = Measured temperature should stay below 230°C/446°F
# Used by: 7 meters
# In: AC Current; AC Voltage
note-non-sine-add-4-pct-crest = Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)
# Used by: UT181A
# In: (15) Diode Test
note-normal-silicon-junction-0-5v-0 = Normal silicon junction: 0.5V–0.8V
# Used by: UT181A
# In: (14) Continuity Test
note-open-alarm-beeps-50ohm-silent-10ohm = Open alarm: beeps >50Ω, silent <10Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms); Ohms
note-open-circuit-1-2vdc-1-0vdc = Open circuit < 1.2VDC (< 1.0VDC on 60.00MΩ)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-open-circuit-3-5vdc = Open circuit < 3.5VDC
# Used by: UT803
# In: F. Continuity Test
note-open-circuit-1-2v = Open circuit ≈-1.2V
# Used by: UT804
# In: F. Continuity Test
note-open-circuit-1-2v-2 = Open circuit ≈1.2V
# Used by: 7 meters
# In: Resistance
note-open-circuit-1v = Open circuit ≈1V
# Used by: UT803
# In: G. Diode Testst
note-open-circuit-2-7v = Open circuit ≈2.7V
# Used by: UT804
# In: G. Diode Test
note-open-circuit-2-8v = Open circuit ≈2.8V
# Used by: UT181A
# In: (14) Continuity Test
note-open-circuit-3v = Open circuit ≈3V
# Used by: UT181A
# In: (15) Diode Test
note-open-circuit-3v-forward-drop-up = Open circuit ≈3V; forward drop up to ≈3V
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; DC Current
note-open-circuit-residual-10-digits = Open circuit: residual ≤10 digits
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; DC Current
note-open-circuit-residual-5-digits = Open circuit: residual ≤5 digits
# Used by: Mock ZT-5B, ZT-5B
# In: AC current (A); DC current (A)
note-over-2a-measure-for-less-than = Over 2A: measure for less than 3s
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-range-accuracy-depend-on-acmv-acv = Range & accuracy depend on ACmV/ACV and reference impedance
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-ranges-2-2uf-add-10-digits = Ranges ≤2.2µF: add 10 digits when accuracy is ≤3%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Transistor Magnification
note-reading-for-reference-only = Reading for reference only
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Record mode
note-record-specified-accuracy-adds-10-digits = Record: specified accuracy adds 10 digits for changes > 100 ms
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-a-ma-picks = Red lead in A mA: picks current
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-vohm-picks-v = Red lead in VΩ: picks V above 0.8V or Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-response-time-100us = Response time < 100µs
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Capacitance
note-result-reading-open-lead-capacitance-1uf = Result = reading − open-lead capacitance; ≤1µF: use REL
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-result-reading-open-lead-capacitance-22nf = Result = reading − open-lead capacitance; ≤22nF: use REL
# Used by: 7 meters
# In: Resistance
note-result-reading-shorted-lead-resistance = Result = reading − shorted-lead resistance
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-sel-ncv-continuity-diode-cap-hz = SEL/NCV: continuity/diode, cap, Hz, °C
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-sensitivity-2-5vp-square-wave-for = Sensitivity: 2.5Vp square wave, for 3V & 5V logic family
# Used by: UT181A
# In: (14) Continuity Test
note-short-alarm-beeps-10ohm-silent-50ohm = Short alarm: beeps <10Ω, silent >50Ω
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-shorted-input-residual-under-30-digits = Shorted input: residual under ~30 digits, accuracy unaffected
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-shorted-leads-80-digit-residual-accuracy = Shorted leads: 80-digit residual, accuracy unaffected
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Voltage
note-shorted-leads-residual-10-digits = Shorted leads: residual ≤10 digits
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Voltage
note-shorted-leads-residual-3-digits = Shorted leads: residual ≤3 digits
# Used by: 7 meters
# In: DC Voltage
note-shorted-leads-residual-5-digits = Shorted leads: residual ≤5 digits
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-test-current-0-4ma-typical = Test current 0.4mA typical
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-then-typically-18kohm-100v-125kohm-300v = Then typically 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 460kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-then-typically-18kohm-100v-125kohm-300v-2 = Then typically 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 500kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-tolerance-10v-36v-23v-83v-59v = Tolerance: 10V~36V, 23V~83V, 59V~165V, 124V~330V, > 250V
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10 = True RMS and accuracy valid 10%–100% of range
# Used by: UT804
# In: D. AC Current (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10-2 = True RMS and accuracy valid 10%–100% of range; crest factor up to 3.0
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage
note-true-rms-from-10-pct-to = True RMS from 10% to 100% of range; BM821s: average sensing
# Used by: UT181A
# In: 4 tables
note-true-rms-valid-10-pct-100 = True RMS valid 10%–100% of range
# Used by: UT803
# In: B. AC Voltage
note-true-rms-valid-10-pct-95 = True RMS valid 10%–95% of range
# Used by: UT803
# In: D. AC Current
note-true-rms-valid-10-pct-95-2 = True RMS valid 10%–95% of range; crest factor 3.0
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-true-rms-accuracy-valid-10-pct = True RMS; accuracy valid 10%–100% of range
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-2-0 = True RMS; crest factor ≤2.0 at 10000, ≤1 at 22000 counts
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-3-0 = True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Temperature
note-type-k-thermocouple-range-accuracy-not = Type-K thermocouple range & accuracy not included
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-typical-20v-55v-110v-220v-440v = Typical: - 20V, -- 55V, --- 110V, ---- 220V, ----- 440V
# Used by: UT804
# In: J. Duty Cycle
note-valid-10-pct-90-pct-of = Valid 10%–90% of range, 5Hz–2kHz only
# Used by: UT803
# In: K. Transistor
note-vce-2-2v-bo-10ua = Vce ≈2.2V, bo ≈10µA
# Used by: UT181A
# In: (12) Duty Cycle; (13) Pulse Width
note-when-rise-time-1us-signals-center = When rise time <1µs, signals center on trigger level
# Used by: UT803
# In: G. Diode Testst
note-working-current-1ma = Working current ≈1mA
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current
note-ua-ranges-min-30ua = µA ranges: min 30µA
# Used by: UT803
# In: C. DC Current; D. AC Current
note-5a-range-continuous-5a-range-10s = ≤5A range: continuous; >5A range: ≤10s at a time, ≥15min apart
