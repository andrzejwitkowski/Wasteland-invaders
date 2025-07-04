use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use crate::terrain::noise::{TerrainNoise, TerrainType};
use crate::terrain::resources::*;

#[derive(Resource)]
pub struct TerrainGenerator {
    pub noise: TerrainNoise,
}

impl TerrainGenerator {
    pub fn new() -> Self {
        Self {
            noise: TerrainNoise::new(12345),
        }
    }

    pub fn with_seed(seed: u32) -> Self {
        Self {
            noise: TerrainNoise::new(seed),
        }
    }

    pub fn generate_chunk_mesh(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        chunk_size: u32,
        scale: f32,
        height_scale: f32,
    ) -> (Mesh, Vec<TerrainType>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        // let mut normals = Vec::new();
        let mut uvs = Vec::new();  // Added back UV coordinates
        let mut terrain_types = Vec::new();
 
        for z in 0..=chunk_size {
            for x in 0..=chunk_size {
                // Local coordinates within the chunk
                let local_x = x as f32 * scale;
                let local_z = z as f32 * scale;
                
                // But sample noise using world coordinates
                let world_x = chunk_x as f32 * chunk_size as f32 * scale + local_x;
                let world_z = chunk_z as f32 * chunk_size as f32 * scale + local_z;
                
                let height = self.noise.sample_terrain_height(world_x, world_z) * height_scale;
                let terrain_type = self.noise.sample_terrain_type(world_x, world_z, height / height_scale);
                
                // Store LOCAL coordinates in the mesh
                vertices.push([local_x, height, local_z]);
                // normals.push([0.0, 1.0, 0.0]);
                
                let u = x as f32 / chunk_size as f32;
                let v = z as f32 / chunk_size as f32;
                uvs.push([u, v]);
                
                terrain_types.push(terrain_type);
            }
        }

        let vertices_per_row = chunk_size + 1;
        for z in 0..chunk_size {
            for x in 0..chunk_size {
                let top_left = z * vertices_per_row + x;
                let top_right = top_left + 1;
                let bottom_left = (z + 1) * vertices_per_row + x;
                let bottom_right = bottom_left + 1;

                // First triangle
                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                // Second triangle
                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            bevy::render::render_asset::RenderAssetUsages::default(),
        );
        
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        // mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);  // Added back UV coordinates
        mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
        mesh.compute_smooth_normals();

        (mesh, terrain_types)
    }

    pub fn get_dominant_terrain_type(&self, terrain_types: &[TerrainType]) -> TerrainType {
        let mut counts = std::collections::HashMap::new();
        
        for &terrain_type in terrain_types {
            *counts.entry(terrain_type).or_insert(0) += 1;
        }
        
        counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(terrain_type, _)| terrain_type)
            .unwrap_or(TerrainType::Plains)
    }
}
