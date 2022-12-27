use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use crate::{card_sprite::CardSprite, zing_layout::CardStack};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        if cfg!(debug_assertions) {
            app.add_plugins(WorldInspectorPlugin::new());
            app.register_type::<CardStack>();
            app.register_type::<CardSprite>();
        }
    }
}
