---
paths:
  - "README.md"
---

# README rules

The README is read two ways: by a visitor deciding in ten seconds what the
project does and where to learn more, and by search engines. It is an entry
point, not a fourth reference. The general rules are in
`docs-user-facing.md`.

- The first paragraph names the desktop app and the command-line tool, what
  they do, "digital multimeters (DMMs)", the meter brands and the platforms,
  in prose — a crawler weights the first hundred words, and a
  table cell less than a sentence. Cable names belong in the catalog, not
  the intro.
- The intro names about a dozen of the most-searched models, at least one
  per brand, and ends with "and more" linking to the device table, which
  lists every model. A new device gets a table row, not an intro mention,
  unless it outsells the models already named for its brand.
- The badges sit under the title, so the tagline and the model list read as
  one block.
- The reader is a hobbyist or a repair technician with a meter on the cable or
  in Bluetooth range. A line says what a feature does in general terms, not
  through one use of it, in the references' feature names. No per-meter
  conditions ("on supported meters", "where the model allows"): the
  references and the device catalog carry those.
- Features, under the screenshot, opens on "Both the desktop app and the
  command-line tool:" and what both do, then has a subsection each, headed in
  full words with the executable: "Desktop app (`dmm-gui`)", "Command-line
  tool (`dmm-cli`)", each led by "The desktop app also:" or "The command-line
  tool also:". A capability both programs have is in the shared list only.
- Every bullet is a verb phrase completing its lead.
- Each program's list holds what only that program does: at most four bullets,
  one line each, the most-used first. No mechanism, no option names except
  the one flag that names the feature.
- A new feature gets a bullet only by outranking one already there; otherwise
  it goes in its reference alone.
- The CLI list is followed by one generated output sample. Other examples go
  in the CLI reference.
- The program headings are plain text. Each subsection ends on a line that
  opens with its reference's link, labelled "GUI help" or "CLI help", so a
  reader skimming for help finds it.
- Quick start is the three steps to a first reading: get the binary, the one
  platform step that blocks a first run, run. Everything else links to
  `docs/setup.md`; never restate it.
- The Documentation index is split by persona: user docs as a list, one line
  for contributors. Supported devices is a user doc.
- Acknowledgements are credit, kept compact: one bullet per project, what it
  contributed, no methodology.
- The device table is hand-written and guarded by a test; keep its status
  words those of `Stability::label()`, capitalised, with the emoji
  `device-catalog.md` gives them.
- One device row per verification issue and status, brand first in the Meter cell;
  columns Meter, USB, Bluetooth, Issue. A Bluetooth status says "(adapter)"
  or "(built in)"; `—` marks a link the meter is not supported over.
- Row order, so the table opens on what hardware confirmed and keeps a
  brand's series together: brands, then each brand's series (its rows in one
  `docs/supported-devices.md` section), then rows, each ranked by best status
  (✅ over 🟡 over 🧪), then by how many links have that status, 🧪 counting
  none, then by name with numbers by value (UT61E+ before UT161E; a series
  by its first row). A status change can move a row, its series and its
  brand.
- The screenshot is refreshed at a release whose changelog changes what the
  picture shows.
