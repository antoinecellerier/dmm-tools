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

colors-theme = Theme:
# The Dark, Light and System chips. $state is pinned when --theme picked
# this mode for the session, customized when the user changed some of its
# colours (Dark and Light only), plain otherwise.
colors-theme-dark =
    { $state ->
        [pinned] Dark (--theme)
        [customized] Dark (customized)
       *[plain] Dark
    }
    .hover = Use Dark mode for the whole GUI
# $state as for Dark.
colors-theme-light =
    { $state ->
        [pinned] Light (--theme)
        [customized] Light (customized)
       *[plain] Light
    }
    .hover = Use Light mode for the whole GUI
# $state as for Dark, never customized: System has no colours of its own.
colors-theme-system =
    { $state ->
        [pinned] System (--theme)
       *[plain] System
    }
    .hover = Follow the desktop's light/dark setting (Dark if it reports none)
# A named theme's chip when --theme picked it for the session; otherwise
# the chip is the theme's name alone. $name is the theme's name, e.g.
# Midnight.
colors-theme-named-pinned = { $name } (--theme)
# A named theme's chip after the user changed some of its colours. $name is
# the theme's name.
colors-theme-named-customized = { $name } (customized)
# A built-in theme's hover. $mode is dark or light: the mode the theme is
# drawn in.
colors-theme-builtin =
    .hover =
        { $mode ->
            [light] Built-in light theme
           *[dark] Built-in dark theme
        }
# The hover of a theme the user added to the themes folder. $mode is dark
# or light; $path is the theme's file, in full.
colors-theme-file =
    .hover =
        { $mode ->
            [light] Light theme, from { $path }. Delete the file to remove it.
           *[dark] Dark theme, from { $path }. Delete the file to remove it.
        }
# The same when the themes folder can't be found, with $mode as there.
# $file is the theme's file name.
colors-theme-file-unplaced =
    .hover =
        { $mode ->
            [light] Light theme, from { $file }
           *[dark] Dark theme, from { $file }
        }

## After the Theme row's chips: the theme files in the themes folder that
## could not be offered, with why on hover, one file to a line.

# $count is how many files were skipped.
colors-skipped =
    { $count ->
        [one] { $count } theme file skipped
       *[other] { $count } theme files skipped
    }
# The hover's first line. $folder is the themes folder, in full.
colors-skipped-folder = In { $folder }:
# A line of the hover. $file is a file name; $reason one of the reasons
# below, starting lower case.
colors-skipped-file = { $file }: { $reason }
# The line for the files left unread because the folder holds too many.
# $count is how many; $max how many files are read, e.g. 100.
colors-skipped-too-many =
    { $count ->
        [one] { $count } more: over the { $max }-file limit
       *[other] { $count } more: over the { $max }-file limit
    }

## Why a theme file can't be used: after its name in the hover above, or
## after "Not saved:" when Save as theme would write such a file. Each
## starts lower case. Others come from the file system or the file reader
## and are shown in their own words.

colors-issue-not-a-file = not a file
# $size is the largest file read, in KiB (kibibytes), e.g. 64.
colors-issue-too-big = over { $size } KiB
colors-issue-name-empty = the name is empty
# $max is the most characters a name may have, e.g. 32.
colors-issue-name-too-long = the name is over { $max } characters
# Invisible characters, or $ or `.
colors-issue-name-unusable = the name has characters a theme name can't use
# The theme's name is Dark, Light or System, which the Theme row's own
# chips go by. $name is the name as the file has it; put it in your
# language's quotes.
colors-issue-name-reserved = "{ $name }" is taken by the Theme row
# $what is text or button: the theme's text on its background, or its
# button captions on the button.
colors-issue-not-opaque =
    { $what ->
        [button] button text on the button is not opaque
       *[text] text on the background is not opaque
    }
# The colours can't be read on each other. $what is as above; $ratio is
# their contrast, e.g. 1.8, and $floor the least allowed, e.g. 3.
colors-issue-low-contrast =
    { $what ->
        [button] button text on the button is { $ratio }:1, under the { $floor }:1 floor
       *[text] text on the background is { $ratio }:1, under the { $floor }:1 floor
    }
