use bevy::prelude::*;
use image::{ImageBuffer, Rgb};
use std::collections::VecDeque;

#[derive(Resource, Clone)]
pub struct EnemyPlacementConfig {
    pub river_threshold: f32,
    pub bank_margin: f32,
    pub min_distance_from_river: f32,
    pub building_radius: f32,
    pub tank_radius: f32,
    pub vehicle_radius: f32,
    pub max_slope: f32,
    pub min_flat_area: f32,
    pub flatness_safety_margin: f32,
}

impl Default for EnemyPlacementConfig {
    fn default() -> Self {
        Self {
            river_threshold: 0.3,
            bank_margin: 8.0,
            min_distance_from_river: 12.0,
            building_radius: 4.0,
            tank_radius: 3.0,
            vehicle_radius: 2.0,
            max_slope: 0.2,
            min_flat_area: 0.7,
            flatness_safety_margin: 1.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RiverAnalysis {
    pub distance_field: Vec<Vec<f32>>,
    pub water_mask: Vec<Vec<bool>>,
    pub exclusion_mask: Vec<Vec<bool>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoneType {
    Building,
    Tank,
    Vehicle,
}

#[derive(Debug, Clone)]
pub struct PlacementZone {
    pub position: Vec2,
    pub zone_type: ZoneType,
    pub suitability_score: f32,
}

#[derive(Debug, Clone)]
pub struct TerrainAnalysis {
    pub height_map: Vec<Vec<f32>>,
    pub slope_map: Vec<Vec<f32>>,
    pub building_flatness_map: Vec<Vec<f32>>,
    pub tank_flatness_map: Vec<Vec<f32>>,
    pub vehicle_flatness_map: Vec<Vec<f32>>,
    pub river_analysis: RiverAnalysis,
}

#[derive(Resource, Clone)]
pub struct EnemyPlacementGenerator {
    pub river_config: EnemyPlacementConfig,
}

impl FromWorld for EnemyPlacementGenerator {
    fn from_world(_world: &mut World) -> Self {
        Self::new()
    }
}

impl EnemyPlacementGenerator {
    pub fn new() -> Self {
        Self {
            river_config: EnemyPlacementConfig::default(),
        }
    }

    pub fn analyze_river_exclusion(
        &self,
        river_mask: &[Vec<f32>],
        width: usize,
        height: usize,
    ) -> RiverAnalysis {
        let mut water_mask = vec![vec![false; width]; height];
        let mut exclusion_mask = vec![vec![false; width]; height];

        for y in 0..height {
            for x in 0..width {
                water_mask[y][x] = river_mask[y][x] >= self.river_config.river_threshold;
            }
        }

        let distance_field = self.calculate_distance_field(&water_mask, width, height);

        for y in 0..height {
            for x in 0..width {
                exclusion_mask[y][x] = water_mask[y][x] 
                    || distance_field[y][x] < self.river_config.bank_margin;
            }
        }

        RiverAnalysis {
            distance_field,
            water_mask,
            exclusion_mask,
        }
    }

    fn calculate_distance_field(
        &self,
        water_mask: &[Vec<bool>],
        width: usize,
        height: usize,
    ) -> Vec<Vec<f32>> {
        let mut distance_field = vec![vec![f32::INFINITY; width]; height];
        let mut queue = VecDeque::new();

        for y in 0..height {
            for x in 0..width {
                if water_mask[y][x] {
                    distance_field[y][x] = 0.0;
                    queue.push_back((x, y));
                }
            }
        }

        let directions = [(0, 1), (1, 0), (0, -1), (-1, 0), (1, 1), (-1, -1), (1, -1), (-1, 1)];
        
        while let Some((x, y)) = queue.pop_front() {
            let current_distance = distance_field[y][x];
            
            for &(dx, dy) in &directions {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let nx = nx as usize;
                    let ny = ny as usize;
                    
                    let step_distance = if dx.abs() + dy.abs() == 2 { 1.414 } else { 1.0 };
                    let new_distance = current_distance + step_distance;
                    
                    if new_distance < distance_field[ny][nx] {
                        distance_field[ny][nx] = new_distance;
                        queue.push_back((nx, ny));
                    }
                }
            }
        }

        distance_field
    }

    fn draw_zone(
        &self,
        img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        center: Vec2,
        radius: f32,
        color: Rgb<u8>,
    ) {
        let center_x = center.x as i32;
        let center_y = center.y as i32;
        let radius_i = radius as i32;

        for dy in -radius_i..=radius_i {
            for dx in -radius_i..=radius_i {
                let x = center_x + dx;
                let y = center_y + dy;
                
                if x >= 0 && x < img.width() as i32 && y >= 0 && y < img.height() as i32 {
                    let distance = ((dx * dx + dy * dy) as f32).sqrt();
                    if distance <= radius {
                        img.put_pixel(x as u32, y as u32, color);
                    }
                }
            }
        }
    }

    pub fn generate_enemy_placement_map(
        &self,
        height_map: &[Vec<f32>],
        river_mask: &[Vec<f32>],
        width: usize,
        height: usize,
    ) -> (Vec<PlacementZone>, TerrainAnalysis) {
        let river_analysis = self.analyze_river_exclusion(river_mask, width, height);
        
        let slope_map = self.calculate_slope_map(height_map, width, height);
        
        let building_flat_radius = (self.river_config.building_radius * self.river_config.flatness_safety_margin) as usize;
        let tank_flat_radius = (self.river_config.tank_radius * self.river_config.flatness_safety_margin) as usize;
        let vehicle_flat_radius = (self.river_config.vehicle_radius * self.river_config.flatness_safety_margin) as usize;
        
        let building_flatness_map = self.calculate_flatness_map(&slope_map, building_flat_radius, width, height);
        let tank_flatness_map = self.calculate_flatness_map(&slope_map, tank_flat_radius, width, height);
        let vehicle_flatness_map = self.calculate_flatness_map(&slope_map, vehicle_flat_radius, width, height);
        
        let terrain_analysis = TerrainAnalysis {
            height_map: height_map.to_vec(),
            slope_map,
            building_flatness_map,
            tank_flatness_map,
            vehicle_flatness_map,
            river_analysis,
        };
        
        let zones = self.find_suitable_zones(&terrain_analysis, width, height);
        
        (zones, terrain_analysis)
    }

    fn calculate_slope_map(
        &self,
        height_map: &[Vec<f32>],
        width: usize,
        height: usize,
    ) -> Vec<Vec<f32>> {
        let mut slope_map = vec![vec![0.0; width]; height];
        let mut max_slope = 0.0;
        let mut slope_values = Vec::new();
        
        for y in 1..height-1 {
            for x in 1..width-1 {
                let gx = (-1.0 * height_map[y-1][x-1] + 1.0 * height_map[y-1][x+1] +
                          -2.0 * height_map[y][x-1]   + 2.0 * height_map[y][x+1] +
                          -1.0 * height_map[y+1][x-1] + 1.0 * height_map[y+1][x+1]) / 8.0;
                
                let gy = (-1.0 * height_map[y-1][x-1] - 2.0 * height_map[y-1][x] - 1.0 * height_map[y-1][x+1] +
                          1.0 * height_map[y+1][x-1] + 2.0 * height_map[y+1][x] + 1.0 * height_map[y+1][x+1]) / 8.0;
                
                let magnitude = (gx * gx + gy * gy).sqrt();
                slope_map[y][x] = magnitude;
                
                if magnitude > max_slope {
                    max_slope = magnitude;
                }
                slope_values.push(magnitude);
            }
        }
        
        slope_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95 = slope_values[(slope_values.len() as f32 * 0.95) as usize];
        
        info!("Slope statistics - Max: {:.4}, 95th percentile: {:.4}", max_slope, p95);
        
        for y in 1..height-1 {
            for x in 1..width-1 {
                slope_map[y][x] = (slope_map[y][x] / p95).min(1.0);
            }
        }
        
        slope_map
    }

    fn calculate_flatness_map(
        &self,
        slope_map: &[Vec<f32>],
        radius: usize,
        width: usize,
        height: usize,
    ) -> Vec<Vec<f32>> {
        let mut flatness_map = vec![vec![0.0; width]; height];
        
        for y in radius..height-radius {
            for x in radius..width-radius {
                let mut total_flatness = 0.0;
                let mut sample_count = 0;
                
                for dy in -(radius as i32)..=radius as i32 {
                    for dx in -(radius as i32)..=radius as i32 {
                        let nx = (x as i32 + dx) as usize;
                        let ny = (y as i32 + dy) as usize;
                        
                        if nx < width && ny < height {
                            let is_flat = slope_map[ny][nx] <= self.river_config.max_slope;
                            total_flatness += if is_flat { 1.0 } else { 0.0 };
                            sample_count += 1;
                        }
                    }
                }
                
                flatness_map[y][x] = total_flatness / sample_count as f32;
            }
        }
        
        flatness_map
    }

    fn find_suitable_zones(
        &self,
        terrain_analysis: &TerrainAnalysis,
        width: usize,
        height: usize,
    ) -> Vec<PlacementZone> {
        let mut zones = Vec::new();

        for y in 5..height-5 {
            for x in 5..width-5 {
                if terrain_analysis.river_analysis.exclusion_mask[y][x] {
                    continue;
                }

                let river_distance = terrain_analysis.river_analysis.distance_field[y][x];
                if river_distance < self.river_config.min_distance_from_river {
                    continue;
                }

                let slope = terrain_analysis.slope_map[y][x];
                let building_flatness = terrain_analysis.building_flatness_map[y][x];
                let tank_flatness = terrain_analysis.tank_flatness_map[y][x];
                let vehicle_flatness = terrain_analysis.vehicle_flatness_map[y][x];
                
                if building_flatness >= self.river_config.min_flat_area && 
                   slope <= self.river_config.max_slope &&
                   river_distance > 20.0 
                {
                    zones.push(PlacementZone {
                        position: Vec2::new(x as f32, y as f32),
                        zone_type: ZoneType::Building,
                        suitability_score: self.calculate_suitability_score(
                            river_distance, slope, building_flatness, 
                            terrain_analysis.height_map[y][x]
                        ),
                    });
                }
                else if tank_flatness >= self.river_config.min_flat_area && 
                        slope <= self.river_config.max_slope &&
                        river_distance > 15.0 
                {
                    zones.push(PlacementZone {
                        position: Vec2::new(x as f32, y as f32),
                        zone_type: ZoneType::Tank,
                        suitability_score: self.calculate_suitability_score(
                            river_distance, slope, tank_flatness,
                            terrain_analysis.height_map[y][x]
                        ),
                    });
                }
                else if vehicle_flatness >= self.river_config.min_flat_area && 
                        slope <= self.river_config.max_slope
                {
                    zones.push(PlacementZone {
                        position: Vec2::new(x as f32, y as f32),
                        zone_type: ZoneType::Vehicle,
                        suitability_score: self.calculate_suitability_score(
                            river_distance, slope, vehicle_flatness,
                            terrain_analysis.height_map[y][x]
                        ),
                    });
                }
            }
        }

        zones.sort_by(|a, b| b.suitability_score.partial_cmp(&a.suitability_score).unwrap());
        zones
    }

    fn calculate_suitability_score(
        &self,
        river_distance: f32,
        slope: f32,
        flatness: f32,
        height: f32,
    ) -> f32 {
        let river_score = (river_distance - self.river_config.min_distance_from_river) / 20.0;
        let slope_score = 1.0 - (slope / self.river_config.max_slope);
        let flatness_score = flatness;
        let height_score = height.min(1.0);
        
        (river_score * 0.25) + 
        (slope_score * 0.3) + 
        (flatness_score * 0.3) + 
        (height_score * 0.15)
    }
    
    pub fn save_terrain_analysis_map(
        &self,
        terrain_analysis: &TerrainAnalysis,
        zones: &[PlacementZone],
        width: usize,
        height: usize,
        filename: &str,
    ) -> Result<(), image::ImageError> {
        let mut img_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width as u32, height as u32);
        let mut zones_img_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width as u32, height as u32);

        for y in 0..height {
            for x in 0..width {
                let color = if terrain_analysis.river_analysis.water_mask[y][x] {
                    Rgb([0, 0, 255])
                } else if terrain_analysis.river_analysis.exclusion_mask[y][x] {
                    Rgb([255, 200, 200])
                } else {
                    let slope_intensity = (terrain_analysis.slope_map[y][x] * 255.0) as u8;
                    let flatness_intensity = (terrain_analysis.building_flatness_map[y][x] * 255.0) as u8;
                    Rgb([slope_intensity, flatness_intensity, 100])
                };
                
                img_buffer.put_pixel(x as u32, y as u32, color);
            }
        }

        for zone in zones {
            let color = match zone.zone_type {
                ZoneType::Building => Rgb([255, 0, 0]),
                ZoneType::Tank => Rgb([255, 165, 0]),
                ZoneType::Vehicle => Rgb([255, 255, 0]),
            };

            let radius = match zone.zone_type {
                ZoneType::Building => self.river_config.building_radius,
                ZoneType::Tank => self.river_config.tank_radius,
                ZoneType::Vehicle => self.river_config.vehicle_radius,
            };

            self.draw_zone(&mut zones_img_buffer, zone.position, radius, color);
        }

        img_buffer.save(filename)
            .and_then(|_| zones_img_buffer.save(filename.replace(".png", "_zones.png")))
    }
}