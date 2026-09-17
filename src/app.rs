use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align2, Color32, FontId, Key, Pos2, RichText, Sense, Stroke, StrokeKind, Vec2, vec2,
};
use serde::{Deserialize, Serialize};

use crate::audio::{Engine, Noise};
use crate::presets::{self, BEAT_RANGE, BUILTINS, CARRIER_RANGE, Preset};

const STORAGE_KEY: &str = "settings";
const TIMER_CHOICES: [u32; 7] = [0, 15, 25, 30, 45, 60, 90];
/// Fade-out used when a session timer ends, so it doesn't cut off abruptly.
const TIMER_RELEASE_SECS: f32 = 8.0;
const STOP_RELEASE_SECS: f32 = 0.08;
const ACCENT: Color32 = Color32::from_rgb(64, 170, 160);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Active {
    Builtin(String),
    User(String),
    Custom,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    carrier: f32,
    beat: f32,
    active: Active,
    volume: f32,
    noise: Noise,
    noise_level: f32,
    glide_secs: f32,
    timer_minutes: u32,
    user_presets: Vec<Preset>,
}

impl Default for Settings {
    fn default() -> Self {
        let first = &BUILTINS[0];
        Self {
            carrier: first.carrier,
            beat: first.beat,
            active: Active::Builtin(first.name.to_string()),
            volume: 0.4,
            noise: Noise::Pink,
            noise_level: 0.15,
            glide_secs: 4.0,
            timer_minutes: 0,
            user_presets: Vec::new(),
        }
    }
}

