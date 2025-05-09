use bevy::prelude::*;
use crate::rendering::plane::Plane;

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlaneAnimationState>()
            .add_systems(Update, plane_swing_animation);
    }
}

#[derive(Resource, Default)]
struct PlaneAnimationState {
    target_roll: f32,
    current_roll: f32,
    initial_rotation: Option<Quat>,
}

fn plane_swing_animation(
    mut query: Query<&mut Transform, With<Plane>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut anim_state: ResMut<PlaneAnimationState>,
) {
    // Define animation parameters
    const MAX_ROLL: f32 = 0.5; // About 30 degrees
    const ROLL_SPEED: f32 = 3.0;
    const RETURN_SPEED: f32 = 2.0;

    // Get the transform
    if let Ok(mut transform) = query.single_mut() {
        // Store initial rotation if we haven't yet
        if anim_state.initial_rotation.is_none() {
            anim_state.initial_rotation = Some(transform.rotation);
        }

        // Determine target roll based on input
        if keyboard.pressed(KeyCode::ArrowLeft) {
            anim_state.target_roll = -MAX_ROLL;
        } else if keyboard.pressed(KeyCode::ArrowRight) {
            anim_state.target_roll = MAX_ROLL;
        } else {
            anim_state.target_roll = 0.0;
        }

        // Smoothly interpolate current roll to target
        let delta = time.elapsed().as_secs_f32();
        let speed = if anim_state.target_roll == 0.0 { RETURN_SPEED } else { ROLL_SPEED };
        anim_state.current_roll = lerp(
            anim_state.current_roll,
            anim_state.target_roll,
            delta * speed
        );

        // Apply roll rotation while preserving initial rotation
        if let Some(initial_rot) = anim_state.initial_rotation {
            transform.rotation = initial_rot * Quat::from_rotation_z(anim_state.current_roll);
        }
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t.clamp(0.0, 1.0)
}