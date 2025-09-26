use bevy::prelude::*;
use noise::{NoiseFn, OpenSimplex, Fbm, Perlin};
use image::{ImageBuffer, Luma, Rgb};
use std::path::Path;

#[derive(Resource, Clone)]
pub struct HeightmapConfig {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub terrain_amplitude: f32,
    pub river_width: f32,
    pub river_depth: f32,
    pub bank_slope_distance: f32,
    pub meander_frequency: f32,
    pub meander_amplitude: f32,
    pub meander_chaos: f32,       
    pub meander_scale_variation: f32, 
    pub flow_irregularity: f32,    
    pub domain_warp_strength: f32,

    pub erosion_strength: f32,     // How much the river erodes terrain
    pub erosion_radius: f32,       // How far erosion extends from river
    pub valley_flattening: f32,    // How much the valley floor is flattened
    pub erosion_smoothing: f32,    // Smoothing factor for eroded areas

    pub flat_area_radius: f32,          // Radius for flat area generation
    pub flat_area_strength: f32,        // How flat the areas should be (0-1)
    pub flat_area_frequency: f32,       // Frequency of flat area occurrence
    pub hill_steepness: f32,            // How steep hills should be
    pub terrain_roughness: f32,  

    pub river_start: Vec2,
    pub river_direction: Vec2,
    pub seed: u32,
}

impl Default for HeightmapConfig {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 1024,
            scale: 0.005,
            terrain_amplitude: 50.0,
            river_width: 20.0,
            river_depth: 8.0,
            bank_slope_distance: 80.0,
            meander_frequency: 0.008,
            meander_amplitude: 40.0,
            meander_chaos: 0.6,
            meander_scale_variation: 0.4,
            flow_irregularity: 0.3,
            domain_warp_strength: 25.0,
            erosion_strength: 0.8,    
            erosion_radius: 120.0,     
            valley_flattening: 0.7,    
            erosion_smoothing: 0.6,   
            flat_area_radius: 100.0,
            flat_area_strength: 0.8,
            flat_area_frequency: 0.002,
            hill_steepness: 1.2,
            terrain_roughness: 0.5,
            river_start: Vec2::new(-256.0, 0.0),
            river_direction: Vec2::new(1.0, 0.1),
            seed: 42,
        }
    }
}

#[derive(Resource)]
pub struct HeightmapNoise {
    pub terrain_base: Fbm<OpenSimplex>,
    pub terrain_detail: OpenSimplex,
    pub domain_warp_x: Perlin,
    pub domain_warp_y: Perlin,

    // Enhanced river noise for realistic meandering
    pub river_primary_meander: Perlin,
    pub river_chaos_noise: Fbm<OpenSimplex>, // Chaotic variations
    pub river_scale_noise: Perlin,
    pub river_width_noise: Perlin,
    pub flat_area_noise: Perlin,        // Noise for flat area placement
    pub hill_noise: Fbm<OpenSimplex>,   // Additional noise for hilly areas
}

impl HeightmapNoise {
    pub fn new(seed: u32) -> Self {
        let mut terrain_base = Fbm::<OpenSimplex>::new(seed);
        terrain_base.frequency = 0.01;
        terrain_base.lacunarity = 2.0;
        terrain_base.persistence = 0.5;
        terrain_base.octaves = 4;

        let mut river_chaos_noise = Fbm::<OpenSimplex>::new(seed + 6);
        river_chaos_noise.frequency = 0.02;
        river_chaos_noise.lacunarity = 2.5;
        river_chaos_noise.persistence = 0.4;
        river_chaos_noise.octaves = 3;

        let mut hill_noise = Fbm::<OpenSimplex>::new(seed + 20);
        hill_noise.frequency = 0.008;
        hill_noise.lacunarity = 2.2;
        hill_noise.persistence = 0.6;
        hill_noise.octaves = 5;

        Self {
            terrain_base,
            terrain_detail: OpenSimplex::new(seed + 1),
            domain_warp_x: Perlin::new(seed + 2),
            domain_warp_y: Perlin::new(seed + 3),
            river_primary_meander: Perlin::new(seed + 4),
            river_chaos_noise,
            river_scale_noise: Perlin::new(seed + 7),
            river_width_noise: Perlin::new(seed + 9),
            flat_area_noise: Perlin::new(seed + 18),
            hill_noise,
        }
    }

