### The colour and theme rows near the foot of the settings panel, which
### the ⚙ button at the top bar's right end opens (assets/gui-settings.webp):
### the Theme and Colors rows of chips, the Customize colors section with a
### swatch for every colour of the palette and the editor each swatch opens,
### and Save as theme with the messages it shows when it is done.
###
### Theme names (Midnight, Bubble Gum, a theme the user saved) are kept as
### they are: they are file names and command-line values too. Text in
### parentheses starting with -- names a command-line option, kept as it
### is, that set the row for this session only. Rows wrap on a narrow
### window, so keep the chips and swatch names short.

## The Theme row: the Dark, Light and System modes, then the named themes.

colors-theme = テーマ:
# The Dark, Light and System chips. $state is pinned when --theme picked
# this mode for the session, customized when the user changed some of its
# colours (Dark and Light only), plain otherwise.
colors-theme-dark =
    { $state ->
        [pinned] ダーク (--theme)
        [customized] ダーク (カスタマイズ済み)
       *[plain] ダーク
    }
    .hover = GUI 全体でダーク モードを使用する
# $state as for Dark.
colors-theme-light =
    { $state ->
        [pinned] ライト (--theme)
        [customized] ライト (カスタマイズ済み)
       *[plain] ライト
    }
    .hover = GUI 全体でライト モードを使用する
# $state as for Dark, never customized: System has no colours of its own.
colors-theme-system =
    { $state ->
        [pinned] システム (--theme)
       *[plain] システム
    }
    .hover = デスクトップのライト/ダーク設定に従う (設定がない場合はダーク)
# A named theme's chip when --theme picked it for the session; otherwise
# the chip is the theme's name alone. $name is the theme's name, e.g.
# Midnight.
colors-theme-named-pinned = { $name } (--theme)
# A named theme's chip after the user changed some of its colours. $name is
# the theme's name.
colors-theme-named-customized = { $name } (カスタマイズ済み)
# A built-in theme's hover. $mode is dark or light: the mode the theme is
# drawn in.
colors-theme-builtin =
    .hover =
        { $mode ->
            [light] 組み込みのライト テーマ
           *[dark] 組み込みのダーク テーマ
        }
# The hover of a theme the user added to the themes folder. $mode is dark
# or light; $path is the theme's file, in full.
colors-theme-file =
    .hover =
        { $mode ->
            [light] { $path } のライト テーマです。このテーマを外すには、ファイルを削除してください。
           *[dark] { $path } のダーク テーマです。このテーマを外すには、ファイルを削除してください。
        }
# The same when the themes folder can't be found, with $mode as there.
# $file is the theme's file name.
colors-theme-file-unplaced =
    .hover =
        { $mode ->
            [light] { $file } のライト テーマ
           *[dark] { $file } のダーク テーマ
        }

## After the Theme row's chips: the theme files in the themes folder that
## could not be offered, with why on hover, one file to a line.

# $count is how many files were skipped.
colors-skipped = テーマ ファイル { $count } 個をスキップ
# The hover's first line. $folder is the themes folder, in full.
colors-skipped-folder = { $folder } 内:
# A line of the hover. $file is a file name; $reason one of the reasons
# below, starting lower case.
colors-skipped-file = { $file }: { $reason }
# The line for the files left unread because the folder holds too many.
# $count is how many; $max how many files are read, e.g. 100.
colors-skipped-too-many = ほか { $count } 個: ファイル数の上限 ({ $max } 個) を超えています

## Why a theme file can't be used: after its name in the hover above, or
## after "Not saved:" when Save as theme would write such a file. Each
## starts lower case. Others come from the file system or the file reader
## and are shown in their own words.

colors-issue-not-a-file = ファイルではありません
# $size is the largest file read, in KiB (kibibytes), e.g. 64.
colors-issue-too-big = { $size } KiB を超えています
colors-issue-name-empty = 名前が空です
# $max is the most characters a name may have, e.g. 32.
colors-issue-name-too-long = 名前が { $max } 文字を超えています
# Invisible characters, or $ or `.
colors-issue-name-unusable = テーマ名に使用できない文字が含まれています
# The theme's name is Dark, Light or System, which the Theme row's own
# chips go by. $name is the name as the file has it; put it in your
# language's quotes.
colors-issue-name-reserved = 「{ $name }」は [テーマ] の行で使われている名前です
# $what is text or button: the theme's text on its background, or its
# button captions on the button.
colors-issue-not-opaque =
    { $what ->
        [button] ボタンのテキストが不透明ではありません
       *[text] 背景上のテキストが不透明ではありません
    }
