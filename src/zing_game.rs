use crate::{
    card_action::{CardAction, CardRotation},
    decks::shuffled_deck,
    game::{GameState, StackState},
};

pub struct ZingGame {
    pub game_state: GameState,
}

impl ZingGame {
    pub fn new_from_table(table: crate::table::Table) -> Self {
        let mut game_state = GameState::new_from_table(table);
        game_state.stacks.push(StackState::new_from_deck(
            "stock".into(),
            shuffled_deck(crate::Back::Blue),
            false,
        ));
        game_state.stacks.push(StackState::new("table".into()));

        let mut result = Self { game_state };
        result.hand_out_cards();
        result.initial_cards_to_table();

        result
    }

    pub fn hand_out_cards(&mut self) {
        for player in 0..self.game_state.players.len() {
            CardAction::new()
                .from_stack_top(&self.game_state, 0, 4)
                .to_hand(&self.game_state, player)
                .rotate(CardRotation::FaceUp)
                .apply(&mut self.game_state);
        }
    }

    pub fn initial_cards_to_table(&mut self) {
        CardAction::new()
            .from_stack_top(&self.game_state, 0, 4)
            .to_stack_top(&self.game_state, 1)
            .rotate(CardRotation::FaceUp)
            .apply(&mut self.game_state);
    }
}