# The theme's text on its frame, the surface of the top bar and Settings.
colors-issue-frame-not-opaque = text on the frame is not opaque
# As above, on the frame. $ratio and $floor are as above.
colors-issue-frame-low-contrast = text on the frame is { $ratio }:1, under the { $floor }:1 floor
# Another file in the folder gives its theme the same name. $name is the
# name as the file has it, to put in your language's quotes; $file the
# other file's name.
colors-issue-name-taken = "{ $name }" is already the name of { $file }
colors-issue-file-name-unprintable = the file name has characters that don't print
# The folder holds too many theme files. $max is how many are read, e.g.
# 100.
colors-issue-too-many = over the { $max }-file limit

## The Colors row: the palette presets under Dark and Light.

colors-colors = Colors:
# The preset chips. $state is customized on the picked chip while the user
# has changed some of its colours, plain otherwise.
colors-preset-default =
    { $state ->
        [customized] Default (customized)
       *[plain] Default
    }
    .hover = Balanced palette tuned for everyday use
# $state as for Default.
colors-preset-high-contrast =
    { $state ->
        [customized] High Contrast (customized)
       *[plain] High Contrast
    }
    .hover = Maximum-contrast palette for bright lighting or projectors
# $state as for Default. Protan and deutan are the red-green colour-vision
# deficiencies.
colors-preset-colorblind =
    { $state ->
        [customized] Colorblind (customized)
       *[plain] Colorblind
    }
    .hover = Palette that stays distinguishable for protan/deutan vision
# After the chips while there are changes a click on another preset would
# lose; the preset in use clicked again keeps them.
colors-other-preset-clears = Selecting another preset will clear customizations

## The Customize colors section, folded under its header until opened.

colors-customize = Customize colors
    .hover = Per-color overrides on top of the selected preset or theme
# Beside the header: whose colours the swatches change. $mode is dark or
# light.
colors-editing =
    { $mode ->
        [light] (editing light theme colors)
       *[dark] (editing dark theme colors)
    }
# The same while a named theme is on. $name is the theme's name.
colors-editing-named = (editing { $name } colors)

# The captions at the start of the swatch rows.
colors-group-ui = UI:
colors-group-graph = Graph:
colors-group-status = Status:
# The arrow after a colour that others follow, before those it leads.
# $anchor is the leading colour's name, as its swatch below has it.
colors-follow-arrow =
    .hover = These follow { $anchor }, keeping their offset from it, until you pick one on its own

## The swatches, each named beside it, in the rows the section shows them.

colors-swatch-background = Background
    .hover = Panel background color
colors-swatch-frame = Frame
    .hover = Top bar and reading panel background (the panel background unless set)
colors-swatch-button = Button
    .hover = Button background color
# Lines between and around the panels.
colors-swatch-border = Border
    .hover = Separators, panel edges, frames and the plot outline
# Short for the plot's background.
colors-swatch-plot-background = Plot bg
    .hover = Graph plot area background color
colors-swatch-text = Text
    .hover = Primary text color
# The meter's value as the reading panel shows it, in large type.
colors-swatch-reading = Reading
    .hover = The reading and sub-value readings (the text color unless set)
# Text drawn fainter than the rest.
colors-swatch-weak-text = Weak text
    .hover = Secondary text: mode line, sub-value labels, hints and captions
colors-swatch-heading = Heading
    .hover = Panel headings: Specifications, Statistics, Visible (the weak text color unless set)
# The lines that follow the pointer over the graph.
colors-swatch-crosshair = Crosshair
    .hover = Hover crosshair color on the graph
colors-swatch-accent = Accent
    .hover = Mode badges, links, and the fill behind selected toggles, chips and text
# The graph's main line, the readings over time.
colors-swatch-data-line = Data line
    .hover = Data line color on the graph
# The band between the lowest and highest readings.
colors-swatch-envelope = Envelope
    .hover = Min/Max envelope fill color
# The lines of the other values a meter shows at once beside the main one.
colors-swatch-overlay-1 = Overlay 1
    .hover = Line color of the first sub-value drawn beside the plotted series
colors-swatch-overlay-2 = Overlay 2
    .hover = Line color of the second sub-value drawn beside the plotted series
