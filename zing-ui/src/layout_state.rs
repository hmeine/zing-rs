use std::time::Duration;

use bevy::prelude::*;
use zing_game::game::GameState;
use zing_game::zing_game::GamePhase;
use zing_game::{card_action::CardAction, zing_game::ZingGame};

use crate::constants::STEP_DURATION_MILLIS;

#[derive(Resource)]
pub struct LayoutState {
    game: ZingGame,
    pub we_are_player: usize,
    last_synced_history_len: usize,
    pub displayed_state: GameState,
    pub step_animation_timer: Timer,
}

impl LayoutState {
    pub fn new(game: ZingGame, we_are_player: usize) -> Self {
        let initial_state = game.state().new_view_for_player(we_are_player);
        let initial_history_len = game.history().len();

        Self {
            game,
            we_are_player,
            last_synced_history_len: initial_history_len,
            displayed_state: initial_state,
            step_animation_timer: Timer::new(
                Duration::from_millis(STEP_DURATION_MILLIS),
                TimerMode::Once,
            ),
        }
    }

    pub fn get_next_action(&mut self) -> Option<CardAction> {
        if self.game.phase() == GamePhase::Initial {
            self.game.setup_game();
        }

        if self.game.history().len() > self.last_synced_history_len {
            // we need to clone in order to allow for the mutable borrow of displayed_state:
            let action = self.game.history()[self.last_synced_history_len]
                .new_view_for_player(self.we_are_player);
            self.last_synced_history_len += 1;
            Some(action)
        } else {
            None
        }
    }

    pub fn phase(&self) -> GamePhase {
        self.game.phase()
    }
}

pub fn handle_keyboard_input(
    mut layout_state: ResMut<LayoutState>,
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
        let game = &mut layout_state.game;
        let player_index = game.current_player();
        // ignore possible failure from too high card indices:
        let _ = game.play_card(player_index, card_index);
    }
}
