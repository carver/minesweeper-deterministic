//! "Request help": granted only when nothing on the board can be deduced.
//! Asking while a deduction exists loses the game and marks what was missed.
//!
//! The cheap local checks run immediately. The global solver can take a
//! while, so it is handed back to the caller as a job to run elsewhere.

use std::sync::atomic::AtomicBool;

use super::{Event, Game, Status};
use crate::grid::Pos;
use crate::solver::{self, Puzzle, Solution};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpVerdict {
    /// The next square the player opens is opened safely.
    Granted,
    /// The player missed something. The game is lost.
    Denied,
}

#[derive(Debug)]
pub enum HelpStart {
    /// The game is over or automation is still running.
    Unavailable,
    Decided(HelpVerdict),
    /// Run [`solver::solve`] on the job's puzzle, then call
    /// [`Game::conclude_help`].
    NeedsSolver(SolverJob),
}

#[derive(Clone, Debug)]
pub struct SolverJob {
    pub puzzle: Puzzle,
    pub revision: u64,
}

impl Game {
    pub fn request_help(&mut self) -> HelpStart {
        self.record(Event::PlayerRequestedHelp);
        if self.status != Status::Playing || self.is_busy() {
            return HelpStart::Unavailable;
        }
        let overflagged = self.board.overflagged_numbers();
        if !overflagged.is_empty() {
            return HelpStart::Decided(self.deny(&overflagged));
        }
        let forced = self.board.locally_forced();
        if !forced.is_empty() {
            return HelpStart::Decided(self.deny(&forced));
        }
        HelpStart::NeedsSolver(SolverJob {
            puzzle: self.board.puzzle(),
            revision: self.revision,
        })
    }

    /// Applies the solver's answer. Returns `None` if the board changed
    /// since the job was made, in which case the answer is discarded.
    pub fn conclude_help(&mut self, job: &SolverJob, solution: Solution) -> Option<HelpVerdict> {
        if job.revision != self.revision || self.status != Status::Playing {
            self.record(Event::HelpAnswerDiscarded);
            return None;
        }
        let verdict = match solution {
            Solution::Inconsistent => self.deny(&self.board.wrong_flags()),
            Solution::Deductions(found) if found.is_empty() => {
                self.help_granted = true;
                self.record(Event::HelpDecided {
                    verdict: HelpVerdict::Granted,
                    missed: Vec::new(),
                });
                HelpVerdict::Granted
            }
            Solution::Deductions(found) => {
                let missed: Vec<Pos> = found.iter().map(|d| d.pos).collect();
                self.deny(&missed)
            }
        };
        Some(verdict)
    }

    /// Notes that a solver job was dropped before it answered.
    pub fn abandon_help(&mut self) {
        self.record(Event::HelpAnswerDiscarded);
    }

    /// Runs the whole help check on the calling thread.
    pub fn request_help_blocking(&mut self) -> Option<HelpVerdict> {
        match self.request_help() {
            HelpStart::Unavailable => None,
            HelpStart::Decided(verdict) => Some(verdict),
            HelpStart::NeedsSolver(job) => {
                let solution = solver::solve(&job.puzzle, &AtomicBool::new(false)).ok()?;
                self.conclude_help(&job, solution)
            }
        }
    }

    fn deny(&mut self, missed: &[Pos]) -> HelpVerdict {
        let dims = self.dims();
        for &pos in missed {
            self.missed[dims.index(pos)] = true;
        }
        self.record(Event::HelpDecided {
            verdict: HelpVerdict::Denied,
            missed: missed.to_vec(),
        });
        self.lose(None);
        HelpVerdict::Denied
    }
}
