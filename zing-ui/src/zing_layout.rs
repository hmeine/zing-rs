use std::time::Duration;

use crate::card_sprite::CardSprite;
use crate::constants::*;
use crate::layout_state::{handle_keyboard_input, GamePhase, LayoutState};
use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_tweening::lens::TransformPositionLens;
use bevy_tweening::{Animator, EaseFunction, Tween};
use zing_game::game::GameState;
use zing_game::zing_game::ZingGame;
use zing_game::{card_action::CardLocation, game::CardState};

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_camera);
        app.add_startup_system(setup_card_stacks);

        app.add_startup_system_to_stage(
            StartupStage::PostStartup,
            spawn_cards_for_initial_game_state,
        );

        app.add_system(handle_keyboard_input.before(update_cards_from_action));
        app.add_system(update_cards_from_action);
        app.add_system_to_stage(CoreStage::PostUpdate, reposition_cards_after_action);
    }
}

// TODO: we need to consider having four players

#[derive(Component)]
struct CardStack {
    location: CardLocation,
    index: usize,
    peeping_offset: Vec3,
    score_offset: Vec3,
}

impl CardStack {
    fn spawn(
        commands: &mut Commands,
        top_left_position: Vec3,
        peeping_offset: Vec3,
        score_offset: Vec3,
        scale: Vec3,
        location: CardLocation,
        index: usize,
    ) -> Entity {
        commands
            .spawn(SpatialBundle {
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
                score_offset,
            })
            .id()
    }

    fn card_states<'a>(&self, game: &'a GameState) -> &'a Vec<CardState> {
        match self.location {
            CardLocation::PlayerHand => &game.players[self.index].hand,
            CardLocation::Stack => &game.stacks[self.index].cards,
        }
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical(1.0),
            ..Default::default()
        },
        ..Default::default()
    });
}

fn setup_card_stacks(mut commands: Commands, game_state: Res<LayoutState>) {
    let opposite_hand_pos_y = PLAYING_CENTER_Y + VERTICAL_SPACING + CARD_HEIGHT;

    let we_are_player = game_state.we_are_player;
    drop(game_state);

    info!("layouting card stacks");

    CardStack::spawn(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - FULL_HAND_WIDTH / 2.,
            opposite_hand_pos_y,
            0.,
        ),
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::ONE,
        CardLocation::PlayerHand,
        1 - we_are_player, // opponent's hand
    );

    let own_hand_pos_y =
        PLAYING_CENTER_Y - 0.5 * CARD_HEIGHT - 0.5 * OWN_CARD_ZOOM * CARD_HEIGHT - VERTICAL_SPACING;

    CardStack::spawn(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X - FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2.,
            own_hand_pos_y,
            0.,
        ),
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::splat(OWN_CARD_ZOOM),
        CardLocation::PlayerHand,
        we_are_player, // own hand
    );

    // TODO: we need to know if we have two or four players

    CardStack::spawn(
        &mut commands,
        Vec3::new(PLAYING_CENTER_X - CARD_WIDTH * 2.3, PLAYING_CENTER_Y, 0.),
        Vec3::new(-HORIZONTAL_PEEPING, 0., 0.),
        Vec3::ZERO,
        Vec3::ONE,
        CardLocation::Stack,
        0, // "stock"
    );

    CardStack::spawn(
        &mut commands,
        Vec3::new(PLAYING_CENTER_X - CARD_WIDTH / 2., PLAYING_CENTER_Y, 0.),
        Vec3::new(HORIZONTAL_PEEPING, 0., 0.),
        Vec3::ZERO,
        Vec3::ONE,
        CardLocation::Stack,
        1, // "table"
    );

    CardStack::spawn(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X + FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2. + SCORE_STACK_SPACING,
            opposite_hand_pos_y,
            0.,
        ),
        Vec3::new(0., -VERTICAL_PEEPING, 0.),
        Vec3::ZERO,
        Vec3::ONE,
        CardLocation::Stack,
        2 + ((we_are_player + 1) % 2), // opponent's winning stack
    );

    CardStack::spawn(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X + FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2. + SCORE_STACK_SPACING,
            own_hand_pos_y,
            0.,
        ),
        Vec3::new(0., VERTICAL_PEEPING, 0.),
        Vec3::ZERO,
        Vec3::ONE,
        CardLocation::Stack,
        2 + we_are_player % 2, // own winning stack
    );

    CardStack::spawn(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X + FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2.,
            opposite_hand_pos_y - 2. * VERTICAL_PEEPING,
            0.,
        ),
        Vec3::new(-SCORE_PEEPING, 0., 0.),
        Vec3::new(0., VERTICAL_PEEPING, 0.),
        Vec3::ONE,
        CardLocation::Stack,
        4 + ((we_are_player + 1) % 2), // opponent's score stack
    );

    CardStack::spawn(
        &mut commands,
        Vec3::new(
            PLAYING_CENTER_X + FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2.,
            own_hand_pos_y,
            0.,
        ),
        Vec3::new(-SCORE_PEEPING, 0., 0.),
        Vec3::new(0., VERTICAL_PEEPING, 0.),
        Vec3::ONE,
        CardLocation::Stack,
        4 + we_are_player % 2, // own score stack
    );
}

