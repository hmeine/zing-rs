use crate::{
    card_action::{CardAction, CardRotation},
    decks::shuffled_deck,
    game::{GameState, StackState},
};

pub struct ZingGame {}

impl ZingGame {
    pub fn new_from_table(table: crate::table::Table) -> GameState {
        let mut result = GameState::new_from_table(table);
        result.stacks.push(StackState::new_from_deck(
            "stock".into(),
            shuffled_deck(crate::Back::Blue),
            false,
        ));
        result.stacks.push(StackState::new("table".into()));
        for player in 0..result.players.len() {
            CardAction::new()
                .from_stack_top(&result, 0, 4)
                .to_hand(&result, player)
                .rotate(CardRotation::FaceUp)
                .apply(&mut result);
        }
        CardAction::new()
            .from_stack_top(&result, 0, 4)
            .to_stack_top(&result, 1)
            .rotate(CardRotation::FaceUp)
            .apply(&mut result);
        result
    }
}
