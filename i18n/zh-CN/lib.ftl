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
mode-a-ac-peak = A AC 峰值
# Used by: UT171A/B/C
mode-a-ac-plus-dc = A AC+DC
# Used by: UT171A/B/C, UT181A
mode-a-dc = A DC
# Used by: UT181A
mode-a-dc-ac-plus-dc = A DC AC+DC
# Used by: UT181A
mode-a-dc-peak = A DC 峰值
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
mode-auto = 自动
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s
mode-auto-v = 自动 V
# Used by: 46 meters
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
# Used by: 45 meters
mode-continuity = 通断
# Used by: UT181A
mode-continuity-open = 通断 (开路)
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
# Used by: 27 meters
mode-dc-ua = DC µA
# Used by: 121GW
mode-dc-uva = DC µVA
# Used by: 45 meters
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
# Used by: 8 meters
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
# Used by: UT71A/B, UT71C/D/E, UT804, VC920/VC940/VC960
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
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1 = T1
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t1-t2 = T1-T2
# Used by: BM525s/BM521s, BM788BT/BM787BT, BM829s/BM827s/BM822s/BM821s, BM869s/BM867s
mode-t2 = T2
# Used by: UT803
mode-tachometer = 转速表
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

## Meter keys: the remote panel's buttons, named after the meter's keys.
## Keep a name the meter prints on its key (SEL, ZERO) as it is.

# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-auto-function = 自动功能
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-capacitance = 电容
# Used by: ZT-300AB, ZT-5566SE, ZT-5BQ
key-current = 电流
# Used by: Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-diode-continuity = 二极管/通断
# Used by: 9 meters
key-hz-duty = Hz/Duty
# Used by: Mock ZT-5B, ZT-5566SE, ZT-5B, ZT-5BQ
key-ncv = NCV
# Used by: CM2100B, Mock ZT-5B, ZT-300AB, ZT-5566SE, ZT-5B, ZT-5BQ
key-zero = ZERO

## Meter key hovers: how to do on the meter what a remote button does.
## Keep the meter's key names as it prints them.

# Used by: 121GW
key-hover-hold-the-meter-s-rel-key = 长按万用表的 REL 键 (1 kHz 低通滤波)
# Used by: OW18B/OW16B, OW18E
key-hover-press-the-meter-s-hz-duty = 短按万用表的 Hz/Duty△/ᛒ 键
# Used by: CM2100B
key-hover-press-the-meter-s-zero-key = 短按万用表的 ZERO/ᛒ 键

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
# Used by: UT71A/B, UT71C/D/E, VC920/VC940/VC960
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
    1. 断开万用表与手机应用的连接
    2. 将旋钮转到除 Auto V/LoZ 以外的任意功能档
    3. 长按 Δ 键至少 1 秒，直到显示屏上显示 ((D))
    注意: 万用表空闲 15 到 30 分钟后自动关机；按住 SELECT 键的同时开机可禁用自动关机 (显示 dSAPO)。
    如果找不到万用表，或其连接密码已被更改，请按住 Hz 键，同时在 0.6 s 内将旋钮从 OFF 转到电容档: 万用表显示 "Org"，连接密码和蓝牙名称恢复为出厂设置。
# Used by: OW69B
activation-ow69b =
    1. 断开万用表与手机应用的连接
    2. 打开万用表
    3. 按住 < 键 (上方印有 BLE)，直到显示蓝牙图标
# Used by: OW67B
activation-ow67b =
    1. 断开万用表与手机应用的连接
    2. 打开万用表
    3. 按住 < 键 (上方印有 BLE)，直到显示蓝牙图标
    注意: 蓝牙在空闲 5 分钟后关闭；再次按住 < 键可重新打开。
# Used by: VC915, VC925 PV
activation-vc915 =
    1. 断开万用表与手机应用的连接
    2. 打开万用表
    3. 按住 BLE 键约 2 秒，直到万用表发出蜂鸣声并显示蓝牙符号
# Used by: VC871, VC891
activation-vc871 =
    1. 断开万用表与手机应用的连接
    2. 打开万用表
    3. 按住 BLE 键约 2 秒，直到万用表发出蜂鸣声并在左上角显示蓝牙符号
    注意: 每次开机后蓝牙都处于关闭状态，需再次按住 BLE 键打开。
# Used by: OW18B/OW16B, OW18E
activation-ow18b =
    1. 断开万用表与手机应用的连接
    2. 打开万用表
    3. 按住 Hz/Duty△/ᛒ 键，直到显示 ᛒ 图标
    注意: 空闲 10 分钟后，万用表发出两声蜂鸣，蓝牙随即关闭；再次按住 Hz/Duty△/ᛒ 键可重新打开。
