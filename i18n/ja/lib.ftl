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
mode-a-ac-peak = A AC ピーク
# Used by: UT171A/B/C
mode-a-ac-plus-dc = A AC+DC
# Used by: UT171A/B/C, UT181A
mode-a-dc = A DC
# Used by: UT181A
mode-a-dc-ac-plus-dc = A DC AC+DC
# Used by: UT181A
mode-a-dc-peak = A DC ピーク
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
mode-angle = 角度
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
mode-auto = オート
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-auto-v = Auto V
# Used by: 47 meters
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
# Used by: 46 meters
mode-continuity = 導通
# Used by: UT181A
mode-continuity-open = 導通 (開放)
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
# Used by: 28 meters
mode-dc-ua = DC µA
# Used by: 121GW
mode-dc-uva = DC µVA
# Used by: 46 meters
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
# Used by: 9 meters
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
# Used by: UT71A, UT71B, UT71C/D/E, UT804, VC920/VC940/VC960
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
# Used by: UT171A/B/C
mode-square-wave = 方形波
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1 = T1
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1-t2 = T1-T2
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t2 = T2
# Used by: Mock UT61E+
mode-temp-degc = 温度 °C
# Used by: Mock UT61E+
mode-temp-degc-t1-t2 = 温度 °C T1 (T2)
# Used by: Mock UT61E+
mode-temp-degc-t1-t2-2 = 温度 °C T1-T2
# Used by: Mock UT61E+
mode-temp-degc-t2-t1 = 温度 °C T2-T1
# Used by: UT71A, UT71B, UT71C/D/E, UT803, UT804, VC920/VC940/VC960
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
# Used by: UT171A/B/C
aux-pulse-width = パルス幅
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

## Meter keys named in words for what they do. A name printed on a key is
## not listed here: it stays as printed.

# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-auto-function = オート機能
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-capacitance = 静電容量
# Used by: ZT-300AB, ZT-5566SE, ZT-5BQ
key-current = 電流
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-diode-continuity = ダイオード/導通
# Used by: VC891, VC915
key-hover-1-khz-low-pass-filter = 1 kHz ローパスフィルター
# Used by: 121GW
key-hover-capture-1-ms-peaks-in-ac = AC V で 1 ms のピークを取得する
# Used by: OW67B, VC871
key-hover-change-what-the-displays-show = ディスプレイの表示内容を切り替える
# Used by: VC915, VC925 PV
key-hover-compare-the-reading-with-the-set = 測定値を設定した制限値と比較する
# Used by: CMS061, CMS101
key-hover-enter-max-min-click-again-to = MAX/MIN モードにする (もう一度クリックで解除)

## Meter key hovers: how to do on the meter what a remote button does.
## Keep the meter's key names as it prints them.

