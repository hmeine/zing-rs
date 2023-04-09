use std::time::Duration;

use crate::card_sprite::CardSprite;
use crate::constants::*;
use crate::game_logic::GameLogic;
use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_tweening::lens::TransformPositionLens;
use bevy_tweening::{Animator, EaseFunction, Tween};
use zing_game::card_action::CardAction;
use zing_game::game::GameState;
use zing_game::zing_game::ZingGame;
use zing_game::{card_action::CardLocation, game::CardState};

#[derive(Resource)]
pub struct LayoutState {
    /// Current displayed state of the game (not always in sync with core game logic, but will be eventually)
    pub displayed_state: Option<GameState>,
    /// We need to know the index of the player to be displayed in front ("ourselves")
    pub we_are_player: usize,
    /// Timer suppressing interactions during active animations
    pub step_animation_timer: Timer,
    /// During the game, only the topmost card put to the table is visible, but
    /// initially, they are dealt spread out
    pub table_stack_spread_out: bool,
}

impl LayoutState {
    pub fn new() -> Self {
        Self {
            displayed_state: None,
            we_are_player: 0,
            step_animation_timer: Timer::new(
                Duration::from_millis(STEP_DURATION_MILLIS),
                TimerMode::Once,
            ),
            table_stack_spread_out: false,
        }
    }
}

pub struct LayoutPlugin;

struct InitialGameStateEvent {
    pub game_state: GameState,
    pub we_are_player: usize,
    pub table_stack_spread_out: bool,
}

struct CardActionEvent {
    pub action: CardAction,
    pub table_stack_spread_out: bool,
}

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<InitialGameStateEvent>();
        app.add_event::<CardActionEvent>();
        app.insert_resource(LayoutState::new());

        app.add_startup_system(setup_camera);
        app.add_startup_system(setup_card_stacks);

        app.add_system(handle_keyboard_input.before(update_cards_from_action));
        app.add_system(get_next_action_after_animation_finished);
        app.add_system(spawn_cards_for_initial_state);
        app.add_system(update_cards_from_action);
        app.add_system(reposition_cards_after_action.in_base_set(CoreSet::PostUpdate));
    }
}

// TODO: we need to consider having four players

/// Bevy component representing a stack or hand of cards.
#[derive(Component)]
struct CardStack {
    /// CardStack may represent either a stack or a hand of cards
    location: CardLocation,
    /// Index of player ([CardLocation::PlayerHand]) or stack ([CardLocation::Stack])
    index: usize,
    /// Offset applied to cards that are face up in order to make them stand out and recognizable
    peeping_offset: Vec3,
    /// Additional offset indicating points assigned during final counting
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

fn setup_card_stacks(mut commands: Commands, layout_state: Res<LayoutState>) {
    let opposite_hand_pos_y = PLAYING_CENTER_Y + VERTICAL_SPACING + CARD_HEIGHT;

    let we_are_player = layout_state.we_are_player;

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
    table_stack_spread_out: bool,
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

    let peeping_offset =
        if stack.location == CardLocation::Stack && stack.index == 1 && !table_stack_spread_out {
            Vec3::ZERO
        } else {
            stack.peeping_offset
        };
    let score_offset = stack.score_offset;

    // The bottommost cards have to stand out far enough to be recognizable
    let mut total_peeping: i8 = card_states.iter().map(|cs| i8::from(cs.face_up)).sum();
    if let Some(CardState { face_up: true, .. }) = card_states.last() {
        // We do not need a peeping offset for the topmost card:
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

fn spawn_cards_for_initial_state(
    mut commands: Commands,
    mut initial_state_events: EventReader<InitialGameStateEvent>,
    mut layout_state: ResMut<LayoutState>,
    query_stacks: Query<(Entity, &CardStack)>,
    asset_server: Res<AssetServer>,
) {
    for initial_state_event in initial_state_events.into_iter() {
        layout_state.displayed_state = Some(initial_state_event.game_state.clone());
        layout_state.we_are_player = initial_state_event.we_are_player;
        layout_state.table_stack_spread_out = initial_state_event.table_stack_spread_out;

        for (stack_id, stack) in query_stacks.iter() {
            let card_states = stack.card_states(layout_state.displayed_state.as_ref().unwrap());
    
            let card_entities: Vec<_> = card_states
                .iter()
                .zip(card_offsets_for_stack(
                    card_states,
                    stack,
                    layout_state.table_stack_spread_out,
                ))
                .map(|(card_state, card_offset)| {
                    CardSprite::spawn(&mut commands, &asset_server, card_state, card_offset)
                })
                .collect();
    
            commands.entity(stack_id).push_children(&card_entities);
        }
    }
}

fn get_next_action_after_animation_finished(
    mut game_logic: ResMut<GameLogic>,
    mut layout_state: ResMut<LayoutState>,
    mut initial_state_events: EventWriter<InitialGameStateEvent>,
    mut card_events: EventWriter<CardActionEvent>,
    time: Res<Time>,
) {
    layout_state.step_animation_timer.tick(time.delta());
    if !layout_state.step_animation_timer.finished() {
        return;
    }

    if layout_state.displayed_state.is_none() {
        initial_state_events.send(InitialGameStateEvent {
            game_state: game_logic.our_view_of_game_state(),
            we_are_player: game_logic.we_are_player(),
            table_stack_spread_out: !game_logic.game_phase_is_ingame(),
        })
    } else {
        if let Some(action) = game_logic.get_next_action() {
            card_events.send(CardActionEvent {
                action,
                table_stack_spread_out: !game_logic.game_phase_is_ingame(),
            });
        }
    }
}

#[derive(Component)]
struct StackRepositioning;

fn update_cards_from_action(
    mut commands: Commands,
    mut layout_state: ResMut<LayoutState>,
    mut action_events: EventReader<CardActionEvent>,
    query_stacks: Query<(Entity, &CardStack, &Transform)>,
    query_children: Query<&Children>,
    mut query_cards: Query<(&CardSprite, &mut Transform), Without<CardStack>>,
    asset_server: Res<AssetServer>,
) {
    for card_action_event in action_events.iter() {
        let action = &card_action_event.action;

        action.apply(
            &mut layout_state
                .displayed_state
                .as_mut()
                .expect("can only update cards if displayed state is not None"),
        );

        layout_state.table_stack_spread_out = card_action_event.table_stack_spread_out;

        // state update is finished; update entities accordingly:

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
                // TODO: we should extend card_sprite::CardSprite to allow for
                // changing the CardState and the corresponding sprite PNG, so
                // that we do not have to despawn + spawn just for that...
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

        layout_state.step_animation_timer.reset();
    }
}

fn reposition_cards_after_action(
    mut commands: Commands,
    layout_state: Res<LayoutState>,
    query_stacks: Query<(Entity, &Children, &CardStack), With<StackRepositioning>>,
    mut query_transform: Query<&mut Transform>,
) {
    for (entity, children, stack) in &query_stacks {
        for (pos, card) in card_offsets_for_stack(
            stack.card_states(&layout_state.displayed_state.as_ref().unwrap()),
            stack,
            layout_state.table_stack_spread_out,
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

pub fn handle_keyboard_input(
    layout_state: ResMut<LayoutState>,
    mut game_logic: ResMut<GameLogic>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if !layout_state.step_animation_timer.finished() {
        return;
    }

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
        game_logic.play_card(card_index);
    }
}
