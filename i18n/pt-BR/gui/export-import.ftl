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
    .hover = Abrir um arquivo CSV, JSON ou de reprodução exportado ({ $keys })
# The left part of the Export… split button: saves a CSV file at once.
# $buffer is recording when a recording holds the samples, graph when
# nothing is recorded and Export… saves the samples the graph holds.
export-button = Exportar…
    .hover =
        { $buffer ->
            [recording] Salvar a gravação como arquivo CSV ({ $keys })
           *[graph] Salvar as amostras do gráfico como arquivo CSV ({ $keys })
        }
# The arrow at the right of the Export… split button, which drops the menu
# of file types. It has no text. $buffer is as for Export….
export-menu =
    .hover =
        { $buffer ->
            [recording] Salvar a gravação como arquivo CSV, JSON ou de reprodução
           *[graph] Salvar as amostras do gráfico como arquivo CSV, JSON ou de reprodução
        }
    .a11y = Tipo de arquivo

## The menu under the Export… arrow: one entry per file type. Each opens
## the save dialog for that type.

# CSV is the file type's name.
export-csv = CSV…
# JSON is the file type's name. dmm-cli read --format json is a command
# line, kept as it is.
export-json = JSON…
    .hover = Um objeto por linha, como o dmm-cli read --format json exibe
# A replay file. dmm-gui --replay is a command line, kept as it is; "as the
# meter" means as if it were the meter attached.
export-replay = Reprodução…
    .hover = Um arquivo que o dmm-gui --replay reproduz como se fosse o multímetro conectado
# The Replay… entry's hover while it is greyed out, and the error shown if
# a replay is asked for anyway. The mock is the simulated meter the app
# offers for trying it out; its samples hold none of the bytes a real meter
# sends (its wire format), which a replay file is made of.
export-no-wire-format = O multímetro simulado não tem dados brutos para exportar; conecte um multímetro real

## A small line under the buttons after Import…: the samples are the
## file's.

# $count is how many readings the file gave; $file the file's name, e.g.
# measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv. Export… is the button.
import-hint =
    { $count ->
        [one] { $count } leitura importada de { $file }. Use “Exportar…” para salvá-la com os marcadores e a visualização.
       *[other] { $count } leituras importadas de { $file }. Use “Exportar…” para salvá-las com os marcadores e a visualização.
    }

## The prompt shown over the window before something would discard samples
## or markers no file holds: Record over a recording that was never
## exported, Discard on a recording whose samples have left the graph,
## Import… or Connect while the session holds them. Markers are the
## numbered flags the user puts on readings, with a note each; "unsaved
## changes to a marker" means the marker, or its note, was added or changed
## since the last export.

# The prompt's title when samples would be lost (and maybe markers too).
export-unsaved-heading-samples = Descartar as amostras não exportadas?
# The prompt's title when only markers would be lost.
export-unsaved-heading-markers = Descartar os marcadores não exportados?
# The prompt after Record, which starts a new recording in place of the
# stopped one. $lost is samples when only samples would be lost, markers
# when only markers, both when both. $samples is how many samples were
# never exported; $markers how many markers have unsaved changes. Markers
# stay on the graph's readings even though the recording goes.
export-unsaved-record =
    { $lost ->
        [samples]
            { $samples ->
                [one] Iniciar uma nova gravação descartará { $samples } amostra não exportada.
               *[other] Iniciar uma nova gravação descartará { $samples } amostras não exportadas.
            }
        [markers]
            { $markers ->
                [one] Iniciar uma nova gravação descartará as alterações não salvas de { $markers } marcador. Os marcadores permanecem no gráfico enquanto as leituras deles continuarem nele.
               *[other] Iniciar uma nova gravação descartará as alterações não salvas de { $markers } marcadores. Os marcadores permanecem no gráfico enquanto as leituras deles continuarem nele.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Iniciar uma nova gravação descartará { $samples } amostra não exportada e as alterações não salvas de { $markers } marcador. Os marcadores permanecem no gráfico enquanto as leituras deles continuarem nele.
                       *[other] Iniciar uma nova gravação descartará { $samples } amostra não exportada e as alterações não salvas de { $markers } marcadores. Os marcadores permanecem no gráfico enquanto as leituras deles continuarem nele.
                    }
               *[other]
                    { $markers ->
                        [one] Iniciar uma nova gravação descartará { $samples } amostras não exportadas e as alterações não salvas de { $markers } marcador. Os marcadores permanecem no gráfico enquanto as leituras deles continuarem nele.
                       *[other] Iniciar uma nova gravação descartará { $samples } amostras não exportadas e as alterações não salvas de { $markers } marcadores. Os marcadores permanecem no gráfico enquanto as leituras deles continuarem nele.
                    }
            }
    }
