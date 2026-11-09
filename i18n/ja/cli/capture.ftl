### What 'dmm-cli capture' prints around its steps: the guided run a user
### makes to send a report about their meter. It walks the meter through a
### list of steps (set the dial to DC V, press HOLD…), reads what the meter
### sends at each, and writes a report file to attach to an issue on GitHub.
### These lines go to the terminal's error stream.
###
### Each step's own instruction, the readings (5.5687 V [AUTO]) and the
### report file stay as they are, in English: a maintainer reads the report.
### A user may translate the instructions for themselves: see
### --export-instructions below.
### Step ids (dcv, hold_on, extra_0), setting names (range, hold) and words
### in --flag form are typed or read back by those names and stay as they
### are. The keys the user presses to answer (s, q, r, n, y) arrive as
### variables: the run only understands those letters, so they can't be
### changed. "Enter" is the key's name and can be translated.

## Checking that the meter answers, before anything else.

# Dim, while the tool asks the meter for its name.
capture-checking = マルチメーターとの通信を確認中…
# Yellow and bold. The cable or Bluetooth link opened, but the meter sent
# nothing. $link is usb (a USB cable), adapter (a Bluetooth adapter plugged
# into the meter), radio (the meter's own Bluetooth) or other.
capture-not-responding =
    { $link ->
        [usb] USB ケーブルは接続されていますが、マルチメーターが応答しません。
        [adapter] Bluetooth アダプターは接続されていますが、マルチメーターが応答しません。
        [radio] Bluetooth リンクは確立されていますが、マルチメーターが応答しません。
       *[other] リンクは確立されていますが、マルチメーターが応答しません。
    }
# Followed by the meter's own steps for switching its data transmission on.
capture-enable-transmission = データ転送をオンにするには、次の手順を実行します:
capture-run-again = その後、このコマンドをもう一度実行してください。
# What the meter sent was kept in a file of its own, as evidence. $bytes is
# how many bytes the meter sent, $path the file.
capture-no-response-saved = { $bytes } バイトを受信し、{ $path } に保存しました
# $url is the web address of the meter's issue on GitHub.
capture-no-response-attach = マルチメーターの電源が入っていても応答しない場合は、このファイルを { $url } に添付してください。
# $error is why the file could not be written.
capture-no-response-not-saved = 受信したバイトを保存できませんでした: { $error }

## The banner once the meter answers, after the lines naming the meter
## (connect.ftl). Each support level is what follows "Status:"; the line
## under the partly verified and experimental ones is indented under it.

# $status is one of the capture-status-* levels below, coloured.
capture-status = 状態: { $status }
capture-status-verified = 対応機種
capture-status-partly-verified = 一部検証済み (接続と主なモードは確認済み)
capture-status-partly-verified-detail = 今回の実行では、確認されていないモードを検証します。
# The protocol was worked out from the meter's traffic, not from its maker.
capture-status-experimental = 試験的 (プロトコルはリバース エンジニアリングで解析したもので、確認されていません)
capture-status-experimental-detail = できるだけ多くのステップを完了して、レポートを共有してください。

## The meter names another model than the one the run opened (a CMS061
## opened as a CMS101): asked before the report file is touched, so the
## operator can run again with the right meter. $named and $opened are
## meter names; $id is the --device id that opens the named one.

# Yellow and bold, after the status banner.
capture-other-model = マルチメーターが返した機種名は、今回接続した { $opened } ではなく { $named } です。
capture-other-model-detail = このまま続けると、ステップとリモート キーは { $opened } のものになります。
# $yes and $no are the keys that answer: y and n. Any other key quits.
capture-other-model-confirm = このまま { $opened } としてキャプチャしますか? { $yes }/{ $no }:
capture-other-model-rerun = --device { $id } を指定して、キャプチャをもう一度実行してください。

## A report from an earlier run is already in the file: resume it, start
## again or stop. Each prompt waits for one key, after a space.

