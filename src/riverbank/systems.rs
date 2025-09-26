use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use super::components::*;
use super::resources::*;
use super::utils::mesh_utilities::*;

use crate::rendering::complex_water::{CompleteComplexWaterMaterial, ComplexWaterMaterial};

pub fn setup_river_system(mut commands: Commands, config: Res<RiverConfig>) {
    commands.insert_resource(GeneratedRiverChunks::default());
    
    // Generate the global river path
    let global_path = generate_global_river_path(&config);
    commands.insert_resource(global_path);
}

pub fn update_river_water(
    mut water_query: Query<&mut Transform, (With<RiverWater>, With<RiverFlow>)>,
    time: Res<Time>,
) {
    let time_offset = time.elapsed_secs() * 0.1;
    for mut transform in water_query.iter_mut() {
        let base_y = -0.1;
        transform.translation.y = base_y + (time_offset * 2.0).sin() * 0.02;
    }
}

pub fn river_config_ui(
    mut contexts: EguiContexts,
    mut config: ResMut<RiverConfig>,
    mut generated_chunks: ResMut<GeneratedRiverChunks>,
    mut global_river_path: ResMut<GlobalRiverPath>,
    mut commands: Commands,
    river_entities: Query<Entity, Or<(With<RiverWater>, With<RiverBank>)>>,
    terrain_entities: Query<Entity, With<crate::terrain::resources::TerrainChunk>>,
    mut terrain_chunks: ResMut<crate::terrain::resources::TerrainChunks>,
    mut terrain_events: EventWriter<crate::terrain::resources::GenerateTerrainEvent>,
) {
    egui::Window::new("River Bank Controls")
        .default_width(300.0)
        .show(contexts.ctx_mut().unwrap(), |ui| {
            ui.heading("River Properties");
            
            let mut changed = false;
            
            changed |= ui.add(egui::Slider::new(&mut config.river_width, 5.0..=30.0) // Increased range
                .text("River Width")).changed();
                
            changed |= ui.add(egui::Slider::new(&mut config.meander_frequency, 0.001..=0.1)
                .text("Meander Frequency")).changed();
                
            changed |= ui.add(egui::Slider::new(&mut config.meander_amplitude, 0.0..=50.0)
                .text("Meander Amplitude")).changed();
            
            if ui.button("Regenerate River & Terrain").clicked() {
                // Despawn existing river entities
                for entity in river_entities.iter() {
                    commands.entity(entity).despawn();
                }
                
                // Despawn existing terrain entities
                for entity in terrain_entities.iter() {
                    commands.entity(entity).despawn();
                }
                
                // Clear terrain chunks
                terrain_chunks.chunks.clear();
                
                // Regenerate global river path
                *global_river_path = generate_global_river_path(&config);
                
                // Clear generated chunks to force regeneration
                generated_chunks.chunks.clear();
                
                // Trigger terrain regeneration
                terrain_events.write(crate::terrain::resources::GenerateTerrainEvent {
                    center_x: 0.0,
                    center_z: 0.0,
                    radius: 2,
                });
            }
            
            if changed {
                config.set_changed();
            }
        });
}

