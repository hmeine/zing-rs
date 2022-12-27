use serde::{Deserialize, Serialize};

use crate::game::{CardState, GameState};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardLocation {
    PlayerHand,
    Stack,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardRotation {
    FaceUp,
    FaceDown,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct CardAction {
    pub source_location: Option<CardLocation>,
    pub source_index: usize,
    pub source_card_indices: Vec<usize>,

    pub dest_location: Option<CardLocation>,
    pub dest_index: usize,
    pub dest_card_indices: Vec<usize>,

    pub rotation: Option<CardRotation>,

    pub resulting_card_states: Vec<CardState>,
}

impl CardAction {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_view_for_player(&self, player_index: usize) -> Self {
        match self.dest_location.expect("destination not set up") {
            CardLocation::PlayerHand => {
                if self.dest_index != player_index {
                    CardAction {
                        resulting_card_states: self
                            .resulting_card_states
                            .iter()
                            .map(CardState::covered)
                            .collect(),
                        ..self.clone()
                    }
                } else {
                    self.clone()
                }
            }
            CardLocation::Stack => self.clone(),
        }
    }

    fn stack_mut(
        game: &mut GameState,
        location: CardLocation,
        index: usize,
    ) -> &mut Vec<CardState> {
        match location {
            CardLocation::PlayerHand => &mut game.players[index].hand,
            CardLocation::Stack => &mut game.stacks[index].cards,
        }
    }

    pub fn from_hand<'a>(
        &'a mut self,
        game: &GameState,
        player: usize,
        card_indices: Vec<usize>,
    ) -> &'a mut Self {
        let hand = &game.players.get(player).unwrap().hand;
        self.source_location = Some(CardLocation::PlayerHand);
        self.source_index = player;
        for index in card_indices.iter() {
            assert!(*index < hand.len());
        }
        self.source_card_indices = card_indices;
        self
    }

    pub fn to_hand<'a>(&'a mut self, game: &GameState, player: usize) -> &'a mut Self {
        let hand = &game.players.get(player).unwrap().hand;
        self.dest_location = Some(CardLocation::PlayerHand);
        self.dest_index = player;
        self.dest_card_indices =
            (hand.len()..hand.len() + self.source_card_indices.len()).collect();
        self
    }

    pub fn from_stack<'a>(
        &'a mut self,
        game: &GameState,
        stack: usize,
        card_indices: Vec<usize>,
    ) -> &'a mut Self {
        let cards = &game.stacks.get(stack).unwrap().cards;
        self.source_location = Some(CardLocation::Stack);
        self.source_index = stack;
        for index in card_indices.iter() {
            assert!(*index < cards.len());
        }
        self.source_card_indices = card_indices;
        self
    }

    pub fn from_stack_top<'a>(
        &'a mut self,
        game: &GameState,
        stack: usize,
        card_count: usize,
    ) -> &'a mut Self {
        let cards = &game.stacks.get(stack).unwrap().cards;
        self.from_stack(
            game,
            stack,
            (cards.len() - card_count..cards.len()).collect(),
        )
    }

    pub fn to_stack_top<'a>(&'a mut self, game: &GameState, stack: usize) -> &'a mut Self {
        let cards = &game.stacks.get(stack).unwrap().cards;
        self.dest_location = Some(CardLocation::Stack);
        self.dest_index = stack;
        self.dest_card_indices =
            (cards.len()..cards.len() + self.source_card_indices.len()).collect();
        self
    }

    pub fn to_stack_bottom<'a>(&'a mut self, game: &GameState, stack: usize) -> &'a mut Self {
        assert!(stack < game.stacks.len());
        self.dest_location = Some(CardLocation::Stack);
        self.dest_index = stack;
        self.dest_card_indices = (0..self.source_card_indices.len()).collect();
        self
    }

    pub fn rotate(&mut self, target_rotation: CardRotation) -> &mut Self {
        self.rotation = Some(target_rotation);
        self
    }

    pub fn apply_and_remember_cards(&mut self, game: &mut GameState) {
        self.resulting_card_states = self.apply(game);
    }

    pub fn apply(&self, game: &mut GameState) -> Vec<CardState> {
        assert_eq!(self.source_card_indices.len(), self.dest_card_indices.len());
        assert!(!self.source_card_indices.is_empty());

        let source_cards: Vec<CardState> = {
            let source_stack = Self::stack_mut(
                game,
                self.source_location.expect("CardAction source not set up"),
                self.source_index,
            );

            let rotated_cards = if !self.resulting_card_states.is_empty() {
                self.resulting_card_states.clone()
            } else {
                self.source_card_indices
                    .iter()
                    .map(|source_index| CardState {
                        card: source_stack[*source_index].card,
                        face_up: match self.rotation {
                            None => source_stack[*source_index].face_up,
                            Some(CardRotation::FaceDown) => false,
                            Some(CardRotation::FaceUp) => true,
                        },
                    })
                    .collect()
            };

            let mut remove_indices = self.source_card_indices.clone();
            remove_indices.sort_unstable();
            for i in remove_indices.iter().rev() {
                source_stack.remove(*i);
            }

            rotated_cards
        };

        let dest_stack = Self::stack_mut(
            game,
            self.dest_location
                .expect("CardAction destination not set up"),
            self.dest_index,
        );

        for (dest_index, card_state) in self.dest_card_indices.iter().zip(&source_cards) {
            dest_stack.insert(*dest_index, card_state.clone());
        }

        source_cards
    }
}
