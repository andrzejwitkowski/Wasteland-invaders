use bevy::prelude::*;

#[derive(Component)]
pub struct Enemy {
    pub speed: f32,
}

// Helper function to spawn an enemy.
pub fn spawn_enemy(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>   
) -> Entity {
    // Load the enemy GLTF model
    let model_scene_handle: Handle<Scene> = asset_server.load("models/enemy/enemy_plane.gltf#Scene0");
    commands.spawn((
        SceneRoot::from(model_scene_handle),
        Transform::from_xyz(0.0, 2.0, -20.0) // Position in the middle of the scene
            .with_scale(Vec3::new(3.3, 3.3, 3.3))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            Enemy {
                speed: 15.0
            },
    )).id()
}
