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
mode-a-ac-peak = A AC 峰值
# Used by: UT171A/B/C
mode-a-ac-plus-dc = A AC+DC
# Used by: UT171A/B/C, UT181A
mode-a-dc = A DC
# Used by: UT181A
mode-a-dc-ac-plus-dc = A DC AC+DC
# Used by: UT181A
mode-a-dc-peak = A DC 峰值
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
mode-ac-power-a = AC 功率 A
# Used by: 9 meters
mode-ac-power-ah = AC 功率 Ah
# Used by: 9 meters
mode-ac-power-v = AC 功率 V
# Used by: 9 meters
mode-ac-power-va = AC 功率 VA
# Used by: 9 meters
mode-ac-power-w = AC 功率 W
# Used by: 9 meters
mode-ac-power-wh = AC 功率 Wh
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
mode-auto = 自动
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-auto-v = 自动 V
# Used by: 47 meters
mode-capacitance = 电容
# Used by: UT8803
mode-capacitance-d = 电容 D
# Used by: UT8803
mode-capacitance-r = 电容 R
# Used by: 8 meters
mode-clamp-ac-a = 钳形 AC A
# Used by: 8 meters
mode-clamp-ac-plus-dc-a = 钳形 AC+DC A
# Used by: 8 meters
mode-clamp-dc-a = 钳形 DC A
# Used by: 8 meters
mode-clamp-lpf-a = 钳形 LPF A
# Used by: 9 meters
mode-compass = 指南针
# Used by: 46 meters
mode-continuity = 通断
# Used by: UT181A
mode-continuity-open = 通断 (开路)
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
mode-dc-power-a = DC 功率 A
# Used by: 9 meters
mode-dc-power-ah = DC 功率 Ah
# Used by: 9 meters
mode-dc-power-v = DC 功率 V
# Used by: 9 meters
mode-dc-power-va = DC 功率 VA
# Used by: 9 meters
mode-dc-power-w = DC 功率 W
# Used by: 9 meters
mode-dc-power-wh = DC 功率 Wh
# Used by: 28 meters
mode-dc-ua = DC µA
# Used by: 121GW
mode-dc-uva = DC µVA
# Used by: 46 meters
mode-diode = 二极管
# Used by: UT181A
mode-diode-alarm = 二极管报警
# Used by: 44 meters
mode-duty-pct = 占空比 %
# Used by: BM829s/BM827s/BM822s/BM821s
mode-ef = EF
# Used by: BM788BT/BM787BT
mode-ef-h = EF-H
# Used by: BM788BT/BM787BT
mode-ef-l = EF-L
# Used by: 9 meters
mode-frequency = 频率
# Used by: 9 meters
mode-hv-ac-v = HV AC V
# Used by: 9 meters
mode-hv-dc-v = HV DC V
# Used by: UT8803
mode-inductance = 电感
# Used by: UT8803
mode-inductance-q = 电感 Q
# Used by: UT8803
mode-inductance-r = 电感 R
# Used by: 22 meters
mode-inrush = 浪涌电流
# Used by: BM788BT/BM787BT
mode-line-hz = 市电 Hz
# Used by: 8 meters
mode-live = 火线
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
mode-logic-hz = 逻辑 Hz
# Used by: 9 meters
mode-motor = 电机
# Used by: 9 meters
mode-motor-1-2-3 = 电机 1-2-3
# Used by: 9 meters
mode-motor-3-2-1 = 电机 3-2-1
# Used by: 20 meters
mode-ncv = NCV
# Used by: UT71A, UT71B, UT71C/D/E, UT804, VC920/VC940/VC960
mode-power = 功率
# Used by: 9 meters
mode-power-a = 功率 A
# Used by: 9 meters
mode-power-ah = 功率 Ah
# Used by: 9 meters
mode-power-v = 功率 V
# Used by: 9 meters
mode-power-va = 功率 VA
# Used by: 9 meters
mode-power-w = 功率 W
# Used by: 9 meters
mode-power-wh = 功率 Wh
# Used by: 9 meters
mode-power-factor = 功率因数
# Used by: 121GW, UT181A
mode-pulse-width = 脉冲宽度
# Used by: BM525s/BM521s
mode-recall = 数据回读
# Used by: UT8802, UT8803
mode-scr = SCR
# Used by: 9 meters
mode-scope = 示波器
# Used by: 9 meters
mode-solar = 太阳能
# Used by: UT171A/B/C
mode-square-wave = 方波
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
mode-time = 时间
# Used by: 9 meters
mode-usb-power-a = USB 功率 A
# Used by: 9 meters
mode-usb-power-ah = USB 功率 Ah
# Used by: 9 meters
mode-usb-power-v = USB 功率 V
# Used by: 9 meters
mode-usb-power-va = USB 功率 VA
# Used by: 9 meters
mode-usb-power-w = USB 功率 W
# Used by: 9 meters
mode-usb-power-wh = USB 功率 Wh
# Used by: UT171A/B/C, UT181A
mode-v-ac = V AC
# Used by: UT181A
mode-v-ac-hz = V AC Hz
# Used by: UT181A
mode-v-ac-lpf = V AC LPF
# Used by: UT181A
mode-v-ac-peak = V AC 峰值
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
mode-v-dc-peak = V DC 峰值
# Used by: UT171A/B/C
mode-vfc = VFC
# Used by: BM788BT/BM787BT, BM869s/BM867s
mode-vfd-ac-v = VFD AC V
# Used by: BM788BT/BM787BT, BM869s/BM867s
mode-vfd-hz = VFD Hz
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-dbm-reference = dBm 参考阻抗
# Used by: 25 meters
mode-hfe = hFE
# Used by: UT171A/B/C, UT181A
mode-ma-ac = mA AC
# Used by: UT181A
mode-ma-ac-hz = mA AC Hz
# Used by: UT181A
mode-ma-ac-peak = mA AC 峰值
# Used by: UT171A/B/C
mode-ma-ac-plus-dc = mA AC+DC
# Used by: UT171A/B/C, UT181A
mode-ma-dc = mA DC
# Used by: UT181A
mode-ma-dc-ac-plus-dc = mA DC AC+DC
# Used by: UT181A
mode-ma-dc-peak = mA DC 峰值
# Used by: UT171A/B/C, UT181A
mode-mv-ac = mV AC
# Used by: UT181A
mode-mv-ac-ac-plus-dc = mV AC AC+DC
# Used by: UT181A
mode-mv-ac-hz = mV AC Hz
# Used by: UT181A
mode-mv-ac-peak = mV AC 峰值
# Used by: UT171A/B/C
mode-mv-ac-plus-dc = mV AC+DC
# Used by: UT171A/B/C, UT181A
mode-mv-dc = mV DC
# Used by: UT181A
mode-mv-dc-peak = mV DC 峰值
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
mode-ua-ac-peak = µA AC 峰值
# Used by: UT171A/B/C
mode-ua-ac-plus-dc = µA AC+DC
# Used by: UT171A/B/C, UT181A
mode-ua-dc = µA DC
# Used by: UT181A
mode-ua-dc-ac-plus-dc = µA DC AC+DC
# Used by: UT181A
mode-ua-dc-peak = µA DC 峰值

