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
        [pinned] Oscuro (--theme)
        [customized] Oscuro (personalizado)
       *[plain] Oscuro
    }
    .hover = Usar el modo oscuro en toda la interfaz
# $state as for Dark.
colors-theme-light =
    { $state ->
        [pinned] Claro (--theme)
        [customized] Claro (personalizado)
       *[plain] Claro
    }
    .hover = Usar el modo claro en toda la interfaz
# $state as for Dark, never customized: System has no colours of its own.
colors-theme-system =
    { $state ->
        [pinned] Sistema (--theme)
       *[plain] Sistema
    }
    .hover = Seguir el ajuste claro/oscuro del escritorio (Oscuro si no indica ninguno)
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
           *[dark] Tema oscuro integrado
        }
# The hover of a theme the user added to the themes folder. $mode is dark
# or light; $path is the theme's file, in full.
colors-theme-file =
    .hover =
        { $mode ->
            [light] Tema claro, cargado desde { $path }. Elimina el archivo para quitarlo.
           *[dark] Tema oscuro, cargado desde { $path }. Elimina el archivo para quitarlo.
        }
# The same when the themes folder can't be found, with $mode as there.
# $file is the theme's file name.
colors-theme-file-unplaced =
    .hover =
        { $mode ->
            [light] Tema claro, cargado desde { $file }
           *[dark] Tema oscuro, cargado desde { $file }
        }

## After the Theme row's chips: the theme files in the themes folder that
## could not be offered, with why on hover, one file to a line.

# $count is how many files were skipped.
colors-skipped =
    { $count ->
        [one] { $count } archivo de tema omitido
       *[other] { $count } archivos de tema omitidos
    }
# The hover's first line. $folder is the themes folder, in full.
colors-skipped-folder = En { $folder }:
# A line of the hover. $file is a file name; $reason one of the reasons
# below, starting lower case.
colors-skipped-file = { $file }: { $reason }
# The line for the files left unread because the folder holds too many.
# $count is how many; $max how many files are read, e.g. 100.
colors-skipped-too-many =
    { $count ->
        [one] { $count } más: supera el límite de { $max } archivos
       *[other] { $count } más: superan el límite de { $max } archivos
    }

## Why a theme file can't be used: after its name in the hover above, or
## after "Not saved:" when Save as theme would write such a file. Each
## starts lower case. Others come from the file system or the file reader
## and are shown in their own words.

colors-issue-not-a-file = no es un archivo
# $size is the largest file read, in KiB (kibibytes), e.g. 64.
colors-issue-too-big = supera los { $size } KiB
colors-issue-name-empty = el nombre está vacío
# $max is the most characters a name may have, e.g. 32.
colors-issue-name-too-long = el nombre supera los { $max } caracteres
# Invisible characters, or $ or `.
colors-issue-name-unusable = el nombre contiene caracteres no válidos para un tema
# The theme's name is Dark, Light or System, which the Theme row's own
# chips go by. $name is the name as the file has it; put it in your
# language's quotes.
colors-issue-name-reserved = “{ $name }” está reservado para la fila Tema
# $what is text or button: the theme's text on its background, or its
# button captions on the button.
colors-issue-not-opaque =
    { $what ->
        [button] el texto no es opaco sobre el botón
       *[text] el texto no es opaco sobre el fondo
    }
# The colours can't be read on each other. $what is as above; $ratio is
# their contrast, e.g. 1.8, and $floor the least allowed, e.g. 3.
colors-issue-low-contrast =
    { $what ->
        [button] el contraste del texto sobre el botón es de { $ratio }:1, por debajo del mínimo de { $floor }:1
       *[text] el contraste del texto sobre el fondo es de { $ratio }:1, por debajo del mínimo de { $floor }:1
    }
# The theme's text on its frame, the surface of the top bar and Settings.
colors-issue-frame-not-opaque = el texto no es opaco sobre el marco
# As above, on the frame. $ratio and $floor are as above.
colors-issue-frame-low-contrast = el contraste del texto sobre el marco es de { $ratio }:1, por debajo del mínimo de { $floor }:1
# Another file in the folder gives its theme the same name. $name is the
# name as the file has it, to put in your language's quotes; $file the
# other file's name.
colors-issue-name-taken = “{ $name }” ya es el nombre de { $file }
colors-issue-file-name-unprintable = el nombre del archivo tiene caracteres no imprimibles
# The folder holds too many theme files. $max is how many are read, e.g.
# 100.
colors-issue-too-many = supera el límite de { $max } archivos

