use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect::Reflect,
    render::render_resource::{AsBindGroup, ShaderRef},
};
use bevy_egui::{egui, EguiContexts};
use bevy::prelude::AlphaMode;

use crate::rendering::caustic_floor_material::CompleteCausticFloorMaterial;

/// This struct packs the custom shader data into Vec4 fields to ensure a stable
/// and predictable memory layout for the GPU. It must match the struct in the shader.
#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
pub struct ComplexWaterMaterial {
    // .x = wave_amplitude, .y = wave_frequency, .z = wave_speed, .w = wave_steepness
    #[uniform(100)]
    pub wave_params: Vec4,
    
    // .x = foam_intensity, .y = foam_cutoff, .z = transparency, .w = time
    #[uniform(100, visibility(fragment))]
    pub misc_params: Vec4,
}

#[derive(Resource)]
struct WaterConfigUI {
    // Wave parameters
    wave_amplitude: f32,
    wave_frequency: f32,
    wave_speed: f32,
    wave_steepness: f32,
    
    // Misc parameters
    foam_intensity: f32,
    foam_cutoff: f32,
    transparency: f32,
    // time is handled automatically, so we don't expose it in UI

    // Caustic parameters
    caustic_intensity: f32,
    caustic_scale: f32,
    caustic_speed: f32,
    caustic_depth_fade: f32,

    // New crystal clear water controls
    pub water_clarity: f32,
    pub reflectance: f32,
    pub roughness: f32,
    pub refraction_strength: f32,
}
impl WaterConfigUI {
    pub fn apply_crystal_clear_preset(&mut self) {
        self.water_clarity = 0.95;
        self.reflectance = 0.9;
        self.roughness = 0.02;
        self.transparency = 0.9;
        self.wave_amplitude = 0.1;
        self.foam_intensity = 0.3;
    }
    
    pub fn apply_shallow_lagoon_preset(&mut self) {
        self.water_clarity = 0.98;
        self.reflectance = 0.85;
        self.roughness = 0.01;
        self.transparency = 0.95;
        self.wave_amplitude = 0.05;
        self.foam_intensity = 0.1;
    }
}


impl Default for ComplexWaterMaterial {
    fn default() -> Self {
        Self {
            wave_params: Vec4::new(3.0, 0.3, 1.0, 4.0), // amplitude, frequency, speed, steepness
            misc_params: Vec4::new(0.8, 0.3, 0.7, 0.0), // foam_intensity, foam_cutoff, transparency, time
        }
    }
}

impl Default for WaterConfigUI {
    fn default() -> Self {
        Self {
            // Good starting values for realistic water
            wave_amplitude: 3.0,
            wave_frequency: 0.3,
            wave_speed: 1.0,
            wave_steepness: 3.0,
            foam_intensity: 1.0,
            foam_cutoff: 0.7,
            transparency: 0.7,
            caustic_intensity: 1.5,
            caustic_scale: 3.0,
            caustic_speed: 1.0,
            caustic_depth_fade: 0.3,
            water_clarity: 0.95,
            reflectance: 0.9,
            roughness: 0.02,
            refraction_strength: 0.1,
        }
    }
}

impl MaterialExtension for ComplexWaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/simplex_water.wgsl".into() // Make sure this path is correct
    }
    
    fn vertex_shader() -> ShaderRef {
        "shaders/simplex_water.wgsl".into() // Make sure this path is correct
    }
}

// A type alias for the full material, for convenience.
pub type CompleteComplexWaterMaterial = ExtendedMaterial<StandardMaterial, ComplexWaterMaterial>;

// The plugin that registers our custom material.
pub struct ComplexWaterPlugin;

impl Plugin for ComplexWaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CompleteComplexWaterMaterial>::default())
            .add_plugins(MaterialPlugin::<CompleteCausticFloorMaterial>::default()) // Add this
            .add_systems(Update, (
                update_water_time,
                update_caustic_time, // Add this
            ))
            .init_resource::<WaterConfigUI>()
            .add_systems(Update, (
                water_ui_system,
                update_all_water_materials,
                update_all_caustic_materials, // Add this
            ));
    }
}

// This system updates the time component of the misc_params uniform.
pub fn update_water_time(
    time: Res<Time>,
    mut materials: ResMut<Assets<CompleteComplexWaterMaterial>>,
) {
    let elapsed = time.elapsed_secs();
    for (_, material) in materials.iter_mut() {
        material.extension.misc_params.w = elapsed;
    }
}

