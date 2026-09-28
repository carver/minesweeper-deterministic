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
    /// Reported once the first frame is drawn, then cleared.
    startup: Option<Startup>,
    /// Asking whether to abandon the game in progress.
    confirming_new_game: bool,
}

/// When the process started and when the renderer was ready, to find out
/// where slow launches spend their time.
#[derive(Clone, Copy, Debug)]
pub struct Startup {
    pub launched: Instant,
    pub renderer_ready: Instant,
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
            ui.weak("Flag: right-click or Ctrl+click.   Keys: N or F2 new game, H request help.");
        });
        egui::CentralPanel::default().show(ui, |ui| self.board(ui));
        if self.confirming_new_game {
            self.confirm_new_game(ui.ctx());
        } else {
            self.shortcuts(ui);
        }

        if self.game.is_busy() {
            ui.ctx().request_repaint();
        }
        self.write_journal();
        self.report_startup();
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
            startup: None,
            confirming_new_game: false,
        }
    }

    /// Logs startup timing to stderr and the journal on the first frame.
    pub fn with_startup(mut self, startup: Startup) -> Self {
        self.startup = Some(startup);
        self
    }

    fn report_startup(&mut self) {
        let Some(Startup {
            launched,
            renderer_ready,
        }) = self.startup.take()
        else {
            return;
        };
        let ms = |t: Instant| t.duration_since(launched).as_millis();
        let text = format!(
            "startup: renderer ready after {} ms, first frame after {} ms",
            ms(renderer_ready),
            ms(Instant::now())
        );
        eprintln!("{text}");
        if let Some(journal) = &mut self.journal {
            let _ = journal.note(SystemTime::now(), &text);
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

    fn can_ask_for_help(&self) -> bool {
        self.game.status() == Status::Playing && !self.working()
    }

    fn shortcuts(&mut self, ui: &Ui) {
        let (new_game, help) = ui.input_mut(|i| {
            let mut pressed = |key| i.consume_key(egui::Modifiers::NONE, key);
            (
                pressed(egui::Key::N) | pressed(egui::Key::F2),
                pressed(egui::Key::H),
            )
        });
        if new_game {
            self.ask_for_new_game();
        }
        if help && self.can_ask_for_help() {
            self.ask_for_help(ui.ctx());
        }
    }

    /// Starts over at once unless that would throw away a game in progress.
    fn ask_for_new_game(&mut self) {
        if self.game.in_progress() {
            self.confirming_new_game = true;
        } else {
            self.restart();
        }
    }

    fn confirm_new_game(&mut self, ctx: &egui::Context) {
        let (confirm, cancel) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });
        let modal = egui::Modal::new(egui::Id::new("confirm-new-game")).show(ctx, |ui| {
            ui.label("Abandon this game and start a new one?");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let start = ui.button("Start new game").clicked();
                let keep = ui.button("Keep playing").clicked();
                (start, keep)
            })
            .inner
        });
        let (start, keep) = modal.inner;
        if start || confirm {
            self.confirming_new_game = false;
            self.restart();
        } else if keep || cancel || modal.should_close() {
            self.confirming_new_game = false;
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
                self.ask_for_new_game();
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

            if ui
                .add_enabled(self.can_ask_for_help(), egui::Button::new("Request help"))
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
        if self.game.status() != Status::Playing || self.confirming_new_game {
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
