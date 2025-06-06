use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect::{Reflect},
    render::render_resource::{AsBindGroup, ShaderRef},
};

/// This struct packs the custom shader data into Vec4 fields to ensure a stable
/// and predictable memory layout for the GPU. It must match the struct in the shader.
#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
pub struct ComplexWaterMaterial {
    // .x = wave_amplitude, .y = wave_frequency, .z = wave_speed, .w = wave_steepness
    #[uniform(100)]
    pub wave_params: Vec4,
    
    // .x = foam_intensity, .y = foam_cutoff, .z = transparency, .w = time
    #[uniform(101)]
    pub misc_params: Vec4,
}

impl Default for ComplexWaterMaterial {
    fn default() -> Self {
        Self {
            wave_params: Vec4::new(0.3, 0.8, 1.2, 0.4), // amplitude, frequency, speed, steepness
            misc_params: Vec4::new(1.0, 0.7, 0.6, 0.0), // foam_intensity, foam_cutoff, transparency, time
        }
    }
}

impl MaterialExtension for ComplexWaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/water.wgsl".into() // Make sure this path is correct
    }
    
    fn vertex_shader() -> ShaderRef {
        "shaders/water.wgsl".into() // Make sure this path is correct
    }
}

// A type alias for the full material, for convenience.
pub type CompleteComplexWaterMaterial = ExtendedMaterial<StandardMaterial, ComplexWaterMaterial>;

// The plugin that registers our custom material.
pub struct ComplexWaterPlugin;

impl Plugin for ComplexWaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CompleteComplexWaterMaterial>::default())
            .add_systems(Update, update_water_time);
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