    pub fn generate_heightmap(&self, config: &HeightmapConfig) -> Vec<Vec<f32>> {
        let mut heightmap = vec![vec![0.0; config.width as usize]; config.height as usize];
        
        let world_size = 512.0; // World units the heightmap represents
        let pixel_to_world = world_size / config.width as f32;
        
        for y in 0..config.height {
            for x in 0..config.width {
                let world_x = (x as f32 - config.width as f32 * 0.5) * pixel_to_world;
                let world_z = (y as f32 - config.height as f32 * 0.5) * pixel_to_world;
                
                let height = self.sample_height_with_river(world_x, world_z, config);
                heightmap[y as usize][x as usize] = height;
            }
        }
        
        heightmap
    }

    pub fn sample_height_with_river(&self, x: f32, z: f32, config: &HeightmapConfig) -> f32 {
        // Apply domain warping to terrain
        let warp_x = self.domain_warp_x.get([x as f64 * 0.003, z as f64 * 0.003]) as f32 * config.domain_warp_strength;
        let warp_z = self.domain_warp_y.get([x as f64 * 0.003, z as f64 * 0.003]) as f32 * config.domain_warp_strength;
        
        let warped_x = x + warp_x;
        let warped_z = z + warp_z;
        
        // Generate base terrain height
        let base_terrain_height = self.sample_enhanced_terrain_height(warped_x, warped_z, config);
        
        // Calculate river effects (erosion + carving)
        let (river_modification, erosion_factor) = self.calculate_river_effects(Vec2::new(x, z), config);
        
        // Apply erosion to smooth and flatten terrain under river influence
        let eroded_terrain_height = self.apply_erosion_effects(
            base_terrain_height, 
            Vec2::new(x, z), 
            erosion_factor, 
            config
        );
        
        eroded_terrain_height + river_modification
    }

    pub fn calculate_river_effects(&self, position: Vec2, config: &HeightmapConfig) -> (f32, f32) {
        let relative_pos = position - config.river_start;
        let base_river_direction = config.river_direction.normalize();
        let distance_along_river = relative_pos.dot(base_river_direction);
        
        // Generate meander offset
        let meander_offset = self.calculate_realistic_meander(distance_along_river, config);
        
        // Calculate river center with meandering
        let perpendicular = Vec2::new(-base_river_direction.y, base_river_direction.x);
        let river_center = config.river_start + base_river_direction * distance_along_river + perpendicular * meander_offset;
        
        // Distance from point to river centerline
        let distance_to_river = position.distance(river_center);
        
        // Calculate variable river width
        let width_noise = self.river_width_noise.get([
            distance_along_river as f64 * 0.0005,
            0.0
        ]) as f32;
        let actual_river_width = config.river_width * (1.0 + width_noise * 0.3);
        
        // Calculate river profile (carving)
        let river_carving = self.calculate_river_profile(distance_to_river, actual_river_width, config);
        
        // Calculate erosion factor (how much terrain is eroded/smoothed)
        let erosion_factor = self.calculate_erosion_factor(distance_to_river, actual_river_width, config);
        
        (river_carving, erosion_factor)
    }

    fn calculate_erosion_factor(&self, distance_to_river: f32, river_width: f32, config: &HeightmapConfig) -> f32 {
        let water_edge = river_width * 0.5;
        let erosion_end = water_edge + config.erosion_radius;
        
        if distance_to_river <= water_edge {
            // Maximum erosion in river channel
            config.erosion_strength
        } else if distance_to_river <= erosion_end {
            // Gradual erosion falloff
            let erosion_progress = (distance_to_river - water_edge) / config.erosion_radius;
            let falloff = (1.0 - erosion_progress).powf(2.0); // Quadratic falloff
            config.erosion_strength * falloff
        } else {
            // No erosion
            0.0
        }
    }

    fn apply_erosion_effects(&self, base_height: f32, position: Vec2, erosion_factor: f32, config: &HeightmapConfig) -> f32 {
        if erosion_factor <= 0.0 {
            return base_height;
        }
        
        // Calculate target elevation for valley floor
        let valley_target_height = self.calculate_valley_floor_height(position, config);
        
        // Smooth the terrain towards valley floor
        let flattened_height = base_height * (1.0 - config.valley_flattening * erosion_factor) + 
                              valley_target_height * config.valley_flattening * erosion_factor;
        
        // Apply smoothing by reducing high-frequency terrain variations
        let smoothed_height = self.apply_terrain_smoothing(flattened_height, position, erosion_factor, config);
        
        smoothed_height
    }

