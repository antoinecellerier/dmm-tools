### dmm-cli's --help: what each command and option does.
###
### Generated from the doc comments in crates/dmm-cli/src/cli.rs by
### `UPDATE_L10N=1 cargo test -p dmm-cli cli_help_ftl`: edit those, not this
### file. The run names each ID whose English changed. When the meaning changed
### too, delete that message from every language's cli-help.ftl, so it shows in
### English until it is translated again.
###
### IDs follow the command tree: read.about is what the read command does,
### read--interval-ms.help what its --interval-ms option does. clap wraps each
### paragraph to the terminal's width: keep a paragraph on one line. Option
### names (--device), values (auto, mock) and command lines ('dmm-cli list')
### stay as they are. clap's own words around these texts (Usage:, Options:,
### [default: …], [possible values: …]) are in shared.ftl.

# dmm-cli
dmm-cli =
    .about = USB および Bluetooth デジタル マルチメーター用の CLI ツール
# dmm-cli --device
dmm-cli--device =
    .help = 接続するデバイス [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...]。省略すると ~/.config/dmm-tools/settings.json (dmm-gui が書き込む) の `device_family`、次に USB ケーブル経由でマルチメーターを検出する `auto` を使用する
# dmm-cli --adapter
dmm-cli--adapter =
    .help = 複数のアダプターに接続できるとき、特定のアダプターを選択する。'dmm-cli list' が表示する USB ケーブルのシリアル番号か HID パス、または Bluetooth アダプターかマルチメーターのアドレスを指定する
# dmm-cli --no-bluetooth
dmm-cli--no-bluetooth =
    .help = 今回の実行に限り Bluetooth スキャンをオフにする: アダプターやマルチメーターを一切スキャンせず、'list' はケーブルのみを表示する。保存された 'bluetooth' 設定より優先される。--adapter に指定したアドレスはそのまま開かれる
# dmm-cli --lang
dmm-cli--lang =
    .help = メッセージとヘルプの言語。システムの言語を使うには `system` を指定する。省略すると設定ファイルの `language` (dmm-gui の [言語] 行で書き込まれる)、次にシステムの言語を使用する
# dmm-cli list
list =
    .about = 接続されている USB ケーブルと、通信範囲内の Bluetooth アダプターおよびマルチメーターを一覧表示する
# dmm-cli info
info =
    .about = 接続してデバイス情報を表示する
# dmm-cli read
read =
    .about = 測定値を連続して読み取る
# dmm-cli read --interval-ms
read--interval-ms =
    .help = 指定した間隔 (ミリ秒) ごとに最大 1 件の測定値を保持する (0 = マルチメーターが送るすべての測定値)
# dmm-cli read --format
# .value-… says what that value does, in --help's list of values.
read--format =
    .help = 出力形式 [既定値: text、または -o の拡張子が示す形式]
    .value-replay = マルチメーターが送ったフレームそのもの (--replay で再生できる)
# dmm-cli read --output
read--output =
    .help = 出力ファイル (指定しない場合は標準出力)。名前なしで指定すると、マルチメーター、モード、実行開始時刻からファイル名を付ける
# dmm-cli read --count
read--count =
    .help = 測定値の件数 (0 = 無制限: Ctrl+C、または --replay ファイルの終わりで停止)
# dmm-cli read --no-reconnect
read--no-reconnect =
    .help = マルチメーターとのリンクが切断されたら、再接続せずに実行を終了する
# dmm-cli read --integrate
read--integrate =
    .help = 時間積分の累計を表示する (電流モードでは電荷、電圧では V·s)
# dmm-cli read --scale
read--scale =
    .help = 基本単位 (V, A, Ω, …) で取得した測定値に FACTOR を掛ける。10 mV/A のクランプは --scale 100、100:1 のプローブは --scale 100
# dmm-cli read --offset
read--offset =
    .help = スケーリング後に VALUE を加える (--scale 1.8 と組み合わせて 32 を加えると、°C が °F になる)
# dmm-cli read --unit
read--unit =
    .help = スケーリングした測定値に、マルチメーターの基本単位の代わりにこの単位を付ける
# dmm-cli read --alarm-high
read--alarm-high =
    .help = 測定値が VALUE を上回ったときに警告する。VALUE は基本単位 (V, A, Ω, …)、スケーリングした測定値では --unit の単位
# dmm-cli read --alarm-low
read--alarm-low =
    .help = 測定値が VALUE を下回ったときに警告する (--alarm-high と同様)
# dmm-cli read --alarm-hysteresis
read--alarm-hysteresis =
    .help = 同じ制限値で再びアラームが鳴るまでに、測定値が内側へ戻る必要のある幅: 制限値の単位の値、または制限値に対する割合 (`1%`) [既定値: マルチメーターの最終桁の数カウント]
