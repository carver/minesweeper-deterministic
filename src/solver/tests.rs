use std::sync::atomic::AtomicBool;

use proptest::prelude::*;

use super::*;

/// Rows separated by `/`: `.` unknown, `F` flag, digits revealed.
fn puzzle(rows: &str, mines: usize) -> Puzzle {
    let rows: Vec<&str> = rows.split('/').collect();
    let dims = Dims::new(rows[0].len(), rows.len());
    let cells = rows
        .iter()
        .flat_map(|r| r.chars())
        .map(|c| match c {
            '.' => Known::Unknown,
            'F' => Known::Flag,
            d => Known::Number(d.to_digit(10).expect("digit") as u8),
        })
        .collect();
    Puzzle { dims, cells, mines }
}

fn run(p: &Puzzle) -> Solution {
    solve(p, &AtomicBool::new(false)).expect("not cancelled")
}

fn d(x: usize, y: usize, verdict: Verdict) -> Deduction {
    Deduction {
        pos: Pos::new(x, y),
        verdict,
    }
}

#[test]
fn article_case_all_mines_known_frees_unconstrained_square() {
    let p = puzzle("0000/0122/02FF/02F.", 3);
    assert_eq!(run(&p), Solution::Deductions(vec![d(3, 3, Verdict::Safe)]));
}

#[test]
fn article_case_last_mine_confined_to_corner_pair() {
    let p = puzzle("..F20000/13F20122/011102FF/000002F.", 6);
    assert_eq!(run(&p), Solution::Deductions(vec![d(3 + 4, 3, Verdict::Safe)]));
}

#[test]
fn fifty_fifty_has_no_deductions() {
    let p = puzzle("..", 1);
    assert_eq!(run(&p), Solution::Deductions(vec![]));
}

#[test]
fn zero_next_to_pair_forces_the_mine() {
    let p = puzzle("...0/1110", 1);
    assert_eq!(
        run(&p),
        Solution::Deductions(vec![
            d(0, 0, Verdict::Safe),
            d(1, 0, Verdict::Mine),
            d(2, 0, Verdict::Safe),
        ])
    );
}

#[test]
fn over_flagged_number_is_inconsistent() {
    let p = puzzle("FF/1.", 3);
    assert_eq!(run(&p), Solution::Inconsistent);
}

#[test]
fn flag_can_contradict_a_number_it_does_not_over_fill() {
    let ok = puzzle("1F1/...", 1);
    assert_eq!(
        run(&ok),
        Solution::Deductions(vec![
            d(0, 1, Verdict::Safe),
            d(1, 1, Verdict::Safe),
            d(2, 1, Verdict::Safe)
        ])
    );
    // Each 2 would need a second mine below, but the count allows only one.
    let bad = puzzle("2F2/...", 1);
    assert_eq!(run(&bad), Solution::Inconsistent);
}

#[test]
fn more_flags_than_mines_is_inconsistent() {
    let p = puzzle("FF.", 1);
    assert_eq!(run(&p), Solution::Inconsistent);
}

#[test]
fn mine_count_links_separate_components() {
    // Two separate pairs each need at least one mine; with only two mines,
    // the rest of the board is safe.
    let p = puzzle(".1001.../.1001...", 2);
    assert_eq!(
        run(&p),
        Solution::Deductions(vec![
            d(6, 0, Verdict::Safe),
            d(7, 0, Verdict::Safe),
            d(6, 1, Verdict::Safe),
            d(7, 1, Verdict::Safe),
        ])
    );
}

#[test]
fn cancelled_search_reports_cancelled() {
    let p = puzzle("..../1111", 2);
    assert_eq!(solve(&p, &AtomicBool::new(true)), Err(Cancelled));
}

/// Tries every mine layout. The oracle for the property test.
fn brute_force(p: &Puzzle) -> Solution {
    let unknown: Vec<usize> = (0..p.cells.len())
        .filter(|&i| p.cells[i] == Known::Unknown)
        .collect();
    let flags = p.cells.iter().filter(|&&c| c == Known::Flag).count();
    let Some(remaining) = p.mines.checked_sub(flags) else {
        return Solution::Inconsistent;
    };
    let mut can_mine = vec![false; unknown.len()];
    let mut can_safe = vec![false; unknown.len()];
    let mut any = false;
    for mask in 0u32..(1 << unknown.len()) {
        if mask.count_ones() as usize != remaining {
            continue;
        }
        let mut mine = p.cells.iter().map(|&c| c == Known::Flag).collect::<Vec<_>>();
        for (bit, &i) in unknown.iter().enumerate() {
            mine[i] = mask & (1 << bit) != 0;
        }
        let consistent = p.dims.positions().all(|pos| match p.cells[p.dims.index(pos)] {
            Known::Number(n) => {
                p.dims.neighbours(pos).filter(|&q| mine[p.dims.index(q)]).count() == usize::from(n)
            }
            _ => true,
        });
        if !consistent {
            continue;
        }
        any = true;
        for (bit, &i) in unknown.iter().enumerate() {
            if mine[i] {
                can_mine[bit] = true;
            } else {
                can_safe[bit] = true;
            }
        }
    }
    if !any {
        return Solution::Inconsistent;
    }
    let deductions = unknown
        .iter()
        .enumerate()
        .filter_map(|(bit, &i)| {
            let verdict = match (can_mine[bit], can_safe[bit]) {
                (true, false) => Verdict::Mine,
                (false, true) => Verdict::Safe,
                _ => return None,
            };
            Some(Deduction {
                pos: p.dims.pos(i),
                verdict,
            })
        })
        .collect();
    Solution::Deductions(deductions)
}

/// A random real board, partly revealed, with some flags that may be wrong.
fn arbitrary_puzzle() -> impl Strategy<Value = Puzzle> {
    (2usize..=5, 2usize..=4)
        .prop_flat_map(|(w, h)| {
            let n = w * h;
            (
                Just(Dims::new(w, h)),
                proptest::collection::vec(any::<bool>(), n),
                proptest::collection::vec(0u8..10, n),
            )
        })
        .prop_map(|(dims, mines, roll)| {
            let mine_count = mines.iter().filter(|&&m| m).count();
            let cells = dims
                .positions()
                .map(|pos| {
                    let i = dims.index(pos);
                    let adjacent = dims.neighbours(pos).filter(|&q| mines[dims.index(q)]).count() as u8;
                    match (mines[i], roll[i]) {
                        (_, 0) => Known::Flag,
                        (false, 1..=5) => Known::Number(adjacent),
                        _ => Known::Unknown,
                    }
                })
                .collect();
            Puzzle {
                dims,
                cells,
                mines: mine_count,
            }
        })
        .prop_filter("brute force stays small", |p| {
            p.cells.iter().filter(|&&c| c == Known::Unknown).count() <= 14
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn matches_brute_force(p in arbitrary_puzzle()) {
        prop_assert_eq!(run(&p), brute_force(&p));
    }
}