# The colours can't be read on each other. $what is as above; $ratio is
# their contrast, e.g. 1.8, and $floor the least allowed, e.g. 3.
colors-issue-low-contrast =
    { $what ->
        [button] ボタンのテキストとボタン背景のコントラストが { $ratio }:1 で、下限の { $floor }:1 未満です
       *[text] テキストと背景のコントラストが { $ratio }:1 で、下限の { $floor }:1 未満です
    }
# The theme's text on its frame, the surface of the top bar and Settings.
colors-issue-frame-not-opaque = 枠上のテキストが不透明ではありません
# As above, on the frame. $ratio and $floor are as above.
colors-issue-frame-low-contrast = テキストと枠のコントラストが { $ratio }:1 で、下限の { $floor }:1 未満です
# Another file in the folder gives its theme the same name. $name is the
# name as the file has it, to put in your language's quotes; $file the
# other file's name.
colors-issue-name-taken = 「{ $name }」はすでに { $file } の名前です
colors-issue-file-name-unprintable = ファイル名に表示できない文字が含まれています
# The folder holds too many theme files. $max is how many are read, e.g.
# 100.
colors-issue-too-many = ファイル数の上限 ({ $max } 個) を超えています

## The Colors row: the palette presets under Dark and Light.

colors-colors = 色:
# The preset chips. $state is customized on the picked chip while the user
# has changed some of its colours, plain otherwise.
colors-preset-default =
    { $state ->
        [customized] 既定 (カスタマイズ済み)
       *[plain] 既定
    }
    .hover = 普段使い向けに調整したバランスのよいパレット
# $state as for Default.
colors-preset-high-contrast =
    { $state ->
        [customized] ハイ コントラスト (カスタマイズ済み)
       *[plain] ハイ コントラスト
    }
    .hover = 明るい照明やプロジェクター向けの最大コントラストのパレット
# $state as for Default. Protan and deutan are the red-green colour-vision
# deficiencies.
colors-preset-colorblind =
    { $state ->
        [customized] 色覚サポート (カスタマイズ済み)
       *[plain] 色覚サポート
    }
    .hover = P 型/D 型色覚でも区別しやすいパレット
# After the chips while there are changes a click would lose.
colors-preset-clears = プリセットを選択すると、カスタマイズした色は消去されます。

## The Customize colors section, folded under its header until opened.

colors-customize = 色のカスタマイズ
    .hover = 選択したプリセットまたはテーマの色を 1 色ずつ変更する
# Beside the header: whose colours the swatches change. $mode is dark or
# light.
colors-editing =
    { $mode ->
        [light] (ライト テーマの色を編集中)
       *[dark] (ダーク テーマの色を編集中)
    }
# The same while a named theme is on. $name is the theme's name.
colors-editing-named = ({ $name } の色を編集中)

# The captions at the start of the swatch rows.
colors-group-ui = UI:
colors-group-graph = グラフ:
colors-group-status = 状態:
# The arrow after a colour that others follow, before those it leads.
# $anchor is the leading colour's name, as its swatch below has it.
colors-follow-arrow =
    .hover = これらの色は、個別に選択するまで、差を保ったまま { $anchor } に追従します。

## The swatches, each named beside it, in the rows the section shows them.

colors-swatch-background = 背景
    .hover = パネルの背景色
colors-swatch-frame = 枠
    .hover = トップ バーと測定値パネルの背景 (未設定の場合はパネルの背景)
colors-swatch-button = ボタン
    .hover = ボタンの背景色
# Lines between and around the panels.
colors-swatch-border = 境界線
    .hover = 区切り線、パネルの端、枠、プロットの外枠
# Short for the plot's background.
colors-swatch-plot-background = プロット背景
    .hover = グラフのプロット領域の背景色
colors-swatch-text = テキスト
    .hover = 主なテキストの色
# The meter's value as the reading panel shows it, in large type.
colors-swatch-reading = 測定値
    .hover = 測定値とサブ表示値 (未設定の場合はテキストの色)
# Text drawn fainter than the rest.
colors-swatch-weak-text = 補助テキスト
    .hover = 補助テキスト: モード行、サブ表示値のラベル、ヒント、キャプション
colors-swatch-heading = 見出し
    .hover = パネルの見出し: 仕様、統計、表示範囲 (未設定の場合は補助テキストの色)
# The lines that follow the pointer over the graph.
colors-swatch-crosshair = 十字線
    .hover = グラフ上でポインターに追従する十字線の色
colors-swatch-accent = アクセント
    .hover = モード バッジ、リンク、選択したトグル/チップ/テキストの背景
# The graph's main line, the readings over time.
colors-swatch-data-line = データ線
    .hover = グラフのデータ線の色
# The band between the lowest and highest readings.
colors-swatch-envelope = エンベロープ
    .hover = 最小/最大エンベロープの塗りつぶしの色
# The lines of the other values a meter shows at once beside the main one.
colors-swatch-overlay-1 = オーバーレイ 1
    .hover = プロットした系列と一緒に描く 1 つ目のサブ表示値の線の色
