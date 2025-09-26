use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension}, prelude::*, reflect::Reflect, render::render_resource::{AsBindGroup, ShaderRef}
};
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::heightmap_material::GpuHeightmapRenderConfig;

/// GPU Heightmap material matching the WGSL struct
#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
pub struct GpuHeightmapMaterial {
    // .x = terrain_scale, .y = terrain_amplitude, .z = river_depth, .w = seed
    #[uniform(100)]
    pub terrain_params: Vec4,
    
    // .x = river_width, .y = bank_slope_distance, .z = meander_frequency, .w = meander_amplitude
    #[uniform(100)]
    pub river_params: Vec4,
    
    // .x = erosion_strength, .y = erosion_radius, .z = valley_flattening, .w = erosion_smoothing
    #[uniform(100)]
    pub erosion_params: Vec4,
    
    // .x = flat_area_radius, .y = flat_area_strength, .z = hill_steepness, .w = terrain_roughness
    #[uniform(100)]
    pub terrain_features: Vec4,
    
    // .x = river_start_x, .y = river_start_y, .z = river_dir_x, .w = river_dir_y
    #[uniform(100)]
    pub river_position: Vec4,

    // .x = octaves, .y = lacunarity, .z = persistence, .w = seed
    #[uniform(100)]
    pub noise_config: Vec4,

    #[uniform(100)]
    pub debug_options: Vec4,

    #[texture(101)]
    #[sampler(102)]
    pub terrain_texture: Handle<Image>,
}

#[derive(Resource)]
pub struct GpuHeightmapConfigUI {
    // Terrain parameters
    pub terrain_scale: f32,
    pub terrain_amplitude: f32,
    pub river_depth: f32,
    pub seed: f32,
    
    // River parameters
    pub river_width: f32,
    pub bank_slope_distance: f32,
    pub meander_frequency: f32,
    pub meander_amplitude: f32,
    
    // Erosion parameters
    pub erosion_strength: f32,
    pub erosion_radius: f32,
    pub valley_flattening: f32,
    pub erosion_smoothing: f32,
    
    // Terrain features
    pub flat_area_radius: f32,
    pub flat_area_strength: f32,
    pub hill_steepness: f32,
    pub terrain_roughness: f32,
    
    // River position
    pub river_start_x: f32,
    pub river_start_y: f32,
    pub river_dir_x: f32,
    pub river_dir_y: f32,

    // Noise settings
    pub noise_octaves: i32,
    pub noise_lacunarity: f32,
    pub noise_persistence: f32,
    pub noise_seed: f32,

    // NEW: debug toggle
    pub show_water_mask: bool,
    pub river_margin_rings: u32,
}

impl Default for GpuHeightmapMaterial {
    fn default() -> Self {
        Self {
            terrain_params: Vec4::new(0.005, 50.0, 8.0, 42.0),
            river_params: Vec4::new(20.0, 80.0, 0.008, 40.0),
            erosion_params: Vec4::new(0.8, 120.0, 0.7, 0.6),
            terrain_features: Vec4::new(100.0, 0.8, 1.2, 0.5),
            river_position: Vec4::new(0.0, -200.0, 1.0, 0.2),
            noise_config: Vec4::new(6.0, 2.5, 0.5, 0.0),
            debug_options: Vec4::new(0.0, 0.0, 0.0, 0.0),
            terrain_texture: Handle::default(),
        }
    }
}

impl Default for GpuHeightmapConfigUI {
    fn default() -> Self {
        Self {
            terrain_scale: 0.005,
            terrain_amplitude: 50.0,
            river_depth: 8.0,
            seed: 42.0,
            river_width: 20.0,
            bank_slope_distance: 80.0,
            meander_frequency: 0.008,
            meander_amplitude: 40.0,
            erosion_strength: 0.8,
            erosion_radius: 120.0,
            valley_flattening: 0.7,
            erosion_smoothing: 0.6,
            flat_area_radius: 100.0,
            flat_area_strength: 0.8,
            hill_steepness: 1.2,
            terrain_roughness: 0.5,
            river_start_x: -256.0,
            river_start_y: 0.0,
            river_dir_x: 1.0,
            river_dir_y: 0.1,
            noise_octaves: 6,
            noise_lacunarity: 2.5,
            noise_persistence: 0.5,
            noise_seed: 0.0,
            show_water_mask: false,
            river_margin_rings: 1,
        }
    }
}

impl MaterialExtension for GpuHeightmapMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/heightmap_terrain_2.wgsl".into()
    }
    
    fn vertex_shader() -> ShaderRef {
        "shaders/heightmap_terrain_2.wgsl".into()
    }
}

// Type alias for convenience
pub type CompleteGpuHeightmapMaterial = ExtendedMaterial<StandardMaterial, GpuHeightmapMaterial>;

// Plugin for GPU heightmap terrain
pub struct GpuHeightmapTerrainPlugin;

impl Plugin for GpuHeightmapTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CompleteGpuHeightmapMaterial>::default())
            .init_resource::<GpuHeightmapConfigUI>()
            .add_systems(EguiPrimaryContextPass, gpu_heightmap_ui_system)
            .add_systems(Update, (
                update_all_gpu_heightmap_materials,
            ));
    }
}

