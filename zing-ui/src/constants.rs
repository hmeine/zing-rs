use bevy::prelude::Vec3;

/// margin at top / bottom of screen (relative to full screen height)
pub const MARGIN: f32 = 0.05;
/// vertical base size of all cards (relative to full screen height)
pub const CARD_HEIGHT: f32 = 0.23;
/// horizontal base size of all cards
pub const CARD_WIDTH: f32 = CARD_HEIGHT / 1.4;
/// horizontal offset for making card value visible
pub const HORIZONTAL_PEEPING: f32 = CARD_WIDTH * 0.166;
/// vertical offset for making card value visible
pub const VERTICAL_PEEPING: f32 = CARD_HEIGHT * 0.2;
/// horizontal offset for spreading out cards on player hands
pub const HAND_CARD_OFFSET_X: f32 = CARD_WIDTH * 1.14;
/// full width of four spread out cards on hand
pub const FULL_HAND_WIDTH: f32 = CARD_WIDTH + 3. * HAND_CARD_OFFSET_X;
/// additional scale factor > 1 for cards representing own player hand
pub const OWN_CARD_ZOOM: f32 = 1.15;
/// horizontal offset between (own) player hand and (own) score stack
pub const SCORE_STACK_SPACING: f32 = MARGIN;
/// horizontal offset between (own) player hand and (own) score stack
pub const SCORE_PEEPING: f32 = HORIZONTAL_PEEPING * 0.8;

/// offset for spreading out cards on player hands
pub const HAND_CARD_OFFSET: Vec3 = Vec3 {
    x: HAND_CARD_OFFSET_X,
    y: 0.,
    z: 0.,
};
/// offset for visualizing stacks of cards
pub const ISOMETRIC_CARD_OFFSET: Vec3 = Vec3 {
    x: CARD_WIDTH / 300.,
    y: CARD_WIDTH / 250.,
    z: 0.,
};

/// remaining space after subtracting three rows of cards and margins is evenly distributed:
pub const VERTICAL_SPACING: f32 = (1. - 2. * MARGIN - (2. + OWN_CARD_ZOOM) * CARD_HEIGHT) / 2.;
/// reserving some space at the right for the score stacks, the center should shift to left
pub const PLAYING_CENTER_X: f32 = -0.1 * CARD_WIDTH;
/// reserving some space at the bottom for the zoomed in own hand, the center should be above 0
pub const PLAYING_CENTER_Y: f32 = (OWN_CARD_ZOOM - 1.) * CARD_HEIGHT / 2.;

pub const ANIMATION_MILLIS: u64 = 500;
pub const STEP_DURATION_MILLIS: u64 = 700;