# Used by: OW65B
activation-ow65b =
    1. 断开万用表与手机应用的连接
    2. 打开万用表
    3. 按住 SETUP 键 (上方印有 BLE)，直到显示蓝牙图标
    注意: 再次按住 SETUP 键可关闭蓝牙。
# Used by: CMS061, CMS101
activation-cms061 =
    1. 断开万用表与手机应用的连接
    2. 打开万用表
    3. 按住 Tab⇌ 键 (上方印有 BLE)，直到显示蓝牙图标
    注意: 蓝牙打开期间，万用表不会自动关机。
# Used by: CM2100B
activation-cm2100b =
    1. 断开万用表与手机应用的连接
    2. 打开万用表
    3. 按住 ZERO/ᛒ 键约 2 秒，直到显示屏左上角显示 ᛒ 图标
    注意: 蓝牙在空闲 5 分钟后关闭，再次按住 ZERO/ᛒ 键时也会关闭。万用表空闲约 15 分钟后自动关机；按住 SELECT 键的同时开机，可禁用自动关机，直到下次关机。
# Used by: B33
activation-b33 =
    1. 断开万用表与手机应用的连接
    2. 打开万用表
    3. 按住 △/ᛒ 键，直到显示 ᛒ 图标
    注意: 空闲 10 分钟后，万用表发出两声蜂鸣，蓝牙随即关闭；离线记录结束时蓝牙也会关闭；再次按住 △/ᛒ 键可重新打开。
# Used by: B35T+, B41T+
activation-b35t-plus =
    1. 断开万用表与手机应用的连接
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

## Specification table names, in the Specifications panel, as the
## meter's manual titles them. Keep the manual's numbering: (1), A.