## The Colors row: the palette presets under Dark and Light.

colors-colors = Colores:
# The preset chips. $state is customized on the picked chip while the user
# has changed some of its colours, plain otherwise.
colors-preset-default =
    { $state ->
        [customized] Predeterminado (personalizado)
       *[plain] Predeterminado
    }
    .hover = Paleta equilibrada para el uso diario
# $state as for Default.
colors-preset-high-contrast =
    { $state ->
        [customized] Contraste alto (personalizado)
       *[plain] Contraste alto
    }
    .hover = Paleta de contraste máximo para luz intensa o proyectores
# $state as for Default. Protan and deutan are the red-green colour-vision
# deficiencies.
colors-preset-colorblind =
    { $state ->
        [customized] Daltonismo (personalizado)
       *[plain] Daltonismo
    }
    .hover = Paleta cuyos colores se distinguen bien con daltonismo protán o deután
# After the chips while there are changes a click on another preset would
# lose; the preset in use clicked again keeps them.
colors-other-preset-clears = Elegir otra paleta predefinida borra las personalizaciones

## The Customize colors section, folded under its header until opened.

colors-customize = Personalizar colores
    .hover = Cambiar colores uno a uno a partir de la paleta o el tema seleccionado
# Beside the header: whose colours the swatches change. $mode is dark or
# light.
colors-editing =
    { $mode ->
        [light] (editando los colores del tema claro)
       *[dark] (editando los colores del tema oscuro)
    }
# The same while a named theme is on. $name is the theme's name.
colors-editing-named = (editando los colores de { $name })

# The captions at the start of the swatch rows.
colors-group-ui = Interfaz:
colors-group-graph = Gráfico:
colors-group-status = Estado:
# The arrow after a colour that others follow, before those it leads.
# $anchor is the leading colour's name, as its swatch below has it.
colors-follow-arrow =
    .hover = Estos colores siguen al color { $anchor } y mantienen su diferencia con él, hasta que cambies uno por separado

## The swatches, each named beside it, in the rows the section shows them.

colors-swatch-background = Fondo
    .hover = Color de fondo de los paneles
colors-swatch-frame = Marco
    .hover = Fondo de la barra superior y del panel de lectura (el fondo de los paneles si no se define)
colors-swatch-button = Botón
    .hover = Color de fondo de los botones
# Lines between and around the panels.
colors-swatch-border = Borde
    .hover = Separadores, bordes de los paneles, marcos y contorno del gráfico
# Short for the plot's background.
colors-swatch-plot-background = Fondo del gráfico
    .hover = Color de fondo del área de trazado del gráfico
colors-swatch-text = Texto
    .hover = Color del texto principal
# The meter's value as the reading panel shows it, in large type.
colors-swatch-reading = Lectura
    .hover = La lectura y las lecturas de los valores secundarios (el color del texto si no se define)
# Text drawn fainter than the rest.
colors-swatch-weak-text = Texto secundario
    .hover = Texto secundario: línea del modo, etiquetas de los valores secundarios, sugerencias y rótulos
colors-swatch-heading = Título
    .hover = Títulos de los paneles: Especificaciones, Estadísticas, Visible (el color del texto secundario si no se define)
# The lines that follow the pointer over the graph.
colors-swatch-crosshair = Cruz
    .hover = Color de la cruz del cursor al pasar el mouse sobre el gráfico
colors-swatch-accent = Énfasis
    .hover = Distintivos de modo, vínculos y el fondo de las opciones, chips y textos seleccionados
# The graph's main line, the readings over time.
colors-swatch-data-line = Curva
    .hover = Color de la curva de las lecturas en el gráfico
# The band between the lowest and highest readings.
colors-swatch-envelope = Envolvente
    .hover = Color de relleno de la envolvente Mín./Máx.
# The lines of the other values a meter shows at once beside the main one.
colors-swatch-overlay-1 = Superp. 1
    .hover = Color de la línea del primer valor secundario trazado junto a la serie
colors-swatch-overlay-2 = Superp. 2
    .hover = Color de la línea del segundo valor secundario trazado junto a la serie
colors-swatch-overlay-3 = Superp. 3
    .hover = Color de la línea del tercer valor secundario trazado junto a la serie