fn generate_global_river_path(config: &RiverConfig) -> GlobalRiverPath {
    let mut global_path = GlobalRiverPath::default();
    
    // Generate a long river path that spans multiple chunks
    let river_length = 1000.0; // Total river length
    let segments = 200; // Total segments for the entire river
    
    let mut path_points = Vec::new();
    
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let distance_along_river = t * river_length;
        
        // Base position along the river direction
        let base_pos = config.global_river_start + config.global_river_direction * distance_along_river;
        
        // Add meandering (perpendicular to river direction)
        let perpendicular = Vec2::new(-config.global_river_direction.y, config.global_river_direction.x);
        let meander_offset = (distance_along_river * config.meander_frequency).sin() * config.meander_amplitude;
        
        let final_pos = base_pos + perpendicular * meander_offset;
        path_points.push(Vec3::new(final_pos.x, 0.0, final_pos.y));
    }
    
    global_path.path_points = path_points.clone();
    
    // Calculate which chunks each river segment intersects WITH PROPER CONNECTIVITY
    let chunk_size = 64.0; // Should match your terrain chunk size
    
    // Process each segment (line between consecutive points)
    for window in path_points.windows(2) {
        let start_point = window[0];
        let end_point = window[1];
        
        // Get all chunks this segment passes through
        let chunks_on_segment = get_chunks_on_line_segment(start_point, end_point, chunk_size);
        
        for chunk_coord in chunks_on_segment {
            global_path.chunk_intersections
                .entry(chunk_coord)
                .or_insert_with(Vec::new)
                .extend_from_slice(&[start_point, end_point]);
        }
    }
    
    // Remove duplicates and sort points along the river path for each chunk
    for (chunk_coord, points) in global_path.chunk_intersections.iter_mut() {
        points.sort_by(|a, b| {
            let dist_a = config.global_river_start.distance(Vec2::new(a.x, a.z));
            let dist_b = config.global_river_start.distance(Vec2::new(b.x, b.z));
            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        points.dedup_by(|a, b| a.distance(*b) < 1.0);
    }
    
    global_path
}

// Helper function to get all chunks a line segment passes through
fn get_chunks_on_line_segment(start: Vec3, end: Vec3, chunk_size: f32) -> Vec<(i32, i32)> {
    let mut chunks = Vec::new();
    
    let start_chunk_x = (start.x / chunk_size).floor() as i32;
    let start_chunk_z = (start.z / chunk_size).floor() as i32;
    let end_chunk_x = (end.x / chunk_size).floor() as i32;
    let end_chunk_z = (end.z / chunk_size).floor() as i32;
    
    // Use Bresenham-like algorithm for chunk traversal
    let dx = (end_chunk_x - start_chunk_x).abs();
    let dz = (end_chunk_z - start_chunk_z).abs();
    let sx = if start_chunk_x < end_chunk_x { 1 } else { -1 };
    let sz = if start_chunk_z < end_chunk_z { 1 } else { -1 };
    let mut err = dx - dz;
    
    let mut x = start_chunk_x;
    let mut z = start_chunk_z;
    
    loop {
        chunks.push((x, z));
        
        if x == end_chunk_x && z == end_chunk_z {
            break;
        }
        
        let e2 = 2 * err;
        if e2 > -dz {
            err -= dz;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            z += sz;
        }
    }
    
    chunks
}

// Replace the entire generate_river_for_chunk function:

fn generate_river_for_chunk(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    water_materials: &mut ResMut<Assets<CompleteComplexWaterMaterial>>,
    standard_materials: &mut ResMut<Assets<StandardMaterial>>,
    config: &RiverConfig,
    chunk_x: i32,
    chunk_z: i32,
    chunk_world_x: f32,
    chunk_world_z: f32,
    river_points: &[Vec3],
) {
    if river_points.len() < 2 {
        return;
    }
    
    // Get chunk boundaries
    let chunk_size = 64.0;
    let chunk_min_x = chunk_world_x;
    let chunk_max_x = chunk_world_x + chunk_size;
    let chunk_min_z = chunk_world_z;
    let chunk_max_z = chunk_world_z + chunk_size;
    
    // Create a continuous path through this chunk
    let mut chunk_river_points = Vec::new();
    
    // Add points that are in or near this chunk
    let buffer = config.river_width * 2.0; // Larger buffer for better blending
    for point in river_points {
        if point.x >= chunk_min_x - buffer && point.x <= chunk_max_x + buffer &&
           point.z >= chunk_min_z - buffer && point.z <= chunk_max_z + buffer {
            chunk_river_points.push(*point);
        }
    }
    
    if chunk_river_points.len() < 2 {
        return;
    }
    
    // Sort points to maintain river flow direction
    chunk_river_points.sort_by(|a, b| {
        let dist_a = config.global_river_start.distance(Vec2::new(a.x, a.z));
        let dist_b = config.global_river_start.distance(Vec2::new(b.x, b.z));
        dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    // Remove duplicates
    chunk_river_points.dedup_by(|a, b| a.distance(*b) < 1.0);
    
    // Convert to local coordinates with fixed height
    let fixed_water_height = -config.river_depth * 2.0; // Place water well below terrain
    let local_river_points: Vec<Vec3> = chunk_river_points.iter()
        .map(|point| {
            Vec3::new(
                point.x - chunk_world_x,
                fixed_water_height, // Fixed low height for water
                point.z - chunk_world_z,
            )
        })
        .collect();
    
    if local_river_points.len() < 2 {
        return;
    }
    
    // Generate water surface mesh
    let water_mesh = create_river_water_mesh(&local_river_points, config.river_width);
    let water_mesh_handle = meshes.add(water_mesh);
    
    // Create water material with more transparency
    let water_material = CompleteComplexWaterMaterial {
        base: StandardMaterial {
            base_color: Color::srgba(0.1, 0.3, 0.8, 0.8), // More blue, more transparent
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.0,
            reflectance: 1.0,
            ..default()
        },
        extension: ComplexWaterMaterial {
            wave_params: Vec4::new(0.02, 0.1, config.flow_speed, 1.0),
            misc_params: Vec4::new(1.0, 0.1, 0.9, 0.0),
        },
    };
    let water_material_handle = water_materials.add(water_material);
    
    // Spawn water entity
    commands.spawn((
        Mesh3d(water_mesh_handle),
        MeshMaterial3d(water_material_handle),
        Transform::from_xyz(chunk_world_x, 0.0, chunk_world_z),
        RiverChunk { chunk_x, chunk_z },
        RiverWater,
        RiverFlow {
            direction: Vec3::new(config.global_river_direction.x, 0.0, config.global_river_direction.y),
            speed: config.flow_speed,
        },
        Name::new(format!("RiverWater_{}_{}", chunk_x, chunk_z)),
    ));
}

pub fn generate_river_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut water_materials: ResMut<Assets<CompleteComplexWaterMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    config: Res<RiverConfig>,
    global_river_path: Res<GlobalRiverPath>,
    mut generated_chunks: ResMut<GeneratedRiverChunks>,
    mut terrain_events: EventReader<crate::terrain::resources::GenerateTerrainEvent>,
) {
    if terrain_events.is_empty() {
        return;
    }

    for event in terrain_events.read() {
        let chunk_size = 64.0;
        let radius = event.radius as i32;
        let center_chunk_x = (event.center_x / chunk_size).floor() as i32;
        let center_chunk_z = (event.center_z / chunk_size).floor() as i32;

        for chunk_x in (center_chunk_x - radius)..=(center_chunk_x + radius) {
            for chunk_z in (center_chunk_z - radius)..=(center_chunk_z + radius) {
                let chunk_coord = (chunk_x, chunk_z);
                
                // Skip if already generated
                if generated_chunks.chunks.contains(&chunk_coord) {
                    continue;
                }

                // Check if this chunk has river intersections
                if let Some(river_points) = global_river_path.chunk_intersections.get(&chunk_coord) {
                    let chunk_world_x = chunk_x as f32 * chunk_size;
                    let chunk_world_z = chunk_z as f32 * chunk_size;

                    generate_river_for_chunk(
                        &mut commands,
                        &mut meshes,
                        &mut water_materials,
                        &mut standard_materials,
                        &config,
                        chunk_x,
                        chunk_z,
                        chunk_world_x,
                        chunk_world_z,
                        river_points,
                    );

                    generated_chunks.chunks.insert(chunk_coord);
                }
            }
        }
    }
}

// Update the river terrain modifier function to create flat riverbeds
fn get_river_terrain_modifier(position: Vec3, river_points: &[Vec3], config: &RiverConfig) -> (f32, bool) {
    if river_points.len() < 2 {
        return (0.0, false);
    }
    
    let point_2d = Vec2::new(position.x, position.z);
    let mut min_distance = f32::MAX;
    let mut closest_segment_height = 0.0;
    
    // Find minimum distance to river path and get the height at that point
    for window in river_points.windows(2) {
        let start_2d = Vec2::new(window[0].x, window[0].z);
        let end_2d = Vec2::new(window[1].x, window[1].z);
        
        // Distance from point to line segment
        let line_vec = end_2d - start_2d;
        let point_vec = point_2d - start_2d;
        
        let line_len_sq = line_vec.length_squared();
        if line_len_sq < 0.0001 {
            continue; // Skip degenerate segments
        }
        
        let t = (point_vec.dot(line_vec) / line_len_sq).clamp(0.0, 1.0);
        let projection = start_2d + line_vec * t;
        let distance = point_2d.distance(projection);
        
        if distance < min_distance {
            min_distance = distance;
            // Interpolate height along the river segment
            closest_segment_height = window[0].y * (1.0 - t) + window[1].y * t;
        }
    }
    
    // Calculate carving profile
    let carve_radius = config.river_width * 12.0; // Wider carving area
    let river_center_width = config.river_width * 1.2; // River channel width
    let transition_width = carve_radius - river_center_width; // Width of the transition zone
    
    if min_distance > carve_radius {
        return (0.0, false); // No effect outside carving radius
    }
    
    // Create flat riverbed with very gentle transitions
    if min_distance <= river_center_width {
        // Return the absolute riverbed height (river path height minus depth)
        let riverbed_height = closest_segment_height - config.river_depth * 2.0;
        return (riverbed_height, true); // This is an absolute height for riverbed
    } else {
        // Much gentler transition to banks using a cubic curve for very smooth falloff
        let transition_factor = (min_distance - river_center_width) / transition_width;
        
        // Use multiple smoothing curves for ultra-gentle slopes
        let smooth_factor1 = 1.0 - (transition_factor * transition_factor * transition_factor * transition_factor); // Quartic for very gentle
        let smooth_factor2 = 1.0 - ((transition_factor * std::f32::consts::PI * 0.5).sin().powi(4)); // Quartic sine
        let smooth_factor3 = (1.0 + (transition_factor * std::f32::consts::PI).cos()) * 0.5; // Cosine
        
        // Combine all smoothing factors for ultra-gentle slopes
        let combined_factor = (smooth_factor1 + smooth_factor2 + smooth_factor3) / 3.0;
        
        // Reduce the maximum carve depth for gentler overall effect
        let carve_depth = config.river_depth * 1.5 * combined_factor; // Reduced from 2.0 to 1.5
        return (carve_depth, false); // This is a carve depth for banks
    }
}

// Update the detailed function to use the corrected modifier function
pub fn get_river_height_modifier_detailed(
    position: Vec3, 
    global_river_path: &GlobalRiverPath, 
    config: &RiverConfig,
    chunk_coord: (i32, i32)
) -> (f32, bool) { // Returns (height_or_carve_depth, is_riverbed)
    // Get extended river points including adjacent chunks
    let adjacent_chunks = [
        (chunk_coord.0 - 1, chunk_coord.1 - 1),
        (chunk_coord.0, chunk_coord.1 - 1),
        (chunk_coord.0 + 1, chunk_coord.1 - 1),
        (chunk_coord.0 - 1, chunk_coord.1),
        chunk_coord,
        (chunk_coord.0 + 1, chunk_coord.1),
        (chunk_coord.0 - 1, chunk_coord.1 + 1),
        (chunk_coord.0, chunk_coord.1 + 1),
        (chunk_coord.0 + 1, chunk_coord.1 + 1),
    ];
    
    let mut all_river_points = Vec::new();
    for adj_chunk in adjacent_chunks {
        if let Some(points) = global_river_path.chunk_intersections.get(&adj_chunk) {
            all_river_points.extend_from_slice(points);
        }
    }
    
    if !all_river_points.is_empty() {
        // Remove duplicates and sort
        all_river_points.sort_by(|a, b| {
            let dist_a = config.global_river_start.distance(Vec2::new(a.x, a.z));
            let dist_b = config.global_river_start.distance(Vec2::new(b.x, b.z));
            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        all_river_points.dedup_by(|a, b| a.distance(*b) < 1.0);
        
        // Use the corrected modifier function that returns both values
        return get_river_terrain_modifier(position, &all_river_points, config);
    }
    
    (0.0, false)
}