# Uncertainties

Judgement calls made while copying the original game, written while you were
offline. Each one lists the options, what I picked and the best case against it.

## Flags that contradict the numbers

The original's solver finds no valid layout, returns an empty list, and so
grants help. You chose: lose, and mark the wrong flags red.

- Options: grant (original), lose and mark wrong flags, lose and mark the
  numbers that cannot be satisfied.
- Counter-argument: marking the wrong flags reveals ground truth the player
  never earned. Marking the unsatisfiable numbers explains the mistake using
  only what was visible.
- When a single number has more flags than its value, the original's own
  check catches it first and marks that number. I kept that. The wrong flag
  still shows crossed out once the game is lost.

## Asking for help while automation is still rippling

The original lets you press the button mid-ripple. Its local check then sees
moves the automation was about to make and fails you.

- Options: allow it (original), disable the button until automation
  finishes, or wait for automation and then check.
- Picked: the button is disabled while "working" shows. Ripples last a
  fraction of a second, and losing to the program's own pending move is
  clearly a bug.
- Counter-argument: waiting and then checking would feel smoother than a
  button that is briefly greyed out.

## What uses up granted help

In the original the next square opened by anything uses it up, including
automation that was still rippling and the neighbours opened by a chord
click. So a chord could spend your help on a square you never chose.

- Picked: only opening a covered square yourself uses it up. Chords and
  automation do not.
- Counter-argument: none that I find convincing. The original's behaviour
  looks accidental.

## Help granted before the first click

Asking before opening anything is always granted, since nothing is known. In
the original the first click then spends that help on a square that was
guaranteed safe anyway.

- Picked: the first square never uses up help, so it stays for later.
- Counter-argument: asking before the first click is pointless, and parity
  says to let the player waste it.

## Unflagging a mine found through help

The original flags the mine but lets you remove that flag afterwards, which
leaves a known mine unmarked.

- Picked: the flag is locked.
- Counter-argument: players who like full control might want every flag to
  behave the same. You asked for parity, so this is a one-line revert in
  `Game::toggle_flag` if you disagree.

## Switching to "Resolve local constraints" mid-game

The original swaps the rule set but applies it only from the next action on.

- Picked: switching on resolves the whole board right away.
- Counter-argument: parity. Someone flipping the setting to compare modes
  might expect nothing to happen until they click. Reverting means deleting
  `resolve_everywhere` in `src/game.rs`.

## Pre-commit hook

Your standards say hooks must finish in 5 seconds and should lint. Clippy
takes 9 to 12 seconds on this mounted checkout, so the hook only checks
formatting and CI runs clippy and the tests.

## Graphics

The original uses sprite images from its site. I drew everything as vector
shapes in the same classic style instead of copying another person's artwork,
which also scales cleanly to any window size.

- Counter-argument: the sprites are what the game looks like. If you want
  them, ask the author, or I can redraw closer to them.

## Solver strength

The original enumerates every layout of the constrained squares and could run
for a very long time on some boards (the article says so). Mine uses a
dynamic program and considers the total mine count exactly. On boards the
original could finish, both find the same deductions. On boards the original
could not finish, it would never answer, and mine does.

- Counter-argument: none for correctness. The DP can still blow up on
  contrived boards where many numbers are half-filled at once; the UI keeps
  running and shows "working" while it thinks.

## Numbers left unresolved next to a 0

Found from your screenshot. It is a bug in the original too. When a 0 opened,
the numbers around it never re-checked, so a number whose last extra covered
square was that 0 stayed unresolved. Now every opened square, 0 or not,
makes its neighbours re-check. No real trade-off here: the original's rule
description ("resolve local constraints") says it should resolve them.

