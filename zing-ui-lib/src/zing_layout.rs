use std::time::Duration;

use crate::app_state::AppState;
use crate::card_sprite::CardSprite;
use crate::constants::*;
use crate::game_logic::{GameLogic, StateChange, TasksRuntime};
use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_tweening::*;
use zing_game::card_action::CardAction;
use zing_game::game::{GamePhase, GameState};
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

#[derive(Event)]
struct InitialGameStateEvent {
    pub game_state: GameState,
    pub we_are_player: usize,
    pub table_stack_spread_out: bool,
}

#[derive(Event)]
struct CardActionEvent {
    pub action: CardAction,
}

#[derive(Component, Clone)]
struct ZoomedOnHover;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<InitialGameStateEvent>();
        app.add_event::<CardActionEvent>();
        app.init_state::<AppState>();
        app.insert_resource(LayoutState::new());

        app.add_systems(Startup, (setup_camera, setup_card_stacks));

        app.add_systems(
            Update,
            (
                get_next_action_after_animation_finished
                    .before(spawn_cards_for_initial_state)
                    .before(update_cards_from_action),
                spawn_cards_for_initial_state,
                zoom_on_hover.run_if(in_state(AppState::Interaction)),
                unzoom_after_hover.run_if(in_state(AppState::Interaction)),
                handle_keyboard_input
                    .run_if(in_state(AppState::Interaction))
                    .before(update_cards_from_action),
                update_cards_from_action,
            ),
        );

        app.add_systems(PostUpdate, reposition_cards_after_action);
    }
}

// TODO: we need to consider having four players

