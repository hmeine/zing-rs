use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_svg::prelude::*;
use rand::{thread_rng, Rng};
use zing_rs::{card_action::CardLocation, game::CardState, Back, Rank, Suit};
use zing_rs::{table::Table, zing_game::ZingGame};

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_camera);
        app.add_startup_system(setup_card_stacks);

        app.add_startup_system(setup_random_game);
        app.add_startup_system_to_stage(StartupStage::PostStartup, show_game_state);
    }
}

/// margin at top / bottom of screen (relative to full screen height)
const MARGIN: f32 = 0.05;
/// vertical base size of all cards (relative to full screen height)
const CARD_HEIGHT: f32 = 0.23;
/// horizontal base size of all cards
const CARD_WIDTH: f32 = CARD_HEIGHT / 1.4;
/// horizontal offset for making card value visible
const HORIZONTAL_PEEPING: f32 = CARD_WIDTH * 0.166;
/// vertical offset for making card value visible
const VERTICAL_PEEPING: f32 = CARD_HEIGHT * 0.2;
/// horizontal offset for spreading out cards on player hands
const HAND_CARD_OFFSET_X: f32 = CARD_WIDTH * 1.14;
/// full width of four spread out cards on hand
const FULL_HAND_WIDTH: f32 = CARD_WIDTH + 3. * HAND_CARD_OFFSET_X;
/// additional scale factor > 1 for cards representing own player hand
const OWN_CARD_ZOOM: f32 = 1.15;
/// horizontal offset between (own) player hand and (own) score stack
const SCORE_STACK_SPACING: f32 = MARGIN;

/// offset for spreading out cards on player hands
const HAND_CARD_OFFSET: Vec3 = Vec3 {
    x: HAND_CARD_OFFSET_X,
    y: 0.,
    z: 0.,
};
/// offset for visualizing stacks of cards
const ISOMETRIC_CARD_OFFSET: Vec3 = Vec3 {
    x: CARD_WIDTH / 300.,
    y: CARD_WIDTH / 250.,
    z: 0.,
};

/// remaining space after subtracting three rows of cards and margins is evenly distributed:
const VERTICAL_SPACING: f32 = (1. - 2. * MARGIN - (2. + OWN_CARD_ZOOM) * CARD_HEIGHT) / 2.;
/// reserving some space at the right for the score stacks, the center should shift to left
const PLAYING_CENTER_X: f32 = -0.6 * CARD_WIDTH;
/// reserving some space at the bottom for the zoomed in own hand, the center should be above 0
const PLAYING_CENTER_Y: f32 = (OWN_CARD_ZOOM - 1.) * CARD_HEIGHT / 2.;

// TODO: we need to consider having four players

#[derive(Component)]
struct Card(CardState);

