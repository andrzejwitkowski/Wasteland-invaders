use bevy::prelude::*;
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(20.0, 35.0, 20.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
        GlobalTransform::default(),
    ));
}