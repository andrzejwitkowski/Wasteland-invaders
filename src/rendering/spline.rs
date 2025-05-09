use bevy::prelude::*;
use rand::Rng;
use bevy::math::primitives::Cuboid;

pub struct SplinePlugin;

#[derive(Component)]
struct Spline {
    control_points: Vec<Vec3>,
}

impl Plugin for SplinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_spline)
           .add_systems(Update, draw_spline);
    }
}

fn spawn_spline(mut commands: Commands) {
    let mut rng = rand::rng();
    
    // Generate control points for the Bézier curve
    let mut points = vec![
        Vec3::new(
            rng.random_range(-30.0..30.0),
            0.1,
            0.0
        )
    ];
    
    // Add control points
    let num_control_points = rng.random_range(2..=3);
    for i in 0..num_control_points {
        let t = (i + 1) as f32 / (num_control_points + 1) as f32;
        points.push(Vec3::new(
            rng.random_range(-30.0..30.0),
            0.1,
            -t * 100.0
        ));
    }
    
    points.push(Vec3::new(
        rng.random_range(-30.0..30.0),
        0.1,
        -100.0
    ));

    commands.spawn(Spline {
        control_points: points,
    });
}

// Compute point on a Bézier curve at parameter t
fn bezier_point(control_points: &[Vec3], t: f32) -> Vec3 {
    let n = control_points.len() - 1;
    let mut point = Vec3::ZERO;
    
    for i in 0..=n {
        let binomial = binomial_coefficient(n, i);
        let t_complement = 1.0 - t;
        let factor = binomial as f32 * t_complement.powi((n - i) as i32) * t.powi(i as i32);
        point += control_points[i] * factor;
    }
    
    point
}

// Calculate binomial coefficient (n choose k)
fn binomial_coefficient(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    if k == 0 || k == n {
        return 1;
    }
    
    let k = k.min(n - k); // Use symmetry to reduce calculations
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
    splines: Query<&Spline>,
) {
    for spline in splines.iter() {
        // Number of segments to divide the curve into
        const SEGMENTS: usize = 50;
        
        // Generate points along the Bézier curve
        let mut curve_points = Vec::with_capacity(SEGMENTS + 1);
        for i in 0..=SEGMENTS {
            let t = i as f32 / SEGMENTS as f32;
            curve_points.push(bezier_point(&spline.control_points, t));
        }
        
        // Draw segments between curve points
        for points in curve_points.windows(2) {
            let start = points[0];
            let end = points[1];
            let direction = (end - start).normalize();
            let length = (end - start).length();
            
            // Create a thin box mesh for the segment
            let line = Cuboid::new(0.2, 0.2, length);
            let rotation = Quat::from_rotation_arc(Vec3::Z, direction);
            
            commands.spawn(
                (
                    Mesh3d(meshes.add(line)),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::BLACK,
                        ..default()
                    })),
                    Transform::from_translation(start + direction * length * 0.5)
                        .with_rotation(rotation),
                )
            );
        }
    }
}