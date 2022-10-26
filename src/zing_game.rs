use std::cmp::Ordering;

use itertools::Itertools;

use crate::{
    card_action::{CardAction, CardLocation, CardRotation},
    decks::shuffled_deck,
    game::{CardState, GameState, StackState},
    Card, Rank, Suit,
};

pub struct ZingGame {
    game_state: GameState,
    turn: usize,
    last_winner: usize,
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
            last_winner: 999, // will always be overwritten; needs to be 0/1
        };

        result.hand_out_cards();
        result.show_bottom_card_of_dealer();
        result.initial_cards_to_table();

        result
    }

    pub fn state(&self) -> &GameState {
        &self.game_state
    }

    pub fn current_player(&self) -> usize {
        self.turn
    }

    pub fn card_points(card: &Card) -> u32 {
        match card.rank {
            Rank::Jack | Rank::Queen | Rank::King | Rank::Ace => 1,
            Rank::Ten => {
                if card.suit == Suit::Diamonds {
                    2
                } else {
                    1
                }
            }
            Rank::Two => {
                if card.suit == Suit::Clubs {
                    1
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    pub fn zing_points(card_state: &CardState) -> u32 {
        match (card_state.face_up, card_state.card.rank) {
            (true, Rank::Jack) => 20,
            (true, _) => 10,
            (false, _) => 0,
        }
    }

    pub fn total_card_points(&self) -> (u32, u32) {
        self.game_state.stacks[2..4]
            .iter()
            .map(|score_stack| {
                score_stack
                    .cards
                    .iter()
                    .map(|card_state| Self::card_points(&card_state.card))
                    .sum()
            })
            .collect_tuple()
            .unwrap()
    }

    pub fn total_zing_points(&self) -> (u32, u32) {
        self.game_state.stacks[2..4]
            .iter()
            .map(|score_stack| score_stack.cards.iter().map(Self::zing_points).sum())
            .collect_tuple()
            .unwrap()
    }

    pub fn card_count_points(&self) -> (u32, u32) {
        let len0 = self.game_state.stacks[2].cards.len();
        let len1 = self.game_state.stacks[3].cards.len();
        match len0.cmp(&len1) {
            Ordering::Equal => (0, 0),
            Ordering::Greater => (3, 0),
            Ordering::Less => (0, 3),
        }
    }

    pub fn total_points(&self) -> (u32, u32) {
        let card_points = self.total_card_points();
        let zing_points = self.total_zing_points();
        let card_count_points = self.card_count_points();
        (
            card_points.0 + card_count_points.0 + zing_points.0,
            card_points.1 + card_count_points.1 + zing_points.1,
        )
    }

    pub fn apply(&mut self, action: CardAction) {
        action.apply(&mut self.game_state);
    }

    pub fn play_card(&mut self, player: usize, card_index: usize) {
        assert!(player == self.turn);

        CardAction::new()
            .from_hand(&self.game_state, player, vec![card_index])
            .to_stack_top(&self.game_state, 1)
            .rotate(CardRotation::FaceUp)
            .apply(&mut self.game_state);

        self.auto_actions();

        self.turn = (self.turn + 1) % self.state().player_count();
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

    pub fn show_bottom_card_of_dealer(&mut self) {
        // rotate bottom card face up (belongs to dealer, who is in advantage)
        CardAction::new()
            .from_stack(&self.game_state, 0, vec![0])
            .to_stack_bottom(&self.game_state, 0)
            .rotate(CardRotation::FaceUp)
            .apply(&mut self.game_state);
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
                self.last_winner = target_stack;

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

        let table_stack = &self.game_state.stacks[1];
        if let Some(top_card) = table_stack.cards.last() {
            if top_card.card.rank == Rank::Jack {
                let target_stack = 2 + self.turn % 2;
                self.last_winner = target_stack;

                CardAction::new()
                    .from_stack_top(&self.game_state, 1, table_stack.cards.len())
                    .to_stack_top(&self.game_state, target_stack)
                    .rotate(CardRotation::FaceDown)
                    .apply(&mut self.game_state);
            }
        }

        if self
            .game_state
            .players
            .iter()
            .all(|player| player.hand.is_empty())
        {
            if !self.game_state.stacks[0].cards.is_empty() {
                self.hand_out_cards();
            } else {
                let table_stack = &self.game_state.stacks[1];
                CardAction::new()
                    .from_stack_top(&self.game_state, 1, table_stack.cards.len())
                    .to_stack_top(&self.game_state, self.last_winner)
                    .rotate(CardRotation::FaceDown)
                    .apply(&mut self.game_state);
            }
        }
    }
}
