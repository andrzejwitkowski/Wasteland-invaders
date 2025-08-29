use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_resource::{AsBindGroup, ShaderRef, ShaderType}
    }
};

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

#[derive(Component, Clone, Debug)]
pub struct Terrain {
    pub seed: u32,
    pub size: UVec2,
    pub plane_size: Vec2,
    pub height_scale: f32,
    pub frequency: f64,
    pub lacunarity: f32,
    pub octaves: usize,
    pub persistence: f32,
    pub material: Handle<StandardMaterial>,
}

#[derive(Clone, Debug)]
struct RiverSettings {
    width: f32,
    depth: f32,
    meander_frequency: f32,  // Controls how often the river bends
    meander_amplitude: f32,  // Controls how far the river bends
    noise_scale: f32,       // Add some noise to the river path
    channel_smoothing: f32, // How smoothly the river banks transition
}

#[derive(Component)]
struct RiverWater;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct RiverMaterial {
    #[uniform(0)]
    color_and_time: Vec4, // RGB = color; A = time
}


impl Material for RiverMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/river_water.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
impl Default for Terrain {
    fn default() -> Self {
        Self {
            seed: 0,
            size: UVec2::new(100, 100),
            plane_size: Vec2::new(50.0, 100.0),
            height_scale: 10.0,
            frequency: 0.05,
            lacunarity: 2.0,
            octaves: 5,
            persistence: 0.5,
            material: Handle::default(),
        }
    }
}

impl Default for RiverSettings {
    fn default() -> Self {
        Self {
            width: 8.0,
            depth: 5.0,
            meander_frequency: 0.05,
            meander_amplitude: 15.0,
            noise_scale: 2.0,
            channel_smoothing: 4.0,
        }
    }
}

pub struct FbmTerrainPlugin;
impl Plugin for FbmTerrainPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_plugins(MaterialPlugin::<RiverMaterial>::default())
        .add_systems(Startup, 
            (
                prepare_terrain,
                generate_terrain_system.after(prepare_terrain)
            ).chain()
        )
        .add_systems(Update, update_river_material);
    }
}

fn generate_river_heightmap(
    terrain: &Terrain,
    settings: &RiverSettings,
) -> Vec<f32> {
    let mut heights = vec![0.0; (terrain.size.x * terrain.size.y) as usize];
    
    let noise = Fbm::<Perlin>::new(terrain.seed)
        .set_frequency(0.1)
        .set_persistence(0.5)
        .set_octaves(3);

    let step_x = terrain.plane_size.x / (terrain.size.x as f32);
    let step_z = terrain.plane_size.y / (terrain.size.y as f32);
    
    // River centerline calculation
    for z_idx in 0..terrain.size.y {
        for x_idx in 0..terrain.size.x {
            let x = (x_idx as f32 * step_x) - (terrain.plane_size.x / 2.0);
            let z = (z_idx as f32 * step_z) - (terrain.plane_size.y / 2.0);

            // Calculate the meandering river centerline
            let phase = z * settings.meander_frequency;
            let noise_offset = noise.get([x as f64 * 0.1, z as f64 * 0.1]) as f32 * settings.noise_scale;
            
            // River centerline position (using sine for meandering)
            let river_center_x = settings.meander_amplitude * 
                (phase.sin() + (phase * 2.0).sin() * 0.3) + noise_offset;

            // Calculate distance from the centerline
            let dist_to_river = (x - river_center_x).abs();

            // Create smmoth river channel profile
            let river_profile = 1.0 - smooth_step(
                settings.width * 0.5,           // Inner edge of river bank
                settings.width * 1.5,           // Outer edge of river bank
                dist_to_river
            );

            // Apply river depth and smooth the channel
            let river_depth = settings.depth * river_profile;

            let idx = (z_idx * terrain.size.x + x_idx) as usize;
            heights[idx] = -river_depth; // Negative because we're carving into the terrain
        }
    }

    heights
}

