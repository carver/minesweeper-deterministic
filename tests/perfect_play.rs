//! A player who never misses a deduction and asks for help only when stuck
//! must always win. This exercises the solver, automation and help together
//! on full-size boards.

use std::sync::atomic::AtomicBool;

use minesweeper_deterministic::game::{Automation, Config, Game, HelpVerdict, Look, Status};
use minesweeper_deterministic::grid::Pos;
use minesweeper_deterministic::rng::Rng;
use minesweeper_deterministic::solver::{self, Solution, Verdict};

fn play(seed: u64, automation: Automation) -> Game {
    let mut game = Game::new(Config::EXPERT, automation, Rng::seeded(seed));
    let mut picker = Rng::seeded(seed ^ 0xABCD);
    let dims = game.dims();
    game.click(Pos::new(dims.width / 2, dims.height / 2));
    game.settle();

    while game.status() == Status::Playing {
        let puzzle = game_puzzle(&game);
        let Ok(Solution::Deductions(found)) = solver::solve(&puzzle, &AtomicBool::new(false)) else {
            panic!("seed {seed}: correct flags cannot be inconsistent");
        };
        if found.is_empty() {
            assert_eq!(
                game.request_help_blocking(),
                Some(HelpVerdict::Granted),
                "seed {seed}"
            );
            let covered: Vec<Pos> = dims
                .positions()
                .filter(|&p| game.cell(p).look == Look::Covered)
                .collect();
            game.click(covered[picker.below(covered.len())]);
        }
        for d in found {
            // Automation may already have handled a square since the solve.
            if game.cell(d.pos).look != Look::Covered {
                continue;
            }
            match d.verdict {
                Verdict::Mine => game.toggle_flag(d.pos),
                Verdict::Safe => game.click(d.pos),
            }
        }
        game.settle();
    }
    game
}

fn game_puzzle(game: &Game) -> solver::Puzzle {
    let dims = game.dims();
    let cells = dims
        .positions()
        .map(|p| match game.cell(p).look {
            Look::Covered => solver::Known::Unknown,
            Look::Flag => solver::Known::Flag,
            Look::Number(n) => solver::Known::Number(n),
            other => panic!("unexpected {other:?} while playing"),
        })
        .collect();
    solver::Puzzle {
        dims,
        cells,
        mines: Config::EXPERT.mines,
    }
}

#[test]
fn perfect_player_always_wins_with_local_automation() {
    for seed in 0..200 {
        assert_eq!(
            play(seed, Automation::LocalConstraints).status(),
            Status::Won,
            "seed {seed}"
        );
    }
}

#[test]
fn perfect_player_always_wins_without_automation() {
    for seed in 1000..1100 {
        assert_eq!(
            play(seed, Automation::ZerosOnly).status(),
            Status::Won,
            "seed {seed}"
        );
    }
}
