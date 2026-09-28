//! Board geometry: positions, bounds and neighbourhoods.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

impl Pos {
    pub const fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

/// Neighbour offsets, orthogonal first. The ripple animation relies on this
/// split: orthogonal neighbours are reached sooner than diagonal ones.
pub const ORTHOGONAL: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
pub const DIAGONAL: [(isize, isize); 4] = [(-1, -1), (1, 1), (1, -1), (-1, 1)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dims {
    pub width: usize,
    pub height: usize,
}

impl Dims {
    pub const fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub const fn area(self) -> usize {
        self.width * self.height
    }

    pub fn index(self, pos: Pos) -> usize {
        debug_assert!(self.contains(pos));
        pos.y * self.width + pos.x
    }

    pub fn pos(self, index: usize) -> Pos {
        Pos::new(index % self.width, index / self.width)
    }

    pub fn contains(self, pos: Pos) -> bool {
        pos.x < self.width && pos.y < self.height
    }

    pub fn offset(self, pos: Pos, (dx, dy): (isize, isize)) -> Option<Pos> {
        let x = pos.x.checked_add_signed(dx)?;
        let y = pos.y.checked_add_signed(dy)?;
        let moved = Pos::new(x, y);
        self.contains(moved).then_some(moved)
    }

    pub fn orthogonal(self, pos: Pos) -> impl Iterator<Item = Pos> {
        ORTHOGONAL.into_iter().filter_map(move |d| self.offset(pos, d))
    }

    pub fn diagonal(self, pos: Pos) -> impl Iterator<Item = Pos> {
        DIAGONAL.into_iter().filter_map(move |d| self.offset(pos, d))
    }

    pub fn neighbours(self, pos: Pos) -> impl Iterator<Item = Pos> {
        self.orthogonal(pos).chain(self.diagonal(pos))
    }

    pub fn positions(self) -> impl Iterator<Item = Pos> {
        (0..self.area()).map(move |i| self.pos(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corner_has_three_neighbours() {
        let dims = Dims::new(3, 3);
        let mut n: Vec<_> = dims.neighbours(Pos::new(0, 0)).collect();
        n.sort();
        assert_eq!(n, vec![Pos::new(0, 1), Pos::new(1, 0), Pos::new(1, 1)]);
    }

    #[test]
    fn centre_has_eight_neighbours_orthogonal_first() {
        let dims = Dims::new(3, 3);
        let n: Vec<_> = dims.neighbours(Pos::new(1, 1)).collect();
        assert_eq!(n.len(), 8);
        assert!(n[..4].iter().all(|p| p.x == 1 || p.y == 1));
    }

    #[test]
    fn index_round_trips() {
        let dims = Dims::new(5, 4);
        for i in 0..dims.area() {
            assert_eq!(dims.index(dims.pos(i)), i);
        }
    }
}
