use strum_macros::EnumIter;

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Suit {
    Diamonds,
    Hearts,
    Spades,
    Clubs,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

#[derive(Copy, Clone, Debug)]
pub enum Back {
    Red,
    Blue,
}

#[derive(Debug)]
pub struct Card {
    rank: Rank,
    suit: Suit,
    back: Back,
}

impl Card {
    pub fn new(rank: Rank, suit: Suit, back: Back) -> Self {
        Self{rank, suit, back}
    }

    fn rank_str(&self) -> &'static str {
        match self.rank {
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
            Rank::Ace => "A",
        }    
    }    
    
    fn suit_str(&self) -> &'static str {
        match self.suit {
            Suit::Diamonds => "♦",
            Suit::Hearts => "♥",
            Suit::Spades => "♠",
            Suit::Clubs => "♣",
        }
    }

    pub fn short_str(&self) -> String {
        String::from(self.rank_str()) + self.suit_str()
    }

    fn rank_unicode_offset(&self) -> u8 {
        match self.rank {
            Rank::Ace => 1,
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten => 10,
            Rank::Jack => 11,
            Rank::Queen => 13,
            Rank::King => 14,
        }    
    }    

    pub fn unicode(&self) -> char {
        char::from_u32(match self.suit {
            Suit::Diamonds => 0x1f0c0u32,
            Suit::Hearts => 0x1f0b0u32,
            Suit::Clubs => 0x1f0a0u32,
            Suit::Spades => 0x1f0d0u32,
        } + u32::from(self.rank_unicode_offset()))
        .unwrap()
    }
}


#[cfg(test)]
mod tests {
    use super::Card;

    #[test]
    fn test_card_str() {
        let card = Card::new(super::Rank::King, super::Suit::Hearts, super::Back::Blue);
        assert_eq!(card.short_str(), "K♥");
    }

    #[test]
    fn test_card_unicode() {
        let card = Card{ rank: super::Rank::King, suit: super::Suit::Hearts, back: super::Back::Blue };
        assert_eq!(card.unicode(), '🂾');
    }
}
