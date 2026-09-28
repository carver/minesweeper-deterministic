//! The minefield and what the player can see of it. No rules or timing.

use crate::grid::{Dims, Pos};
use crate::rng::Rng;
use crate::solver::{Known, Puzzle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Hidden,
    Flagged,
    Revealed(u8),
}

#[derive(Clone, Debug)]
pub struct Board {
    dims: Dims,
    mines: Vec<bool>,
    view: Vec<View>,
}

impl Board {
    /// Places `mine_count` mines uniformly at random.
    pub fn random(dims: Dims, mine_count: usize, rng: &mut Rng) -> Self {
        assert!(mine_count < dims.area(), "need at least one safe square");
        let mut board = Self::from_mines(dims, &[]);
        for _ in 0..mine_count {
            board.place_random_mine(rng);
        }
        board
    }

    pub fn from_mines(dims: Dims, mines: &[Pos]) -> Self {
        let mut field = vec![false; dims.area()];
        for &p in mines {
            field[dims.index(p)] = true;
        }
        Self {
            dims,
            mines: field,
            view: vec![View::Hidden; dims.area()],
        }
    }

    pub fn dims(&self) -> Dims {
        self.dims
    }

    pub fn view(&self, pos: Pos) -> View {
        self.view[self.dims.index(pos)]
    }

    pub fn is_mine(&self, pos: Pos) -> bool {
        self.mines[self.dims.index(pos)]
    }

    pub fn mine_count(&self) -> usize {
        self.mines.iter().filter(|&&m| m).count()
    }

    pub fn flag_count(&self) -> usize {
        self.view.iter().filter(|&&v| v == View::Flagged).count()
    }

    pub fn revealed_count(&self) -> usize {
        self.view
            .iter()
            .filter(|v| matches!(v, View::Revealed(_)))
            .count()
    }

    pub fn all_safe_revealed(&self) -> bool {
        self.revealed_count() == self.dims.area() - self.mine_count()
    }

    pub fn adjacent_mines(&self, pos: Pos) -> u8 {
        self.dims.neighbours(pos).filter(|&q| self.is_mine(q)).count() as u8
    }

    /// Unrevealed neighbours, flagged or not.
    pub fn covered_neighbours(&self, pos: Pos) -> usize {
        self.dims
            .neighbours(pos)
            .filter(|&q| !matches!(self.view(q), View::Revealed(_)))
            .count()
    }

    pub fn flagged_neighbours(&self, pos: Pos) -> usize {
        self.dims
            .neighbours(pos)
            .filter(|&q| self.view(q) == View::Flagged)
            .count()
    }

    pub fn hidden_neighbours(&self, pos: Pos) -> impl Iterator<Item = Pos> + '_ {
        self.dims
            .neighbours(pos)
            .filter(|&q| self.view(q) == View::Hidden)
    }

    /// Reveals a safe square and returns its number.
    pub fn reveal(&mut self, pos: Pos) -> u8 {
        debug_assert!(!self.is_mine(pos));
        let n = self.adjacent_mines(pos);
        self.view[self.dims.index(pos)] = View::Revealed(n);
        n
    }

    pub fn set_flag(&mut self, pos: Pos, flagged: bool) {
        let i = self.dims.index(pos);
        debug_assert!(!matches!(self.view[i], View::Revealed(_)));
        self.view[i] = if flagged { View::Flagged } else { View::Hidden };
    }

    /// Moves the mine at `pos` to a random mine-free square other than `pos`.
    pub fn relocate_mine(&mut self, pos: Pos, rng: &mut Rng) {
        let i = self.dims.index(pos);
        debug_assert!(self.mines[i]);
        self.place_random_mine_excluding(rng, Some(i));
        self.mines[i] = false;
    }

    fn place_random_mine(&mut self, rng: &mut Rng) {
        self.place_random_mine_excluding(rng, None);
    }

    fn place_random_mine_excluding(&mut self, rng: &mut Rng, excluded: Option<usize>) {
        let free: Vec<usize> = (0..self.mines.len())
            .filter(|&i| !self.mines[i] && Some(i) != excluded)
            .collect();
        let chosen = free[rng.below(free.len())];
        self.mines[chosen] = true;
    }

    /// Numbers with more flags around them than their value.
    pub fn overflagged_numbers(&self) -> Vec<Pos> {
        self.numbers()
            .filter(|&(p, n)| self.flagged_neighbours(p) > usize::from(n))
            .map(|(p, _)| p)
            .collect()
    }

    /// Squares that a single number settles: its covered neighbours are
    /// exactly its mines, or its flags already account for all of them.
    pub fn locally_forced(&self) -> Vec<Pos> {
        let mut forced = Vec::new();
        for (p, n) in self.numbers() {
            let n = usize::from(n);
            if self.covered_neighbours(p) == n || self.flagged_neighbours(p) == n {
                forced.extend(self.hidden_neighbours(p));
            }
        }
        forced.sort_by_key(|p| (p.y, p.x));
        forced.dedup();
        forced
    }

    /// The board as the player sees it, flags taken at face value.
    pub fn puzzle(&self) -> Puzzle {
        let cells = self
            .view
            .iter()
            .map(|v| match *v {
                View::Hidden => Known::Unknown,
                View::Flagged => Known::Flag,
                View::Revealed(n) => Known::Number(n),
            })
            .collect();
        Puzzle {
            dims: self.dims,
            cells,
            mines: self.mine_count(),
        }
    }

    pub fn wrong_flags(&self) -> Vec<Pos> {
        self.dims
            .positions()
            .filter(|&p| self.view(p) == View::Flagged && !self.is_mine(p))
            .collect()
    }

    fn numbers(&self) -> impl Iterator<Item = (Pos, u8)> + '_ {
        self.dims.positions().filter_map(|p| match self.view(p) {
            View::Revealed(n) => Some((p, n)),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_board_has_requested_mines() {
        let board = Board::random(Dims::new(30, 16), 99, &mut Rng::seeded(3));
        assert_eq!(board.mine_count(), 99);
    }

    #[test]
    fn relocation_keeps_count_and_clears_square() {
        let dims = Dims::new(3, 3);
        let origin = Pos::new(1, 1);
        for seed in 0..20 {
            let mut board = Board::from_mines(dims, &[origin, Pos::new(0, 0)]);
            board.relocate_mine(origin, &mut Rng::seeded(seed));
            assert!(!board.is_mine(origin));
            assert!(board.is_mine(Pos::new(0, 0)));
            assert_eq!(board.mine_count(), 2);
        }
    }

    #[test]
    fn locally_forced_finds_both_rules() {
        // Mine at (0,0). Reveal (1,1)=1 with (0,0) flagged: its other hidden
        // neighbours are safe.
        let dims = Dims::new(3, 3);
        let mut board = Board::from_mines(dims, &[Pos::new(0, 0)]);
        board.reveal(Pos::new(1, 1));
        assert!(board.locally_forced().is_empty());
        board.set_flag(Pos::new(0, 0), true);
        assert_eq!(board.locally_forced().len(), 7);
    }

    #[test]
    fn locally_forced_ignores_settled_numbers() {
        let dims = Dims::new(2, 1);
        let mut board = Board::from_mines(dims, &[Pos::new(0, 0)]);
        board.reveal(Pos::new(1, 0));
        assert_eq!(board.locally_forced(), vec![Pos::new(0, 0)]);
        board.set_flag(Pos::new(0, 0), true);
        assert!(board.locally_forced().is_empty());
    }
}
