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
band-4000-counts = (4000 カウント)
# Used by: VC920/VC940/VC960
band-plus-40-to-plus-400-degc = +40 から +400 °C
# Used by: VC920/VC940/VC960
band-plus-400-to-plus-1000-degc = +400 から +1000 °C
# Used by: VC920/VC940/VC960
band-40-to-plus-40-degc = -40 から +40 °C
# Used by: VC920/VC940/VC960
band-10khz-120khz-10-15-pct-of = 10kHz~120kHz、レンジの 10 - 15%
# Used by: VC920/VC940/VC960
band-10khz-120khz-15-100-pct-of = 10kHz~120kHz、レンジの 15 - 100%
# Used by: VC920/VC940/VC960
band-120khz-250khz-10-30-pct-of = 120kHz~250kHz、レンジの 10 - 30%
# Used by: VC920/VC940/VC960
band-120khz-250khz-30-100-pct-of = 120kHz~250kHz、レンジの 30 - 100%
# Used by: VC920/VC940/VC960
band-250khz-400khz-10-30-pct-of = 250kHz~400kHz、レンジの 10 - 30%
# Used by: VC920/VC940/VC960
band-30-100-pct-of-mr = レンジの 30 - 100%
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
band-in-rel-state = REL モード時
# Used by: UT181A, UT804
# In: (9) Capacitance; H. Capacitance
band-not-specified-2 = 規定なし
# Used by: UT181A
# In: (1) AC Voltage; (3) AC Voltage + DC Voltage
band-only-for-reference = 参考値
# Used by: CMS061, CMS101
# In: Duty Cycle (%)
band-typical-vrms-1-v-f-100hz = 標準: Vrms=1 V, f=100Hz

## Inputs: a table's input impedance or overload protection.

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
# In: A. DC Voltage
input-around-2-5gohm = 約 2.5GΩ
# Used by: UT803
# In: A. DC Voltage; B. AC Voltage
input-around-3000mohm = 約 > 3000MΩ
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
input-current-fuse-10a-1000v-voltage-1000v = 電流: ヒューズ 10A 1000V、電圧: 1000V
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current; Line Level Frequency (~Hz)
input-fuse-0-44a-1000v = ヒューズ 0.44A 1000V
# Used by: UT71A, UT71B, UT71C/D/E
# In: C. DC Current; D. AC Current (AC+DC measurement is available); K. 4~20 mA loop current (UT71B/C/D/E)
input-fuse-0-5a-1000v = ヒューズ 0.5A 1000V
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

## Notes, under a table. Keep the exact sense, comparisons and qualifiers
## such as "typical" included.

