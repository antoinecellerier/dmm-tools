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
band-4000-counts = (4000 计数)
# Used by: VC920/VC940/VC960
band-30-100-pct-of-mr = 量程的 30 - 100%
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
band-in-rel-state = REL 模式下
# Used by: UT181A, UT804
# In: (9) Capacitance; H. Capacitance
band-not-specified-2 = 未指定
# Used by: UT181A
# In: (1) AC Voltage; (3) AC Voltage + DC Voltage
band-only-for-reference = 仅供参考
# Used by: CMS061, CMS101
# In: Duty Cycle (%)
band-typical-vrms-1-v-f-100hz = 典型值: Vrms=1 V，f=100Hz

## Inputs: a table's input impedance or overload protection.

# Used by: UT803
# In: 6 tables
input-250v-rms = 250V rms
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
input-current-fuse-10a-1000v-voltage-1000v = 电流: 保险丝 10A 1000V；电压: 1000V
# Used by: UT803
# In: K. Transistor
input-fuse-200ma-250v-fuse-500ma-125v = 保险丝 200mA 250V；保险丝 500mA 125V

## Notes, under a table. Keep the exact sense, comparisons and qualifiers
## such as "typical" included.

# Used by: VC920/VC940/VC960
# In: Power
note-0-1150w-continuous-1150w-2500w-max = 0–1150W 可连续使用；>1150W–2500W 每次最长 10s，之后需暂停 15min
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-10a-continuous-10a-to-20a-30s = 10A 可连续使用；>10A 到 20A: 最长 30s，需冷却 5 min
# Used by: VC920/VC940/VC960
# In: AC / A; DC / A
note-10a-0-5a-continuous-5a-10a = 10A: 0–5A 可连续使用；>5A–10A 每次最长 10s，之后需暂停 15min
# Used by: UT181A
# In: (13) Pulse Width
note-10hz-200khz-pulse-width-2us-depends = 10Hz–200kHz: 脉冲宽度 >2µs，取决于信号频率
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
note-20a-30s-on-then-10min-off = 20A: 连续测量 30s，之后间隔 10min；10A 以上未指定
# Used by: UT804
# In: E. Resistance
note-400ohm-accuracy-plus-test-lead-open = 400Ω: 准确度 + 表笔开路值
# Used by: UT71B, UT71C/D/E
# In: K. 4~20 mA loop current (UT71B/C/D/E)
note-4ma-shows-0-pct-20ma-shows-2 = 4mA 显示 0%；20mA 显示 100%
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms)
note-60-00mohm-tempco-0-6-accuracy = 60.00MΩ: 温度系数 0.6× 准确度/°C @(0°C ~ 18°C 或 28°C ~ 45°C)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: dBm
note-600ohm-ref-typical-acmv-42-22 = 600Ω 参考，典型值: ACmV -42.22 到 -02.22dBm，ACV -17.78 到 62.22dBm
# Used by: UT181A
# In: (7) Resistance
note-60mohm-humidity-50-pct = 60MΩ: 湿度 <50%
# Used by: UT181A
# In: (2) DC Voltage
note-60mv-use-rel-to-compensate-bias = 60mV: 使用 REL 补偿偏置电压
# Used by: UT803
# In: H. Capacitance
note-6nf-60nf-600nf-subtract-open-circuit = 6nF，60nF，600nF: 减去表笔开路电容
# Used by: UT804
# In: L. 4~20 mA loop current
note-4ma-shows-lo-20ma-shows-hi = <4mA 显示 LO，>20mA 显示 HI
# Used by: UT71B, UT71C/D/E
# In: K. 4~20 mA loop current (UT71B/C/D/E)
note-4ma-display-shows-lo = <4mA: 显示 LO
# Used by: UT71B, UT71C/D/E
# In: K. 4~20 mA loop current (UT71B/C/D/E)
note-20ma-display-shows-hi = >20mA: 显示 HI
# Used by: 7 meters
# In: AC Current; DC Current
note-5a-max-10s-rest-15-min = >5A: 最长 10s，之后间隔 ≥15 min
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-ac-plus-dc-range-accuracy-plus = AC+DC: 量程准确度 + 1%
# Used by: UT803
# In: J. Temperature
note-above-230degc-use-the-rod-type = 230°C 以上: 使用杆式探头
# Used by: UT181A
# In: (10) Temperature
note-accessory-point-contact-probe-below-230degc = 配件点式探头: 仅限 230°C 以下
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-accuracy-valid-1-pct-100-pct-2 = 准确度有效范围: 量程的 1%–100% (1kHz–10kHz: 10%–100%)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-accuracy-with-film-capacitor-or-better = 准确度适用于薄膜电容或性能更优的电容
# Used by: CMS061, CMS101
# In: Capacitance(F); Resistance(Ω)
note-allow-for-the-test-leads-own = 需计入表笔自身的电阻和电抗
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
# Used by: UT71A, UT71B, UT71C/D/E
# In: F. Continuity Test
note-beeps-continuously-40ohm = ≤40Ω 时连续蜂鸣
# Used by: UT71A, UT71B, UT71C/D/E
# In: B. AC Voltage (AC+DC measurement is available)
note-below-100khz-accuracy-guaranteed-10-pct = 100kHz 以下: 在 10%–100% 范围内保证准确度
# Used by: UT181A
# In: (15) Diode Test
note-brief-beep-on-a-normal-junction = 被测结正常时短促蜂鸣，短路时连续蜂鸣
# Used by: 7 meters
# In: Continuity and Diode
note-continuity-audio-visual-alarm-50ohm-no = 通断: <50Ω 声光报警；≥70Ω 不蜂鸣
# Used by: VC920/VC940/VC960
# In: Ω
note-continuity-beeps-50ohm = 通断: <50Ω 时蜂鸣
# Used by: UT803
# In: B. AC Voltage
note-crest-factor-3-0-1000v-1 = 波峰因数 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage; DC AC & AC+DC AC Voltage
note-crest-factor-2-1-at-full = 波峰因数 < 2:1 (满量程)，< 4:1 (半量程)
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-crest-factor-up-to-3-0 = 波峰因数最高 3.0 (1000V: 1.5)
# Used by: VC920/VC940/VC960
# In: AC / A; AC / V
note-crest-factor-3 = 波峰因数 ≤3
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
# Used by: VC920/VC940/VC960
# In: Ω
note-fixed-at-4000-counts = 固定为 4000 计数
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-for-ac-voltage-shorted-leads-residual = 交流电压，表笔短路: 剩余读数 ≤200 个字
# Used by: Mock ZT-5B, ZT-5B
# In: AC Voltage (V); AC current (A); AC current (mA)
note-frequency-response-40hz-1khz = 频率响应 40Hz–1kHz
# Used by: UT804
# In: G. Diode Test
note-good-silicon-junction-0-5v-0 = 正常硅结: 0.5V–0.8V
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-hold-sel-ncv-for-ncv = 长按 SEL/NCV 键进入 NCV
# Used by: UT181A
# In: (8) Conductance
note-humidity-50-pct = 湿度 <50%
# Used by: 7 meters
# In: Frequency/Duty Ratio
note-hz-input-100khz-1mhz-600mv-20v = Hz 输入 >100kHz–1MHz: 600mV–20V rms
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Frequency/Duty Ratio
note-hz-input-1mhz-40mhz-1v-20v = Hz 输入 >1MHz–40MHz: 1V–20V rms；>40MHz 未指定
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
# Used by: UT181A
# In: (11) Frequency
note-input-30mhz-not-specified = 输入 >30MHz: 未指定
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
# Used by: UT161D, UT61D+
# In: Temperature
note-measured-temperature-should-stay-below-230degc = 被测温度应低于 230°C/446°F
# Used by: UT71A, UT71B, UT71C/D/E
# In: F. Continuity Test
note-no-beep-60ohm = >60Ω 时不蜂鸣
# Used by: 7 meters
# In: AC Current; AC Voltage
note-non-sine-add-4-pct-crest = 非正弦波: 加 4% (波峰因数 1–2)，5% (2–2.5)，7% (2.5–3)
# Used by: UT181A
# In: (14) Continuity Test
note-open-alarm-beeps-50ohm-silent-10ohm = 开路报警: >50Ω 时蜂鸣，<10Ω 时无声
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AutoCheck™ (Ohms); Ohms
note-open-circuit-1-2vdc-1-0vdc = 开路电压 < 1.2VDC (60.00MΩ 量程 < 1.0VDC)
# Used by: UT181A
# In: (15) Diode Test
note-open-circuit-3v-forward-drop-up = 开路电压 ≈3V；正向压降最高 ≈3V
# Used by: Mock ZT-5B, ZT-5B
# In: AC current (A); DC current (A)
note-over-2a-measure-for-less-than = 超过 2A: 测量时间应少于 3s
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
note-power-factor-input-0-000-1 = 功率因数输入范围 0.000–1.000
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
# Used by: 7 meters
# In: Resistance
note-result-reading-shorted-lead-resistance = 结果 = 读数 − 表笔短路电阻
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-sel-ncv-continuity-diode-cap-hz = SEL/NCV: 通断/二极管、电容、Hz、°C
# Used by: VC920/VC940/VC960
# In: Frequency
note-sensitivity-10hz-40mhz-200mv-40mhz-not = 灵敏度 10Hz–40MHz: 200mV；>40MHz 未指定
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
# Used by: CMS061, CMS101
# In: On-Off
note-shows-ol-above-200-0ohm = 高于 200.0Ω 时显示 OL
# Used by: CMS061, CMS101
# In: Frequency (Hz)
note-signal-1hz-20mhz-1v-rms = 信号: 1Hz–20MHz，≥ 1V rms
# Used by: VC920/VC940/VC960
# In: Temperature
note-supplied-wire-thermocouple-max-plus-230degc = 随附的线式热电偶: 最高 +230°C
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-test-current-0-4ma-typical = 测试电流典型值 0.4mA
# Used by: VC920/VC940/VC960
# In: Diode test
note-test-voltage-max-2-8v-test = 测试电压最大 2.8V；测试电流 ≈1mA
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
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage
note-true-rms-from-10-pct-to = 真有效值: 量程的 10%–100%；BM821s: 平均值检测
# Used by: UT71A, UT71B, UT71C/D/E
# In: B. AC Voltage (AC+DC measurement is available)
note-true-rms-valid-10-pct-100-3 = 真有效值有效范围: 量程的 10%–100%；波峰因数 ≤3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Temperature
note-type-k-thermocouple-range-accuracy-not = 不含 K 型热电偶的量程和准确度
# Used by: CMS061, CMS101
# In: Duty Cycle (%)
note-typical-waveform-square = 典型波形: 方波
# Used by: CMS061, CMS101
# In: Frequency (Hz)
note-typical-waveform-square-or-sine = 典型波形: 方波或正弦波
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-typical-20v-55v-110v-220v-440v = 典型值: - 20V，-- 55V，--- 110V，---- 220V，----- 440V
# Used by: UT804
# In: J. Duty Cycle
note-valid-10-pct-90-pct-of = 有效范围: 量程的 10%–90%，仅限 5Hz–2kHz
# Used by: UT803
# In: K. Transistor
note-vce-2-2v-bo-10ua = Vce ≈2.2V, bo ≈10µA
# Used by: VC920/VC940/VC960
# In: Power
note-voltage-190v-250v-ac = 电压 190V–250V AC
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
note-voltage-input-50v-250v-ac = 电压输入 50V–250V AC
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
note-voltage-input-impedance-10mohm = 电压输入阻抗 ≈10MΩ
# Used by: UT181A
# In: (12) Duty Cycle; (13) Pulse Width
note-when-rise-time-1us-signals-center = 上升时间 <1µs 时，信号以触发电平为中心
# Used by: UT803
# In: G. Diode Testst
note-working-current-1ma = 工作电流 ≈1mA
# Used by: VC920/VC940/VC960
# In: AC / A; AC / V
note-true-rms-accuracy-valid-10-pct = 真有效值准确度有效范围: 量程的 10%–100%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC+DC Voltage
note-true-rms-accuracy-valid-10-pct-2 = 真有效值；准确度有效范围: 量程的 10%–100%
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current
note-ua-ranges-min-30ua = µA 量程: 最小 30µA
# Used by: UT803
# In: C. DC Current; D. AC Current
note-5a-range-continuous-5a-range-10s = ≤5A 量程: 可连续使用；>5A 量程: 每次 ≤10s，间隔 ≥15min
