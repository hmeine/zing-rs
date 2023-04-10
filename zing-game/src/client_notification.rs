use serde::{Deserialize, Serialize};

use crate::{card_action::CardAction, game::GameState};

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientNotification {
    GameStarted(GameState, usize),
    CardActions(Vec<CardAction>),
}
