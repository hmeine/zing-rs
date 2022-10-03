use crate::table::Table;
use crate::{Back, Card};

pub struct Player {
    pub name: String,
    pub hand: Vec<Card>,
}

impl Player {
    pub fn new(name: String) -> Self {
        Self {
            name,
            hand: Vec::new(),
        }
    }
}

pub struct CardState {
    pub card: Card,
    pub face_up: bool,
}

pub enum CardView {
    FaceUp(Card),
    FaceDown(Back),
}

impl From<CardState> for CardView {
    fn from(card_state: CardState) -> Self {
        if card_state.face_up {
            CardView::FaceUp(card_state.card)
        } else {
            CardView::FaceDown(card_state.card.back)
        }
    }
}

pub struct StackState {
    id: String,
    pub cards: Vec<CardState>,
}

impl StackState {
    pub fn new_from_deck(id: String, deck: Vec<Card>, face_up: bool) -> Self {
        Self {
            id,
            cards: deck
                .iter()
                .map(|card| CardState {
                    card: *card,
                    face_up,
                })
                .collect(),
        }
    }

    pub fn flip_cards(cards: &Vec<CardState>) -> Vec<CardState> {
        cards.iter().rev().map(|card_state| CardState {
            card: card_state.card,
            face_up: !card_state.face_up,
        }).collect()
    }
}

pub struct GameState {
    pub players: Vec<Player>,
    pub stacks: Vec<StackState>,
}

pub enum CardGameError {
    DrawingStackEmpty,
}

impl GameState {
    /// Initialize a game with the players of the given `Table`.  No card stacks
    /// are initialized.
    pub fn new_from_table(table: Table) -> Self {
        let mut result = Self {
            players: Vec::new(),
            stacks: Vec::new(),
        };
        for player in table.players {
            result.players.push(Player::new(player.name.clone()));
        }
        result
    }

    /// Pop off `count` cards from the first stack and give them to the player's
    /// hand with the 0-based index `player`.  This will panic if there is no
    /// card stack, and it may return a `CardGameError::DrawingStackEmpty` error
    /// if the first stack does not contain at least `count` cards.
    pub fn hand_out_cards(&mut self, player: usize, count: usize) -> Result<(), CardGameError> {
        let draw_stack = self.stacks.first_mut().unwrap_or_else(|| {
            panic!("handing out cards requires at least one stack to draw from")
        });
        if draw_stack.cards.len() < count {
            return Err(CardGameError::DrawingStackEmpty);
        }

        let player_hand = &mut self.players[player].hand;

        player_hand.extend(
            draw_stack
                .cards
                .drain(draw_stack.cards.len() - count..)
                .map(|card_state| card_state.card),
        );
        Ok(())
    }

    /// Play a single card from the given player's hand to the target stack.
    /// Currently always face up.
    pub fn play_card_to_stack(&mut self, player: usize, card_index: usize, stack_index: usize) {
        let draw_stack = self
            .stacks
            .get_mut(stack_index)
            .unwrap_or_else(|| panic!("trying to play card to inexistant stack"));

        let player_hand = &mut self.players[player].hand;

        let card = player_hand.remove(card_index);

        draw_stack.cards.push(CardState {
            card,
            face_up: true,
        });
    }
}