fn card_offsets_for_stack<'a>(
    card_states: &'a [CardState],
    stack: &CardStack,
    phase: GamePhase,
) -> impl Iterator<Item = Vec3> + 'a {
    let card_offset = match stack.location {
        CardLocation::PlayerHand => HAND_CARD_OFFSET,
        CardLocation::Stack => {
            if stack.index < 4 {
                ISOMETRIC_CARD_OFFSET
            } else {
                Vec3::ZERO
            }
        }
    } + Vec3::new(0., 0., 1.);

    let peeping_offset = if stack.location == CardLocation::Stack
        && stack.index == 1
        && phase == GamePhase::InGame
    {
        Vec3::ZERO
    } else {
        stack.peeping_offset
    };
    let score_offset = stack.score_offset;

    let mut total_peeping: i8 = card_states.iter().map(|cs| i8::from(cs.face_up)).sum();
    if let Some(CardState { face_up: true, .. }) = card_states.last() {
        total_peeping -= 1;
    }
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
                + score_offset * (ZingGame::card_points(&card_state.card) as f32)
        })
}

fn spawn_cards_for_initial_game_state(
    mut commands: Commands,
    game_state: Res<LayoutState>,
    query_stacks: Query<(Entity, &CardStack)>,
    asset_server: Res<AssetServer>,
) {
    for (stack_id, stack) in query_stacks.iter() {
        let card_states = stack.card_states(&game_state.displayed_state);

        let card_entities: Vec<_> = card_states
            .iter()
            .zip(card_offsets_for_stack(card_states, stack, game_state.phase))
            .map(|(card_state, card_offset)| {
                CardSprite::spawn(&mut commands, &asset_server, card_state, card_offset)
            })
            .collect();

        commands.entity(stack_id).push_children(&card_entities);
    }
}

#[derive(Component)]
struct StackRepositioning;

fn update_cards_from_action(
    mut commands: Commands,
    mut game_state: ResMut<LayoutState>,
    query_stacks: Query<(Entity, &CardStack, &Transform)>,
    query_children: Query<&Children>,
    mut query_cards: Query<(&CardSprite, &mut Transform), Without<CardStack>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    game_state.step_animation_timer.tick(time.delta());
    if !game_state.step_animation_timer.finished() {
        return;
    }

    if let Some(action) = game_state.get_next_action() {
        action.apply(&mut game_state.displayed_state);

        let mut source_parent = None;
        let mut target_parent = None;

        for (parent, card_stack, transform) in &query_stacks {
            if action.source_location.unwrap() == card_stack.location
                && action.source_index == card_stack.index
            {
                source_parent = Some((parent, transform));
            }
            if action.dest_location.unwrap() == card_stack.location
                && action.dest_index == card_stack.index
            {
                target_parent = Some((parent, transform));
            }
        }

        // determine translation offset between the source and destination stacks
        let (source_parent, source_transform) = source_parent.unwrap();
        let (target_parent, target_transform) = target_parent.unwrap();
        let stack_offset = source_transform.translation - target_transform.translation;

        let source_children: Vec<_> = query_children.iter_descendants(source_parent).collect();

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

        let mut do_rotation = false;
        let mut states_and_offsets = Vec::new();

        if action.rotation.is_some() {
            for (entity, card_state) in source_cards.iter().zip(&action.resulting_card_states) {
                let (card, transform) = query_cards.get(*entity).unwrap();
                states_and_offsets.push((card_state.clone(), transform.translation));
                if card.0.face_up != card_state.face_up {
                    do_rotation = true;
                }
            }
        }

        if do_rotation {
            for entity in source_cards {
                commands.entity(entity).despawn();
            }

            source_cards = states_and_offsets
                .iter()
                .map(|(new_state, old_pos)| {
                    CardSprite::spawn(
                        &mut commands,
                        &asset_server,
                        new_state,
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

        game_state.step_animation_timer.reset();
    }
}

fn reposition_cards_after_action(
    mut commands: Commands,
    game_state: Res<LayoutState>,
    query_stacks: Query<(Entity, &Children, &CardStack), With<StackRepositioning>>,
    mut query_transform: Query<&mut Transform>,
) {
    for (entity, children, stack) in &query_stacks {
        for (pos, card) in card_offsets_for_stack(
            stack.card_states(&game_state.displayed_state),
            stack,
            game_state.phase,
        )
        .zip(children)
        {
            let old_pos = &mut query_transform.get_mut(*card).unwrap().translation;
            if old_pos.x != pos.x || old_pos.y != pos.y {
                commands.entity(*card).insert(Animator::new(Tween::new(
                    EaseFunction::QuadraticInOut,
                    Duration::from_millis(ANIMATION_MILLIS),
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