## Mode qualifiers: a word around a mode's name for a low-pass filter, a
## held peak or a relative reading, around any of the modes above. $mode is the
## mode, already translated: put it where your language puts it.

# Used by: 9 meters
# $mode is the mode, already translated.
qualifier-lpf-mode = LPF { $mode }
# Used by: 9 meters
# $mode is the mode, already translated.
qualifier-peak-mode = { $mode } 峰值
# Used by: UT181A
# $mode is the mode, already translated.
qualifier-mode-rel = { $mode } REL
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
# $mode is the mode, already translated.
qualifier-mode-peak = { $mode } 峰值

## Ranges: beside the reading and in the range menu. Ranges that are a
## number and a unit only are not listed.

# Used by: UT161E, UT61E+
range-1000beta = 1000β
# Used by: 12 meters
range-auto = 自动
# Used by: 7 meters
range-cont = 通断
# Used by: 7 meters
range-diode = 二极管
# Used by: 7 meters
range-duty = 占空比
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
aux-absolute = 绝对值
# Used by: 9 meters
aux-angle = 角度
# Used by: UT171A/B/C
aux-aux = 辅助
# Used by: UT181A
aux-average = 平均值
# Used by: 121GW
aux-battery = 电池
# Used by: 121GW
aux-burden-voltage = 负载电压
# Used by: UT181A
aux-comp-high = COMP 偏高
# Used by: UT181A
aux-comp-low = COMP 偏低
# Used by: 10 meters
aux-capacitance = 电容
# Used by: UT202BT
aux-clamp-ac-a = 钳形 AC A
# Used by: UT202BT
aux-clamp-lpf-a = 钳形 LPF A
# Used by: 9 meters
aux-compass = 指南针
# Used by: 10 meters
aux-continuity = 通断
# Used by: 121GW
aux-current = 电流
# Used by: 12 meters
aux-dc = DC
# Used by: 9 meters
aux-dc-a = DC A
# Used by: 10 meters
aux-dc-v = DC V
# Used by: 9 meters
aux-diode = 二极管
# Used by: 10 meters
aux-duty = 占空比
# Used by: 9 meters
aux-frequency = 频率
# Used by: 9 meters
aux-hv-ac-v = HV AC V
# Used by: 9 meters
aux-hv-dc-v = HV DC V
# Used by: UT181A
aux-impedance = 阻抗
# Used by: UT202BT
aux-inrush = 浪涌电流
# Used by: 121GW
aux-internal-temperature = 内部温度
# Used by: UT202BT
aux-lpf-v = LPF V
# Used by: VC871
aux-live = 火线
# Used by: UT181A
aux-max = 最大值
# Used by: UT181A
aux-min = 最小值
# Used by: 9 meters
aux-motor = 电机
# Used by: CMS061, CMS101
aux-ncv = NCV
# Used by: 9 meters
aux-pf = PF
# Used by: UT181A
aux-peak-min = 峰值最小
# Used by: Mock UT61E+, UT181A
aux-period = 周期
# Used by: UT171A/B/C
aux-pulse-width = 脉冲宽度
# Used by: UT181A, VC871
aux-reference = 参考值
# Used by: 9 meters
aux-solar = 太阳能
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, Mock UT61E+, UT181A
aux-t1 = T1
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s, Mock UT61E+, UT181A
aux-t2 = T2
# Used by: UT202BT
aux-temperature = 温度
# Used by: 9 meters
aux-time = 时间
# Used by: 121GW, UT181A
aux-voltage = 电压
# Used by: 9 meters
aux-hfe = hFE

