use bevy::prelude::*;
use crate::terrain::generation::TerrainGenerator;
use crate::terrain::resources::*;
use crate::terrain::noise::TerrainType;
use crate::riverbank::resources::{GlobalRiverPath, RiverConfig};

pub fn setup_terrain_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("Setting up terrain materials with natural colors...");
    
    let terrain_materials = TerrainMaterials {
        mountain_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.4, 0.3), // Brown/rocky color for mountains
            perceptual_roughness: 0.9,
            metallic: 0.0,
            ..default()
        }),
        hill_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.6, 0.3), // Green/brown for hills
            perceptual_roughness: 0.8,
            metallic: 0.0,
            ..default()
        }),
        plains_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.7, 0.2), // Green for plains
            perceptual_roughness: 0.7,
            metallic: 0.0,
            ..default()
        }),
        valley_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.5, 0.1), // Darker green for valleys
            perceptual_roughness: 0.6,
            metallic: 0.0,
            ..default()
        }),
        water_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.5, 0.3), // Sandy/muddy color (NOT blue)
            perceptual_roughness: 0.5,
            metallic: 0.0,
            ..default()
        }),
    };

    commands.insert_resource(terrain_materials);
    commands.insert_resource(TerrainChunks::default());
}

pub fn generate_initial_terrain(
    mut generate_events: EventWriter<GenerateTerrainEvent>,
    config: Res<TerrainConfig>,
) {
    println!("Starting initial terrain generation...");

    // Generate initial terrain around origin
    generate_events.write(GenerateTerrainEvent {
        center_x: 0.0,
        center_z: 0.0,
        radius: 2, // Generate 2x2 chunks initially
    });
}

pub fn handle_terrain_generation(
    mut commands: Commands,
    mut events: EventReader<GenerateTerrainEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_chunks: ResMut<TerrainChunks>,
    terrain_generator: Res<TerrainGenerator>,
    terrain_materials: Res<TerrainMaterials>,
    config: Res<TerrainConfig>,
    mut generated_events: EventWriter<TerrainGeneratedEvent>,
    global_river_path: Option<Res<GlobalRiverPath>>,
    river_config: Option<Res<RiverConfig>>,
) {
    for event in events.read() {
        println!("Processing terrain generation event at ({}, {})", event.center_x, event.center_z);
        let chunks_to_generate = calculate_chunks_in_radius(
            event.center_x,
            event.center_z,
            event.radius,
            config.chunk_size
        );

        let mut generated_chunks = Vec::new();

        for (chunk_x, chunk_z) in chunks_to_generate {
            // Skip if chunk already exists
            if terrain_chunks.chunks.contains_key(&(chunk_x, chunk_z)) {
                continue;
            }

            // Generate mesh for this chunk with river carving
            let (mesh, terrain_types) = terrain_generator.generate_chunk_mesh(
                chunk_x,
                chunk_z,
                config.chunk_size,
                config.scale,
                config.height_scale,
                global_river_path.as_deref(), // Convert Option<Res<T>> to Option<&T>
                river_config.as_deref(),      // Convert Option<Res<T>> to Option<&T>
            );

            // Determine material based on dominant terrain type
            let dominant_type = TerrainType::Mountain; // TEMPORARY
            //terrain_generator.get_dominant_terrain_type(&terrain_types);
            
            // Choose appropriate material based on terrain type
            let material = match dominant_type {
                TerrainType::Mountain => terrain_materials.mountain_material.clone(),
                TerrainType::Hill => terrain_materials.hill_material.clone(),
                TerrainType::Plains => terrain_materials.plains_material.clone(),
                TerrainType::Valley => terrain_materials.valley_material.clone(),
                TerrainType::Water => terrain_materials.water_material.clone(),
            };

            // Create the terrain chunk entity
            let chunk_entity = commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::IDENTITY,
                TerrainChunk {
                    chunk_x,
                    chunk_z,
                    vertices: Vec::new(),
                    terrain_types,
                },
                Name::new(format!("TerrainChunk_{}_{}", chunk_x, chunk_z)),
            )).id();

            terrain_chunks.chunks.insert((chunk_x, chunk_z), chunk_entity);
            generated_chunks.push((chunk_x, chunk_z));
        }

        if !generated_chunks.is_empty() {
            generated_events.write(TerrainGeneratedEvent {
                chunk_coords: generated_chunks,
            });
        }
    }
}

pub fn update_terrain_chunks(
    mut terrain_chunks: ResMut<TerrainChunks>,
    mut events: EventReader<TerrainGeneratedEvent>,
) {
    for event in events.read() {
        for &chunk_coord in &event.chunk_coords {
            terrain_chunks.loaded_chunks.push(chunk_coord);
        }
    }
}

fn calculate_chunks_in_radius(
    center_x: f32,
    center_z: f32,
    radius: u32,
    chunk_size: u32,
) -> Vec<(i32, i32)> {
    let mut chunks = Vec::new();
    let chunk_world_size = chunk_size as f32;
    
    let center_chunk_x = (center_x / chunk_world_size).floor() as i32;
    let center_chunk_z = (center_z / chunk_world_size).floor() as i32;

    for dz in -(radius as i32)..=(radius as i32) {
        for dx in -(radius as i32)..=(radius as i32) {
            chunks.push((center_chunk_x + dx, center_chunk_z + dz));
        }
    }

    chunks
}