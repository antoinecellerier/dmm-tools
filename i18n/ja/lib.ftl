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
mode-a-ac-peak = A AC ピーク
# Used by: UT171A/B/C
mode-a-ac-plus-dc = A AC+DC
# Used by: UT171A/B/C, UT181A
mode-a-dc = A DC
# Used by: UT181A
mode-a-dc-ac-plus-dc = A DC AC+DC
# Used by: UT181A
mode-a-dc-peak = A DC ピーク
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
mode-ac-power-a = AC 電力 A
# Used by: 9 meters
mode-ac-power-ah = AC 電力 Ah
# Used by: 9 meters
mode-ac-power-v = AC 電力 V
# Used by: 9 meters
mode-ac-power-va = AC 電力 VA
# Used by: 9 meters
mode-ac-power-w = AC 電力 W
# Used by: 9 meters
mode-ac-power-wh = AC 電力 Wh
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
mode-angle = 角度
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
mode-auto = オート
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-auto-v = Auto V
# Used by: 46 meters
mode-capacitance = 静電容量
# Used by: UT8803
mode-capacitance-d = 静電容量 D
# Used by: UT8803
mode-capacitance-r = 静電容量 R
# Used by: 8 meters
mode-clamp-ac-a = クランプ AC A
# Used by: 8 meters
mode-clamp-ac-plus-dc-a = クランプ AC+DC A
# Used by: 8 meters
mode-clamp-dc-a = クランプ DC A
# Used by: 8 meters
mode-clamp-lpf-a = クランプ LPF A
# Used by: 9 meters
mode-compass = コンパス
# Used by: 45 meters
mode-continuity = 導通
# Used by: UT181A
mode-continuity-open = 導通 (開放)
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
mode-dc-power-a = DC 電力 A
# Used by: 9 meters
mode-dc-power-ah = DC 電力 Ah
# Used by: 9 meters
mode-dc-power-v = DC 電力 V
# Used by: 9 meters
mode-dc-power-va = DC 電力 VA
# Used by: 9 meters
mode-dc-power-w = DC 電力 W
# Used by: 9 meters
mode-dc-power-wh = DC 電力 Wh
# Used by: 27 meters
mode-dc-ua = DC µA
# Used by: 121GW
mode-dc-uva = DC µVA
# Used by: 45 meters
mode-diode = ダイオード
# Used by: UT181A
mode-diode-alarm = ダイオード アラーム
# Used by: 44 meters
mode-duty-pct = デューティー比 %
# Used by: BM829s/BM827s/BM822s/BM821s
mode-ef = EF
# Used by: BM788BT/BM787BT
mode-ef-h = EF-H
# Used by: BM788BT/BM787BT
mode-ef-l = EF-L
# Used by: 8 meters
mode-frequency = 周波数
# Used by: 9 meters
mode-hv-ac-v = HV AC V
# Used by: 9 meters
mode-hv-dc-v = HV DC V
# Used by: UT8803
mode-inductance = インダクタンス
# Used by: UT8803
mode-inductance-q = インダクタンス Q
# Used by: UT8803
mode-inductance-r = インダクタンス R
# Used by: 22 meters
mode-inrush = 突入電流
# Used by: BM788BT/BM787BT
mode-line-hz = ライン Hz
# Used by: 8 meters
mode-live = 活線
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
mode-logic-hz = ロジック Hz
# Used by: 9 meters
mode-motor = モーター
# Used by: 9 meters
mode-motor-1-2-3 = モーター 1-2-3
# Used by: 9 meters
mode-motor-3-2-1 = モーター 3-2-1
# Used by: 20 meters
mode-ncv = NCV
# Used by: UT71A/B, UT71C/D/E, UT804, VC920/VC940/VC960
mode-power = 電力
# Used by: 9 meters
mode-power-a = 電力 A
# Used by: 9 meters
mode-power-ah = 電力 Ah
# Used by: 9 meters
mode-power-v = 電力 V
# Used by: 9 meters
mode-power-va = 電力 VA
# Used by: 9 meters
mode-power-w = 電力 W
# Used by: 9 meters
mode-power-wh = 電力 Wh
# Used by: 9 meters
mode-power-factor = 力率
# Used by: 121GW, UT181A
mode-pulse-width = パルス幅
# Used by: BM525s/BM521s
mode-recall = データ呼び出し
# Used by: UT8802, UT8803
mode-scr = SCR
# Used by: 9 meters
mode-scope = オシロスコープ
# Used by: 9 meters
mode-solar = ソーラー
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1 = T1
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1-t2 = T1-T2
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t2 = T2
# Used by: UT803
mode-tachometer = 回転計
# Used by: Mock UT61E+
mode-temp-degc = 温度 °C
# Used by: Mock UT61E+
mode-temp-degc-t1-t2 = 温度 °C T1 (T2)
# Used by: Mock UT61E+
mode-temp-degc-t1-t2-2 = 温度 °C T1-T2
# Used by: Mock UT61E+
mode-temp-degc-t2-t1 = 温度 °C T2-T1
# Used by: UT71A/B, UT71C/D/E, UT803, UT804, VC920/VC940/VC960
mode-temperature = 温度
# Used by: 9 meters
mode-time = 時間
# Used by: 9 meters
mode-usb-power-a = USB 電力 A
# Used by: 9 meters
mode-usb-power-ah = USB 電力 Ah
# Used by: 9 meters
mode-usb-power-v = USB 電力 V
# Used by: 9 meters
mode-usb-power-va = USB 電力 VA
# Used by: 9 meters
mode-usb-power-w = USB 電力 W
# Used by: 9 meters
mode-usb-power-wh = USB 電力 Wh
# Used by: UT171A/B/C, UT181A
mode-v-ac = V AC
# Used by: UT181A
mode-v-ac-hz = V AC Hz
# Used by: UT181A
mode-v-ac-lpf = V AC LPF
# Used by: UT181A
mode-v-ac-peak = V AC ピーク
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
mode-v-dc-peak = V DC ピーク
# Used by: UT171A/B/C
mode-vfc = VFC
# Used by: BM788BT/BM787BT, BM869s/BM867s
mode-vfd-ac-v = VFD AC V
# Used by: BM788BT/BM787BT, BM869s/BM867s
mode-vfd-hz = VFD Hz
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-dbm-reference = dBm 基準値
# Used by: 25 meters
mode-hfe = hFE
# Used by: UT171A/B/C, UT181A
mode-ma-ac = mA AC
# Used by: UT181A
mode-ma-ac-hz = mA AC Hz
# Used by: UT181A
mode-ma-ac-peak = mA AC ピーク
# Used by: UT171A/B/C
mode-ma-ac-plus-dc = mA AC+DC
# Used by: UT171A/B/C, UT181A
mode-ma-dc = mA DC
# Used by: UT181A
mode-ma-dc-ac-plus-dc = mA DC AC+DC
# Used by: UT181A
mode-ma-dc-peak = mA DC ピーク
# Used by: UT171A/B/C, UT181A
mode-mv-ac = mV AC
# Used by: UT181A
mode-mv-ac-ac-plus-dc = mV AC AC+DC
# Used by: UT181A
mode-mv-ac-hz = mV AC Hz
# Used by: UT181A
mode-mv-ac-peak = mV AC ピーク
# Used by: UT171A/B/C
mode-mv-ac-plus-dc = mV AC+DC
# Used by: UT171A/B/C, UT181A
mode-mv-dc = mV DC
# Used by: UT181A
mode-mv-dc-peak = mV DC ピーク
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
mode-ua-ac-peak = µA AC ピーク
# Used by: UT171A/B/C
mode-ua-ac-plus-dc = µA AC+DC
# Used by: UT171A/B/C, UT181A
mode-ua-dc = µA DC
# Used by: UT181A
mode-ua-dc-ac-plus-dc = µA DC AC+DC
# Used by: UT181A
mode-ua-dc-peak = µA DC ピーク

