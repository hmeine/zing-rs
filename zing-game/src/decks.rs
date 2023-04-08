use rand::seq::SliceRandom;
use rand::thread_rng;
use strum::IntoEnumIterator;

use crate::cards::{Back, Card, Rank, Suit};

/// Returns a full deck of cards (all [suits](Suit), all [ranks](Rank), in
/// order) with the given back.  Currently, this always produces exactly 4*13 =
/// 52 cards, see [Rank] and [Suit] enums.
pub fn deck(backs: Back) -> Vec<Card> {
    let mut result = Vec::new();
    for suit in Suit::iter() {
        for rank in Rank::iter() {
            result.push(Card::new(rank, suit, backs))
        }
    }
    result
}

/// Returns a shuffled [deck].
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

        let first = deck52.first().unwrap();
        assert_eq!(first.back, crate::Back::Blue);

        // quick check that deck is not shuffled
        assert_eq!(first.suit, crate::Suit::Diamonds);
        assert_eq!(first.rank, crate::Rank::Two);
    }
}