# The prompt's button that goes ahead: discards, then starts recording.
export-unsaved-record-confirm = Descartar e gravar
# The prompt after Discard, which drops a stopped recording; Export… then
# saves the samples the graph still holds, so only the ones that have
# scrolled out of it are lost. $lost, $samples and $markers are as for
# Record; "them" in the last sentences means those samples.
export-unsaved-discard =
    { $lost ->
        [samples]
            { $samples ->
                [one] Ao descartar a gravação, { $samples } amostra que já saiu do gráfico será perdida.
               *[other] Ao descartar a gravação, { $samples } amostras que já saíram do gráfico serão perdidas.
            }
        [markers]
            { $markers ->
                [one] Ao descartar a gravação, as alterações não salvas de { $markers } marcador em leituras que já saíram do gráfico serão perdidas.
               *[other] Ao descartar a gravação, as alterações não salvas de { $markers } marcadores em leituras que já saíram do gráfico serão perdidas.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Ao descartar a gravação, { $samples } amostra que já saiu do gráfico e as alterações não salvas de { $markers } marcador nela serão perdidas.
                       *[other] Ao descartar a gravação, { $samples } amostra que já saiu do gráfico e as alterações não salvas de { $markers } marcadores nela serão perdidas.
                    }
               *[other]
                    { $markers ->
                        [one] Ao descartar a gravação, { $samples } amostras que já saíram do gráfico e as alterações não salvas de { $markers } marcador nelas serão perdidas.
                       *[other] Ao descartar a gravação, { $samples } amostras que já saíram do gráfico e as alterações não salvas de { $markers } marcadores nelas serão perdidas.
                    }
            }
    }
# The prompt's button that goes ahead: drops the recording.
export-unsaved-discard-confirm = Descartar a gravação
# The prompt after picking a file to import, which replaces everything the
# session holds. $lost, $samples and $markers are as for Record.
export-unsaved-import =
    { $lost ->
        [samples]
            { $samples ->
                [one] Importar substitui esta sessão e descarta { $samples } amostra não exportada.
               *[other] Importar substitui esta sessão e descarta { $samples } amostras não exportadas.
            }
        [markers]
            { $markers ->
                [one] Importar substitui esta sessão e descarta as alterações não salvas de { $markers } marcador.
               *[other] Importar substitui esta sessão e descarta as alterações não salvas de { $markers } marcadores.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Importar substitui esta sessão e descarta { $samples } amostra não exportada e as alterações não salvas de { $markers } marcador.
                       *[other] Importar substitui esta sessão e descarta { $samples } amostra não exportada e as alterações não salvas de { $markers } marcadores.
                    }
               *[other]
                    { $markers ->
                        [one] Importar substitui esta sessão e descarta { $samples } amostras não exportadas e as alterações não salvas de { $markers } marcador.
                       *[other] Importar substitui esta sessão e descarta { $samples } amostras não exportadas e as alterações não salvas de { $markers } marcadores.
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
                [one] Conectar encerra a sessão importada e descarta { $samples } amostra não exportada.
               *[other] Conectar encerra a sessão importada e descarta { $samples } amostras não exportadas.
            }
        [markers]
            { $markers ->
                [one] Conectar encerra a sessão importada e descarta as alterações não salvas de { $markers } marcador.
               *[other] Conectar encerra a sessão importada e descarta as alterações não salvas de { $markers } marcadores.
            }
       *[both]
            { $samples ->
                [one]
                    { $markers ->
                        [one] Conectar encerra a sessão importada e descarta { $samples } amostra não exportada e as alterações não salvas de { $markers } marcador.
                       *[other] Conectar encerra a sessão importada e descarta { $samples } amostra não exportada e as alterações não salvas de { $markers } marcadores.
                    }
               *[other]
                    { $markers ->
                        [one] Conectar encerra a sessão importada e descarta { $samples } amostras não exportadas e as alterações não salvas de { $markers } marcador.
                       *[other] Conectar encerra a sessão importada e descarta { $samples } amostras não exportadas e as alterações não salvas de { $markers } marcadores.
                    }
            }
    }