## Mode qualifiers: a word around a mode's name for a low-pass filter, a
## held peak or a relative reading, around any of the modes above. $mode is the
## mode, already translated: put it where your language puts it.

# Used by: 9 meters
# $mode is the mode, already translated.
qualifier-lpf-mode = LPF { $mode }
# Used by: 9 meters
# $mode is the mode, already translated.
qualifier-peak-mode = { $mode } ピーク
# Used by: UT181A
# $mode is the mode, already translated.
qualifier-mode-rel = { $mode } REL
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
# $mode is the mode, already translated.
qualifier-mode-peak = { $mode } ピーク

## Ranges: beside the reading and in the range menu. Ranges that are a
## number and a unit only are not listed.

# Used by: UT161E, UT61E+
range-1000beta = 1000β
# Used by: 12 meters
range-auto = オート
# Used by: 7 meters
range-cont = 導通
# Used by: 7 meters
range-diode = ダイオード
# Used by: 7 meters
range-duty = デューティー比
# Used by: Mock UT61E+, UT161E, UT61E+
range-temp = 温度

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
aux-absolute = 絶対値
# Used by: 9 meters
aux-angle = 角度
# Used by: UT171A/B/C
aux-aux = 補助
# Used by: UT181A
aux-average = 平均値
# Used by: 121GW
aux-battery = 電池
# Used by: 121GW
aux-burden-voltage = バーデン電圧
# Used by: UT181A
aux-comp-high = COMP 上
# Used by: UT181A
aux-comp-low = COMP 下
# Used by: 10 meters
aux-capacitance = 静電容量
# Used by: UT202BT
aux-clamp-ac-a = クランプ AC A
# Used by: UT202BT
aux-clamp-lpf-a = クランプ LPF A
# Used by: 9 meters
aux-compass = コンパス
# Used by: 10 meters
aux-continuity = 導通
# Used by: 121GW
aux-current = 電流
# Used by: 12 meters
aux-dc = DC
# Used by: 9 meters
aux-dc-a = DC A
# Used by: 10 meters
aux-dc-v = DC V
# Used by: 9 meters
aux-diode = ダイオード
# Used by: 10 meters
aux-duty = デューティー比
# Used by: 9 meters
aux-frequency = 周波数
# Used by: 9 meters
aux-hv-ac-v = HV AC V
# Used by: 9 meters
aux-hv-dc-v = HV DC V
# Used by: UT181A
aux-impedance = インピーダンス
# Used by: UT202BT
aux-inrush = 突入電流
# Used by: 121GW
aux-internal-temperature = 内部温度
# Used by: UT202BT
aux-lpf-v = LPF V
# Used by: VC871
aux-live = 活線
# Used by: UT181A
aux-max = 最大
# Used by: UT181A
aux-min = 最小
# Used by: 9 meters
aux-motor = モーター
# Used by: CMS061, CMS101
aux-ncv = NCV
# Used by: 9 meters
aux-pf = PF
# Used by: UT181A
aux-peak-min = ピーク最小
# Used by: Mock UT61E+, UT181A
aux-period = 周期
# Used by: UT181A, VC871
aux-reference = 基準値
# Used by: 9 meters
aux-solar = ソーラー
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, Mock UT61E+, UT181A
aux-t1 = T1
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s, Mock UT61E+, UT181A
aux-t2 = T2
# Used by: UT202BT
aux-temperature = 温度
# Used by: 9 meters
aux-time = 時間
# Used by: 121GW, UT181A
aux-voltage = 電圧
# Used by: 9 meters
aux-hfe = hFE

## On and off, for a meter setting offered as a choice.

# Used by: 13 meters
toggle-off = オフ
# Used by: 13 meters
toggle-on = オン

## Meter keys: the remote panel's buttons, named after the meter's keys.
## Keep a name the meter prints on its key (SEL, ZERO) as it is.

# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-auto-function = オート機能
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-capacitance = 静電容量
# Used by: ZT-300AB, ZT-5566SE, ZT-5BQ
key-current = 電流
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-diode-continuity = ダイオード/導通
# Used by: 9 meters
key-hz-duty = Hz/Duty
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-ncv = NCV
# Used by: CM2100B, Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-zero = ZERO

## Meter key hovers: how to do on the meter what a remote button does.
## Keep the meter's key names as it prints them.

# Used by: 121GW
key-hover-hold-the-meter-s-rel-key = マルチメーターの REL キーを長押しする (1 kHz ローパスフィルター)
# Used by: OW18B/OW16B, OW18E
key-hover-press-the-meter-s-hz-duty = マルチメーターの Hz/Duty△/ᛒ キーを短く押す
# Used by: CM2100B
key-hover-press-the-meter-s-zero-key = マルチメーターの ZERO/ᛒ キーを短く押す

## Model names with words in them: the simulated meters. Keep the part
## number as it is.

# Used by: Mock UT61E+
model-mock-ut61e-plus = UT61E+ (シミュレーション)
# Used by: Mock ZT-5B
model-mock-zt-5b = ZT-5B (シミュレーション)