## On and off, for a meter setting offered as a choice.

# Used by: 13 meters
toggle-off = 关
# Used by: 13 meters
toggle-on = 开

## Meter keys named in words for what they do. A name printed on a key is
## not listed here: it stays as printed.

# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-auto-function = 自动功能
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-capacitance = 电容
# Used by: ZT-300AB, ZT-5566SE, ZT-5BQ
key-current = 电流
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-diode-continuity = 二极管/通断

## Meter key hovers: how to do on the meter what a remote button does.
## Keep the meter's key names as it prints them.

# Used by: VC891, VC915
key-hover-1-khz-low-pass-filter = 1 kHz 低通滤波
# Used by: 121GW
key-hover-capture-1-ms-peaks-in-ac = 在 AC V 模式下捕捉 1 ms 峰值
# Used by: OW67B, VC871
key-hover-change-what-the-displays-show = 切换显示屏的内容
# Used by: VC915, VC925 PV
key-hover-compare-the-reading-with-the-set = 将读数与设定的上下限比较
# Used by: CMS061, CMS101
key-hover-enter-max-min-click-again-to = 进入 MAX/MIN，再次单击退出
# Used by: 121GW
key-hover-hold-the-meter-s-rel-key = 长按万用表的 REL 键 (1 kHz 低通滤波)
# Used by: CMS061, CMS101
key-hover-measure-inrush-current-click-again-to = 测量浪涌电流，再次单击退出
# Used by: UT171A/B/C
key-hover-peak-hold-in-ac-v-and = AC V 和 AC A 模式下的峰值保持: 单击进入或退出
# Used by: OW18B/OW16B, OW18E
key-hover-press-the-meter-s-hz-duty = 短按万用表的 Hz/Duty△/ᛒ 键
# Used by: CM2100B
key-hover-press-the-meter-s-zero-key = 短按万用表的 ZERO/ᛒ 键
# Used by: OW18B/OW16B, OW18E
key-hover-rel-in-ac-and-hz-steps = 执行 REL，在 AC 和 Hz 模式下改为切换 Hz/Duty
# Used by: OW67B, OW69B, VC871, VC891, VC915, VC925 PV
key-hover-read-a-4-20-ma-loop = 以百分比读取 4-20 mA 环路电流
# Used by: ZT-5566SE
key-hover-record-maximum-and-minimum-readings = 记录最大值和最小值
# Used by: UT171A/B/C
key-hover-record-maximum-average-and-minimum-readings-2 = 记录最大值、平均值和最小值: 单击循环切换，单击 × 退出
# Used by: UT181A
key-hover-record-maximum-average-and-minimum-readings = 记录最大值、平均值和最小值: 单击 × 退出
# Used by: VC-880, VC-890, VC650BT
key-hover-record-maximum-minimum-and-average-readings = 记录最大值、最小值和平均值: 单击循环切换，单击 × 退出
# Used by: 121GW
key-hover-record-minimum-maximum-and-average-readings = 记录最小值、最大值和平均值: 单击循环切换，单击 × 退出
# Used by: UT171A/B/C, UT181A
key-hover-save-the-reading-to-the-meter = 将读数保存到万用表的存储器
# Used by: CMS061, CMS101
key-hover-send-select-switches-v-dc-to = 发送 Select 按键命令，将 V,DC 切换为 V,AC
# Used by: VC915
key-hover-show-3-phase-rotation-direction = 显示三相相序
# Used by: VC915
key-hover-show-vac-or-vdc-on-the = 在副显示屏上显示 VAC 或 VDC
# Used by: 121GW
key-hover-step-through-hold-and-autohold = 在 HOLD 和 AutoHOLD 之间循环切换
# Used by: VC915
key-hover-swap-hz-pct-and-voltage = 交换 Hz/% 和电压的显示位置
# Used by: VC925 PV
key-hover-swap-the-main-and-second-displays = 交换主显示屏和副显示屏的内容
# Used by: OW69B
key-hover-switch-the-low-pass-filter-on = 开启或关闭低通滤波器
# Used by: UT171A/B/C
key-hover-take-the-reading-as-the-reference = 将当前读数作为参考值: 单击 × 退出
# Used by: OW18B/OW16B, OW18E
key-hover-toggle-the-backlight-and-an-ow18 = 开启/关闭背光和 OW18 的手电筒
# Used by: VC871, VC891, VC915, VC925 PV
key-hover-zero-the-reading-to-exit = 将读数清零: 单击 × 退出

