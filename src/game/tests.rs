use proptest::prelude::*;

use super::*;
use crate::rng::Rng;
use crate::solver::Solution;

/// See [`Board::from_layout`].
fn game(rows: &str, automation: Automation) -> Game {
    let board = Board::from_layout(rows).expect("valid layout");
    Game::with_board(board, automation, Rng::seeded(0))
}

/// Two 1s over a pair of squares, one of them a mine: a true guess.
const GUESS: &str = "../../*.";

fn p(x: usize, y: usize) -> Pos {
    Pos::new(x, y)
}

fn look(g: &Game, x: usize, y: usize) -> Look {
    g.cell(p(x, y)).look
}

/// Clicks and lets automation finish.
fn click(g: &mut Game, x: usize, y: usize) {
    g.click(p(x, y));
    g.settle();
}

#[test]
fn zero_flood_ripples_outwards() {
    let mut g = game("..../..../...*", Automation::ZerosOnly);
    g.click(p(0, 0));
    assert_eq!(look(&g, 0, 0), Look::Number(0));
    assert_eq!(look(&g, 1, 0), Look::Covered);
    g.advance_to(ORTHOGONAL_DELAY);
    assert_eq!(look(&g, 1, 0), Look::Number(0));
    assert_eq!(look(&g, 1, 1), Look::Covered);
    g.advance_to(DIAGONAL_DELAY);
    assert_eq!(look(&g, 1, 1), Look::Number(0));
    g.settle();
    assert_eq!(look(&g, 3, 1), Look::Number(1));
    assert_eq!(g.status(), Status::Won);
    assert_eq!(look(&g, 3, 2), Look::Flag);
}

#[test]
fn opening_a_mine_loses_and_shows_the_board() {
    let mut g = game("*..*/..../....", Automation::ZerosOnly);
    click(&mut g, 2, 2);
    g.toggle_flag(p(1, 0));
    click(&mut g, 3, 0);
    assert_eq!(g.status(), Status::Lost);
    assert_eq!(look(&g, 3, 0), Look::Exploded);
    assert_eq!(look(&g, 0, 0), Look::Mine);
    assert_eq!(look(&g, 1, 0), Look::WrongFlag);
}

#[test]
fn first_opened_square_is_never_a_mine() {
    let mut g = game("*./..", Automation::ZerosOnly);
    click(&mut g, 0, 0);
    assert_eq!(g.status(), Status::Playing);
    assert!(matches!(look(&g, 0, 0), Look::Number(_)));
    assert_eq!(g.mines_left(), 1);
}

#[test]
fn only_the_first_square_is_protected() {
    let mut g = game("*../...", Automation::ZerosOnly);
    click(&mut g, 2, 1);
    click(&mut g, 0, 0);
    assert_eq!(g.status(), Status::Lost);
}

#[test]
fn winning_flags_every_mine() {
    let mut g = game("*.", Automation::ZerosOnly);
    click(&mut g, 1, 0);
    assert_eq!(g.status(), Status::Won);
    assert_eq!(look(&g, 0, 0), Look::Flag);
    assert_eq!(g.mines_left(), 0);
}

#[test]
fn chording_opens_the_rest_when_flags_match() {
    let mut g = game("*../.../...", Automation::ZerosOnly);
    click(&mut g, 1, 1);
    click(&mut g, 1, 1);
    assert_eq!(look(&g, 2, 2), Look::Covered, "no flags yet, chord does nothing");
    g.toggle_flag(p(0, 0));
    click(&mut g, 1, 1);
    assert_eq!(look(&g, 2, 2), Look::Number(0));
    assert_eq!(g.status(), Status::Won);
}

#[test]
fn chording_with_a_wrong_flag_loses() {
    let mut g = game("*../.../...", Automation::ZerosOnly);
    click(&mut g, 1, 1);
    g.toggle_flag(p(2, 0));
    click(&mut g, 1, 1);
    assert_eq!(g.status(), Status::Lost);
    assert_eq!(look(&g, 0, 0), Look::Exploded);
}