## Activation steps: how to switch a meter's data transmission on, shown
## when a connection gets no data. Keep each step on its own line, and the
## meter's key names, menu items and display text as the meter prints them.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
activation-bm52x =
    1. BU-86X ケーブルをマルチメーター背面の光学式 PC-Comm ポートに取り付けて、コンピューターに接続します。
    2. ファンクションスイッチを電圧の測定機能に合わせて、マルチメーターの電源を入れます。
    注意: BM869s と BM867s では約 17 分、その他の機種では 30 分操作しないと、マルチメーターの電源が切れます。無効にするには、SELECT を押しながら電源を入れます。
# Used by: UT8802, UT8803
activation-ut8802 =
    1. USB ケーブルをマルチメーターに接続します。
    2. マルチメーターの電源を入れます。
# Used by: UT171A/B/C
activation-ut171 =
    1. USB ケーブルをマルチメーターに接続します。
    2. マルチメーターの電源を入れます。
    3. SETUP -> Communication -> ON の順に選択します。
# Used by: UT181A
activation-ut181a =
    1. USB ケーブルをマルチメーターに接続します。
    2. マルチメーターの電源を入れます。
    3. SETUP -> Communication -> ON の順に選択します。
    注意: この設定は電源を入れ直すとリセットされます。
# Used by: UT71A/B, UT71C/D/E, VC920/VC940/VC960
activation-ut71ab =
    1. USB ケーブルをマルチメーターに接続します。
    2. マルチメーターの電源を入れます。
    3. MAX MIN を 1 s 押し続けます (UT71A では SEND を押します)。ディスプレイに SEND が表示されます。
    注意: EXIT を押すと SEND がオフになります。
# Used by: UT803
activation-ut803 =
    1. USB ケーブルをマルチメーターに接続します。
    2. マルチメーターの電源を入れます。
    3. RS232 を押します。ディスプレイに RS232 が表示されます。
# Used by: UT804
activation-ut804 =
    1. USB ケーブルをマルチメーターに接続します。
    2. マルチメーターの電源を入れます。
    3. SEND を押します。ディスプレイに SEND が表示されます。
    注意: HOLD を押すとデータ送信が一時停止し、EXIT を押すと SEND がオフになります。
# Used by: VC-880, VC-890, VC650BT
activation-vc650bt =
    1. USB ケーブルをマルチメーターに接続します。
    2. マルチメーターの電源を入れます。
    3. マルチメーターの PC ボタンを押します。
# Used by: BM788BT/BM787BT
activation-bm78xbt =
    1. スマートフォン アプリとマルチメーターの接続をすべて切断します。
    2. ファンクションスイッチを Auto V/LoZ 以外の任意の測定機能に合わせます。
    3. ディスプレイに ((D)) が表示されるまで、Δ を 1 秒以上押し続けます。
    注意: 15 から 30 分操作しないと、マルチメーターの電源が切れます。無効にするには、SELECT を押しながら電源を入れます (dSAPO が表示されます)。
    マルチメーターが見つからない場合や接続パスワードが変更された場合は、Hz ボタンを押しながら、ファンクションスイッチを 0.6 s 以内に OFF から静電容量の位置まで回します。マルチメーターに「Org」と表示され、接続パスワードと Bluetooth 名が工場出荷時の設定に戻ります。
# Used by: OW69B
activation-ow69b =
    1. スマートフォン アプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. Bluetooth アイコンが表示されるまで、上に BLE と印字された < キーを押し続けます。
# Used by: OW67B
activation-ow67b =
    1. スマートフォン アプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. Bluetooth アイコンが表示されるまで、上に BLE と印字された < キーを押し続けます。
    注意: 5 分間操作しないと Bluetooth がオフになります。もう一度 < を押し続けるとオンに戻ります。
# Used by: VC915, VC925 PV
activation-vc915 =
    1. スマートフォン アプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. マルチメーターのビープ音が鳴り、Bluetooth マークが表示されるまで、BLE ボタンを約 2 秒間押し続けます。
# Used by: VC871, VC891
activation-vc871 =
    1. スマートフォン アプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. マルチメーターのビープ音が鳴り、左上に Bluetooth マークが表示されるまで、BLE ボタンを約 2 秒間押し続けます。
    注意: マルチメーターの電源を入れるたびに Bluetooth はオフになるので、そのつど BLE を押し続けます。
# Used by: OW18B/OW16B, OW18E
activation-ow18b =
    1. スマートフォン アプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. ᛒ アイコンが表示されるまで、Hz/Duty△/ᛒ キーを押し続けます。
    注意: 10 分間操作しないと、ビープ音が 2 回鳴った後に Bluetooth がオフになります。もう一度 Hz/Duty△/ᛒ を押し続けるとオンに戻ります。
# Used by: OW65B
activation-ow65b =
    1. スマートフォン アプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. Bluetooth アイコンが表示されるまで、上に BLE と印字された SETUP キーを押し続けます。
    注意: もう一度 SETUP を押し続けると Bluetooth がオフになります。
# Used by: CMS061, CMS101
activation-cms061 =
    1. スマートフォン アプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. Bluetooth アイコンが表示されるまで、上に BLE と印字された Tab⇌ キーを押し続けます。
    注意: Bluetooth がオンの間は、マルチメーターの電源は自動で切れません。
# Used by: CM2100B
activation-cm2100b =
    1. スマートフォン アプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. ディスプレイの左上に ᛒ アイコンが表示されるまで、ZERO/ᛒ キーを約 2 秒間押し続けます。
    注意: 5 分間操作しないか、もう一度 ZERO/ᛒ を押し続けると、Bluetooth がオフになります。マルチメーターは約 15 分間操作しないと電源が切れます。次に電源を切るまでこの機能を無効にするには、SELECT を押しながら電源を入れます。
# Used by: B33
activation-b33 =
    1. スマートフォン アプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. ᛒ アイコンが表示されるまで、△/ᛒ キーを押し続けます。
    注意: 10 分間操作しないと (ビープ音が 2 回鳴った後)、またはオフライン記録が終了すると、Bluetooth がオフになります。もう一度 △/ᛒ を押し続けるとオンに戻ります。
# Used by: B35T+, B41T+
activation-b35t-plus =
    1. スマートフォン アプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. ᛒ アイコンが表示されるまで、△/ᛒ キーを押し続けます。
    注意: 10 分間操作しないと、ビープ音が 2 回鳴った後に Bluetooth がオフになります。もう一度 △/ᛒ を押し続けるとオンに戻ります。
# Used by: ZT-5B
activation-zt5b =
    1. 赤いボタンを 2 秒間押し続けて、マルチメーターの電源を入れます。
    2. 赤いボタンを短く押します。Bluetooth マークが表示されます。
    注意: 15 分間操作しないと、マルチメーターの電源が切れます。無効にするには、電源を入れる前に NCV を押します。
