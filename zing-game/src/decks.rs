use rand::seq::SliceRandom;
use rand::thread_rng;
use strum::IntoEnumIterator;

use crate::cards::{Back, Card, Rank, Suit};

pub fn deck(backs: Back) -> Vec<Card> {
    let mut result = Vec::new();
    for suit in Suit::iter() {
        for rank in Rank::iter() {
            result.push(Card::new(rank, suit, backs))
        }
    }
    result
}

pub fn shuffled_deck(backs: Back) -> Vec<Card> {
    let mut result = deck(backs);
    result.shuffle(&mut thread_rng());
    result
}

#[cfg(test)]
mod tests {
    use super::deck;

    #[test]
    fn test_deck() {
        let deck52 = deck(crate::Back::Blue);
        assert_eq!(deck52.len(), 52);
    }
}
