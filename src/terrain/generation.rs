use bevy::prelude::*;
use crate::terrain::noise::{TerrainNoise, TerrainType};
use crate::riverbank::{GlobalRiverPath, RiverConfig, get_river_height_modifier_detailed};

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
        global_river_path: Option<&GlobalRiverPath>,
        river_config: Option<&RiverConfig>,
    ) -> (Mesh, Vec<TerrainType>) {
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut terrain_types = Vec::new();

        // Calculate chunk world position
        let chunk_world_x = chunk_x as f32 * chunk_size as f32;
        let chunk_world_z = chunk_z as f32 * chunk_size as f32;
        let chunk_coord = (chunk_x, chunk_z);

        // PASS 1: Generate base heights first
        let resolution = chunk_size + 1;
        let mut heights = Vec::with_capacity((resolution * resolution) as usize);
        let mut riverbed_mask = Vec::with_capacity((resolution * resolution) as usize);

        for z in 0..=chunk_size {
            for x in 0..=chunk_size {
                let world_x = chunk_world_x + (x as f32);
                let world_z = chunk_world_z + (z as f32);

                // Generate base terrain height using noise
                let noise_x = world_x * scale;
                let noise_z = world_z * scale;
                let base_height = self.noise.sample_terrain_height(noise_x, noise_z) * height_scale;
                
                heights.push(base_height);
                riverbed_mask.push(false);
            }
        }

        // PASS 2: Apply river carving with proper flat riverbeds
        if let (Some(global_river_path), Some(river_config)) = (global_river_path, river_config) {
            // First pass: Apply river carving
            for z in 0..=chunk_size {
                for x in 0..=chunk_size {
                    let world_x = chunk_world_x + (x as f32);
                    let world_z = chunk_world_z + (z as f32);
                    let world_pos = Vec3::new(world_x, 0.0, world_z);
                    
                    let height_index = (z * resolution + x) as usize;
                    let original_height = heights[height_index];
                    
                    // Use the detailed river carving function
                    let (river_modifier, is_riverbed) = get_river_height_modifier_detailed(
                        world_pos,
                        global_river_path,
                        river_config,
                        chunk_coord
                    );
                    
                    if is_riverbed {
                        // Force to the calculated riverbed height (absolute height)
                        heights[height_index] = river_modifier;
                        riverbed_mask[height_index] = true;
                    } else if river_modifier > 0.0 {
                        // Apply gradual carving for banks (subtract carve depth from original height)
                        heights[height_index] = original_height - river_modifier;
                        
                        // Ensure we don't go below a reasonable minimum height
                        let min_height = -river_config.river_depth * 3.0;
                        heights[height_index] = heights[height_index].max(min_height);
                        
                        // Also ensure banks don't go below riverbed level
                        let nearby_riverbed_height = river_modifier - river_config.river_depth * 2.0;
                        heights[height_index] = heights[height_index].max(nearby_riverbed_height);
                    }
                }
            }
            
            // Second pass: Apply enhanced smoothing that preserves riverbed flatness
            self.smooth_river_terrain_preserving_riverbed(
                &mut heights, 
                &riverbed_mask, 
                resolution as usize, 
                global_river_path, 
                river_config, 
                chunk_coord, 
                chunk_world_x, 
                chunk_world_z
            );
        }

        // PASS 3: Generate vertices using the final heights
        for z in 0..=chunk_size {
            for x in 0..=chunk_size {
                let world_x = chunk_world_x + (x as f32);
                let world_z = chunk_world_z + (z as f32);
                let height_index = (z * resolution + x) as usize;
                let final_height = heights[height_index];

                positions.push([world_x, final_height, world_z]);
                uvs.push([x as f32 / chunk_size as f32, z as f32 / chunk_size as f32]);
                
                // Determine terrain type based on final height
                let terrain_type = if riverbed_mask[height_index] {
                    TerrainType::Water // Mark riverbed areas as water
                } else if final_height < -1.0 {
                    TerrainType::Water
                } else if final_height < 5.0 {
                    TerrainType::Valley
                } else if final_height < 15.0 {
                    TerrainType::Plains
                } else {
                    TerrainType::Mountain
                };
                terrain_types.push(terrain_type);
            }
        }

        // Calculate normals using the final positions
        normals.resize(positions.len(), [0.0, 1.0, 0.0]);
        for i in 0..=chunk_size {
            for j in 0..=chunk_size {
                let idx = (i * (chunk_size + 1) + j) as usize;
                
                // Use safe bounds for accessing neighbors
                let left_idx = if j > 0 { idx - 1 } else { idx };
                let right_idx = if j < chunk_size { idx + 1 } else { idx };
                let up_idx = if i > 0 { idx - (chunk_size + 1) as usize } else { idx };
                let down_idx = if i < chunk_size { idx + (chunk_size + 1) as usize } else { idx };
                
                let left = positions[left_idx];
                let right = positions[right_idx];
                let up = positions[up_idx];
                let down = positions[down_idx];
                
                let dx = Vec3::new(right[0] - left[0], right[1] - left[1], right[2] - left[2]);
                let dz = Vec3::new(down[0] - up[0], down[1] - up[1], down[2] - up[2]);
                let normal = if dx.length() > 0.0 && dz.length() > 0.0 {
                    dz.cross(dx).normalize()
                } else {
                    Vec3::Y // Default upward normal
                };
                
                normals[idx] = [normal.x, normal.y, normal.z];
            }
        }

        // Generate indices for triangles
        for i in 0..chunk_size {
            for j in 0..chunk_size {
                let top_left = i * (chunk_size + 1) + j;
                let top_right = top_left + 1;
                let bottom_left = (i + 1) * (chunk_size + 1) + j;
                let bottom_right = bottom_left + 1;

                // Two triangles per quad
                indices.extend_from_slice(&[
                    top_left, bottom_left, top_right,
                    top_right, bottom_left, bottom_right,
                ]);
            }
        }

        // Create the mesh
        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD | bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
        );

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
        
        (mesh, terrain_types)
    }

    // Add the smoothing function
    fn smooth_river_terrain_preserving_riverbed(
        &self, 
        heights: &mut [f32], 
        riverbed_mask: &[bool],
        resolution: usize,
        global_river_path: &GlobalRiverPath,
        river_config: &RiverConfig,
        chunk_coord: (i32, i32),
        chunk_world_x: f32,
        chunk_world_z: f32
    ) {
        let mut smoothed = heights.to_vec();
        
        // Apply multiple passes of smoothing
        for _pass in 0..4 {
            for z in 1..resolution - 1 {
                for x in 1..resolution - 1 {
                    let idx = z * resolution + x;
                    
                    // Skip smoothing for riverbed points to keep them flat
                    if riverbed_mask[idx] {
                        continue;
                    }
                    
                    let world_x = chunk_world_x + x as f32;
                    let world_z = chunk_world_z + z as f32;
                    let world_pos = Vec3::new(world_x, 0.0, world_z);
                    
                    // Check if this point is near a river
                    let (river_modifier, _) = get_river_height_modifier_detailed(
                        world_pos,
                        global_river_path,
                        river_config,
                        chunk_coord
                    );
                    
                    if river_modifier > 0.01 {
                        // Apply stronger smoothing near rivers (but not on riverbed)
                        let mut sum = 0.0;
                        let mut total_weight = 0.0;
                        
                        // Sample 5x5 neighborhood for river areas
                        for dz in -3..=3 {
                            for dx in -3..=3 {
                                let nx = x as i32 + dx;
                                let nz = z as i32 + dz;
                                
                                if nx >= 0 && nx < resolution as i32 && nz >= 0 && nz < resolution as i32 {
                                    let nidx = (nz as usize) * resolution + (nx as usize);
                                    
                                    // Use Gaussian-like weighting for smoother results
                                    let distance_sq = (dx * dx + dz * dz) as f32;
                                    let weight = (-distance_sq / 8.0).exp(); // Gaussian falloff
                                    
                                    sum += heights[nidx] * weight;
                                    total_weight += weight;
                                }
                            }
                        }
                        
                        if total_weight > 0.0 {
                            smoothed[idx] = heights[idx] * 0.1 + (sum / total_weight) * 0.99;
                        }
                    } else {
                        // Standard smoothing for non-river areas
                        let mut sum = 0.0;
                        let mut count = 0;
                        
                        for dz in -1..=1 {
                            for dx in -1..=1 {
                                let nx = x as i32 + dx;
                                let nz = z as i32 + dz;
                                
                                if nx >= 0 && nx < resolution as i32 && nz >= 0 && nz < resolution as i32 {
                                    let nidx = (nz as usize) * resolution + (nx as usize);
                                    sum += heights[nidx];
                                    count += 1;
                                }
                            }
                        }
                        
                        if count > 0 {
                            smoothed[idx] = heights[idx] * 0.8 + (sum / count as f32) * 0.2;
                        }
                    }
                }
            }
            
            // Copy back for next pass, but preserve riverbed heights
            for i in 0..heights.len() {
                if !riverbed_mask[i] {
                    heights[i] = smoothed[i];
                }
            }
        }
    }
}

impl Default for TerrainGenerator {
    fn default() -> Self {
        Self::new()
    }
}