#[test]
fn local_constraints_flag_and_open_by_themselves() {
    // The 2 below the mine pair flags both, then the 1 opens the last square.
    let rows = "**./.../...";
    let mut manual = game(rows, Automation::ZerosOnly);
    click(&mut manual, 2, 2);
    assert_eq!(manual.status(), Status::Playing);

    let mut auto = game(rows, Automation::LocalConstraints);
    click(&mut auto, 2, 2);
    assert_eq!(auto.status(), Status::Won);
}

#[test]
fn switching_automation_on_resolves_the_existing_board() {
    let mut g = game("**./.../...", Automation::ZerosOnly);
    click(&mut g, 2, 2);
    g.set_automation(Automation::LocalConstraints);
    g.settle();
    assert_eq!(g.status(), Status::Won);
}

#[test]
fn flag_count_can_exceed_mines() {
    let mut g = game("*...", Automation::ZerosOnly);
    g.toggle_flag(p(1, 0));
    g.toggle_flag(p(2, 0));
    assert_eq!(g.mines_left(), -1);
    g.toggle_flag(p(2, 0));
    assert_eq!(g.mines_left(), 0);
}

#[test]
fn help_is_granted_on_a_true_guess() {
    // Both bottom squares touch exactly the same two 1s.
    let mut g = game(GUESS, Automation::LocalConstraints);
    click(&mut g, 0, 0);
    assert_eq!(g.request_help_blocking(), Some(HelpVerdict::Granted));
    assert!(g.help_granted());
}

#[test]
fn granted_help_opens_a_mine_safely_once() {
    let mut g = game(GUESS, Automation::ZerosOnly);
    click(&mut g, 0, 0);
    g.request_help_blocking();
    click(&mut g, 0, 2);
    assert_eq!(g.status(), Status::Playing);
    assert_eq!(look(&g, 0, 2), Look::Flag);
    assert!(g.cell(p(0, 2)).opened_by_help);
    assert!(!g.help_granted());

    g.toggle_flag(p(0, 2));
    assert_eq!(look(&g, 0, 2), Look::Flag, "a mine found by help stays flagged");
}

#[test]
fn granted_help_marks_the_safe_square_it_opened() {
    let mut g = game(GUESS, Automation::ZerosOnly);
    click(&mut g, 0, 0);
    g.request_help_blocking();
    click(&mut g, 1, 2);
    assert_eq!(look(&g, 1, 2), Look::Number(1));
    assert!(g.cell(p(1, 2)).opened_by_help);
    assert_eq!(g.status(), Status::Won);
}

#[test]
fn first_square_keeps_granted_help() {
    let mut g = game("*../...", Automation::ZerosOnly);
    assert_eq!(g.request_help_blocking(), Some(HelpVerdict::Granted));
    click(&mut g, 0, 0);
    assert!(g.help_granted());
    assert!(!g.cell(p(0, 0)).opened_by_help);
}

#[test]
fn chording_does_not_use_up_granted_help() {
    let mut g = game(GUESS, Automation::ZerosOnly);
    click(&mut g, 0, 0);
    g.request_help_blocking();
    click(&mut g, 0, 1);
    assert!(g.help_granted());
}

#[test]
fn help_is_denied_when_a_number_settles_something() {
    let mut g = game("**./.../...", Automation::ZerosOnly);
    click(&mut g, 2, 2);
    assert_eq!(g.request_help_blocking(), Some(HelpVerdict::Denied));
    assert_eq!(g.status(), Status::Lost);
    assert!(g.cell(p(0, 0)).missed);
    assert!(g.cell(p(1, 0)).missed);
    assert!(!g.cell(p(2, 0)).missed);
}

#[test]
fn help_is_denied_when_flags_already_settle_a_number() {
    let mut g = game("*../.../...", Automation::ZerosOnly);
    click(&mut g, 1, 1);
    g.toggle_flag(p(0, 0));
    assert_eq!(g.request_help_blocking(), Some(HelpVerdict::Denied));
    assert!(g.cell(p(2, 2)).missed);
}

