use bevy::prelude::*;
use std::collections::HashMap;
use super::resources::*;

pub struct RiverCarving;

impl RiverCarving {
    /// Calculate how much terrain should be carved based on distance to river
    pub fn calculate_terrain_influence(
        position: Vec3,
        river_points: &[Vec3],
        config: &RiverConfig,
    ) -> f32 {
        if river_points.len() < 2 {
            return 0.0;
        }
        
        let point_2d = Vec2::new(position.x, position.z);
        let mut min_distance = f32::MAX;
        
        // Find minimum distance to river path
        for window in river_points.windows(2) {
            let start_2d = Vec2::new(window[0].x, window[0].z);
            let end_2d = Vec2::new(window[1].x, window[1].z);
            
            let distance = Self::distance_point_to_line_segment(point_2d, start_2d, end_2d);
            min_distance = min_distance.min(distance);
        }
        
        let carving_depth = Self::calculate_carving_depth(min_distance, config);

        // Debug output for extreme values that might cause artifacts
        if carving_depth > 20.0 {
            println!("Warning: Very deep carving {} at position {:?}, distance: {}", 
                    carving_depth, position, min_distance);
        }
        
        carving_depth
    }
    
    /// Calculate carving depth based on distance from river
    fn calculate_carving_depth(distance: f32, config: &RiverConfig) -> f32 {
        let river_half_width = config.river_width * 0.5;
        let influence_radius = config.river_width * 3.0; // Extend influence beyond river width
        let base_depth = config.river_depth * 3.0; // How deep to carve the banks
        
        if distance <= river_half_width {
            // Inside river - full depth
            base_depth + 2.0
        } else if distance <= influence_radius {
            // River bank area - very smooth gradient falloff
            let falloff_distance = distance - river_half_width;
            let falloff_range = influence_radius - river_half_width;
            let falloff_factor = (falloff_distance / falloff_range).clamp(0.0, 1.0);
            
            // Use quintic smoothing for ultra-smooth transitions
            let smooth_factor = Self::quintic_smooth_step(falloff_factor);
            let remaining_influence = 1.0 - smooth_factor;
            
            // Apply smoothed carving with minimum threshold
            let carved_depth = base_depth * remaining_influence;
            if carved_depth < 0.5 {
                0.0 // Cut off very small influences to avoid noise
            } else {
                carved_depth
            }
        } else {
            // No influence
            0.0
        }
    }
        
    /// Get extended river points for a chunk (including neighboring chunks for smooth transitions)
    pub fn get_extended_river_points_for_chunk(
        chunk_coord: (i32, i32),
        chunk_intersections: &HashMap<(i32, i32), Vec<Vec3>>,
    ) -> Vec<Vec3> {
        let mut extended_points = Vec::new();
        
        // Get points from this chunk and neighboring chunks
        for dx in -2..=2 {
            for dz in -2..=2 {
                let neighbor_coord = (chunk_coord.0 + dx, chunk_coord.1 + dz);
                if let Some(points) = chunk_intersections.get(&neighbor_coord) {
                    extended_points.extend_from_slice(points);
                }
            }
        }
        
        // Sort points to ensure consistent river flow direction
        if !extended_points.is_empty() {
            extended_points.sort_by(|a, b| {
                // Sort by distance from origin to maintain river flow order
                let dist_a = a.x * a.x + a.z * a.z;
                let dist_b = b.x * b.x + b.z * b.z;
                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            });
            
            // Remove duplicate points that are too close together
            extended_points.dedup_by(|a, b| a.distance(*b) < 2.0);
        }
    
        extended_points
    }
    
    /// Calculate distance from point to line segment
    fn distance_point_to_line_segment(point: Vec2, line_start: Vec2, line_end: Vec2) -> f32 {
        let line_vec = line_end - line_start;
        let point_vec = point - line_start;
        
        let line_len_sq = line_vec.length_squared();
        if line_len_sq < 1e-6 {
            return point_vec.length();
        }
        
        let t = (point_vec.dot(line_vec) / line_len_sq).clamp(0.0, 1.0);
        let projection = line_start + line_vec * t;
        point.distance(projection)
    }
    
    fn quintic_smooth_step(x: f32) -> f32 {
        let t = x.clamp(0.0, 1.0);
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }
}