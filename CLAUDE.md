# Deterministic Minesweeper

`README.md` has the rules, controls and code layout. `uncertainties.md` records
every place this game departs from the original web version and why: add an
entry whenever you make such a call. `todo.md` has the owner's ideas and
yours, in separate sections.

## Checking the UI

- Engine behaviour: unit tests in `src/game/tests.rs`, where boards are
  written as layouts (`*../...`, see `Board::from_layout`).
- Window behaviour: `tests/window.rs` drives the real `App` with egui_kittest.
- Anything visual, or anything that depends on the display backend: run the
  app under `gui-shot` (on PATH, `gui-shot -h`), on **both** `--backend x11`
  and `--backend wayland`. The owner plays on GNOME Wayland; the title bar
  and window chrome differ between the two.

## Gotchas

- **egui_kittest**: the accessibility tree lags one frame, so step twice
  before asserting that something closed. A `ComboBox` opens its list a frame
  after the click, and has no label: find it with `get_by_value(<selected
  text>)`. Keyboard modifiers arrive on the pointer event, not in
  `InputState::modifiers`, which is why `handle_pointer` reads them there.
- **eframe features**: `default-features = false` also drops winit's Wayland
  title bar (`wayland-csd-adwaita`), leaving square buttons. Before changing
  eframe or winit features, compare `cargo tree -e features -i winit` before
  and after, and look at the window on Wayland.
- **Startup time**: each launch logs `startup: renderer ready after … ms` to
  the journal. Glow is the default because it measured faster on the owner's
  laptop; compare with `MINESWEEPER_RENDERER=wgpu`.
- **Builds**: `target/` sits on the host mount, which is slow, and the host
  builds here too (installs use `target/install/`). Run one cargo command at
  a time. A release build takes several minutes, so run it in the
  background. Each eframe feature change compiles a new copy of every
  dependency; after experimenting with features, `cargo clean` reclaims the
  stale ones (it once reached 16 GB against a 5 GB free sandbox disk).