    fn calculate_valley_floor_height(&self, position: Vec2, config: &HeightmapConfig) -> f32 {
        // Create a gentle, smooth valley floor that follows the general terrain slope
        // but removes high-frequency variations
        
        // Sample terrain at a much lower frequency for valley floor baseline
        let valley_base = self.terrain_base.get([
            position.x as f64 * config.scale as f64 * 0.3,  // Much lower frequency
            position.y as f64 * config.scale as f64 * 0.3
        ]) as f32;
        
        // Create gentle slope along river direction
        let relative_pos = position - config.river_start;
        let distance_along_river = relative_pos.dot(config.river_direction.normalize());
        let river_slope = distance_along_river * 0.001; // Very gentle slope
        
        (valley_base * config.terrain_amplitude * 0.3) + river_slope
    }

    fn apply_terrain_smoothing(&self, height: f32, position: Vec2, erosion_factor: f32, config: &HeightmapConfig) -> f32 {
        // Reduce high-frequency noise in eroded areas
        let smoothing_strength = config.erosion_smoothing * erosion_factor;
        
        // Sample multiple nearby points for averaging (simulating erosion smoothing)
        let sample_radius = 2.0;
        let mut height_sum = height;
        let mut sample_count = 1.0;
        
        // Sample in a small circle around the point
        for i in 0..4 {
            let angle = (i as f32 / 4.0) * std::f32::consts::TAU;
            let sample_x = position.x + angle.cos() * sample_radius;
            let sample_z = position.y + angle.sin() * sample_radius;
            
            let sample_height = self.sample_terrain_height(sample_x, sample_z, config);
            height_sum += sample_height;
            sample_count += 1.0;
        }
        
        let averaged_height = height_sum / sample_count;
        
        // Blend between original and smoothed height based on smoothing strength
        height * (1.0 - smoothing_strength) + averaged_height * smoothing_strength
    }

    fn sample_terrain_height(&self, x: f32, z: f32, config: &HeightmapConfig) -> f32 {
        let pos = [x as f64 * config.scale as f64, z as f64 * config.scale as f64];
        
        // Base terrain
        let base = self.terrain_base.get(pos) as f32;
        
        // Detail layer
        let detail = self.terrain_detail.get([x as f64 * 0.05, z as f64 * 0.05]) as f32 * 0.1;
        
        (base + detail) * config.terrain_amplitude
    }

    pub fn calculate_river_modification(&self, position: Vec2, config: &HeightmapConfig) -> f32 {
        // Use consistent coordinate system - no additional transformations
        let relative_pos = position - config.river_start;
        let base_river_direction = config.river_direction.normalize();
        
        // Simplified flow variation to avoid artifacts
        let distance_along_river = relative_pos.dot(base_river_direction);
        
        // Generate meander offset with consistent sampling
        let meander_offset = self.calculate_realistic_meander(distance_along_river, config);
        
        // Calculate perpendicular direction for meandering offset
        let perpendicular = Vec2::new(-base_river_direction.y, base_river_direction.x);
        let river_center = config.river_start + base_river_direction * distance_along_river + perpendicular * meander_offset;
        
        // Distance from point to river centerline
        let distance_to_river = position.distance(river_center);
        
        // Simplified width variation
        let width_noise = self.river_width_noise.get([
            distance_along_river as f64 * 0.0005,
            0.0  // Remove secondary coordinate dependency
        ]) as f32;
        
        let actual_river_width = config.river_width * (1.0 + width_noise * 0.3);
        
        self.calculate_river_profile(distance_to_river, actual_river_width, config)
    }
    
