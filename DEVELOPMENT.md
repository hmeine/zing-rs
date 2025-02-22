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
  locally, so one has to run `shuttle run` first / in parallel.
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
