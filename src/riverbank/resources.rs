use bevy::prelude::*;

#[derive(Resource)]
pub struct RiverConfig {
    pub river_width: f32,
    pub river_depth: f32,
    pub bank_height: f32,
    pub bank_slope: f32,
    pub meander_frequency: f32,
    pub meander_amplitude: f32,
    pub flow_speed: f32,
    pub segments_per_chunk: u32,
    // New: Global river parameters
    pub global_river_direction: Vec2,
    pub global_river_start: Vec2,
}

impl Default for RiverConfig {
    fn default() -> Self {
        Self {
            river_width: 8.0,
            river_depth: 2.5, // Slightly deeper for better carving
            bank_height: 1.0,
            bank_slope: 0.5,
            meander_frequency: 0.015, // Even lower for more natural curves
            meander_amplitude: 20.0, // Slightly larger meanders
            flow_speed: 1.2,
            segments_per_chunk: 32,
            global_river_direction: Vec2::new(1.0, 0.3).normalize(), // More diagonal flow
            global_river_start: Vec2::new(-300.0, 0.0), // Start further away
        }
    }
}

#[derive(Resource, Default)]
pub struct GeneratedRiverChunks {
    pub chunks: std::collections::HashSet<(i32, i32)>,
}

// New: Global river path cache
#[derive(Resource)]
pub struct GlobalRiverPath {
    pub path_points: Vec<Vec3>,
    pub chunk_intersections: std::collections::HashMap<(i32, i32), Vec<Vec3>>,
}

impl Default for GlobalRiverPath {
    fn default() -> Self {
        Self {
            path_points: Vec::new(),
            chunk_intersections: std::collections::HashMap::new(),
        }
    }
}