    fn calculate_realistic_meander(&self, distance_along_river: f32, config: &HeightmapConfig) -> f32 {
        let meander_phase = distance_along_river * config.meander_frequency;
        
        // Primary meandering - base sine wave
        let primary_meander = (meander_phase * std::f32::consts::TAU).sin();
        
        // Secondary meandering with consistent sampling
        let secondary_phase = distance_along_river * config.meander_frequency * 1.7;
        let secondary_meander = (secondary_phase * std::f32::consts::TAU).sin() * 0.4;
        
        // Simplified chaotic variations - only along river flow
        let chaos_variation = self.river_chaos_noise.get([
            distance_along_river as f64 * 0.001,
            0.0  // Remove cross-river coordinate dependency
        ]) as f32;
        
        // Scale variation
        let scale_variation = self.river_scale_noise.get([
            distance_along_river as f64 * 0.0003,
            0.0
        ]) as f32;
        let scale_factor = 1.0 + scale_variation * config.meander_scale_variation;
        
        // Simplified asymmetric variations
        let asymmetry = self.river_primary_meander.get([
            meander_phase as f64 * 0.8,
            1000.0
        ]) as f32;
        
        // Combine components with cleaner math
        let base_meander = primary_meander * 0.7 + secondary_meander * 0.3;
        let chaotic_component = chaos_variation * config.meander_chaos * 0.5;
        let asymmetric_component = asymmetry * 0.2;
        
        let total_meander = (base_meander + chaotic_component + asymmetric_component) * scale_factor;
        
        total_meander * config.meander_amplitude
    }

    fn calculate_river_profile(&self, distance_to_river: f32, river_width: f32, config: &HeightmapConfig) -> f32 {
        let water_edge = river_width * 0.5;
        let bank_end = water_edge + config.bank_slope_distance;
        
        if distance_to_river <= water_edge {
            // River bed - flat bottom
            -config.river_depth
        } else if distance_to_river <= bank_end {
            // River banks with smooth transition using multiple curves
            let bank_progress = (distance_to_river - water_edge) / config.bank_slope_distance;
            
            // Ultra-smooth transition using combined smoothing functions
            let smooth1 = 1.0 - bank_progress.powi(3);  // Cubic easing
            let smooth2 = ((1.0 - bank_progress) * std::f32::consts::PI * 0.5).sin();  // Sine wave
            let smooth3 = (1.0 + (bank_progress * std::f32::consts::PI).cos()) * 0.5;  // Cosine wave
            
            // Combine smoothing functions for ultra-smooth banks
            let combined_smooth = smooth1 * 0.5 + smooth2 * 0.3 + smooth3 * 0.2;
            -config.river_depth * combined_smooth
        } else {
            // No river influence
            0.0
        }
    }

    fn sample_enhanced_terrain_height(&self, x: f32, z: f32, config: &HeightmapConfig) -> f32 {
        let pos = [x as f64 * config.scale as f64, z as f64 * config.scale as f64];
        
        // Base terrain with increased hill steepness
        let mut base = self.terrain_base.get(pos) as f32;
        base = base.abs().powf(config.hill_steepness) * base.signum(); // Enhance hills
        
        // Additional hill noise for more varied topography
        let hill_detail = self.hill_noise.get([
            x as f64 * config.scale as f64 * 2.0,
            z as f64 * config.scale as f64 * 2.0
        ]) as f32 * 0.3 * config.terrain_roughness;
        
        // Detail layer with roughness control
        let detail = self.terrain_detail.get([x as f64 * 0.05, z as f64 * 0.05]) as f32 * 
                    0.1 * config.terrain_roughness;
        
        // Apply flat area masking
        let flat_mask = self.calculate_flat_area_mask(x, z, config);
        let enhanced_terrain = (base + hill_detail + detail) * config.terrain_amplitude;
        
        // Blend between enhanced terrain and flattened version
        enhanced_terrain * (1.0 - flat_mask) + (enhanced_terrain * 0.3) * flat_mask
    }

    fn calculate_flat_area_mask(&self, x: f32, z: f32, config: &HeightmapConfig) -> f32 {
        // Generate flat area centers using noise
        let flat_center_value = self.flat_area_noise.get([
            x as f64 * config.flat_area_frequency as f64,
            z as f64 * config.flat_area_frequency as f64
        ]) as f32;
        
        // Threshold to determine if this is a flat area center
        if flat_center_value > 0.6 {
            // Sample nearby points to create smooth circular flat areas
            let mut total_flatness = 0.0;
            let sample_count = 8;
            
            for i in 0..sample_count {
                let angle = (i as f32 / sample_count as f32) * std::f32::consts::TAU;
                let sample_x = x + angle.cos() * config.flat_area_radius * 0.5;
                let sample_z = z + angle.sin() * config.flat_area_radius * 0.5;
                
                let sample_value = self.flat_area_noise.get([
                    sample_x as f64 * config.flat_area_frequency as f64,
                    sample_z as f64 * config.flat_area_frequency as f64
                ]) as f32;
                
                total_flatness += sample_value;
            }
            
            let avg_flatness = total_flatness / sample_count as f32;
            
            // Create smooth falloff from center to edge
            let distance_factor = 1.0 - (flat_center_value - 0.6) / 0.4; // 0.6-1.0 -> 1.0-0.0
            let flat_strength = avg_flatness * distance_factor * config.flat_area_strength;
            
            flat_strength.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

}

pub struct HeightmapGeneratorPlugin;

impl Plugin for HeightmapGeneratorPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<HeightmapConfig>()
            .add_systems(Startup, setup_heightmap_generator)
            .add_systems(Update, heightmap_ui);
    }
}

