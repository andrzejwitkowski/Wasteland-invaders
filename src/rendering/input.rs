use bevy::prelude::*;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, plane_movement_system);
    }
}

#[derive(Component)]
pub struct ControllablePlane;

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