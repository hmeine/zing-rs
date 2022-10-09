use crate::game::{CardState, GameState};

#[derive(Clone, Debug)]
pub enum CardLocation {
    PlayerHand,
    Stack,
}

#[derive(Clone, Debug)]
pub enum CardRotation {
    FaceUp,
    FaceDown,
}

#[derive(Clone, Default, Debug)]
pub struct CardAction {
    pub source_location: Option<CardLocation>,
    pub source_index: usize,
    pub source_card_indices: Vec<usize>,

    dest_location: Option<CardLocation>,
    dest_index: usize,
    dest_card_indices: Vec<usize>,

    rotation: Option<CardRotation>,
}

impl CardAction {
    pub fn new() -> Self {
        Default::default()
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

    pub fn rotate<'a>(&'a mut self, target_rotation: CardRotation) -> &'a mut Self {
        self.rotation = Some(target_rotation);
        self
    }

    pub fn apply(&self, game: &mut GameState) {
        assert_eq!(self.source_card_indices.len(), self.dest_card_indices.len());

        let source_cards: Vec<CardState> = {
            let source_stack = match self.source_location {
                Some(CardLocation::PlayerHand) => &mut game.players[self.source_index].hand,
                Some(CardLocation::Stack) => &mut game.stacks[self.source_index].cards,
                None => panic!("CardAction source not set up"),
            };

            let rotated_cards = self
                .source_card_indices
                .iter()
                .map(|source_index| CardState {
                    card: source_stack[*source_index].card,
                    face_up: match self.rotation {
                        None => source_stack[*source_index].face_up,
                        Some(CardRotation::FaceDown) => false,
                        Some(CardRotation::FaceUp) => true,
                    },
                })
                .collect();

            let mut remove_indices = self.source_card_indices.clone();
            remove_indices.sort();
            for i in remove_indices.iter().rev() {
                source_stack.remove(*i);
            }

            rotated_cards
        };

        let dest_stack = match self.dest_location {
            Some(CardLocation::PlayerHand) => &mut game.players[self.dest_index].hand,
            Some(CardLocation::Stack) => &mut game.stacks[self.dest_index].cards,
            // if we were to return an error, we would have to check before removing the cards:
            None => panic!("CardAction destination not set up"),
        };

        for (dest_index, card_state) in self.dest_card_indices.iter().zip(source_cards) {
            dest_stack.insert(*dest_index, card_state);
        }
    }
}
