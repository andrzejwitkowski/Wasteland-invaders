use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

use super::height_map_generator::{HeightmapConfig, HeightmapNoise};
use crate::rendering::complex_water::CompleteComplexWaterMaterial;

#[derive(Component)]
pub struct HeightmapTerrain;

#[derive(Component)]
pub struct HeightmapWater;

#[derive(Resource)]
pub struct HeightmapRenderConfig {
    pub chunk_size: f32,
    pub vertex_density: usize,  // vertices per chunk edge
    pub water_level_offset: f32, // how far above riverbed water surface sits
    pub enable_water_rendering: bool,
}

#[derive(Resource, Default)]
pub struct LastWaterLevelOffset {
    offset: f32,
}

impl Default for HeightmapRenderConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512.0,
            vertex_density: 513, // 257x257 vertices for good detail
            water_level_offset: 5.0,
            enable_water_rendering: true,
        }
    }
}

pub struct HeightmapRendererPlugin;

impl Plugin for HeightmapRendererPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<HeightmapRenderConfig>()
            .init_resource::<LastWaterLevelOffset>()
            .add_systems(Update, (
                heightmap_render_ui,
                update_water_level_on_change,
            )
            );
    }
}

pub fn heightmap_render_ui(
    mut contexts: bevy_egui::EguiContexts,
    mut render_config: ResMut<HeightmapRenderConfig>,
    heightmap_config: Res<HeightmapConfig>,
    heightmap_noise: Res<HeightmapNoise>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut water_materials: ResMut<Assets<CompleteComplexWaterMaterial>>,
    terrain_query: Query<Entity, Or<(With<HeightmapTerrain>, With<HeightmapWater>)>>,
) {
    bevy_egui::egui::Window::new("Heightmap Renderer")
        .default_width(350.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Render Settings");
            
            ui.add(bevy_egui::egui::Slider::new(&mut render_config.vertex_density, 128..=1025)
                .text("Vertex Density")
                .step_by(32.0));
                
            // In the heightmap_render_ui function, change the slider range:
            ui.add(bevy_egui::egui::Slider::new(&mut render_config.water_level_offset, -15.0..=15.0)
                .text("Water Level Offset"));
                
            ui.checkbox(&mut render_config.enable_water_rendering, "Render Water");
            
            ui.separator();
            
            // Show current water level for debugging
            let water_level = -heightmap_config.river_depth + render_config.water_level_offset;
            ui.label(format!("Water Level: {:.2}", water_level));
            ui.label(format!("River Depth: -{:.2}", heightmap_config.river_depth));
            
            if ui.button("Render Heightmap as 3D Terrain").clicked() {
                render_heightmap_terrain(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut water_materials,
                    &heightmap_noise,
                    &heightmap_config,
                    &render_config,
                    &terrain_query,
                );
            }
            
            if ui.button("Clear Rendered Terrain").clicked() {
                clear_rendered_terrain(&mut commands, &terrain_query);
            }
        });
}

fn clear_rendered_terrain(
    commands: &mut Commands,
    terrain_query: &Query<Entity, Or<(With<HeightmapTerrain>, With<HeightmapWater>)>>,
) {
    for entity in terrain_query.iter() {
        commands.entity(entity).despawn();
    }
    info!("Cleared rendered terrain");
}

