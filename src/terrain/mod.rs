pub mod generation;
pub mod noise;
pub mod resources;
pub mod systems;

use bevy::prelude::*;
use resources::*;
use systems::*;

pub struct TerrainPlugin {
    pub auto_generate: bool,
    pub terrain_size: u32,
    pub chunk_size: u32,
}

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        // Insert terrain configuration
        app.insert_resource(TerrainConfig {
            terrain_size: self.terrain_size,
            chunk_size: self.chunk_size,
            scale: 1.0,
            height_scale: 200.0,
            seed: 42,
            river_enabled: false, // Start with rivers disabled
        });

        // Add terrain generation resource
        app.insert_resource(generation::TerrainGenerator::with_seed(42));

        // Add events - make sure this is done first
        app.add_event::<GenerateTerrainEvent>();
        app.add_event::<TerrainGeneratedEvent>();

        // Add systems
        app.add_systems(Startup, (
            setup_terrain_materials,
            generate_initial_terrain.run_if(resource_exists::<TerrainConfig>),
        ).chain()); // Use chain() to ensure proper ordering

        app.add_systems(Update, (
            handle_terrain_generation,
            update_terrain_chunks.after(handle_terrain_generation), // Explicit ordering
        ).run_if(resource_exists::<TerrainConfig>));
    }
}
