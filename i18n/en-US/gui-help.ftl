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
    .about = GUI application for USB and Bluetooth digital multimeters
# dmm-gui --device
dmm-gui--device =
    .help = Device to connect to [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...]
# dmm-gui --theme
dmm-gui--theme =
    .help = Theme override
# dmm-gui --renderer
dmm-gui--renderer =
    .help = Graphics renderer
# dmm-gui --replay
dmm-gui--replay =
    .help = Play back a file written by 'dmm-cli read --format replay' or by Export… → Replay…, instead of connecting to a meter
# dmm-gui --import
dmm-gui--import =
    .help = Open an exported CSV, JSON or replay file, with its markers, instead of connecting to a meter
# dmm-gui --completions
dmm-gui--completions =
    .help = Print a shell completion script and exit
