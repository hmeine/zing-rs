use crate::{
    card_action::{CardAction, CardLocation, CardRotation},
    decks::shuffled_deck,
    game::{GameState, StackState},
};

pub struct ZingGame {
    game_state: GameState,
    turn: usize,
}

impl ZingGame {
    pub fn new_from_table(table: crate::table::Table, first_turn: usize) -> Self {
        let mut game_state = GameState::new_from_table(table);
        game_state.stacks.push(StackState::new_from_deck(
            "stock".into(),
            shuffled_deck(crate::Back::Blue),
            false,
        ));
        game_state.stacks.push(StackState::new("table".into()));

        game_state.stacks.push(StackState::new("score_0".into()));
        game_state.stacks.push(StackState::new("score_1".into()));

        let mut result = Self {
            game_state,
            turn: first_turn,
        };
        
        result.hand_out_cards();
        result.initial_cards_to_table();

        result
    }

    pub fn state(&self) -> &GameState {
        &self.game_state
    }

    pub fn apply(&mut self, action: CardAction) {
        action.apply(&mut self.game_state);
    }

    // TODO: return actions, incl. automatic actions; requires card offset (table stack is +1 after this)
    pub fn play_card(&mut self, player: usize, card_index: usize) {
        assert!(player == self.turn);

        CardAction::new()
            .from_hand(&self.game_state, player, vec![card_index])
            .to_stack_top(&self.game_state, 1)
            .rotate(CardRotation::FaceUp)
            .apply(&mut self.game_state);
    }

    pub fn hand_out_cards(&mut self) {
        for player in 0..self.game_state.player_count() {
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

    pub fn is_valid_action(&self, action: &CardAction) -> bool {
        match action.source_location {
            Some(CardLocation::PlayerHand) => {
                (action.source_index == self.turn)
                    && (action.source_card_indices.len() == 1)
                    && (*action.source_card_indices.first().unwrap()
                        < self.game_state.players[self.turn].hand.len())
            }
            _ => false,
        }
    }

    pub fn auto_actions(&mut self) {
        let table_stack = &self.game_state.stacks[1];
        if let [.., card1, card2] = &table_stack.cards[..] {
            if card1.card.rank == card2.card.rank {
                let target_stack = 2 + self.turn % 2;

                if table_stack.cards.len() == 2 {
                    // Zing!
                    CardAction::new()
                        .from_stack_top(&self.game_state, 1, 1)
                        .to_stack_top(&self.game_state, target_stack)
                        .rotate(CardRotation::FaceDown)
                        .apply(&mut self.game_state);
                    CardAction::new()
                        .from_stack_top(&self.game_state, 1, 1)
                        .to_stack_bottom(&self.game_state, target_stack)
                        .rotate(CardRotation::FaceUp)
                        .apply(&mut self.game_state);
                } else {
                    CardAction::new()
                        .from_stack_top(&self.game_state, 1, table_stack.cards.len())
                        .to_stack_top(&self.game_state, target_stack)
                        .rotate(CardRotation::FaceDown)
                        .apply(&mut self.game_state);
                }
            }
        }

        if self
            .game_state
            .players
            .iter()
            .all(|player| player.hand.is_empty())
        {
            self.hand_out_cards();
        }
    }
}
