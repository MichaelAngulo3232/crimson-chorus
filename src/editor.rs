// editor.rs — Crimson UI (egui, custom-painted)
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use std::sync::Arc;

use crate::{ChorusParams, WaveType};

// ---------- palette ----------
const BG: egui::Color32 = egui::Color32::from_rgb(15, 11, 12);
const PANEL: egui::Color32 = egui::Color32::from_rgb(28, 21, 23);
const TRACK: egui::Color32 = egui::Color32::from_rgb(48, 40, 43);
const BURGUNDY: egui::Color32 = egui::Color32::from_rgb(128, 22, 44);
const CRIMSON_LIGHT: egui::Color32 = egui::Color32::from_rgb(214, 72, 94);
const WHITE: egui::Color32 = egui::Color32::from_rgb(238, 234, 235);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(148, 138, 141);

pub(crate) fn default_state() -> Arc<EguiState> {
    EguiState::from_size(640, 400)
}

/// Points along a circular arc (screen coords: y-down, angles clockwise).
fn arc_points(
    center: egui::Pos2,
    radius: f32,
    start: f32,
    sweep: f32,
    segments: usize,
) -> Vec<egui::Pos2> {
    (0..=segments)
        .map(|i| {
            let a = start + sweep * (i as f32 / segments as f32);
            center + egui::vec2(a.cos(), a.sin()) * radius
        })
        .collect()
}

/// A custom-painted rotary knob bound to any nih-plug parameter.
/// Drag vertically to change, Shift-drag for fine control, double-click to reset.
fn param_knob<P: Param>(ui: &mut egui::Ui, setter: &ParamSetter, param: &P, label: &str) {
    const DIAMETER: f32 = 84.0;
    const START: f32 = 0.75 * std::f32::consts::PI; // 135 deg: bottom-left
    const SWEEP: f32 = 1.5 * std::f32::consts::PI; // 270 deg, gap at the bottom

    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new(label).size(11.0).color(TEXT_DIM));
        ui.add_space(6.0);

        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(DIAMETER, DIAMETER), egui::Sense::click_and_drag());

        // --- interaction: begin/end gestures so host automation records cleanly ---
        if response.drag_started() {
            setter.begin_set_parameter(param);
        }
        if response.dragged() {
            let fine = if ui.input(|i| i.modifiers.shift) { 0.1 } else { 1.0 };
            let delta = -response.drag_delta().y * 0.005 * fine;
            let new = (param.unmodulated_normalized_value() + delta).clamp(0.0, 1.0);
            setter.set_parameter_normalized(param, new);
        }
        if response.drag_stopped() {
            setter.end_set_parameter(param);
        }
        if response.double_clicked() {
            let default = param.preview_normalized(param.default_plain_value());
            setter.begin_set_parameter(param);
            setter.set_parameter_normalized(param, default);
            setter.end_set_parameter(param);
        }

        // --- paint ---
        let value = param.unmodulated_normalized_value();
        let center = rect.center();
        let radius = rect.width() * 0.5 - 3.0;
        let painter = ui.painter();

        painter.circle_filled(center, radius * 0.74, PANEL);
        painter.circle_stroke(center, radius * 0.74, egui::Stroke::new(1.0, TRACK));

        // full track
        painter.add(egui::Shape::line(
            arc_points(center, radius, START, SWEEP, 48),
            egui::Stroke::new(3.0, TRACK),
        ));

        // value arc
        if value > 0.001 {
            let color = if response.dragged() || response.hovered() {
                CRIMSON_LIGHT
            } else {
                BURGUNDY
            };
            painter.add(egui::Shape::line(
                arc_points(center, radius, START, SWEEP * value, 48),
                egui::Stroke::new(3.0, color),
            ));
        }

        // indicator needle
        let angle = START + SWEEP * value;
        let dir = egui::vec2(angle.cos(), angle.sin());
        painter.line_segment(
            [
                center + dir * (radius * 0.32),
                center + dir * (radius * 0.66),
            ],
            egui::Stroke::new(2.0, WHITE),
        );

        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(param.normalized_value_to_string(value, true))
                .size(12.5)
                .color(WHITE),
        );
    });
}

/// One segment of the waveform selector.
fn wave_button(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &EnumParam<WaveType>,
    this: WaveType,
    label: &str,
) {
    let selected = param.value() == this;
    let text = egui::RichText::new(label)
        .size(11.0)
        .color(if selected { WHITE } else { TEXT_DIM });
    let response = ui.add_sized([58.0, 26.0], egui::SelectableLabel::new(selected, text));
    if response.clicked() && !selected {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, this);
        setter.end_set_parameter(param);
    }
}

pub(crate) fn create(
    params: Arc<ChorusParams>,
    editor_state: Arc<EguiState>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        editor_state,
        params,
        // build: one-time theme setup
        |ctx, _| {
            let mut visuals = egui::Visuals::dark();
            visuals.selection.bg_fill = BURGUNDY;
            visuals.selection.stroke = egui::Stroke::new(1.0, CRIMSON_LIGHT);
            ctx.set_visuals(visuals);
        },
        // update: runs every frame
        |ctx, setter, params| {
            egui::CentralPanel::default()
                .frame(egui::Frame::default().fill(BG).inner_margin(24.0))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        // ---------- header ----------
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new("C R I M S O N")
                                .size(30.0)
                                .color(WHITE)
                                .strong(),
                        );
                        ui.add_space(7.0);
                        let (rule, _) = ui
                            .allocate_exact_size(egui::vec2(190.0, 3.0), egui::Sense::hover());
                        ui.painter().rect_filled(rule, 1.5, BURGUNDY);
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("WARM  METALLIC  CHORUS")
                                .size(9.5)
                                .color(TEXT_DIM),
                        );

                        ui.add_space(26.0);

                        // ---------- knobs ----------
                        ui.columns(4, |cols| {
                            param_knob(&mut cols[0], setter, &params.rate, "RATE");
                            param_knob(&mut cols[1], setter, &params.depth, "DEPTH");
                            param_knob(&mut cols[2], setter, &params.feedback, "FEEDBACK");
                            param_knob(&mut cols[3], setter, &params.mix, "MIX");
                        });

                        ui.add_space(22.0);

                        // ---------- waveform ----------
                        ui.label(egui::RichText::new("WAVE").size(11.0).color(TEXT_DIM));
                        ui.add_space(6.0);
                        let row_width = 4.0 * 58.0 + 3.0 * 8.0;
                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() - row_width).max(0.0) * 0.5);
                            ui.spacing_mut().item_spacing.x = 8.0;
                            wave_button(ui, setter, &params.wave_type, WaveType::Sine, "SINE");
                            wave_button(ui, setter, &params.wave_type, WaveType::Triangle, "TRI");
                            wave_button(ui, setter, &params.wave_type, WaveType::Square, "SQUARE");
                            wave_button(ui, setter, &params.wave_type, WaveType::Sawtooth, "SAW");
                        });

                        ui.add_space(20.0);
                        ui.label(egui::RichText::new("PYFESSIONAL").size(9.0).color(TEXT_DIM));
                    });
                });
        },
    )
}