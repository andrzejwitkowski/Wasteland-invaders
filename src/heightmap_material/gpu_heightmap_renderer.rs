use bevy::ecs::error::info;
use bevy::{log, prelude::*};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy_egui::EguiPrimaryContextPass;

use crate::heightmap_material::{CompleteGpuHeightmapMaterial, GpuHeightmapMaterial, MaskedRiverWaterPlugin};
use crate::rendering::complex_water::CompleteComplexWaterMaterial;

#[derive(Component)]
pub struct GpuHeightmapTerrain;

#[derive(Component)]
pub struct GpuHeightmapWater;

#[derive(Resource, Clone)]
pub struct GpuHeightmapRenderConfig {
    pub chunk_size: f32,
    pub vertex_density: usize,
    pub live_update: bool,
    pub water_level_offset: f32,
    pub enable_water_rendering: bool,
}

#[derive(Resource, Default)]
pub struct LastWaterLevelOffset {
    offset: f32,
}

#[derive(Resource, Default)]
pub struct GpuTerrainState {
    pub terrain_entity: Option<Entity>,
}

impl Default for GpuHeightmapRenderConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512.0,
            vertex_density: 257,
            live_update: true,
            water_level_offset: 0.5,
            enable_water_rendering: true,
        }
    }
}
pub struct GpuHeightmapRendererPlugin;

impl Plugin for GpuHeightmapRendererPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<GpuHeightmapRenderConfig>()
            .init_resource::<GpuTerrainState>()
            .init_resource::<LastWaterLevelOffset>()
            .add_systems(EguiPrimaryContextPass, gpu_heightmap_render_ui)
            .add_systems(Update, (
                update_water_level_on_change,
            ));
    }
}

pub fn gpu_heightmap_render_ui(
    mut contexts: bevy_egui::EguiContexts,
    mut render_config: ResMut<GpuHeightmapRenderConfig>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_materials: ResMut<Assets<CompleteGpuHeightmapMaterial>>,
    mut water_materials: ResMut<Assets<CompleteComplexWaterMaterial>>,
    //mut water_materials: ResMut<Assets<MaskedRiverWaterPlugin>>,
    terrain_query: Query<Entity, With<GpuHeightmapTerrain>>,
    water_query: Query<Entity, With<GpuHeightmapWater>>,
    terrain_state: Res<GpuTerrainState>,
) {
    bevy_egui::egui::Window::new("GPU Heightmap Renderer")
        .default_width(300.0)
        .show(contexts.ctx_mut().unwrap(), |ui| {
            ui.heading("GPU Render Settings");
            
            ui.add(bevy_egui::egui::Slider::new(&mut render_config.vertex_density, 64..=513)
                .text("Vertex Density")
                .step_by(32.0));

            ui.separator();
                
            ui.add(bevy_egui::egui::Slider::new(&mut render_config.chunk_size, 100.0..=1000.0)
                .text("Chunk Size")
                .step_by(10.0));
                
            ui.checkbox(&mut render_config.live_update, "Live Update");

            ui.add(bevy_egui::egui::Slider::new(&mut render_config.water_level_offset, -150.0..=15.0)
                .text("Water Level Offset"));
                
            ui.checkbox(&mut render_config.enable_water_rendering, "Render Water");
            
            ui.separator();
            
            if ui.button("Render GPU Terrain").clicked() {
                render_gpu_terrain(
                    &mut commands,
                    &mut meshes,
                    &mut terrain_materials,
                    &mut water_materials,
                    &render_config,
                    &terrain_query,
                    &water_query,
                );
            }
            
            if ui.button("Clear GPU Terrain").clicked() {
                clear_gpu_terrain(&mut commands, &terrain_query, &water_query);
            }
            
            if terrain_state.terrain_entity.is_some() {
                ui.label("✅ GPU Terrain Active");
                ui.label("Changes update in real-time!");
            } else {
                ui.label("❌ No GPU Terrain");
            }
        });
}

fn render_gpu_terrain(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    terrain_materials: &mut ResMut<Assets<CompleteGpuHeightmapMaterial>>,
    water_materials: &mut ResMut<Assets<CompleteComplexWaterMaterial>>,
    render_config: &GpuHeightmapRenderConfig,
    terrain_query: &Query<Entity, With<GpuHeightmapTerrain>>,
    water_query: &Query<Entity, With<GpuHeightmapWater>>,
) {
    // Clear existing terrain first
    clear_gpu_terrain(commands, terrain_query, water_query);
    
    info!("Generating GPU-based 3D terrain using stencil buffer approach...");

    let terrain_mesh = create_gpu_terrain_plane_mesh(render_config);

    let main_terrain_entity = commands.spawn((Name::new("Main Terrain"),)).id();
    
    setup_terrain(
        commands,
        meshes,
        terrain_materials,
        render_config,
        &terrain_mesh,
    );
    
    if render_config.enable_water_rendering {
        setup_water(
            commands,
            meshes,
            water_materials,
            render_config,
        );
    }

    commands.insert_resource(GpuTerrainState {
        terrain_entity: Some(main_terrain_entity),
    });

    info!("GPU terrain rendered successfully with stencil buffer approach!");
}

