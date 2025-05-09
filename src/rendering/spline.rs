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
    
    // Generate 3-4 control points for the spline
    // Start from top of screen
    let mut points = vec![
        Vec3::new(
            rng.random_range(-30.0..30.0), // x random within visible area
            0.1,                           // y slightly above ground level
            0.0                            // z at start
        )
    ];
    
    // Add 1-2 control points in between
    let num_middle_points = rng.random_range(1..=2);
    for i in 0..num_middle_points {
        let t = (i + 1) as f32 / (num_middle_points + 1) as f32;
        points.push(Vec3::new(
            rng.random_range(-30.0..30.0),
            0.1,
            -t * 100.0 // Spread points along Z axis
        ));
    }
    
    // End point at bottom of screen
    points.push(Vec3::new(
        rng.random_range(-30.0..30.0),
        0.1,
        -100.0
    ));

    commands.spawn(Spline {
        control_points: points,
    });
}

fn draw_spline(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    splines: Query<&Spline>,
) {
    for spline in splines.iter() {
        // Draw line segments between control points
        for points in spline.control_points.windows(2) {
            let start = points[0];
            let end = points[1];
            let direction = (end - start).normalize();
            let length = (end - start).length();
            
            // Create a thin box mesh for the line segment
            let line = Cuboid::new(0.2, 0.2, length);
            let rotation = Quat::from_rotation_arc(Vec3::Z, direction);

            /*
                commands.spawn((
                    Mesh3d(meshes.add(Plane3d::default().mesh().size(200.0, 1000.0))), // Much wider and longer
                    MeshMaterial3d(materials.add(Color::srgb(0.2, 0.3, 0.8))), // Blue-ish color for water
                    Transform::from_xyz(0.0, -0.1, -400.0), // Centered, slightly below 0 and extending forward more
                ));
             */

            commands.spawn(
                (
                    Mesh3d(meshes.add(line)),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::BLACK,
                        ..default()
                    })),
                    Transform::from_translation(start + direction * length * 0.5)
                        .with_rotation(rotation)
                ),
            );

        }
    }
}