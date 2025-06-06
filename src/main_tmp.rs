mod rendering;

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use rendering::CameraPlugin;
use rendering::DebugRenderPlugin;
use rendering::InputPlugin;
use rendering::AnimationPlugin;
use rendering::BulletPlugin;
use rendering::EnemySplineFollowerPlugin;
use rendering::fbm_terrain::FbmTerrainPlugin;
use rendering::PlanePlugin;

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
        .add_plugins(FbmTerrainPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(
    mut commands: Commands
) {
    // Spawn directional light
    let light_pos = Vec3::new(0.0, 50.0, 0.0);
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(light_pos).looking_at(Vec3::ZERO, Vec3::Z),
    ));
}
