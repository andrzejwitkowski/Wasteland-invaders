use bevy::prelude::*;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::reflect::Reflect;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::heightmap_material::gpu_heightmap_terrain::GpuHeightmapConfigUI;
use crate::heightmap_material::GpuHeightmapRenderConfig;

// Extended material for river‑masked water
#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
pub struct MaskedRiverWaterMaterial {
    // .x amp .y freq .z speed .w steepness
    #[uniform(100)]
    pub wave_params: Vec4,
    // .x water_clarity(alpha base) .y foam_intensity .z foam_cutoff .w time
    #[uniform(100)]
    pub misc_params: Vec4,
    // .x river_width .y bank_slope_distance .z meander_freq .w meander_amp
    #[uniform(100)]
    pub river_params: Vec4,
    // .x start_x .y start_y .z dir_x .w dir_y
    #[uniform(100)]
    pub river_position: Vec4,
    // .x terrain_scale .y terrain_amplitude .z river_depth .w unused
    #[uniform(100)]
    pub terrain_params: Vec4,
    #[uniform(100)]
    pub debug_options: Vec4,
}

impl Default for MaskedRiverWaterMaterial {
    fn default() -> Self {
        Self {
            wave_params: Vec4::new(0.05, 0.6, 0.8, 2.0),
            misc_params: Vec4::new(0.9, 0.8, 0.6, 0.0),
            river_params: Vec4::new(20.0, 80.0, 0.008, 40.0),
            river_position: Vec4::new(-256.0, 0.0, 1.0, 0.1),
            terrain_params: Vec4::new(0.005, 50.0, 8.0, 0.0),
            debug_options: Vec4::ZERO,
        }
    }
}

// Optional: label to ensure ordering after terrain sync
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct RiverWaterSyncSet;

// Optional: label to ensure ordering after terrain sync
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HeightmapMaterialSyncSet;

impl MaterialExtension for MaskedRiverWaterMaterial {
    fn fragment_shader() -> ShaderRef { "shaders/masked_river_water.wgsl".into() }
    fn vertex_shader() -> ShaderRef { "shaders/masked_river_water.wgsl".into() }
}

pub type CompleteMaskedRiverWaterMaterial =
    ExtendedMaterial<StandardMaterial, MaskedRiverWaterMaterial>;

#[derive(Resource)]
pub struct MaskedRiverWaterConfig {
    // Wave
    pub wave_amplitude: f32,
    pub wave_frequency: f32,
    pub wave_speed: f32,
    pub wave_steepness: f32,
    // Appearance
    pub foam_intensity: f32,
    pub foam_cutoff: f32,
    pub water_clarity: f32,
    // PBR extras
    pub reflectance: f32,
    pub roughness: f32,
    pub refraction_strength: f32,
    // Caustic placeholders (not yet used in this shader – kept for parity)
    pub caustic_intensity: f32,
    pub caustic_scale: f32,
    pub caustic_speed: f32,
    pub caustic_depth_fade: f32,
    pub bank_fill_ratio: f32,
}

impl Default for MaskedRiverWaterConfig {
    fn default() -> Self {
        Self {
            wave_amplitude: 3.0,
            wave_frequency: 0.6,
            wave_speed: 0.8,
            wave_steepness: 2.0,
            foam_intensity: 0.8,
            foam_cutoff: 0.6,
            water_clarity: 0.9,
            reflectance: 0.9,
            roughness: 0.03,
            refraction_strength: 0.1,
            caustic_intensity: 1.5,
            caustic_scale: 3.0,
            caustic_speed: 1.0,
            caustic_depth_fade: 0.3,
            bank_fill_ratio: 0.8,
        }
    }
}

impl MaskedRiverWaterConfig {
    pub fn apply_crystal_clear_preset(&mut self) {
        self.water_clarity = 0.95;
        self.reflectance = 0.9;
        self.roughness = 0.02;
        self.wave_amplitude = 0.1;
        self.foam_intensity = 0.3;
    }
    pub fn apply_shallow_lagoon_preset(&mut self) {
        self.water_clarity = 0.98;
        self.reflectance = 0.85;
        self.roughness = 0.01;
        self.wave_amplitude = 0.05;
        self.foam_intensity = 0.1;
    }
}