# Used by: 121GW
key-hover-hold-the-meter-s-rel-key = マルチメーターの REL キーを長押しする (1 kHz ローパスフィルター)
# Used by: CMS061, CMS101
key-hover-measure-inrush-current-click-again-to = 突入電流を測定する (もう一度クリックで解除)
# Used by: UT171A/B/C
key-hover-peak-hold-in-ac-v-and = AC V と AC A でピークをホールドする (クリックで開始/解除)
# Used by: OW18B/OW16B, OW18E
key-hover-press-the-meter-s-hz-duty = マルチメーターの Hz/Duty△/ᛒ キーを短く押す
# Used by: CM2100B
key-hover-press-the-meter-s-zero-key = マルチメーターの ZERO/ᛒ キーを短く押す
# Used by: OW18B/OW16B, OW18E
key-hover-rel-in-ac-and-hz-steps = REL (AC と Hz では Hz/Duty を順に切り替える)
# Used by: OW67B, OW69B, VC871, VC891, VC915, VC925 PV
key-hover-read-a-4-20-ma-loop = 4-20 mA ループ電流をパーセントで読み取る
# Used by: ZT-5566SE
key-hover-record-maximum-and-minimum-readings = 最大値と最小値を記録する
# Used by: UT171A/B/C
key-hover-record-maximum-average-and-minimum-readings-2 = 最大値、平均値、最小値を記録する (クリックで切り替え、× で終了)
# Used by: UT181A
key-hover-record-maximum-average-and-minimum-readings = 最大値、平均値、最小値を記録する (× で終了)
# Used by: VC-880, VC-890, VC650BT
key-hover-record-maximum-minimum-and-average-readings = 最大値、最小値、平均値を記録する (クリックで切り替え、× で終了)
# Used by: 121GW
key-hover-record-minimum-maximum-and-average-readings = 最小値、最大値、平均値を記録する (クリックで切り替え、× で終了)
# Used by: UT171A/B/C, UT181A
key-hover-save-the-reading-to-the-meter = 測定値をマルチメーターのメモリに保存する
# Used by: CMS061, CMS101
key-hover-send-select-switches-v-dc-to = Select を送信する (V,DC から V,AC に切り替わる)
# Used by: VC915
key-hover-show-3-phase-rotation-direction = 3 相の相回転方向を表示する
# Used by: VC915
key-hover-show-vac-or-vdc-on-the = サブ表示部に VAC または VDC を表示する
# Used by: 121GW
key-hover-step-through-hold-and-autohold = HOLD と AutoHOLD を順に切り替える
# Used by: VC915
key-hover-swap-hz-pct-and-voltage = Hz/% と電圧を入れ替える
# Used by: VC925 PV
key-hover-swap-the-main-and-second-displays = メイン表示部とサブ表示部を入れ替える
# Used by: OW69B
key-hover-switch-the-low-pass-filter-on = ローパスフィルターをオン/オフする
# Used by: UT171A/B/C
key-hover-take-the-reading-as-the-reference = 現在の測定値を基準値にする (× で終了)
# Used by: OW18B/OW16B, OW18E
key-hover-toggle-the-backlight-and-an-ow18 = バックライトと OW18 のライトをオン/オフする
# Used by: VC871, VC891, VC915, VC925 PV
key-hover-zero-the-reading-to-exit = 測定値をゼロにする (× で終了)

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
    注意: BM869s と BM867s では約 17 分、その他の機種では 30 分操作しないと、マルチメーターの電源が切れます。オートパワーオフを無効にするには、SELECT を押しながら電源を入れます。
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
# Used by: UT71A, UT71B, UT71C/D/E, VC920/VC940/VC960
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
    1. ほかのアプリとマルチメーターの接続をすべて切断します。
    2. ファンクションスイッチを Auto V/LoZ 以外の任意の測定機能に合わせます。
    3. ディスプレイに ((D)) が表示されるまで、Δ を 1 秒以上押し続けます。
    注意: 15 分から 30 分操作しないと、マルチメーターの電源が切れます。オートパワーオフを無効にするには、SELECT を押しながら電源を入れます (「dSAPO」と表示されます)。
    マルチメーターが見つからない場合や接続パスワードが変更された場合は、Hz ボタンを押しながら、ファンクションスイッチを 0.6 s 以内に OFF から静電容量の位置まで回します。マルチメーターに「Org」と表示され、接続パスワードと Bluetooth 名が工場出荷時の設定に戻ります。
# Used by: OW69B
activation-ow69b =
    1. ほかのアプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. Bluetooth アイコンが表示されるまで、上に BLE と印字された < キーを押し続けます。
# Used by: OW67B
activation-ow67b =
    1. ほかのアプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. Bluetooth アイコンが表示されるまで、上に BLE と印字された < キーを押し続けます。
    注意: 5 分間操作しないと Bluetooth がオフになります。もう一度 < を押し続けるとオンに戻ります。
# Used by: VC915, VC925 PV
activation-vc915 =
    1. ほかのアプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. ビープ音が鳴り、Bluetooth マークが表示されるまで、BLE ボタンを約 2 秒間押し続けます。
