Zing Game
=========

This is a WIP implementation of a simple, but very fun card game named "Zing", apparently originating in Montenegro.  It is a trick-taking game, the [rules for which are documented here in English](Rules_en.md) and [in German](Rules_de.md).

The idea to implement an electronic edition came to me during the pandemic, when people were staying home, and playing online games was a good way to socialize with your peers or to have teambuilding activities for new groups.

Currently, the status is that it is not yet conveniently playable online yet, but the components are mostly functional already:

* There is an axum-based server implementing the game logic, offering a restful API as well as persistent WebSocket connections for real-time notifications.
* A simple web frontend allows to log in, open new tables, or join existing ones by others.
* A Bevy-based UI can connect to the server and provides an animated 2D card game UI.  The networking part is currently based on tokio, which turned out to be hindering a WASM build (which a previous single-player version of the game ran in as a proof of concept).

I am also using it as a fun project to learn and practice Rust in, and I have been doing it in my limited "spare" time only, besides work and family, so there have been (and always will be) longish stretches of non-activity.

As of 2023-05-01, the game server is now running on shuttle.rs under the address zing.shuttleapp.rs, but in addition to the limited usefulness without the WASM build, it seems to be frequently reset at the moment (possibly due to the free tier it is running on).

License
-------

This project is shared with you under the permissive MIT license.  The card faces used here are LGPL'd:

    Vector Playing Cards 3.2
    https://totalnonsense.com/open-source-vector-playing-cards/
    Copyright 2011,2021 – Chris Aguilar – conjurenation@gmail.com
    Licensed under: LGPL 3.0 - https://www.gnu.org/licenses/lgpl-3.0.html
