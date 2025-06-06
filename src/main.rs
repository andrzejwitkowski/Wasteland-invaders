mod rendering;

use bevy::prelude::*;
use rendering::WaterPlugin;

use crate::rendering::water::CompleteWaterMaterial;
use crate::rendering::water::WaterMaterial;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WaterPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(                                       
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CompleteWaterMaterial>>, // Changed this line
    asset_server: Res<AssetServer>,
) {
    let water_mesh_handle = meshes.add(
        Mesh::from(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(50))
            .with_generated_tangents()
            .unwrap()
    );
    
    // Use CompleteWaterMaterial (ExtendedMaterial)
    let water_material = materials.add(CompleteWaterMaterial {
        base: StandardMaterial {
            base_color: Color::srgba(0.1, 0.3, 0.6, 0.8),
            alpha_mode: AlphaMode::Blend,
            ..default()
        },
        extension: WaterMaterial {
            data: Vec4::new(0.1, 0.3, 0.6, 0.0),
        },
    });
    
    commands.spawn((
        Mesh3d(water_mesh_handle),
        MeshMaterial3d(water_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    
    // Add camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 70.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
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
