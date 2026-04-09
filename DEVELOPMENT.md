Development Notes
=================

Crates Overview
---------------

* The crate `zing-game` is meant to contain the game logic.  In theory, I would
  have liked to separate general card game stuff from Zing-specific things, but
  I found that quite difficult, so the separation is obviously not perfect yet.
  My goal was to be able to support other games in the future as well.  This
  crate contains a few unit tests.
* The `zing-server` provides a Rest API on top of the above.  There are tests
  that can be excuted with `cargo test`, but they require a server to run
  locally, so start it in parallel with

  ```sh
  # you need some postgres server, e.g. with docker:
  docker run --rm -it --env POSTGRES_PASSWORD='MYPASSWORD' -p 5432:5432 postgres -d
  
  # adapt .env to your environment (e.g. above password)
  cp .env.example .env
  
  cargo run -p zing-server
  ```

  The server loads `.env` automatically, accepts optional `HOST` / `PORT`
  variables for the bind address, and applies DB migrations automatically
  during startup.
  * There is a `migration` sub-crate that is used for sea-orm database
    migration.  That is important since the sea-orm versions of the two crates
    need to be in sync.
* The `zing-ui-lib` crate implements a bevy-based UI that talks to the above
  server.  The `zing-ui` crate is a small binary around this library, and this
  separation is needed because the zing-ui-lib can also be built as WASM binary
  for embedding into a webpage.  Use

  ```sh
  RUSTFLAGS='--cfg getrandom_backend="wasm_js"' wasm-pack build zing-ui-lib --release --target web
  ```

  in order to build the WASM UI.
* The Quasar-based web frontend is relatively minimal at the moment; it is
  located in `zing-server/assets/index.html` (single file with CDN JS
  embeddings).

Design
------

The server supports subscriptions via WebSockets, and the UIs (both the web
interface and the Bevy game UI) open persistent WebSocket connections to the
server to receive real-time updates on the game state.  These come in the form
of `ClientNotification` messages.

The server also offers a REST API for performing actions; the WebSocket
connection is purely uni-directional to push updates to the clients.  There are
currently exactly two types of client notifications, namely GameStatus (which
sends the full game state, for instance when a player joins a table or when the
connection is first established) and CardActions (can be incrementally applied
to change the game state, replicating the server-side state at each client).

The Bevy UI uses events to react to these notifications and update the UI
accordingly, and there are currently exactly two types of events corresponding
to these two types of client notifications.