fn setup_terrain(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<CompleteGpuHeightmapMaterial>>,
    render_config: &GpuHeightmapRenderConfig,
    terrain_mesh: &Mesh,
) {
    let material = CompleteGpuHeightmapMaterial {
        base: StandardMaterial {
            perceptual_roughness: 0.8,
            metallic: 0.1,
            reflectance: 0.3,
            ..Default::default()
        },
        extension: GpuHeightmapMaterial::default(),
    };

    commands.spawn((
        Mesh3d(meshes.add(terrain_mesh.clone())),
        MeshMaterial3d(materials.add(material)),
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_scale(Vec3::new(render_config.chunk_size, 1.0, render_config.chunk_size)),
        GpuHeightmapTerrain,
    ));
}

fn setup_water(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    water_materials: &mut ResMut<Assets<CompleteComplexWaterMaterial>>,
    config: &GpuHeightmapRenderConfig,
) {
    let water_mesh = create_water_plane_mesh(config);
    
    let water_material = CompleteComplexWaterMaterial::default();

    commands.spawn((
        Mesh3d(meshes.add(water_mesh)),
        MeshMaterial3d(water_materials.add(water_material)),
        Transform::from_xyz(0.0, config.water_level_offset, 0.0),
        GpuHeightmapWater,
    ));
}

fn create_water_plane_mesh(render_config: &GpuHeightmapRenderConfig) -> Mesh {
    let width = render_config.vertex_density;
    let height = render_config.vertex_density;
    
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    
    // Create a dense plane for detailed wave displacement
    for z in 0..height {
        for x in 0..width {
            let u = x as f32 / (width - 1) as f32;
            let v = z as f32 / (height - 1) as f32;
            
            // Create vertices in world space
            let x_pos = (u - 0.5) * render_config.chunk_size;
            let z_pos = (v - 0.5) * render_config.chunk_size;
            
            vertices.push([x_pos, 0.0, z_pos]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([u * 10.0, v * 10.0]); // Scale UVs for better texture mapping
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
    
    mesh
}

fn create_gpu_terrain_plane_mesh(render_config: &GpuHeightmapRenderConfig) -> Mesh {
    let width = render_config.vertex_density;
    let height = render_config.vertex_density;
    
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    
    // Create a flat plane that will be deformed by the vertex shader
    for z in 0..height {
        for x in 0..width {
            let u = x as f32 / (width - 1) as f32;
            let v = z as f32 / (height - 1) as f32;
            
            // Create vertices in normalized space (-0.5 to 0.5)
            let x_pos = u - 0.5;
            let z_pos = v - 0.5;
            
            vertices.push([x_pos, 0.0, z_pos]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([u * 10.0, v * 10.0]); // Scale UVs for better texture mapping
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
    
    mesh
}

fn clear_gpu_terrain(
    commands: &mut Commands,
    terrain_query: &Query<Entity, With<GpuHeightmapTerrain>>,
    water_query: &Query<Entity, With<GpuHeightmapWater>>,
) {
    for entity in terrain_query.iter() {
        commands.entity(entity).despawn_recursive();
    }

    for entity in water_query.iter() {
        commands.entity(entity).despawn_recursive();
    }

    commands.insert_resource(GpuTerrainState {
        terrain_entity: None,
    });
    
    info!("GPU terrain cleared.");
}

fn update_water_level_on_change(
    render_config: Res<GpuHeightmapRenderConfig>,
    mut last_offset: ResMut<LastWaterLevelOffset>,
    mut water_query: Query<&mut Transform, With<GpuHeightmapWater>>,
) {
    let offset_diff = (render_config.water_level_offset - last_offset.offset).abs();
    
    if offset_diff > 0.01 && !water_query.is_empty() {
        info!("🌊 Updating water level from {:.2} to {:.2}", 
        last_offset.offset, render_config.water_level_offset);
        

        for mut transform in water_query.iter_mut() {
            log::info!("Water Y set to {}", transform.translation.y);
            transform.translation.y = render_config.water_level_offset;
        }
        
        last_offset.offset = render_config.water_level_offset;
    }
}