# $path is the report file. $captured and $skipped count its steps.
capture-found-existing = 既存のキャプチャが見つかりました: { $path } (キャプチャ済み { $captured } 件、スキップ { $skipped } 件)
# $resume, $fresh and $abort are the keys that answer: r, n and q.
capture-resume-prompt = { $resume }=再開、{ $fresh }=最初から開始、{ $abort }=中止:
capture-aborted = 中止しました。
# $yes and $no are the keys that answer: y and n.
capture-overwrite-confirm = 既存のキャプチャを上書きしますか? { $yes }/{ $no }:
capture-resuming = 再開します。キャプチャ済みのステップはスキップされます。
# The file is there but isn't a report this tool can read. $path is the
# file.
capture-unreadable = { $path } が見つかりましたが、解析できませんでした。
# $yes is the key that answers: y.
capture-overwrite-unreadable = 上書きしますか? { $yes }=最初から開始、ほかのキー=中止:
# $path is the report file the run writes.
capture-output-file = 出力ファイル: { $path }

## What the steps ask for besides the meter and its test leads, listed
## before the first step so the user can gather it all at once. Each item is
## on a line of its own after its number in brackets.

capture-needs-heading = 次のものが必要です:
# $item is one of the capture-need-* items below; $steps lists the ids of
# the steps that need it, e.g. "dcv_short, ohm_short".
capture-needs-item = { $item }  (ステップ: { $steps })
# The user types the numbers in brackets of what they don't have, or just
# presses Enter.
capture-needs-prompt = 用意できないものの番号を入力してください (すべてそろっている場合は Enter キー):
# What the user typed at a numbers prompt (this one or the review's) isn't
# one of the numbers listed. $token is what they typed, in quotes; $max the
# last number listed.
capture-not-a-number = { $token } は 1 から { $max } までの数値ではありません
capture-need-shorted-leads = 短絡したテストリード
capture-need-dc-source = 電池などの直流電源
capture-need-thermocouple = K 型熱電対
# For the non-contact voltage detector, which senses a live wire nearby.
capture-need-live-wire = 近くにある商用電源の活線
# The meter's power-measurement adapter, plugged into a live outlet with a
# load plugged into it.
capture-need-power-adapter = 電力測定アダプター、通電中のコンセント、負荷
capture-need-transistor = トランジスタ
capture-need-scr = SCR (サイリスタ)
capture-need-loop-source = 4-20 mA ループと 15-48 V のループ電源
capture-need-power-module = 直流または USB の電力測定モジュール、電源、負荷
# LX-925 is the adapter's name.
capture-need-pv-adapter = 日射強度アダプター LX-925

## The steps. Each opens with its id in brackets and its instruction, and
## the tool watches the meter until it shows what the step asks for.

# Bold, between heavy lines.
capture-modes-heading = 測定モード
# Dim, under the heading. $skip and $quit are the keys: s and q.
capture-modes-keys = 測定値が安定すると、ステップごとに自動でキャプチャします。Enter=今すぐキャプチャ、{ $skip }=このステップをスキップ、{ $quit }=残りをスキップして保存
# Dim, after a step's instruction: the step the rest of the run relies on.
capture-gate-mark = (前提ステップ)
# A resumed run passes over a step the report already has. $step is the
# step's id.
capture-already-captured = { $step } はキャプチャ済みのためスキップします
# Dim, once the tool has waited a while. $skip and $quit are the keys: s and
# q.
capture-press-enter = マルチメーターの準備ができたら Enter キーを押してください ({ $skip }=スキップ、{ $quit }=終了)
# Dim, on a single line the run keeps rewriting while it waits for the meter.
# $reading is the latest reading (5.5687 V [AUTO]); $frame counts the meter's
# replies so far.
capture-progress-watching = マルチメーターを監視中 · { $reading } · フレーム { $frame }
# Dim, on that same line while a step's readings are taken: $done of $total
# ("reading 4 of 12"), and $reading the latest one.
capture-progress-reading = 読み取り中 { $done }/{ $total } · { $reading }
# Dim: a step that presses a button on the meter by remote control holds it
# until the meter shows the state the instruction asks for. $skip and $quit
# are the keys: s and q.
capture-wait-start = 上記のとおりマルチメーターを設定してください。設定が完了するとキーを送信します (Enter=今すぐ送信、{ $skip }=スキップ、{ $quit }=終了)
# Dim: the meter left the state the step starts from just before the key
# went out, so the step waits for it again.
capture-wait-moved = キーを送信する前にマルチメーターの状態が変わったため、もう一度待機します
# When the terminal can't be read key by key. $skip and $quit are the keys:
# s and q.
capture-any-key = 任意のキー=キャプチャ、{ $skip }=スキップ、{ $quit }=終了:
# Red: the button the step presses on the meter by remote control was
# refused. $error is why.
capture-command-failed = コマンドを実行できませんでした: { $error }
# Yellow: the tool pressed a button on the meter by remote control and the
# meter didn't change. $command is the button's name as the tool sends it,
# e.g. hold; $reading what the meter shows, e.g. 5.5687 V [AUTO].
capture-did-nothing-shows = { $command } を押しても変化せず、マルチメーターの表示は { $reading } のままです
# Yellow: the tool pressed a button on the meter by remote control and the
# meter changed, but not to what the step waits for. $command is the button's
# name as the tool sends it, e.g. hold; $reading what the meter shows, e.g.
# 0.00 VA [HOLD].
capture-command-missed = { $command } を押しても、このステップで必要な状態になりませんでした。現在の表示は { $reading } です
# $command is the button's name as the tool sends it, e.g. hold.
capture-did-nothing-silent = { $command } を押しましたが、マルチメーターから測定値が届きませんでした
# Yellow: the meter changed function while the step was reading it, so the
# step starts over. $mode is the function the step is for, $other the one
# the meter went to, e.g. Ω and Continuity.
capture-left-mode = サンプリング中にマルチメーターのモードが { $mode } から { $other } に変わったため、取り直しています…
# The reading the step took, in green. $reading is it, e.g. 5.5687 V [AUTO].
capture-we-read = 測定値: { $reading }
# Something the meter sent couldn't be read; the step goes on. $error is
# why.
capture-frame-error = エラー: { $error }
# Yellow: the step read nothing from the meter.
capture-no-response = マルチメーターから応答がありません。
# Yellow, before capture-few-samples on the same line.
capture-few-samples-label = 警告:
# The step got fewer readings than it asks for. $got and $wanted count them.
capture-few-samples = サンプルを { $got }/{ $wanted } 件しか取得できませんでした

