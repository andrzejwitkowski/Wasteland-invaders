use bevy::math::Vec2;
use noise::{NoiseFn, OpenSimplex, Fbm, Seedable};

pub struct TerrainNoise {
    pub base_noise: Fbm<OpenSimplex>,
    pub mountain_noise: Fbm<OpenSimplex>,
    pub detail_noise: OpenSimplex,
    pub valley_noise: OpenSimplex,
}

impl TerrainNoise {
    pub fn new(seed: u32) -> Self {
        let mut base_noise = Fbm::<OpenSimplex>::new(seed);
        base_noise.frequency = 0.01;
        base_noise.lacunarity = 2.0;
        base_noise.persistence = 0.5;
        base_noise.octaves = 3;

        let mut mountain_noise = Fbm::<OpenSimplex>::new(seed.wrapping_add(1));
        mountain_noise.frequency = 0.005;
        mountain_noise.lacunarity = 2.2;
        mountain_noise.persistence = 0.6;
        mountain_noise.octaves = 5;

        let detail_noise = OpenSimplex::new(seed.wrapping_add(2));
        let valley_noise = OpenSimplex::new(seed.wrapping_add(3));

        Self {
            base_noise,
            mountain_noise,
            detail_noise,
            valley_noise,
        }
    }

    pub fn sample_terrain_height(&self, x: f32, z: f32) -> f32 {
        let pos = [x as f64, z as f64];
        
        // Base terrain with rolling hills
        let base_height = self.base_noise.get(pos) as f32;
        
        // Mountain ridges
        let mountain_height = self.mountain_noise.get(pos) as f32;
        let mountain_factor = ((mountain_height + 1.0) * 0.5).powf(2.0);
        
        // Valley carving
        let valley_factor = self.valley_noise.get([x as f64 * 0.008, z as f64 * 0.008]) as f32;
        let valley_carve = (valley_factor * 0.5 + 0.5).max(0.1);
        
        // Detail noise for surface variation
        let detail = self.detail_noise.get([x as f64 * 0.05, z as f64 * 0.05]) as f32 * 0.1;
        
        // Combine all noise layers
        let combined_height = base_height * 0.3 + mountain_height * mountain_factor * 0.7;
        let final_height = combined_height * valley_carve + detail;
        
        final_height
    }

    pub fn sample_terrain_type(&self, x: f32, z: f32, height: f32) -> TerrainType {
        let slope = self.calculate_slope(x, z);
        
        match height {
            h if h > 0.6 => TerrainType::Mountain,
            h if h > 0.2 => {
                if slope > 0.4 {
                    TerrainType::Hill
                } else {
                    TerrainType::Plains
                }
            }
            h if h > -0.2 => TerrainType::Valley,
            _ => TerrainType::Valley,
        }
    }

    fn calculate_slope(&self, x: f32, z: f32) -> f32 {
        let sample_distance = 1.0;
        let height_center = self.sample_terrain_height(x, z);
        let height_right = self.sample_terrain_height(x + sample_distance, z);
        let height_up = self.sample_terrain_height(x, z + sample_distance);
        
        let dx = height_right - height_center;
        let dz = height_up - height_center;
        
        (dx * dx + dz * dz).sqrt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainType {
    Mountain,
    Hill,
    Plains,
    Valley,
    Water,
}
