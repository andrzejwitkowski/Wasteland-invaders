use bevy::prelude::*;

pub struct PlanePlugin;

impl Plugin for PlanePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_plane);
    }
}

#[derive(Component)]
pub struct Plane {
    pub speed: f32,
}

fn spawn_plane(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Load and spawn the GLTF model - positioned at bottom center
    let model_scene = asset_server.load("models/plane.gltf#Scene0");
    commands.spawn((
        SceneRoot::from(model_scene),
        Transform::from_xyz(0.0, 2.0, -5.0)
            .with_scale(Vec3::new(3.3, 3.3, 3.3))
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::PI)),
        Plane { speed: 20.0 },
    ));
}