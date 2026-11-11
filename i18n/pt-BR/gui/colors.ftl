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

colors-theme = Tema:
# The Dark, Light and System chips. $state is pinned when --theme picked
# this mode for the session, customized when the user changed some of its
# colours (Dark and Light only), plain otherwise.
colors-theme-dark =
    { $state ->
        [pinned] Escuro (--theme)
        [customized] Escuro (personalizado)
       *[plain] Escuro
    }
    .hover = Usar o modo escuro em toda a interface
# $state as for Dark.
colors-theme-light =
    { $state ->
        [pinned] Claro (--theme)
        [customized] Claro (personalizado)
       *[plain] Claro
    }
    .hover = Usar o modo claro em toda a interface
# $state as for Dark, never customized: System has no colours of its own.
colors-theme-system =
    { $state ->
        [pinned] Sistema (--theme)
       *[plain] Sistema
    }
    .hover = Seguir a configuração clara/escura da área de trabalho (Escuro se não houver nenhuma)
# A named theme's chip when --theme picked it for the session; otherwise
# the chip is the theme's name alone. $name is the theme's name, e.g.
# Midnight.
colors-theme-named-pinned = { $name } (--theme)
# A named theme's chip after the user changed some of its colours. $name is
# the theme's name.
colors-theme-named-customized = { $name } (personalizado)
# A built-in theme's hover. $mode is dark or light: the mode the theme is
# drawn in.
colors-theme-builtin =
    .hover =
        { $mode ->
            [light] Tema claro integrado
           *[dark] Tema escuro integrado
        }
# The hover of a theme the user added to the themes folder. $mode is dark
# or light; $path is the theme's file, in full.
colors-theme-file =
    .hover =
        { $mode ->
            [light] Tema claro, de { $path }. Exclua o arquivo para removê-lo.
           *[dark] Tema escuro, de { $path }. Exclua o arquivo para removê-lo.
        }
# The same when the themes folder can't be found, with $mode as there.
# $file is the theme's file name.
colors-theme-file-unplaced =
    .hover =
        { $mode ->
            [light] Tema claro, de { $file }
           *[dark] Tema escuro, de { $file }
        }

## After the Theme row's chips: the theme files in the themes folder that
## could not be offered, with why on hover, one file to a line.

# $count is how many files were skipped.
colors-skipped =
    { $count ->
        [one] { $count } arquivo de tema ignorado
       *[other] { $count } arquivos de tema ignorados
    }
# The hover's first line. $folder is the themes folder, in full.
colors-skipped-folder = Em { $folder }:
# A line of the hover. $file is a file name; $reason one of the reasons
# below, starting lower case.
colors-skipped-file = { $file }: { $reason }
# The line for the files left unread because the folder holds too many.
# $count is how many; $max how many files are read, e.g. 100.
colors-skipped-too-many =
    { $count ->
        [one] Mais { $count }: acima do limite de { $max } arquivos
       *[other] Mais { $count }: acima do limite de { $max } arquivos
    }

## Why a theme file can't be used: after its name in the hover above, or
## after "Not saved:" when Save as theme would write such a file. Each
## starts lower case. Others come from the file system or the file reader
## and are shown in their own words.

colors-issue-not-a-file = não é um arquivo
# $size is the largest file read, in KiB (kibibytes), e.g. 64.
colors-issue-too-big = acima de { $size } KiB
colors-issue-name-empty = o nome está vazio
# $max is the most characters a name may have, e.g. 32.
colors-issue-name-too-long = o nome tem mais de { $max } caracteres
# Invisible characters, or $ or `.
colors-issue-name-unusable = o nome tem caracteres não permitidos em nomes de tema
# The theme's name is Dark, Light or System, which the Theme row's own
# chips go by. $name is the name as the file has it; put it in your
# language's quotes.
colors-issue-name-reserved = “{ $name }” já é usado na linha Tema
# $what is text or button: the theme's text on its background, or its
# button captions on the button.
colors-issue-not-opaque =
    { $what ->
        [button] o texto sobre o botão não é opaco
       *[text] o texto sobre o plano de fundo não é opaco
    }
# The colours can't be read on each other. $what is as above; $ratio is
# their contrast, e.g. 1.8, and $floor the least allowed, e.g. 3.
colors-issue-low-contrast =
    { $what ->
        [button] o texto sobre o botão tem contraste de { $ratio }:1, abaixo do mínimo de { $floor }:1
       *[text] o texto sobre o plano de fundo tem contraste de { $ratio }:1, abaixo do mínimo de { $floor }:1
    }
