use bevy::prelude::*;

pub struct DebugRenderPlugin;

impl Plugin for DebugRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_coordinate_system)
            .add_systems(Startup, spawn_grid_planes);
    }
}

fn spawn_coordinate_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn coordinate system axes
    // X axis (red)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 0.1, 0.1))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
        Transform::from_xyz(5.0, 0.05, 0.0),
    ));

    // Y axis (green)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.1, 10.0, 0.1))),
        MeshMaterial3d(materials.add(Color::srgb(0.0, 1.0, 0.0))),
        Transform::from_xyz(0.0, 5.0, 0.0),
    ));

    // Z axis (blue)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.1, 0.1, 10.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.0, 0.0, 1.0))),
        Transform::from_xyz(0.0, 0.05, 5.0),
    ));
}

fn spawn_grid_plane(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    size: f32,
    color: Color,
    transform: Transform,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(size, size))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        transform,
    ));
}

fn spawn_grid_planes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Add grid planes
    // YZ plane (semi-transparent blue)
    spawn_grid_plane(
        &mut commands,
        &mut meshes,
        &mut materials,
        10.0,
        Color::srgba(0.0, 0.0, 1.0, 0.2),
        Transform::from_xyz(0.0, 5.0, 5.0).with_rotation(Quat::from_rotation_z(4.71)),
    );

    // XY plane (semi-transparent green)
    spawn_grid_plane(
        &mut commands,
        &mut meshes,
        &mut materials,
        10.0,
        Color::srgba(0.0, 1.0, 0.0, 0.2),
        Transform::from_xyz(5.0, 5.0, 0.0)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    );

    // XZ plane (semi-transparent red)
    spawn_grid_plane(
        &mut commands,
        &mut meshes,
        &mut materials,
        10.0,
        Color::srgba(1.0, 0.0, 0.0, 0.2),
        Transform::from_xyz(5.0, 0.0, 5.0)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
    );
}
