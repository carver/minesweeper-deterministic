# Deterministic Minesweeper

A Linux desktop version of the last game in Magnus Hoff's
[Solving Minesweeper](https://magnushoff.com/articles/minesweeper/), the one
under "The deterministic version".

The twist is the **Request help** button. Press it when you believe nothing
on the board can be deduced. If the solver agrees, you get to open one square
safely, even a mine. If it finds a move you missed, you lose, and the squares
you missed turn red. Luck never decides a game. Only mistakes do.

## Playing

- Left click opens a square. Release over a different square to open that
  one instead, as in the classic game.
- Left click on a number whose flags are all placed opens its other
  neighbours. Middle click does the same as left click.
- Right click, or Ctrl/Shift/Alt with left click, toggles a flag.
- The smiley starts a new game.
- **No extra automation** only floods open squares with no adjacent mines.
- **Resolve local constraints** (the default) also flags and opens whatever a
  single number settles on its own. A wrong flag can then open a mine for
  you, just like in the original.
- After help is granted the board gets a gold frame and "Granted!" shows. The
  next square you open is opened safely. A mine found that way stays flagged
  with a yellow tint.

The board is 30×16 with 99 mines. Your first square is never a mine.

Window size and the automation choice are remembered between runs, in
`~/.local/share/minesweeper-deterministic/app.ron`.

## Journal

Every game is logged to `~/.local/state/minesweeper-deterministic/`
(or `$XDG_STATE_HOME/minesweeper-deterministic/`), one file per UTC day.
Each line has the wall time, the game's own clock and what happened: your
clicks, flags and help requests, and everything the game did in response.
The `new game` line lists every mine, so a game can be replayed. Files older
than 30 days are deleted when the game starts. The path is printed to stderr
on startup.

## Building

Needs a Rust toolchain (edition 2024, so Rust 1.85 or newer).

```sh
cargo run --release
```

eframe draws with OpenGL and works on both X11 and Wayland.

To install it with a launcher entry and icon for your user:

```sh
scripts/install-desktop.sh            # -h for options, --uninstall to remove
```

## Development

```sh
git config core.hooksPath .githooks   # rustfmt check before each commit
cargo clippy --all-targets -- -D warnings
cargo test                            # CI runs this with --release
cargo run --release --example solver_stress -- 200 40   # solver timing
```

`solver_stress` times the solver on expert boards with a given percentage of
safe squares revealed at random. Scattered reveals make wider frontiers than
real play does; at 20 to 40% the worst case is around 150 ms.

Layout:

| Path | What |
| --- | --- |
| `src/grid.rs` | Positions and neighbourhoods |
| `src/board.rs` | Mines and what the player sees; the local checks help uses |
| `src/game.rs` | Rules: opening, flagging, automation, winning, losing |
| `src/game/help.rs` | The Request help flow |
| `src/schedule.rs` | Virtual clock that turns automation into a ripple |
| `src/solver.rs`, `src/solver/` | Global solver |
| `src/game/event.rs` | What the journal records |
| `src/journal.rs` | Daily journal files and pruning |
| `src/ui.rs`, `src/ui/` | egui window, drawing, background solver thread |
| `tests/perfect_play.rs` | A flawless player must win every game |

The game logic never looks at wall time. The UI feeds it milliseconds and it
runs whatever automation is due, which keeps every rule testable without a
window.

The solver treats unknown squares next to numbers as a constraint problem,
splits it into components that share no number, and counts each component
with a dynamic program over "how many mines does each half-filled number have
so far". The components are tied back together through the total mine count.
The original page enumerated every layout, which could take forever on some
boards; this one handles a full expert game in milliseconds.

See `todo.md` for planned features and `uncertainties.md` for judgement calls
made while matching the original.
