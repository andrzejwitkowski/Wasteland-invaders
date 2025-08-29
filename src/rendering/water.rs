use bevy::{
    prelude::*,
    reflect::{Reflect},
    render::render_resource::{AsBindGroup, ShaderRef},
    pbr::{ExtendedMaterial, MaterialExtension},
};

#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
pub struct WaterMaterial {
    #[uniform(100)]
    pub data: Vec4,
}

// Use this instead:
impl MaterialExtension for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/water.wgsl".into()
    }
    
    fn vertex_shader() -> ShaderRef {
        "shaders/water.wgsl".into()
    }
}

// Type alias for the complete water material
pub type CompleteWaterMaterial = ExtendedMaterial<StandardMaterial, WaterMaterial>;

pub struct WaterPlugin;

// Update your plugin to use the extended material
impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CompleteWaterMaterial>::default())
            .add_systems(Update, update_water_time);
    }
}

// And update your time system:
pub fn update_water_time(
    time: Res<Time>,
    mut materials: ResMut<Assets<CompleteWaterMaterial>>,
) {
    let elapsed = time.elapsed_secs();
    for (_, material) in materials.iter_mut() {
        material.extension.data.w = elapsed;
    }
}