pub struct App {
    s: Settings,
    engine: Engine,
    carrier_text: String,
    beat_text: String,
    input_error: Option<String>,
    new_preset_name: String,
    timer_end: Option<Instant>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_style(&cc.egui_ctx);
        let s: Settings = cc
            .storage
            .and_then(|st| eframe::get_value(st, STORAGE_KEY))
            .unwrap_or_default();
        let engine = Engine::new();
        engine.set_tone(s.carrier, s.beat, 0.0);
        engine.set_volume(s.volume);
        engine.set_noise(s.noise, s.noise_level);
        Self {
            carrier_text: fmt_hz(s.carrier),
            beat_text: fmt_hz(s.beat),
            s,
            engine,
            input_error: None,
            new_preset_name: String::new(),
            timer_end: None,
        }
    }

    fn select(&mut self, carrier: f32, beat: f32, active: Active) {
        self.s.carrier = carrier;
        self.s.beat = beat;
        self.s.active = active;
        self.carrier_text = fmt_hz(carrier);
        self.beat_text = fmt_hz(beat);
        self.input_error = None;
        let glide = if self.engine.is_playing() {
            self.s.glide_secs
        } else {
            0.0
        };
        self.engine.set_tone(carrier, beat, glide);
    }

    fn select_builtin(&mut self, i: usize) {
        let p = &BUILTINS[i];
        self.select(p.carrier, p.beat, Active::Builtin(p.name.to_string()));
    }

    fn apply_custom(&mut self) {
        let carrier = presets::parse_hz(&self.carrier_text, &CARRIER_RANGE)
            .map_err(|e| format!("Carrier {e}"));
        let beat = presets::parse_hz(&self.beat_text, &BEAT_RANGE).map_err(|e| format!("Beat {e}"));
        match (carrier, beat) {
            (Ok(c), Ok(b)) => self.select(c, b, Active::Custom),
            (Err(e), _) | (_, Err(e)) => self.input_error = Some(e),
        }
    }

    fn save_preset(&mut self) {
        let name = self.new_preset_name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let preset = Preset {
            name: name.clone(),
            carrier: self.s.carrier,
            beat: self.s.beat,
        };
        match self.s.user_presets.iter_mut().find(|p| p.name == name) {
            Some(existing) => *existing = preset,
            None => self.s.user_presets.push(preset),
        }
        self.s.active = Active::User(name);
        self.new_preset_name.clear();
    }

    fn toggle_play(&mut self) {
        if self.engine.is_playing() {
            self.engine.stop(STOP_RELEASE_SECS);
            self.timer_end = None;
        } else {
            self.engine.play();
            self.timer_end = (self.s.timer_minutes > 0 && self.engine.is_playing()).then(|| {
                Instant::now() + Duration::from_secs(u64::from(self.s.timer_minutes) * 60)
            });
        }
    }

    fn housekeeping(&mut self, ctx: &egui::Context) {
        if let Some(end) = self.timer_end {
            let now = Instant::now();
            if now >= end {
                self.engine.stop(TIMER_RELEASE_SECS);
                self.timer_end = None;
            } else {
                // Repaint once per second for the countdown; nothing else runs while idle.
                ctx.request_repaint_after((end - now).min(Duration::from_secs(1)));
            }
        }
        if self.engine.tick() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let keys = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5];
        let (space, digit) = ctx.input(|i| {
            (
                i.key_pressed(Key::Space),
                keys.iter().position(|k| i.key_pressed(*k)),
            )
        });
        if space {
            self.toggle_play();
        }
        if let Some(i) = digit.filter(|i| *i < BUILTINS.len()) {
            self.select_builtin(i);
        }
    }

    fn now_playing(&mut self, ui: &mut egui::Ui) {
        let (title, subtitle) = match &self.s.active {
            Active::Builtin(name) | Active::User(name) => (
                name.clone(),
                format!(
                    "{} · {} Hz beat",
                    presets::band_for(self.s.beat),
                    fmt_hz(self.s.beat)
                ),
            ),
            Active::Custom => (
                "Custom".to_string(),
                format!(
                    "{} · {} Hz beat",
                    presets::band_for(self.s.beat),
                    fmt_hz(self.s.beat)
                ),
            ),
        };
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).size(22.0).strong());
                    ui.label(RichText::new(subtitle).weak());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "L {} Hz\nR {} Hz",
                            fmt_hz(self.s.carrier),
                            fmt_hz(self.s.carrier + self.s.beat)
                        ))
                        .monospace()
                        .weak(),
                    );
                });
            });
            ui.add_space(6.0);
            beat_envelope(ui, self.s.beat, self.engine.is_playing());
            ui.add_space(6.0);

            let playing = self.engine.is_playing();
            let label = if playing { "Stop" } else { "Play" };
            let button = egui::Button::new(RichText::new(label).size(18.0).strong())
                .fill(if playing {
                    Color32::from_rgb(70, 70, 78)
                } else {
                    ACCENT
                })
                .corner_radius(8.0);
            if ui
                .add_sized([ui.available_width(), 44.0], button)
                .on_hover_text("Space")
                .clicked()
            {
                self.toggle_play();
            }

            if let Some(end) = self.timer_end {
                let left = end.saturating_duration_since(Instant::now()).as_secs();
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!("Ends in {}:{:02}", left / 60, left % 60)).weak(),
                    );
                });
            }
            if let Some(err) = &self.engine.error {
                ui.colored_label(Color32::from_rgb(230, 110, 100), err);
            }
        });
    }

    fn preset_list(&mut self, ui: &mut egui::Ui) {
        section(ui, "Presets");
        let mut picked = None;
        for (i, p) in BUILTINS.iter().enumerate() {
            let selected = self.s.active == Active::Builtin(p.name.to_string());
            let right = format!("{} Hz", fmt_hz(p.beat));
            let subtitle = format!("{} · {}", p.band, p.blurb);
            if preset_row(ui, selected, p.name, &subtitle, &right)
                .on_hover_text(format!("Key {}", i + 1))
                .clicked()
            {
                picked = Some(i);
            }
        }
        if let Some(i) = picked {
            self.select_builtin(i);
        }

        if self.s.user_presets.is_empty() {
            return;
        }
        section(ui, "My presets");
        let mut picked = None;
        let mut remove = None;
        for (i, p) in self.s.user_presets.iter().enumerate() {
            let selected = self.s.active == Active::User(p.name.clone());
            let subtitle = format!(
                "{} · carrier {} Hz",
                presets::band_for(p.beat),
                fmt_hz(p.carrier)
            );
            let right = format!("{} Hz", fmt_hz(p.beat));
            let resp = preset_row(ui, selected, &p.name, &subtitle, &right);
            if resp.clicked() {
                picked = Some(i);
            }
            resp.context_menu(|ui| {
                if ui.button("Delete preset").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = picked {
            let p = self.s.user_presets[i].clone();
            self.select(p.carrier, p.beat, Active::User(p.name));
        }
        if let Some(i) = remove {
            let p = self.s.user_presets.remove(i);
            if self.s.active == Active::User(p.name) {
                self.s.active = Active::Custom;
            }
        }
        ui.label(
            RichText::new("Right-click a preset to delete it.")
                .small()
                .weak(),
        );
    }

    fn custom(&mut self, ui: &mut egui::Ui) {
        section(ui, "Custom frequencies");
        card(ui, |ui| {
            let mut submit = false;
            egui::Grid::new("custom")
                .num_columns(3)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Carrier");
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.carrier_text)
                            .desired_width(90.0)
                            .hint_text("200"),
                    );
                    submit |= r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    ui.label(RichText::new("Hz  tone in left ear").weak());
                    ui.end_row();

                    ui.label("Beat");
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.beat_text)
                            .desired_width(90.0)
                            .hint_text("40"),
                    );
                    submit |= r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    ui.label(RichText::new("Hz  difference, right ear").weak());
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Apply").clicked() {
                    submit = true;
                }
                ui.label(RichText::new("or press Enter").small().weak());
            });
            if submit {
                self.apply_custom();
            }
            if let Some(err) = &self.input_error {
                ui.colored_label(Color32::from_rgb(230, 110, 100), err);
            }

            ui.separator();
            ui.horizontal(|ui| {
                let r = ui.add(
                    egui::TextEdit::singleline(&mut self.new_preset_name)
                        .desired_width(ui.available_width() - 120.0)
                        .hint_text("Name this setting"),
                );
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                let can_save = !self.new_preset_name.trim().is_empty();
                if ui
                    .add_enabled(can_save, egui::Button::new("Save preset"))
                    .clicked()
                    || (enter && can_save)
                {
                    self.save_preset();
                }
            });
        });
    }

    fn sound(&mut self, ui: &mut egui::Ui) {
        section(ui, "Sound");
        card(ui, |ui| {
            egui::Grid::new("sound")
                .num_columns(2)
                .spacing([12.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Volume");
                    let mut pct = self.s.volume * 100.0;
                    if ui
                        .add(
                            egui::Slider::new(&mut pct, 0.0..=100.0)
                                .suffix("%")
                                .fixed_decimals(0),
                        )
                        .changed()
                    {
                        self.s.volume = pct / 100.0;
                        self.engine.set_volume(self.s.volume);
                    }
                    ui.end_row();

                    ui.label("Noise");
                    ui.horizontal(|ui| {
                        let mut changed = false;
                        egui::ComboBox::from_id_salt("noise")
                            .width(80.0)
                            .selected_text(self.s.noise.label())
                            .show_ui(ui, |ui| {
                                for n in Noise::ALL {
                                    changed |= ui
                                        .selectable_value(&mut self.s.noise, n, n.label())
                                        .changed();
                                }
                            });
                        ui.add_enabled_ui(self.s.noise != Noise::Off, |ui| {
                            let mut pct = self.s.noise_level * 100.0;
                            if ui
                                .add(
                                    egui::Slider::new(&mut pct, 0.0..=100.0)
                                        .suffix("%")
                                        .fixed_decimals(0),
                                )
                                .changed()
                            {
                                self.s.noise_level = pct / 100.0;
                                changed = true;
                            }
                        });
                        if changed {
                            self.engine.set_noise(self.s.noise, self.s.noise_level);
                        }
                    });
                    ui.end_row();

                    ui.label("Transition");
                    ui.add(
                        egui::Slider::new(&mut self.s.glide_secs, 0.0..=30.0)
                            .suffix(" s")
                            .fixed_decimals(0),
                    )
                    .on_hover_text("Glide time when switching presets during playback");
                    ui.end_row();

                    ui.label("Timer");
                    egui::ComboBox::from_id_salt("timer")
                        .width(80.0)
                        .selected_text(timer_label(self.s.timer_minutes))
                        .show_ui(ui, |ui| {
                            for m in TIMER_CHOICES {
                                if ui
                                    .selectable_value(&mut self.s.timer_minutes, m, timer_label(m))
                                    .changed()
                                    && self.engine.is_playing()
                                {
                                    self.timer_end = (m > 0).then(|| {
                                        Instant::now() + Duration::from_secs(u64::from(m) * 60)
                                    });
                                }
                            }
                        });
                    ui.end_row();
                });
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.housekeeping(&ctx);
        self.shortcuts(&ctx);

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(4.0);
                self.now_playing(ui);
                self.preset_list(ui);
                self.custom(ui);
                self.sound(ui);
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Use stereo headphones").small().weak());
                });
                ui.add_space(4.0);
            });
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, STORAGE_KEY, &self.s);
    }
}

