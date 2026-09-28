# To do

## Jason's ideas

- Custom board size and mine count (the three classic sizes are done).
- Start the game for you: open a first square, ideally one that is a zero so
  the board opens up.
- Resolve more trivial cases automatically. For example two unknown squares
  that share every constraint and hold exactly one mine between them: nothing
  can tell them apart, so it is a forced guess. Offer to spend help on it
  straight away, or open one of them for free.
- identify if a click was a guess that could not have been deduced. Possibly:
    - lose game
    - report when game is over as lucky

## Claude's ideas

- Replay a game from its journal, to reproduce a reported bug in a test.
- Timer and best times per board size.
- Show a warning while the solver runs long, with a way to cancel.
- Undo the last move after a denied help request, as a practice mode.
