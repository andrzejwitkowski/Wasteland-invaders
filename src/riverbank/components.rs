use bevy::prelude::*;

#[derive(Component)]
pub struct RiverChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

#[derive(Component)]
pub struct RiverWater;

#[derive(Component)]
pub struct RiverBank;

#[derive(Component)]
pub struct RiverFlow {
    pub direction: Vec3,
    pub speed: f32,
}