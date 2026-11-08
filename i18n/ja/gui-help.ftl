### dmm-gui's --help: what the program and each option do.
###
### Generated from the doc comments in crates/dmm-gui/src/main.rs by
### `UPDATE_L10N=1 cargo test -p dmm-gui gui_help_ftl`: edit those, not this
### file. The run names each ID whose English changed. When the meaning changed
### too, delete that message from every language's gui-help.ftl, so it shows in
### English until it is translated again.
###
### IDs follow the options: dmm-gui--replay.help is what --replay does. clap
### wraps each paragraph to the terminal's width: keep a paragraph on one line.
### Option names (--device), values (auto, mock) and command lines ('dmm-cli
### list') stay as they are. The help built as the program runs is in
### gui/command-line.ftl, and clap's own words around these texts (Usage:,
### Options:, [possible values: …]) are in shared.ftl.

# dmm-gui
dmm-gui =
    .about = USB および Bluetooth デジタル マルチメーター用の GUI アプリケーション
# dmm-gui --device
dmm-gui--device =
    .help = 接続するデバイス [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...]
# dmm-gui --mock-mode
dmm-gui--mock-mode =
    .help = シミュレーション デバイスを特定のモードに固定する (--device mock を指定したものとして扱う)
# dmm-gui --theme
dmm-gui--theme =
    .help = テーマを指定する (保存された設定より優先される)
# dmm-gui --renderer
dmm-gui--renderer =
    .help = 使用するグラフィックス レンダラーを指定する
# dmm-gui --adapter
dmm-gui--adapter =
    .help = 複数のアダプターに接続できるとき、特定のアダプターを選択する。'dmm-cli list' が表示する USB ケーブルのシリアル番号か HID パス、または Bluetooth アダプターかマルチメーターのアドレスを指定する
# dmm-gui --no-bluetooth
dmm-gui--no-bluetooth =
    .help = 今回の実行に限り Bluetooth スキャンをオフにする。[Bluetooth デバイスを探す] 設定より優先される。--adapter に指定したアドレスはそのまま開かれる
# dmm-gui --replay
dmm-gui--replay =
    .help = マルチメーターに接続する代わりに、'dmm-cli read --format replay' または [エクスポート…] → [リプレイ…] で書き出したファイルを再生する
# dmm-gui --import
dmm-gui--import =
    .help = マルチメーターに接続する代わりに、エクスポートした CSV, JSON またはリプレイ ファイルをマーカーも含めて開く
# dmm-gui --completions
dmm-gui--completions =
    .help = シェル補完スクリプトを表示して終了する
