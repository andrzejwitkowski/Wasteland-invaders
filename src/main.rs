mod rendering;
mod terrain;
mod riverbank;
mod heightmapgenerator;
mod flyby;
mod heightmap_material;

use bevy::prelude::*;
use bevy_blendy_cameras::BlendyCamerasPlugin;
use bevy_blendy_cameras::FlyCameraController;
use bevy_blendy_cameras::OrbitCameraController;
use bevy_egui::EguiPlugin;
use crate::flyby::FlyByPlugin;
use crate::flyby::RiverRaidCamera;
use crate::heightmap_material::GpuHeightmapRendererPlugin;
use crate::heightmap_material::GpuHeightmapTerrainPlugin;
// Import the component instead
use crate::rendering::ComplexWaterPlugin;

use bevy::input::keyboard::KeyCode;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Wasteland Invaders".to_string(),
                resolution: (1920., 1080.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin { enable_multipass_for_primary_context: false })
        .add_systems(Startup, (
            setup_camera_and_light,
            crate::terrain::systems::setup_terrain_materials,
        ))
        .add_systems(Update, (
            camera_controls,
        ))
        .add_plugins(ComplexWaterPlugin)
        // .add_plugins(HeightmapGeneratorPlugin)
        // .add_plugins(HeightmapRendererPlugin)
        .add_plugins(GpuHeightmapTerrainPlugin)
        .add_plugins(GpuHeightmapRendererPlugin)
        .add_plugins(BlendyCamerasPlugin)
        // .add_plugins(FlyByPlugin)
        .run();
}

fn camera_controls(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<(&mut Transform, &mut OrbitCameraController), (With<Camera3d>, Without<RiverRaidCamera>)>,
    time: Res<Time>,
) {
    for (mut transform, mut orbit_controller) in camera_query.iter_mut() {
        let mut movement = Vec3::ZERO;
        let speed = 500.0 * time.delta_secs();
        
        // Ruch strzałkami
        if keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW) {
            movement += transform.forward().as_vec3() * speed;
        }
        if keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS) {
            movement += transform.back().as_vec3() * speed;
        }
        if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
            movement += transform.left().as_vec3() * speed;
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
            movement += transform.right().as_vec3() * speed;
        }
        
        if keyboard_input.pressed(KeyCode::Space) {
            movement.y += speed;
        }
        if keyboard_input.pressed(KeyCode::ShiftLeft) {
            movement.y -= speed;
        }
        
        transform.translation += movement;
        
        if movement.length_squared() > 0.0 {
            orbit_controller.focus = transform.translation + transform.forward() * 10.0;
        }
    }
}

pub fn setup_camera_and_light(mut commands: Commands) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 250.0, 50.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        OrbitCameraController {
            is_enabled: true, // Start with orbit camera
            ..default()
        },
        FlyCameraController {
            is_enabled: false, // Fly camera disabled initially
            speed: 100.0,
            ..default()
        },  
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}