# Used by: VC871, VC891
activation-vc871 =
    1. ほかのアプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. ビープ音が鳴り、左上に Bluetooth マークが表示されるまで、BLE ボタンを約 2 秒間押し続けます。
    注意: マルチメーターの電源を入れるたびに Bluetooth はオフになるので、そのつど BLE を押し続けます。
# Used by: OW18B/OW16B, OW18E
activation-ow18b =
    1. ほかのアプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. ᛒ アイコンが表示されるまで、Hz/Duty△/ᛒ キーを押し続けます。
    注意: 10 分間操作しないと、ビープ音が 2 回鳴った後に Bluetooth がオフになります。もう一度 Hz/Duty△/ᛒ を押し続けるとオンに戻ります。
# Used by: OW65B
activation-ow65b =
    1. ほかのアプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. Bluetooth アイコンが表示されるまで、上に BLE と印字された SETUP キーを押し続けます。
    注意: もう一度 SETUP を押し続けると Bluetooth がオフになります。
# Used by: CMS061, CMS101
activation-cms061 =
    1. ほかのアプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. Bluetooth アイコンが表示されるまで、上に BLE と印字された Tab⇌ キーを押し続けます。
    注意: Bluetooth がオンの間は、マルチメーターの電源は自動で切れません。
# Used by: CM2100B
activation-cm2100b =
    1. ほかのアプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. ディスプレイの左上に ᛒ アイコンが表示されるまで、ZERO/ᛒ キーを約 2 秒間押し続けます。
    注意: 5 分間操作しないか、もう一度 ZERO/ᛒ を押し続けると、Bluetooth がオフになります。マルチメーターは約 15 分間操作しないと電源が切れます。次に電源を切るまでこの機能を無効にするには、SELECT を押しながら電源を入れます。
# Used by: B33
activation-b33 =
    1. ほかのアプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. ᛒ アイコンが表示されるまで、△/ᛒ キーを押し続けます。
    注意: 10 分間操作しないと、ビープ音が 2 回鳴った後に Bluetooth がオフになります。オフライン記録が終了したときもオフになります。もう一度 △/ᛒ を押し続けるとオンに戻ります。
# Used by: B35T+, B41T+
activation-b35t-plus =
    1. ほかのアプリとマルチメーターの接続をすべて切断します。
    2. マルチメーターの電源を入れます。
    3. ᛒ アイコンが表示されるまで、△/ᛒ キーを押し続けます。
    注意: 10 分間操作しないと、ビープ音が 2 回鳴った後に Bluetooth がオフになります。もう一度 △/ᛒ を押し続けるとオンに戻ります。
# Used by: ZT-5B
activation-zt5b =
    1. 赤いボタンを 2 秒間押し続けて、マルチメーターの電源を入れます。
    2. 赤いボタンを短く押します。Bluetooth マークが表示されます。
    注意: 15 分間操作しないと、マルチメーターの電源が切れます。オートパワーオフを無効にするには、電源を入れる前に NCV を押します。
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
    注意: 30 分経つとマルチメーターの電源が切れます。オートパワーオフを無効にするには、SETUP で APO.oF に設定します。
# Used by: ZT-300AB
activation-zt300ab =
    1. マルチメーターの電源を入れます。
    2. Hz% ボタンを 2 秒間押し続けます。Bluetooth マークが表示されます。
    注意: 15 分間操作しないと、マルチメーターの電源が切れます。オートパワーオフを無効にするには、SEL を押しながら電源を入れます。
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
    注意: 15 分間操作しないと、マルチメーターの電源が切れます。オートパワーオフを無効にするには、Hz/NCV を押しながら電源を入れます。
# Used by: UT202BT
activation-ut202bt =
    1. マルチメーターの電源を入れます。
    2. Bluetooth ボタンを短く押します。Bluetooth マークが点滅します。
    注意: 接続がないまま 5 分経つと、Bluetooth は自動でオフになります。
# Used by: Mock UT61E+, Mock ZT-5B
activation-mock = シミュレーション用のデバイスのため、設定は不要です。
