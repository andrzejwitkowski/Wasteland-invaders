// Add this after your existing ComplexWaterMaterial

use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect::{Reflect},
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
pub struct CausticFloorMaterial {
    // .x = intensity, .y = scale, .z = speed, .w = depth_fade
    #[uniform(100)]
    pub caustic_params: Vec4,
    
    // Water parameters for surface simulation
    // .x = wave_amplitude, .y = wave_frequency, .z = wave_speed, .w = wave_steepness
    #[uniform(100, visibility(fragment))]
    pub water_params: Vec4,
    
    // .x = water_surface_y, .y = unused, .z = unused, .w = time
    #[uniform(100, visibility(fragment))]
    pub misc_params: Vec4,
}

impl Default for CausticFloorMaterial {
    fn default() -> Self {
        Self {
            caustic_params: Vec4::new(1.5, 3.0, 1.0, 0.3), // intensity, scale, speed, depth_fade
            water_params: Vec4::new(0.35, 0.3, 1.8, 6.0),  // Match water surface parameters
            misc_params: Vec4::new(0.0, 0.0, 0.0, 0.0),    // water_surface_y, unused, unused, time
        }
    }
}

impl MaterialExtension for CausticFloorMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/caustic_floor.wgsl".into()
    }
    
    fn vertex_shader() -> ShaderRef {
        "shaders/caustic_floor.wgsl".into()
    }
}

pub type CompleteCausticFloorMaterial = ExtendedMaterial<StandardMaterial, CausticFloorMaterial>;
