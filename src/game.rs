//! Game rules: revealing, flagging, automation and winning or losing.
//!
//! Automation runs on a virtual clock so the UI can play it back as a
//! ripple. Each step schedules follow-ups on the neighbours of the square it
//! touched, orthogonal ones sooner than diagonal ones.

mod event;
mod help;
#[cfg(test)]
mod tests;

pub use event::{Event, Timed};
pub use help::{HelpStart, HelpVerdict, SolverJob};

use crate::board::{Board, View};
use crate::grid::{Dims, Pos};
use crate::rng::Rng;
use crate::schedule::{Millis, Schedule};

pub const ORTHOGONAL_DELAY: Millis = 20;
/// Diagonal neighbours are √2 times further away.
pub const DIAGONAL_DELAY: Millis = 28;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub dims: Dims,
    pub mines: usize,
}

impl Config {
    pub const EXPERT: Self = Self {
        dims: Dims::new(30, 16),
        mines: 99,
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Automation {
    /// Only the classic flood fill from squares with no adjacent mines.
    ZerosOnly,
    /// Also flag and reveal whatever a single number settles.
    #[default]
    LocalConstraints,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Playing,
    Won,
    Lost,
}

/// How a square should be drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    Covered,
    Flag,
    Number(u8),
    /// An unflagged mine, shown after losing.
    Mine,
    /// A flag on a safe square, shown after losing.
    WrongFlag,
    Exploded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellInfo {
    pub look: Look,
    /// Highlighted because the player missed it when asking for help.
    pub missed: bool,
    /// Opened with granted help.
    pub opened_by_help: bool,
}

#[derive(Clone, Copy, Debug)]
enum Task {
    Reveal(Pos),
    RevealAroundIfFlagged(Pos),
    FlagAroundIfCovered(Pos),
}

pub struct Game {
    config: Config,
    board: Board,
    rng: Rng,
    automation: Automation,
    status: Status,
    /// The first revealed square is never a mine; its mine gets moved.
    first_reveal: bool,
    schedule: Schedule<Task>,
    /// Help was granted: the next square the player opens is safe to open.
    help_granted: bool,
    missed: Vec<bool>,
    opened_by_help: Vec<bool>,
    exploded: Option<Pos>,
    /// Bumped on every change to the visible board, so stale help answers
    /// can be recognised.
    revision: u64,
    /// Not yet collected by [`Game::take_events`].
    events: Vec<Timed>,
}

impl Game {
    pub fn new(config: Config, automation: Automation, rng: Rng) -> Self {
        let mut rng = rng;
        let board = Board::random(config.dims, config.mines, &mut rng);
        Self::with_board(board, automation, rng)
    }

    pub fn with_board(board: Board, automation: Automation, rng: Rng) -> Self {
        let dims = board.dims();
        let layout = board.layout();
        let mut game = Self {
            config: Config {
                dims,
                mines: board.mine_count(),
            },
            board,
            rng,
            automation,
            status: Status::Playing,
            first_reveal: true,
            schedule: Schedule::default(),
            help_granted: false,
            missed: vec![false; dims.area()],
            opened_by_help: vec![false; dims.area()],
            exploded: None,
            revision: 0,
            events: Vec::new(),
        };
        game.record(Event::NewGame { layout });
        game
    }

    /// A fresh board with the same size, mine count and automation.
    pub fn restart(&mut self) {
        let board = Board::random(self.config.dims, self.config.mines, &mut self.rng);
        let rng = self.rng.clone();
        let revision = self.revision + 1;
        let mut events = std::mem::take(&mut self.events);
        *self = Self::with_board(board, self.automation, rng);
        events.append(&mut self.events);
        self.events = events;
        self.revision = revision;
    }

    /// Everything that happened since the last call, oldest first.
    pub fn take_events(&mut self) -> Vec<Timed> {
        std::mem::take(&mut self.events)
    }

    fn record(&mut self, event: Event) {
        let at = self.now();
        self.events.push(Timed { at, event });
    }

    pub fn dims(&self) -> Dims {
        self.config.dims
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn automation(&self) -> Automation {
        self.automation
    }

    /// Can go negative when the player places too many flags.
    pub fn mines_left(&self) -> i64 {
        self.config.mines as i64 - self.board.flag_count() as i64
    }

    pub fn help_granted(&self) -> bool {
        self.help_granted
    }

    /// Automation is still rippling out.
    pub fn is_busy(&self) -> bool {
        !self.schedule.is_empty()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn cell(&self, pos: Pos) -> CellInfo {
        let i = self.dims().index(pos);
        CellInfo {
            look: self.look(pos),
            missed: self.missed[i],
            opened_by_help: self.opened_by_help[i],
        }
    }

    fn look(&self, pos: Pos) -> Look {
        let lost = self.status == Status::Lost;
        let mine = self.board.is_mine(pos);
        match self.board.view(pos) {
            View::Revealed(n) => Look::Number(n),
            _ if self.exploded == Some(pos) => Look::Exploded,
            View::Flagged if lost && !mine => Look::WrongFlag,
            View::Flagged => Look::Flag,
            View::Hidden if lost && mine => Look::Mine,
            View::Hidden => Look::Covered,
        }
    }

    pub fn set_automation(&mut self, automation: Automation) {
        self.record(Event::PlayerSetAutomation(automation));
        let switched_on = automation == Automation::LocalConstraints && self.automation != automation;
        self.automation = automation;
        if switched_on && self.status == Status::Playing {
            self.resolve_everywhere();
        }
    }

    /// Runs automation due by `now`, milliseconds on the game's own clock.
    pub fn advance_to(&mut self, now: Millis) {
        while let Some(task) = self.schedule.pop_due(now) {
            self.run(task);
        }
    }

    /// Runs all pending automation to completion.
    pub fn settle(&mut self) {
        while let Some(due) = self.schedule.next_due() {
            self.advance_to(due);
        }
    }

    pub fn now(&self) -> Millis {
        self.schedule.now()
    }

    /// Primary click: open a covered square, or open the neighbours of a
    /// number whose flags are all placed.
    pub fn click(&mut self, pos: Pos) {
        self.record(Event::PlayerClicked(pos));
        if self.status != Status::Playing {
            return;
        }
        match self.board.view(pos) {
            View::Hidden => self.reveal(pos, self.help_granted),
            View::Revealed(n) if self.board.flagged_neighbours(pos) == usize::from(n) => {
                self.schedule_around(pos, Task::Reveal)
            }
            _ => {}
        }
    }

    pub fn toggle_flag(&mut self, pos: Pos) {
        self.record(Event::PlayerToggledFlag(pos));
        let i = self.dims().index(pos);
        if self.status != Status::Playing || self.opened_by_help[i] {
            return;
        }
        let flagged = match self.board.view(pos) {
            View::Hidden => true,
            View::Flagged => false,
            View::Revealed(_) => return,
        };
        self.set_flag(pos, flagged);
    }

    fn set_flag(&mut self, pos: Pos, flagged: bool) {
        self.board.set_flag(pos, flagged);
        self.revision += 1;
        self.record(if flagged {
            Event::Flagged(pos)
        } else {
            Event::Unflagged(pos)
        });
        self.after_flag(pos);
    }

    fn run(&mut self, task: Task) {
        if self.status != Status::Playing {
            return;
        }
        match task {
            Task::Reveal(pos) => self.reveal(pos, false),
            Task::RevealAroundIfFlagged(pos) => self.reveal_around_if_flagged(pos),
            Task::FlagAroundIfCovered(pos) => self.flag_around_if_covered(pos),
        }
    }

    fn reveal(&mut self, pos: Pos, with_help: bool) {
        if self.board.view(pos) != View::Hidden {
            return;
        }
        let first = std::mem::take(&mut self.first_reveal);
        if first && self.board.is_mine(pos) {
            let to = self.board.relocate_mine(pos, &mut self.rng);
            self.record(Event::MineMoved { from: pos, to });
        }
        // The first square is safe anyway, so granted help is kept for later.
        let with_help = with_help && !first;
        if with_help {
            self.help_granted = false;
            let i = self.dims().index(pos);
            self.opened_by_help[i] = true;
            self.record(Event::HelpUsed(pos));
        }
        if self.board.is_mine(pos) {
            if with_help {
                self.set_flag(pos, true);
            } else {
                self.lose(Some(pos));
            }
            return;
        }

        let n = self.board.reveal(pos);
        self.revision += 1;
        self.record(Event::Revealed { pos, number: n });
        if self.board.all_safe_revealed() {
            self.win();
        } else if n == 0 {
            self.schedule_around(pos, Task::Reveal);
        } else if self.automation == Automation::LocalConstraints {
            self.flag_around_if_covered(pos);
            self.schedule_around(pos, Task::FlagAroundIfCovered);
            self.reveal_around_if_flagged(pos);
            self.schedule_around(pos, Task::RevealAroundIfFlagged);
        }
    }

    fn after_flag(&mut self, pos: Pos) {
        if self.automation == Automation::LocalConstraints {
            self.schedule_around(pos, Task::RevealAroundIfFlagged);
        }
    }

    fn reveal_around_if_flagged(&mut self, pos: Pos) {
        if let View::Revealed(n) = self.board.view(pos)
            && self.board.flagged_neighbours(pos) == usize::from(n)
        {
            self.schedule_around(pos, Task::Reveal);
        }
    }

    fn flag_around_if_covered(&mut self, pos: Pos) {
        let View::Revealed(n) = self.board.view(pos) else {
            return;
        };
        if self.board.covered_neighbours(pos) != usize::from(n) {
            return;
        }
        let hidden: Vec<Pos> = self.board.hidden_neighbours(pos).collect();
        for q in hidden {
            self.set_flag(q, true);
        }
    }

    fn schedule_around(&mut self, pos: Pos, task: fn(Pos) -> Task) {
        let dims = self.dims();
        for q in dims.orthogonal(pos) {
            self.schedule.after(ORTHOGONAL_DELAY, task(q));
        }
        for q in dims.diagonal(pos) {
            self.schedule.after(DIAGONAL_DELAY, task(q));
        }
    }

    /// Applies the local rules to every number, as if each had just been
    /// revealed.
    fn resolve_everywhere(&mut self) {
        let dims = self.dims();
        for pos in dims.positions() {
            if matches!(self.board.view(pos), View::Revealed(_)) {
                self.schedule.after(0, Task::FlagAroundIfCovered(pos));
                self.schedule.after(0, Task::RevealAroundIfFlagged(pos));
            }
        }
    }

    fn win(&mut self) {
        self.status = Status::Won;
        self.end();
        let dims = self.dims();
        for pos in dims.positions() {
            if self.board.view(pos) == View::Hidden {
                self.board.set_flag(pos, true);
            }
        }
        self.record(Event::Won);
    }

    fn lose(&mut self, exploded: Option<Pos>) {
        self.status = Status::Lost;
        self.exploded = exploded;
        self.end();
        self.record(Event::Lost { exploded });
    }

    fn end(&mut self) {
        self.schedule.clear();
        self.help_granted = false;
        self.revision += 1;
    }
}