pub struct MaskedRiverWaterPlugin;
impl Plugin for MaskedRiverWaterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MaskedRiverWaterConfig>()
            .add_plugins(MaterialPlugin::<CompleteMaskedRiverWaterMaterial>::default())
            .add_systems(EguiPrimaryContextPass, masked_river_water_ui_system)
            .add_systems(Update, (
                sync_masked_river_water_from_heightmap,
                advance_masked_river_water_time,
            ));
    }
}

fn masked_river_water_ui_system(
    mut contexts: EguiContexts,
    mut cfg: ResMut<MaskedRiverWaterConfig>,
) -> Result<(), BevyError> {
    let ctx = contexts.ctx_mut()?;
    egui::Window::new("Masked River Water Controls")
        .default_width(320.0)
        .show(ctx, |ui| {
            ui.heading("Wave Parameters");
            ui.add(egui::Slider::new(&mut cfg.wave_amplitude, 0.0..=5.0).text("Amplitude")
            .step_by(0.1));
            ui.add(egui::Slider::new(&mut cfg.wave_frequency, 0.05..=3.0).text("Frequency"));
            ui.add(egui::Slider::new(&mut cfg.wave_speed, 0.0..=3.0).text("Speed"));
            ui.add(egui::Slider::new(&mut cfg.wave_steepness, 1.0..=10.0).text("Steepness"));
            ui.separator();

            ui.heading("Mask / Banks");
            ui.add(egui::Slider::new(&mut cfg.bank_fill_ratio, 0.0..=1.0).text("Bank Fill Ratio"));
            ui.separator();

            ui.heading("Appearance");
            ui.add(egui::Slider::new(&mut cfg.foam_intensity, 0.0..=2.0).text("Foam Intensity"));
            ui.add(egui::Slider::new(&mut cfg.foam_cutoff, 0.0..=1.0).text("Foam Cutoff"));
            ui.add(egui::Slider::new(&mut cfg.water_clarity, 0.0..=1.0).text("Water Clarity"));
            ui.separator();

            ui.heading("PBR / Optical");
            ui.add(egui::Slider::new(&mut cfg.reflectance, 0.0..=1.0).text("Reflectance"));
            ui.add(egui::Slider::new(&mut cfg.roughness, 0.0..=0.2).text("Roughness"));
            ui.add(egui::Slider::new(&mut cfg.refraction_strength, 0.0..=0.5).text("Refraction Strength"));
            ui.separator();

            ui.heading("Caustic (Reserved)");
            ui.add(egui::Slider::new(&mut cfg.caustic_intensity, 0.0..=3.0).text("Intensity"));
            ui.add(egui::Slider::new(&mut cfg.caustic_scale, 1.0..=10.0).text("Scale"));
            ui.add(egui::Slider::new(&mut cfg.caustic_speed, 0.0..=3.0).text("Speed"));
            ui.add(egui::Slider::new(&mut cfg.caustic_depth_fade, 0.0..=1.0).text("Depth Fade"));
            ui.separator();

            ui.heading("Presets");
            ui.horizontal(|ui| {
                if ui.button("Calm Lake").clicked() {
                    cfg.wave_amplitude = 0.05;
                    cfg.wave_frequency = 0.2;
                    cfg.wave_speed = 0.3;
                    cfg.wave_steepness = 2.0;
                    cfg.foam_intensity = 0.5;
                    cfg.foam_cutoff = 0.8;
                    cfg.water_clarity = 0.8;
                }
                if ui.button("Ocean Waves").clicked() {
                    cfg.wave_amplitude = 0.3;
                    cfg.wave_frequency = 0.15;
                    cfg.wave_speed = 0.6;
                    cfg.wave_steepness = 4.0;
                    cfg.foam_intensity = 1.5;
                    cfg.foam_cutoff = 0.6;
                    cfg.water_clarity = 0.5;
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Fast Stream").clicked() {
                    cfg.wave_amplitude = 0.08;
                    cfg.wave_frequency = 0.8;
                    cfg.wave_speed = 2.0;
                    cfg.wave_steepness = 2.0;
                    cfg.foam_intensity = 0.8;
                    cfg.foam_cutoff = 0.7;
                    cfg.water_clarity = 0.7;
                }
                if ui.button("Rough Sea").clicked() {
                    cfg.wave_amplitude = 0.4;
                    cfg.wave_frequency = 0.12;
                    cfg.wave_speed = 0.8;
                    cfg.wave_steepness = 5.0;
                    cfg.foam_intensity = 2.0;
                    cfg.foam_cutoff = 0.5;
                    cfg.water_clarity = 0.4;
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Crystal Clear").clicked() {
                    cfg.apply_crystal_clear_preset();
                }
                if ui.button("Shallow Lagoon").clicked() {
                    cfg.apply_shallow_lagoon_preset();
                }
            });

            ui.collapsing("Debug Values", |ui| {
                ui.label(format!(
                    "wave_params: ({:.2},{:.2},{:.2},{:.2})",
                    cfg.wave_amplitude, cfg.wave_frequency, cfg.wave_speed, cfg.wave_steepness
                ));
                ui.label(format!(
                    "misc_params: ({:.2} clarity, {:.2} foamI, {:.2} foamCut, time)",
                    cfg.water_clarity, cfg.foam_intensity, cfg.foam_cutoff
                ));
            });
        });
    
    Ok(())
}

