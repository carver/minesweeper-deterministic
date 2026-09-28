//! The desktop window: controls on top, the board below, scaled to fit.

mod help_task;
mod paint;

use std::time::Instant;

use eframe::egui::{self, Color32, CursorIcon, PointerButton, Rect, Sense, Stroke, StrokeKind, Ui, vec2};

use crate::game::{Automation, Config, Game, HelpStart, Look, Status};
use crate::grid::Pos;
use crate::rng::Rng;
use help_task::HelpTask;
use paint::Mood;

const FACE_SIZE: f32 = 34.0;
const BORDER: f32 = 10.0;
const MIN_SQUARE: f32 = 12.0;
const MAX_SQUARE: f32 = 64.0;
const GRANTED_GLOW: Color32 = Color32::from_rgb(0xe8, 0xc5, 0x00);

pub struct App {
    game: Game,
    /// Zero on the game's automation clock.
    started: Instant,
    /// The square under a held primary or middle button, if any. `Some(None)`
    /// means held but off the board's squares.
    held: Option<Option<Pos>>,
    help: Option<HelpTask>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            game: Game::new(Config::EXPERT, Automation::default(), Rng::from_time()),
            started: Instant::now(),
            held: None,
            help: None,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.game.advance_to(self.clock());
        self.collect_help();

        egui::Panel::top("controls").show(ui, |ui| {
            ui.add_space(4.0);
            self.controls(ui);
            ui.add_space(4.0);
        });
        egui::CentralPanel::default().show(ui, |ui| self.board(ui));

        if self.game.is_busy() {
            ui.ctx().request_repaint();
        }
    }
}

impl App {
    fn clock(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    fn restart(&mut self) {
        self.game.restart();
        self.started = Instant::now();
        self.held = None;
        self.help = None;
    }

    /// Applies a finished help answer, or cancels one the board has
    /// outdated.
    fn collect_help(&mut self) {
        let Some(task) = &self.help else {
            return;
        };
        if task.job().revision != self.game.revision() {
            self.help = None;
            return;
        }
        let Some(answer) = task.poll() else {
            return;
        };
        let task = self.help.take().expect("checked above");
        if let Ok(solution) = answer {
            self.game.conclude_help(task.job(), solution);
        }
    }

    fn ask_for_help(&mut self, ctx: &egui::Context) {
        if let HelpStart::NeedsSolver(job) = self.game.request_help() {
            let ctx = ctx.clone();
            self.help = Some(HelpTask::spawn(job, move || ctx.request_repaint()));
        }
    }

    fn working(&self) -> bool {
        self.game.is_busy() || self.help.is_some()
    }

    fn controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("Mines left: {}", self.game.mines_left()));
            ui.add_space(8.0);
            if self.face_button(ui) {
                self.restart();
            }
            ui.add_space(8.0);

            let automation = self.game.automation();
            if ui
                .radio(automation == Automation::ZerosOnly, "No extra automation")
                .clicked()
            {
                self.game.set_automation(Automation::ZerosOnly);
            }
            if ui
                .radio(
                    automation == Automation::LocalConstraints,
                    "Resolve local constraints",
                )
                .clicked()
            {
                self.game.set_automation(Automation::LocalConstraints);
            }
            ui.add_space(8.0);

            let can_ask = self.game.status() == Status::Playing && !self.working();
            if ui
                .add_enabled(can_ask, egui::Button::new("Request help"))
                .clicked()
            {
                self.ask_for_help(ui.ctx());
            }
            if self.working() {
                ui.label("working");
            } else if self.game.help_granted() {
                ui.label(egui::RichText::new("Granted!").strong());
            }
        });
    }

    /// Returns whether it was clicked.
    fn face_button(&self, ui: &mut Ui) -> bool {
        let (rect, response) = ui.allocate_exact_size(vec2(FACE_SIZE, FACE_SIZE), Sense::click());
        let mood = match self.game.status() {
            Status::Won => Mood::Won,
            Status::Lost => Mood::Dead,
            Status::Playing if self.held.is_some() => Mood::Tense,
            Status::Playing => Mood::Happy,
        };
        let pressed = response.is_pointer_button_down_on();
        paint::face(ui.painter(), rect, mood, pressed);
        response.clicked()
    }

    fn board(&mut self, ui: &mut Ui) {
        let dims = self.game.dims();
        let available = ui.available_rect_before_wrap();
        let fit = |space: f32, count: usize| (space - 2.0 * BORDER) / count as f32;
        let side = fit(available.width(), dims.width)
            .min(fit(available.height(), dims.height))
            .floor()
            .clamp(MIN_SQUARE, MAX_SQUARE);
        let grid_size = vec2(side * dims.width as f32, side * dims.height as f32);
        let (area, _) = ui.allocate_exact_size(available.size(), Sense::click_and_drag());
        let grid = Rect::from_center_size(area.center(), grid_size);

        let square_at = |p: egui::Pos2| {
            grid.contains(p).then(|| {
                let x = ((p.x - grid.min.x) / side) as usize;
                let y = ((p.y - grid.min.y) / side) as usize;
                Pos::new(x.min(dims.width - 1), y.min(dims.height - 1))
            })
        };
        self.handle_pointer(ui, square_at);

        let painter = ui.painter_at(area);
        let frame_rect = grid.expand(BORDER);
        paint::sunken_frame(&painter, frame_rect, BORDER);
        if self.game.help_granted() {
            painter.rect_stroke(
                frame_rect,
                0.0,
                Stroke::new(3.0, GRANTED_GLOW),
                StrokeKind::Inside,
            );
        }
        let held = self.held.flatten();
        for pos in dims.positions() {
            let min = grid.min + vec2(pos.x as f32 * side, pos.y as f32 * side);
            let rect = Rect::from_min_size(min, vec2(side, side));
            paint::cell(&painter, rect, self.game.cell(pos), held == Some(pos));
        }

        let hovering = ui.ctx().pointer_hover_pos().is_some_and(|p| grid.contains(p));
        if hovering && self.game.help_granted() {
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        }
    }

    /// Mirrors the original: right click (or a modifier with left click)
    /// toggles a flag on press; left or middle opens or chords on release,
    /// on whatever square the pointer is over by then.
    fn handle_pointer(&mut self, ui: &Ui, square_at: impl Fn(egui::Pos2) -> Option<Pos>) {
        if self.game.status() != Status::Playing {
            self.held = None;
            return;
        }
        let (pointer, pressed, released, flag_modifier) = ui.input(|i| {
            let pressed = |b| i.pointer.button_pressed(b);
            let released = |b| i.pointer.button_released(b);
            (
                i.pointer.latest_pos(),
                [
                    pressed(PointerButton::Primary),
                    pressed(PointerButton::Secondary),
                    pressed(PointerButton::Middle),
                ],
                released(PointerButton::Primary) || released(PointerButton::Middle),
                i.modifiers.ctrl || i.modifiers.shift || i.modifiers.alt,
            )
        });
        let [primary, secondary, middle] = pressed;
        let under = pointer.and_then(&square_at);
        let on_window = pointer.is_some_and(|p| ui.max_rect().contains(p));

        if let Some(pos) = under {
            if secondary || (primary && flag_modifier) {
                self.game.toggle_flag(pos);
            } else if primary || middle {
                self.held = Some(Some(pos));
            }
        }
        if self.held.is_some() {
            self.held = on_window.then_some(under);
        }
        if released
            && let Some(Some(pos)) = self.held.take()
            && self.game.cell(pos).look != Look::Flag
        {
            self.game.click(pos);
        }
    }
}