## Dim, while a step waits: the meter settled, but not in what the step
## expects. Each says what the reading has and what the step wants.

# $got and $want are functions, in quotes, e.g. "DC V" and "AC V".
capture-mismatch-mode = マルチメーターのモードは { $got } ですが、{ $want } が必要です
# $flag is a status flag the meter shows, e.g. HOLD. $want says whether the
# step wants it on or off; the meter has it the other way.
capture-mismatch-flag =
    { $want ->
        [on] マルチメーターの { $flag } はオフですが、オンにする必要があります
       *[off] マルチメーターの { $flag } はオンですが、オフにする必要があります
    }
# $range is the reading's range, e.g. 22V. $shown is none (the reading has
# no range), auto (a range the meter picked itself) or plain. $want is auto
# (the step wants the meter to pick the range itself) or manual (the user
# picks it).
capture-mismatch-range =
    { $want ->
        [auto]
            { $shown ->
                [none] マルチメーターの表示にはレンジがありませんが、オートレンジが必要です
                [auto] マルチメーターのレンジは { $range } (オート) ですが、オートレンジが必要です
               *[plain] マルチメーターのレンジは { $range } ですが、オートレンジが必要です
            }
       *[manual]
            { $shown ->
                [none] マルチメーターの表示にはレンジがありませんが、マニュアルレンジが必要です
                [auto] マルチメーターのレンジは { $range } (オート) ですが、マニュアルレンジが必要です
               *[plain] マルチメーターのレンジは { $range } ですが、マニュアルレンジが必要です
            }
    }
# $value is what the meter shows, e.g. 12.345 or OL; $shown is absent when
# the reading has no main value, else value. $want is the kind of reading the
# step wants: overload (OL, the meter's over-range sign), negative, finite
# (any number), ncv (the non-contact voltage detector's level 1 or more),
# word (a word the meter shows instead of a number) or zero.
capture-mismatch-value =
    マルチメーターの{ $shown ->
        [absent] 表示にメイン測定値がありません
       *[value] 表示は { $value } です
    }が、{ $want ->
        [overload] OL が必要です
        [negative] 負の測定値が必要です
        [ncv] レベル 1 以上の NCV が必要です
        [word] 数値ではなく文字の表示が必要です
        [zero] ゼロの表示が必要です
       *[finite] 数値の表示が必要です
    }
