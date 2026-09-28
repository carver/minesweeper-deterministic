use std::sync::atomic::{AtomicU32, Ordering};

use super::*;
use crate::game::{Automation, Event};
use crate::grid::Pos;

const DAY: u64 = 86_400;

/// A fresh directory, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("ms-journal-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn at_day(days: i64, secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(days as u64 * DAY + secs)
}

fn day(y: i64, m: i64, d: i64) -> i64 {
    Date::from_ymd(y, m, d).expect("valid date").days
}

fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("readable")
        .map(|e| e.expect("entry").file_name().into_string().expect("utf-8"))
        .collect();
    names.sort();
    names
}

#[test]
fn dates_match_the_calendar() {
    assert_eq!(day(1970, 1, 1), 0);
    assert_eq!(day(2000, 3, 1), 11_017);
    assert_eq!(
        Date {
            days: day(2024, 2, 29)
        }
        .to_string(),
        "2024-02-29"
    );
    for days in [0, 59, 60, 11_016, 20_000, 20_723, 40_000] {
        let (y, m, d) = Date { days }.ymd();
        assert_eq!(day(y, m, d), days);
    }
}

#[test]
fn file_names_round_trip() {
    let date = Date {
        days: day(2026, 9, 27),
    };
    assert_eq!(file_name(date), "journal-2026-09-27.log");
    assert_eq!(parse_file_name(&file_name(date)), Some(date));
    assert_eq!(parse_file_name("journal-2026-13-01.log"), None);
    assert_eq!(parse_file_name("notes.txt"), None);
}

#[test]
fn opening_prunes_only_old_journals() {
    let tmp = TempDir::new();
    let today = day(2026, 9, 27);
    for name in [
        "journal-2026-08-27.log",
        "journal-2026-08-28.log",
        "journal-2026-09-27.log",
        "notes.txt",
    ] {
        fs::write(tmp.0.join(name), "x").expect("write");
    }
    Journal::open(tmp.0.clone(), at_day(today, 60)).expect("open");
    assert_eq!(
        files(&tmp.0),
        ["journal-2026-08-28.log", "journal-2026-09-27.log", "notes.txt"]
    );
}

#[test]
fn writes_timestamped_lines_and_rolls_over_at_midnight() {
    let tmp = TempDir::new();
    let today = day(2026, 9, 27);
    let mut journal = Journal::open(tmp.0.clone(), at_day(today, 0)).expect("open");
    let events = [
        Timed {
            at: 0,
            event: Event::NewGame {
                layout: "*./..".into(),
            },
        },
        Timed {
            at: 1520,
            event: Event::PlayerClicked(Pos::new(4, 7)),
        },
    ];
    journal.write(at_day(today, 3600 + 61), &events).expect("write");
    let late = [Timed {
        at: 2000,
        event: Event::PlayerSetAutomation(Automation::ZerosOnly),
    }];
    journal.write(at_day(today + 1, 5), &late).expect("write");

    let first = fs::read_to_string(tmp.0.join("journal-2026-09-27.log")).expect("read");
    assert_eq!(
        first,
        "01:01:01.000 +0ms new game, mines *./..\n01:01:01.000 +1520ms player clicked (4,7)\n"
    );
    let second = fs::read_to_string(tmp.0.join("journal-2026-09-28.log")).expect("read");
    assert_eq!(second, "00:00:05.000 +2000ms player set automation ZerosOnly\n");
}

#[test]
fn a_new_day_prunes_again() {
    let tmp = TempDir::new();
    let start = day(2026, 9, 1);
    let mut journal = Journal::open(tmp.0.clone(), at_day(start, 0)).expect("open");
    let event = [Timed {
        at: 0,
        event: Event::Won,
    }];
    journal.write(at_day(start, 0), &event).expect("write");
    journal
        .write(at_day(start + KEEP_DAYS + 1, 0), &event)
        .expect("write");
    assert_eq!(files(&tmp.0), ["journal-2026-10-02.log"]);
}

#[test]
fn notes_are_timestamped() {
    let tmp = TempDir::new();
    let today = day(2026, 9, 28);
    let mut journal = Journal::open(tmp.0.clone(), at_day(today, 0)).expect("open");
    journal.note(at_day(today, 7), "startup: hello").expect("note");
    let text = fs::read_to_string(tmp.0.join("journal-2026-09-28.log")).expect("read");
    assert_eq!(text, "00:00:07.000 startup: hello\n");
}

#[test]
fn nothing_to_write_creates_no_file() {
    let tmp = TempDir::new();
    let mut journal = Journal::open(tmp.0.clone(), at_day(0, 0)).expect("open");
    journal.write(at_day(0, 0), &[]).expect("write");
    assert!(files(&tmp.0).is_empty());
}