# dmm-cli read --alarm-bell
read--alarm-bell =
    .help = アラームのたびに端末のベルを鳴らす
# dmm-cli read --mock-mode
read--mock-mode =
    .help = シミュレーション デバイスを特定のモードに固定する (--device mock のときのみ)。指定しない場合は設定ファイルのシミュレーション モードが適用され、それもなければすべてのモードを順に切り替える
# dmm-cli read --replay
read--replay =
    .help = マルチメーターを開く代わりに、--format replay で書き出したファイルを再生する。ファイルの終わりで終了する
# dmm-cli read --import
read--import =
    .help = マルチメーターを開く代わりに、エクスポートした CSV, JSON またはリプレイ ファイルを待たずに読み込む: 変換や概要の表示に使う
# dmm-cli command
command =
    .about = マルチメーターにボタン操作のコマンドを送信する。引数なしで実行すると、選択したデバイスで使用できるコマンドを一覧表示する
# dmm-cli command <ACTION>
command--action =
    .help = コマンド名 (引数なしで実行すると、使用できるコマンドを表示する)
# dmm-cli get
get =
    .about = マルチメーターの各設定について、現在の状態から切り替えられる値を一覧表示する。引数なしで実行すると、選択肢のあるすべての設定を表示する
# dmm-cli get <SETTING>
get--setting =
    .help = 一覧表示する設定 (省略するとすべて)
# dmm-cli get --format
get--format =
    .help = 出力形式
# dmm-cli get --mock-mode
get--mock-mode =
    .help = シミュレーション デバイスを特定のモードに固定する (--device mock のときのみ)。指定しない場合は設定ファイルのシミュレーション モードが適用され、それもなければすべてのモードを順に切り替える
# dmm-cli set
set =
    .about = マルチメーターに触れずに設定を 1 つ切り替える。値を指定せずに実行すると、その設定で現在の状態から選べる値を一覧表示する
# dmm-cli set <SETTING>
set--setting =
    .help = 切り替える設定
# dmm-cli set <CHOICE>
set--choice =
    .help = 一覧に表示される値のラベル、またはそれを一意に特定できる一部 (指定せずに実行すると一覧を表示する)
# dmm-cli set --mock-mode
set--mock-mode =
    .help = シミュレーション デバイスを特定のモードに固定する (--device mock のときのみ)。指定しない場合は設定ファイルのシミュレーション モードが適用され、それもなければすべてのモードを順に切り替える
# dmm-cli debug
debug =
    .about = プロトコル デバッグ用の生の 16 進ダンプ モード
# dmm-cli debug --count
debug--count =
    .help = 送信する要求の数 (0 = 無制限)
# dmm-cli debug --interval-ms
debug--interval-ms =
    .help = 指定した間隔 (ミリ秒) ごとに最大 1 件の測定値を保持する (0 = すべての測定値)
# dmm-cli completions
completions =
    .about = シェル補完を生成する
# dmm-cli completions <SHELL>
completions--shell =
    .help = 補完を生成するシェル
# dmm-cli capture
capture =
    .about = バグ報告と検証のためのガイド付きプロトコル キャプチャ
# dmm-cli capture --output
capture--output =
    .help = 出力ファイル (既定値: capture-<device>.yaml)。自動で付くファイル名より優先される
# dmm-cli capture --steps
capture--steps =
    .help = 特定のステップのみ実行する (カンマ区切りの ID。例: 「dcmv,temp,duty」)
# dmm-cli capture --unverified
capture--unverified =
    .help = 実機でまだ確認されていないステップと、最後の自由操作のステップのみ実行する
# dmm-cli capture --plan
capture--plan =
    .help = デバイス既定のステップ一覧の代わりに、メンテナーのプラン ファイルにあるステップを実行する
# dmm-cli capture --sniff
capture--sniff =
    .help = パーサーの結果を一切信用しない: 生のバイトの変化でステップを検出し、1 つずつ確認する
# dmm-cli capture --no-drive
capture--no-drive =
    .help = 各モード ステップの後に、ツールがレンジやフラグを自動で設定しないようにする
# dmm-cli capture --settle
capture--settle =
    .help = 各ステップのサンプリング前に MS ミリ秒待つ (測定値の安定が遅い場合)
# dmm-cli capture --list-steps
capture--list-steps =
    .help = 使用できるすべてのステップ ID を一覧表示して終了する
# dmm-cli capture --format
capture--format =
    .help = --list-steps の一覧の出力形式 (md は Issue のチェックリスト)
# dmm-cli capture --export-instructions
capture--export-instructions =
    .help = ステップの説明を FILE に書き出して終了する (翻訳して --instructions で使うため)
# dmm-cli capture --instructions
capture--instructions =
    .help = --export-instructions で書き出したものを翻訳した FILE から、ステップの説明を表示する
