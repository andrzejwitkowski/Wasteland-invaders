use bevy::prelude::*;
use crate::riverbank::{resources::*, systems::*};

pub struct RiverBankPlugin;

impl Plugin for RiverBankPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<RiverConfig>()
            .init_resource::<GeneratedRiverChunks>()
            .init_resource::<GlobalRiverPath>()
            .add_systems(Startup, setup_river_system.before(crate::terrain::systems::generate_initial_terrain))
            .add_systems(Update, (
                generate_river_chunks,
                update_river_water,
                river_config_ui,
            ));
    }
}