### Import… and Export…, the first and last buttons of the row under the
### graph (assets/gui-wide-layout.webp): Import… opens a file saved
### earlier, the Export… split button saves the samples to a file, and its
### arrow drops a menu of the file types. Then the line under the row
### after an import, the prompt shown before something would discard
### samples or markers no file holds, the file types the system's open and
### save dialogs list, and the messages both buttons show when they are
### done.
###
### A sample is one reading the meter sent; a reading, in the import
### messages, is one row of the file. The buttons sit in one row with
### Record and Discard, so keep them short. CSV, JSON and replay are the
### file types; a replay file is one dmm-gui plays back as if the meter were
### attached. $keys is a keyboard shortcut as this computer writes it, e.g.
### Ctrl+E or ⌘E.

## The row of buttons under the graph.

# The first button of the row.
import-button = Importar…
    .hover = Abrir un archivo CSV, JSON o de reproducción exportado ({ $keys })
# The left part of the Export… split button: saves a CSV file at once.
# $buffer is recording when a recording holds the samples, graph when
# nothing is recorded and Export… saves the samples the graph holds.
export-button = Exportar…
    .hover =
        { $buffer ->
            [recording] Guardar la grabación como archivo CSV ({ $keys })
           *[graph] Guardar las muestras del gráfico como archivo CSV ({ $keys })
        }
# The arrow at the right of the Export… split button, which drops the menu
# of file types. It has no text. $buffer is as for Export….
export-menu =
    .hover =
        { $buffer ->
            [recording] Guardar la grabación como archivo CSV, JSON o de reproducción
           *[graph] Guardar las muestras del gráfico como archivo CSV, JSON o de reproducción
        }
    .a11y = Tipo de archivo de exportación

## The menu under the Export… arrow: one entry per file type. Each opens
## the save dialog for that type.

# CSV is the file type's name.
export-csv = CSV…
# JSON is the file type's name. dmm-cli read --format json is a command
# line, kept as it is.
export-json = JSON…
    .hover = Un objeto por línea, como lo escribe dmm-cli read --format json
# A replay file. dmm-gui --replay is a command line, kept as it is; "as the
# meter" means as if it were the meter attached.
export-replay = Reproducción…
    .hover = Un archivo que dmm-gui --replay reproduce como si fuera el multímetro
# The Replay… entry's hover while it is greyed out, and the error shown if
# a replay is asked for anyway. The mock is the simulated meter the app
# offers for trying it out; its samples hold none of the bytes a real meter
# sends (its wire format), which a replay file is made of.
export-no-wire-format = El multímetro simulado no tiene formato de transmisión que exportar. Conecta un multímetro real.

## A small line under the buttons after Import…: the samples are the
## file's.

# $count is how many readings the file gave; $file the file's name, e.g.
# measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv. Export… is the button.
import-hint =
    { $count ->
        [one] Se importó { $count } lectura de { $file }. “Exportar” la guarda con los marcadores y la vista.
       *[other] Se importaron { $count } lecturas de { $file }. “Exportar” las guarda con los marcadores y la vista.
    }

## The prompt shown over the window before something would discard samples
## or markers no file holds: Record over a recording that was never
## exported, Discard on a recording whose samples have left the graph,
## Import… or Connect while the session holds them. Markers are the
## numbered flags the user puts on readings, with a note each; "unsaved
## changes to a marker" means the marker, or its note, was added or changed
## since the last export.

# The prompt's title when samples would be lost (and maybe markers too).
export-unsaved-heading-samples = ¿Descartar las muestras sin exportar?
# The prompt's title when only markers would be lost.
export-unsaved-heading-markers = ¿Descartar los marcadores sin exportar?
# The prompt after Record, which starts a new recording in place of the
# stopped one. $lost is samples when only samples would be lost, markers
# when only markers, both when both. $samples is how many samples were
# never exported; $markers how many markers have unsaved changes. Markers
# stay on the graph's readings even though the recording goes.
export-unsaved-record =
    { $lost ->
        [samples]
            { $samples ->
                [one] Iniciar una nueva grabación descartará { $samples } muestra sin exportar.
               *[other] Iniciar una nueva grabación descartará { $samples } muestras sin exportar.
            }
        [markers]
            { $markers ->
                [one] Iniciar una nueva grabación descartará los cambios no guardados de { $markers } marcador. Los marcadores permanecen en el gráfico mientras este contenga sus lecturas.
               *[other] Iniciar una nueva grabación descartará los cambios no guardados de { $markers } marcadores. Los marcadores permanecen en el gráfico mientras este contenga sus lecturas.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Iniciar una nueva grabación descartará { $samples } muestra sin exportar y los cambios no guardados de { $markers } marcador. Los marcadores permanecen en el gráfico mientras este contenga sus lecturas.
                       *[other] Iniciar una nueva grabación descartará { $samples } muestra sin exportar y los cambios no guardados de { $markers } marcadores. Los marcadores permanecen en el gráfico mientras este contenga sus lecturas.
                    }
               *[other]
                    { $markers ->
                        [one] Iniciar una nueva grabación descartará { $samples } muestras sin exportar y los cambios no guardados de { $markers } marcador. Los marcadores permanecen en el gráfico mientras este contenga sus lecturas.
                       *[other] Iniciar una nueva grabación descartará { $samples } muestras sin exportar y los cambios no guardados de { $markers } marcadores. Los marcadores permanecen en el gráfico mientras este contenga sus lecturas.
                    }
            }
    }
