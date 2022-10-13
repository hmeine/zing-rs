use crate::table::Table;
use crate::{Back, Card};

pub struct Player {
    pub name: String,
    /// The difference between player hands and other stacks of cards is that
    /// other players are never able to see these cards, even if they're face
    /// up.
    pub hand: Vec<CardState>,
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

pub fn unicode(cards: &Vec<CardState>) -> String {
    let cards = String::from_iter(itertools::intersperse(
        cards.iter().map(|card_state| card_state.card.unicode()),
        ' ',
    ));
    cards //.join(" ").collect()
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
    pub id: String,
    pub cards: Vec<CardState>,
}

impl StackState {
    pub fn new(id: String) -> Self {
        Self {
            id,
            cards: Vec::new(),
        }
    }

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
        cards
            .iter()
            .rev()
            .map(|card_state| CardState {
                card: card_state.card,
                face_up: !card_state.face_up,
            })
            .collect()
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

    pub fn player_count(&self) -> usize {
        self.players.len()
    }
}
