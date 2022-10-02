use crate::{game::{GameState, StackState}, decks::deck};

pub struct ZingGame {}

impl ZingGame {
    pub fn new_from_table(table: crate::table::Table) -> GameState {
        let mut result = GameState::new_from_table(table);
        result.stacks.push(StackState::new_from_deck("Draw Stack".into(), deck(crate::Back::Blue), false));
        result
    }
}