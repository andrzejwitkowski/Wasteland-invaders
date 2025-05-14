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
    // // Spawn a much larger ground plane to ensure full coverage
    // commands.spawn((
    //     Mesh3d(meshes.add(Plane3d::default().mesh().size(200.0, 1000.0))), // Much wider and longer
    //     MeshMaterial3d(materials.add(Color::srgb(0.2, 0.3, 0.8))), // Blue-ish color for water
    //     Transform::from_xyz(0.0, -0.1, -400.0), // Centered, slightly below 0 and extending forward more
    // ));
    
        // info!("Spawning terrain");
        // commands.spawn((
        //     Terrain {
        //         seed: 12345, // Example seed
        //         size: UVec2::new(250, 1000), // Vertices: 250 wide, 500 long
        //         plane_size: Vec2::new(250.0, 500.0), // World units: 250m wide, 500m long
        //         height_scale: 30.0,
        //         frequency: 0.015,
        //         lacunarity: 2.2,
        //         octaves: 7,
        //         persistence: 0.4,
        //         material: materials.add(StandardMaterial { // Assign a material for the terrain
        //             base_color: Color::srgb(1.0, 0.6, 0.25), // A greenish color
        //             metallic: 0.05,
        //             perceptual_roughness: 0.75,
        //             ..default()
        //         }),
        //     },
        //     Transform::from_xyz(0.0, 0.0 ,0.0),
        // ));

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