# The prompt's button that goes ahead: discards, then starts recording.
export-unsaved-record-confirm = Descartar y grabar
# The prompt after Discard, which drops a stopped recording; Export… then
# saves the samples the graph still holds, so only the ones that have
# scrolled out of it are lost. $lost, $samples and $markers are as for
# Record; "them" in the last sentences means those samples.
export-unsaved-discard =
    { $lost ->
        [samples]
            { $samples ->
                [one] Si descartas la grabación, se perderá { $samples } muestra que ya salió del gráfico.
               *[other] Si descartas la grabación, se perderán { $samples } muestras que ya salieron del gráfico.
            }
        [markers]
            { $markers ->
                [one] Si descartas la grabación, se perderán los cambios no guardados de { $markers } marcador en lecturas que ya salieron del gráfico.
               *[other] Si descartas la grabación, se perderán los cambios no guardados de { $markers } marcadores en lecturas que ya salieron del gráfico.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Si descartas la grabación, se perderán { $samples } muestra que ya salió del gráfico y los cambios no guardados de { $markers } marcador en ella.
                       *[other] Si descartas la grabación, se perderán { $samples } muestra que ya salió del gráfico y los cambios no guardados de { $markers } marcadores en ella.
                    }
               *[other]
                    { $markers ->
                        [one] Si descartas la grabación, se perderán { $samples } muestras que ya salieron del gráfico y los cambios no guardados de { $markers } marcador en ellas.
                       *[other] Si descartas la grabación, se perderán { $samples } muestras que ya salieron del gráfico y los cambios no guardados de { $markers } marcadores en ellas.
                    }
            }
    }
# The prompt's button that goes ahead: drops the recording.
export-unsaved-discard-confirm = Descartar grabación
# The prompt after picking a file to import, which replaces everything the
# session holds. $lost, $samples and $markers are as for Record.
export-unsaved-import =
    { $lost ->
        [samples]
            { $samples ->
                [one] La importación reemplaza esta sesión y descartará { $samples } muestra sin exportar.
               *[other] La importación reemplaza esta sesión y descartará { $samples } muestras sin exportar.
            }
        [markers]
            { $markers ->
                [one] La importación reemplaza esta sesión y descartará los cambios no guardados de { $markers } marcador.
               *[other] La importación reemplaza esta sesión y descartará los cambios no guardados de { $markers } marcadores.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] La importación reemplaza esta sesión y descartará { $samples } muestra sin exportar y los cambios no guardados de { $markers } marcador.
                       *[other] La importación reemplaza esta sesión y descartará { $samples } muestra sin exportar y los cambios no guardados de { $markers } marcadores.
                    }
               *[other]
                    { $markers ->
                        [one] La importación reemplaza esta sesión y descartará { $samples } muestras sin exportar y los cambios no guardados de { $markers } marcador.
                       *[other] La importación reemplaza esta sesión y descartará { $samples } muestras sin exportar y los cambios no guardados de { $markers } marcadores.
                    }
            }
    }
# The prompt's button that goes ahead: discards, then imports the file.
export-unsaved-import-confirm = Descartar e importar
# The prompt after Connect while an imported file is open: the meter's
# session takes its place. $lost, $samples and $markers are as for Record.
export-unsaved-connect =
    { $lost ->
        [samples]
            { $samples ->
                [one] Conectar cierra la sesión importada y descartará { $samples } muestra sin exportar.
               *[other] Conectar cierra la sesión importada y descartará { $samples } muestras sin exportar.
            }
        [markers]
            { $markers ->
                [one] Conectar cierra la sesión importada y descartará los cambios no guardados de { $markers } marcador.
               *[other] Conectar cierra la sesión importada y descartará los cambios no guardados de { $markers } marcadores.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Conectar cierra la sesión importada y descartará { $samples } muestra sin exportar y los cambios no guardados de { $markers } marcador.
                       *[other] Conectar cierra la sesión importada y descartará { $samples } muestra sin exportar y los cambios no guardados de { $markers } marcadores.
                    }
               *[other]
                    { $markers ->
                        [one] Conectar cierra la sesión importada y descartará { $samples } muestras sin exportar y los cambios no guardados de { $markers } marcador.
                       *[other] Conectar cierra la sesión importada y descartará { $samples } muestras sin exportar y los cambios no guardados de { $markers } marcadores.
                    }
            }
    }