# Used by: UT181A
table-1-ac-voltage = (1) 交流电压
# Used by: UT181A
table-10-temperature = (10) 温度
# Used by: UT181A
table-11-frequency = (11) 频率
# Used by: UT181A
table-12-duty-cycle = (12) 占空比
# Used by: UT181A
table-13-pulse-width = (13) 脉冲宽度
# Used by: UT181A
table-14-continuity-test = (14) 通断测试
# Used by: UT181A
table-15-diode-test = (15) 二极管测试
# Used by: UT181A
table-2-dc-voltage = (2) 直流电压
# Used by: UT181A
table-3-ac-voltage-plus-dc-voltage = (3) 交流+直流电压
# Used by: UT181A
table-4-ac-current = (4) 交流电流
# Used by: UT181A
table-5-dc-current = (5) 直流电流
# Used by: UT181A
table-6-ac-current-plus-dc-current = (6) 交流+直流电流
# Used by: UT181A
table-7-resistance = (7) 电阻
# Used by: UT181A
table-8-conductance = (8) 电导
# Used by: UT181A
table-9-capacitance = (9) 电容
# Used by: UT803, UT804
table-a-dc-voltage = A. 直流电压
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-ac-ac-plus-dc-current = 交流和交流+直流电流
# Used by: 7 meters
table-ac-current = 交流电流
# Used by: 9 meters
table-ac-voltage = 交流电压
# Used by: Mock ZT-5B, ZT-5B
table-ac-voltage-v = 交流电压 (V)
# Used by: Mock ZT-5B, ZT-5B
table-ac-current-a = 交流电流 (A)
# Used by: Mock ZT-5B, ZT-5B
table-ac-current-ma = 交流电流 (mA)
# Used by: Mock UT61E+, UT161E, UT61E+
table-ac-plus-dc-voltage = 交流+直流电压
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-audible-continuity-tester = 蜂鸣通断测试
# Used by: Mock ZT-5B, ZT-5B
table-auto = 自动
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-acv = AutoCheck™ (ACV)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-dcv = AutoCheck™ (DCV)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-autocheck-ohms = AutoCheck™ (Ohms)
# Used by: UT803
table-b-ac-voltage = B. 交流电压
# Used by: UT804
table-b-ac-voltage-ac-plus-dc = B. 交流电压 (可测量交流+直流)
# Used by: UT803, UT804
table-c-dc-current = C. 直流电流
# Used by: 11 meters
table-capacitance = 电容
# Used by: Mock ZT-5B, ZT-5B
table-continuity = 通断
# Used by: 7 meters
table-continuity-and-diode = 通断和二极管
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-crest-mode-instantaneous-peak-hold = 波峰模式 (瞬时峰值保持)
# Used by: UT803
table-d-ac-current = D. 交流电流
# Used by: UT804
table-d-ac-current-ac-plus-dc = D. 交流电流 (可测量交流+直流)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-dc-ac-ac-plus-dc-ac = 直流电压、交流电压和交流+直流电压
# Used by: 9 meters
table-dc-current = 直流电流
# Used by: Mock ZT-5B, ZT-5B
table-dc-voltage-v = 直流电压 (V)
# Used by: 9 meters
table-dc-voltage = 直流电压
# Used by: Mock ZT-5B, ZT-5B
table-dc-current-a = 直流电流 (A)
# Used by: Mock ZT-5B, ZT-5B
table-dc-current-ma = 直流电流 (mA)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-diode-tester = 二极管测试
# Used by: UT803, UT804
table-e-resistance = E. 电阻
# Used by: UT803, UT804
table-f-continuity-test = F. 通断测试
# Used by: Mock ZT-5B, ZT-5B
table-frequency = 频率
# Used by: 7 meters
table-frequency-duty-ratio = 频率/占空比
# Used by: UT804
table-g-diode-test = G. 二极管测试
# Used by: UT803
table-g-diode-testst = G. 二极管测试
# Used by: UT803, UT804
table-h-capacitance = H. 电容
# Used by: UT803, UT804
table-i-frequency = I. 频率
# Used by: UT804
table-j-duty-cycle = J. 占空比
# Used by: UT803
table-j-temperature = J. 温度
# Used by: UT804
table-k-temperature-1-1-degrees-celsius = K. 温度 1-1. 摄氏度
# Used by: UT804
table-k-temperature-1-2-fahrenheit = K. 温度 1-2. 华氏度
# Used by: UT803
table-k-transistor = K. 三极管
# Used by: UT804
table-l-4-20-ma-loop-current = L. 4~20 mA 环路电流
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-line-level-frequency-hz = 市电频率 (~Hz)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-logic-level-frequency-hz-duty-cycle = 逻辑电平频率 (⎍Hz) 和占空比 (D%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-non-contact-ef-detection = 非接触电场检测
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-ohms = 电阻
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
table-record-mode = 记录模式
# Used by: 9 meters
table-resistance = 电阻
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, Mock ZT-5B, UT161D, UT61D+, ZT-5B
table-temperature = 温度
# Used by: Mock UT61E+, UT161E, UT61E+
table-transistor-magnification = 三极管放大倍数 (hFE)

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
band-in-rel-state = REL 状态下
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); I. Frequency
band-not-specified = 未指定
# Used by: UT181A, UT804
# In: (9) Capacitance; H. Capacitance
band-not-specified-2 = 未指定
# Used by: UT181A
# In: (1) AC Voltage; (3) AC Voltage + DC Voltage
band-only-for-reference = 仅供参考
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage; DC AC & AC+DC AC Voltage
band-unspec-d = 未指定
# Used by: UT804
# In: A. DC Voltage
band-under-rel-mode = REL 模式下

## Specification inputs: a table's input impedance or overload
## protection. Keep every number, sign and ± exactly; a decimal comma is fine
## (0,6). A translation that changes, drops or adds a number is shown in
## English.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: 11 tables
input-1000v-dc-ac-rms = 1000V DC/AC rms
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: 4 tables
input-10mohm-50pf-nominal = 标称 10MΩ，50pF
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage; DC AC & AC+DC AC Voltage; DC Voltage
input-10mohm-80pf-nominal = 标称 10MΩ，80pF
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: 6 tables
input-1100v-dc-ac-rms = 1100V DC/AC rms
# Used by: UT803
# In: 6 tables
input-250v-rms = 250V rms
# Used by: 8 meters
# In: 6 tables
input-about-10mohm = 约 10MΩ
# Used by: 7 meters
# In: DC Voltage
input-about-1gohm = 约 1GΩ
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
input-approx-10mohm = 约 10MΩ
# Used by: UT803, UT804
# In: A. DC Voltage; B. AC Voltage
input-around-10mohm = 约 10MΩ
# Used by: UT804
# In: A. DC Voltage
input-around-2-5gohm = 约 2.5GΩ
# Used by: UT803
# In: A. DC Voltage; B. AC Voltage
input-around-3000mohm = 约 > 3000MΩ
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current; Line Level Frequency (~Hz)
input-fuse-0-44a-1000v = 保险丝 0.44A 1000V
# Used by: UT804
# In: C. DC Current; D. AC Current (AC+DC measurement is available); L. 4~20 mA loop current
input-fuse-0-5a-250v = 保险丝 0.5A 250V
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
input-fuse-0-8a-1000v = 保险丝 0.8A 1000V
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
input-fuse-10a-1000v = 保险丝 10A 1000V
# Used by: Mock UT61E+, UT61B+, UT61D+, UT61E+
# In: AC Current; DC Current
input-fuse-10a-240v = 保险丝 10A 240V
# Used by: UT803, UT804
# In: C. DC Current; D. AC Current; D. AC Current (AC+DC measurement is available)
input-fuse-10a-250v = 保险丝 10A 250V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s, UT161B, UT161D, UT161E
# In: 4 tables
input-fuse-11a-1000v = 保险丝 11A 1000V
# Used by: Mock UT61E+, UT61B+, UT61D+, UT61E+
# In: AC Current; DC Current
input-fuse-1a-240v = 保险丝 1A 240V
# Used by: UT803
# In: K. Transistor
input-fuse-200ma-250v-fuse-500ma-125v = 保险丝 200mA 250V；保险丝 500mA 125V
# Used by: UT803
# In: C. DC Current; D. AC Current
input-fuse-500ma-125v = 保险丝 500mA 125V
# Used by: UT161B, UT161D, UT161E
# In: AC Current; DC Current
input-fuse-600ma-1000v = 保险丝 600mA 1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
input-initially-approx-3-0kohm-150pf-nominal = 初始约 3.0kΩ，标称 150pF
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
input-initially-approx-3-0kohm-165pf-nominal = 初始约 3.0kΩ，标称 165pF

