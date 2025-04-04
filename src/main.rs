mod rendering;

use bevy::DefaultPlugins;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use rendering::DebugRenderPlugin;
use rendering::CameraPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(DebugRenderPlugin)
        .add_plugins(CameraPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(200.0, 200.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.5, 0.5, 0.5))),
    ));

    //spawn cube above the plane
    let cube_size = 1.0;
    let cube_half_size = cube_size * 0.5;
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.5, 0.5))),
        Transform::from_xyz(8.0, cube_half_size, 8.0),
    ));

    let camera_pos = Vec3::new(20.0, 35.0, 20.0);

    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_translation(camera_pos).looking_at(Vec3::ZERO, Vec3::Y),
     ));
}
