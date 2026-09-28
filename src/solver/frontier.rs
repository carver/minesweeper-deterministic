//! Turns a board into constraints over the unknown squares that touch
//! numbers, split into independent components.

use crate::grid::{Dims, Pos};

use super::{Known, Puzzle};

/// A number's demand on the unknown squares around it.
#[derive(Clone, Debug)]
pub struct Constraint {
    pub cells: Vec<usize>,
    pub need: usize,
}

pub struct Frontier {
    /// Frontier id -> board position.
    pub positions: Vec<Pos>,
    /// Board index -> frontier id.
    ids: Vec<Option<usize>>,
    constraints: Vec<Constraint>,
    pub unconstrained: usize,
}

impl Frontier {
    /// `None` when some number already has too many or too few candidates.
    pub fn build(puzzle: &Puzzle) -> Option<Self> {
        let dims = puzzle.dims;
        let mut frontier = Self {
            positions: Vec::new(),
            ids: vec![None; dims.area()],
            constraints: Vec::new(),
            unconstrained: 0,
        };
        for pos in dims.positions() {
            let Known::Number(n) = puzzle.cells[dims.index(pos)] else {
                continue;
            };
            let mut flags = 0;
            let mut cells = Vec::new();
            for q in dims.neighbours(pos) {
                match puzzle.cells[dims.index(q)] {
                    Known::Flag => flags += 1,
                    Known::Unknown => cells.push(frontier.id_for(dims, q)),
                    Known::Number(_) => {}
                }
            }
            let need = usize::from(n).checked_sub(flags)?;
            if need > cells.len() {
                return None;
            }
            if !cells.is_empty() {
                frontier.constraints.push(Constraint { cells, need });
            }
        }
        let unknown = puzzle.cells.iter().filter(|&&c| c == Known::Unknown).count();
        frontier.unconstrained = unknown - frontier.positions.len();
        Some(frontier)
    }

    fn id_for(&mut self, dims: Dims, pos: Pos) -> usize {
        let slot = &mut self.ids[dims.index(pos)];
        *slot.get_or_insert_with(|| {
            self.positions.push(pos);
            self.positions.len() - 1
        })
    }

    pub fn unconstrained_positions<'a>(&'a self, puzzle: &'a Puzzle) -> impl Iterator<Item = Pos> + 'a {
        let dims = puzzle.dims;
        dims.positions().filter(move |&p| {
            let i = dims.index(p);
            puzzle.cells[i] == Known::Unknown && self.ids[i].is_none()
        })
    }

    /// Splits the frontier into groups of squares linked by shared numbers.
    pub fn components(&self) -> Vec<Component> {
        let links = Links::new(self.positions.len(), &self.constraints);
        let mut seen = vec![false; self.positions.len()];
        let mut components = Vec::new();
        for start in 0..self.positions.len() {
            if seen[start] {
                continue;
            }
            // Starting from the far end of a component keeps the set of
            // half-assigned numbers small while the tally walks through it.
            let reached = links.breadth_first(start);
            let far_end = *reached.last().expect("contains start");
            let cells = links.breadth_first(far_end);
            for &c in &cells {
                seen[c] = true;
            }
            components.push(Component::new(cells, &self.constraints, &links));
        }
        components
    }
}

/// Which constraints each frontier cell takes part in.
struct Links<'a> {
    constraints: &'a [Constraint],
    by_cell: Vec<Vec<usize>>,
}

impl<'a> Links<'a> {
    fn new(cells: usize, constraints: &'a [Constraint]) -> Self {
        let mut by_cell = vec![Vec::new(); cells];
        for (c, constraint) in constraints.iter().enumerate() {
            for &cell in &constraint.cells {
                by_cell[cell].push(c);
            }
        }
        Self { constraints, by_cell }
    }

    /// Cells reachable from `start` through shared constraints, nearest first.
    fn breadth_first(&self, start: usize) -> Vec<usize> {
        let mut seen = vec![false; self.by_cell.len()];
        seen[start] = true;
        let mut order = vec![start];
        let mut next = 0;
        while next < order.len() {
            for &c in &self.by_cell[order[next]] {
                for &other in &self.constraints[c].cells {
                    if !std::mem::replace(&mut seen[other], true) {
                        order.push(other);
                    }
                }
            }
            next += 1;
        }
        order
    }
}

/// A self-contained problem with cells renumbered from zero in the order
/// they should be assigned.
pub struct Component {
    /// Local index -> frontier id.
    pub cells: Vec<usize>,
    /// Constraint cells are local indices, sorted.
    pub constraints: Vec<Constraint>,
}

impl Component {
    fn new(cells: Vec<usize>, all: &[Constraint], links: &Links) -> Self {
        let mut local = std::collections::HashMap::new();
        for (i, &cell) in cells.iter().enumerate() {
            local.insert(cell, i);
        }
        let mut constraint_ids: Vec<usize> = cells
            .iter()
            .flat_map(|&c| links.by_cell[c].iter().copied())
            .collect();
        constraint_ids.sort_unstable();
        constraint_ids.dedup();
        let constraints = constraint_ids
            .into_iter()
            .map(|c| {
                let mut cells: Vec<usize> = all[c].cells.iter().map(|f| local[f]).collect();
                cells.sort_unstable();
                Constraint {
                    cells,
                    need: all[c].need,
                }
            })
            .collect();
        Self { cells, constraints }
    }
}
