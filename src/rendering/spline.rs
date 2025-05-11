use bevy::prelude::*;
use rand::{Rng, thread_rng};
use bevy::math::primitives::Cuboid;
use super::enemy::spawn_enemy_with_spline;

pub struct SplinePlugin;

#[derive(Component)]
pub struct Spline {
pub control_points: Vec<Vec3>,
}

impl Plugin for SplinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_spline)
        .add_systems(Startup, draw_spline.after(spawn_spline));
    }
}

fn spawn_spline(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut rng = thread_rng(); 

    // Z coordinates for screen fit (remains the same)
    let z_top_screen = -5.0; 
    let z_bottom_screen = -45.0; // Total depth of 40 units for the spline

    // --- Define X-axis boundaries ---
    // Visible area for control points P1 and P2
    const VISIBLE_SCREEN_X_MIN: f32 = -15.0;
    const VISIBLE_SCREEN_X_MAX: f32 = 15.0;

    // Wider spawn area for start/end points P0 and P3, allowing them to be slightly off-screen
    const SPAWN_AREA_X_MIN: f32 = -20.0; 
    const SPAWN_AREA_X_MAX: f32 = 20.0;

    // --- Dynamic Start (P0) and End (P3) X-coordinates ---
    let start_x = rng.gen_range(SPAWN_AREA_X_MIN..SPAWN_AREA_X_MAX); 
    let end_x = rng.gen_range(SPAWN_AREA_X_MIN..SPAWN_AREA_X_MAX);

    let mut points = vec![
        Vec3::new(start_x, 0.1, z_top_screen) // P0: Start point
    ];

    // --- Stretched Curves with Control Points (P1, P2) Clamped to Visible Area ---
    // Increased horizontal pull strength for wider curves.
    let pull_strength1 = rng.gen_range(20.0..30.0); // Increased strength
    let pull_strength2 = rng.gen_range(20.0..30.0); // Increased strength

    let raw_control1_x: f32;
    let raw_control2_x: f32;

    // Randomly decide the direction of the first pull (left or right for P1)
    // P2 will pull in the opposite direction relative to its anchor (end_x)
    // to maintain the S-shape.
    if rng.gen_bool(0.5) { 
        // P1 pulls left from start_x
        raw_control1_x = start_x - pull_strength1;
        // P2 pulls right from end_x
        raw_control2_x = end_x + pull_strength2;
    } else { 
        // P1 pulls right from start_x
        raw_control1_x = start_x + pull_strength1;
        // P2 pulls left from end_x
        raw_control2_x = end_x - pull_strength2;
    }
    
    // Clamp control points to be within the visible screen area
    let control1_x = raw_control1_x.clamp(VISIBLE_SCREEN_X_MIN, VISIBLE_SCREEN_X_MAX);
    let control2_x = raw_control2_x.clamp(VISIBLE_SCREEN_X_MIN, VISIBLE_SCREEN_X_MAX);
    
    // P1 Z-coordinate: approx 1/3 down the spline's depth
    let p1_z = z_top_screen + 0.33 * (z_bottom_screen - z_top_screen);
    points.push(Vec3::new(
        control1_x, // Clamped X
        0.1,
        p1_z 
    ));

    // P2 Z-coordinate: approx 2/3 down the spline's depth
    let p2_z = z_top_screen + 0.66 * (z_bottom_screen - z_top_screen);
    points.push(Vec3::new(
        control2_x, // Clamped X
        0.1,
        p2_z 
    ));

    // points.push(Vec3::new(
    //     end_x,
    //     0.1,
    //     z_bottom_screen // P3: End point
    // ));

    // commands.spawn(Spline {
    //     control_points: points,
    // });

    // Spawn spline and enemy
    let spline_entity = commands.spawn(
        Spline { control_points: points }
    ).id();
    
    spawn_enemy_with_spline(&mut commands, &asset_server, spline_entity);

}

