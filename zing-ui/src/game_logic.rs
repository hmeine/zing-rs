use bevy::prelude::Resource;
use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::sync::Mutex;
use zing_game::card_action::CardAction;
use zing_game::client_notification::ClientNotification;
use zing_game::game::GameState;

#[derive(Resource)]
pub struct GameLogic {
    notifications: VecDeque<StateChange>,
    // `std::sync::mpsc::Receiver<ClientNotification>` cannot be shared between threads safely
    // because it is not Sync, i.e., multiple threads may not use it at the same time
    pub notification_rx: Mutex<Receiver<ClientNotification>>,
    // pub card_tx: Mutex<Sender<usize>>,
}

pub enum StateChange {
    GameStarted(GameState, usize),
    CardAction(CardAction),
}

impl GameLogic {
    pub fn new(notification_receiver: Receiver<ClientNotification>) -> Self {
        Self {
            notifications: VecDeque::new(),
            notification_rx: Mutex::new(notification_receiver),
            // card_tx: Mutex::new(playing_sender)
        }
    }

    pub fn get_next_state_change(&mut self) -> Option<StateChange> {
        match self
            .notification_rx
            .lock()
            .ok()
            .and_then(|notification| notification.try_recv().ok())
        {
            Some(ClientNotification::GameStarted(initial_state, we_are_player)) => self
                .notifications
                .push_back(StateChange::GameStarted(initial_state, we_are_player)),
            Some(ClientNotification::CardActions(actions)) => self
                .notifications
                .extend(actions.into_iter().map(StateChange::CardAction)),
            None => {}
        }
        self.notifications.pop_front()
    }

    pub fn play_card(&mut self, card_index: usize) {
        // let card_tx = layout_state.card_tx.lock().unwrap();
        // card_tx.send(card_index);
    }
}