# The frame the overview strip under the graph draws around the part on
# show.
colors-swatch-viewport = Área visible
    .hover = Color del rectángulo del área visible en el minimapa
# A stretch with no readings.
colors-swatch-gap = Interrupción
    .hover = Color que marca las interrupciones en los datos grabados
# The line at the average of the readings.
colors-swatch-mean = Media
    .hover = Color de la línea de la media
# Short for reference: a line the user sets at a value.
colors-swatch-ref = Ref.
    .hover = Color de la línea de referencia
# Where the readings cross a set level.
colors-swatch-crossing = Cruce
    .hover = Color de las marcas de disparo en los cruces
colors-swatch-cursor = Cursor
    .hover = Color de los cursores de medición
# The numbered markers the user drops on the graph.
colors-swatch-marker = Marcador
    .hover = Líneas, banderas y números de los marcadores
# The status colours, after the states they mark. Connected is the top
# bar's status word, in quotes in the hover.
colors-swatch-connected = Conectado
    .hover = Color del estado “Conectado”
colors-swatch-warning = Advertencia
    .hover = Color del estado de advertencia
colors-swatch-error = Error
    .hover = Color del estado de error
colors-swatch-inactive = Inactivo
    .hover = Color del estado inactivo / desconectado

## Beside a swatch.

# A swatch's name for a screen reader while it follows another colour.
# $color is its name, $anchor the name of the colour it follows.
colors-swatch-linked = { $color }, sigue a { $anchor }
# A small button beside a colour that was set on its own though it can
# follow another; ↺ is part of it. $color is the colour's name, $anchor
# the name of the one it would follow.
colors-relink = ↺ revincular
    .hover = Volver a seguir a { $anchor }
    .a11y = Volver a vincular { $color } con { $anchor }
# A warning after a colour the user made hard to see on another. $ratio is
# their contrast, e.g. 2.1; $ground the other colour's name; $floor the
# least contrast it needs, e.g. 4.5.
colors-low-contrast = { $ratio }:1 sobre { $ground }
    .hover = Por debajo del contraste de { $floor }:1 que este color necesita sobre { $ground }
# A warning after a graph colour the user made hard to tell from another.
# $other is the other colour's name; $distance how far apart they are and
# $floor how far apart they need to be, as whole numbers. ΔE is the
# measure of colour difference.
colors-too-close = parecido a { $other }
    .hover = Solo { $distance } de diferencia (ΔE), cuando los colores del gráfico necesitan { $floor } para distinguirse

## The editor a swatch opens: a colour picker, then the colour as text.

# The colour written as # and six hexadecimal digits.
colors-hex = Hex
    .hover = Copiar este color o pegar uno en formato #RRGGBB

## Under the swatches, once a colour was changed.

colors-reset = Restablecer colores
    .hover = Descartar tus cambios en los colores del modo actual
# The same while a named theme is on. $name is the theme's name.
colors-reset-named =
    .hover = Descartar tus cambios en los colores del tema { $name }
colors-save = Guardar como tema…
    .hover = Guardar estos colores como archivo de tema. Si lo guardas en tu carpeta de temas, se agrega a la fila Tema
# The file type in the save dialog's list of types.
colors-save-file-type = Tema

## What Save as theme says when it is done, in a toast: a short message that
## floats under the top bar's right end for a few seconds.

# $name is the theme's name, from the file name the user typed.
colors-saved = Se guardó el tema { $name } en tu carpeta de temas
# Saved outside the themes folder. $file is the file name.
colors-saved-elsewhere = Se guardó { $file }. Cópialo en tu carpeta de temas para usarlo como tema
# The save dialog closed without an answer.
colors-save-dialog-failed = No se guardó: error en el cuadro de diálogo Guardar
# A name typed without .json gets it added, and the file of that name
# already exists. $path is that file, in full.
colors-save-exists = No se guardó: { $path } ya existe. Elígelo en el cuadro de diálogo para reemplazarlo
# Another file in the themes folder gives its theme the same name. $name is
# the name; $file the other file's name.
colors-save-name-taken = No se guardó: { $name } ya es el nombre de { $file } en tu carpeta de temas. Elige otro nombre de archivo
# The theme would be a file the folder can't use. $reason is one of the
# reasons above, starting lower case.
colors-save-invalid = No se guardó: { $reason }
# Writing the file failed. $path is the file, in full; $error why, in the
# words of the system that raised it.
colors-save-failed = No se pudo guardar { $path }: { $error }