pub fn bezier_point(control_points: &[Vec3], t: f32) -> Vec3 {
    let n = control_points.len() - 1;
    if n == 0 { 
        return control_points.first().copied().unwrap_or(Vec3::ZERO);
    }
    let mut point = Vec3::ZERO;
    
    for i in 0..=n {
        let binomial = binomial_coefficient(n, i);
        let t_complement = 1.0 - t;
        let factor = binomial as f32 * t_complement.powi((n - i) as i32) * t.powi(i as i32);
        point += control_points[i] * factor;
    }
    
    point
}

fn binomial_coefficient(n: usize, k: usize) -> usize {
    if k > n { return 0; }
    if k == 0 || k == n { return 1; }
    if k > n / 2 { return binomial_coefficient(n, n - k); }
    
    let mut result = 1;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

fn draw_spline(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    splines: Query<&Spline>, // Query for the Spline component data
) {
    // This function will now only run once at startup.
    // It finds the Spline component created by spawn_spline and draws its visuals.
    for spline in splines.iter() {
        // Draw the control points
        for (idx, point_translation) in spline.control_points.iter().enumerate() {
            let cube_size = 2.0; 
            let cube_mesh_handle = meshes.add(Cuboid::new(cube_size, cube_size, cube_size));
            
            let point_color_enum = match idx {
                0 => Color::srgb(0.0, 1.0, 0.0), // Green for P0 (Start)
                1 => Color::srgb(1.0, 0.5, 0.0), // Orange for P1
                2 => Color::srgb(1.0, 0.0, 1.0), // Magenta for P2
                3 => Color::srgb(0.0, 1.0, 1.0), // Cyan for P3 (End)
                _ => Color::srgb(1.0, 0.0, 0.0), // Red fallback
            };

            let point_srgba = point_color_enum.to_srgba(); 
            let emissive_srgb = Color::srgb( 
                point_srgba.red * 0.5,
                point_srgba.green * 0.5,
                point_srgba.blue * 0.5
            );
            let emissive_linear_rgba = emissive_srgb.to_linear(); 

            let material_handle = materials.add(StandardMaterial {
                base_color: point_color_enum, 
                emissive: emissive_linear_rgba, 
                ..default()
            });

            commands.spawn((
                Mesh3d(cube_mesh_handle), // Assuming Mesh3d is your component
                MeshMaterial3d(material_handle), // Assuming MeshMaterial3d is your component
                Transform::from_translation(*point_translation),
            ));
        }

        const SEGMENTS: usize = 50;
        let mut curve_points = Vec::with_capacity(SEGMENTS + 1);
        for i in 0..=SEGMENTS {
            let t = i as f32 / SEGMENTS as f32;
            curve_points.push(bezier_point(&spline.control_points, t));
        }
        
        for window in curve_points.windows(2) {
            let start_point = window[0];
            let end_point = window[1];
            
            let length = (end_point - start_point).length();
            if length < 0.001 { continue; } 

            let direction = (end_point - start_point).normalize_or_zero();
            
            let line_thickness = 0.3; 
            let line_mesh_handle = meshes.add(Cuboid::new(line_thickness, line_thickness, length));
            let rotation = Quat::from_rotation_arc(Vec3::Z, direction); 
            let midpoint = start_point + direction * length * 0.5;
            
            let near_control_point_threshold: f32 = 10.0; 
            let is_near_control_point = spline.control_points.iter().any(|cp| {
                (midpoint - *cp).length_squared() < near_control_point_threshold.powi(2)
            });
            
            let segment_color_enum = if is_near_control_point { 
                Color::srgb(1.0, 1.0, 0.0) // Yellow
            } else {
                Color::srgb(0.1, 0.1, 0.1) // Darker Black/Grey
            };
            
            let segment_material_handle = materials.add(StandardMaterial {
                base_color: segment_color_enum, 
                ..default()
            });

            commands.spawn((
                Mesh3d(line_mesh_handle), // Assuming Mesh3d is your component
                MeshMaterial3d(segment_material_handle), // Assuming MeshMaterial3d is your component
                Transform::from_translation(midpoint)
                    .with_rotation(rotation),
            ));
        }
    }
}
