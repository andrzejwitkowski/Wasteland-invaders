use bevy::prelude::*;

pub fn generate_river_curve(
    start_x: f32,
    start_z: f32,
    length: f32,
    frequency: f32,
    amplitude: f32,
    segments: u32,
) -> Vec<Vec3> {
    let mut points = Vec::new();
    
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let x = start_x + t * length;
        
        // Add meandering using sine waves
        let meander_offset = (x * frequency).sin() * amplitude;
        let z = start_z + length * 0.5 + meander_offset;
        
        // Add some noise for more natural curves
        let noise_offset = ((x * 0.3).sin() * (z * 0.2).cos()) * amplitude * 0.3;
        
        points.push(Vec3::new(x, 0.0, z + noise_offset));
    }
    
    points
}

pub fn calculate_curve_normals(curve: &[Vec3]) -> Vec<Vec3> {
    let mut normals = Vec::new();
    
    for i in 0..curve.len() {
        let tangent = if i == 0 {
            curve[1] - curve[0]
        } else if i == curve.len() - 1 {
            curve[i] - curve[i - 1]
        } else {
            (curve[i + 1] - curve[i - 1]) * 0.5
        };
        
        // Calculate perpendicular vector (normal to the curve)
        let normal = Vec3::new(-tangent.z, 0.0, tangent.x).normalize();
        normals.push(normal);
    }
    
    normals
}