# Used by: VC920/VC940/VC960
# In: Power
note-0-1150w-continuous-1150w-2500w-max = 0–1150W は連続、>1150W–2500W は最大 10s、その後 15min 休止
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; DC Current
note-10a-continuous-10a-to-20a-30s = 10A は連続、>10A から 20A まで: 最大 30s、冷却 5 min
# Used by: VC920/VC940/VC960
# In: AC / A; DC / A
note-10a-0-5a-continuous-5a-10a = 10A: 0–5A は連続、>5A–10A は最大 10s、その後 15min 休止
# Used by: UT71A, UT71B, UT71C/D/E
# In: C. DC Current; D. AC Current (AC+DC measurement is available)
note-10a-5a-continuous-5a-10a-10s-2 = 10A: ≤5A は連続、5A–10A は ≤10s ずつ、間隔 >15min
# Used by: UT804
# In: C. DC Current; D. AC Current (AC+DC measurement is available)
note-10a-5a-continuous-5a-10a-10s = 10A: ≤5A は連続、>5A–10A は ≤10s ずつ、間隔 >15min
# Used by: UT181A
# In: (13) Pulse Width
note-10hz-200khz-pulse-width-2us-depends = 10Hz–200kHz: パルス幅 >2µs、信号の周波数に依存
# Used by: CMS061, CMS101
# In: Capacitance(F)
note-20-00mf-measuring-duration-should-be = 20.00mF: 測定時間は 30s を超えること
# Used by: UT71A, UT71B
# In: E. Resistance
note-200ohm-accuracy-plus-test-lead-short = 200Ω: 確度 + テストリード短絡時の値
# Used by: UT181A
# In: (4) AC Current; (5) DC Current; (6) AC Current + DC Current
note-20a-30s-on-then-10min-off = 20A: 30s 通電後 10min 休止、10A 超は規定なし
# Used by: UT71A, UT71B
# In: H. Capacitance
note-20nf-accuracy-plus-open-circuit-test = 20nF: 確度 + テストリード開放時の静電容量
# Used by: Mock UT61E+, UT161E, UT61E+
# In: Capacitance
note-220mf-range-measurement-takes-20s = 220mF レンジ: 測定時間 ≈20s
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-25-00mf-manual-range-not-specified = 25.00mF、マニュアルレンジ: 5.4mF 未満は規定なし
# Used by: UT804
# In: E. Resistance
note-400ohm-accuracy-plus-test-lead-open = 400Ω: 確度 + テストリード開放時の値
# Used by: UT71C/D/E
# In: E. Resistance
note-400ohm-accuracy-plus-test-lead-short = 400Ω: 確度 + テストリード短絡時の値
# Used by: UT804
# In: H. Capacitance
note-40nf-accuracy-plus-open-circuit-test = 40nF: 確度 + テストリード開放時の静電容量
# Used by: UT71B, UT71C/D/E
# In: K. 4~20 mA loop current (UT71B/C/D/E)
note-4ma-shows-0-pct-20ma-shows-2 = 4mA で 0%、20mA で 100% を表示
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
# Used by: UT71B, UT71C/D/E
# In: K. 4~20 mA loop current (UT71B/C/D/E)
note-4ma-display-shows-lo = <4mA: ディスプレイに LO を表示
# Used by: UT71B, UT71C/D/E
# In: K. 4~20 mA loop current (UT71B/C/D/E)
note-20ma-display-shows-hi = >20mA: ディスプレイに HI を表示
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
# Used by: VC920/VC940/VC960
# In: AC / A; AC / V
note-ac-plus-dc-add-1-pct-2 = AC+DC: 確度に 1%+30 カウントを加算
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available); D. AC Current (AC+DC measurement is available)
note-ac-plus-dc-add-1-pct = AC+DC: 確度に 1%+35 カウントを加算
# Used by: UT803
# In: B. AC Voltage; D. AC Current
note-ac-plus-dc-range-accuracy-plus = AC+DC: レンジの確度 + 1%
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
# Used by: CMS061, CMS101
# In: AC Current (A); AC Voltage (V)
note-accuracy-valid-10-pct-100-pct = 確度はレンジの 10%–100% で有効
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
# Used by: CMS061, CMS101
# In: Capacitance(F); DC Current (A); DC Voltage (V)
note-accuracy-valid-5-pct-100-pct = 確度はレンジの 5%–100% で有効
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Capacitance
note-accuracy-with-film-capacitor-or-better = 確度はフィルムコンデンサーと同等以上のコンデンサーで規定
# Used by: CMS061, CMS101
# In: Capacitance(F); Resistance(Ω)
note-allow-for-the-test-leads-own = テストリード自体の抵抗とリアクタンスを考慮すること
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
# Used by: UT71A, UT71B, UT71C/D/E
# In: F. Continuity Test
note-beeps-continuously-40ohm = ≤40Ω で連続ビープ音
# Used by: UT71A, UT71B, UT71C/D/E
# In: B. AC Voltage (AC+DC measurement is available)
note-below-100khz-accuracy-guaranteed-10-pct = 100kHz 未満: 10%–100% で確度を保証
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
# Used by: VC920/VC940/VC960
# In: Ω
note-continuity-beeps-50ohm = 導通: <50Ω でビープ音
# Used by: UT803
# In: B. AC Voltage
note-crest-factor-3-0-1000v-1 = クレストファクター 3.0 (1000V: 1.5)
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: AC & AC+DC Current; AC Voltage; DC AC & AC+DC AC Voltage
note-crest-factor-2-1-at-full = クレストファクター: フルスケールで < 2:1、ハーフスケールで < 4:1
# Used by: UT804
# In: B. AC Voltage (AC+DC measurement is available)
note-crest-factor-up-to-3-0 = クレストファクターは最大 3.0 (1000V: 1.5)
# Used by: VC920/VC940/VC960
# In: AC / A; AC / V
note-crest-factor-3 = クレストファクター ≤3
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
# Used by: VC920/VC940/VC960
# In: Ω
note-fixed-at-4000-counts = 4000 カウント固定
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
# Used by: UT71A, UT71B, UT71C/D/E
# In: F. Continuity Test
note-no-beep-60ohm = >60Ω でビープ音なし
# Used by: 7 meters
# In: AC Current; AC Voltage
note-non-sine-add-4-pct-crest = 非正弦波: 4% (クレストファクター 1–2)、5% (2–2.5)、7% (2.5–3) を加算
# Used by: UT181A
# In: (14) Continuity Test
note-open-alarm-beeps-50ohm-silent-10ohm = 開放アラーム: >50Ω でビープ音、<10Ω で鳴らない
# Used by: CMS061, CMS101
# In: Diode
note-open-circuit-3-2v = 開放電圧 3.2V
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
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
note-power-factor-input-0-000-1 = 力率の入力範囲 0.000–1.000
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
note-red-lead-in-a-ma-picks = 赤のテストリードを A mA 端子に接続すると電流が選択される
# Used by: Mock ZT-5B, ZT-5B
# In: Auto
note-red-lead-in-vohm-picks-v = 赤のテストリードを VΩ 端子に接続すると、0.8V 超で V、それ以外は Ω が選択される
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
# Used by: VC920/VC940/VC960
# In: Frequency
note-sensitivity-10hz-40mhz-200mv-40mhz-not = 感度 10Hz–40MHz: 200mV、>40MHz は規定なし
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
# Used by: VC920/VC940/VC960
# In: AC / V
note-shorted-leads-display-70-digits = テストリード短絡時: 表示 ≤70 カウント
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Voltage
note-shorted-leads-residual-10-digits = テストリード短絡時: 残留 ≤10 カウント
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Voltage
note-shorted-leads-residual-3-digits = テストリード短絡時: 残留 ≤3 カウント
# Used by: 7 meters
# In: DC Voltage
note-shorted-leads-residual-5-digits = テストリード短絡時: 残留 ≤5 カウント
# Used by: CMS061, CMS101
# In: On-Off
note-shows-ol-above-200-0ohm = 200.0Ω を超えると OL を表示
# Used by: CMS061, CMS101
# In: Frequency (Hz)
note-signal-1hz-20mhz-1v-rms = 信号: 1Hz–20MHz、≥ 1V rms
# Used by: VC920/VC940/VC960
# In: Temperature
note-supplied-wire-thermocouple-max-plus-230degc = 付属のワイヤー型熱電対: 最大 +230°C
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Diode Tester
note-test-current-0-4ma-typical = テスト電流 0.4mA (標準)
# Used by: VC920/VC940/VC960
# In: Diode test
note-test-voltage-max-2-8v-test = テスト電圧は最大 2.8V、テスト電流 ≈1mA
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
# Used by: UT71A, UT71B, UT71C/D/E
# In: D. AC Current (AC+DC measurement is available)
note-true-rms-valid-10-pct-100-2 = 真の実効値はレンジの 10%–100% で有効、クレストファクター ≤3.0
# Used by: UT71A, UT71B, UT71C/D/E
# In: B. AC Voltage (AC+DC measurement is available)
note-true-rms-valid-10-pct-100-3 = 真の実効値はレンジの 10%–100% で有効、クレストファクター ≤3.0 (1000V: 1.5)
# Used by: UT803
# In: B. AC Voltage
note-true-rms-valid-10-pct-95 = 真の実効値はレンジの 10%–95% で有効
# Used by: UT803
# In: D. AC Current
note-true-rms-valid-10-pct-95-2 = 真の実効値はレンジの 10%–95% で有効、クレストファクター 3.0
# Used by: Mock UT61E+, UT161E, UT61E+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-2-0 = 真の実効値、クレストファクター: 10000 カウントで ≤2.0、22000 カウントで ≤1
# Used by: UT161B, UT161D, UT61B+, UT61D+
# In: AC Current; AC Voltage
note-true-rms-crest-factor-3-0 = 真の実効値、クレストファクター: 3000 カウントで ≤3.0、6000 カウントで ≤1.5
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Temperature
note-type-k-thermocouple-range-accuracy-not = K 型熱電対のレンジと確度は含まれない
# Used by: CMS061, CMS101
# In: Duty Cycle (%)
note-typical-waveform-square = 標準波形: 方形波
# Used by: CMS061, CMS101
# In: Frequency (Hz)
note-typical-waveform-square-or-sine = 標準波形: 方形波または正弦波
# Used by: BM525s/BM521s, BM829s/BM827s/BM822s/BM821s
# In: Non-Contact EF-Detection
note-typical-20v-55v-110v-220v-440v = 標準: - 20V, -- 55V, --- 110V, ---- 220V, ----- 440V
# Used by: UT804
# In: J. Duty Cycle
note-valid-10-pct-90-pct-of = レンジの 10%–90%、5Hz–2kHz のみで有効
# Used by: UT803
# In: K. Transistor
note-vce-2-2v-bo-10ua = Vce ≈2.2V, bo ≈10µA
# Used by: VC920/VC940/VC960
# In: Power
note-voltage-190v-250v-ac = 電圧 190V–250V AC
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
note-voltage-input-50v-250v-ac = 電圧入力 50V–250V AC
# Used by: UT71C/D/E
# In: L. Power Measurement (UT71E only)
note-voltage-input-impedance-10mohm = 電圧入力インピーダンス ≈10MΩ
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
