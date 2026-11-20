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
mode-a-ac-peak = A AC Peak
# Used by: UT171A/B/C
mode-a-ac-plus-dc = A AC+DC
# Used by: UT171A/B/C, UT181A
mode-a-dc = A DC
# Used by: UT181A
mode-a-dc-ac-plus-dc = A DC AC+DC
# Used by: UT181A
mode-a-dc-peak = A DC Peak
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
# Used by: 46 meters
mode-continuity = Continuity
# Used by: UT181A
mode-continuity-open = Continuity (open)
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
# Used by: 28 meters
mode-dc-ua = DC µA
# Used by: 121GW
mode-dc-uva = DC µVA
# Used by: 46 meters
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
# Used by: 9 meters
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
# Used by: UT71A, UT71B, UT71C/D/E, UT804, VC920/VC940/VC960
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
# Used by: UT171A/B/C
mode-square-wave = Square wave
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
# Used by: 26 meters
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
# Used by: 10 meters
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
# Used by: 13 meters
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

# Used by: 16 meters
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
# Used by: 13 meters
aux-dc = DC
# Used by: 9 meters
aux-dc-a = DC A
# Used by: 10 meters
aux-dc-v = DC V
# Used by: 9 meters
aux-diode = Diode
# Used by: 11 meters
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
# Used by: CMS061, CMS101, UT181A
aux-min = Min
# Used by: 9 meters
aux-motor = Motor
# Used by: CMS061, CMS101
aux-ncv = NCV
# Used by: 9 meters
aux-pf = PF
# Used by: UT181A
aux-peak-min = Peak Min
# Used by: Mock UT61E+, UT171A/B/C, UT181A
aux-period = Period
# Used by: UT171A/B/C
aux-pulse-width = Pulse Width
# Used by: UT171A/B/C, UT181A, VC871
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

# Used by: VC891, VC915
key-hover-1-khz-low-pass-filter = 1 kHz low-pass filter
# Used by: 121GW
key-hover-capture-1-ms-peaks-in-ac = Capture 1 ms peaks in AC V
# Used by: OW67B, VC871
key-hover-change-what-the-displays-show = Change what the displays show
# Used by: VC915, VC925 PV
key-hover-compare-the-reading-with-the-set = Compare the reading with the set limits
# Used by: CMS061, CMS101
key-hover-enter-max-min-click-again-to = Enter MAX/MIN; click again to leave it
# Used by: 121GW
key-hover-hold-the-meter-s-rel-key = Hold the meter's REL key (1 kHz low-pass filter)
# Used by: CMS061, CMS101
key-hover-measure-inrush-current-click-again-to = Measure inrush current; click again to leave it
# Used by: UT171A/B/C
key-hover-peak-hold-in-ac-v-and = Peak hold in AC V and A — click to enter or leave
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
# Used by: UT171A/B/C
key-hover-record-maximum-average-and-minimum-readings-2 = Record maximum, average and minimum readings — click to cycle, × to exit
# Used by: UT181A
key-hover-record-maximum-average-and-minimum-readings = Record maximum, average and minimum readings — × to exit
# Used by: VC-880, VC-890, VC650BT
key-hover-record-maximum-minimum-and-average-readings = Record maximum, minimum, and average readings — click to cycle, × to exit
# Used by: 121GW
key-hover-record-minimum-maximum-and-average-readings = Record minimum, maximum, and average readings — click to cycle, × to exit
# Used by: UT171A/B/C, UT181A
key-hover-save-the-reading-to-the-meter = Save the reading to the meter's memory
# Used by: CMS061, CMS101
key-hover-send-select-switches-v-dc-to = Send Select; switches V,DC to V,AC
# Used by: VC915
key-hover-show-3-phase-rotation-direction = Show 3-phase rotation direction
# Used by: VC915
key-hover-show-vac-or-vdc-on-the = Show VAC or VDC on the second display
# Used by: 121GW
key-hover-step-through-hold-and-autohold = Step through HOLD and AutoHOLD
# Used by: VC915
key-hover-swap-hz-pct-and-voltage = Swap Hz/% and voltage
# Used by: VC925 PV
key-hover-swap-the-main-and-second-displays = Swap the main and second displays
# Used by: OW69B
key-hover-switch-the-low-pass-filter-on = Switch the low-pass filter on or off
# Used by: UT171A/B/C
key-hover-take-the-reading-as-the-reference = Take the reading as the reference — × to exit
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
# Used by: UT71A, UT71B, UT71C/D/E, VC920/VC940/VC960
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
    1. Disconnect the meter from any other app
    2. Turn the dial to any function but Auto V/LoZ
    3. Hold Δ for one second or more, until ((D)) shows on the display
    Note: the meter switches off after 15 to 30 minutes idle; hold SELECT while turning it on to disable that (dSAPO shows).
    If the meter is not found or its connection password was changed, hold the Hz button while turning the dial from OFF to capacitance within 0.6 s: the meter shows "Org" and its connection password and Bluetooth name are back to the factory settings.