#[test]
fn help_is_denied_when_a_number_has_too_many_flags() {
    let mut g = game("*../.../...", Automation::ZerosOnly);
    click(&mut g, 1, 1);
    g.toggle_flag(p(0, 0));
    g.toggle_flag(p(1, 0));
    assert_eq!(g.request_help_blocking(), Some(HelpVerdict::Denied));
    assert!(g.cell(p(1, 1)).missed);
}

#[test]
fn help_is_denied_when_only_the_global_solver_sees_the_move() {
    // The article's example: the last mine is in the top-left pair, so the
    // far corner is safe, yet no single number says so.
    let rows = "*.*...../..*...../......**/......*.";
    let mut g = game(rows, Automation::ZerosOnly);
    click(&mut g, 5, 0);
    for (x, y) in [(2, 0), (2, 1), (6, 2), (7, 2), (6, 3)] {
        g.toggle_flag(p(x, y));
    }
    assert_eq!(look(&g, 7, 3), Look::Covered);
    assert_eq!(g.request_help_blocking(), Some(HelpVerdict::Denied));
    assert!(g.cell(p(7, 3)).missed);
}

#[test]
fn help_is_denied_when_flags_contradict_the_numbers() {
    // Both 2s touch the real mines. A flag on (0,1) leaves the left 2 wanting
    // one more mine and the right 2 wanting two, but only one mine is left.
    let mut g = game(".*./.*.", Automation::ZerosOnly);
    click(&mut g, 0, 0);
    click(&mut g, 2, 0);
    g.toggle_flag(p(0, 1));
    assert_eq!(g.request_help_blocking(), Some(HelpVerdict::Denied));
    assert!(g.cell(p(0, 1)).missed);
    assert!(!g.cell(p(1, 1)).missed);
}

#[test]
fn stale_solver_answer_is_ignored() {
    let mut g = game(GUESS, Automation::LocalConstraints);
    click(&mut g, 0, 0);
    let HelpStart::NeedsSolver(job) = g.request_help() else {
        panic!("expected solver job");
    };
    g.toggle_flag(p(1, 2));
    assert_eq!(g.conclude_help(&job, Solution::Deductions(vec![])), None);
    assert!(!g.help_granted());
}

#[test]
fn help_waits_for_automation_to_finish() {
    let mut g = game(GUESS, Automation::ZerosOnly);
    g.click(p(0, 0));
    assert!(g.is_busy());
    assert!(matches!(g.request_help(), HelpStart::Unavailable));
}

#[test]
fn in_progress_once_something_is_opened_or_flagged() {
    let mut g = game("*../.../...", Automation::ZerosOnly);
    assert!(!g.in_progress());
    g.toggle_flag(p(0, 0));
    assert!(g.in_progress());
    g.toggle_flag(p(0, 0));
    click(&mut g, 1, 1);
    assert!(g.in_progress());
    click(&mut g, 0, 0);
    assert_eq!(g.status(), Status::Lost);
    assert!(!g.in_progress(), "a finished game is not in progress");
}

#[test]
fn restart_keeps_settings_and_clears_state() {
    let mut g = Game::new(Config::EXPERT, Automation::ZerosOnly, Rng::seeded(9));
    g.toggle_flag(p(0, 0));
    let before = g.revision();
    g.restart();
    assert_eq!(g.mines_left(), 99);
    assert_eq!(g.automation(), Automation::ZerosOnly);
    assert!(g.revision() > before);
    assert_eq!(look(&g, 0, 0), Look::Covered);
}

fn events(g: &mut Game) -> Vec<Event> {
    g.take_events().into_iter().map(|t| t.event).collect()
}

