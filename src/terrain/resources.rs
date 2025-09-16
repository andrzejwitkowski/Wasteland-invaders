use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct TerrainConfig {
    pub terrain_size: u32,
    pub chunk_size: u32,
    pub scale: f32,
    pub height_scale: f32,
    pub seed: u32,
    pub river_enabled: bool,
}

#[derive(Resource)]
pub struct TerrainMaterials {
    pub mountain_material: Handle<StandardMaterial>,
    pub hill_material: Handle<StandardMaterial>,
    pub plains_material: Handle<StandardMaterial>,
    pub valley_material: Handle<StandardMaterial>,
    pub water_material: Handle<StandardMaterial>, // For future river use
}

#[derive(Resource)]
pub struct TerrainChunks {
    pub chunks: HashMap<(i32, i32), Entity>,
    pub loaded_chunks: Vec<(i32, i32)>,
}

impl Default for TerrainChunks {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            loaded_chunks: Vec::new(),
        }
    }
}

#[derive(Component)]
pub struct TerrainChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub vertices: Vec<Vec3>,
    pub terrain_types: Vec<crate::terrain::noise::TerrainType>,
}

#[derive(Event)]
pub struct GenerateTerrainEvent {
    pub center_x: f32,
    pub center_z: f32,
    pub radius: u32,
}

#[derive(Event)]
pub struct TerrainGeneratedEvent {
    pub chunk_coords: Vec<(i32, i32)>,
}
