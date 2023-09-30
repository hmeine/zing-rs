use bevy::prelude::*;
#[cfg(feature = "bevy-inspector-egui")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

#[cfg(feature = "bevy-inspector-egui")]
use crate::{card_sprite::CardSprite, zing_layout::CardStack};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    #[cfg(feature = "bevy-inspector-egui")]
    fn build(&self, app: &mut App) {
        if cfg!(debug_assertions) {
            app.add_plugins(WorldInspectorPlugin::new());
            app.register_type::<CardStack>();
            app.register_type::<CardSprite>();
        }
    }

    #[cfg(not(feature = "bevy-inspector-egui"))]
    fn build(&self, _app: &mut App) {}
}
