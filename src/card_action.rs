use crate::game::GameState;

pub enum CardLocation {
    PlayerHand,
    Stack
}

pub enum CardRotation {
    FaceUp,
    FaceDown,
}

#[derive(Default)]
pub struct CardAction {
    source_location: Option<CardLocation>,
    source_index: usize,
    source_card_indices: Vec<usize>,

    dest_location: Option<CardLocation>,
    dest_index: usize,
    dest_card_indices: Vec<usize>,

    rotation: Option<CardRotation>
}

impl CardAction {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn from_hand<'a>(&'a mut self, game: &GameState, player: usize, card_indices: Vec<usize>) -> &'a mut Self {
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
        self.dest_card_indices = (hand.len()..hand.len()+self.source_card_indices.len()).collect();
        self
    }

    pub fn from_stack<'a>(&'a mut self, game: &GameState, stack: usize, card_indices: Vec<usize>) -> &'a mut Self {
        let cards = &game.stacks.get(stack).unwrap().cards;
        self.source_location = Some(CardLocation::Stack);
        self.source_index = stack;
        for index in card_indices.iter() {
            assert!(*index < cards.len());
        }
        self.source_card_indices = card_indices;
        self
    }

    pub fn from_stack_top<'a>(&'a mut self, game: &GameState, stack: usize, card_count: usize) -> &'a mut Self {
        let cards = &game.stacks.get(stack).unwrap().cards;
        self.from_stack(game, stack, (cards.len()-card_count..cards.len()).collect())
    }

    pub fn to_stack_top<'a>(&'a mut self, game: &GameState, stack: usize) -> &'a mut Self {
        let cards = &game.stacks.get(stack).unwrap().cards;
        self.dest_location = Some(CardLocation::Stack);
        self.dest_index = stack;
        self.dest_card_indices = (cards.len()..cards.len()+self.source_card_indices.len()).collect();
        self
    }

    pub fn rotate<'a>(&'a mut self, target_rotation: CardRotation) -> &'a mut Self {
        self.rotation = Some(target_rotation);
        self
    }
}