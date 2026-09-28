//! The desktop window: controls on top, the board below, scaled to fit.

mod help_task;
mod paint;

use std::time::{Instant, SystemTime};

use eframe::egui::{self, Color32, CursorIcon, PointerButton, Rect, Sense, Stroke, StrokeKind, Ui, vec2};

use crate::game::{Automation, Config, Game, HelpStart, Status};
use crate::grid::{Dims, Pos};
use crate::journal::Journal;
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
    /// The square under a held primary or middle button. Leaving the board
    /// lets go, as in the original.
    held: Option<Pos>,
    help: Option<HelpTask>,
    /// `None` if it could not be opened or a write failed.
    journal: Option<Journal>,
    /// Where the squares were last drawn.
    grid: Option<GridLayout>,
}

impl Default for App {
    fn default() -> Self {
        Self::restore(None, open_journal())
    }
}

/// Where the automation choice is kept between runs. The window size is
/// saved by eframe itself.
const AUTOMATION_KEY: &str = "automation";

fn automation_key(automation: Automation) -> &'static str {
    match automation {
        Automation::ZerosOnly => "zeros-only",
        Automation::LocalConstraints => "local-constraints",
    }
}

fn automation_from_key(key: &str) -> Option<Automation> {
    [Automation::ZerosOnly, Automation::LocalConstraints]
        .into_iter()
        .find(|&a| automation_key(a) == key)
}

/// The journal in its usual place, or `None` with the reason on stderr.
pub fn open_journal() -> Option<Journal> {
    let Some(dir) = Journal::default_dir() else {
        eprintln!("journal disabled: no HOME or XDG_STATE_HOME");
        return None;
    };
    match Journal::open(dir.clone(), SystemTime::now()) {
        Ok(journal) => {
            eprintln!("journal: {}", journal.dir().display());
            Some(journal)
        }
        Err(e) => {
            eprintln!("journal disabled: cannot open {}: {e}", dir.display());
            None
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let key = automation_key(self.game.automation());
        storage.set_string(AUTOMATION_KEY, key.to_owned());
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.game.advance_to(self.clock());
        self.collect_help();

        egui::Panel::top("controls").show(ui, |ui| {
            ui.add_space(4.0);
            self.controls(ui);
            ui.add_space(4.0);
        });
        egui::Panel::bottom("hint").show(ui, |ui| {
            ui.weak("Flag with right-click or Ctrl+click. Middle-click works like left-click.");
        });
        egui::CentralPanel::default().show(ui, |ui| self.board(ui));

        if self.game.is_busy() {
            ui.ctx().request_repaint();
        }
        self.write_journal();
    }
}

impl App {
    /// A new expert game with the automation chosen last time, if saved.
    pub fn restore(storage: Option<&dyn eframe::Storage>, journal: Option<Journal>) -> Self {
        let automation = storage
            .and_then(|s| s.get_string(AUTOMATION_KEY))
            .and_then(|key| automation_from_key(&key))
            .unwrap_or_default();
        let game = Game::new(Config::EXPERT, automation, Rng::from_time());
        Self::new(game, journal)
    }

    pub fn new(game: Game, journal: Option<Journal>) -> Self {
        Self {
            game,
            started: Instant::now(),
            held: None,
            help: None,
            journal,
            grid: None,
        }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    /// The centre of a square as last drawn, for driving the window in tests.
    pub fn square_centre(&self, pos: Pos) -> Option<egui::Pos2> {
        Some(self.grid?.square(pos).center())
    }

    fn clock(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    fn write_journal(&mut self) {
        let events = self.game.take_events();
        let Some(journal) = &mut self.journal else {
            return;
        };
        if let Err(e) = journal.write(SystemTime::now(), &events) {
            eprintln!("journal disabled: write failed: {e}");
            self.journal = None;
        }
    }

    fn restart(&mut self) {
        if self.help.take().is_some() {
            self.game.abandon_help();
        }
        self.game.restart();
        self.started = Instant::now();
        self.held = None;
    }

    /// Applies a finished help answer, or cancels one the board has
    /// outdated.
    fn collect_help(&mut self) {
        let Some(task) = &self.help else {
            return;
        };
        if task.job().revision != self.game.revision() {
            self.help = None;
            self.game.abandon_help();
            return;
        }
        let Some(answer) = task.poll() else {
            return;
        };
        let task = self.help.take().expect("checked above");
        match answer {
            Ok(solution) => {
                self.game.conclude_help(task.job(), solution);
            }
            Err(_) => self.game.abandon_help(),
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
            }
            if self.game.help_granted() {
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
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "New game"));
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
        let (area, _) = ui.allocate_exact_size(available.size(), Sense::click_and_drag());
        let grid = GridLayout {
            rect: Rect::from_center_size(
                area.center(),
                vec2(side * dims.width as f32, side * dims.height as f32),
            ),
            side,
        };
        self.grid = Some(grid);
        self.handle_pointer(ui, grid);

        let painter = ui.painter_at(area);
        let frame_rect = grid.rect.expand(BORDER);
        paint::sunken_frame(&painter, frame_rect, BORDER);
        if self.game.help_granted() {
            painter.rect_stroke(
                frame_rect,
                0.0,
                Stroke::new(3.0, GRANTED_GLOW),
                StrokeKind::Inside,
            );
        }
        for pos in dims.positions() {
            let pressed = self.held == Some(pos);
            paint::cell(&painter, grid.square(pos), self.game.cell(pos), pressed);
        }

        let hovering = ui
            .ctx()
            .pointer_hover_pos()
            .is_some_and(|p| grid.rect.contains(p));
        if hovering && self.game.help_granted() {
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        }
    }

    /// Mirrors the original: right click (or a modifier with left click)
    /// toggles a flag on press; left or middle opens or chords on release,
    /// on whatever square the pointer is over by then.
    fn handle_pointer(&mut self, ui: &Ui, grid: GridLayout) {
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
                i.events.iter().any(|e| {
                    matches!(e, egui::Event::PointerButton { button: PointerButton::Primary, pressed: true, modifiers, .. }
                        if modifiers.ctrl || modifiers.shift || modifiers.alt)
                }),
            )
        });
        let [primary, secondary, middle] = pressed;
        let under = pointer.and_then(|p| grid.square_at(p, self.game.dims()));

        if let Some(pos) = under {
            if secondary || (primary && flag_modifier) {
                self.game.toggle_flag(pos);
            } else if primary || middle {
                self.held = Some(pos);
            }
        }
        if self.held.is_some() {
            self.held = under;
        }
        if released && let Some(pos) = self.held.take() {
            self.game.click(pos);
        }
    }
}

/// Where the board's squares are on screen.
#[derive(Clone, Copy, Debug)]
struct GridLayout {
    rect: Rect,
    side: f32,
}

impl GridLayout {
    fn square(self, pos: Pos) -> Rect {
        let min = self.rect.min + vec2(pos.x as f32, pos.y as f32) * self.side;
        Rect::from_min_size(min, vec2(self.side, self.side))
    }

    fn square_at(self, p: egui::Pos2, dims: Dims) -> Option<Pos> {
        if !self.rect.contains(p) {
            return None;
        }
        let x = ((p.x - self.rect.min.x) / self.side) as usize;
        let y = ((p.y - self.rect.min.y) / self.side) as usize;
        Some(Pos::new(x.min(dims.width - 1), y.min(dims.height - 1)))
    }
}
