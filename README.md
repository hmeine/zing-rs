Zing Game
=========

This is a WIP implementation of a simple, but very fun card game named "Zing", apparently originating in Montenegro.  It is a trick-taking game, the [rules for which are documented here in English](Rules_en.md) and [in German](Rules_de.md).

The idea to implement an electronic edition came to me during the pandemic, when people were staying home, and playing online games was a good way to socialize with your peers or to have teambuilding activities for new groups.

As of October 2023, the status is that the game is at last playable, only the official deployment running on shuttle.rs under the address [zing.shuttleapp.rs](https://zing.shuttleapp.rs/) gets frequently reset (as expected on the free tier), which currently requires reloading the frontend (TODO: show a proper "connection lost" popup).

How to Play
-----------

Although there is some code for computer players, the game can currently only be played with human players.  Each "table" requires exactly two players (four are not supported yet).

Visit the server URL (e.g. [on shuttle](https://zing.shuttleapp.rs/) or on [localhost](http://localhost:8000/) or wherever a server is running) and log in with a player name of your choice.  One player needs to open a table and send an opponent a link to join this table.  Then, games can be started and played according to [rules](Rules_en.md).  By reloading the URL, one returns to the table overview.  This might come in handy if the connection is lost, in which case the game can be resumed, and it is currently a necessary step if the game has finished, in order to start a new game.

Technical Details
-----------------

* There is an axum-based server implementing the game logic, offering a restful API as well as persistent WebSocket connections for real-time notifications.  The game does not require any registration / personal details, but it no longer uses ephemeral (in-memory) storage only, but uses a (Postgres) DB.  The advantage of the latter is that one can reliably play full matches by resuming (reloading) when the server gets reset or the connection breaks.
* A simple Quasar-based reactive web frontend allows to log in, open new tables (for matches with multiple games), or join existing ones by others. It also opens a persistent websocket connection to the server and updates the status of tables in realtime.
* A Bevy-based UI can connect to the server and provides an animated 2D card game UI.  This component can be compiled as a standalone app (with a networking part based on tokio) or as WASM build (then using the browser's networking stack).  The WASM is embedded in the webpage, making it possible for players to start games without having to download, install, or run other binaries.

I am also using it as a fun project to learn and practice Rust in, and I have been doing it in my limited "spare" time only, besides work and family, so there have been (and always will be) longish stretches of non-activity.  For details on the software design, how to build or develop, see [DEVELOPMENT.md](DEVELOPMENT.md).

Limitations
-----------

* In general, both the web UI as well as the game itself could be more beautiful.
* The game currently *only* renders cards and does not have *any* text or UI elements.  Although it is totally playable as-is, it would be cooler to display UI elements, for instance
  * player names
  * whose turn it is
  * proper messages if the connection is lost
  * scores
  * exit / finish / next round buttons
* The four player mode is not fully supported yet (mostly due to UI layout complications).
* Practical nuisance: The shuttle deployment is frequently reset (every 30 minutes or so), which means that all server connections break. The web UI would still show the most recent state, but games suddenly stop working (you would notice that you can no longer play cards, and you could see 401 unauthorized errors in the browser console because the login cookie becomes invalid). Reloading, however, should allow to resume games and matches.

See also the [issue section on GitHub](https://github.com/hmeine/zing-rs/issues).

License
-------

This project is shared with you under the permissive MIT license.  The card faces used here are LGPL'd:

    Vector Playing Cards 3.2
    https://totalnonsense.com/open-source-vector-playing-cards/
    Copyright 2011,2021 – Chris Aguilar – conjurenation@gmail.com
    Licensed under: LGPL 3.0 - https://www.gnu.org/licenses/lgpl-3.0.html