/// Bevy component representing a stack or hand of cards.
#[derive(Component, TypePath)]
pub struct CardStack {
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
            .spawn((
                Self {
                    location,
                    index,
                    peeping_offset,
                    score_offset,
                },
                Transform {
                    translation: top_left_position,
                    scale,
                    ..Default::default()
                },
                InheritedVisibility::VISIBLE,
            ))
            .id()
    }

    fn card_states<'a>(&self, game: &'a GameState) -> &'a Vec<CardState> {
        match self.location {
            CardLocation::PlayerHand => &game.players[self.index].hand,
            CardLocation::Stack => &game.stacks[self.index].cards,
        }
    }

    /// HACK: we_are_player is initialized to 0, and setup_card_stacks() will
    /// always think we are player 0.  In order to fix that when we receive the
    /// InitialGameStateEvent which carries our real player index, we support
    /// belated swapping of player 0 and player 1 with this method. (I do think
    /// it would make more sense to setup the card stacks only after we received
    /// the InitialGameStateEvent, but I think that must be done one tick
    /// earlier than setting up the cards, so I am playing safe for now.)
    fn swap_player(&mut self) {
        if self.location == CardLocation::Stack && self.index < 2 {
            return; // stock or table, must not change
        }
        self.index ^= 1; // FIXME would probably not work for four players
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 1.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform {
            // by default, the camera is at z=0 and only displays stuff with z<0
            // (2D frustrum culling came with 0.11)
            translation: Vec3::new(0., 0., 500.),
            ..Default::default()
        },
    ));
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

    let own_hand = CardStack::spawn(
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
    commands
        .entity(own_hand)
        .observe(card_clicked)
        .observe(|trigger: Trigger<Pointer<Over>>, mut commands: Commands| {
            commands.entity(trigger.target).insert(ZoomedOnHover);
        })
        .observe(|trigger: Trigger<Pointer<Out>>, mut commands: Commands| {
            commands.entity(trigger.target).remove::<ZoomedOnHover>();
        });

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

    const WINNING_STACK_X: f32 =
        PLAYING_CENTER_X + FULL_HAND_WIDTH * OWN_CARD_ZOOM / 2. + CARD_WIDTH / 2.;

    CardStack::spawn(
        &mut commands,
        Vec3::new(WINNING_STACK_X, opposite_hand_pos_y, 0.),
        Vec3::new(0., -VERTICAL_PEEPING, 0.),
        Vec3::ZERO,
        Vec3::ONE,
        CardLocation::Stack,
        2 + ((we_are_player + 1) % 2), // opponent's winning stack
    );

    CardStack::spawn(
        &mut commands,
        Vec3::new(WINNING_STACK_X, own_hand_pos_y, 0.),
        Vec3::new(0., VERTICAL_PEEPING, 0.),
        Vec3::ZERO,
        Vec3::ONE,
        CardLocation::Stack,
        2 + we_are_player % 2, // own winning stack
    );

    const SCORING_STACK_X: f32 = WINNING_STACK_X - SCORE_STACK_SPACING - CARD_WIDTH;

    CardStack::spawn(
        &mut commands,
        Vec3::new(
            SCORING_STACK_X,
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
        Vec3::new(SCORING_STACK_X, own_hand_pos_y, 0.),
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
    let mut peeping_offsets = (0i8..).map(move |i| f32::from(total_peeping - i) * peeping_offset);

    (0i8..)
        .zip(card_states.iter())
        .map(move |(index, card_state)| {
            card_offset * f32::from(index)
                + if card_state.face_up {
                    peeping_offsets.next().unwrap()
                } else {
                    Vec3::ZERO
                }
                + score_offset * (ZingGame::card_points(&card_state.card) as f32)
        })
}

fn get_next_action_after_animation_finished(
    mut game_logic: ResMut<GameLogic>,
    mut layout_state: ResMut<LayoutState>,
    mut initial_state_events: EventWriter<InitialGameStateEvent>,
    mut card_events: EventWriter<CardActionEvent>,
    mut next_state: ResMut<NextState<AppState>>,
    time: Res<Time>,
) {
    layout_state.step_animation_timer.tick(time.delta());
    if !layout_state.step_animation_timer.finished() {
        return;
    }

    match game_logic.get_next_state_change() {
        Some(StateChange::GameStarted(game_state, we_are_player)) => {
            let table_stack_spread_out = game_state.phase() != GamePhase::InGame;
            initial_state_events.send(InitialGameStateEvent {
                game_state,
                we_are_player,
                table_stack_spread_out,
            });
        }
        Some(StateChange::CardAction(action)) => {
            card_events.send(CardActionEvent { action });
        }
        None => {
            next_state.set(AppState::Interaction);
        }
    }
}

fn spawn_cards_for_initial_state(
    mut commands: Commands,
    mut initial_state_events: EventReader<InitialGameStateEvent>,
    mut layout_state: ResMut<LayoutState>,
    mut query_stacks: Query<(Entity, &mut CardStack)>,
    asset_server: Res<AssetServer>,
) {
    for initial_state_event in initial_state_events.read() {
        let swap_stacks = initial_state_event.we_are_player != layout_state.we_are_player;

        layout_state.displayed_state = Some(initial_state_event.game_state.clone());
        layout_state.we_are_player = initial_state_event.we_are_player;
        layout_state.table_stack_spread_out = initial_state_event.table_stack_spread_out;

        for (stack_id, mut stack) in query_stacks.iter_mut() {
            if swap_stacks {
                stack.swap_player();
            }

            let card_states = stack.card_states(layout_state.displayed_state.as_ref().unwrap());

            let card_entities: Vec<_> = card_states
                .iter()
                .zip(card_offsets_for_stack(
                    card_states,
                    &stack,
                    layout_state.table_stack_spread_out,
                ))
                .map(|(card_state, card_offset)| {
                    CardSprite::spawn(&mut commands, &asset_server, card_state, card_offset)
                })
                .collect();

            commands.entity(stack_id).add_children(&card_entities);
        }
    }
}

#[derive(Component)]
struct StackRepositioning;

fn update_cards_from_action(
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
    mut layout_state: ResMut<LayoutState>,
    mut action_events: EventReader<CardActionEvent>,
    query_stacks: Query<(Entity, &CardStack, &Transform)>,
    query_children: Query<&Children>,
    mut query_sprites: Query<(&mut CardSprite, &mut Sprite), Without<CardStack>>,
    mut query_transforms: Query<&mut Transform, (With<CardSprite>, Without<CardStack>)>,
    asset_server: Res<AssetServer>,
) {
    for card_action_event in action_events.read() {
        let action = &card_action_event.action;

        action.apply(
            layout_state
                .displayed_state
                .as_mut()
                .expect("can only update cards if displayed state is not None"),
        );

        // not very nice, but we currently have no direct access to the game phase
        if action.source_location == Some(CardLocation::PlayerHand) {
            layout_state.table_stack_spread_out = false;
        }

        // state update is finished; now update bevy entities accordingly:

        // determine old and new parent CardStacks
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
        let source_offset = source_transform.translation - target_transform.translation;
        let source_scale = source_transform.scale / target_transform.scale;

        // pick source card entities from source stack based on action indices
        let source_children: Vec<_> = query_children.iter_descendants(source_parent).collect();
        let source_cards: Vec<_> = action
            .source_card_indices
            .iter()
            .map(|i| source_children[*i])
            .collect();

        // remove from source_parent, reposition source stack if necessary
        commands
            .entity(source_parent)
            .remove_children(&source_cards);
        if let Some(CardLocation::Stack) = action.source_location {
            commands.entity(source_parent).insert(StackRepositioning);
        }

        // possibly change card state (face up/down rotation)
        for (entity, card_state) in source_cards.iter().zip(&action.resulting_card_states) {
            let (mut card, mut sprite) = query_sprites.get_mut(*entity).unwrap();
            if card.0.face_up != card_state.face_up {
                card.change_state(&mut sprite, &asset_server, card_state);
            }
        }

        // modify transform to reflect old position with new parent
        for card in &source_cards {
            let transform = &mut query_transforms.get_mut(*card).unwrap();
            transform.translation += source_offset;
            transform.scale *= source_scale;
        }

        // add below target_parent, reposition stack accordingly
        commands
            .entity(target_parent)
            .insert_children(*action.dest_card_indices.first().unwrap(), &source_cards)
            .insert(StackRepositioning);

        layout_state.step_animation_timer.reset();
        next_state.set(AppState::AnimationActive);
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TransformPositionScaleLens {
    /// Start value of the translation.
    pub start_position: Vec3,
    /// End value of the translation.
    pub end_position: Vec3,
    /// Start value of the scale.
    pub start_scale: Vec3,
    /// End value of the scale.
    pub end_scale: Vec3,
}

impl Lens<Transform> for TransformPositionScaleLens {
    fn lerp(&mut self, target: &mut dyn Targetable<Transform>, ratio: f32) {
        let value = self.start_position + (self.end_position - self.start_position) * ratio;
        target.translation = value;
        let value = self.start_scale + (self.end_scale - self.start_scale) * ratio;
        target.scale = value;
    }
}

fn reposition_cards_after_action(
    mut commands: Commands,
    layout_state: Res<LayoutState>,
    query_stacks: Query<(Entity, &Children, &CardStack), With<StackRepositioning>>,
    mut query_transform: Query<&mut Transform>,
) {
    let target_scale = CardSprite::default_scale();

    for (entity, children, stack) in &query_stacks {
        for (pos, card) in card_offsets_for_stack(
            stack.card_states(layout_state.displayed_state.as_ref().unwrap()),
            stack,
            layout_state.table_stack_spread_out,
        )
        .zip(children)
        {
            let old_transform = &mut query_transform.get_mut(*card).unwrap();

            if old_transform.translation.x != pos.x
                || old_transform.translation.y != pos.y
                || ((old_transform.scale.x / target_scale.x) - 1.0).abs() > 0.01
            {
                // TODO: some cards "fly through" stacks, but should be on top
                // (others must not get a large Z value, though)
                commands.entity(*card).insert(Animator::new(Tween::new(
                    EaseFunction::QuadraticInOut,
                    Duration::from_millis(ANIMATION_MILLIS),
                    TransformPositionScaleLens {
                        start_position: old_transform.translation,
                        end_position: pos,
                        start_scale: old_transform.scale,
                        end_scale: target_scale,
                    },
                )));
            } else {
                // we do not want to animate pure z changes:
                old_transform.translation.z = pos.z;
            }
        }

        commands.entity(entity).remove::<StackRepositioning>();
    }
}

pub fn handle_keyboard_input(
    mut game_logic: ResMut<GameLogic>,
    runtime: ResMut<TasksRuntime>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let mut play_card = None;
    if keyboard_input.just_pressed(KeyCode::Digit1) {
        play_card = Some(0);
    } else if keyboard_input.just_pressed(KeyCode::Digit2) {
        play_card = Some(1);
    } else if keyboard_input.just_pressed(KeyCode::Digit3) {
        play_card = Some(2);
    } else if keyboard_input.just_pressed(KeyCode::Digit4) {
        play_card = Some(3);
    }

    if let Some(card_index) = play_card {
        game_logic.play_card(runtime, card_index);
    }
}

pub fn card_clicked(
    click: Trigger<Pointer<Click>>,
    layout_state: ResMut<LayoutState>,
    query_stacks: Query<(Entity, &Children)>,
    mut game_logic: ResMut<GameLogic>,
    runtime: ResMut<TasksRuntime>,
) {
    if !layout_state.step_animation_timer.finished() {
        return;
    }
    if click.button != PointerButton::Primary {
        return;
    }

    let mut play_card = None;

    for (_entity, children) in &query_stacks {
        for (card_index, card) in children.iter().enumerate() {
            if *card == click.target {
                play_card = Some(card_index);
            }
        }
    }

    if let Some(card_index) = play_card {
        debug!("clicked card {}", card_index);
        game_logic.play_card(runtime, card_index);
    }
}

fn zoom_on_hover(
    mut interaction_query: Query<&mut Transform, (Added<ZoomedOnHover>, With<CardSprite>)>,
) {
    // if !layout_state.step_animation_timer.finished() {
    //     return;
    // }
    for mut transform in &mut interaction_query {
        transform.scale *= HOVER_ZOOM;
    }
}

fn unzoom_after_hover(
    mut transform_query: Query<&mut Transform>,
    mut cards: RemovedComponents<ZoomedOnHover>,
) {
    // if !layout_state.step_animation_timer.finished() {
    //     return;
    // }
    for id in cards.read() {
        if let Ok(mut transform) = transform_query.get_mut(id) {
            transform.scale /= HOVER_ZOOM;
        }
    }
}
