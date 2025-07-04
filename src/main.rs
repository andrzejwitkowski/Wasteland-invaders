mod rendering;

use bevy::prelude::*;
use bevy_blendy_cameras::BlendyCamerasPlugin;
use bevy_blendy_cameras::FlyCameraController;
use bevy_blendy_cameras::OrbitCameraController;
use bevy_egui::EguiPlugin;
use rendering::ComplexWaterPlugin;

use crate::rendering::caustic_floor_material::CausticFloorMaterial;
use crate::rendering::caustic_floor_material::CompleteCausticFloorMaterial;
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
    mut materials: ResMut<Assets<CompleteComplexWaterMaterial>>,
    mut caustic_materials: ResMut<Assets<CompleteCausticFloorMaterial>>,
) {
    let water_mesh_handle = meshes.add(
        Mesh::from(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(500))
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
        extension: ComplexWaterMaterial {
            // wave_params: [amplitude, frequency, speed, steepness/octaves]
            wave_params: Vec4::new(0.35, 0.3, 1.8, 6.0),
            // misc_params: [unused, unused, transparency, time]
            misc_params: Vec4::new(1.0, 0.7, 0.7, 0.0),
        },
    });

    let caustic_floor_material = caustic_materials.add(CompleteCausticFloorMaterial {
        base: StandardMaterial {
            base_color: Color::srgb(0.8, 0.7, 0.6), // Sandy color
            perceptual_roughness: 0.8,
            metallic: 0.0,
            ..default()
        },
        extension: CausticFloorMaterial {
            caustic_params: Vec4::new(2.0, 3.0, 1.0, 0.2), // intensity, scale, speed, depth_fade
            water_params: Vec4::new(0.35, 0.3, 1.8, 6.0),  // Match your water parameters
            misc_params: Vec4::new(0.0, 0.0, 0.0, 0.0),    // water_surface_y will be set by system
        },
    });

    let floor_mesh = meshes.add(
        Mesh::from(Plane3d::default().mesh().size(60.0, 60.0).subdivisions(500))
    );
    
    commands.spawn((
        Mesh3d(floor_mesh),
        MeshMaterial3d(caustic_floor_material),
        Transform::from_xyz(0.0, -2.0, 0.0),
    ));
    
    commands.spawn((
        Mesh3d(water_mesh_handle),
        MeshMaterial3d(water_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    
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