#[test]
fn events_record_player_and_game_actions() {
    let mut g = game("*../.../...", Automation::ZerosOnly);
    click(&mut g, 1, 1);
    g.toggle_flag(p(0, 0));
    g.request_help_blocking();
    assert_eq!(
        events(&mut g),
        vec![
            Event::NewGame {
                layout: "*../.../...".into()
            },
            Event::PlayerClicked(p(1, 1)),
            Event::Revealed {
                pos: p(1, 1),
                number: 1
            },
            Event::PlayerToggledFlag(p(0, 0)),
            Event::Flagged(p(0, 0)),
            Event::PlayerRequestedHelp,
            Event::HelpDecided {
                verdict: HelpVerdict::Denied,
                missed: vec![p(1, 0), p(2, 0), p(0, 1), p(2, 1), p(0, 2), p(1, 2), p(2, 2)],
            },
            Event::Lost { exploded: None },
        ]
    );
    assert!(g.take_events().is_empty());
}

#[test]
fn events_carry_the_game_clock() {
    let mut g = game("..../..../...*", Automation::ZerosOnly);
    g.click(p(0, 0));
    g.advance_to(ORTHOGONAL_DELAY);
    let times: Vec<Millis> = g.take_events().iter().map(|t| t.at).collect();
    assert_eq!(times, vec![0, 0, 0, 20, 20]);
}

#[test]
fn moving_the_first_mine_is_recorded() {
    let mut g = game("*../...", Automation::ZerosOnly);
    click(&mut g, 0, 0);
    let moved = events(&mut g)
        .into_iter()
        .any(|e| matches!(e, Event::MineMoved { from, .. } if from == p(0, 0)));
    assert!(moved);
}

#[test]
fn restart_keeps_uncollected_events() {
    let mut g = game("*..", Automation::ZerosOnly);
    g.toggle_flag(p(1, 0));
    g.restart();
    let all = events(&mut g);
    assert!(all.contains(&Event::Flagged(p(1, 0))));
    assert!(matches!(all.last(), Some(Event::NewGame { .. })));
}

proptest! {
    #[test]
    fn first_click_never_loses(seed: u64, x in 0usize..30, y in 0usize..16) {
        let mut g = Game::new(Config::EXPERT, Automation::LocalConstraints, Rng::seeded(seed));
        click(&mut g, x, y);
        prop_assert_ne!(g.status(), Status::Lost);
        prop_assert_eq!(g.mines_left() + g.board.flag_count() as i64, 99);
    }

    /// Once automation settles, no single number is left with a move it
    /// forces. Only safe squares are clicked, so every game runs long.
    #[test]
    fn local_automation_reaches_a_fixed_point(
        seed: u64,
        (w, h) in (3usize..9, 3usize..9),
        density in 10usize..35,
        picks in proptest::collection::vec(any::<usize>(), 1..40),
    ) {
        let dims = Dims::new(w, h);
        let mines = (dims.area() * density / 100).max(1);
        let mut g = Game::new(Config { dims, mines }, Automation::LocalConstraints, Rng::seeded(seed));
        for pick in picks {
            let safe: Vec<Pos> = dims
                .positions()
                .filter(|&q| g.cell(q).look == Look::Covered && !g.board.is_mine(q))
                .collect();
            if g.status() != Status::Playing || safe.is_empty() {
                break;
            }
            let target = safe[pick % safe.len()];
            click(&mut g, target.x, target.y);
            prop_assert_ne!(g.status(), Status::Lost);
            if g.status() == Status::Playing {
                prop_assert_eq!(g.board.locally_forced(), vec![], "after opening {:?}", target);
            }
        }
    }

    /// Flags from automation are always right, since it only acts on what
    /// the numbers force.
    #[test]
    fn automation_never_flags_a_safe_square(seed: u64, x in 0usize..30, y in 0usize..16) {
        let mut g = Game::new(Config::EXPERT, Automation::LocalConstraints, Rng::seeded(seed));
        click(&mut g, x, y);
        prop_assert!(g.board.wrong_flags().is_empty());
        prop_assert_ne!(g.status(), Status::Lost);
    }
}