pub fn setup_heightmap_generator(mut commands: Commands, config: Res<HeightmapConfig>) {
    let noise = HeightmapNoise::new(config.seed);
    commands.insert_resource(noise);
}

pub fn heightmap_ui(
    mut contexts: bevy_egui::EguiContexts,
    mut config: ResMut<HeightmapConfig>,
    mut noise: ResMut<HeightmapNoise>,
) {
    bevy_egui::egui::Window::new("Heightmap Generator")
        .default_width(450.0)
        .show(contexts.ctx_mut().unwrap(), |ui| {
            ui.heading("Terrain Settings");
            
            ui.add(bevy_egui::egui::Slider::new(&mut config.terrain_amplitude, 10.0..=1000.0)
                .text("Terrain Amplitude"));
            
            ui.add(bevy_egui::egui::Slider::new(&mut config.scale, 0.001..=0.02)
                .text("Terrain Scale"));
                
            ui.add(bevy_egui::egui::Slider::new(&mut config.domain_warp_strength, 0.0..=50.0)
                .text("Domain Warp Strength"));
            
            ui.separator();
            ui.heading("River Settings");
            
            ui.add(bevy_egui::egui::Slider::new(&mut config.river_width, 0.0..=50.0)
                .text("River Width"));
                
            ui.add(bevy_egui::egui::Slider::new(&mut config.river_depth, 2.0..=200.0)
                .text("River Depth"));
                
            ui.add(bevy_egui::egui::Slider::new(&mut config.bank_slope_distance, 30.0..=150.0)
                .text("Bank Slope Distance"));
            
            ui.separator();
            ui.heading("Erosion & Valley Formation");
            
            ui.add(bevy_egui::egui::Slider::new(&mut config.erosion_strength, 0.0..=1.0)
                .text("Erosion Strength"));
                
            ui.add(bevy_egui::egui::Slider::new(&mut config.erosion_radius, 50.0..=200.0)
                .text("Erosion Radius"));
                
            ui.add(bevy_egui::egui::Slider::new(&mut config.valley_flattening, 0.0..=1.0)
                .text("Valley Flattening"));
                
            ui.add(bevy_egui::egui::Slider::new(&mut config.erosion_smoothing, 0.0..=1.0)
                .text("Erosion Smoothing"));
            
            ui.separator();
            ui.heading("Meandering & Flow");
                
            ui.add(bevy_egui::egui::Slider::new(&mut config.meander_frequency, 0.001..=0.02)
                .text("Meander Frequency"));
                
            ui.add(bevy_egui::egui::Slider::new(&mut config.meander_amplitude, 0.0..=80.0)
                .text("Meander Amplitude"));
            
            ui.add(bevy_egui::egui::Slider::new(&mut config.meander_chaos, 0.0..=1.0)
                .text("Meander Chaos"));
            
            ui.add(bevy_egui::egui::Slider::new(&mut config.meander_scale_variation, 0.0..=1.0)
                .text("Meander Scale Variation"));
            
            ui.add(bevy_egui::egui::Slider::new(&mut config.flow_irregularity, 0.0..=1.0)
                .text("Flow Irregularity"));

                        ui.separator();
            ui.heading("Hills & Flat Areas");
            
            ui.add(bevy_egui::egui::Slider::new(&mut config.hill_steepness, 0.5..=3.0)
                .text("Hill Steepness"));
                
            ui.add(bevy_egui::egui::Slider::new(&mut config.terrain_roughness, 0.1..=2.0)
                .text("Terrain Roughness"));
            
            ui.add(bevy_egui::egui::Slider::new(&mut config.flat_area_radius, 20.0..=200.0)
                .text("Flat Area Radius"));
                
            ui.add(bevy_egui::egui::Slider::new(&mut config.flat_area_strength, 0.0..=1.0)
                .text("Flat Area Strength"));
                
            ui.add(bevy_egui::egui::Slider::new(&mut config.flat_area_frequency, 0.0005..=0.01)
                .text("Flat Area Frequency"));
            
            ui.separator();
            ui.heading("Generation");
            
            ui.horizontal(|ui| {
                ui.add(bevy_egui::egui::Slider::new(&mut config.width, 256..=2048)
                    .text("Width"));
                ui.add(bevy_egui::egui::Slider::new(&mut config.height, 256..=2048)
                    .text("Height"));
            });
            
            if ui.button("Regenerate Noise").clicked() {
                *noise = HeightmapNoise::new(config.seed);
            }
            
            if ui.button("Generate & Save Heightmap").clicked() {
                generate_and_save_heightmap(&*noise, &*config);
            }
            
            if ui.button("Generate & Save River Mask").clicked() {
                generate_and_save_river_mask(&*noise, &*config);
            }
            
            ui.label(format!("Seed: {}", config.seed));
            if ui.button("Random Seed").clicked() {
                config.seed = rand::random();
                *noise = HeightmapNoise::new(config.seed);
            }
        });
}

