use bevy::prelude::*;
use crate::rendering::input::ControllablePlane;

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
}

fn plane_swing_animation(
    mut query: Query<&mut Transform, With<ControllablePlane>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut anim_state: ResMut<PlaneAnimationState>,
) {
    // Define animation parameters
    const MAX_ROLL: f32 = 0.5; // About 30 degrees
    const ROLL_SPEED: f32 = 3.0;
    const RETURN_SPEED: f32 = 2.0;

    // Determine target roll based on input
    if keyboard.pressed(KeyCode::ArrowLeft) {
        anim_state.target_roll = MAX_ROLL;
    } else if keyboard.pressed(KeyCode::ArrowRight) {
        anim_state.target_roll = -MAX_ROLL;
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

    // Apply rotation to plane - now we set the absolute rotation instead of multiplying
    if let Ok(mut transform) = query.single_mut() {
        transform.rotation = Quat::from_rotation_z(anim_state.current_roll);
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t.clamp(0.0, 1.0)
}