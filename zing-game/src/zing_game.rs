use std::cmp::Ordering;

use itertools::Itertools;
use serde::Serialize;

use crate::{
    card_action::{CardAction, CardLocation, CardRotation},
    decks::shuffled_deck,
    game::{CardState, GamePhase, GameState, StackState},
    Card, Rank, Suit,
};

/// Extension of the generic card [GameState] by elements relevant for the Zing
/// rules.  It is a little unclear to me if this separation makes sense; the
/// intent was to support other games in the future. Probably, a generic version
/// of game "phases", dealers etc. can be put into GameState eventually.  For
/// now, this is catching all the rule-specific state until a more generic
/// version can be properly modeled.
pub struct ZingGame {
    game_state: GameState,
    /// index of player who deals/dealt cards in this game
    dealer: usize,
    /// number of cards actively played
    turn: usize,
    /// index of the player who last scored a stack of cards (not won a full
    /// game - better terminology needed!)
    last_winner: usize,
    /// outlier field - this is not a Zing-specific extension, but extends
    /// [GameState] via a generic history of actions performed, so it should
    /// possibly be moved
    history: Vec<CardAction>,
}

#[derive(Serialize, Clone)]
pub struct ZingGamePoints {
    pub card_points: (u32, u32),
    pub card_count_points: (u32, u32),
    pub zing_points: (u32, u32),
}

impl ZingGamePoints {
    pub fn total_points(&self) -> (u32, u32) {
        (
            self.card_points.0 + self.card_count_points.0 + self.zing_points.0,
            self.card_points.1 + self.card_count_points.1 + self.zing_points.1,
        )
    }
}

impl ZingGame {
    // TODO: names must have length 2 or eventually 4:
    pub fn new_with_player_names(names: Vec<String>, dealer: usize) -> Self {
        let mut game_state = GameState::new_with_player_names(names);

        game_state.stacks.push(StackState::new_from_deck(
            "stock".into(),
            shuffled_deck(crate::Back::Blue),
            false,
        ));

        game_state.stacks.push(StackState::new("table".into()));

        game_state.stacks.push(StackState::new("score_0".into()));
        game_state.stacks.push(StackState::new("score_1".into()));

        game_state
            .stacks
            .push(StackState::new("open_counting_0".into()));
        game_state
            .stacks
            .push(StackState::new("open_counting_1".into()));

        Self {
            game_state,
            dealer,
            turn: 0,
            last_winner: 999, // will always be overwritten; needs to be 0/1
            history: Vec::new(),
        }
    }

    pub fn setup_game(&mut self) {
        assert!(self.game_state.phase == GamePhase::Initial);
        self.hand_out_cards();
        self.show_bottom_card_of_dealer();
        self.initial_cards_to_table();
        self.game_state.phase = GamePhase::Prepared;
    }

    pub fn state(&self) -> &GameState {
        &self.game_state
    }

    pub fn turn(&self) -> usize {
        self.turn
    }

    pub fn current_player(&self) -> usize {
        (self.dealer + 1 + self.turn) % self.state().player_count()
    }

    pub fn finished(&self) -> bool {
        self.state()
            .players
            .iter()
            .all(|player| player.hand.is_empty())
    }

    pub fn history(&self) -> &Vec<CardAction> {
        &self.history
    }

    #[allow(clippy::bool_to_int_with_if)]
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

    fn zing_points(card_state: &CardState) -> u32 {
        match (card_state.face_up, card_state.card.rank) {
            (true, Rank::Jack) => 20,
            (true, _) => 10,
            (false, _) => 0,
        }
    }

    fn total_card_points(&self) -> (u32, u32) {
        let (score0, score1, open0, open1) = self.game_state.stacks[2..6]
            .iter()
            .map(|score_stack| {
                score_stack
                    .cards
                    .iter()
                    .map(|card_state| Self::card_points(&card_state.card))
                    .sum::<u32>()
            })
            .collect_tuple()
            .unwrap();
        (score0 + open0, score1 + open1)
    }

    fn total_zing_points(&self) -> (u32, u32) {
        self.game_state.stacks[2..4]
            .iter()
            .map(|score_stack| score_stack.cards.iter().map(Self::zing_points).sum())
            .collect_tuple()
            .unwrap()
    }

    fn card_count_points(&self) -> (u32, u32) {
        let card_counts: Vec<_> = self.game_state.stacks[2..6]
            .iter()
            .map(|stack| stack.cards.len())
            .collect();
        let len0 = card_counts[0] + card_counts[2];
        let len1 = card_counts[1] + card_counts[3];
        match len0.cmp(&len1) {
            Ordering::Equal => (0, 0),
            Ordering::Greater => (3, 0),
            Ordering::Less => (0, 3),
        }
    }

    pub fn points(&self) -> ZingGamePoints {
        ZingGamePoints {
            card_points: self.total_card_points(),
            card_count_points: self.card_count_points(),
            zing_points: self.total_zing_points(),
        }
    }

    fn perform_and_remember_action(&mut self, action: &CardAction) {
        if !action.source_card_indices.is_empty() {
            let mut action = action.clone();
            action.apply_and_remember_cards(&mut self.game_state);
            self.history.push(action);
        }
    }

    pub fn play_card(&mut self, player: usize, card_index: usize) -> Result<(), &'static str> {
        if player != self.current_player() {
            return Err("not player's turn");
        }