# $value and $shown as in capture-mismatch-value. $min is the smallest
# number the step wants, positive or negative, e.g. 1.
capture-mismatch-at-least =
    マルチメーターの{ $shown ->
        [absent] 表示にメイン測定値がありません
       *[value] 表示は { $value } です
    }が、絶対値 { $min } 以上の値が必要です

## Asking whether the reading matches the meter's screen. Each prompt waits
## for a line, after a space.

# Dim. $retake is yes when the step can be done over. $no and $redo are the
# keys that answer: n and r.
capture-confirm =
    { $retake ->
        [yes] マルチメーターにこの値が表示されましたか? (Enter = はい、{ $no } = いいえ、{ $redo } = 取り直し)
       *[no] マルチメーターにこの値が表示されましたか? (Enter = はい、{ $no } = いいえ)
    }
# Yellow, when the answer was none of those. $retake, $no and $redo as in
# capture-confirm.
capture-confirm-again =
    { $retake ->
        [yes] { $no } または { $redo } を入力するか、Enter キーを押してください。
       *[no] { $no } を入力するか、Enter キーを押してください。
    }
# The reading didn't match: the user types what the meter showed.
capture-what-shown = マルチメーターには何が表示されましたか?
# Dim: the user typed nothing at capture-what-shown.
capture-recorded-mismatch = 測定値の不一致を記録しました
# Dim. $shown is what the user typed, in quotes.
capture-recorded-shown = マルチメーターの表示 { $shown } を記録しました
# Dim: the step starts over.
capture-retaking = 取り直し中…

## After the first steps, which check that the tool reads the meter right
## ("core semantics": mode, range and flags).

# Green.
capture-core-confirmed = 基本項目の読み取りが正しいことを確認しました。残りのステップは最後に確認します。
# Yellow. $steps lists the ids of the steps that failed, e.g. "dcv, ohm".
capture-core-not-confirmed = { $steps } で基本項目の読み取りが正しいことを確認できませんでした。以降はステップごとに確認します。

## Once the core semantics are confirmed, the tool sets ranges and flags on
## the meter itself by remote control, and prints a dim line for each. Each
## starts with an arrow.

# $mode is the function, e.g. Continuity.
capture-drive-switching = { $mode } に切り替え中 (ツールから送信)
# $mode is the function, e.g. Continuity; $error why the meter refused.
capture-drive-switch-failed = { $mode } に切り替えられませんでした: { $error } (手動で切り替えてください)
# $setting is the setting's name, rel (relative mode): the meter refuses it
# while it shows OL, its over-range sign.
capture-drive-rel-skipped-ol = 測定値が OL のため { $setting } をスキップしました
# $setting is the setting's name, rel (relative mode).
capture-drive-rel-skipped-blank = 測定値がないため { $setting } をスキップしました
# In place of a reading, after the setting and value tried, e.g.
# "range:22V".
capture-drive-no-reading = 測定値なし
# Yellow. $setting is the setting's name, e.g. range or hold.
capture-drive-reset-failed = { $setting } を元に戻せませんでした。マルチメーターのボタンを押してください。
# Yellow: the meter refused too many of the tool's commands.
capture-drive-unreliable = このマルチメーターではリモート操作が不安定なため、以降の自動設定を中止します

## The end-of-run review: the readings the run took without asking, each
## with its number, to check against what the meter showed.

# Bold, between heavy lines.
capture-review-heading = 測定値の確認
# The user types the numbers of the wrong readings, or just presses Enter.
capture-review-prompt = マルチメーターのディスプレイと一致しなかった測定値の番号を入力してください (すべて正しい場合は Enter キー):

## The freeform pass: the user sets the meter to anything the steps didn't
## cover and describes it.

# Bold, between heavy lines; "Part 4" is the run's last part.
capture-freeform-heading = パート 4: 追加のキャプチャ (任意)
capture-freeform-intro = 上記で扱わなかったモードや状態にマルチメーターを設定してください。
# After the capture's id in brackets, e.g. [extra_0]; waits for a line.
# $quit is the word that ends the pass: q.
capture-freeform-prompt = マルチメーターの設定内容を入力してください (「{ $quit }」で終了):

## The last check: does the tool's automatic detection recognise the
## meter? The user switches it off and on, and the tool detects it afresh.