# The prompt's button that goes ahead: discards, then connects the meter.
export-unsaved-connect-confirm = Descartar y conectar
# The prompt's other button, which keeps everything; it has the focus.
export-unsaved-cancel = Cancelar

## The file type the system's open and save dialogs list, next to the
## file's extension (.csv, .json, .replay).

# Import…'s open dialog: every type it reads.
import-file-type = Lecturas exportadas
# Export…'s save dialog after CSV…; CSV is the file type's name.
export-file-type-csv = CSV
# After JSON…; JSON is the file type's name.
export-file-type-json = JSON
# After Replay…: a replay file.
export-file-type-replay = Reproducción

## Messages that show for a few seconds at the top right.

# While a large file is read in. $file is the file's name; $percent how
# much of it is in so far, a whole number from 0 to 99.
import-progress = Importando { $file }… { $percent } %
# When the file is in. $count is how many readings it gave; $file is its
# name.
import-done =
    { $count ->
        [one] Se importó { $count } lectura de { $file }
       *[other] Se importaron { $count } lecturas de { $file }
    }
# When the file holds more readings than the Buffer size setting (in
# Settings) lets the app keep: the first ones are in, the rest are not.
# $count is how many came in; $left how many were left out; $file is the
# file's name.
import-done-partial =
    { $count ->
        [one]
            { $left ->
                [one] Se importó { $count } lectura (la primera) de { $file }; se omitió { $left } lectura que superaba el Tamaño del búfer fijado en Configuración
               *[other] Se importó { $count } lectura (la primera) de { $file }; se omitieron { $left } lecturas que superaban el Tamaño del búfer fijado en Configuración
            }
       *[other]
            { $left ->
                [one] Se importaron las primeras { $count } lecturas de { $file }; se omitió { $left } lectura que superaba el Tamaño del búfer fijado en Configuración
               *[other] Se importaron las primeras { $count } lecturas de { $file }; se omitieron { $left } lecturas que superaban el Tamaño del búfer fijado en Configuración
            }
    }
# The file was read but has no readings in it. $file is its name.
import-empty = { $file } no contiene lecturas
# The file could not be read. $path is where it is; $reason why, in the
# words of the file reader or the system, often in English.
import-failed = Error al importar: { $path }: { $reason }
# Reading the file stopped on an internal error. $path is where it is.
import-failed-crashed = Error al importar: { $path }: la importación no se pudo completar
# The reader stopped without saying why.
import-failed-unexplained = Error al importar
# After Export… saved the file. $count is how many samples it holds; $file
# its name, e.g. measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv.
export-done =
    { $count ->
        [one] Se exportó { $count } muestra a { $file }
       *[other] Se exportaron { $count } muestras a { $file }
    }
# After Export… with nothing to save: the recording is running or stopped
# but empty.
export-nothing-recorded = Nada que exportar: la grabación aún no tiene muestras
# After Export… with nothing to save and no meter connected.
export-nothing-connect = Nada que exportar: conecta primero un multímetro
# After Export… with nothing to save: a meter is connected but has sent
# nothing yet.
export-nothing-yet = Nada que exportar: aún no hay lecturas
# After Export… while an earlier export still waits on its save dialog or
# is being written.
export-in-progress = Ya hay una exportación en curso. Termínala primero.
# The file could not be written. $reason is why, in the words of the
# system or the file writer, often in English.
export-failed = Error al exportar: { $reason }
# The save dialog went away without saying which file to write.
export-failed-dialog = Error al exportar: el cuadro de diálogo Guardar se cerró sin indicar ningún archivo
# The part of the app writing the file stopped without saying how it went.
export-failed-writer = Error al exportar: el proceso de escritura se detuvo sin respuesta
# The samples to export were dropped while the save dialog was open, so
# the file would not hold what was asked for.
export-failed-changed = Error al exportar: las muestras cambiaron mientras el cuadro de diálogo Guardar estaba abierto