colors-swatch-overlay-2 = オーバーレイ 2
    .hover = プロットした系列と一緒に描く 2 つ目のサブ表示値の線の色
colors-swatch-overlay-3 = オーバーレイ 3
    .hover = プロットした系列と一緒に描く 3 つ目のサブ表示値の線の色
# The frame the overview strip under the graph draws around the part on
# show.
colors-swatch-viewport = ビューポート
    .hover = ミニマップのビューポート枠の色
# A stretch with no readings.
colors-swatch-gap = 欠落
    .hover = 記録データの欠落を示す色
# The line at the average of the readings.
colors-swatch-mean = 平均値
    .hover = 平均値の線の色
# Short for reference: a line the user sets at a value.
colors-swatch-ref = 基準
    .hover = 基準線の色
# Where the readings cross a set level.
colors-swatch-crossing = 交差
    .hover = トリガー交差マーカーの色
colors-swatch-cursor = カーソル
    .hover = 測定カーソルの色
# The numbered markers the user drops on the graph.
colors-swatch-marker = マーカー
    .hover = マーカーの線、フラグ、番号
# The status colours, after the states they mark. Connected is the top
# bar's status word, in quotes in the hover.
colors-swatch-connected = 接続済み
    .hover = 「接続済み」状態の色
colors-swatch-warning = 警告
    .hover = 警告状態の色
colors-swatch-error = エラー
    .hover = エラー状態の色
colors-swatch-inactive = 非アクティブ
    .hover = 非アクティブ/切断状態の色

## Beside a swatch.

# A swatch's name for a screen reader while it follows another colour.
# $color is its name, $anchor the name of the colour it follows.
colors-swatch-linked = { $color }、{ $anchor } に追従
# A small button beside a colour that was set on its own though it can
# follow another; ↺ is part of it. $color is the colour's name, $anchor
# the name of the one it would follow.
colors-relink = ↺ 再リンク
    .hover = もう一度 { $anchor } に追従する
    .a11y = { $color } を { $anchor } に再リンクする
# A warning after a colour the user made hard to see on another. $ratio is
# their contrast, e.g. 2.1; $ground the other colour's name; $floor the
# least contrast it needs, e.g. 4.5.
colors-low-contrast = { $ground } に対して { $ratio }:1
    .hover = { $ground } に対するコントラストが、この色に必要な { $floor }:1 を下回っています
# A warning after a graph colour the user made hard to tell from another.
# $other is the other colour's name; $distance how far apart they are and
# $floor how far apart they need to be, as whole numbers. ΔE is the
# measure of colour difference.
colors-too-close = { $other } に近い
    .hover = 色差が { $distance } (ΔE) しかありません。グラフの色の区別には { $floor } 必要です。

## The editor a swatch opens: a colour picker, then the colour as text.

# The colour written as # and six hexadecimal digits.
colors-hex = Hex
    .hover = この色をコピーするか、#RRGGBB 形式で色を貼り付ける

## Under the swatches, once a colour was changed.

colors-reset = 色のリセット
    .hover = このモードの色への変更を破棄する
# The same while a named theme is on. $name is the theme's name.
colors-reset-named =
    .hover = { $name } の色への変更を破棄する
colors-save = テーマとして保存…
    .hover = これらの色をテーマ ファイルとして保存します。テーマ フォルダーに保存したファイルは [テーマ] の行に表示されます。
# The file type in the save dialog's list of types.
colors-save-file-type = テーマ

## What Save as theme says when it is done, in a toast: a short message that
## floats under the top bar's right end for a few seconds.

# $name is the theme's name, from the file name the user typed.
colors-saved = { $name } をテーマ フォルダーに保存しました。
# Saved outside the themes folder. $file is the file name.
colors-saved-elsewhere = { $file } を保存しました。テーマとして使用するには、テーマ フォルダーにコピーしてください。
# The save dialog closed without an answer.
colors-save-dialog-failed = 保存されませんでした: 保存ダイアログでエラーが発生しました。
# A name typed without .json gets it added, and the file of that name
# already exists. $path is that file, in full.
colors-save-exists = 保存されませんでした: { $path } はすでに存在します。置き換えるにはダイアログでそのファイルを選択してください。
# Another file in the themes folder gives its theme the same name. $name is
# the name; $file the other file's name.
colors-save-name-taken = 保存されませんでした: { $name } はすでにテーマ フォルダー内の { $file } の名前です。別のファイル名を選択してください。
# The theme would be a file the folder can't use. $reason is one of the
# reasons above, starting lower case.
colors-save-invalid = 保存されませんでした: { $reason }
# Writing the file failed. $path is the file, in full; $error why, in the
# words of the system that raised it.
colors-save-failed = { $path } を保存できませんでした: { $error }
