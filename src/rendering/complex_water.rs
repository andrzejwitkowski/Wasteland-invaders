use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect::{Reflect},
    render::render_resource::{AsBindGroup, ShaderRef},
};
use bevy_egui::{egui, EguiContexts};

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
}

impl Default for ComplexWaterMaterial {
    fn default() -> Self {
        Self {
            wave_params: Vec4::new(0.6, 1.5, 1.5, 0.8), // amplitude, frequency, speed, steepness
            misc_params: Vec4::new(1.0, 0.7, 0.6, 0.0), // foam_intensity, foam_cutoff, transparency, time
        }
    }
}

impl Default for WaterConfigUI {
    fn default() -> Self {
        Self {
            // Good starting values for realistic water
            wave_amplitude: 0.15,
            wave_frequency: 0.3,
            wave_speed: 0.8,
            wave_steepness: 3.0,
            foam_intensity: 1.0,
            foam_cutoff: 0.7,
            transparency: 0.7,
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
            .add_systems(Update, update_water_time)
            .init_resource::<WaterConfigUI>()
            .add_systems(Update, (
                water_ui_system,
                update_all_water_materials,
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
            
            ui.add(egui::Slider::new(&mut config.wave_amplitude, 0.0..=1.5)
                .text("Amplitude")
                .step_by(0.01));
                
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
    time: Res<Time>,
    mut materials: ResMut<Assets<CompleteComplexWaterMaterial>>,
) {
    let elapsed = time.elapsed_secs();
    let config_changed = config.is_changed();
    
    for (_, material) in materials.iter_mut() {
        // Always update time
        material.extension.misc_params.w = elapsed;
        
        // Only update other parameters if config changed (for performance)
        if config_changed {
            material.extension.wave_params = Vec4::new(
                config.wave_amplitude,
                config.wave_frequency,
                config.wave_speed,
                config.wave_steepness,
            );
            
            material.extension.misc_params.x = config.foam_intensity;
            material.extension.misc_params.y = config.foam_cutoff;
            material.extension.misc_params.z = config.transparency;
        }
    }
}
