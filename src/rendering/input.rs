use bevy::prelude::*;
use crate::rendering::bullet::spawn_bullet;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (plane_movement_system, handle_shooting));
    }
}

#[derive(Component)]
pub struct ControllablePlane;

fn handle_shooting(
    keyboard: Res<ButtonInput<KeyCode>>,
    query: Query<&Transform, With<ControllablePlane>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        if let Ok(plane_transform) = query.single() {
            // Spawn bullet slightly below the plane
            let bullet_pos = plane_transform.translation + Vec3::new(0.0, -1.0, 0.0);
            spawn_bullet(&mut commands, &mut meshes, &mut materials, bullet_pos);
        }
    }
}

fn plane_movement_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<ControllablePlane>>,
    time: Res<Time>,
) {
    let movement_speed = 20.0;
    
    for mut transform in query.iter_mut() {
        let mut movement = Vec3::ZERO;

        // Forward/Backward movement
        if keyboard.pressed(KeyCode::ArrowUp) {
            movement.z -= 1.0;
        }
        if keyboard.pressed(KeyCode::ArrowDown) {
            movement.z += 1.0;
        }

        // Left/Right movement
        if keyboard.pressed(KeyCode::ArrowLeft) {
            movement.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::ArrowRight) {
            movement.x += 1.0;
        }

        if movement != Vec3::ZERO {
            transform.translation += movement.normalize() * movement_speed * time.delta_secs();
        }
    }
}