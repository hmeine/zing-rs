use serde::{Deserialize, Serialize};

use crate::{card_action::CardAction, game::GameState};

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientNotification {
    GameStatus(GameState, usize, Option<usize>),
    CardActions(Vec<CardAction>, Option<usize>),
}