fn gpu_heightmap_ui_system(
    mut contexts: EguiContexts,
    mut config: ResMut<GpuHeightmapConfigUI>,
) {
    egui::Window::new("GPU Heightmap Controls")
        .default_width(350.0)
        .show(contexts.ctx_mut().unwrap(), |ui| {

            ui.heading("Noise Settings");

            ui.add(egui::Slider::new(&mut config.noise_octaves, 1..=10)
                .text("Octaves"));

            ui.add(egui::Slider::new(&mut config.noise_lacunarity, 0.1..=2.0)
                .text("Lacunarity"));

            ui.add(egui::Slider::new(&mut config.noise_persistence, 0.0..=1.0)
                .text("Persistence"));

            ui.add(egui::Slider::new(&mut config.noise_seed, 0.0..=1000.0)
                .text("Seed"));

            ui.separator();

            ui.heading("GPU Terrain Parameters");
            
            ui.add(egui::Slider::new(&mut config.terrain_scale, 0.001..=0.02)
                .text("Terrain Scale"));
                
            ui.add(egui::Slider::new(&mut config.terrain_amplitude, 10.0..=1000.0)
                .text("Terrain Amplitude"));
                
            ui.add(egui::Slider::new(&mut config.river_depth, 2.0..=200.0)
                .text("River Depth"));
            
            ui.separator();
            ui.heading("River Parameters");
            
            ui.add(egui::Slider::new(&mut config.river_width, 0.0..=50.0)
                .text("River Width"));
                
            ui.add(egui::Slider::new(&mut config.bank_slope_distance, 30.0..=150.0)
                .text("Bank Slope Distance"));
                
            ui.add(egui::Slider::new(&mut config.meander_frequency, 0.001..=0.02)
                .text("Meander Frequency"));
                
            ui.add(egui::Slider::new(&mut config.meander_amplitude, 0.0..=80.0)
                .text("Meander Amplitude"));
            
            ui.separator();
            ui.heading("Erosion Parameters");
            
            ui.add(egui::Slider::new(&mut config.erosion_strength, 0.0..=1.0)
                .text("Erosion Strength"));
                
            ui.add(egui::Slider::new(&mut config.erosion_radius, 50.0..=200.0)
                .text("Erosion Radius"));
                
            ui.add(egui::Slider::new(&mut config.valley_flattening, 0.0..=1.0)
                .text("Valley Flattening"));
                
            ui.add(egui::Slider::new(&mut config.erosion_smoothing, 0.0..=1.0)
                .text("Erosion Smoothing"));
            
            ui.separator();
            ui.heading("Terrain Features");
            
            ui.add(egui::Slider::new(&mut config.flat_area_radius, 20.0..=200.0)
                .text("Flat Area Radius"));
                
            ui.add(egui::Slider::new(&mut config.flat_area_strength, 0.0..=1.0)
                .text("Flat Area Strength"));
                
            ui.add(egui::Slider::new(&mut config.hill_steepness, 0.5..=3.0)
                .text("Hill Steepness"));
                
            ui.add(egui::Slider::new(&mut config.terrain_roughness, 0.1..=2.0)
                .text("Terrain Roughness"));
            
            ui.separator();
            ui.heading("River Position");
            
            ui.add(egui::Slider::new(&mut config.river_start_x, -500.0..=500.0)
                .text("River Start X"));
                
            ui.add(egui::Slider::new(&mut config.river_start_y, -500.0..=500.0)
                .text("River Start Y"));
                
            ui.add(egui::Slider::new(&mut config.river_dir_x, -1.0..=1.0)
                .text("River Direction X"));
                
            ui.add(egui::Slider::new(&mut config.river_dir_y, -1.0..=1.0)
                .text("River Direction Y"));

            ui.separator();
            ui.heading("Debug");
            ui.checkbox(&mut config.show_water_mask, "Show Water/River Mask");
            ui.add(egui::Slider::new(&mut config.river_margin_rings, 0..=5).text("River Margin Rings"));
        });
}

fn update_all_gpu_heightmap_materials(
    config: Res<GpuHeightmapConfigUI>,
    render_cfg: Option<Res<GpuHeightmapRenderConfig>>,
    mut materials: ResMut<Assets<CompleteGpuHeightmapMaterial>>,
) {

    if!(config.is_changed() || render_cfg.as_ref().map_or(false, |r| r.is_changed())) {
        return;
    }

   let cell_size = render_cfg
        .as_ref()
        .map(|rc| rc.chunk_size / (rc.vertex_density.saturating_sub(1) as f32))
        .unwrap_or(1.0);

    let margin_step_world = cell_size * config.river_margin_rings as f32;

    for (_, material) in materials.iter_mut() {
        material.extension.terrain_params = Vec4::new(
            config.terrain_scale,
            config.terrain_amplitude,
            config.river_depth,
            config.seed,
        );
        material.extension.river_params = Vec4::new(
            config.river_width,
            config.bank_slope_distance,
            config.meander_frequency,
            config.meander_amplitude,
        );
        material.extension.erosion_params = Vec4::new(
            config.erosion_strength,
            config.erosion_radius,
            config.valley_flattening,
            config.erosion_smoothing,
        );
        material.extension.terrain_features = Vec4::new(
            config.flat_area_radius,
            config.flat_area_strength,
            config.hill_steepness,
            config.terrain_roughness,
        );
        material.extension.river_position = Vec4::new(
            config.river_start_x,
            config.river_start_y,
            config.river_dir_x,
            config.river_dir_y,
        );
        material.extension.noise_config = Vec4::new(
            config.noise_octaves as f32,
            config.noise_lacunarity,
            config.noise_persistence,
            config.noise_seed,
        );
        // debug_options: x=show mask, y=margin step, z,w free
        material.extension.debug_options = Vec4::new(
            if config.show_water_mask { 1.0 } else { 0.0 },
            margin_step_world,
            0.0,
            0.0,
        );
    }
}