# The prompt's button that goes ahead: discards, then connects the meter.
export-unsaved-connect-confirm = Descartar e conectar
# The prompt's other button, which keeps everything; it has the focus.
export-unsaved-cancel = Cancelar

## The file type the system's open and save dialogs list, next to the
## file's extension (.csv, .json, .replay).

# Import…'s open dialog: every type it reads.
import-file-type = Leituras exportadas
# Export…'s save dialog after CSV…; CSV is the file type's name.
export-file-type-csv = CSV
# After JSON…; JSON is the file type's name.
export-file-type-json = JSON
# After Replay…: a replay file.
export-file-type-replay = Reprodução

## Messages that show for a few seconds at the top right.

# While a large file is read in. $file is the file's name; $percent how
# much of it is in so far, a whole number from 0 to 99.
import-progress = Importando { $file }… { $percent }%
# When the file is in. $count is how many readings it gave; $file is its
# name.
import-done =
    { $count ->
        [one] { $count } leitura importada de { $file }
       *[other] { $count } leituras importadas de { $file }
    }
# When the file holds more readings than the Buffer size setting (in
# Settings) lets the app keep: the first ones are in, the rest are not.
# $count is how many came in; $left how many were left out; $file is the
# file's name.
import-done-partial =
    { $count ->
        [one]
            { $left ->
                [one] Importada a primeira leitura de { $file }; { $left } leitura ficou de fora por exceder o Tamanho do buffer definido em Configurações
               *[other] Importada a primeira leitura de { $file }; { $left } leituras ficaram de fora por exceder o Tamanho do buffer definido em Configurações
            }
       *[other]
            { $left ->
                [one] Importadas as primeiras { $count } leituras de { $file }; { $left } leitura ficou de fora por exceder o Tamanho do buffer definido em Configurações
               *[other] Importadas as primeiras { $count } leituras de { $file }; { $left } leituras ficaram de fora por exceder o Tamanho do buffer definido em Configurações
            }
    }
# The file was read but has no readings in it. $file is its name.
import-empty = { $file } não contém nenhuma leitura
# The file could not be read. $path is where it is; $reason why, in the
# words of the file reader or the system, often in English.
import-failed = Falha ao importar: { $path }: { $reason }
# Reading the file stopped on an internal error. $path is where it is.
import-failed-crashed = Falha ao importar: { $path }: a importação falhou
# The file dialog of Import… failed.
import-dialog-failed = Falha ao importar
# After Export… saved the file. $count is how many samples it holds; $file
# its name, e.g. measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv.
export-done =
    { $count ->
        [one] { $count } amostra exportada para { $file }
       *[other] { $count } amostras exportadas para { $file }
    }
# After Export… with nothing to save: the recording is running or stopped
# but empty.
export-nothing-recorded = Nada para exportar: a gravação ainda não tem amostras
# After Export… with nothing to save and no meter connected.
export-nothing-connect = Nada para exportar: conecte um multímetro primeiro
# After Export… with nothing to save: a meter is connected but has sent
# nothing yet.
export-nothing-yet = Nada para exportar: ainda não há leituras
# After Export… while an earlier export still waits on its save dialog or
# is being written.
export-in-progress = Já há uma exportação em andamento: conclua-a primeiro
# The file could not be written. $reason is why, in the words of the
# system or the file writer, often in English.
export-failed = Falha ao exportar: { $reason }
# The samples to export were dropped while the save dialog was open, so
# the file would not hold what was asked for.
export-failed-changed = Falha ao exportar: as amostras mudaram enquanto a caixa de diálogo Salvar estava aberta
