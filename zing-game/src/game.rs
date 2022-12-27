use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::Card;

/// Represents a named player.
#[derive(Clone, Serialize, Deserialize, Debug)]
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

/// Represents a single card (without position) with front and back side and
/// whether it is face up or face down.
#[derive(Clone, Debug, Serialize, Deserialize, Reflect)]
pub struct CardState {
    pub card: Card,
    pub face_up: bool,
}

impl CardState {
    /// Produce a face down version of this CardState, with rank/suit replaced
    /// with an arbitrary choice (ace of clubs, currently – not guaranteed to
    /// stay like that).
    pub fn covered(&self) -> Self {
        Self {
            card: Card {
                rank: crate::Rank::Ace,
                suit: crate::Suit::Clubs,
                back: self.card.back,
            },
            face_up: false,
        }
    }

    /// Return a copy of this state, but use covered() if face down. (The
    /// purpose of this method is for network communication in multi-player
    /// scenarios in which it should not be possible to "peek" below the card.)
    pub fn covered_if_face_down(&self) -> Self {
        if self.face_up {
            self.clone()
        } else {
            self.covered()
        }
    }
}

pub fn unicode(cards: &[CardState]) -> String {
    let cards = String::from_iter(itertools::intersperse(
        cards.iter().map(|card_state| card_state.card.unicode()),
        ' ',
    ));
    cards //.join(" ").collect()
}

/// Represents an ordered stack of cards, each with front and back side and
/// which side is up. Also has a stack ID (string).
#[derive(Clone, Serialize, Deserialize, Debug)]
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

    pub fn flip_cards(cards: &[CardState]) -> Vec<CardState> {
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

/// Represents a generic card game with a list of players (each of which has a
/// name and a hand of cards) and stacks on the table.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GameState {
    pub players: Vec<Player>,
    pub stacks: Vec<StackState>,
    pub phase: GamePhase,
}

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug)]
pub enum GamePhase {
    Initial,
    Prepared,
    InGame,
    Finished,
}

pub enum CardGameError {
    DrawingStackEmpty,
}

impl GameState {
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn phase(&self) -> GamePhase {
        self.phase
    }

    /// Initialize a game with the players of the given `Table`.  No card stacks
    /// are initialized.
    pub fn new_with_player_names(names: Vec<String>) -> Self {
        let mut result = Self {
            players: Vec::new(),
            stacks: Vec::new(),
            phase: GamePhase::Initial,
        };
        for name in names {
            result.players.push(Player::new(name));
        }
        result
    }

    pub fn new_view_for_player(&self, player_index: usize) -> Self {
        Self {
            players: self
                .players
                .iter()
                .enumerate()
                .map(|(i, player)| {
                    if i == player_index {
                        player.clone()
                    } else {
                        Player {
                            name: player.name.clone(),
                            hand: player.hand.iter().map(CardState::covered).collect(),
                        }
                    }
                })
                .collect(),
            stacks: self
                .stacks
                .iter()
                .map(|stack| StackState {
                    id: stack.id.clone(),
                    cards: stack
                        .cards
                        .iter()
                        .map(CardState::covered_if_face_down)
                        .collect(),
                })
                .collect(),
            phase: self.phase,
        }
    }
}
