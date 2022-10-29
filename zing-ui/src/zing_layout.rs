use std::time::Duration;

use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_tweening::lens::TransformPositionLens;
use bevy_tweening::{Animator, EaseFunction, Tween, TweeningPlugin, TweeningType};
use zing_game::card_action::CardRotation;
use zing_game::zing_ai::{RandomPlayer, ZingAI};
use zing_game::{card_action::CardLocation, game::CardState, Back, Rank, Suit};
use zing_game::{table::Table, zing_game::ZingGame};

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(TweeningPlugin);
        app.add_startup_system(setup_camera);
        app.add_startup_system(setup_card_stacks);

        app.add_startup_system(setup_random_game);
        app.add_startup_system_to_stage(StartupStage::PostStartup, spawn_cards_for_game_state);

        //app.add_system(perform_random_action.before(update_cards_from_game_state));
        app.add_system(handle_keyboard_input.before(update_cards_from_game_state));
        app.add_system(update_cards_from_game_state);
        app.add_system_to_stage(CoreStage::PostUpdate, reposition_cards_after_action);
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
const PLAYING_CENTER_X: f32 = -0.1 * CARD_WIDTH;
/// reserving some space at the bottom for the zoomed in own hand, the center should be above 0
const PLAYING_CENTER_Y: f32 = (OWN_CARD_ZOOM - 1.) * CARD_HEIGHT / 2.;

// TODO: we need to consider having four players

#[derive(Component)]
struct Card(CardState);