# Dim. $bridge is the chip in the cable or the Bluetooth link, e.g. CP2110.
capture-detect-confirmed = { $bridge } 経由の自動検出は確認済みのため、チェックをスキップします。
# Bold, between heavy lines.
capture-detect-heading = 最後の確認: 自動検出
capture-detect-off-cable = マルチメーターの電源を切り、USB ケーブルを抜いてください。
capture-detect-off = マルチメーターの電源を切ってください。
# Waits for one key. $skip is the key that skips: s.
capture-detect-off-prompt = 電源を切ったら Enter キーを押してください (スキップするには { $skip }):
# $mode is the function to turn the dial to, DC V.
capture-detect-on-cable = ケーブルを接続し直し、ファンクションスイッチを { $mode } に合わせてマルチメーターの電源を入れてください。
# $mode is the function to turn the dial to, DC V.
capture-detect-on = ファンクションスイッチを { $mode } に合わせて、もう一度電源を入れてください。
# Followed by the meter's own steps for switching its data transmission on.
capture-detect-transmission = マルチメーターのデータ転送がオフになった場合は、もう一度オンにしてください:
# Waits for one key. $skip is the key that skips: s.
capture-detect-ready-prompt = 準備ができたら Enter キーを押してください (スキップするには { $skip }):
# Dim.
capture-detect-checking = 自動検出を確認中…
# Detection found the meter; the tool reads it once more. $instruction is
# what to do, the step's own instruction, in English.
capture-detect-now = 次の操作: { $instruction }
# Dim. $skip is the key that skips: s.
capture-detect-read-keys = 測定値が安定すると自動で取得します。Enter キーで今すぐ取得、{ $skip } でスキップします。

## The line the check ends on: dim when detection found the meter, yellow
## with advice when it didn't. $found is the model detection picked, e.g.
## UNI-T UT61E+; $reading what the meter then read, e.g. 5.5687 V [AUTO].
## The advice names the meter the run was for: $device is its id on the
## command line, e.g. ut61eplus, and $meter its name, e.g. UNI-T UT61E+.

# $reason is why the link didn't open again, in English, or
# capture-detect-no-reason.
capture-detect-not-reopened = リンクをもう一度確立できなかったため、自動検出を確認できませんでした: { $reason }
capture-detect-no-reason = 理由は示されていません
capture-detect-found = { $found } を自動検出しました。
capture-detect-found-read = { $found } を自動検出し、{ $reading } を読み取りました。
capture-detect-found-read-other = { $found } を自動検出しましたが、その後の読み取り結果は { $reading } でした。引き続き --device { $device } を使用するか、dmm-gui で { $meter } を選択してください。
capture-detect-found-read-nothing = { $found } を自動検出しましたが、その後は何も読み取れませんでした。引き続き --device { $device } を使用するか、dmm-gui で { $meter } を選択してください。
capture-detect-picked-other = 自動検出はこのマルチメーターを { $found } と判定しました。引き続き --device { $device } を使用するか、dmm-gui で { $meter } を選択してください。
capture-detect-not-found = このマルチメーターを自動検出できませんでした。引き続き --device { $device } を使用するか、dmm-gui で { $meter } を選択してください。

## The end of the run.

# After "Warning:" (common.ftl): the detection check stopped. $error is why.
capture-detect-failed = 自動検出を確認できませんでした: { $error }
# Bold and green, between equals signs.
capture-complete = キャプチャが完了しました。
# $path is the report file, in bold.
capture-report-saved = レポートの保存先: { $path }
# The run followed a list of steps a maintainer wrote (a "plan"). $plan is
# its file; $covered and $total count its steps.
capture-plan-covered = プラン { $plan }: { $total } ステップ中 { $covered } ステップをキャプチャしました
capture-nothing-new = 検証されていないステップはキャプチャされなかったため、新たに報告する内容はありません。
# $covered and $total count the meter's steps that hardware hasn't confirmed
# yet; $meter is its name, e.g. UNI-T UT61E+.
capture-covered = { $meter } の検証されていないステップ { $total } 件のうち { $covered } 件を実行しました。
# $resume is the key that resumes, r (capture-resume-prompt).
capture-not-finished = 完了していません。再開するには、同じコマンドをもう一度実行して { $resume } を押してください。
# $flag is the option as typed, --steps <ids>.
capture-steps-hint = { $flag } を使用すると、指定したステップだけを実行できます。
# $url is the web address of the meter's issue on GitHub.
capture-attach = レポートを { $url } に添付してください。

