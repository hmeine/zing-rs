use std::time::Duration;

use bevy::prelude::*;
use zing_game::zing_game::ZingGame;

use crate::constants::STEP_DURATION_MILLIS;

pub struct GameState {
    pub game: ZingGame,
    pub we_are_player: usize,
    pub last_synced_history_len: usize,
    pub displayed_state: zing_game::game::GameState,
    pub step_animation_timer: Timer,
}

impl GameState {
    pub fn new(game: ZingGame, we_are_player: usize) -> Self {
        let initial_state = game.state().new_view_for_player(we_are_player);
        let initial_history_len = game.history().len();

        Self {
            game,
            we_are_player,
            last_synced_history_len: initial_history_len,
            displayed_state: initial_state,
            step_animation_timer: Timer::new(Duration::from_millis(STEP_DURATION_MILLIS), false),
        }
    }
}

pub fn handle_keyboard_input(
    mut game_state: ResMut<GameState>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if !game_state.step_animation_timer.finished() {
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
        let game = &mut game_state.game;
        let player_index = game.current_player();
        let hand_size = game.state().players[player_index].hand.len();
        if card_index < hand_size {
            game.play_card(player_index, card_index);
        }
    }
}
