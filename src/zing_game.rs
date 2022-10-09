use crate::{
    card_action::{CardAction, CardLocation, CardRotation},
    decks::shuffled_deck,
    game::{GameState, StackState},
};

pub struct ZingGame {
    pub game_state: GameState,
    pub turn: usize,
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

        let mut result = Self {
            game_state,
            turn: first_turn,
        };
        for a in result.hand_out_cards_actions() {
            a.apply(&mut result.game_state);
        }
        result
            .initial_cards_to_table_action()
            .apply(&mut result.game_state);

        result
    }

    pub fn hand_out_cards_actions(&self) -> Vec<CardAction> {
        (0..self.game_state.players.len())
            .map(
                |player| {
                    let mut action = CardAction::new();
                    action
                        .from_stack_top(&self.game_state, 0, 4)
                        .to_hand(&self.game_state, player)
                        .rotate(CardRotation::FaceUp);
                    action
                }, //.apply(&mut self.game_state);
            )
            .collect()
    }

    pub fn initial_cards_to_table_action(&self) -> CardAction {
        let mut action = CardAction::new();
        action
            .from_stack_top(&self.game_state, 0, 4)
            .to_stack_top(&self.game_state, 1)
            .rotate(CardRotation::FaceUp);
        action
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

    pub fn auto_action(&self) -> Vec<CardAction> {
        if self
            .game_state
            .players
            .iter()
            .all(|player| player.hand.is_empty())
        {
            return self.hand_out_cards_actions();
        }
        let table_stack = &self.game_state.stacks[0];
        if let [.., card1, card2] = &table_stack.cards[..] {
            if card1.card.rank == card2.card.rank {}
        }
        //        if table_stack.cards.len() >= 2 {
        //            let (card1, card2) = table_stack.cards.iter().rev().take(2);
        //        }
        Vec::new()
    }
}
