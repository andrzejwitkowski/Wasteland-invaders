use bevy::prelude::*;
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    // Position camera high up with a slight angle for 3D perspective
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 40.0, 10.0) // Higher up with small Z offset for angle
            .looking_at(Vec3::new(0.0, 0.0, -20.0), Vec3::Y), // Looking slightly forward
    ));
}