colors-swatch-overlay-3 = Overlay 3
    .hover = Line color of the third sub-value drawn beside the plotted series
# The frame the overview strip under the graph draws around the part on
# show.
colors-swatch-viewport = Viewport
    .hover = Minimap viewport rectangle color
# A stretch with no readings.
colors-swatch-gap = Gap
    .hover = Color used to mark gaps in recorded data
# The line at the average of the readings.
colors-swatch-mean = Mean
    .hover = Mean overlay line color
# Short for reference: a line the user sets at a value.
colors-swatch-ref = Ref
    .hover = Reference line color
# Where the readings cross a set level.
colors-swatch-crossing = Crossing
    .hover = Trigger-crossing marker color
colors-swatch-cursor = Cursor
    .hover = Measurement cursor color
# The numbered markers the user drops on the graph.
colors-swatch-marker = Marker
    .hover = Marker lines, flags and numbers
# The status colours, after the states they mark. Connected is the top
# bar's status word, in quotes in the hover.
colors-swatch-connected = Connected
    .hover = "Connected" status color
colors-swatch-warning = Warning
    .hover = Warning status color
colors-swatch-error = Error
    .hover = Error status color
colors-swatch-inactive = Inactive
    .hover = Inactive / disconnected status color

## Beside a swatch.

# A swatch's name for a screen reader while it follows another colour.
# $color is its name, $anchor the name of the colour it follows.
colors-swatch-linked = { $color }, follows { $anchor }
# A small button beside a colour that was set on its own though it can
# follow another; ↺ is part of it. $color is the colour's name, $anchor
# the name of the one it would follow.
colors-relink = ↺ relink
    .hover = Follow { $anchor } again
    .a11y = Relink { $color } to { $anchor }
# A warning after a colour the user made hard to see on another. $ratio is
# their contrast, e.g. 2.1; $ground the other colour's name; $floor the
# least contrast it needs, e.g. 4.5.
colors-low-contrast = { $ratio }:1 on { $ground }
    .hover = Under the { $floor }:1 contrast this color needs on { $ground }
# A warning after a graph colour the user made hard to tell from another.
# $other is the other colour's name; $distance how far apart they are and
# $floor how far apart they need to be, as whole numbers. ΔE is the
# measure of colour difference.
colors-too-close = close to { $other }
    .hover = Only { $distance } apart (ΔE) where graph colors need { $floor } to read apart

## The editor a swatch opens: a colour picker, then the colour as text.

# The colour written as # and six hexadecimal digits.
colors-hex = Hex
    .hover = Copy this color, or paste one as #RRGGBB

## Under the swatches, once a colour was changed.

colors-reset = Reset colors
    .hover = Discard your changes to the colors of this mode
# The same while a named theme is on. $name is the theme's name.
colors-reset-named =
    .hover = Discard your changes to { $name }'s colors
colors-save = Save as theme…
    .hover = Save these colors as a theme file; one saved in your themes folder joins the Theme row
# The file type in the save dialog's list of types.
colors-save-file-type = Theme

## What Save as theme says when it is done, in a toast: a short message that
## floats under the top bar's right end for a few seconds.

# $name is the theme's name, from the file name the user typed.
colors-saved = Saved { $name } to your themes folder
# Saved outside the themes folder. $file is the file name.
colors-saved-elsewhere = Saved { $file }. Copy it into your themes folder to use it as a theme
# The save dialog closed without an answer.
colors-save-dialog-failed = Not saved: the save dialog failed
# A name typed without .json gets it added, and the file of that name
# already exists. $path is that file, in full.
colors-save-exists = Not saved: { $path } already exists; pick it in the dialog to replace it
# Another file in the themes folder gives its theme the same name. $name is
# the name; $file the other file's name.
colors-save-name-taken = Not saved: { $name } is already the name of { $file } in your themes folder; pick another file name
# The theme would be a file the folder can't use. $reason is one of the
# reasons above, starting lower case.
colors-save-invalid = Not saved: { $reason }
# Writing the file failed. $path is the file, in full; $error why, in the
# words of the system that raised it.
colors-save-failed = Could not save { $path }: { $error }
