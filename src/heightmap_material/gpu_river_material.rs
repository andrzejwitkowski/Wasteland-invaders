use bevy::prelude::*;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::reflect::Reflect;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

use crate::heightmap_material::gpu_heightmap_terrain::GpuHeightmapConfigUI; // adjust path if different

// Material (river‑masked water)
#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
pub struct MaskedRiverWaterMaterial {
    // .x amp .y freq .z speed .w steepness
    #[uniform(100)]
    pub wave_params: Vec4,
    // .x transparency .y foam_intensity .z foam_cutoff .w time
    #[uniform(100)]
    pub misc_params: Vec4,
    // .x river_width .y bank_slope_distance .z meander_freq .w meander_amp
    #[uniform(100)]
    pub river_params: Vec4,
    // .x start_x .y start_y .z dir_x .w dir_y
    #[uniform(100)]
    pub river_position: Vec4,
    // .x terrain_scale .y terrain_amplitude (unused) .z river_depth .w unused
    #[uniform(100)]
    pub terrain_params: Vec4,
}

impl Default for MaskedRiverWaterMaterial {
    fn default() -> Self {
        Self {
            wave_params: Vec4::new(0.05, 0.6, 0.8, 2.0),
            misc_params: Vec4::new(0.9, 0.8, 0.6, 0.0),
            river_params: Vec4::new(20.0, 80.0, 0.008, 40.0),
            river_position: Vec4::new(-256.0, 0.0, 1.0, 0.1),
            terrain_params: Vec4::new(0.005, 50.0, 8.0, 0.0),
        }
    }
}

impl MaterialExtension for MaskedRiverWaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/masked_river_water.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "shaders/masked_river_water.wgsl".into()
    }
}

pub type CompleteMaskedRiverWaterMaterial = ExtendedMaterial<StandardMaterial, MaskedRiverWaterMaterial>;

// Optional runtime config resource (hook into your existing UI if desired)
#[derive(Resource)]
pub struct MaskedRiverWaterConfig {
    pub wave_amplitude: f32,
    pub wave_frequency: f32,
    pub wave_speed: f32,
    pub wave_steepness: f32,
    pub transparency: f32,
    pub foam_intensity: f32,
    pub foam_cutoff: f32,
}
impl Default for MaskedRiverWaterConfig {
    fn default() -> Self {
        Self {
            wave_amplitude: 0.05,
            wave_frequency: 0.6,
            wave_speed: 0.8,
            wave_steepness: 2.0,
            transparency: 0.9,
            foam_intensity: 0.8,
            foam_cutoff: 0.6,
        }
    }
}

// Plugin
pub struct MaskedRiverWaterPlugin;
impl Plugin for MaskedRiverWaterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MaskedRiverWaterConfig>()
            .add_plugins(MaterialPlugin::<CompleteMaskedRiverWaterMaterial>::default())
            .add_systems(Update, (
                sync_masked_river_water_from_heightmap,
                advance_masked_river_water_time,
            ));
    }
}

// Sync params from UI + heightmap config
fn sync_masked_river_water_from_heightmap(
    water_cfg: Res<MaskedRiverWaterConfig>,
    height_cfg: Option<Res<GpuHeightmapConfigUI>>,
    mut materials: ResMut<Assets<CompleteMaskedRiverWaterMaterial>>,
) {
    if !water_cfg.is_changed() && height_cfg.as_ref().map_or(true, |h| !h.is_changed()) {
        return;
    }
    for (_, mat) in materials.iter_mut() {
        mat.extension.wave_params = Vec4::new(
            water_cfg.wave_amplitude,
            water_cfg.wave_frequency,
            water_cfg.wave_speed,
            water_cfg.wave_steepness,
        );
        // keep existing time (w)
        let time = mat.extension.misc_params.w;
        mat.extension.misc_params = Vec4::new(
            water_cfg.transparency,
            water_cfg.foam_intensity,
            water_cfg.foam_cutoff,
            time,
        );
        if let Some(h) = height_cfg.as_ref() {
            mat.extension.river_params = Vec4::new(
                h.river_width,
                h.bank_slope_distance,
                h.meander_frequency,
                h.meander_amplitude,
            );
            mat.extension.river_position = Vec4::new(
                h.river_start_x,
                h.river_start_y,
                h.river_dir_x,
                h.river_dir_y,
            );
            mat.extension.terrain_params = Vec4::new(
                h.terrain_scale,
                h.terrain_amplitude,
                h.river_depth,
                0.0,
            );
        }
    }
}

// Simple time advance
fn advance_masked_river_water_time(
    time: Res<Time>,
    mut materials: ResMut<Assets<CompleteMaskedRiverWaterMaterial>>,
) {
    let dt = time.elapsed_secs();
    for (_, mat) in materials.iter_mut() {
        let mut mp = mat.extension.misc_params;
        mp.w += dt;
        mat.extension.misc_params = mp;
    }
}