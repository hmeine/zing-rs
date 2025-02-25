use bevy::prelude::*;
use zing_game::{game::CardState, Back, Rank, Suit};

use crate::constants::CARD_HEIGHT;

#[derive(Component, TypePath)]
pub struct CardSprite(pub CardState);

impl CardSprite {
    pub fn png_path(card_state: &CardState) -> String {
        let basename = if card_state.face_up {
            format!(
                "{}-{}",
                match card_state.card.suit {
                    Suit::Diamonds => "DIAMOND",
                    Suit::Hearts => "HEART",
                    Suit::Spades => "SPADE",
                    Suit::Clubs => "CLUB",
                },
                match card_state.card.rank {
                    Rank::Jack => "11-JACK",
                    Rank::Queen => "12-QUEEN",
                    Rank::King => "13-KING",
                    Rank::Ace => "1",
                    _ => card_state.card.rank_str(),
                }
            )
        } else {
            match card_state.card.back {
                Back::Blue => "BACK-BLUE",
                Back::Red => "BACK-RED",
            }
            .into()
        };
        format!("vector_cards_3.2/{}.png", basename)
    }

    pub fn default_scale() -> Vec3 {
        let scale = CARD_HEIGHT / 559.;
        Vec3::new(scale, scale, 1.0)
    }

    pub fn spawn(
        commands: &mut Commands,
        asset_server: &Res<AssetServer>,
        card_state: &CardState,
        translation: Vec3,
    ) -> Entity {
        let png_path = Self::png_path(card_state);
        let png = asset_server.load(png_path);

        commands
            .spawn((
                Sprite::from_image(png),
                Transform {
                    translation,
                    scale: Self::default_scale(),
                    ..Default::default()
                },
            ))
            .insert(Self(card_state.clone()))
            .id()
    }

    pub fn change_state(
        &mut self,
        card_sprite: &mut Sprite,
        asset_server: &Res<AssetServer>,
        card_state: &CardState,
    ) {
        let png_path = Self::png_path(card_state);
        card_sprite.image = asset_server.load(png_path);
        self.0 = card_state.clone();
    }
}