fn render_heightmap_terrain(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    water_materials: &mut ResMut<Assets<CompleteComplexWaterMaterial>>,
    heightmap_noise: &HeightmapNoise,
    heightmap_config: &HeightmapConfig,
    render_config: &HeightmapRenderConfig,
    terrain_query: &Query<Entity, Or<(With<HeightmapTerrain>, With<HeightmapWater>)>>,
) {
    // Clear existing terrain first
    clear_rendered_terrain(commands, terrain_query);
    
    info!("Generating 3D terrain from heightmap...");
    
    // Generate heightmap data
    let heightmap_data = heightmap_noise.generate_heightmap(heightmap_config);
    
    // Create terrain mesh
    let (terrain_mesh, water_areas) = create_terrain_mesh_from_heightmap(
        &heightmap_data,
        heightmap_config,
        render_config,
        heightmap_noise,
    );
    
    let terrain_mesh_handle = meshes.add(terrain_mesh);
    
    // Spawn terrain entity
    commands.spawn((
        Mesh3d(terrain_mesh_handle),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.5, 0.4),
            perceptual_roughness: 0.8,
            metallic: 0.0,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        HeightmapTerrain,
    ));
    
    // Create water mesh if enabled and water areas exist
    if render_config.enable_water_rendering && !water_areas.is_empty() {
        let water_mesh = create_water_mesh_from_areas(
            &water_areas,
            &render_config,
        );
        
        let water_mesh_handle = meshes.add(water_mesh);
        
        // Use the complex water material with shader effects!
        let water_material = CompleteComplexWaterMaterial {
            base: StandardMaterial {
                base_color: Color::srgba(0.0, 0.4, 0.8, 0.7),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.02,
                metallic: 0.1,
                reflectance: 0.9,
                ..default()
            },
            extension: crate::rendering::complex_water::ComplexWaterMaterial {
                wave_params: Vec4::new(0.08, 0.8, 2.0, 2.0), // Good for rivers: small amplitude, high frequency, fast speed
                misc_params: Vec4::new(0.95, 0.8, 0.7, 0.0), // water_clarity, foam_intensity, foam_cutoff, time
            },
        };
        
        // Make water more visible with brighter color and less transparency
        commands.spawn((
            Mesh3d(water_mesh_handle),
            MeshMaterial3d(water_materials.add(water_material)),
            Transform::from_xyz(0.0, 0.0, 0.0),
            HeightmapWater,
        ));
        
        info!("Water mesh created with {} areas", water_areas.len());
    }
    
    info!("Terrain rendered successfully!");
}

fn create_terrain_mesh_from_heightmap(
    heightmap_data: &[Vec<f32>],
    heightmap_config: &HeightmapConfig,
    render_config: &HeightmapRenderConfig,
    heightmap_noise: &HeightmapNoise,
) -> (Mesh, Vec<WaterArea>) {
    let width = render_config.vertex_density;
    let height = render_config.vertex_density;
    let world_size = render_config.chunk_size;
    
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut water_areas = Vec::new();
    
    // Get dimensions of the pre-generated heightmap
    let heightmap_width = heightmap_data[0].len();
    let heightmap_height = heightmap_data.len();
    
    // Sample terrain mesh at our desired resolution using the pre-generated heightmap
    for z in 0..height {
        for x in 0..width {
            let world_x = (x as f32 / (width - 1) as f32 - 0.5) * world_size;
            let world_z = (z as f32 / (height - 1) as f32 - 0.5) * world_size;
            
            // Map world coordinates to heightmap indices
            let heightmap_x = ((x as f32 / (width - 1) as f32) * (heightmap_width - 1) as f32) as usize;
            let heightmap_z = ((z as f32 / (height - 1) as f32) * (heightmap_height - 1) as f32) as usize;
            
            // Sample height from the pre-generated heightmap data
            let terrain_height = heightmap_data[heightmap_z.min(heightmap_height - 1)][heightmap_x.min(heightmap_width - 1)];
            
            // Check if this point is water by calculating river effects
            let (river_mod, _) = heightmap_noise.calculate_river_effects(Vec2::new(world_x, world_z), heightmap_config);
            let is_water = river_mod < -0.7; // Only deep river areas get water
            
            vertices.push([world_x, terrain_height, world_z]);
            uvs.push([x as f32 / (width - 1) as f32, z as f32 / (height - 1) as f32]);
            
            if is_water {
                // Water sits at the actual terrain height (riverbed) plus offset
                let water_surface_level = terrain_height + render_config.water_level_offset;
                water_areas.push(WaterArea {
                    position: Vec3::new(world_x, water_surface_level, world_z),
                    size: world_size / width as f32,
                });
            }
        }
    }
    
    // Calculate normals using the vertex data
    for z in 0..height {
        for x in 0..width {
            let normal = calculate_normal_from_vertices(&vertices, x, z, width, height);
            normals.push(normal);
        }
    }
    
    // Generate indices for triangles
    for z in 0..(height - 1) {
        for x in 0..(width - 1) {
            let i = (z * width + x) as u32;
            
            // Two triangles per quad
            indices.extend_from_slice(&[
                i, i + width as u32, i + 1,
                i + 1, i + width as u32, i + width as u32 + 1,
            ]);
        }
    }
    
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    
    (mesh, water_areas)
}

