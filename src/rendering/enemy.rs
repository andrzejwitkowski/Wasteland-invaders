use bevy::prelude::*;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_enemy);
    }
}

#[derive(Component)]
pub struct Enemy {
    pub speed: f32,
}

fn spawn_enemy(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Load and spawn the enemy GLTF model
    let model_scene = asset_server.load("models/enemy/enemy_plane.gltf#Scene0");
    commands.spawn((
        SceneRoot::from(model_scene),
        Transform::from_xyz(0.0, 2.0, -20.0) // Position in the middle of the scene
            .with_scale(Vec3::new(3.3, 3.3, 3.3))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        Enemy { speed: 15.0 },
    ));
}