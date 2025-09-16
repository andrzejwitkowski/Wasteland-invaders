use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use super::curve_generation::calculate_curve_normals;

pub fn create_river_water_mesh(curve: &[Vec3], width: f32) -> Mesh {
    if curve.len() < 2 {
        return Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD | bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
        );
    }
    
    let normals = calculate_curve_normals(curve);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut uvs = Vec::new();
    
    let half_width = width * 0.5;
    
    // Generate vertices along both sides of the river
    for (i, (point, normal)) in curve.iter().zip(normals.iter()).enumerate() {
        let left = *point + Vec3::new(normal.x, 0.0, normal.z) * half_width;
        let right = *point - Vec3::new(normal.x, 0.0, normal.z) * half_width;
        
        // The point.y is already at the riverbed level from the carving calculation
        // Add just a tiny bit above the riverbed for the water surface
        let water_surface_height = point.y + 0.1;
        
        vertices.push([left.x, water_surface_height, left.z]);
        vertices.push([right.x, water_surface_height, right.z]);
        
        // UV coordinates for water shader
        let u_left = 0.0;
        let u_right = 1.0;
        let v = i as f32 / (curve.len() - 1) as f32;
        
        uvs.push([u_left, v]);
        uvs.push([u_right, v]);
    }
    
    // Generate indices for triangular strips
    for i in 0..(curve.len() - 1) {
        let base = i * 2;
        
        // First triangle
        indices.push(base as u32);
        indices.push((base + 2) as u32);
        indices.push((base + 1) as u32);
        
        // Second triangle
        indices.push((base + 1) as u32);
        indices.push((base + 2) as u32);
        indices.push((base + 3) as u32);
    }
    
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD | bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    );
    
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    
    mesh
}