# Used by: OW69B
activation-ow69b =
    1. Disconnect the meter from any other app
    2. Turn the meter on
    3. Hold the < key (BLE printed above it) until the Bluetooth icon shows
# Used by: OW67B
activation-ow67b =
    1. Disconnect the meter from any other app
    2. Turn the meter on
    3. Hold the < key (BLE printed above it) until the Bluetooth icon shows
    Note: Bluetooth switches off after 5 minutes idle; hold < again to switch it back on.
# Used by: VC915, VC925 PV
activation-vc915 =
    1. Disconnect the meter from any other app
    2. Turn the meter on
    3. Hold the BLE button for about 2 seconds, until the meter beeps and the Bluetooth symbol shows
# Used by: VC871, VC891
activation-vc871 =
    1. Disconnect the meter from any other app
    2. Turn the meter on
    3. Hold the BLE button for about 2 seconds, until the meter beeps and the Bluetooth symbol shows at the top left
    Note: Bluetooth is off each time the meter is switched on; hold BLE again then.
# Used by: OW18B/OW16B, OW18E
activation-ow18b =
    1. Disconnect the meter from any other app
    2. Turn the meter on
    3. Hold the Hz/Duty△/ᛒ key until the ᛒ icon shows
    Note: Bluetooth switches off after 10 minutes idle, after two beeps; hold Hz/Duty△/ᛒ again to switch it back on.
# Used by: OW65B
activation-ow65b =
    1. Disconnect the meter from any other app
    2. Turn the meter on
    3. Hold the SETUP key (BLE printed above it) until the Bluetooth icon shows
    Note: holding SETUP again switches Bluetooth off.
# Used by: CMS061, CMS101
activation-cms061 =
    1. Disconnect the meter from any other app
    2. Turn the meter on
    3. Hold the Tab⇌ key (BLE printed above it) until the Bluetooth icon shows
    Note: the meter does not switch itself off while Bluetooth is on.
# Used by: CM2100B
activation-cm2100b =
    1. Disconnect the meter from any other app
    2. Turn the meter on
    3. Hold the ZERO/ᛒ key for about 2 seconds, until the ᛒ icon shows at the top left of the display
    Note: Bluetooth switches off after 5 minutes idle, or when ZERO/ᛒ is held again. The meter switches off after about 15 minutes idle; hold SELECT while turning it on to disable that until it is next switched off.
# Used by: B33
activation-b33 =
    1. Disconnect the meter from any other app
    2. Turn the meter on
    3. Hold the △/ᛒ key until the ᛒ icon shows
    Note: Bluetooth switches off after 10 minutes idle, after two beeps, and when an offline recording ends; hold △/ᛒ again to switch it back on.
# Used by: B35T+, B41T+
activation-b35t-plus =
    1. Disconnect the meter from any other app
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