# Used by: UT161B, UT161D, UT161E, UT61B+, UT61D+, UT61E+
activation-ut161b =
    1. USB モジュールをマルチメーターに差し込みます。
    2. マルチメーターの電源を入れます。
    3. USB/Hz ボタンを長押しします。
    4. LCD に S アイコンが表示されます。
# Used by: 121GW
activation-121gw =
    1. ファンクションスイッチを任意の測定機能に合わせます。
    2. ディスプレイに BT が表示されるまで、1ms PEAK を押し続けます。
    注意: 30 分経つとマルチメーターの電源が切れます。無効にするには、SETUP で APO.oF に設定します。
# Used by: ZT-300AB
activation-zt300ab =
    1. マルチメーターの電源を入れます。
    2. Hz% ボタンを 2 秒間押し続けます。Bluetooth マークが表示されます。
    注意: 15 分間操作しないと、マルチメーターの電源が切れます。無効にするには、SEL を押しながら電源を入れます。
# Used by: UT60BT
activation-ut60bt =
    1. マルチメーターの電源を入れます。
    2. Bluetooth マークが表示されるまで、SEL を長押しします。
# Used by: ZT-5566SE
activation-zt5566se =
    1. マルチメーターの電源を入れます。
    2. POWER を押して Bluetooth をオンにします。Bluetooth マークが点滅します。
    注意: 取扱説明書には、スピーカー用の Bluetooth しか記載されていません。Bluetooth 経由の測定値は検証されていません。
# Used by: ZT-5BQ
activation-zt5bq =
    1. マルチメーターの電源を入れます。
    2. Power と Hz を同時に押します。Bluetooth マークが表示されます。
    注意: 15 分間操作しないと、マルチメーターの電源が切れます。無効にするには、Hz/NCV を押しながら電源を入れます。
# Used by: UT202BT
activation-ut202bt =
    1. マルチメーターの電源を入れます。
    2. Bluetooth ボタンを短く押します。Bluetooth マークが点滅します。
    注意: 接続がないまま 5 分経つと、Bluetooth は自動でオフになります。
# Used by: Mock UT61E+, Mock ZT-5B
activation-mock = シミュレーション用のデバイスのため、設定は不要です。

## Specification table names, in the Specifications panel, as the
## meter's manual titles them. Keep the manual's numbering: (1), A.

# Used by: UT181A
table-1-ac-voltage = (1) 交流電圧
# Used by: UT181A
table-10-temperature = (10) 温度
# Used by: UT181A
table-11-frequency = (11) 周波数
# Used by: UT181A
table-12-duty-cycle = (12) デューティー比
# Used by: UT181A
table-13-pulse-width = (13) パルス幅
# Used by: UT181A
table-14-continuity-test = (14) 導通チェック
# Used by: UT181A
table-15-diode-test = (15) ダイオードテスト
# Used by: UT181A
table-2-dc-voltage = (2) 直流電圧
# Used by: UT181A
table-3-ac-voltage-plus-dc-voltage = (3) 交流電圧+直流電圧
# Used by: UT181A
table-4-ac-current = (4) 交流電流
# Used by: UT181A
table-5-dc-current = (5) 直流電流
# Used by: UT181A
table-6-ac-current-plus-dc-current = (6) 交流電流+直流電流
# Used by: UT181A
table-7-resistance = (7) 抵抗
# Used by: UT181A
table-8-conductance = (8) コンダクタンス
# Used by: UT181A
table-9-capacitance = (9) 静電容量
# Used by: UT803, UT804
table-a-dc-voltage = A. 直流電圧
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-ac-ac-plus-dc-current = 交流電流と交流+直流電流
# Used by: 7 meters
table-ac-current = 交流電流
# Used by: 9 meters
table-ac-voltage = 交流電圧
# Used by: Mock ZT-5B, ZT-5B
table-ac-voltage-v = 交流電圧 (V)
# Used by: Mock ZT-5B, ZT-5B
table-ac-current-a = 交流電流 (A)
# Used by: Mock ZT-5B, ZT-5B
table-ac-current-ma = 交流電流 (mA)
# Used by: Mock UT61E+, UT161E, UT61E+
table-ac-plus-dc-voltage = 交流+直流電圧
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-audible-continuity-tester = 導通ブザー
# Used by: Mock ZT-5B, ZT-5B
table-auto = オート
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-acv = AutoCheck™ (ACV)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-dcv = AutoCheck™ (DCV)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-ohms = AutoCheck™ (Ohms)
# Used by: UT803
table-b-ac-voltage = B. 交流電圧
# Used by: UT804
table-b-ac-voltage-ac-plus-dc = B. 交流電圧 (交流+直流の測定が可能)
# Used by: UT803, UT804
table-c-dc-current = C. 直流電流
# Used by: 11 meters
table-capacitance = 静電容量
# Used by: Mock ZT-5B, ZT-5B
table-continuity = 導通
# Used by: 7 meters
table-continuity-and-diode = 導通とダイオード
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-crest-mode-instantaneous-peak-hold = クレストモード (瞬時ピークホールド)
# Used by: UT803
table-d-ac-current = D. 交流電流
# Used by: UT804
table-d-ac-current-ac-plus-dc = D. 交流電流 (交流+直流の測定が可能)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-dc-ac-ac-plus-dc-ac = 直流、交流、交流+直流電圧
# Used by: 9 meters
table-dc-current = 直流電流
# Used by: Mock ZT-5B, ZT-5B
table-dc-voltage-v = 直流電圧 (V)
# Used by: 9 meters
table-dc-voltage = 直流電圧
# Used by: Mock ZT-5B, ZT-5B
table-dc-current-a = 直流電流 (A)
# Used by: Mock ZT-5B, ZT-5B
table-dc-current-ma = 直流電流 (mA)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-diode-tester = ダイオードテスト
# Used by: UT803, UT804
table-e-resistance = E. 抵抗
# Used by: UT803, UT804
table-f-continuity-test = F. 導通チェック
# Used by: Mock ZT-5B, ZT-5B
table-frequency = 周波数
# Used by: 7 meters
table-frequency-duty-ratio = 周波数/デューティー比
# Used by: UT804
table-g-diode-test = G. ダイオードテスト
# Used by: UT803
table-g-diode-testst = G. ダイオードテスト
# Used by: UT803, UT804
table-h-capacitance = H. 静電容量
# Used by: UT803, UT804
table-i-frequency = I. 周波数
# Used by: UT804
table-j-duty-cycle = J. デューティー比
# Used by: UT803
table-j-temperature = J. 温度
# Used by: UT804
table-k-temperature-1-1-degrees-celsius = K. 温度 1-1. 摂氏
# Used by: UT804
table-k-temperature-1-2-fahrenheit = K. 温度 1-2. 華氏
# Used by: UT803
table-k-transistor = K. トランジスタ
# Used by: UT804
table-l-4-20-ma-loop-current = L. 4-20 mA ループ電流
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-line-level-frequency-hz = ライン周波数 (~Hz)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-logic-level-frequency-hz-duty-cycle = ロジックレベル周波数 (⎍Hz) とデューティー比 (D%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-non-contact-ef-detection = 非接触電界検出
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-ohms = 抵抗
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-record-mode = 記録モード
# Used by: 9 meters
table-resistance = 抵抗
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, Mock ZT-5B, UT161D, UT61D+, ZT-5B
table-temperature = 温度
# Used by: Mock UT61E+, UT161E, UT61E+
table-transistor-magnification = トランジスタの電流増幅率 (hFE)

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
band-in-rel-state = REL モード時
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); I. Frequency
band-not-specified = 規定なし
# Used by: UT181A, UT804
# In: (9) Capacitance; H. Capacitance
band-not-specified-2 = 規定なし
# Used by: UT181A
# In: (1) AC Voltage; (3) AC Voltage + DC Voltage
band-only-for-reference = 参考値
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage; DC AC & AC+DC AC Voltage
band-unspec-d = 規定なし
# Used by: UT804
# In: A. DC Voltage
band-under-rel-mode = REL モード時