impl Card {
    fn svg_path_and_height(card_state: &CardState) -> (String, f32) {
        let (basename, height) = if card_state.face_up {
            (
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
                ),
                332.6,
            )
        } else {
            (
                match card_state.card.back {
                    Back::Blue => "BACK-BLUE",
                    Back::Red => "BACK-RED",
                }
                .into(),
                88.,
            )
        };
        (format!("vector_cards_3.2/{}.svg", basename), height)
    }

    fn spawn_bundle(
        commands: &mut Commands,
        asset_server: &Res<AssetServer>,
        card_state: &CardState,
        translation: Vec3,
    ) -> Entity {
        let (svg_path, svg_height) = Self::svg_path_and_height(card_state);
        let svg = asset_server.load(&svg_path);
        let scale = CARD_HEIGHT / svg_height;

        commands
            .spawn_bundle(Svg2dBundle {
                svg,
                transform: Transform {
                    translation,
                    scale: Vec3::new(scale, scale, 1.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Self(card_state.clone()))
            .id()
    }
}

#[derive(Component)]
struct CardStack {
    location: CardLocation,
    index: usize,
    peeping_offset: Vec3,
}

impl CardStack {
    fn spawn_bundle(
        commands: &mut Commands,
        top_left_position: Vec3,
        peeping_offset: Vec3,
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
            .insert(Self {
                location,
                index,
                peeping_offset,
            })
            .id()
    }
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

pub fn setup_card_stacks(mut commands: Commands) {
    let opposite_hand_pos_y = PLAYING_CENTER_Y + VERTICAL_SPACING + 1.5 * CARD_HEIGHT;

    info!("layouting card stacks");

    CardStack::spawn_bundle(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - FULL_HAND_WIDTH / 2.,
            opposite_hand_pos_y,
            0.,
        ),
        Vec3::ZERO,
        Vec3::ONE,
        CardLocation::PlayerHand,
        0, // FIXME: we need to know which player we are
    );

    let own_hand_pos_y = PLAYING_CENTER_Y - 0.5 * CARD_HEIGHT - VERTICAL_SPACING;

    CardStack::spawn_bundle(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2.,
            own_hand_pos_y,
            0.,
        ),
        Vec3::ZERO,
        Vec3::splat(OWN_CARD_ZOOM),
        CardLocation::PlayerHand,
        1, // FIXME: we need to know which player we are
    );

    // TODO: we need to know if we have two or four players

    CardStack::spawn_bundle(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - CARD_WIDTH * 2.3,
            PLAYING_CENTER_Y + CARD_HEIGHT / 2.,
            0.,
        ),
        Vec3::new(-HORIZONTAL_PEEPING, 0., 0.),
        Vec3::ONE,
        CardLocation::Stack,
        0, // "stock"
    );

    CardStack::spawn_bundle(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - CARD_WIDTH / 2.,
            PLAYING_CENTER_Y + CARD_HEIGHT / 2.,
            0.,
        ),
        Vec3::new(HORIZONTAL_PEEPING, 0., 0.),
        Vec3::ONE,
        CardLocation::Stack,
        1, // "table"
    );

    CardStack::spawn_bundle(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X + FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2. + SCORE_STACK_SPACING,
            opposite_hand_pos_y,
            0.,
        ),
        Vec3::new(0., -VERTICAL_PEEPING, 0.),
        Vec3::ONE,
        CardLocation::Stack,
        2, // "score_0"
    );

    CardStack::spawn_bundle(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X + FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2. + SCORE_STACK_SPACING,
            own_hand_pos_y,
            0.,
        ),
        Vec3::new(0., VERTICAL_PEEPING, 0.),
        Vec3::ONE,
        CardLocation::Stack,
        3, // "score_1"
    );
}

#[derive(Component)]
struct GameState(ZingGame);

fn setup_random_game(mut commands: Commands) {
    let table = Table {
        players: vec![
            zing_rs::table::Player {
                name: "Hans".into(),
            },
            zing_rs::table::Player {
                name: "Darko".into(),
            },
        ],
    };
    let mut game = ZingGame::new_from_table(table, 1);

    for _i in 0..19 {
        let player = game.current_player();
        game.play_card(
            player,
            thread_rng().gen_range(0..game.state().players[player].hand.len()),
        );
    }

    commands.insert_resource(GameState(game));
}

fn show_game_state(
    mut commands: Commands,
    game_state: Res<GameState>,
    query_stacks: Query<(Entity, &CardStack)>,
    asset_server: Res<AssetServer>,
) {
    info!("assuming game state is set up, looking for stacks...");

    let game = &game_state.0;

    for (stack_id, stack) in query_stacks.iter() {
        let (card_states, card_offset) = match stack.location {
            CardLocation::PlayerHand => (&game.state().players[stack.index].hand, HAND_CARD_OFFSET),
            CardLocation::Stack => (
                &game.state().stacks[stack.index].cards,
                ISOMETRIC_CARD_OFFSET,
            ),
        };

        let peeping_offset =
            if stack.location == CardLocation::Stack && stack.index == 1 && game.turn() > 0 {
                Vec3::ZERO
            } else {
                stack.peeping_offset
            };
        let total_peeping: i8 = card_states
            .iter()
            .map(|cs| if cs.face_up { 1 } else { 0 })
            .sum();
        let mut peeping_offset = (0i8..).map(|i| f32::from(total_peeping - i) * peeping_offset);

        let card_entities: Vec<_> = (0i8..)
            .zip(card_states.iter())
            .map(|(index, card_state)| {
                Card::spawn_bundle(
                    &mut commands,
                    &asset_server,
                    card_state,
                    card_offset * f32::from(index)
                        + Vec3::new(0., 0., f32::from(index))
                        + if card_state.face_up {
                            peeping_offset.next().unwrap()
                        } else {
                            Vec3::ZERO
                        },
                )
            })
            .collect();

        commands.entity(stack_id).push_children(&card_entities);
    }
}
