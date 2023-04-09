use serde::{Serialize, Deserialize};

use crate::{game::GameState, card_action::CardAction};

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientNotification {
    GameStarted(GameState),
    CardActions(Vec<CardAction>),
}
