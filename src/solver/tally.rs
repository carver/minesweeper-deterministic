//! Counts, for one component, which mine totals are possible and which
//! values each cell can take under each total.
//!
//! Cells are assigned in order. The only thing the rest of the assignment
//! cares about is how many mines each half-assigned number has so far, so
//! partial layouts that agree on that collapse into one state. A forward
//! pass finds reachable states, a backward pass finds which of them can be
//! completed, and together they give each cell's possible values without
//! listing layouts one by one.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use super::Cancelled;
use super::frontier::Component;

pub struct Tally {
    /// Mine totals the component can hold.
    totals: Counts,
    /// Per cell, indexed by `usize::from(is_mine)`: totals of the layouts in
    /// which the cell takes that value.
    cell_totals: Vec<[Counts; 2]>,
}

impl Tally {
    pub fn of(component: &Component, max_mines: usize, cancel: &AtomicBool) -> Result<Self, Cancelled> {
        let steps = Steps::new(component);
        let n = component.cells.len();
        let empty = || Counts::new(max_mines);

        let mut forward: Vec<HashMap<State, Counts>> = vec![HashMap::new(); n + 1];
        forward[0].insert(State::new(), Counts::only(0, max_mines));
        for i in 0..n {
            check(cancel)?;
            let (done, rest) = forward.split_at_mut(i + 1);
            for (state, counts) in &done[i] {
                for mine in [false, true] {
                    if let Some(next) = steps.take(i, state, mine) {
                        rest[0]
                            .entry(next)
                            .or_insert_with(empty)
                            .add_shifted(counts, usize::from(mine));
                    }
                }
            }
        }

        let mut backward: Vec<HashMap<State, Counts>> = vec![HashMap::new(); n + 1];
        backward[n].insert(State::new(), Counts::only(0, max_mines));
        let mut cell_totals = vec![[empty(), empty()]; n];
        for i in (0..n).rev() {
            check(cancel)?;
            let (head, tail) = backward.split_at_mut(i + 1);
            for (state, before) in &forward[i] {
                let mut completions = empty();
                for mine in [false, true] {
                    let Some(after) = steps.take(i, state, mine).and_then(|next| tail[0].get(&next)) else {
                        continue;
                    };
                    let shift = usize::from(mine);
                    completions.add_shifted(after, shift);
                    cell_totals[i][shift].add_sums(before, after, shift);
                }
                if !completions.is_empty() {
                    head[i].insert(state.clone(), completions);
                }
            }
        }

        let totals = forward[n].remove(&State::new()).unwrap_or_else(empty);
        Ok(Self { totals, cell_totals })
    }

    pub fn is_empty(&self) -> bool {
        self.totals.is_empty()
    }

    pub fn counts(&self) -> Vec<usize> {
        self.totals.iter().collect()
    }

    /// Whether the cell can be a mine, and whether it can be safe, in some
    /// layout whose total is in `allowed`.
    pub fn possibilities(&self, cell: usize, allowed: &[usize]) -> (bool, bool) {
        let [safe, mine] = &self.cell_totals[cell];
        let any = |c: &Counts| allowed.iter().any(|&k| c.contains(k));
        (any(mine), any(safe))
    }
}

fn check(cancel: &AtomicBool) -> Result<(), Cancelled> {
    if cancel.load(Ordering::Relaxed) {
        Err(Cancelled)
    } else {
        Ok(())
    }
}

/// Mines placed so far for each open constraint, in `Steps::open` order.
type State = Vec<u8>;

struct Steps<'a> {
    component: &'a Component,
    /// `open[i]`: sorted constraints with cells both before `i` and at or
    /// after it.
    open: Vec<Vec<usize>>,
    /// Constraints each cell belongs to.
    touching: Vec<Vec<usize>>,
}

impl<'a> Steps<'a> {
    fn new(component: &'a Component) -> Self {
        let n = component.cells.len();
        let mut open = vec![Vec::new(); n + 1];
        let mut touching = vec![Vec::new(); n];
        for (c, constraint) in component.constraints.iter().enumerate() {
            let first = constraint.cells[0];
            let last = *constraint.cells.last().expect("constraints are non-empty");
            for layer in &mut open[first + 1..=last] {
                layer.push(c);
            }
            for &cell in &constraint.cells {
                touching[cell].push(c);
            }
        }
        Self {
            component,
            open,
            touching,
        }
    }

    /// The state after giving cell `i` the value `mine`, or `None` if some
    /// number it touches can no longer be satisfied.
    fn take(&self, i: usize, state: &State, mine: bool) -> Option<State> {
        let placed_before = |c: usize| {
            self.open[i]
                .binary_search(&c)
                .map_or(0, |k| usize::from(state[k]))
        };
        for &c in &self.touching[i] {
            let constraint = &self.component.constraints[c];
            let placed = placed_before(c) + usize::from(mine);
            let still_open = constraint.cells.len() - constraint.cells.partition_point(|&cell| cell <= i);
            if placed > constraint.need || placed + still_open < constraint.need {
                return None;
            }
        }
        let next = self.open[i + 1]
            .iter()
            .map(|&c| {
                let here = mine && self.touching[i].contains(&c);
                (placed_before(c) + usize::from(here)) as u8
            })
            .collect();
        Some(next)
    }
}

/// A set of mine totals from `0..=max`. Larger totals are dropped: they
/// exceed the mines left.
#[derive(Clone, Debug)]
struct Counts {
    present: Vec<bool>,
}

impl Counts {
    fn new(max: usize) -> Self {
        Self {
            present: vec![false; max + 1],
        }
    }

    fn only(k: usize, max: usize) -> Self {
        let mut counts = Self::new(max);
        counts.insert(k);
        counts
    }

    fn insert(&mut self, k: usize) {
        if let Some(slot) = self.present.get_mut(k) {
            *slot = true;
        }
    }

    fn contains(&self, k: usize) -> bool {
        self.present.get(k).copied().unwrap_or(false)
    }

    fn is_empty(&self) -> bool {
        !self.present.contains(&true)
    }

    fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.present.len()).filter(|&k| self.present[k])
    }

    /// Adds `k + shift` for every `k` in `other`.
    fn add_shifted(&mut self, other: &Counts, shift: usize) {
        for k in other.iter() {
            self.insert(k + shift);
        }
    }

    /// Adds `a + b + shift` for every `a` in `left` and `b` in `right`.
    fn add_sums(&mut self, left: &Counts, right: &Counts, shift: usize) {
        for a in left.iter() {
            for b in right.iter() {
                self.insert(a + b + shift);
            }
        }
    }
}