## Specification notes, under a table in the Specifications panel, from
## the meter's manual. Keep the exact sense, comparisons and qualifiers such as
## "typical" included. Keep every number, sign and ± exactly; a decimal comma
## is fine (0,6). A translation that changes, drops or adds a number is shown
## in English.

# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-10a-continuous-10a-to-20a-30s = 10A 可连续使用；>10A 到 20A: 最长 30s，需冷却 5 min
# Used by: UT804
# In: C. DC Current; D. AC Current (AC+DC measurement is available)
note-10a-5a-continuous-5a-10a-10s = 10A: ≤5A 可连续使用；>5A–10A 每次 ≤10s，间隔 >15min
# Used by: UT181A
# In: (13) Pulse Width
note-10hz-200khz-pulse-width-2us-depends = 10Hz–200kHz: 脉冲宽度 >2µs，取决于信号频率
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
note-20a-30s-on-then-10min-off = 20A: 连续 30s，然后停 10min；10A 以上未指定
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-220mf-range-measurement-takes-20s = 220mF 量程: 测量耗时 ≈20s
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-25-00mf-manual-range-not-specified = 25.00mF，手动量程: 5.4mF 以下未指定
# Used by: UT804
# In: E. Resistance
note-400ohm-accuracy-plus-test-lead-open = 400Ω: 准确度 + 表笔开路值
# Used by: UT804
# In: H. Capacitance
note-40nf-accuracy-plus-open-circuit-test = 40nF: 准确度 + 表笔开路电容
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-0-pct-20ma-shows = 4mA 显示 0%，20mA 显示 100%
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-6-000mf-manual-range-not-specified = 6.000mF，手动量程: 0.54mF 以下未指定
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms)
note-60-00mohm-tempco-0-6-accuracy = 60.00MΩ: 温度系数 0.6× 准确度/°C @(0°C ~ 18°C 或 28°C ~ 45°C)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-60-00mv-600-0mv-1khz-20khz = 60.00mV，600.0mV，1kHz ~ 20kHz: 指定范围为量程的 30%–100%
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage
note-60-00mv-600-0mv-3khz-20khz = 60.00mV，600.0mV，3kHz ~ 20kHz: 指定范围为量程的 30%–100%
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-600-0uf-manual-range-not-specified = 600.0µF，手动量程: 50.0µF 以下未指定
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-600ohm-ref-typical-acmv-42-22 = 600Ω 参考，典型值: ACmV -42.22 到 -02.22dBm，ACV -17.78 到 62.22dBm
# Used by: UT803
# In: E. Resistance
note-600ohm-accuracy-plus-test-lead-short = 600Ω: 准确度 + 表笔短路电阻
# Used by: UT181A
# In: (7) Resistance
note-60mohm-humidity-50-pct = 60MΩ: 湿度 <50%
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Capacitance
note-60mf-range-measurement-takes-20s = 60mF 量程: 测量耗时 ≈20s
# Used by: UT181A
# In: (2) DC Voltage
note-60mv-use-rel-to-compensate-bias = 60mV: 使用 REL 补偿偏置电压
# Used by: UT803
# In: H. Capacitance
note-6nf-60nf-600nf-subtract-open-circuit = 6nF，60nF，600nF: 减去表笔开路电容
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-9-999v-1khz-20khz-specified-for = 9.999V，1kHz ~ 20kHz: 仅在 1kHz ~ 15kHz 范围内指定
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC Voltage
note-9-999v-3khz-20khz-specified-for = 9.999V，3kHz ~ 20kHz: 仅在 3kHz ~ 15kHz 范围内指定
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-lo-20ma-shows-hi = <4mA 显示 LO，>20mA 显示 HI
# Used by: 7 meters
# In: AC Current; DC Current
note-5a-max-10s-rest-15-min = >5A: 最长 10s，之后间隔 ≥15 min
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-10-00a-15-00-3 = AC 10.00A: 15.00 ~ 3.000kHz，灵敏度 6A (正弦波有效值)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-6-000a-15-00-3 = AC 6.000A: 15.00 ~ 3.000kHz，灵敏度 4A (正弦波有效值)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-60-00ma-15-00-3 = AC 60.00mA: 15.00 ~ 3.000kHz，灵敏度 40mA (正弦波有效值)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-60-00mv-15-00-50 = AC 60.00mV: 15.00 ~ 50.00kHz，灵敏度 40mV (正弦波有效值)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0ma-15-00-3 = AC 600.0mA: 15.00 ~ 3.000kHz，灵敏度 60mA (正弦波有效值)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0mv-15-00-50 = AC 600.0mV: 15.00 ~ 50.00kHz，灵敏度 60mV (正弦波有效值)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-600-0ua-15-00-3 = AC 600.0µA: 15.00 ~ 3.000kHz，灵敏度 200µA (正弦波有效值)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-6000ua-15-00-3-000khz = AC 6000µA: 15.00 ~ 3.000kHz，灵敏度 600µA (正弦波有效值)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-9-999v-15-00-10 = AC 9.999V: 15.00 ~ 10.00kHz，灵敏度 2.5V (正弦波有效值)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-99-99v-15-00-10 = AC 99.99V: 15.00 ~ 10.00kHz，灵敏度 25V (正弦波有效值)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Line Level Frequency (~Hz)
note-ac-999-9v-15-00-10 = AC 999.9V: 15.00 ~ 10.00kHz，灵敏度 100V (正弦波有效值)
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-ac-plus-dc-add-1-pct = AC+DC: 准确度加 1%+35 个字
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-ac-plus-dc-range-accuracy-plus = AC+DC: 量程准确度 + 1%
# Used by: UT804
# In: K. Temperature 1-1. Degrees Celsius; K. Temperature 1-2. Fahrenheit
note-above-230degc-use-the-rod-contact = 230°C 以上: 使用杆式接触探头
# Used by: UT803
# In: J. Temperature
note-above-230degc-use-the-rod-type = 230°C 以上: 使用杆式探头
# Used by: UT181A
# In: (10) Temperature
note-accessory-point-contact-probe-below-230degc = 配件点式探头: 仅限 230°C 以下
# Used by: 7 meters
# In: 4 tables
note-accuracy-valid-1-pct-100-pct = 准确度有效范围: 量程的 1%–100%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-accuracy-valid-1-pct-100-pct-2 = 准确度有效范围: 量程的 1%–100% (1kHz–10kHz: 10%–100%)
# Used by: UT161D, UT61D+
# In: AC Current
note-accuracy-valid-40hz-1khz-1-pct = 准确度有效范围: 40Hz–1kHz，量程的 1%–100% (600.0µA: 5%–100%)
# Used by: UT161D, UT61D+
# In: AC Voltage
note-accuracy-valid-40hz-1khz-1-pct-2 = 准确度有效范围: 40Hz–1kHz，量程的 1%–100% (60mV: 2%–100%)
# Used by: UT161B, UT61B+
# In: AC Current
note-accuracy-valid-40hz-500hz-1-pct = 准确度有效范围: 40Hz–500Hz，量程的 1%–100% (600.0µA: 5%–100%)
# Used by: UT161B, UT61B+
# In: AC Voltage
note-accuracy-valid-40hz-500hz-1-pct-2 = 准确度有效范围: 40Hz–500Hz，量程的 1%–100% (60mV: 2%–100%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-accuracy-with-film-capacitor-or-better = 准确度适用于薄膜电容或性能更优的电容
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-audible-threshold-between-20ohm-and-300ohm = 蜂鸣阈值: 20Ω 到 300Ω 之间
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: DC AC & AC+DC AC Voltage
note-bm821s-specified-from-10-pct-to = BM821s: 指定范围为量程的 10%–100%
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-bar-segments-beep-tones-proportional-to = 条形段数和蜂鸣音调与场强成正比
# Used by: Mock ZT-5B, ZT-5B
# In: Continuity
note-beeps-50ohm = <50Ω 时蜂鸣
# Used by: UT803
# In: F. Continuity Test
note-beeps-continuously-10ohm-silent-30ohm = ≤10Ω 时连续蜂鸣；>30Ω 时无声
# Used by: UT804
# In: F. Continuity Test
note-beeps-continuously-10ohm-silent-50ohm = ≤10Ω 时连续蜂鸣；>50Ω 时无声
# Used by: UT181A
# In: (15) Diode Test
note-brief-beep-on-a-normal-junction = 结正常时短促蜂鸣，短路时连续蜂鸣
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-0-02v-a = 负载电压 0.02V/A
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-0-08mv-ua = 负载电压 0.08mV/µA
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-burden-voltage-2-1mv-ma = 负载电压 2.1mV/mA
# Used by: 7 meters
# In: Continuity and Diode
note-continuity-audio-visual-alarm-50ohm-no = 通断: <50Ω 声光报警；≥70Ω 不蜂鸣
# Used by: UT803
# In: B. AC Voltage
note-crest-factor-3-0-1000v-1 = 波峰因数 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage; DC AC & AC+DC AC Voltage
note-crest-factor-2-1-at-full = 波峰因数 < 2:1 (满量程)，< 4:1 (半量程)
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-crest-factor-up-to-3-0 = 波峰因数最高 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Crest mode (Instantaneous Peak Hold)
note-crest-specified-accuracy-adds-250-digits = 波峰模式: 变化 > 1.0 ms 时，指定准确度加 250 个字
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-detects-50-60hz-antenna-at-the = 检测 50/60Hz；天线位于万用表顶端
# Used by: 7 meters
# In: Continuity and Diode
note-diode-one-beep-0-12v-2v = 二极管: 0.12V–2V (正常) 时短鸣，<0.12V (短路) 时长鸣
# Used by: 7 meters
# In: Continuity and Diode
note-diode-open-circuit-3v = 二极管: 开路电压 ≈3V
# Used by: UT181A
# In: (9) Capacitance
note-display-digits-6000 = 显示计数: 6000
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-duty-cycle-specified-for-5hz-10khz = 占空比指定范围: 5Hz ~ 10kHz
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-duty-square-waves-only-1vpp-20vpp = 占空比: 仅限方波，1Vpp–20Vpp，≤10kHz，10.0%–90.0%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-for-ac-voltage-shorted-leads-residual = 交流电压，表笔短路: 剩余读数 ≤200 个字
# Used by: Mock ZT-5B, ZT-5B
# In: AC Voltage (V); AC current (A); AC current (mA)
note-frequency-response-40hz-1khz = 频率响应 40Hz–1kHz
# Used by: UT804
# In: G. Diode Test
note-good-silicon-junction-0-5v-0 = 良好的硅结: 0.5V–0.8V
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-hold-sel-ncv-for-ncv = 长按 SEL/NCV 键进入 NCV
# Used by: UT181A
# In: (8) Conductance
note-humidity-50-pct = 湿度 <50%
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-1mhz-600mv-20v = Hz 输入 >100kHz–1MHz: 600mV–20V rms
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-1v-20v-rms = Hz 输入 >1MHz: 1V–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-40mhz-1v-20v = Hz 输入 >1MHz–40MHz: 1V–20V rms；>40MHz 未指定
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-200mv-20v-rms = Hz 输入 ≤100kHz: 200mV–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Transistor Magnification
note-ib0-1-8ua-vce-2-5v = Ib0 ≈1.8µA, Vce ≈2.5V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV); AutoCheck™ (DCV)
note-impedance-rises-within-a-fraction-of = 高于 50V 时，阻抗在不到一秒内升高 (典型值)
# Used by: UT804
# In: K. Temperature 1-1. Degrees Celsius; K. Temperature 1-2. Fahrenheit
note-included-k-type-point-contact-probe = 随附的 K 型点式探头: 仅限 230°C 以下
# Used by: UT803
# In: J. Temperature
note-included-point-contact-probe-below-230degc = 随附的点式探头: 仅限 230°C 以下
# Used by: UT803
# In: I. Frequency
note-input-10hz-1mhz-150mv-30v-rms = 输入 10Hz–1MHz: 150mV–30V rms
# Used by: UT181A
# In: (11) Frequency
note-input-10hz-30mhz-600mv-30v-rms = 输入 10Hz–30MHz: 600mV–30V rms
# Used by: UT804
# In: I. Frequency; J. Duty Cycle
note-input-10hz-40mhz-200mv-30v-rms = 输入 10Hz–40MHz: 200mV–30V rms
# Used by: UT803
# In: I. Frequency
note-input-10mhz-50mhz-600mv-30v-rms = 输入 >10MHz–50MHz: 600mV–30V rms
# Used by: UT803
# In: I. Frequency
note-input-1mhz-10mhz-300mv-30v-rms = 输入 >1MHz–10MHz: 300mV–30V rms
# Used by: UT181A
# In: (11) Frequency
note-input-30mhz-not-specified = 输入 >30MHz: 未指定
# Used by: UT804
# In: I. Frequency; J. Duty Cycle
note-input-at-zero-dc-level-40mhz = 输入直流电平为零；>40MHz 未指定
# Used by: UT803
# In: I. Frequency
note-input-at-zero-dc-level-50mhz = 输入直流电平为零；>50MHz 未指定
# Used by: UT161D, UT61D+
# In: Temperature
note-k-type-thermocouple-only = 仅限 K 型热电偶
# Used by: UT181A
# In: (10) Temperature
note-k-type-thermocouple-two-channels-via = K 型热电偶；通过温度接口提供两个通道
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-lo-z-threshold-plus-1-5vdc = Lo-Z 阈值: > +1.5VDC 或 < -1.0VDC (标称)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-lo-z-threshold-3vac-50-60hz = Lo-Z 阈值: 标称 > 3VAC (50/60Hz)
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
note-measured-temperature-should-stay-below-230degc = 被测温度应低于 230°C/446°F
# Used by: 7 meters
# In: AC Current; AC Voltage
note-non-sine-add-4-pct-crest = 非正弦波: 加 4% (波峰因数 1–2)，5% (2–2.5)，7% (2.5–3)
# Used by: UT181A
# In: (15) Diode Test
note-normal-silicon-junction-0-5v-0 = 正常硅结: 0.5V–0.8V
# Used by: UT181A
# In: (14) Continuity Test
note-open-alarm-beeps-50ohm-silent-10ohm = 开路报警: >50Ω 时蜂鸣，<10Ω 时无声
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms); Ohms
note-open-circuit-1-2vdc-1-0vdc = 开路电压 < 1.2VDC (60.00MΩ 量程 < 1.0VDC)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-open-circuit-3-5vdc = 开路电压 < 3.5VDC
# Used by: UT803
# In: F. Continuity Test
note-open-circuit-1-2v = 开路电压 ≈-1.2V
# Used by: UT804
# In: F. Continuity Test
note-open-circuit-1-2v-2 = 开路电压 ≈1.2V
# Used by: 7 meters
# In: Resistance
note-open-circuit-1v = 开路电压 ≈1V
# Used by: UT803
# In: G. Diode Testst
note-open-circuit-2-7v = 开路电压 ≈2.7V
# Used by: UT804
# In: G. Diode Test
note-open-circuit-2-8v = 开路电压 ≈2.8V
# Used by: UT181A
# In: (14) Continuity Test
note-open-circuit-3v = 开路电压 ≈3V
# Used by: UT181A
# In: (15) Diode Test
note-open-circuit-3v-forward-drop-up = 开路电压 ≈3V；正向压降最高 ≈3V
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; DC Current
note-open-circuit-residual-10-digits = 开路: 剩余读数 ≤10 个字
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; DC Current
note-open-circuit-residual-5-digits = 开路: 剩余读数 ≤5 个字
# Used by: Mock ZT-5B, ZT-5B
# In: AC current (A); DC current (A)
note-over-2a-measure-for-less-than = 超过 2A: 测量时间应少于 3s
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-range-accuracy-depend-on-acmv-acv = 量程和准确度取决于 ACmV/ACV 和参考阻抗
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-ranges-2-2uf-add-10-digits = 量程 ≤2.2µF: 准确度 ≤3% 时加 10 个字
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Transistor Magnification
note-reading-for-reference-only = 读数仅供参考
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Record mode
note-record-specified-accuracy-adds-10-digits = 记录模式: 变化 > 100 ms 时，指定准确度加 10 个字
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-a-ma-picks = 红表笔插入 A mA: 自动选择电流
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-vohm-picks-v = 红表笔插入 VΩ: 高于 0.8V 时选择 V，否则选择 Ω
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Audible Continuity Tester
note-response-time-100us = 响应时间 < 100µs
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: Capacitance
note-result-reading-open-lead-capacitance-1uf = 结果 = 读数 − 表笔开路电容；≤1µF: 使用 REL
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-result-reading-open-lead-capacitance-22nf = 结果 = 读数 − 表笔开路电容；≤22nF: 使用 REL
# Used by: 7 meters
# In: Resistance
note-result-reading-shorted-lead-resistance = 结果 = 读数 − 表笔短路电阻
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-sel-ncv-continuity-diode-cap-hz = SEL/NCV: 通断/二极管、电容、Hz、°C
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Logic Level Frequency (⎍Hz) & Duty Cycle (D%)
note-sensitivity-2-5vp-square-wave-for = 灵敏度: 2.5Vp 方波，适用于 3V 和 5V 逻辑系列
# Used by: UT181A
# In: (14) Continuity Test
note-short-alarm-beeps-10ohm-silent-50ohm = 短路报警: <10Ω 时蜂鸣，>50Ω 时无声
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-shorted-input-residual-under-30-digits = 输入短路: 剩余读数低于 ~30 个字，不影响准确度
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-shorted-leads-80-digit-residual-accuracy = 表笔短路: 剩余读数 80 个字，不影响准确度
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Voltage
note-shorted-leads-residual-10-digits = 表笔短路: 剩余读数 ≤10 个字
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Voltage
note-shorted-leads-residual-3-digits = 表笔短路: 剩余读数 ≤3 个字
# Used by: 7 meters
# In: DC Voltage
note-shorted-leads-residual-5-digits = 表笔短路: 剩余读数 ≤5 个字
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-test-current-0-4ma-typical = 测试电流典型值 0.4mA
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (ACV)
note-then-typically-18kohm-100v-125kohm-300v = 之后典型值 18kΩ@100V，125kΩ@300V，320kΩ@600V，460kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (DCV)
note-then-typically-18kohm-100v-125kohm-300v-2 = 之后典型值 18kΩ@100V，125kΩ@300V，320kΩ@600V，500kΩ@1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-tolerance-10v-36v-23v-83v-59v = 容差: 10V~36V，23V~83V，59V~165V，124V~330V，> 250V
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10 = 真有效值和准确度有效范围: 量程的 10%–100%
# Used by: UT804
# In: D. AC Current (AC+DC measurement is available)
note-true-rms-and-accuracy-valid-10-2 = 真有效值和准确度有效范围: 量程的 10%–100%；波峰因数最高 3.0
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage
note-true-rms-from-10-pct-to = 真有效值: 量程的 10%–100%；BM821s: 平均值检测
# Used by: UT181A
# In: 4 tables
note-true-rms-valid-10-pct-100 = 真有效值有效范围: 量程的 10%–100%
# Used by: UT803
# In: B. AC Voltage
note-true-rms-valid-10-pct-95 = 真有效值有效范围: 量程的 10%–95%
# Used by: UT803
# In: D. AC Current
note-true-rms-valid-10-pct-95-2 = 真有效值有效范围: 量程的 10%–95%；波峰因数 3.0
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-true-rms-accuracy-valid-10-pct = 真有效值；准确度有效范围: 量程的 10%–100%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-2-0 = 真有效值；波峰因数 10000 计数时 ≤2.0，22000 计数时 ≤1
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-3-0 = 真有效值；波峰因数 3000 计数时 ≤3.0，6000 计数时 ≤1.5
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Temperature
note-type-k-thermocouple-range-accuracy-not = 不含 K 型热电偶的量程和准确度
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-typical-20v-55v-110v-220v-440v = 典型值: - 20V，-- 55V，--- 110V，---- 220V，----- 440V
# Used by: UT804
# In: J. Duty Cycle
note-valid-10-pct-90-pct-of = 有效范围: 量程的 10%–90%，仅限 5Hz–2kHz
# Used by: UT803
# In: K. Transistor
note-vce-2-2v-bo-10ua = Vce ≈2.2V, bo ≈10µA
# Used by: UT181A
# In: (12) Duty Cycle; (13) Pulse Width
note-when-rise-time-1us-signals-center = 上升时间 <1µs 时，信号以触发电平为中心
# Used by: UT803
# In: G. Diode Testst
note-working-current-1ma = 工作电流 ≈1mA
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current
note-ua-ranges-min-30ua = µA 量程: 最小 30µA
# Used by: UT803
# In: C. DC Current; D. AC Current
note-5a-range-continuous-5a-range-10s = ≤5A 量程: 可连续使用；>5A 量程: 每次 ≤10s，间隔 ≥15min