# The theme's text on its frame, the surface of the top bar and Settings.
colors-issue-frame-not-opaque = o texto sobre o quadro não é opaco
# As above, on the frame. $ratio and $floor are as above.
colors-issue-frame-low-contrast = o texto sobre o quadro tem contraste de { $ratio }:1, abaixo do mínimo de { $floor }:1
# Another file in the folder gives its theme the same name. $name is the
# name as the file has it, to put in your language's quotes; $file the
# other file's name.
colors-issue-name-taken = “{ $name }” já é o nome de { $file }
colors-issue-file-name-unprintable = o nome do arquivo tem caracteres não imprimíveis
# The folder holds too many theme files. $max is how many are read, e.g.
# 100.
colors-issue-too-many = acima do limite de { $max } arquivos

## The Colors row: the palette presets under Dark and Light.

colors-colors = Cores:
# The preset chips. $state is customized on the picked chip while the user
# has changed some of its colours, plain otherwise.
colors-preset-default =
    { $state ->
        [customized] Padrão (personalizado)
       *[plain] Padrão
    }
    .hover = Paleta equilibrada para o uso diário
# $state as for Default.
colors-preset-high-contrast =
    { $state ->
        [customized] Alto contraste (personalizado)
       *[plain] Alto contraste
    }
    .hover = Paleta de contraste máximo para iluminação forte ou projetores
# $state as for Default. Protan and deutan are the red-green colour-vision
# deficiencies.
colors-preset-colorblind =
    { $state ->
        [customized] Daltonismo (personalizado)
       *[plain] Daltonismo
    }
    .hover = Paleta com cores distinguíveis para quem tem deficiência protan ou deutan
# After the chips while there are changes a click on another preset would
# lose; the preset in use clicked again keeps them.
colors-other-preset-clears = Escolher outra predefinição apaga as personalizações

## The Customize colors section, folded under its header until opened.

colors-customize = Personalizar cores
    .hover = Ajustar cada cor a partir da predefinição ou do tema selecionado
# Beside the header: whose colours the swatches change. $mode is dark or
# light.
colors-editing =
    { $mode ->
        [light] (editando as cores do tema claro)
       *[dark] (editando as cores do tema escuro)
    }
# The same while a named theme is on. $name is the theme's name.
colors-editing-named = (editando as cores de { $name })

# The captions at the start of the swatch rows.
colors-group-ui = Interface:
colors-group-graph = Gráfico:
colors-group-status = Status:
# The arrow after a colour that others follow, before those it leads.
# $anchor is the leading colour's name, as its swatch below has it.
colors-follow-arrow =
    .hover = Estas cores seguem a cor { $anchor }, mantendo a diferença em relação a ela, até que você escolha uma delas separadamente

## The swatches, each named beside it, in the rows the section shows them.

colors-swatch-background = Plano de fundo
    .hover = Cor do plano de fundo dos painéis
colors-swatch-frame = Quadro
    .hover = Plano de fundo da barra superior e do painel de leitura (se nenhum for definido, o mesmo dos painéis)
colors-swatch-button = Botão
    .hover = Cor do plano de fundo dos botões
# Lines between and around the panels.
colors-swatch-border = Borda
    .hover = Separadores, bordas dos painéis, quadros e contorno do gráfico
# Short for the plot's background.
colors-swatch-plot-background = Fundo do gráfico
    .hover = Cor do plano de fundo da área do gráfico
colors-swatch-text = Texto
    .hover = Cor do texto principal
# The meter's value as the reading panel shows it, in large type.
colors-swatch-reading = Leitura
    .hover = A leitura e as leituras dos valores secundários (a cor do texto, se nenhuma for definida)
# Text drawn fainter than the rest.
colors-swatch-weak-text = Texto secundário
    .hover = Texto secundário: linha do modo, rótulos dos valores secundários, dicas e legendas
colors-swatch-heading = Título
    .hover = Títulos dos painéis: Especificações, Estatísticas, Visível (a cor do texto secundário, se nenhuma for definida)
# The lines that follow the pointer over the graph.
colors-swatch-crosshair = Retícula
    .hover = Cor da retícula que acompanha o mouse sobre o gráfico
colors-swatch-accent = Destaque
    .hover = Selos de modo, links e o fundo de botões de alternância, chips e textos selecionados
# The graph's main line, the readings over time.
colors-swatch-data-line = Curva
    .hover = Cor da curva das leituras no gráfico
# The band between the lowest and highest readings.
colors-swatch-envelope = Envelope
    .hover = Cor de preenchimento do envelope Mín./Máx.
# The lines of the other values a meter shows at once beside the main one.
colors-swatch-overlay-1 = Sobrep. 1
    .hover = Cor da linha do primeiro valor secundário desenhado junto com a série plotada