## Model names with words in them: the simulated meters. Keep the part
## number as it is.

# Used by: Mock UT61E+
model-mock-ut61e-plus = UT61E+ 仿真
# Used by: Mock ZT-5B
model-mock-zt-5b = ZT-5B 仿真

## Activation steps: how to switch a meter's data transmission on, shown
## when a connection gets no data. Keep each step on its own line, and the
## meter's key names, menu items and display text as the meter prints them.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
activation-bm52x =
    1. 将 BU-86X 连接线接到万用表背面的光学 PC-Comm 端口，再将另一端插入电脑
    2. 将旋钮转到电压功能档开机
    注意: BM869s 或 BM867s 万用表空闲约 17 分钟后自动关机，其他型号为 30 分钟；按住 SELECT 键的同时开机可禁用自动关机。
# Used by: UT8802, UT8803
activation-ut8802 =
    1. 将 USB 连接线插入万用表
    2. 打开万用表
# Used by: UT171A/B/C
activation-ut171 =
    1. 将 USB 连接线插入万用表
    2. 打开万用表
    3. 进入 SETUP -> Communication -> ON
# Used by: UT181A
activation-ut181a =
    1. 将 USB 连接线插入万用表
    2. 打开万用表
    3. 进入 SETUP -> Communication -> ON
    注意: 该设置在断电重启后复位。