fn generate_river_water_mesh(
    terrain: &Terrain,
    settings: &RiverSettings,
) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let step_x = terrain.plane_size.x / (terrain.size.x as f32);
    let step_z = terrain.plane_size.y / (terrain.size.y as f32);
    let water_height = 0.05; // Slightly above river bottom

    for z_idx in 0..terrain.size.y {
        for x_idx in 0..terrain.size.x {
            let x = (x_idx as f32 * step_x) - (terrain.plane_size.x / 2.0);
            let z = (z_idx as f32 * step_z) - (terrain.plane_size.y / 2.0);

            // Calculate river center at this point
            let phase = z * settings.meander_frequency;
            let river_center_x = settings.meander_amplitude * 
                (phase.sin() + (phase * 2.0).sin() * 0.3);

            // Only add vertices near the river
            let dist_to_river = (x - river_center_x).abs();
            if dist_to_river < settings.width * 2.0 {
                positions.push([x, water_height, z]);
                uvs.push([
                    (x + terrain.plane_size.x / 2.0) / terrain.plane_size.x,
                    (z + terrain.plane_size.y / 2.0) / terrain.plane_size.y
                ]);
            }
        }
    }

    // Generate indices for visible water segments
    let vertices_per_row = terrain.size.x as u32;
    for z in 0..terrain.size.y - 1 {
        for x in 0..terrain.size.x - 1 {
            let current = z * vertices_per_row + x;
            let next = current + 1;
            let below = current + vertices_per_row;
            let below_next = below + 1;

            // First triangle (counter-clockwise)
            indices.extend_from_slice(&[
                current,     // Top left
                below,      // Bottom left 
                next,       // Top right
            ]);

            // Second triangle (counter-clockwise)
            indices.extend_from_slice(&[
                next,       // Top right
                below,      // Bottom left
                below_next, // Bottom right
            ]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_indices(Indices::U32(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh
}

// Helper function for smooth transitions
fn smooth_step(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn prepare_terrain(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Spawning terrain");
    commands.spawn((
        Terrain {
            seed: 12345, // Example seed
            size: UVec2::new(100, 100), // Vertices: 250 wide, 500 long
            plane_size: Vec2::new(50.0, 100.0), // World units: 250m wide, 500m long
            height_scale: 12.0,
            octaves: 9,
            persistence: 0.455,   // Slightly decreased to avoid too much noise
            lacunarity: 2.4,     // Slightly increased for more variation
            frequency: 0.12, 
            // frequency: 0.15,
            // lacunarity: 2.2,
            // octaves: 7,
            // persistence: 0.4,
            material: materials.add(StandardMaterial { // Assign a material for the terrain
                base_color: Color::srgb(1.0, 0.6, 0.25), // A greenish color
                metallic: 0.05,
                perceptual_roughness: 0.75,
                ..default()
            }),
        },
        Transform::from_xyz(0.0, 0.0 ,0.0),
    ));
}

fn generate_terrain_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<RiverMaterial>>,
    terrain_query: Query<(Entity, &Terrain, Option<&Transform>), With<Terrain>>,
) {
    for (entity, terrain, transform) in terrain_query.iter() {
        info!("Generating terrain for entity {:?}", entity);

        let fbm = Fbm::<Perlin>::new(terrain.seed)
            .set_octaves(terrain.octaves)
            .set_frequency(terrain.frequency as f64)
            .set_lacunarity(terrain.lacunarity as f64)
            .set_persistence(terrain.persistence as f64);

        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        let num_vertices_x = terrain.size.x;
        let num_vertices_z = terrain.size.y;

        let step_x = terrain.plane_size.x / (num_vertices_x as f32);
        let step_z = terrain.plane_size.y / (num_vertices_z as f32);

        for z_idx in 0..num_vertices_z {
            for x_idx in 0..num_vertices_x {
                let x = (x_idx as f32 * step_x) - (terrain.plane_size.x / 2.0);
                let z = (z_idx as f32 * step_z) - (terrain.plane_size.y / 2.0);

                let noise_x = x as f64;
                let noise_z = z as f64;

                let noise_val = fbm.get([
                    noise_x * terrain.frequency, 
                    noise_z * terrain.frequency
                ]);

                let height = terrain.height_scale * noise_val as f32;

                positions.push([x, height, z]);
                uvs.push([
                    (x + terrain.plane_size.x / 2.0) / terrain.plane_size.x,
                    (z + terrain.plane_size.y / 2.0) / terrain.plane_size.y
                    ]);
            }
        }

        // Generate river heightmap
        let river_settings = RiverSettings::default();
        let river_heights = generate_river_heightmap(terrain, &river_settings);

        // Combine terrain and river heights
        for (i, position) in positions.iter_mut().enumerate() {
            position[1] += river_heights[i];
        }

        let river_settings = RiverSettings::default();
        let water_mesh = generate_river_water_mesh(terrain, &river_settings);
        
        // let water_material = materials.add(StandardMaterial {
        //     base_color: Color::srgba(0.2, 0.5, 1.0, 0.6),
        //     alpha_mode: AlphaMode::Blend,
        //     metallic: 0.0,
        //     reflectance: 0.5,
        //     perceptual_roughness: 0.0,
        //     ..default()
        // });
        let water_material = materials.add(RiverMaterial {
            color_and_time: Vec4::new(0.2, 0.5, 1.0, 0.0),
        });

        commands.spawn((
            Mesh3d(meshes.add(water_mesh)),
            MeshMaterial3d(water_material),
            Transform::from_xyz(0.0, 0.5, 0.0), // Slightly above terrain
            GlobalTransform::default(),
            Visibility::default(),
            RiverWater,
        ));

        // Add debug visualization by coloring the river
        let mut colors: Vec<[f32; 4]> = Vec::with_capacity(positions.len());
        for i in 0..positions.len() {
            // Color based on river depth - deeper = more blue
            let river_depth = river_heights[i].abs() / river_settings.depth;
            colors.push([
                0.8 - river_depth * 0.8, // Less red where river is
                0.6 - river_depth * 0.4, // Less green where river is
                0.2 + river_depth * 0.8, // More blue where river is
                1.0
            ]);
        }
        // Triangles
        for z_idx in 0..num_vertices_z - 1 {
            for x_idx in 0..num_vertices_x - 1 {
                let first = z_idx * num_vertices_x + x_idx;
                let second = first + 1;
                let third = (z_idx + 1) * num_vertices_x + x_idx;
                let fourth = third + 1;

                indices.push(first);
                indices.push(third);
                indices.push(second);

                indices.push(second);
                indices.push(third);
                indices.push(fourth);
            }
        }

        // Calculate normals
        let mut normal_sums: Vec<Vec3> = vec![Vec3::ZERO; positions.len()];
        let mut normal_counts: Vec<u32> = vec![0; positions.len()];

        for i in (0..indices.len()).step_by(3) {
            let a = indices[i] as usize;
            let b = indices[i + 1] as usize;
            let c = indices[i + 2] as usize;

            let u = Vec3::from_array(positions[a]);
            let v = Vec3::from_array(positions[b]);
            let w = Vec3::from_array(positions[c]);

            let edge1 = v - u;
            let edge2 = w - u;
            let face_normal = edge1.cross(edge2).normalize_or_zero();

            normal_sums[a] += face_normal;
            normal_sums[b] += face_normal;
            normal_sums[c] += face_normal;

            normal_counts[a] += 1;
            normal_counts[b] += 1;
            normal_counts[c] += 1;
        }

        // Face up if degenerate normals
        let normals: Vec<[f32; 3]> = normal_sums.iter()
            .zip(normal_counts.iter())
            .map(|(sum, &count)| {
                if count > 0 {
                    let averaged = (sum / count as f32).normalize();
                    [averaged.x, averaged.y, averaged.z]
                } else {
                    [0.0, 1.0, 0.0] // Default up-facing normal for degenerate cases
                }
            })
            .collect();
        
        // Create mesh
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        mesh.insert_indices(Indices::U32(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

        let transform = transform.cloned().unwrap_or_default();

        commands.entity(entity).insert((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(terrain.material.clone()),
            transform,
        ));
    }
}

fn update_river_material(
    time: Res<Time>,
    mut materials: ResMut<Assets<RiverMaterial>>,
) {
    for (_, material) in materials.iter_mut() {
        material.color_and_time = Vec4::new(0.2, 0.5, 1.0, time.elapsed_secs());
    }
}