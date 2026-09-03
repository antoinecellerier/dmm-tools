//! The recording buffer's UI: the Record / Export row and its sample log,
//! the prompt shown before a new capture would discard unexported samples,
//! and the drag-resizable split between the graph and the recording panel.

use eframe::egui::{self, RichText, Ui};

use super::App;
use crate::a11y::ResponseA11yExt;

impl App {
    /// Start or stop recording.
    ///
    /// Starting clears the buffer, so if it holds samples that were never
    /// exported this asks first — a second Record press (or a mistyped
    /// Ctrl+R) used to destroy an unexported capture with no prompt, no
    /// toast, and nothing in the log.
    pub(super) fn toggle_recording(&mut self) {
        if !self.recording.active && self.recording.unexported_count() > 0 {
            self.confirm_discard_open = true;
            self.confirm_discard_focus_pending = true;
            return;
        }
        self.apply_recording_toggle();
    }

    /// Flip the recording state, remembering which meter the samples came
    /// from.
    ///
    /// The device has to be captured here rather than read back at export
    /// time: the Settings selection can change while a recording is held in
    /// the buffer (it only schedules a reconnect, and neither connect nor
    /// disconnect clears the samples), so reading it later labelled the file
    /// with whatever meter happened to be picked last.
    fn apply_recording_toggle(&mut self) {
        self.recording.toggle();
        if self.recording.active {
            self.recording_device = Some(self.selected_device().display_name);
            self.recording_aux_slots = self.device_aux_slots;
            // The transform's Raw sub-value needs a fixed column of its own,
            // after the meter's — see `recording_extra_slots`.
            self.recording_extra_slots = self.transform.extra_aux_count();
        }
    }

    /// Confirmation shown when starting a recording would discard samples
    /// that have not been written to a CSV.
    pub(super) fn show_discard_confirmation(&mut self, ctx: &egui::Context) {
        if !self.confirm_discard_open {
            return;
        }
        let unexported = self.recording.unexported_count();
        if unexported == 0 {
            // The buffer emptied under us (Clear, or an export completing
            // while the prompt was up) — nothing left to warn about.
            self.confirm_discard_open = false;
            self.apply_recording_toggle();
            return;
        }

        let focus_pending = std::mem::take(&mut self.confirm_discard_focus_pending);
        let mut discard = false;
        let mut cancel = false;
        // egui::Modal rather than Window: it sets the modal layer, which traps
        // keyboard focus inside the dialog.
        let modal = egui::Modal::new(egui::Id::new("confirm_discard_modal")).show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.heading("Discard unexported samples?");
            ui.add_space(4.0);
            let noun = if unexported == 1 { "sample" } else { "samples" };
            ui.label(format!(
                "Starting a new recording will discard {unexported} unexported {noun}."
            ));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                // Cancel takes focus: Enter and Space then default to the
                // non-destructive choice.
                let cancel_btn = ui.button("Cancel");
                if focus_pending {
                    cancel_btn.request_focus();
                }
                if cancel_btn.clicked() {
                    cancel = true;
                }
                if ui.button("Discard and record").clicked() {
                    discard = true;
                }
            });
        });

        if discard {
            self.confirm_discard_open = false;
            self.apply_recording_toggle();
        } else if cancel || modal.should_close() {
            self.confirm_discard_open = false;
        }
    }

    fn show_recording_section(&mut self, ui: &mut Ui, compact: bool) {
        let (btn_label, btn_tooltip) = if self.recording.active {
            ("\u{25A0} Stop", "Stop recording (Ctrl+R)")
        } else {
            (
                "\u{25CF} Record",
                "Start writing live samples to the recording buffer (Ctrl+R)",
            )
        };

        ui.horizontal(|ui| {
            if ui.button(btn_label).on_hover_text(btn_tooltip).clicked() {
                self.toggle_recording();
            }
            if ui
                .button("Export CSV")
                .on_hover_text("Save the recording buffer to a CSV file (Ctrl+E)")
                .clicked()
            {
                self.export_csv();
            }
            let count = self.recording.samples.len();
            if self.recording.active {
                let status = format!("{count} smp | {:.0}s", self.recording.duration_secs());
                if self.recording.is_full() {
                    let warn = self
                        .settings
                        .theme_colors(ui.visuals().dark_mode)
                        .recording_full_warning();
                    ui.label(RichText::new(format!("{status} (buffer full)")).color(warn));
                } else {
                    ui.label(status);
                }
            } else if count > 0 {
                ui.label(format!("{count} smp"));
            }
        });

        // Scrollable sample log
        if !self.recording.samples.is_empty() {
            let max_height = if compact {
                80.0
            } else {
                ui.available_height().max(60.0)
            };
            egui::ScrollArea::vertical()
                .max_height(max_height)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    let start = self.recording.samples.len().saturating_sub(500);
                    for s in &self.recording.samples[start..] {
                        let time = s.wall_time.format("%H:%M:%S%.3f");
                        let flags_str = s.flags_str();
                        let flags = if flags_str.is_empty() {
                            String::new()
                        } else {
                            format!(" [{flags_str}]")
                        };
                        // Sub-values trail the flags so the timestamp, value
                        // and unit columns stay where they are for meters
                        // that don't report any.
                        let summary = s.measurement.aux_summary();
                        let aux = if summary.is_empty() {
                            String::new()
                        } else {
                            format!("  {summary}")
                        };
                        ui.label(
                            RichText::new(format!(
                                "{time}  {val:>10} {unit}{flags}{aux}",
                                val = s.value_str(),
                                unit = s.unit(),
                            ))
                            .font(egui::FontId::monospace(11.0)),
                        );
                    }
                });
        }
    }

    /// Render the graph+recording area with a resizable drag separator between them.
    pub(super) fn show_graph_recording_split(&mut self, ui: &mut Ui, compact: bool) {
        // Built before `self.graph` is borrowed mutably below: the palette
        // reads `self.settings`, which the borrow checker would otherwise
        // see as overlapping.
        let tc = self.settings.theme_colors(ui.visuals().dark_mode);
        if self.settings.show_graph && self.settings.show_recording {
            let total = ui.available_height();
            let graph_height = (total - self.recording_height).max(80.0);

            ui.allocate_ui(egui::vec2(ui.available_width(), graph_height), |ui| {
                self.graph.show(ui, &tc);
            });

            let sep = ui.separator();
            let sep_id = ui.id().with("rec_resize");
            let sep_response = ui
                .interact(
                    sep.rect.expand2(egui::vec2(0.0, 4.0)),
                    sep_id,
                    egui::Sense::drag(),
                )
                .a11y_label("Resize recording panel (Up/Down to adjust)");
            if sep_response.dragged() {
                self.recording_height = (self.recording_height - sep_response.drag_delta().y)
                    .clamp(40.0, (total - 80.0).max(40.0));
            }
            // Keyboard resize when focused: Up moves the divider up
            // (grows the recording panel), Down moves it down. Matches
            // mouse-drag direction.
            let delta = crate::a11y::arrow_resize(
                ui.ctx(),
                sep_id,
                crate::a11y::ResizeAxis::Vertical,
                20.0,
            );
            if delta != 0.0 {
                self.recording_height =
                    (self.recording_height + delta).clamp(40.0, (total - 80.0).max(40.0));
            }
            crate::a11y::paint_focus_ring(ui, &sep_response);
            if sep_response.hovered() || sep_response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }

            self.show_recording_section(ui, compact);
        } else if self.settings.show_graph {
            self.graph.show(ui, &tc);
        } else if self.settings.show_recording {
            self.show_recording_section(ui, compact);
        }
    }
}