## Specification inputs: a table's input impedance or overload
## protection. Keep every number, sign and ± exactly; a decimal comma is fine
## (0,6). A translation that changes, drops or adds a number is shown in
## English.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: 11 tables
input-1000v-dc-ac-rms = 1000V DC/AC rms
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: 4 tables
input-10mohm-50pf-nominal = 10MΩ, 50pF (定格)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage; DC AC & AC+DC AC Voltage; DC Voltage
input-10mohm-80pf-nominal = 10MΩ, 80pF (定格)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: 6 tables
input-1100v-dc-ac-rms = 1100V DC/AC rms
# Used by: UT803
# In: 6 tables
input-250v-rms = 250V rms
# Used by: 8 meters
# In: 6 tables
input-about-10mohm = 約 10MΩ
# Used by: 7 meters
# In: DC Voltage
input-about-1gohm = 約 1GΩ
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
input-approx-10mohm = 約 10MΩ
# Used by: UT803, UT804
# In: A. DC Voltage; B. AC Voltage
input-around-10mohm = 約 10MΩ
# Used by: UT804
# In: A. DC Voltage
input-around-2-5gohm = 約 2.5GΩ
# Used by: UT803
# In: A. DC Voltage; B. AC Voltage
input-around-3000mohm = 約 > 3000MΩ
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current; Line Level Frequency (~Hz)
input-fuse-0-44a-1000v = ヒューズ 0.44A 1000V
# Used by: UT804
# In: C. DC Current; D. AC Current (AC+DC measurement is available); L. 4~20 mA loop current
input-fuse-0-5a-250v = ヒューズ 0.5A 250V
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
input-fuse-0-8a-1000v = ヒューズ 0.8A 1000V
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
input-fuse-10a-1000v = ヒューズ 10A 1000V
# Used by: Mock UT61E+, UT61B+, UT61D+, UT61E+
# In: AC Current; DC Current
input-fuse-10a-240v = ヒューズ 10A 240V
# Used by: UT803, UT804
# In: C. DC Current; D. AC Current; D. AC Current (AC+DC measurement is available)
input-fuse-10a-250v = ヒューズ 10A 250V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, UT161B, UT161D, UT161E
# In: 4 tables
input-fuse-11a-1000v = ヒューズ 11A 1000V
# Used by: Mock UT61E+, UT61B+, UT61D+, UT61E+
# In: AC Current; DC Current
input-fuse-1a-240v = ヒューズ 1A 240V
# Used by: UT803
# In: K. Transistor
input-fuse-200ma-250v-fuse-500ma-125v = ヒューズ 200mA 250V、ヒューズ 500mA 125V
# Used by: UT803
# In: C. DC Current; D. AC Current
input-fuse-500ma-125v = ヒューズ 500mA 125V
# Used by: UT161B, UT161D, UT161E
# In: AC Current; DC Current
input-fuse-600ma-1000v = ヒューズ 600mA 1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
input-initially-approx-3-0kohm-150pf-nominal = 初期状態で約 3.0kΩ, 150pF (定格)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
input-initially-approx-3-0kohm-165pf-nominal = 初期状態で約 3.0kΩ, 165pF (定格)

