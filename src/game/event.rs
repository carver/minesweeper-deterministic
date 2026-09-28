//! What happened during a game, for the journal. Player actions are recorded
//! as they arrive, whether or not they changed anything; everything else is
//! the game's own doing.

use std::fmt;

use super::{Automation, HelpVerdict};
use crate::grid::Pos;
use crate::schedule::Millis;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Mines as rows separated by `/`, `*` for a mine.
    NewGame {
        layout: String,
    },
    PlayerClicked(Pos),
    PlayerToggledFlag(Pos),
    PlayerRequestedHelp,
    PlayerSetAutomation(Automation),
    /// The first square opened held a mine, so the mine moved.
    MineMoved {
        from: Pos,
        to: Pos,
    },
    Revealed {
        pos: Pos,
        number: u8,
    },
    Flagged(Pos),
    Unflagged(Pos),
    HelpDecided {
        verdict: HelpVerdict,
        missed: Vec<Pos>,
    },
    /// The board changed while the solver ran, so its answer was dropped.
    HelpAnswerDiscarded,
    /// Granted help opened this square.
    HelpUsed(Pos),
    Won,
    Lost {
        exploded: Option<Pos>,
    },
}

/// An event and when it happened on the game's clock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timed {
    pub at: Millis,
    pub event: Event,
}

struct At(Pos);

impl fmt::Display for At {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", self.0.x, self.0.y)
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NewGame { layout } => write!(f, "new game, mines {layout}"),
            Self::PlayerClicked(p) => write!(f, "player clicked {}", At(*p)),
            Self::PlayerToggledFlag(p) => write!(f, "player toggled flag {}", At(*p)),
            Self::PlayerRequestedHelp => write!(f, "player requested help"),
            Self::PlayerSetAutomation(a) => write!(f, "player set automation {a:?}"),
            Self::MineMoved { from, to } => {
                write!(f, "first square: mine moved {} -> {}", At(*from), At(*to))
            }
            Self::Revealed { pos, number } => write!(f, "revealed {} = {number}", At(*pos)),
            Self::Flagged(p) => write!(f, "flagged {}", At(*p)),
            Self::Unflagged(p) => write!(f, "unflagged {}", At(*p)),
            Self::HelpDecided { verdict, missed } => {
                write!(f, "help {verdict:?}")?;
                if !missed.is_empty() {
                    let squares: Vec<String> = missed.iter().map(|&p| At(p).to_string()).collect();
                    write!(f, ", missed {}", squares.join(" "))?;
                }
                Ok(())
            }
            Self::HelpAnswerDiscarded => write!(f, "help answer discarded, board changed"),
            Self::HelpUsed(p) => write!(f, "help used on {}", At(*p)),
            Self::Won => write!(f, "won"),
            Self::Lost { exploded: Some(p) } => write!(f, "lost, mine at {}", At(*p)),
            Self::Lost { exploded: None } => write!(f, "lost"),
        }
    }
}
