use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect::Reflect,
    render::render_resource::{AsBindGroup, ShaderRef},
};
use bevy_egui::{egui, EguiContexts};

/// This struct packs the custom shader data into Vec4 fields to ensure a stable
/// and predictable memory layout for the GPU. It must match the struct in the shader.
#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
pub struct HeightmapTerrainMaterial {
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
}

#[derive(Resource)]
pub struct HeightmapConfigUI {
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
}

impl Default for HeightmapTerrainMaterial {
    fn default() -> Self {
        Self {
            terrain_params: Vec4::new(0.005, 50.0, 8.0, 42.0),
            river_params: Vec4::new(20.0, 80.0, 0.008, 40.0),
            erosion_params: Vec4::new(0.8, 120.0, 0.7, 0.6),
            terrain_features: Vec4::new(100.0, 0.8, 1.2, 0.5),
            river_position: Vec4::new(-256.0, 0.0, 1.0, 0.1),
        }
    }
}

impl Default for HeightmapConfigUI {
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
        }
    }
}

impl MaterialExtension for HeightmapTerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/heightmap_terrain.wgsl".into()
    }
    
    fn vertex_shader() -> ShaderRef {
        "shaders/heightmap_terrain.wgsl".into()
    }
}

// A type alias for the full material, for convenience.
pub type CompleteHeightmapTerrainMaterial = ExtendedMaterial<StandardMaterial, HeightmapTerrainMaterial>;

// The plugin that registers our custom material.
pub struct HeightmapTerrainPlugin;

impl Plugin for HeightmapTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CompleteHeightmapTerrainMaterial>::default())
            .init_resource::<HeightmapConfigUI>()
            .add_systems(Update, (
                heightmap_terrain_ui_system,
                update_all_heightmap_materials,
            ));
    }
}