## Specification notes, under a table in the Specifications panel, from
## the meter's manual. Keep the exact sense, comparisons and qualifiers such as
## "typical" included. Keep every number, sign and ± exactly; a decimal comma
## is fine (0,6). A translation that changes, drops or adds a number is shown
## in English.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-10a-continuous-10a-to-20a-30s = 10A は連続、>10A から 20A まで: 最大 30s、冷却 5 min
# Used by: UT804
# In: C. DC Current; D. AC Current (AC+DC measurement is available)
note-10a-5a-continuous-5a-10a-10s = 10A: ≤5A は連続、>5A–10A は ≤10s ずつ、間隔 >15min
# Used by: UT181A
# In: (13) Pulse Width
note-10hz-200khz-pulse-width-2us-depends = 10Hz–200kHz: パルス幅 >2µs、信号の周波数に依存
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
note-20a-30s-on-then-10min-off = 20A: 30s 通電後 10min 休止、10A 超は規定なし
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-220mf-range-measurement-takes-20s = 220mF レンジ: 測定時間 ≈20s
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-25-00mf-manual-range-not-specified = 25.00mF、マニュアルレンジ: 5.4mF 未満は規定なし
# Used by: UT804
# In: E. Resistance
note-400ohm-accuracy-plus-test-lead-open = 400Ω: 確度 + テストリード開放時の値
# Used by: UT804
# In: H. Capacitance
note-40nf-accuracy-plus-open-circuit-test = 40nF: 確度 + テストリード開放時の静電容量
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-0-pct-20ma-shows = 4mA で 0%、20mA で 100% を表示
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-6-000mf-manual-range-not-specified = 6.000mF、マニュアルレンジ: 0.54mF 未満は規定なし
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms)
note-60-00mohm-tempco-0-6-accuracy = 60.00MΩ: 温度係数 0.6× 確度/°C @(0°C ~ 18°C または 28°C ~ 45°C)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-60-00mv-600-0mv-1khz-20khz = 60.00mV, 600.0mV, 1kHz ~ 20kHz: レンジの 30% から 100% で規定
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage
note-60-00mv-600-0mv-3khz-20khz = 60.00mV, 600.0mV, 3kHz ~ 20kHz: レンジの 30% から 100% で規定
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-600-0uf-manual-range-not-specified = 600.0µF、マニュアルレンジ: 50.0µF 未満は規定なし
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-600ohm-ref-typical-acmv-42-22 = 600Ω 基準、標準: ACmV -42.22 から -02.22dBm、ACV -17.78 から 62.22dBm
# Used by: UT803
# In: E. Resistance
note-600ohm-accuracy-plus-test-lead-short = 600Ω: 確度 + テストリード短絡時の抵抗
# Used by: UT181A
# In: (7) Resistance
note-60mohm-humidity-50-pct = 60MΩ: 湿度 <50%
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Capacitance
note-60mf-range-measurement-takes-20s = 60mF レンジ: 測定時間 ≈20s
# Used by: UT181A
# In: (2) DC Voltage
note-60mv-use-rel-to-compensate-bias = 60mV: バイアス電圧の補正には REL を使用
# Used by: UT803
# In: H. Capacitance
note-6nf-60nf-600nf-subtract-open-circuit = 6nF, 60nF, 600nF: テストリード開放時の静電容量を差し引く
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-9-999v-1khz-20khz-specified-for = 9.999V, 1kHz ~ 20kHz: 1kHz ~ 15kHz のみ規定
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage
note-9-999v-3khz-20khz-specified-for = 9.999V, 3kHz ~ 20kHz: 3kHz ~ 15kHz のみ規定
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-lo-20ma-shows-hi = <4mA で LO、>20mA で HI を表示
# Used by: 7 meters
# In: AC Current; DC Current
note-5a-max-10s-rest-15-min = >5A: 最大 10s、休止 ≥15 min
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-10-00a-15-00-3 = AC 10.00A: 15.00 ~ 3.000kHz、感度 6A (正弦波の実効値)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-6-000a-15-00-3 = AC 6.000A: 15.00 ~ 3.000kHz、感度 4A (正弦波の実効値)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-60-00ma-15-00-3 = AC 60.00mA: 15.00 ~ 3.000kHz、感度 40mA (正弦波の実効値)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-60-00mv-15-00-50 = AC 60.00mV: 15.00 ~ 50.00kHz、感度 40mV (正弦波の実効値)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0ma-15-00-3 = AC 600.0mA: 15.00 ~ 3.000kHz、感度 60mA (正弦波の実効値)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0mv-15-00-50 = AC 600.0mV: 15.00 ~ 50.00kHz、感度 60mV (正弦波の実効値)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0ua-15-00-3 = AC 600.0µA: 15.00 ~ 3.000kHz、感度 200µA (正弦波の実効値)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-6000ua-15-00-3-000khz = AC 6000µA: 15.00 ~ 3.000kHz、感度 600µA (正弦波の実効値)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-9-999v-15-00-10 = AC 9.999V: 15.00 ~ 10.00kHz、感度 2.5V (正弦波の実効値)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-99-99v-15-00-10 = AC 99.99V: 15.00 ~ 10.00kHz、感度 25V (正弦波の実効値)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-999-9v-15-00-10 = AC 999.9V: 15.00 ~ 10.00kHz、感度 100V (正弦波の実効値)
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-ac-plus-dc-add-1-pct = AC+DC: 確度に 1%+35 カウントを加算
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-ac-plus-dc-range-accuracy-plus = AC+DC: レンジの確度 + 1%
# Used by: UT804
# In: K. Temperature 1-1. Degrees Celsius; K. Temperature 1-2. Fahrenheit
note-above-230degc-use-the-rod-contact = 230°C 超: 棒状接触プローブを使用
# Used by: UT803
# In: J. Temperature
note-above-230degc-use-the-rod-type = 230°C 超: 棒状プローブを使用
# Used by: UT181A
# In: (10) Temperature
note-accessory-point-contact-probe-below-230degc = 付属の接触型プローブ: 230°C 未満のみ
# Used by: 7 meters
# In: 4 tables
note-accuracy-valid-1-pct-100-pct = 確度はレンジの 1%–100% で有効
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-accuracy-valid-1-pct-100-pct-2 = 確度はレンジの 1%–100% で有効 (1kHz–10kHz: 10%–100%)
# Used by: UT161D, UT61D+
# In: AC Current
note-accuracy-valid-40hz-1khz-1-pct = 確度は 40Hz–1kHz、レンジの 1%–100% で有効 (600.0µA: 5%–100%)
# Used by: UT161D, UT61D+
# In: AC Voltage
note-accuracy-valid-40hz-1khz-1-pct-2 = 確度は 40Hz–1kHz、レンジの 1%–100% で有効 (60mV: 2%–100%)
# Used by: UT161B, UT61B+
# In: AC Current
note-accuracy-valid-40hz-500hz-1-pct = 確度は 40Hz–500Hz、レンジの 1%–100% で有効 (600.0µA: 5%–100%)
# Used by: UT161B, UT61B+
# In: AC Voltage
note-accuracy-valid-40hz-500hz-1-pct-2 = 確度は 40Hz–500Hz、レンジの 1%–100% で有効 (60mV: 2%–100%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-accuracy-with-film-capacitor-or-better = 確度はフィルムコンデンサー以上の品質のコンデンサーで規定
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-audible-threshold-between-20ohm-and-300ohm = ブザーのしきい値: 20Ω から 300Ω の間
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-bm821s-specified-from-10-pct-to = BM821s: レンジの 10% から 100% で規定
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-bar-segments-beep-tones-proportional-to = バーグラフのセグメントとビープ音は電界強度に比例
# Used by: Mock ZT-5B, ZT-5B
# In: Continuity
note-beeps-50ohm = <50Ω でビープ音
# Used by: UT803
# In: F. Continuity Test
note-beeps-continuously-10ohm-silent-30ohm = ≤10Ω で連続ビープ音、>30Ω で鳴らない
# Used by: UT804
# In: F. Continuity Test
note-beeps-continuously-10ohm-silent-50ohm = ≤10Ω で連続ビープ音、>50Ω で鳴らない
# Used by: UT181A
# In: (15) Diode Test
note-brief-beep-on-a-normal-junction = 正常な接合では短いビープ音、短絡時は連続
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-0-02v-a = バーデン電圧 0.02V/A
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-0-08mv-ua = バーデン電圧 0.08mV/µA
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-2-1mv-ma = バーデン電圧 2.1mV/mA
# Used by: 7 meters
# In: Continuity and Diode
note-continuity-audio-visual-alarm-50ohm-no = 導通: <50Ω で音と表示によるアラーム、≥70Ω でビープ音なし
# Used by: UT803
# In: B. AC Voltage
note-crest-factor-3-0-1000v-1 = クレストファクター 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage; DC AC & AC+DC AC Voltage
note-crest-factor-2-1-at-full = クレストファクター: フルスケールで < 2:1、ハーフスケールで < 4:1
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-crest-factor-up-to-3-0 = クレストファクターは最大 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Crest mode (Instantaneous Peak Hold)
note-crest-specified-accuracy-adds-250-digits = クレスト: > 1.0 ms の変化では規定の確度に 250 カウントを加算
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-detects-50-60hz-antenna-at-the = 50/60Hz を検出、アンテナはマルチメーターの上端
# Used by: 7 meters
# In: Continuity and Diode
note-diode-one-beep-0-12v-2v = ダイオード: 0.12V–2V で単発のビープ音 (正常)、<0.12V で長いビープ音 (短絡)
# Used by: 7 meters
# In: Continuity and Diode
note-diode-open-circuit-3v = ダイオード: 開放電圧 ≈3V
# Used by: UT181A
# In: (9) Capacitance
note-display-digits-6000 = 表示カウント: 6000
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-duty-cycle-specified-for-5hz-10khz = デューティー比は 5Hz ~ 10kHz で規定
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-duty-square-waves-only-1vpp-20vpp = デューティー比: 方形波のみ、1Vpp–20Vpp、≤10kHz、10.0%–90.0%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-for-ac-voltage-shorted-leads-residual = 交流電圧、テストリード短絡時: 残留 ≤200 カウント
# Used by: Mock ZT-5B, ZT-5B
# In: AC Voltage (V); AC current (A); AC current (mA)
note-frequency-response-40hz-1khz = 周波数特性 40Hz–1kHz
# Used by: UT804
# In: G. Diode Test
note-good-silicon-junction-0-5v-0 = 正常なシリコン接合: 0.5V–0.8V
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-hold-sel-ncv-for-ncv = NCV は SEL/NCV を長押し
# Used by: UT181A
# In: (8) Conductance
note-humidity-50-pct = 湿度 <50%
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-1mhz-600mv-20v = Hz 入力 >100kHz–1MHz: 600mV–20V rms
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-1v-20v-rms = Hz 入力 >1MHz: 1V–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-40mhz-1v-20v = Hz 入力 >1MHz–40MHz: 1V–20V rms、>40MHz は規定なし
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-200mv-20v-rms = Hz 入力 ≤100kHz: 200mV–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Transistor Magnification
note-ib0-1-8ua-vce-2-5v = Ib0 ≈1.8µA, Vce ≈2.5V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV); AutoCheck™ (DCV)
note-impedance-rises-within-a-fraction-of = 50V を超えると数分の一秒以内にインピーダンスが上昇 (標準)
# Used by: UT804
# In: K. Temperature 1-1. Degrees Celsius; K. Temperature 1-2. Fahrenheit
note-included-k-type-point-contact-probe = 付属の K 型接触型プローブ: 230°C 未満のみ
# Used by: UT803
# In: J. Temperature
note-included-point-contact-probe-below-230degc = 付属の接触型プローブ: 230°C 未満のみ
# Used by: UT803
# In: I. Frequency
note-input-10hz-1mhz-150mv-30v-rms = 入力 10Hz–1MHz: 150mV–30V rms
# Used by: UT181A
# In: (11) Frequency
note-input-10hz-30mhz-600mv-30v-rms = 入力 10Hz–30MHz: 600mV–30V rms
# Used by: UT804
# In: I. Frequency; J. Duty Cycle
note-input-10hz-40mhz-200mv-30v-rms = 入力 10Hz–40MHz: 200mV–30V rms
# Used by: UT803
# In: I. Frequency
note-input-10mhz-50mhz-600mv-30v-rms = 入力 >10MHz–50MHz: 600mV–30V rms
# Used by: UT803
# In: I. Frequency
note-input-1mhz-10mhz-300mv-30v-rms = 入力 >1MHz–10MHz: 300mV–30V rms
# Used by: UT181A
# In: (11) Frequency
note-input-30mhz-not-specified = 入力 >30MHz: 規定なし
# Used by: UT804
# In: I. Frequency; J. Duty Cycle
note-input-at-zero-dc-level-40mhz = DC 成分ゼロの入力、>40MHz は規定なし
# Used by: UT803
# In: I. Frequency
note-input-at-zero-dc-level-50mhz = DC 成分ゼロの入力、>50MHz は規定なし
# Used by: UT161D, UT61D+
# In: Temperature
note-k-type-thermocouple-only = K 型熱電対のみ
# Used by: UT181A
# In: (10) Temperature
note-k-type-thermocouple-two-channels-via = K 型熱電対、温度コネクタ経由で二チャンネル
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-lo-z-threshold-plus-1-5vdc = Lo-Z しきい値: 定格 > +1.5VDC または < -1.0VDC
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-lo-z-threshold-3vac-50-60hz = Lo-Z しきい値: 定格 > 3VAC (50/60Hz)
# Used by: UT803
# In: K. Transistor
note-max-1000beta = 最大 1000β
# Used by: Mock ZT-5B, ZT-5B
# In: Frequency
note-max-10mhz = 最大 10MHz
# Used by: Mock ZT-5B, ZT-5B
# In: Resistance
note-max-40mohm = 最大 40MΩ
# Used by: Mock ZT-5B, ZT-5B
# In: Capacitance
note-max-4mf = 最大 4mF
# Used by: Mock ZT-5B, ZT-5B
# In: AC Voltage (V); DC VOLTAGE (V)
note-max-600v = 最大 600V
# Used by: Mock ZT-5B, ZT-5B
# In: 4 tables
note-max-9-999a = 最大 9.999A
# Used by: UT161D, UT61D+
# In: Temperature
note-measured-temperature-should-stay-below-230degc = 測定温度は 230°C/446°F 未満であること
# Used by: 7 meters
# In: AC Current; AC Voltage
note-non-sine-add-4-pct-crest = 非正弦波: 4% (クレストファクター 1–2)、5% (2–2.5)、7% (2.5–3) を加算
# Used by: UT181A
# In: (15) Diode Test
note-normal-silicon-junction-0-5v-0 = 正常なシリコン接合: 0.5V–0.8V
# Used by: UT181A
# In: (14) Continuity Test
note-open-alarm-beeps-50ohm-silent-10ohm = 開放アラーム: >50Ω でビープ音、<10Ω で鳴らない
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms); Ohms
note-open-circuit-1-2vdc-1-0vdc = 開放電圧 < 1.2VDC (60.00MΩ では < 1.0VDC)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-open-circuit-3-5vdc = 開放電圧 < 3.5VDC
# Used by: UT803
# In: F. Continuity Test
note-open-circuit-1-2v = 開放電圧 ≈-1.2V
# Used by: UT804
# In: F. Continuity Test
note-open-circuit-1-2v-2 = 開放電圧 ≈1.2V
# Used by: 7 meters
# In: Resistance
note-open-circuit-1v = 開放電圧 ≈1V
# Used by: UT803
# In: G. Diode Testst
note-open-circuit-2-7v = 開放電圧 ≈2.7V
# Used by: UT804
# In: G. Diode Test
note-open-circuit-2-8v = 開放電圧 ≈2.8V
# Used by: UT181A
# In: (14) Continuity Test
note-open-circuit-3v = 開放電圧 ≈3V
# Used by: UT181A
# In: (15) Diode Test
note-open-circuit-3v-forward-drop-up = 開放電圧 ≈3V、順方向電圧降下は最大 ≈3V
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; DC Current
note-open-circuit-residual-10-digits = 入力開放時: 残留 ≤10 カウント
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; DC Current
note-open-circuit-residual-5-digits = 入力開放時: 残留 ≤5 カウント
# Used by: Mock ZT-5B, ZT-5B
# In: AC current (A); DC current (A)
note-over-2a-measure-for-less-than = 2A 超: 測定は 3s 未満
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-range-accuracy-depend-on-acmv-acv = レンジと確度は ACmV/ACV と基準インピーダンスに依存
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-ranges-2-2uf-add-10-digits = ≤2.2µF のレンジ: 確度が ≤3% のとき 10 カウントを加算
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Transistor Magnification
note-reading-for-reference-only = 測定値は参考値
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Record mode
note-record-specified-accuracy-adds-10-digits = 記録: > 100 ms の変化では規定の確度に 10 カウントを加算
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-a-ma-picks = 赤のテストリードを A mA 端子に接続すると電流を選択
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-vohm-picks-v = 赤のテストリードを VΩ 端子に接続すると、0.8V 超で V、それ以外は Ω を選択
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-response-time-100us = 応答時間 < 100µs
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Capacitance
note-result-reading-open-lead-capacitance-1uf = 結果 = 測定値 − テストリード開放時の静電容量、≤1µF: REL を使用
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-result-reading-open-lead-capacitance-22nf = 結果 = 測定値 − テストリード開放時の静電容量、≤22nF: REL を使用
# Used by: 7 meters
# In: Resistance
note-result-reading-shorted-lead-resistance = 結果 = 測定値 − テストリード短絡時の抵抗
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-sel-ncv-continuity-diode-cap-hz = SEL/NCV: 導通/ダイオード、静電容量、Hz、°C
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-sensitivity-2-5vp-square-wave-for = 感度: 2.5Vp 方形波、3V および 5V のロジック ファミリー用
# Used by: UT181A
# In: (14) Continuity Test
note-short-alarm-beeps-10ohm-silent-50ohm = 短絡アラーム: <10Ω でビープ音、>50Ω で鳴らない
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-shorted-input-residual-under-30-digits = 入力短絡時: 残留 ~30 カウント未満、確度に影響なし
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-shorted-leads-80-digit-residual-accuracy = テストリード短絡時: 残留 80 カウント、確度に影響なし
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Voltage
note-shorted-leads-residual-10-digits = テストリード短絡時: 残留 ≤10 カウント
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Voltage
note-shorted-leads-residual-3-digits = テストリード短絡時: 残留 ≤3 カウント
# Used by: 7 meters
# In: DC Voltage
note-shorted-leads-residual-5-digits = テストリード短絡時: 残留 ≤5 カウント
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-test-current-0-4ma-typical = テスト電流 0.4mA (標準)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-then-typically-18kohm-100v-125kohm-300v = その後は標準 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 460kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-then-typically-18kohm-100v-125kohm-300v-2 = その後は標準 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 500kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-tolerance-10v-36v-23v-83v-59v = 許容差: 10V~36V, 23V~83V, 59V~165V, 124V~330V, > 250V
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10 = 真の実効値と確度はレンジの 10%–100% で有効
# Used by: UT804
# In: D. AC Current (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10-2 = 真の実効値と確度はレンジの 10%–100% で有効、クレストファクターは最大 3.0
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage
note-true-rms-from-10-pct-to = レンジの 10% から 100% で真の実効値、BM821s: 平均値方式
# Used by: UT181A
# In: 4 tables
note-true-rms-valid-10-pct-100 = 真の実効値はレンジの 10%–100% で有効
# Used by: UT803
# In: B. AC Voltage
note-true-rms-valid-10-pct-95 = 真の実効値はレンジの 10%–95% で有効
# Used by: UT803
# In: D. AC Current
note-true-rms-valid-10-pct-95-2 = 真の実効値はレンジの 10%–95% で有効、クレストファクター 3.0
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-true-rms-accuracy-valid-10-pct = 真の実効値、確度はレンジの 10%–100% で有効
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-2-0 = 真の実効値、クレストファクター: 10000 カウントで ≤2.0、22000 カウントで ≤1
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-3-0 = 真の実効値、クレストファクター: 3000 カウントで ≤3.0、6000 カウントで ≤1.5
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Temperature
note-type-k-thermocouple-range-accuracy-not = K 型熱電対のレンジと確度は含まれない
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-typical-20v-55v-110v-220v-440v = 標準: - 20V, -- 55V, --- 110V, ---- 220V, ----- 440V
# Used by: UT804
# In: J. Duty Cycle
note-valid-10-pct-90-pct-of = レンジの 10%–90%、5Hz–2kHz のみで有効
# Used by: UT803
# In: K. Transistor
note-vce-2-2v-bo-10ua = Vce ≈2.2V, bo ≈10µA
# Used by: UT181A
# In: (12) Duty Cycle; (13) Pulse Width
note-when-rise-time-1us-signals-center = 立ち上がり時間が <1µs のとき、信号はトリガーレベルを中心とする
# Used by: UT803
# In: G. Diode Testst
note-working-current-1ma = 動作電流 ≈1mA
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current
note-ua-ranges-min-30ua = µA レンジ: 最小 30µA
# Used by: UT803
# In: C. DC Current; D. AC Current
note-5a-range-continuous-5a-range-10s = ≤5A レンジ: 連続、>5A レンジ: ≤10s ずつ、間隔 ≥15min