fn calculate_normal_from_vertices(
    vertices: &[[f32; 3]], 
    x: usize, 
    z: usize, 
    width: usize, 
    height: usize
) -> [f32; 3] {
    let current_idx = z * width + x;
    let current_pos = Vec3::from(vertices[current_idx]);
    
    // Get neighboring vertices for normal calculation
    let left_pos = if x > 0 {
        Vec3::from(vertices[current_idx - 1])
    } else {
        current_pos
    };
    
    let right_pos = if x < width - 1 {
        Vec3::from(vertices[current_idx + 1])
    } else {
        current_pos
    };
    
    let up_pos = if z < height - 1 {
        Vec3::from(vertices[current_idx + width])
    } else {
        current_pos
    };
    
    let down_pos = if z > 0 {
        Vec3::from(vertices[current_idx - width])
    } else {
        current_pos
    };
    
    // Calculate normal using cross product of edge vectors
    let horizontal = right_pos - left_pos;
    let vertical = up_pos - down_pos;
    let normal = vertical.cross(horizontal).normalize();
    
    [normal.x, normal.y, normal.z]
}

#[derive(Clone)]
struct WaterArea {
    position: Vec3,
    size: f32,
}

fn create_water_mesh_from_areas(
    water_areas: &[WaterArea],
    render_config: &HeightmapRenderConfig,
) -> Mesh {
    if water_areas.is_empty() {
        return create_empty_mesh();
    }

    let flat_water_level = if !water_areas.is_empty() {
        water_areas[0].position.y  // Just use the first area's Y level
    } else {
        render_config.water_level_offset
    };
    
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    
    // Find the bounds of all water areas
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    
    for area in water_areas {
        min_x = min_x.min(area.position.x);
        max_x = max_x.max(area.position.x);
        min_z = min_z.min(area.position.z);
        max_z = max_z.max(area.position.z);
    }
    
    // Use render config to determine water mesh quality
    let water_segments = render_config.vertex_density - 1;
    
    let step_x = (max_x - min_x) / water_segments as f32;
    let step_z = (max_z - min_z) / water_segments as f32;
    
    for z in 0..=water_segments {
        for x in 0..=water_segments {
            let world_x = min_x + x as f32 * step_x;
            let world_z = min_z + z as f32 * step_z;
            
            // Dodaj minimalną wariację wysokości (0.001) aby uniknąć idealnie płaskiej siatki
            let height_variation = ((x as f32 * 0.1 + z as f32 * 0.1).sin() * 0.001).abs();
            
            vertices.push([world_x, flat_water_level + height_variation, world_z]);
            normals.push([0.0, 1.0, 0.0]);
            
            let u = (x as f32 / water_segments as f32) * 8.0;
            let v = (z as f32 / water_segments as f32) * 8.0;
            uvs.push([u, v]);
        }
    }
    
    // Generate indices for the water mesh
    for z in 0..water_segments {
        for x in 0..water_segments {
            let i = (z * (water_segments + 1) + x) as u32;
            let width = (water_segments + 1) as u32;
            
            // Two triangles per quad
            indices.extend_from_slice(&[
                i, i + width, i + 1,
                i + 1, i + width, i + width + 1,
            ]);
        }
    }
    
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    info!("Water mesh created with {}x{} vertices ({} polys)", 
          water_segments + 1, water_segments + 1, water_segments * water_segments * 2);

    mesh
}  

fn create_empty_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<[f32; 2]>::new());
    mesh.insert_indices(Indices::U32(Vec::new()));
    mesh
}

fn update_water_level_on_change(
    render_config: Res<HeightmapRenderConfig>,
    mut last_offset: ResMut<LastWaterLevelOffset>,
    mut water_query: Query<&mut Transform, With<HeightmapWater>>,
) {
    let offset_diff = (render_config.water_level_offset - last_offset.offset).abs();
    if offset_diff > 0.01 && !water_query.is_empty() {
        for mut transform in water_query.iter_mut() {
            transform.translation.y = render_config.water_level_offset;
        }
        last_offset.offset = render_config.water_level_offset;
    }
}