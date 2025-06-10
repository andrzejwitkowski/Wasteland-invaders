mod rendering;

use bevy::prelude::*;
use bevy_blendy_cameras::BlendyCamerasPlugin;
use bevy_blendy_cameras::FlyCameraController;
use bevy_blendy_cameras::OrbitCameraController;
use bevy_egui::EguiPlugin;
use rendering::ComplexWaterPlugin;

use crate::rendering::complex_water::CompleteComplexWaterMaterial;
use crate::rendering::complex_water::ComplexWaterMaterial;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin { enable_multipass_for_primary_context: false })
        .add_plugins(BlendyCamerasPlugin)
        .add_plugins(ComplexWaterPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(                                       
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CompleteComplexWaterMaterial>>, // Changed this line
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    let water_mesh_handle = meshes.add(
        Mesh::from(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(200))
            .with_generated_tangents()
            .unwrap()
    );

    // Add the water material to the assets.
    let water_material = materials.add(CompleteComplexWaterMaterial {
        // --- Set Standard PBR properties on the `base` material ---
        base: StandardMaterial {
            base_color: Color::srgb(0.1, 0.4, 0.7),
            alpha_mode: AlphaMode::Blend,
            metallic: 0.0,
            reflectance: 0.8, // Increased for better Fresnel effect
            perceptual_roughness: 0.05, // Very smooth for reflections
            ..default()
        },
        // --- Set your custom water properties on the `extension` ---
        // Calm water: amplitude=0.3, frequency=1.0, speed=1.0, steepness=0.2
        // Moderate waves: amplitude=0.8, frequency=1.5, speed=1.5, steepness=0.5
        // Rough seas: amplitude=1.5, frequency=2.0, speed=2.0, steepness=0.8
        // extension: ComplexWaterMaterial {
        //     wave_params: Vec4::new(0.8, 1.1, 1.1, 0.4),
        //     misc_params: Vec4::new(1.0, 0.7, 0.8, 0.0), // Increased transparency
        // },
        extension: ComplexWaterMaterial {
            // wave_params: [amplitude, frequency, speed, steepness/octaves]
            wave_params: Vec4::new(0.35, 0.3, 1.8, 6.0),
            // misc_params: [unused, unused, transparency, time]
            misc_params: Vec4::new(1.0, 0.7, 0.7, 0.0),
        },
    });

    let floor_mesh = meshes.add(Mesh::from(Plane3d::default().mesh().size(60.0, 60.0)));
    let floor_material = standard_materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.7, 0.6), // Sandy color
        perceptual_roughness: 0.9,
        ..default()
    });
    
    commands.spawn((
        Mesh3d(floor_mesh),
        MeshMaterial3d(floor_material),
        Transform::from_xyz(0.0, -2.0, 0.0), // Below water surface
    ));
    
    commands.spawn((
        Mesh3d(water_mesh_handle),
        MeshMaterial3d(water_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    
    // Add camera
    // commands.spawn((
    //     Camera3d::default(),
    //     Transform::from_xyz(-15.0, 55.0, 15.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    // ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 1.5, 5.0)),
        OrbitCameraController::default(),
        FlyCameraController {
            is_enabled: false,
            ..default()
        },
    ));
    
    // Add light
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.0)),
    ));
}