## --export-instructions: the step instructions written to a file the user
## translates for themselves, then passes back with --instructions. The
## file is Fluent source, like this one.

# The comment at the top of the exported file, a line of it per line here.
# $meter is the meter's name, e.g. Brymen BM829s/BM827s/BM822s/BM821s.
# Keep "=", "-device" and "#" as they are: they are the file's
# syntax.
capture-instructions-header =
    { $meter } のキャプチャ ステップの説明です。翻訳してください。
    各「=」の後のテキストを、同じ行のまま翻訳文に置き換えてください。
    「=」の前のステップ ID、「-device」の行、「#」で始まる行は変更しないでください。
    削除したステップは英語で表示されます。
# A line of the header, when the steps quote words from the meter: its
# key labels and what its screen shows. $marks lists them, e.g. "AC, HOLD,
# MIN/MAX, SELECT". A translation, by a person or a machine, must leave them
# as they are, so say so plainly.
capture-instructions-keep = 次の語は、マルチメーターのキーやディスプレイの表記のまま変更しないでください: { $marks }
# The header's last line. $command is the command that runs the capture
# with the translated file.
capture-instructions-run = その後、次のコマンドを実行してください: { $command }
# In place of writing a file, for a meter with no steps of its own (a
# simulated one). $meter is its name, e.g. Mock UT61E+.
capture-instructions-none = { $meter } には専用のキャプチャ ステップがないため、翻訳するものはありません。
# After writing the file. $count is the number of steps; $path the file.
capture-instructions-written = ステップの説明 { $count } 件を { $path } に書き込みました。
# $command is the command that runs the capture with the translated file.
capture-instructions-next = 翻訳してから、次のコマンドを実行してください: { $command }
# Yellow, before the first step: steps whose English changed after the
# file was exported, so they show a translation of an older wording.
# $path is the file; $steps lists the step ids, e.g. "dcv, hold".
capture-instructions-changed = { $path } のエクスポート後に変更されたステップには、古い英文の翻訳が表示されます: { $steps }。新しい英文を確認するには、ファイルをもう一度エクスポートしてください。
# One problem in the file, on a line of its own under
# capture-error-instructions-unreadable. $line is a line number.
capture-instructions-problem-syntax = { $line } 行目: ステップ ID、「=」、テキストの形式になっていません
# Another problem. $id is a step id, e.g. dcv.
capture-instructions-problem-not-text = { $id }: テキストに波かっこが含まれています。削除してください
# Another problem. $id is a step id, e.g. dcv.
capture-instructions-problem-twice = { $id }: ステップが 2 回記述されています
# Another problem: the file couldn't be read at all.
capture-instructions-problem-unparsable = ファイルを読み取れません。ファイル内の波かっこと引用符をご確認ください

## The line that closes a failed run, after "Error:" (common.ftl).

capture-error-not-responding = マルチメーターが応答しません
capture-error-input-gone = キーボード入力を受け付けなくなりました。キャプチャを終了して、もう一度実行してください
# --steps named steps this meter doesn't have. $steps lists them, e.g.
# "dcmv, tmep"; $command is the command line that lists the meter's steps.
capture-error-unknown-steps =
    不明なキャプチャ ステップ: { $steps }
    このデバイスが対応するステップを確認するには、{ $command } を実行してください。
# The --instructions file can't be read. $path is the file; $problems the
# problems, one per line (capture-instructions-problem-*).
capture-error-instructions-unreadable =
    { $path } をキャプチャの説明として読み取れません:
    { $problems }
# The --instructions file was exported for another meter. $path is the
# file; $named the meter id in it, e.g. ut61eplus; $device this run's, e.g.
# bm82x; $command the command that exports a file for this meter.
capture-error-instructions-device = { $path } は { $device } ではなく { $named } 用の説明です。このマルチメーターの説明は { $command } でエクスポートしてください。
# The --instructions file names steps this run doesn't have, most likely a
# step id that was translated. $path is the file; $ids the ids not found,
# e.g. "vcc, retencao"; $known the ids the run has, e.g. "dcv, acv, hold".
capture-error-instructions-ids = { $path } に、今回の実行に含まれないステップが指定されています: { $ids }。ステップ ID はエクスポートされたときのまま変更しないでください。有効なステップ ID: { $known }