fn sync_masked_river_water_from_heightmap(
    water_cfg: Res<MaskedRiverWaterConfig>,
    height_cfg: Option<Res<GpuHeightmapConfigUI>>,
    render_cfg: Option<Res<GpuHeightmapRenderConfig>>,
    mut materials: ResMut<Assets<CompleteMaskedRiverWaterMaterial>>,
) {
    if !water_cfg.is_changed()
        && height_cfg.as_ref().map_or(true, |h| !h.is_changed())
        && render_cfg.as_ref().map_or(true, |r| !r.is_changed())
    {
        return;
    }

    let margin_step_world = render_cfg
        .as_ref()
        .zip(height_cfg.as_ref())
        .map(|(rc, hc)| {
            let cell = rc.chunk_size / (rc.vertex_density.saturating_sub(1) as f32);
            cell * hc.river_margin_rings as f32
        })
        .unwrap_or(0.0);

    for (_, mat) in materials.iter_mut() {
        // Wave / misc
        let time = mat.extension.misc_params.w;
        mat.extension.wave_params = Vec4::new(
            water_cfg.wave_amplitude,
            water_cfg.wave_frequency,
            water_cfg.wave_speed,
            water_cfg.wave_steepness,
        );
        mat.extension.misc_params = Vec4::new(
            water_cfg.water_clarity,
            water_cfg.foam_intensity,
            water_cfg.foam_cutoff,
            time,
        );
        mat.extension.debug_options = Vec4::new(
            1.0,
            margin_step_world,
            water_cfg.bank_fill_ratio,
            0.0,
        );

        // PBR base
        mat.base.alpha_mode = AlphaMode::Blend;
        mat.base.perceptual_roughness = water_cfg.roughness;
        mat.base.reflectance = water_cfg.reflectance;
        mat.base.base_color = Color::srgba(0.0, 0.4, 0.8, water_cfg.water_clarity);

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
            // debug_options.x = show mask
            mat.extension.debug_options = Vec4::new(
                if h.show_water_mask { 1.0 } else { 0.0 },
                margin_step_world,
                0.0,
                0.0,
            );
        }
    }
}

fn advance_masked_river_water_time(
    time: Res<Time>,
    mut materials: ResMut<Assets<CompleteMaskedRiverWaterMaterial>>,
) {
    let dt = time.delta_secs();
    for (_, mat) in materials.iter_mut() {
        let mut m = mat.extension.misc_params;
        m.w += dt;
        mat.extension.misc_params = m;
    }
}