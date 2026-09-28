//! Drives the real window with synthetic input.

use std::time::{Duration, Instant};

use eframe::egui::{Event, Modifiers, PointerButton, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use minesweeper_deterministic::board::Board;
use minesweeper_deterministic::game::{Automation, Game, Look, Status};
use minesweeper_deterministic::grid::Pos;
use minesweeper_deterministic::rng::Rng;
use minesweeper_deterministic::ui::App;

/// See [`Board::from_layout`].
fn window(rows: &str, automation: Automation) -> Harness<'static, App> {
    let board = Board::from_layout(rows).expect("valid layout");
    let game = Game::with_board(board, automation, Rng::seeded(0));
    let mut harness = Harness::builder()
        .with_size(vec2(700.0, 400.0))
        .build_eframe(move |_| App::new(game, None));
    harness.step();
    harness
}

/// Steps frames until `done` holds, failing after a few seconds.
fn wait_until(harness: &mut Harness<'static, App>, done: impl Fn(&Harness<'static, App>) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !done(harness) {
        assert!(Instant::now() < deadline, "timed out");
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
}

fn settle(harness: &mut Harness<'static, App>) {
    wait_until(harness, |h| !h.state().game().is_busy());
}

fn press(
    harness: &mut Harness<'static, App>,
    at: Pos,
    button: PointerButton,
    pressed: bool,
    modifiers: Modifiers,
) {
    let pos = harness.state().square_centre(at).expect("board drawn");
    harness.event(Event::PointerMoved(pos));
    harness.event(Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers,
    });
    harness.step();
}

fn click(harness: &mut Harness<'static, App>, at: Pos, button: PointerButton) {
    press(harness, at, button, true, Modifiers::NONE);
    press(harness, at, button, false, Modifiers::NONE);
}

fn look(harness: &Harness<'static, App>, x: usize, y: usize) -> Look {
    harness.state().game().cell(Pos::new(x, y)).look
}

#[test]
fn left_click_opens_a_square() {
    let mut h = window("*../.../...", Automation::ZerosOnly);
    click(&mut h, Pos::new(1, 1), PointerButton::Primary);
    assert_eq!(look(&h, 1, 1), Look::Number(1));
}

#[test]
fn releasing_elsewhere_opens_that_square_instead() {
    let mut h = window("*../.../...", Automation::ZerosOnly);
    press(
        &mut h,
        Pos::new(1, 1),
        PointerButton::Primary,
        true,
        Modifiers::NONE,
    );
    assert_eq!(look(&h, 1, 1), Look::Covered, "nothing opens on press");
    press(
        &mut h,
        Pos::new(1, 0),
        PointerButton::Primary,
        false,
        Modifiers::NONE,
    );
    assert_eq!(look(&h, 1, 1), Look::Covered);
    assert_eq!(look(&h, 1, 0), Look::Number(1));
}

#[test]
fn leaving_the_board_lets_go() {
    let mut h = window("*../.../...", Automation::ZerosOnly);
    press(
        &mut h,
        Pos::new(1, 1),
        PointerButton::Primary,
        true,
        Modifiers::NONE,
    );
    let off_board = h.state().square_centre(Pos::new(0, 0)).expect("drawn") - vec2(200.0, 0.0);
    h.event(Event::PointerMoved(off_board));
    h.step();
    press(
        &mut h,
        Pos::new(1, 1),
        PointerButton::Primary,
        false,
        Modifiers::NONE,
    );
    assert_eq!(look(&h, 1, 1), Look::Covered);
}

#[test]
fn right_click_and_ctrl_click_toggle_flags() {
    let mut h = window("*../.../...", Automation::ZerosOnly);
    click(&mut h, Pos::new(0, 0), PointerButton::Secondary);
    assert_eq!(look(&h, 0, 0), Look::Flag);
    h.get_by_label("Mines left: 0");

    press(
        &mut h,
        Pos::new(0, 0),
        PointerButton::Primary,
        true,
        Modifiers::CTRL,
    );
    press(
        &mut h,
        Pos::new(0, 0),
        PointerButton::Primary,
        false,
        Modifiers::CTRL,
    );
    assert_eq!(
        look(&h, 0, 0),
        Look::Covered,
        "ctrl-click unflags without opening"
    );
}

#[test]
fn granted_help_opens_a_mine_safely() {
    let mut h = window("../../*.", Automation::ZerosOnly);
    click(&mut h, Pos::new(0, 0), PointerButton::Primary);
    settle(&mut h);
    h.get_by_label("Request help").click();
    wait_until(&mut h, |h| h.query_by_label("Granted!").is_some());

    click(&mut h, Pos::new(0, 2), PointerButton::Primary);
    assert_eq!(look(&h, 0, 2), Look::Flag);
    assert!(h.state().game().cell(Pos::new(0, 2)).opened_by_help);
    assert_eq!(h.state().game().status(), Status::Playing);
}

#[test]
fn help_on_a_solvable_board_loses() {
    let mut h = window("**./.../...", Automation::ZerosOnly);
    click(&mut h, Pos::new(2, 2), PointerButton::Primary);
    settle(&mut h);
    h.get_by_label("Request help").click();
    h.step();
    assert_eq!(h.state().game().status(), Status::Lost);
    assert!(h.state().game().cell(Pos::new(0, 0)).missed);
}

#[test]
fn switching_automation_resolves_the_board() {
    let mut h = window("**./.../...", Automation::ZerosOnly);
    click(&mut h, Pos::new(2, 2), PointerButton::Primary);
    settle(&mut h);
    h.get_by_label("Resolve local constraints").click();
    h.step();
    settle(&mut h);
    assert_eq!(h.state().game().status(), Status::Won);
}

#[test]
fn smiley_starts_a_new_game() {
    let mut h = window("*../.../...", Automation::ZerosOnly);
    click(&mut h, Pos::new(1, 1), PointerButton::Primary);
    h.get_by_label("New game").click();
    h.step();
    assert_eq!(look(&h, 1, 1), Look::Covered);
}