fn setup_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = vec2(8.0, 6.0);
        style.spacing.button_padding = vec2(10.0, 5.0);
        style.spacing.slider_width = 180.0;
        let v = &mut style.visuals;
        v.selection.bg_fill = ACCENT.linear_multiply(0.55);
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        v.panel_fill = Color32::from_rgb(22, 23, 27);
        v.extreme_bg_color = Color32::from_rgb(14, 15, 18);
    });
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(title.to_uppercase()).small().strong().weak());
    ui.add_space(2.0);
}

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(Color32::from_rgb(32, 33, 39))
        .corner_radius(10.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn preset_row(
    ui: &mut egui::Ui,
    selected: bool,
    title: &str,
    subtitle: &str,
    right: &str,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let base = Color32::from_rgb(32, 33, 39);
        let fill = if selected {
            ACCENT.linear_multiply(0.35)
        } else if resp.hovered() {
            Color32::from_rgb(42, 44, 52)
        } else {
            base
        };
        let stroke = if selected {
            Stroke::new(1.0, ACCENT)
        } else {
            Stroke::NONE
        };
        let p = ui.painter();
        p.rect(rect, 8.0, fill, stroke, StrokeKind::Inside);
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let left = rect.left_center() + vec2(12.0, 0.0);
        p.text(
            left - vec2(0.0, 9.0),
            Align2::LEFT_CENTER,
            title,
            FontId::proportional(15.0),
            text,
        );
        p.text(
            left + vec2(0.0, 10.0),
            Align2::LEFT_CENTER,
            subtitle,
            FontId::proportional(12.0),
            weak,
        );
        p.text(
            rect.right_center() - vec2(12.0, 0.0),
            Align2::RIGHT_CENTER,
            right,
            FontId::monospace(14.0),
            if selected { text } else { weak },
        );
    }
    ui.add_space(2.0);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Static plot of the perceived beat envelope over one second. Not animated, so it costs
/// nothing between interactions (and avoids flicker at gamma rates).
fn beat_envelope(ui: &mut egui::Ui, beat: f32, active: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 6.0, Color32::from_rgb(24, 25, 30));
    let steps = (rect.width() as usize).max(2);
    let points: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let env = (std::f32::consts::PI * beat * t).cos().abs();
            Pos2::new(
                rect.left() + t * rect.width(),
                rect.bottom() - 4.0 - env * (rect.height() - 8.0),
            )
        })
        .collect();
    let color = if active {
        ACCENT
    } else {
        Color32::from_gray(90)
    };
    p.line(points, Stroke::new(1.5, color));
    p.text(
        rect.right_top() + Vec2::new(-6.0, 3.0),
        Align2::RIGHT_TOP,
        "1 s",
        FontId::proportional(10.0),
        Color32::from_gray(110),
    );
}

fn timer_label(minutes: u32) -> String {
    if minutes == 0 {
        "Off".to_string()
    } else {
        format!("{minutes} min")
    }
}

fn fmt_hz(v: f32) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}