        if card_index >= self.game_state.players[player].hand.len() {
            return Err("invalid card index (exceeds player's hand)");
        }

        self.perform_and_remember_action(
            CardAction::new()
                .from_hand(&self.game_state, player, vec![card_index])
                .to_stack_top(&self.game_state, 1)
                .rotate(CardRotation::FaceUp),
        );

        self.auto_actions();

        self.turn += 1;

        match self.game_state.phase {
            GamePhase::Initial => unreachable!(),
            GamePhase::Prepared => self.game_state.phase = GamePhase::InGame,
            GamePhase::InGame => {
                if self.finished() {
                    self.game_state.phase = GamePhase::Finished
                }
            }
            GamePhase::Finished => {}
        }

        Ok(())
    }

    pub fn hand_out_cards(&mut self) {
        for _ in 0..2 {
            for i in 0..self.game_state.player_count() {
                let player = (self.dealer + i + 1) % self.game_state.player_count();
                self.perform_and_remember_action(
                    CardAction::new()
                        .from_stack_top(&self.game_state, 0, 2)
                        .to_hand(&self.game_state, player)
                        .rotate(CardRotation::FaceUp),
                );
            }
        }
    }

    pub fn show_bottom_card_of_dealer(&mut self) {
        // rotate bottom card face up (belongs to dealer, who is in advantage)
        self.perform_and_remember_action(
            CardAction::new()
                .from_stack(&self.game_state, 0, vec![0])
                .to_stack_bottom(&self.game_state, 0)
                .rotate(CardRotation::FaceUp),
        );
    }

    pub fn initial_cards_to_table(&mut self) {
        self.perform_and_remember_action(
            CardAction::new()
                .from_stack_top(&self.game_state, 0, 4)
                .to_stack_top(&self.game_state, 1)
                .rotate(CardRotation::FaceUp),
        );

        while self.game_state.stacks[1].cards.last().unwrap().card.rank == Rank::Jack {
            // put any Jack to bottom of stock, for dealer but public
            self.perform_and_remember_action(
                CardAction::new()
                    .from_stack_top(&self.game_state, 1, 1)
                    .to_stack_bottom(&self.game_state, 0)
                    .rotate(CardRotation::FaceUp),
            );
            // deal a single new card to table
            self.perform_and_remember_action(
                CardAction::new()
                    .from_stack_top(&self.game_state, 0, 1)
                    .to_stack_top(&self.game_state, 1)
                    .rotate(CardRotation::FaceUp),
            );
        }
    }

    pub fn is_valid_action(&self, action: &CardAction) -> bool {
        match action.source_location {
            Some(CardLocation::PlayerHand) => {
                (action.source_index == self.current_player())
                    && (action.source_card_indices.len() == 1)
                    && (*action.source_card_indices.first().unwrap()
                        < self.game_state.players[self.current_player()].hand.len())
            }
            _ => false,
        }
    }

    pub fn auto_actions(&mut self) {
        let table_stack = &self.game_state.stacks[1];
        if let [.., card1, card2] = &table_stack.cards[..] {
            if card1.card.rank == card2.card.rank {
                let target_score_stack = 2 + self.current_player() % 2;
                self.last_winner = target_score_stack;

                if table_stack.cards.len() == 2 {
                    // Zing!
                    self.perform_and_remember_action(
                        CardAction::new()
                            .from_stack_top(&self.game_state, 1, 1)
                            .to_stack_bottom(&self.game_state, target_score_stack)
                            .rotate(CardRotation::FaceUp),
                    );
                    self.perform_and_remember_action(
                        CardAction::new()
                            .from_stack_top(&self.game_state, 1, 1)
                            .to_stack_top(&self.game_state, target_score_stack)
                            .rotate(CardRotation::FaceDown),
                    );
                } else {
                    self.perform_and_remember_action(
                        CardAction::new()
                            .from_stack_top(&self.game_state, 1, table_stack.cards.len())
                            .to_stack_top(&self.game_state, target_score_stack)
                            .rotate(CardRotation::FaceDown),
                    );
                }
            }
        }

        let table_stack = &self.game_state.stacks[1];
        if let Some(top_card) = table_stack.cards.last() {
            if top_card.card.rank == Rank::Jack && table_stack.cards.len() > 1 {
                let target_stack = 2 + self.current_player() % 2;
                self.last_winner = target_stack;

                self.perform_and_remember_action(
                    CardAction::new()
                        .from_stack_top(&self.game_state, 1, table_stack.cards.len())
                        .to_stack_top(&self.game_state, target_stack)
                        .rotate(CardRotation::FaceDown),
                );
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
                self.perform_and_remember_action(
                    CardAction::new()
                        .from_stack_top(&self.game_state, 1, table_stack.cards.len())
                        .to_stack_top(&self.game_state, self.last_winner)
                        .rotate(CardRotation::FaceDown),
                );

                for score_index in 0..2 {
                    let score_stack = &self.state().stacks[2 + score_index].cards;
                    self.perform_and_remember_action(
                        CardAction::new()
                            .from_stack(
                                &self.game_state,
                                2 + score_index,
                                score_stack
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, cs)| !cs.face_up)
                                    .map(|(i, _)| i)
                                    .collect(),
                            )
                            .to_stack_top(&self.game_state, 4 + score_index)
                            .rotate(CardRotation::FaceUp),
                    );
                }
            }
        }
    }
}
