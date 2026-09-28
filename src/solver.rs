//! Global solver: finds every square whose content follows logically from
//! the visible board and the total mine count.
//!
//! Unknown squares next to a number are "constrained"; the rest are
//! "unconstrained" and interchangeable. Constrained squares split into
//! components that share no number, so each component is enumerated on its
//! own. The components interact only through the total mine count, which is
//! reconciled afterwards with a subset-sum over the per-component mine
//! counts.

mod frontier;
mod tally;

use std::sync::atomic::AtomicBool;

use crate::grid::{Dims, Pos};
use frontier::Frontier;
use tally::Tally;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Known {
    Unknown,
    Flag,
    Number(u8),
}

/// What the player can see. Flags are taken as true mines.
#[derive(Clone, Debug)]
pub struct Puzzle {
    pub dims: Dims,
    pub cells: Vec<Known>,
    pub mines: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Mine,
    Safe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deduction {
    pub pos: Pos,
    pub verdict: Verdict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Solution {
    /// Every forced square not already flagged or revealed, in board order.
    Deductions(Vec<Deduction>),
    /// No mine layout agrees with the board, so some flag is wrong.
    Inconsistent,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Cancelled;

pub fn solve(puzzle: &Puzzle, cancel: &AtomicBool) -> Result<Solution, Cancelled> {
    let Some(frontier) = Frontier::build(puzzle) else {
        return Ok(Solution::Inconsistent);
    };
    let flags = puzzle.cells.iter().filter(|&&c| c == Known::Flag).count();
    let Some(remaining) = puzzle.mines.checked_sub(flags) else {
        return Ok(Solution::Inconsistent);
    };

    let mut tallies = Vec::new();
    for component in frontier.components() {
        let tally = Tally::of(&component, remaining, cancel)?;
        if tally.is_empty() {
            return Ok(Solution::Inconsistent);
        }
        tallies.push((component, tally));
    }

    let min_constrained = remaining.saturating_sub(frontier.unconstrained);
    let counts: Vec<Vec<usize>> = tallies.iter().map(|(_, t)| t.counts()).collect();
    let Some(feasible) = feasible_counts(&counts, min_constrained, remaining) else {
        return Ok(Solution::Inconsistent);
    };

    let mut deductions = Vec::new();
    for ((component, tally), allowed) in tallies.iter().zip(&feasible.per_component) {
        for (local, &cell) in component.cells.iter().enumerate() {
            let (mine, safe) = tally.possibilities(local, allowed);
            let verdict = match (mine, safe) {
                (true, false) => Verdict::Mine,
                (false, true) => Verdict::Safe,
                _ => continue,
            };
            let pos = frontier.positions[cell];
            deductions.push(Deduction { pos, verdict });
        }
    }

    if frontier.unconstrained > 0 {
        let unconstrained_mines = feasible.totals.iter().map(|t| remaining - t);
        let fewest = unconstrained_mines.clone().min().unwrap_or(0);
        let most = unconstrained_mines.max().unwrap_or(0);
        let verdict = if fewest == frontier.unconstrained {
            Some(Verdict::Mine)
        } else if most == 0 {
            Some(Verdict::Safe)
        } else {
            None
        };
        if let Some(verdict) = verdict {
            deductions.extend(
                frontier
                    .unconstrained_positions(puzzle)
                    .map(|pos| Deduction { pos, verdict }),
            );
        }
    }

    deductions.sort_by_key(|d| (d.pos.y, d.pos.x));
    Ok(Solution::Deductions(deductions))
}

struct Feasible {
    /// Mine counts each component can hold in some globally valid layout.
    per_component: Vec<Vec<usize>>,
    /// Possible totals across all constrained squares.
    totals: Vec<usize>,
}

/// Reconciles per-component mine counts with the total: the constrained
/// squares together must hold between `lo` and `hi` mines.
fn feasible_counts(counts: &[Vec<usize>], lo: usize, hi: usize) -> Option<Feasible> {
    let n = counts.len();
    let mut prefix = vec![only_zero(hi)];
    for c in counts {
        let next = add_choices(prefix.last().expect("seeded"), c, hi);
        prefix.push(next);
    }
    let mut suffix = vec![only_zero(hi)];
    for c in counts.iter().rev() {
        let next = add_choices(suffix.last().expect("seeded"), c, hi);
        suffix.push(next);
    }
    suffix.reverse();

    let totals: Vec<usize> = (lo..=hi).filter(|&t| prefix[n][t]).collect();
    if totals.is_empty() {
        return None;
    }
    let per_component = (0..n)
        .map(|i| {
            let others = combine(&prefix[i], &suffix[i + 1], hi);
            counts[i]
                .iter()
                .copied()
                .filter(|&k| (0..=hi).any(|s| others[s] && (lo..=hi).contains(&(s + k))))
                .collect()
        })
        .collect();
    Some(Feasible {
        per_component,
        totals,
    })
}

/// Reachability set containing only zero.
fn only_zero(hi: usize) -> Vec<bool> {
    let mut reach = vec![false; hi + 1];
    reach[0] = true;
    reach
}

fn add_choices(reach: &[bool], choices: &[usize], hi: usize) -> Vec<bool> {
    let mut next = vec![false; hi + 1];
    for s in (0..=hi).filter(|&s| reach[s]) {
        for &k in choices {
            if s + k <= hi {
                next[s + k] = true;
            }
        }
    }
    next
}

fn combine(a: &[bool], b: &[bool], hi: usize) -> Vec<bool> {
    let b_values: Vec<usize> = (0..=hi).filter(|&s| b[s]).collect();
    add_choices(a, &b_values, hi)
}

#[cfg(test)]
mod tests;