impl Card {
    fn png_path(card_state: &CardState) -> String {
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

    fn spawn_bundle(
        commands: &mut Commands,
        asset_server: &Res<AssetServer>,
        card_state: &CardState,
        translation: Vec3,
    ) -> Entity {
        let png_path = Self::png_path(card_state);
        let png = asset_server.load(&png_path);
        let scale = CARD_HEIGHT / 559.;

        commands
            .spawn_bundle(SpriteBundle {
                texture: png,
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

    fn card_states<'a>(&self, game: &'a zing_game::game::GameState) -> &'a Vec<CardState> {
        match self.location {
            CardLocation::PlayerHand => &game.players[self.index].hand,
            CardLocation::Stack => &game.stacks[self.index].cards,
        }
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
    let opposite_hand_pos_y = PLAYING_CENTER_Y + VERTICAL_SPACING + CARD_HEIGHT;

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

    let own_hand_pos_y =
        PLAYING_CENTER_Y - 0.5 * CARD_HEIGHT - 0.5 * OWN_CARD_ZOOM * CARD_HEIGHT - VERTICAL_SPACING;

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
        Vec3::new(PLAYING_CENTER_X - CARD_WIDTH * 2.3, PLAYING_CENTER_Y, 0.),
        Vec3::new(-HORIZONTAL_PEEPING, 0., 0.),
        Vec3::ONE,
        CardLocation::Stack,
        0, // "stock"
    );

    CardStack::spawn_bundle(
        &mut commands,
        Vec3::new(PLAYING_CENTER_X - CARD_WIDTH / 2., PLAYING_CENTER_Y, 0.),
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
struct GameState {
    game: ZingGame,
    auto_play_timer: Timer,
    last_synced_history_len: usize,
    displayed_state: zing_game::game::GameState,
    step_animation_timer: Timer,
}

fn setup_random_game(mut commands: Commands) {
    let table = Table {
        players: vec![
            zing_game::table::Player {
                name: "Hans".into(),
            },
            zing_game::table::Player {
                name: "Darko".into(),
            },
        ],
    };
    let game = ZingGame::new_from_table(table, 1);
    let initial_state = game.state().clone();

    commands.insert_resource(GameState {
        game,
        auto_play_timer: Timer::new(Duration::from_millis(400), true),
        last_synced_history_len: 0,
        displayed_state: initial_state,
        step_animation_timer: Timer::new(Duration::from_millis(900), false),
    });
}

fn perform_random_action(mut game_state: ResMut<GameState>, time: Res<Time>) {
    let timer = &mut game_state.auto_play_timer;
    timer.tick(time.delta());

    if timer.just_finished() {
        let game = &mut game_state.game;

        let player = RandomPlayer::new(game.current_player());
        player.auto_play(game);

        if game.finished() {
            game_state.auto_play_timer.pause();
        }
    }
}

fn handle_keyboard_input(mut game_state: ResMut<GameState>, keyboard_input: Res<Input<KeyCode>>) {
    let mut play_card = None;
    if keyboard_input.just_pressed(KeyCode::Key1) {
        play_card = Some(0);
    } else if keyboard_input.just_pressed(KeyCode::Key2) {
        play_card = Some(1);
    } else if keyboard_input.just_pressed(KeyCode::Key3) {
        play_card = Some(2);
    } else if keyboard_input.just_pressed(KeyCode::Key4) {
        play_card = Some(3);
    }

    if let Some(card_index) = play_card {
        let game = &mut game_state.game;
        let player_index = game.current_player();
        let hand_size = game.state().players[player_index].hand.len();
        if card_index < hand_size {
            game.play_card(player_index, card_index);
        }
    }
}

fn card_offsets_for_stack<'a>(
    card_states: &'a [CardState],
    stack: &CardStack,
    in_game: bool,
) -> impl Iterator<Item = Vec3> + 'a {
    let card_offset = match stack.location {
        CardLocation::PlayerHand => HAND_CARD_OFFSET,
        CardLocation::Stack => ISOMETRIC_CARD_OFFSET,
    } + Vec3::new(0., 0., 1.);

    let peeping_offset = if stack.location == CardLocation::Stack && stack.index == 1 && in_game {
        Vec3::ZERO
    } else {
        stack.peeping_offset
    };

    let total_peeping: i8 = card_states
        .iter()
        .map(|cs| if cs.face_up { 1 } else { 0 })
        .sum();
    let mut peeping_offset = (0i8..).map(move |i| f32::from(total_peeping - i) * peeping_offset);

    (0i8..)
        .zip(card_states.iter())
        .map(move |(index, card_state)| {
            card_offset * f32::from(index)
                + if card_state.face_up {
                    peeping_offset.next().unwrap()
                } else {
                    Vec3::ZERO
                }
        })
}

fn spawn_cards_for_game_state(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    query_stacks: Query<(Entity, &CardStack)>,
    asset_server: Res<AssetServer>,
) {
    info!("assuming game state is set up, looking for stacks...");

    let game = &game_state.game;

    for (stack_id, stack) in query_stacks.iter() {
        let card_states = stack.card_states(&game_state.displayed_state);

        let card_entities: Vec<_> = card_states
            .iter()
            .zip(card_offsets_for_stack(card_states, stack, game.turn() > 0))
            .map(|(card_state, card_offset)| {
                Card::spawn_bundle(&mut commands, &asset_server, card_state, card_offset)
            })
            .collect();

        commands.entity(stack_id).push_children(&card_entities);
    }

    game_state.last_synced_history_len = game.history().len();

    game_state.game.setup_game();
}

#[derive(Component)]
struct StackRepositioning;

fn update_cards_from_game_state(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    query_stacks: Query<(Entity, &CardStack, &Children, &Transform)>,
    mut query_cards: Query<(&Card, &mut Transform), Without<CardStack>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    game_state.step_animation_timer.tick(time.delta());
    if !game_state.step_animation_timer.finished() {
        return;
    }

    let game = &game_state.game;

    if game.history().len() > game_state.last_synced_history_len {
        {
            // we need to clone in order to allow for the mutable borrow of displayed_state:
            let action = game.history()[game_state.last_synced_history_len].0.clone();

            action.apply(&mut game_state.displayed_state);
        }

        let action = &game_state.game.history()[game_state.last_synced_history_len].0;

        let mut source_parent = None;
        let mut target_parent = None;

        for (parent, card_stack, children, transform) in &query_stacks {
            if action.source_location.unwrap() == card_stack.location
                && action.source_index == card_stack.index
            {
                source_parent = Some((parent, children, transform));
            }
            if action.dest_location.unwrap() == card_stack.location
                && action.dest_index == card_stack.index
            {
                target_parent = Some((parent, children, transform));
            }
        }

        // determine translation offset between the source and destination stacks
        let (source_parent, source_children, source_transform) = source_parent.unwrap();
        let (target_parent, _target_children, target_transform) = target_parent.unwrap();
        let stack_offset = source_transform.translation - target_transform.translation;

        let mut source_cards: Vec<_> = action
            .source_card_indices
            .iter()
            .map(|i| source_children[*i])
            .collect();

        commands
            .entity(source_parent)
            .remove_children(&source_cards);
        if let Some(CardLocation::Stack) = action.source_location {
            commands.entity(source_parent).insert(StackRepositioning);
        }

        let mut do_rotation = None;
        let mut states_and_offsets = Vec::new();

        if let Some(rotation) = action.rotation {
            let face_up = match rotation {
                CardRotation::FaceUp => true,
                CardRotation::FaceDown => false,
            };

            for entity in &source_cards {
                let (card, transform) = query_cards.get(*entity).unwrap();
                states_and_offsets.push((card.0.clone(), transform.translation));
                if card.0.face_up != face_up {
                    do_rotation = Some(face_up);
                }
            }
        }

        if let Some(face_up) = do_rotation {
            for entity in source_cards {
                commands.entity(entity).despawn();
            }

            source_cards = states_and_offsets
                .iter()
                .map(|(old_state, old_pos)| {
                    Card::spawn_bundle(
                        &mut commands,
                        &asset_server,
                        &CardState {
                            face_up,
                            ..*old_state
                        },
                        *old_pos + stack_offset,
                    )
                })
                .collect();
        } else {
            for card in &source_cards {
                query_cards.get_mut(*card).unwrap().1.translation += stack_offset;
            }
        }

        commands
            .entity(target_parent)
            .insert_children(*action.dest_card_indices.first().unwrap(), &source_cards)
            .insert(StackRepositioning);

        game_state.last_synced_history_len += 1;
        game_state.step_animation_timer.reset();
    }
}

fn reposition_cards_after_action(
    mut commands: Commands,
    game_state: Res<GameState>,
    query_stacks: Query<(Entity, &Children, &CardStack), With<StackRepositioning>>,
    mut query_transform: Query<&mut Transform>,
) {
    let game = &game_state.game;

    for (entity, children, stack) in &query_stacks {
        for (pos, card) in
            card_offsets_for_stack(stack.card_states(&game_state.displayed_state), stack, game.turn() > 0).zip(children)
        {
            let old_pos = &mut query_transform.get_mut(*card).unwrap().translation;
            if old_pos.x != pos.x || old_pos.y != pos.y {
                commands.entity(*card).insert(Animator::new(Tween::new(
                    EaseFunction::QuadraticInOut,
                    TweeningType::Once,
                    Duration::from_millis(600),
                    TransformPositionLens {
                        start: *old_pos,
                        end: pos,
                    },
                )));
            } else {
                old_pos.z = pos.z;
            }
        }

        commands.entity(entity).remove::<StackRepositioning>();
    }
}
