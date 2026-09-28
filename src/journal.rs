//! Appends game events to one text file per day, so a bad game can be
//! looked at afterwards. Files older than [`KEEP_DAYS`] are deleted when a
//! journal opens and whenever it starts a new day's file.
//!
//! Lines look like `21:50:03.120 +1520ms player clicked (4,7)`: UTC wall
//! time, then the game's own clock. A `new game` line lists every mine, so
//! a game can be replayed from its journal.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::game::Timed;

pub const KEEP_DAYS: i64 = 30;
const PREFIX: &str = "journal-";
const SUFFIX: &str = ".log";

pub struct Journal {
    dir: PathBuf,
    open: Option<(Date, BufWriter<File>)>,
}

impl Journal {
    /// `$XDG_STATE_HOME/minesweeper-deterministic`, falling back to
    /// `~/.local/state/minesweeper-deterministic`.
    pub fn default_dir() -> Option<PathBuf> {
        let state = std::env::var_os("XDG_STATE_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".local/state")))?;
        Some(state.join("minesweeper-deterministic"))
    }

    /// Creates the directory if needed and prunes old files.
    pub fn open(dir: PathBuf, now: SystemTime) -> io::Result<Self> {
        fs::create_dir_all(&dir)?;
        prune(&dir, Date::of(now));
        Ok(Self { dir, open: None })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn write(&mut self, now: SystemTime, events: &[Timed]) -> io::Result<()> {
        if events.is_empty() {
            return Ok(());
        }
        let today = Date::of(now);
        let file = self.file_for(today)?;
        let clock = time_of_day(now);
        for Timed { at, event } in events {
            writeln!(file, "{clock} +{at}ms {event}")?;
        }
        file.flush()
    }

    fn file_for(&mut self, today: Date) -> io::Result<&mut BufWriter<File>> {
        if self.open.as_ref().is_none_or(|(date, _)| *date != today) {
            prune(&self.dir, today);
            let path = self.dir.join(file_name(today));
            let file = OpenOptions::new().create(true).append(true).open(path)?;
            self.open = Some((today, BufWriter::new(file)));
        }
        Ok(&mut self.open.as_mut().expect("just opened").1)
    }
}

fn file_name(date: Date) -> String {
    format!("{PREFIX}{date}{SUFFIX}")
}

/// Deletes journal files dated more than [`KEEP_DAYS`] before `today`.
/// Other files are left alone. Failures are reported and skipped: an old
/// file that will not go away is no reason to stop journaling.
fn prune(dir: &Path, today: Date) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("journal: cannot list {}: {e}", dir.display());
            return;
        }
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(date) = name.to_str().and_then(parse_file_name) else {
            continue;
        };
        if today.days - date.days > KEEP_DAYS
            && let Err(e) = fs::remove_file(entry.path())
        {
            eprintln!("journal: cannot delete {}: {e}", entry.path().display());
        }
    }
}

fn parse_file_name(name: &str) -> Option<Date> {
    let date = name.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    let mut parts = date.splitn(3, '-').map(str::parse::<i64>);
    let (y, m, d) = (parts.next()?.ok()?, parts.next()?.ok()?, parts.next()?.ok()?);
    Date::from_ymd(y, m, d)
}

fn since_epoch(t: SystemTime) -> Duration {
    t.duration_since(UNIX_EPOCH).unwrap_or_default()
}

fn time_of_day(t: SystemTime) -> String {
    let d = since_epoch(t);
    let secs = d.as_secs() % 86_400;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        secs / 3600,
        secs / 60 % 60,
        secs % 60,
        d.subsec_millis()
    )
}

/// A UTC calendar day, stored as days since 1970-01-01.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Date {
    days: i64,
}

impl Date {
    fn of(t: SystemTime) -> Self {
        Self {
            days: (since_epoch(t).as_secs() / 86_400) as i64,
        }
    }

    /// Howard Hinnant's `days_from_civil`.
    fn from_ymd(y: i64, m: i64, d: i64) -> Option<Self> {
        if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
            return None;
        }
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Some(Self {
            days: era * 146_097 + doe - 719_468,
        })
    }

    /// Howard Hinnant's `civil_from_days`.
    fn ymd(self) -> (i64, i64, i64) {
        let z = self.days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        (y, m, d)
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (y, m, d) = self.ymd();
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

#[cfg(test)]
mod tests;