fn heightmap_terrain_ui_system(
    mut contexts: EguiContexts,
    mut config: ResMut<HeightmapConfigUI>,
) {
    egui::Window::new("Heightmap Terrain Controls")
        .default_width(350.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Terrain Parameters");
            
            ui.add(egui::Slider::new(&mut config.terrain_scale, 0.001..=0.02)
                .text("Terrain Scale")
                .step_by(0.001));
                
            ui.add(egui::Slider::new(&mut config.terrain_amplitude, 10.0..=1000.0)
                .text("Terrain Amplitude")
                .step_by(1.0));
                
            ui.add(egui::Slider::new(&mut config.river_depth, 2.0..=200.0)
                .text("River Depth")
                .step_by(1.0));
            
            ui.separator();
            ui.heading("River Parameters");
            
            ui.add(egui::Slider::new(&mut config.river_width, 0.0..=50.0)
                .text("River Width")
                .step_by(0.1));
                
            ui.add(egui::Slider::new(&mut config.bank_slope_distance, 30.0..=150.0)
                .text("Bank Slope Distance")
                .step_by(1.0));
                
            ui.add(egui::Slider::new(&mut config.meander_frequency, 0.001..=0.02)
                .text("Meander Frequency")
                .step_by(0.001));
                
            ui.add(egui::Slider::new(&mut config.meander_amplitude, 0.0..=80.0)
                .text("Meander Amplitude")
                .step_by(1.0));
            
            ui.separator();
            ui.heading("Erosion Parameters");
            
            ui.add(egui::Slider::new(&mut config.erosion_strength, 0.0..=1.0)
                .text("Erosion Strength")
                .step_by(0.01));
                
            ui.add(egui::Slider::new(&mut config.erosion_radius, 50.0..=200.0)
                .text("Erosion Radius")
                .step_by(1.0));
                
            ui.add(egui::Slider::new(&mut config.valley_flattening, 0.0..=1.0)
                .text("Valley Flattening")
                .step_by(0.01));
                
            ui.add(egui::Slider::new(&mut config.erosion_smoothing, 0.0..=1.0)
                .text("Erosion Smoothing")
                .step_by(0.01));
            
            ui.separator();
            ui.heading("Terrain Features");
            
            ui.add(egui::Slider::new(&mut config.flat_area_radius, 20.0..=200.0)
                .text("Flat Area Radius")
                .step_by(1.0));
                
            ui.add(egui::Slider::new(&mut config.flat_area_strength, 0.0..=1.0)
                .text("Flat Area Strength")
                .step_by(0.01));
                
            ui.add(egui::Slider::new(&mut config.hill_steepness, 0.5..=3.0)
                .text("Hill Steepness")
                .step_by(0.1));
                
            ui.add(egui::Slider::new(&mut config.terrain_roughness, 0.1..=2.0)
                .text("Terrain Roughness")
                .step_by(0.1));
            
            ui.separator();
            ui.heading("River Position");
            
            ui.add(egui::Slider::new(&mut config.river_start_x, -500.0..=500.0)
                .text("River Start X")
                .step_by(1.0));
                
            ui.add(egui::Slider::new(&mut config.river_start_y, -500.0..=500.0)
                .text("River Start Y")
                .step_by(1.0));
                
            ui.add(egui::Slider::new(&mut config.river_dir_x, -1.0..=1.0)
                .text("River Direction X")
                .step_by(0.01));
                
            ui.add(egui::Slider::new(&mut config.river_dir_y, -1.0..=1.0)
                .text("River Direction Y")
                .step_by(0.01));
            
            ui.separator();
            
            // Display current Vec4 values for debugging
            ui.collapsing("Debug Info", |ui| {
                ui.label(format!("terrain_params: ({:.3}, {:.1}, {:.1}, {:.0})",
                    config.terrain_scale, config.terrain_amplitude, 
                    config.river_depth, config.seed));
                ui.label(format!("river_params: ({:.1}, {:.1}, {:.3}, {:.1})",
                    config.river_width, config.bank_slope_distance, 
                    config.meander_frequency, config.meander_amplitude));
                ui.label(format!("erosion_params: ({:.2}, {:.1}, {:.2}, {:.2})",
                    config.erosion_strength, config.erosion_radius, 
                    config.valley_flattening, config.erosion_smoothing));
                ui.label(format!("terrain_features: ({:.1}, {:.2}, {:.1}, {:.1})",
                    config.flat_area_radius, config.flat_area_strength, 
                    config.hill_steepness, config.terrain_roughness));
                ui.label(format!("river_position: ({:.1}, {:.1}, {:.2}, {:.2})",
                    config.river_start_x, config.river_start_y, 
                    config.river_dir_x, config.river_dir_y));
            });
        });
}

fn update_all_heightmap_materials(
    config: Res<HeightmapConfigUI>,
    mut materials: ResMut<Assets<CompleteHeightmapTerrainMaterial>>,
) {
    if config.is_changed() {
        for (_, material) in materials.iter_mut() {
            // Terrain parameters
            material.extension.terrain_params = Vec4::new(
                config.terrain_scale,
                config.terrain_amplitude,
                config.river_depth,
                config.seed,
            );
            
            // River parameters
            material.extension.river_params = Vec4::new(
                config.river_width,
                config.bank_slope_distance,
                config.meander_frequency,
                config.meander_amplitude,
            );
            
            // Erosion parameters
            material.extension.erosion_params = Vec4::new(
                config.erosion_strength,
                config.erosion_radius,
                config.valley_flattening,
                config.erosion_smoothing,
            );
            
            // Terrain features
            material.extension.terrain_features = Vec4::new(
                config.flat_area_radius,
                config.flat_area_strength,
                config.hill_steepness,
                config.terrain_roughness,
            );
            
            // River position
            material.extension.river_position = Vec4::new(
                config.river_start_x,
                config.river_start_y,
                config.river_dir_x,
                config.river_dir_y,
            );
            
            // Update base material properties for better terrain appearance
            material.base.perceptual_roughness = 0.8;
            material.base.metallic = 0.1;
            material.base.reflectance = 0.3;
        }
    }
}