fn water_ui_system(
    mut contexts: EguiContexts,
    mut config: ResMut<WaterConfigUI>,
) {
    egui::Window::new("Water Controls")
        .default_width(300.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Wave Parameters");
            
            ui.add(egui::Slider::new(&mut config.wave_amplitude, 0.0..=5.0)
                .text("Amplitude")
                .step_by(0.1));
                
            ui.add(egui::Slider::new(&mut config.wave_frequency, 0.1..=3.0)
                .text("Frequency")
                .step_by(0.01));
                
            ui.add(egui::Slider::new(&mut config.wave_speed, 0.0..=3.0)
                .text("Speed")
                .step_by(0.01));
                
            ui.add(egui::Slider::new(&mut config.wave_steepness, 1.0..=10.0)
                .text("Steepness/Octaves")
                .step_by(0.1));
                
            ui.separator();
            ui.heading("Appearance");
            
            ui.add(egui::Slider::new(&mut config.foam_intensity, 0.0..=2.0)
                .text("Foam Intensity")
                .step_by(0.01));
                
            ui.add(egui::Slider::new(&mut config.foam_cutoff, 0.0..=1.0)
                .text("Foam Cutoff")
                .step_by(0.01));
                
            ui.add(egui::Slider::new(&mut config.transparency, 0.0..=1.0)
                .text("Transparency")
                .step_by(0.01));
                
            ui.separator();

            ui.heading("Caustic Effects");
            
            ui.add(egui::Slider::new(&mut config.caustic_intensity, 0.0..=3.0)
                .text("Caustic Intensity")
                .step_by(0.1));
                
            ui.add(egui::Slider::new(&mut config.caustic_scale, 1.0..=10.0)
                .text("Caustic Scale")
                .step_by(0.1));
                
            ui.add(egui::Slider::new(&mut config.caustic_speed, 0.0..=3.0)
                .text("Caustic Speed")
                .step_by(0.1));
                
            ui.add(egui::Slider::new(&mut config.caustic_depth_fade, 0.0..=1.0)
                .text("Depth Fade")
                .step_by(0.01));

            ui.separator();
            ui.heading("Crystal Clear Water");
            
            ui.add(egui::Slider::new(&mut config.water_clarity, 0.0..=1.0)
                .text("Water Clarity")
                .step_by(0.01));
                
            ui.add(egui::Slider::new(&mut config.reflectance, 0.0..=1.0)
                .text("Reflectance")
                .step_by(0.01));
                
            ui.add(egui::Slider::new(&mut config.roughness, 0.0..=0.2)
                .text("Surface Roughness")
                .step_by(0.001));
                
            ui.add(egui::Slider::new(&mut config.refraction_strength, 0.0..=0.5)
                .text("Refraction Strength")
                .step_by(0.01));
            
            // Preset buttons
            ui.heading("Presets");
            ui.horizontal(|ui| {
                if ui.button("Calm Lake").clicked() {
                    config.wave_amplitude = 0.05;
                    config.wave_frequency = 0.2;
                    config.wave_speed = 0.3;
                    config.wave_steepness = 2.0;
                    config.foam_intensity = 0.5;
                    config.foam_cutoff = 0.8;
                    config.transparency = 0.8;
                }
                
                if ui.button("Ocean Waves").clicked() {
                    config.wave_amplitude = 0.3;
                    config.wave_frequency = 0.15;
                    config.wave_speed = 0.6;
                    config.wave_steepness = 4.0;
                    config.foam_intensity = 1.5;
                    config.foam_cutoff = 0.6;
                    config.transparency = 0.5;
                }
            });
            
            ui.horizontal(|ui| {
                if ui.button("Fast Stream").clicked() {
                    config.wave_amplitude = 0.08;
                    config.wave_frequency = 0.8;
                    config.wave_speed = 2.0;
                    config.wave_steepness = 2.0;
                    config.foam_intensity = 0.8;
                    config.foam_cutoff = 0.7;
                    config.transparency = 0.7;
                }
                
                if ui.button("Rough Sea").clicked() {
                    config.wave_amplitude = 0.4;
                    config.wave_frequency = 0.12;
                    config.wave_speed = 0.8;
                    config.wave_steepness = 5.0;
                    config.foam_intensity = 2.0;
                    config.foam_cutoff = 0.5;
                    config.transparency = 0.4;
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Crystal Clear").clicked() {
                    config.apply_crystal_clear_preset();
                }
                
                if ui.button("Shallow Lagoon").clicked() {
                    config.apply_shallow_lagoon_preset();
                }
            });
            
            ui.separator();
            
            // Display current Vec4 values for debugging
            ui.collapsing("Debug Info", |ui| {
                ui.label(format!("wave_params: ({:.2}, {:.2}, {:.2}, {:.2})",
                    config.wave_amplitude, config.wave_frequency, 
                    config.wave_speed, config.wave_steepness));
                ui.label(format!("misc_params: ({:.2}, {:.2}, {:.2}, time)",
                    config.foam_intensity, config.foam_cutoff, config.transparency));
            });
        });
}

fn update_all_water_materials(
    config: Res<WaterConfigUI>,
    mut materials: ResMut<Assets<CompleteComplexWaterMaterial>>,
) {
    if config.is_changed() {
        for (_, material) in materials.iter_mut() {
            // Wave parameters
            material.extension.wave_params = Vec4::new(
                config.wave_amplitude,
                config.wave_frequency,
                config.wave_speed,
                config.wave_steepness,
            );
            
            // Misc parameters including new clarity controls
            material.extension.misc_params = Vec4::new(
                config.water_clarity,
                config.foam_intensity,
                config.foam_cutoff,
                // Time is updated separately
                material.extension.misc_params.w,
            );
            
            // Update the base material for crystal clear properties
            material.base.alpha_mode = AlphaMode::Blend;
            material.base.perceptual_roughness = config.roughness;
            material.base.reflectance = config.reflectance;
            material.base.base_color = Color::srgba(0.0, 0.4, 0.8, config.water_clarity);
        }
    }
}


pub fn update_caustic_time(
    time: Res<Time>,
    mut materials: ResMut<Assets<CompleteCausticFloorMaterial>>,
) {
    let elapsed = time.elapsed_secs();
    for (_, material) in materials.iter_mut() {
        material.extension.misc_params.w = elapsed;
    }
}

fn update_all_caustic_materials(
    config: Res<WaterConfigUI>,
    mut materials: ResMut<Assets<CompleteCausticFloorMaterial>>,
) {
    if config.is_changed() {
        for (_, material) in materials.iter_mut() {
            material.extension.caustic_params = Vec4::new(
                config.caustic_intensity,
                config.caustic_scale,
                config.caustic_speed,
                config.caustic_depth_fade,
            );
            // Keep water parameters in sync
            material.extension.water_params = Vec4::new(
                config.wave_amplitude,
                config.wave_frequency,
                config.wave_speed,
                config.wave_steepness,
            );
        }
    }
}