# Used by: UT71A, UT71B, UT71C/D/E, VC920/VC940/VC960
activation-ut71ab =
    1. 将 USB 连接线插入万用表
    2. 打开万用表
    3. 长按 MAX MIN 键 1 s (UT71A 则按 SEND 键)，显示屏上显示 SEND
    注意: 按 EXIT 键关闭 SEND。
# Used by: UT803
activation-ut803 =
    1. 将 USB 连接线插入万用表
    2. 打开万用表
    3. 按 RS232 键，显示屏上显示 RS232
# Used by: UT804
activation-ut804 =
    1. 将 USB 连接线插入万用表
    2. 打开万用表
    3. 按 SEND 键，显示屏上显示 SEND
    注意: 按 HOLD 键可暂停数据发送，按 EXIT 键可关闭 SEND。
# Used by: VC-880, VC-890, VC650BT
activation-vc650bt =
    1. 将 USB 连接线插入万用表
    2. 打开万用表
    3. 按万用表上的 PC 键
# Used by: BM788BT/BM787BT
activation-bm78xbt =
    1. 断开万用表与其他应用的连接
    2. 将旋钮转到除 Auto V/LoZ 以外的任意功能档
    3. 长按 Δ 键至少 1 秒，直到显示屏上显示 ((D))
    注意: 万用表空闲 15 到 30 分钟后自动关机；按住 SELECT 键的同时开机可禁用自动关机 (显示 dSAPO)。
    如果找不到万用表，或连接密码已更改，请按住 Hz 键，同时在 0.6 s 内将旋钮从 OFF 转到电容档: 万用表显示 "Org"，连接密码和蓝牙名称恢复为出厂设置。
# Used by: OW69B
activation-ow69b =
    1. 断开万用表与其他应用的连接
    2. 打开万用表
    3. 按住 < 键 (上方印有 BLE)，直到显示蓝牙图标
# Used by: OW67B
activation-ow67b =
    1. 断开万用表与其他应用的连接
    2. 打开万用表
    3. 按住 < 键 (上方印有 BLE)，直到显示蓝牙图标
    注意: 蓝牙在空闲 5 分钟后关闭；再次按住 < 键可重新打开。
# Used by: VC915, VC925 PV
activation-vc915 =
    1. 断开万用表与其他应用的连接
    2. 打开万用表
    3. 按住 BLE 键约 2 秒，直到万用表发出蜂鸣声并显示蓝牙符号
# Used by: VC871, VC891
activation-vc871 =
    1. 断开万用表与其他应用的连接
    2. 打开万用表
    3. 按住 BLE 键约 2 秒，直到万用表发出蜂鸣声并在左上角显示蓝牙符号
    注意: 每次开机后蓝牙都处于关闭状态，需再次按住 BLE 键打开。
# Used by: OW18B/OW16B, OW18E
activation-ow18b =
    1. 断开万用表与其他应用的连接
    2. 打开万用表
    3. 按住 Hz/Duty△/ᛒ 键，直到显示 ᛒ 图标
    注意: 空闲 10 分钟后，万用表发出两声蜂鸣，蓝牙随即关闭；再次按住 Hz/Duty△/ᛒ 键可重新打开。
