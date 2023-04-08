use bevy::prelude::Resource;
use zing_game::zing_game::GamePhase;
use zing_game::{card_action::CardAction, zing_game::ZingGame};

#[derive(Resource)]
pub struct GameLogic {
    game: ZingGame,
    pub we_are_player: usize,
    last_synced_history_len: usize,
}

impl GameLogic {
    pub fn new() -> Self {
        let game = ZingGame::new_with_player_names(vec!["Hans".into(), "Darko".into()], 1);
        let history_len = game.history().len();

        Self {
            game,
            we_are_player: 0,
            last_synced_history_len: history_len,
        }
    }

    pub fn game(&self) -> &ZingGame {
        &self.game
    }

    pub fn get_next_action(&mut self) -> Option<CardAction> {
        if self.game.phase() == GamePhase::Initial {
            self.game.setup_game();
        }

        if self.game.history().len() > self.last_synced_history_len {
            let action = self.game.history()[self.last_synced_history_len]
                .new_view_for_player(self.we_are_player);
            self.last_synced_history_len += 1;
            Some(action)
        } else {
            None
        }

        // let action_rx = self.action_rx.lock().unwrap();
        // action_rx.try_recv().ok()
    }

    pub fn play_card(&mut self, card_index: usize) {
        let game = &mut self.game;
        let player_index = game.current_player(); // TODO: we_are_player
        // ignore possible failure from too high card indices:
        let _ = game.play_card(player_index, card_index);


        // let card_tx = layout_state.card_tx.lock().unwrap();

        // card_tx.send(card_index);
    }
}

