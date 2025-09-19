use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

use crate::heightmap_material::gpu_heightmap_terrain::{CompleteGpuHeightmapMaterial, GpuHeightmapMaterial};

#[derive(Component)]
pub struct GpuHeightmapTerrain;

#[derive(Resource)]
pub struct GpuHeightmapRenderConfig {
    pub chunk_size: f32,
    pub vertex_density: usize,
    pub live_update: bool,
}

#[derive(Resource, Default)]
struct PreviousMeshConfig {
    chunk_size: f32,
    vertex_density: usize,
}

#[derive(Resource, Default)]
pub struct GpuTerrainState {
    pub terrain_entity: Option<Entity>,
}

impl Default for GpuHeightmapRenderConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512.0,
            vertex_density: 257, // Good balance of detail and performance
            live_update: true,
        }
    }
}

pub struct GpuHeightmapRendererPlugin;

impl Plugin for GpuHeightmapRendererPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<GpuHeightmapRenderConfig>()
            .init_resource::<GpuTerrainState>()
            .init_resource::<PreviousMeshConfig>()
            .add_systems(Update, (
                gpu_heightmap_render_ui,
                // auto_update_gpu_terrain,
            ));
    }
}

pub fn gpu_heightmap_render_ui(
    mut contexts: bevy_egui::EguiContexts,
    mut render_config: ResMut<GpuHeightmapRenderConfig>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut gpu_materials: ResMut<Assets<CompleteGpuHeightmapMaterial>>,
    terrain_query: Query<Entity, With<GpuHeightmapTerrain>>,
    terrain_state: Res<GpuTerrainState>,
) {
    bevy_egui::egui::Window::new("GPU Heightmap Renderer")
        .default_width(300.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("GPU Render Settings");
            
            ui.add(bevy_egui::egui::Slider::new(&mut render_config.vertex_density, 64..=513)
                .text("Vertex Density")
                .step_by(32.0));
                
            ui.add(bevy_egui::egui::Slider::new(&mut render_config.chunk_size, 100.0..=1000.0)
                .text("Chunk Size")
                .step_by(10.0));
                
            ui.checkbox(&mut render_config.live_update, "Live Update");
            
            ui.separator();
            
            if ui.button("Render GPU Terrain").clicked() {
                render_gpu_terrain(
                    &mut commands,
                    &mut meshes,
                    &mut gpu_materials,
                    &render_config,
                    &terrain_query,
                );
            }
            
            if ui.button("Clear GPU Terrain").clicked() {
                clear_gpu_terrain(&mut commands, &terrain_query);
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
    gpu_materials: &mut ResMut<Assets<CompleteGpuHeightmapMaterial>>,
    render_config: &GpuHeightmapRenderConfig,
    terrain_query: &Query<Entity, With<GpuHeightmapTerrain>>,
) {
    // Clear existing terrain first
    clear_gpu_terrain(commands, terrain_query);
    
    info!("Generating GPU-based 3D terrain using heightmap shader...");
    
    // Create a simple plane mesh that will be deformed by the vertex shader
    let terrain_mesh = create_gpu_terrain_plane_mesh(render_config);
    let terrain_mesh_handle = meshes.add(terrain_mesh);
    
    // Create GPU heightmap material
    let gpu_material = CompleteGpuHeightmapMaterial {
        base: StandardMaterial {
            base_color: Color::srgb(0.6, 0.5, 0.4),
            perceptual_roughness: 0.8,
            metallic: 0.1,
            reflectance: 0.3,
            ..default()
        },
        extension: GpuHeightmapMaterial::default(),
    };
    
    // Spawn terrain entity with GPU heightmap material
    let terrain_entity = commands.spawn((
        Mesh3d(terrain_mesh_handle),
        MeshMaterial3d(gpu_materials.add(gpu_material)),
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_scale(Vec3::new(render_config.chunk_size, 1.0, render_config.chunk_size)),
        GpuHeightmapTerrain,
    )).id();
    
    // Store the terrain entity for live updates
    commands.insert_resource(GpuTerrainState {
        terrain_entity: Some(terrain_entity),
    });
    
    info!("GPU terrain rendered successfully! Using vertex shader for real-time height displacement.");
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
) {
    for entity in terrain_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    
    commands.insert_resource(GpuTerrainState {
        terrain_entity: None,
    });
    
    info!("GPU terrain cleared.");
}

fn auto_update_gpu_terrain(
    render_config: Res<GpuHeightmapRenderConfig>,
    previous_config: Res<PreviousMeshConfig>,
    terrain_state: Res<GpuTerrainState>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut gpu_materials: ResMut<Assets<CompleteGpuHeightmapMaterial>>,
    terrain_query: Query<Entity, With<GpuHeightmapTerrain>>,
) {
    if !render_config.live_update || terrain_state.terrain_entity.is_none() {
        return;
    }
    
    // Check if mesh-relevant parameters actually changed
    let mesh_changed = render_config.chunk_size != previous_config.chunk_size ||
                      render_config.vertex_density != previous_config.vertex_density;
    
    if mesh_changed {
        info!("GPU terrain configuration changed - regenerating terrain...");
        render_gpu_terrain(
            &mut commands,
            &mut meshes,
            &mut gpu_materials,
            &render_config,
            &terrain_query,
        );
    }
}