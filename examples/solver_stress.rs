//! Times the solver on expert boards with randomly scattered reveals, which
//! make wider frontiers than normal play. `cargo run --release --example
//! solver_stress -- [boards] [reveal-percent]`
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use minesweeper_deterministic::board::Board;
use minesweeper_deterministic::grid::Dims;
use minesweeper_deterministic::rng::Rng;
use minesweeper_deterministic::solver::{self, Known, Puzzle};

fn main() {
    let mut args = std::env::args()
        .skip(1)
        .map(|a| a.parse::<usize>().expect("number"));
    let boards = args.next().unwrap_or(200);
    let percent = args.next().unwrap_or(40);
    let dims = Dims::new(30, 16);
    let mut rng = Rng::seeded(42);
    let mut worst = (0.0, 0);
    for n in 0..boards {
        let mut board = Board::random(dims, 99, &mut rng);
        for p in dims.positions() {
            if !board.is_mine(p) && rng.below(100) < percent {
                board.reveal(p);
            }
        }
        let puzzle: Puzzle = board.puzzle();
        let unknown = puzzle.cells.iter().filter(|&&c| c == Known::Unknown).count();
        let start = Instant::now();
        solver::solve(&puzzle, &AtomicBool::new(false)).expect("not cancelled");
        let secs = start.elapsed().as_secs_f64();
        if secs > worst.0 {
            worst = (secs, n);
            println!("board {n}: {unknown} unknown, {:.1} ms", secs * 1000.0);
        }
    }
    println!("worst {:.1} ms (board {})", worst.0 * 1000.0, worst.1);
}