colors-swatch-overlay-2 = Sobrep. 2
    .hover = Cor da linha do segundo valor secundário desenhado junto com a série plotada
colors-swatch-overlay-3 = Sobrep. 3
    .hover = Cor da linha do terceiro valor secundário desenhado junto com a série plotada
# The frame the overview strip under the graph draws around the part on
# show.
colors-swatch-viewport = Área visível
    .hover = Cor do retângulo da área visível no minimapa
# A stretch with no readings.
colors-swatch-gap = Lacuna
    .hover = Cor usada para marcar lacunas nos dados gravados
# The line at the average of the readings.
colors-swatch-mean = Média
    .hover = Cor da linha da média
# Short for reference: a line the user sets at a value.
colors-swatch-ref = Ref.
    .hover = Cor da linha de referência
# Where the readings cross a set level.
colors-swatch-crossing = Cruzamento
    .hover = Cor dos marcadores de cruzamento dos gatilhos
colors-swatch-cursor = Cursor
    .hover = Cor dos cursores de medição
# The numbered markers the user drops on the graph.
colors-swatch-marker = Marcador
    .hover = Linhas, bandeiras e números dos marcadores
# The status colours, after the states they mark. Connected is the top
# bar's status word, in quotes in the hover.
colors-swatch-connected = Conectado
    .hover = Cor do status “Conectado”
colors-swatch-warning = Aviso
    .hover = Cor do status de aviso
colors-swatch-error = Erro
    .hover = Cor do status de erro
colors-swatch-inactive = Inativo
    .hover = Cor do status inativo / desconectado

## Beside a swatch.

# A swatch's name for a screen reader while it follows another colour.
# $color is its name, $anchor the name of the colour it follows.
colors-swatch-linked = { $color }, segue { $anchor }
# A small button beside a colour that was set on its own though it can
# follow another; ↺ is part of it. $color is the colour's name, $anchor
# the name of the one it would follow.
colors-relink = ↺ revincular
    .hover = Seguir { $anchor } de novo
    .a11y = Revincular { $color } a { $anchor }
# A warning after a colour the user made hard to see on another. $ratio is
# their contrast, e.g. 2.1; $ground the other colour's name; $floor the
# least contrast it needs, e.g. 4.5.
colors-low-contrast = { $ratio }:1 sobre { $ground }
    .hover = Abaixo do contraste mínimo de { $floor }:1 necessário para esta cor sobre { $ground }
# A warning after a graph colour the user made hard to tell from another.
# $other is the other colour's name; $distance how far apart they are and
# $floor how far apart they need to be, as whole numbers. ΔE is the
# measure of colour difference.
colors-too-close = próxima de { $other }
    .hover = Distância de apenas { $distance } (ΔE); as cores do gráfico precisam de { $floor } para se distinguir

## The editor a swatch opens: a colour picker, then the colour as text.

# The colour written as # and six hexadecimal digits.
colors-hex = Hex
    .hover = Copiar esta cor, ou colar uma no formato #RRGGBB

## Under the swatches, once a colour was changed.

colors-reset = Redefinir cores
    .hover = Descartar as alterações nas cores deste modo
# The same while a named theme is on. $name is the theme's name.
colors-reset-named =
    .hover = Descartar as alterações nas cores do tema { $name }
colors-save = Salvar como tema…
    .hover = Salvar estas cores como arquivo de tema; um tema salvo na sua pasta de temas aparece na linha Tema
# The file type in the save dialog's list of types.
colors-save-file-type = Tema

## What Save as theme says when it is done, in a toast: a short message that
## floats under the top bar's right end for a few seconds.

# $name is the theme's name, from the file name the user typed.
colors-saved = Tema { $name } salvo na sua pasta de temas
# Saved outside the themes folder. $file is the file name.
colors-saved-elsewhere = Arquivo { $file } salvo. Copie-o para a sua pasta de temas e use-o como tema
# The save dialog closed without an answer.
colors-save-dialog-failed = Não salvo: falha na caixa de diálogo Salvar
# A name typed without .json gets it added, and the file of that name
# already exists. $path is that file, in full.
colors-save-exists = Não salvo: { $path } já existe; escolha-o na caixa de diálogo para substituí-lo
# Another file in the themes folder gives its theme the same name. $name is
# the name; $file the other file's name.
colors-save-name-taken = Não salvo: { $name } já é o nome de { $file } na sua pasta de temas; escolha outro nome de arquivo
# The theme would be a file the folder can't use. $reason is one of the
# reasons above, starting lower case.
colors-save-invalid = Não salvo: { $reason }
# Writing the file failed. $path is the file, in full; $error why, in the
# words of the system that raised it.
colors-save-failed = Não foi possível salvar { $path }: { $error }
