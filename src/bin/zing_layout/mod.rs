use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_svg::prelude::*;
use zing_rs::card_action::CardLocation;

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_camera);
        app.add_startup_system(setup_card_stacks);
    }
}

/// margin at top / bottom of screen (relative to full screen height)
const MARGIN: f32 = 0.05;
/// vertical base size of all cards (relative to full screen height)
const CARD_HEIGHT: f32 = 0.23;
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
const PLAYING_CENTER_Y: f32 = (OWN_CARD_ZOOM - 1.) * CARD_HEIGHT / 2.;

// TODO: we need to consider having four players

#[derive(Component)]
struct Card;

#[derive(Component)]
struct CardStack {
    location: CardLocation,
    index: usize,
}

fn spawn_card_stack(
    commands: &mut Commands,
    top_left_position: Vec3,
    scale: Vec3,
    location: CardLocation,
    index: usize,
) -> Entity {
    commands
        .spawn_bundle(SpatialBundle {
            transform: Transform {
                translation: top_left_position,
                scale,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(CardStack { location, index })
        .id()
}

pub fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle {
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical(1.0),
            ..Default::default()
        },
        ..Default::default()
    });
}

pub fn setup_card_stacks(mut commands: Commands, asset_server: Res<AssetServer>) {
    let opposite_hand_pos_y = PLAYING_CENTER_Y + VERTICAL_SPACING + 1.5 * CARD_HEIGHT;

    let mut stacks = Vec::new();

    stacks.push(spawn_card_stack(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - FULL_HAND_WIDTH / 2.,
            opposite_hand_pos_y,
            0.,
        ),
        Vec3::ONE,
        CardLocation::PlayerHand,
        0, // FIXME: we need to know which player we are
    ));

    let own_hand_pos_y = PLAYING_CENTER_Y - 0.5 * CARD_HEIGHT - VERTICAL_SPACING;

    stacks.push(spawn_card_stack(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2.,
            own_hand_pos_y,
            0.,
        ),
        Vec3::splat(OWN_CARD_ZOOM),
        CardLocation::PlayerHand,
        1, // FIXME: we need to know which player we are
    ));

    // TODO: we need to know if we have two or four players

    stacks.push(spawn_card_stack(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - CARD_WIDTH * 2.3,
            PLAYING_CENTER_Y + CARD_HEIGHT / 2.,
            0.,
        ),
        Vec3::ONE,
        CardLocation::Stack,
        0, // "stock"
    ));

    stacks.push(spawn_card_stack(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - CARD_WIDTH / 2.,
            PLAYING_CENTER_Y + CARD_HEIGHT / 2.,
            0.,
        ),
        Vec3::ONE,
        CardLocation::Stack,
        1, // "table"
    ));

    stacks.push(spawn_card_stack(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X + FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2. + SCORE_STACK_SPACING,
            opposite_hand_pos_y,
            0.,
        ),
        Vec3::ONE,
        CardLocation::Stack,
        2, // "score_0"
    ));

    stacks.push(spawn_card_stack(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X + FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2. + SCORE_STACK_SPACING,
            own_hand_pos_y,
            0.,
        ),
        Vec3::ONE,
        CardLocation::Stack,
        3, // "score_1"
    ));

    for stack in stacks {
        let svg = asset_server.load("vector_cards_3.2/CLUB-6.svg");
        // SVG size: 238.11075, 332.5986
        let scale = CARD_HEIGHT / 332.5986;

        let card = commands
            .spawn_bundle(Svg2dBundle {
                svg,
                transform: Transform {
                    scale: Vec3::new(scale, scale, 1.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Card)
            .id();
        commands.entity(stack).push_children(&[card]);
    }

    //    let svg = asset_server.load("vector_cards_3.2/BACK-BLUE.svg"));
}
