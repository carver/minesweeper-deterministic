# To do

## Jason's ideas

- chord-click for flagging a mine, because Framework laptop right-click is awkward
    - Alt-click?
    - Ctrl-click?
- Choose board size and mine count (beginner, intermediate, expert, custom).
- Start the game for you: open a first square, ideally one that is a zero so
  the board opens up.
- Resolve more trivial cases automatically. For example two unknown squares
  that share every constraint and hold exactly one mine between them: nothing
  can tell them apart, so it is a forced guess. Offer to spend help on it
  straight away, or open one of them for free.
- identify if a click was a guess that could not have been deduced. Possibly:
    - lose game
    - report when game is over as lucky
- Start-up feels a little slow. Any options to speed it up?

## Claude's ideas

- Replay a game from its journal, to reproduce a reported bug in a test.
- Keyboard shortcuts (F2 or N for a new game, H for help).
- Timer and best times per board size.
- Show a warning while the solver runs long, with a way to cancel.
- Undo the last move after a denied help request, as a practice mode.
