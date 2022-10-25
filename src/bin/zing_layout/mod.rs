use bevy::{prelude::*, window::WindowResized};
use zing_rs::card_action::CardLocation;

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_card_stacks);
        app.add_system(adjust_camera_on_resize);
    }
}

/// margin at top / bottom of screen (relative to full screen height)
const MARGIN: f32 = 0.05;
/// vertical base size of all cards (relative to full screen height)
const CARD_HEIGHT: f32 = 0.25;
/// horizontal base size of all cards
const CARD_WIDTH: f32 = CARD_HEIGHT / 1.4;
/// offset for spreading out cards on player hands
const HAND_CARD_OFFSET: f32 = CARD_WIDTH * 1.14;
/// full width of four spread out cards on hand
const FULL_HAND_WIDTH: f32 = CARD_WIDTH + 3. * HAND_CARD_OFFSET;
/// additional scale factor > 1 for cards representing own player hand
const OWN_CARD_ZOOM: f32 = 1.15;
/// horizontal offset between (own) player hand and (own) score stack
const SCORE_STACK_SPACING: f32 = MARGIN;

/// remaining space after subtracting three rows of cards and margins is evenly distributed:
const VERTICAL_SPACING: f32 = (1. - 2. * MARGIN - (2. + OWN_CARD_ZOOM) * CARD_HEIGHT) / 2.;
/// reserving some space at the right for the score stacks, the center should shift to left
const PLAYING_CENTER_X: f32 = -0.6 * CARD_WIDTH;
/// reserving some space at the bottom for the zoomed in own hand, the center should be above 0
const PLAYING_CENTER_Y: f32 = (1. - OWN_CARD_ZOOM) * CARD_HEIGHT / 2.;

// TODO: we need to consider having four players

#[derive(Component)]
struct Card;

#[derive(Component)]
struct CardStack {
    location: CardLocation,
    index: usize,
}

pub fn adjust_camera_on_resize(
    mut commands: Commands,
    mut events: EventReader<WindowResized>,
    camera_query: Query<&mut OrthographicProjection>,
) {
    for ev in events.iter() {
        //layout.screen_width = ev.width;
    }
}

fn spawn_card_stack(
    commands: &mut Commands,
    top_left_position: Vec3,
    location: CardLocation,
    index: usize,
) -> Entity {
    commands
        .spawn_bundle(SpatialBundle {
            transform: Transform {
                translation: top_left_position,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(CardStack { location, index })
        .id()
}

pub fn setup_card_stacks(mut commands: Commands) {
    let opposite_hand_pos_y = PLAYING_CENTER_Y - VERTICAL_SPACING - 1.5 * CARD_HEIGHT;

    spawn_card_stack(
        &mut commands,
        Vec3::new(PLAYING_CENTER_X - FULL_HAND_WIDTH / 2., opposite_hand_pos_y, 0.),
        CardLocation::PlayerHand,
        0, // FIXME: we need to know which player we are
    );

    let own_hand_pos_y = PLAYING_CENTER_Y + 0.5 * CARD_HEIGHT + VERTICAL_SPACING;

    spawn_card_stack(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2.,
            own_hand_pos_y,
            0.,
        ),
        CardLocation::PlayerHand,
        1, // FIXME: we need to know which player we are
    );

    // TODO: we need to know if we have two or four players

    spawn_card_stack(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - CARD_WIDTH * 2.3,
            PLAYING_CENTER_Y - CARD_HEIGHT / 2.,
            0.,
        ),
        CardLocation::Stack,
        0, // "stock"
    );

    spawn_card_stack(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - CARD_WIDTH / 2.,
            PLAYING_CENTER_Y - CARD_HEIGHT / 2.,
            0.,
        ),
        CardLocation::Stack,
        1, // "table"
    );

    spawn_card_stack(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X + FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2. + SCORE_STACK_SPACING,
            opposite_hand_pos_y,
            0.,
        ),
        CardLocation::Stack,
        2, // "score_0"
    );

    spawn_card_stack(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X + FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2. + SCORE_STACK_SPACING,
            own_hand_pos_y,
            0.,
        ),
        CardLocation::Stack,
        3, // "score_1"
    );
}