# Used by: OW65B
activation-ow65b =
    1. 断开万用表与其他应用的连接
    2. 打开万用表
    3. 按住 SETUP 键 (上方印有 BLE)，直到显示蓝牙图标
    注意: 再次按住 SETUP 键可关闭蓝牙。
# Used by: CMS061, CMS101
activation-cms061 =
    1. 断开万用表与其他应用的连接
    2. 打开万用表
    3. 按住 Tab⇌ 键 (上方印有 BLE)，直到显示蓝牙图标
    注意: 蓝牙打开期间，万用表不会自动关机。
# Used by: CM2100B
activation-cm2100b =
    1. 断开万用表与其他应用的连接
    2. 打开万用表
    3. 按住 ZERO/ᛒ 键约 2 秒，直到显示屏左上角显示 ᛒ 图标
    注意: 蓝牙在空闲 5 分钟后关闭，再次按住 ZERO/ᛒ 键时也会关闭。万用表空闲约 15 分钟后自动关机；按住 SELECT 键的同时开机，可禁用自动关机，直到下次关机。
# Used by: B33
activation-b33 =
    1. 断开万用表与其他应用的连接
    2. 打开万用表
    3. 按住 △/ᛒ 键，直到显示 ᛒ 图标
    注意: 空闲 10 分钟后，万用表发出两声蜂鸣，蓝牙随即关闭；离线记录结束时蓝牙也会关闭；再次按住 △/ᛒ 键可重新打开。
# Used by: B35T+, B41T+
activation-b35t-plus =
    1. 断开万用表与其他应用的连接
    2. 打开万用表
    3. 按住 △/ᛒ 键，直到显示 ᛒ 图标
    注意: 空闲 10 分钟后，万用表发出两声蜂鸣，蓝牙随即关闭；再次按住 △/ᛒ 键可重新打开。
# Used by: ZT-5B
activation-zt5b =
    1. 按住红色按键 2 秒开机
    2. 短按红色按键，显示蓝牙符号
    注意: 万用表空闲 15 分钟后自动关机；开机前按 NCV 键可禁用自动关机。
# Used by: UT161B, UT161D, UT161E, UT61B+, UT61D+, UT61E+
activation-ut161b =
    1. 将 USB 模块插入万用表
    2. 打开万用表
    3. 长按 USB/Hz 键
    4. LCD 上出现 S 图标
# Used by: 121GW
activation-121gw =
    1. 将旋钮转到任意功能档
    2. 按住 1ms PEAK 键，直到显示屏上显示 BT
    注意: 万用表 30 分钟后自动关机；在 SETUP 中设置 APO.oF 可禁用自动关机。
# Used by: ZT-300AB
activation-zt300ab =
    1. 打开万用表
    2. 按住 Hz% 键 2 秒，显示蓝牙符号
    注意: 万用表空闲 15 分钟后自动关机；按住 SEL 键的同时开机可禁用自动关机。
# Used by: UT60BT
activation-ut60bt =
    1. 打开万用表
    2. 长按 SEL 键，直到显示蓝牙符号
# Used by: ZT-5566SE
activation-zt5566se =
    1. 打开万用表
    2. 按 POWER 键打开蓝牙，蓝牙符号闪烁
    注意: 手册只说明了扬声器的蓝牙功能；通过蓝牙获取的读数未经验证。
# Used by: ZT-5BQ
activation-zt5bq =
    1. 打开万用表
    2. 同时按 Power 键和 Hz 键，显示蓝牙符号
    注意: 万用表空闲 15 分钟后自动关机；按住 Hz/NCV 键的同时开机可禁用自动关机。
# Used by: UT202BT
activation-ut202bt =
    1. 打开万用表
    2. 短按蓝牙键，蓝牙符号闪烁
    注意: 蓝牙在 5 分钟内无连接时自动关闭。
# Used by: Mock UT61E+, Mock ZT-5B
activation-mock = 无需设置，这是一台仿真设备。