pub fn generate_and_save_heightmap(noise: &HeightmapNoise, config: &HeightmapConfig) {
    info!("Generating heightmap {}x{}", config.width, config.height);
    
    let heightmap = noise.generate_heightmap(config);
    
    // Find min/max for normalization
    let mut min_height = f32::MAX;
    let mut max_height = f32::MIN;
    
    for row in &heightmap {
        for &height in row {
            min_height = min_height.min(height);
            max_height = max_height.max(height);
        }
    }
    
    let height_range = max_height - min_height;
    
    // Create grayscale heightmap image
    let mut img_buffer = ImageBuffer::new(config.width, config.height);
    
    for (x, y, pixel) in img_buffer.enumerate_pixels_mut() {
        let height = heightmap[y as usize][x as usize];
        let normalized = ((height - min_height) / height_range * 255.0) as u8;
        *pixel = Luma([normalized]);
    }
    
    let filename = format!("heightmap_{}x{}_{}.png", config.width, config.height, config.seed);
    if let Err(e) = img_buffer.save(&filename) {
        error!("Failed to save heightmap: {}", e);
    } else {
        info!("Heightmap saved as {}", filename);
        info!("Height range: {:.2} to {:.2}", min_height, max_height);
    }
}

pub fn generate_and_save_river_mask(noise: &HeightmapNoise, config: &HeightmapConfig) {
    info!("Generating river mask {}x{}", config.width, config.height);
    
    let world_size = 512.0;
    let pixel_to_world = world_size / config.width as f32;
    
    // Create RGB image for better visualization
    let mut img_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(config.width, config.height);
    
    for (x, y, pixel) in img_buffer.enumerate_pixels_mut() {
        let world_x = (x as f32 - config.width as f32 * 0.5) * pixel_to_world;
        let world_z = (y as f32 - config.height as f32 * 0.5) * pixel_to_world;
        
        // Calculate just the river modification
        let river_mod = noise.calculate_river_modification(Vec2::new(world_x, world_z), config);
        
        if river_mod < -0.1 {
            // Water areas in blue
            let intensity = ((-river_mod / config.river_depth).clamp(0.0, 1.0) * 255.0) as u8;
            *pixel = Rgb([0, intensity / 2, intensity]);
        } else if river_mod < 0.0 {
            // Bank areas in brown/yellow gradient
            let intensity = ((-river_mod * 10.0).clamp(0.0, 1.0) * 255.0) as u8;
            *pixel = Rgb([intensity, intensity / 2, 0]);
        } else {
            // No river influence - white
            *pixel = Rgb([255, 255, 255]);
        }
    }
    
    let filename = format!("river_mask_{}x{}_{}.png", config.width, config.height, config.seed);
    if let Err(e) = img_buffer.save(&filename) {
        error!("Failed to save river mask: {}", e);
    } else {
        info!("River mask saved as {}", filename);
    }
}