mod rendering;

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use rendering::CameraPlugin;
use rendering::DebugRenderPlugin;
use rendering::InputPlugin;
use rendering::AnimationPlugin;
use rendering::BulletPlugin;
use rendering::PlanePlugin;
use rendering::EnemySplineFollowerPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin { enable_multipass_for_primary_context: true })
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(DebugRenderPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(InputPlugin)
        .add_plugins(AnimationPlugin)
        .add_plugins(BulletPlugin)
        .add_plugins(PlanePlugin)
        .add_plugins(EnemySplineFollowerPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a much larger ground plane to ensure full coverage
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(200.0, 1000.0))), // Much wider and longer
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.3, 0.8))), // Blue-ish color for water
        Transform::from_xyz(0.0, -0.1, -400.0), // Centered, slightly below 0 and extending forward more
    ));

    // Spawn directional light
    let light_pos = Vec3::new(0.0, 50.0, 0.0);
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            shadows_enabled: true,
            illuminance: 15000.0,
            ..default()
        },
        Transform::from_translation(light_pos).looking_at(Vec3::ZERO, Vec3::Z),
    ));
}
