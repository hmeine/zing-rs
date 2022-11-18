Design Thoughts
===============

The server has the full state, clients have "views" and do not know about face
down card values.

During a game, everything that happens is that cards move.  We want to model
such actions, probably using the builder pattern, in order to have a consistent
interface for drawing new cards, passing cards to others, playing cards to the
table, etc.

Clients should receive updates in such a way that it facilitates incrementally
updating the views (or animating card moves), which means that we need some kind
of observation mechanism and action descriptions.

Card moving actions consist of:

- source stack or hand
- source indices of the respective cards (defaulting to last n)
- number of cards (defaulting to 1)
- target stack or hand
- whether the cards should be face up or down (defaulting to source state)

Stacks can be

- "stock" for drawing stacks (face down, shuffled)
- player hands (usually only visible to the respective player, although there
  are exceptions in both directions)
- discard pile (face up, stacked)
- open table cards (face up, all visible, like in "Schwimmen")
- owned cards (theoretically known, but usually placed face down, for later
  score counting)

Each stack has

- a position on the table (view property, actually)
- a list of cards
- each of which can be face up or face down
- or peeking out (like a bookmark)

The server does not have to care about the position of stacks, but it is
responsible for the game logic.  It will be interesting to see how the frontend
can indicate valid moves, because we want the logic to be on the server side
mostly.

Graphical Layout
----------------

There can be two or four players.  In two player mode, player hands should be at
bottom and top, "self" at the bottom, with openly visible hand.  Other hands
will only be backs, but spread out nevertheless.  In four player mode, left and
right should be rotated.

The open card(s) should be in the middle.

The drawing stack can be at the side (two player mode), or as close to that as
possible (four player mode).

The winning stacks should be at the bottom right (for one self) or at the top
left (two player mode enemy).  In four player mode, the position of the latter
should be closer to the side and also rotated (the opposite player is your
friend).

Server State
------------

For multi-player games, there's the zing-server which needs to hold state for an
arbitrary number of clients.  Each client should log in with a user name and get
a login ID as cookie.  Each user may open or join an arbitrary number of tables

Invariants:

- Every table should have at least one user at it; as soon as the last user
  leaves the table, it should be removed.
- Every user may create as many tables as desired, but no more than one table at
  which no other users have joined.
- A game can be started at tables with exactly two or exactly four players.
- After a game has been started, players can no longer leave or join the table.
