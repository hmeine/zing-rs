use rand::{thread_rng, Rng};
use crate::zing_game::ZingGame;

pub trait ZingAI {
    fn auto_play(&self, game: &mut ZingGame);
}

pub struct RandomPlayer {
    player_index: usize,
}

impl RandomPlayer {
    pub fn new(player_index: usize) -> Self {
        Self { player_index }
    }
}

impl ZingAI for RandomPlayer {
    fn auto_play(&self, game: &mut ZingGame) {
        game.play_card(
            self.player_index,
            thread_rng().gen_range(0..game.state().players[self.player_index].hand.len()),
        );
        
    }
}
