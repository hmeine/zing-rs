use crate::{game::{GameState, StackState}, decks::shuffled_deck};

pub struct ZingGame {}

impl ZingGame {
    pub fn new_from_table(table: crate::table::Table) -> GameState {
        let mut result = GameState::new_from_table(table);
        result.stacks.push(StackState::new_from_deck("Draw Stack".into(), shuffled_deck(crate::Back::Blue), false));
        for player in 0..result.players.len() {
            result.hand_out_cards(player, 4);
        }
        result
    }
}