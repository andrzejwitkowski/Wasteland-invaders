use bevy::prelude::*;
// Assuming Spline and bezier_point are correctly imported from your spline module
// e.g., use super::spline_module::{Spline, bezier_point};
// For this example, I'll use a path relative to how it might be structured.
// Please adjust the path to `Spline` and `bezier_point` if it's different in your project.
use crate::rendering::spline::{Spline, bezier_point}; // Assuming spline module is at crate::spline

// If SceneRoot is not in prelude, you might need to import it, e.g.:
// use bevy::scene::SceneRoot;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, follow_spline_path)
           .add_systems(Update, cleanup_enemies);
    }
}

#[derive(Component)]
pub struct Enemy {
    pub speed: f32,
    pub spline_progress: f32, // 0.0 to 1.0 progress along spline
    pub spline_entity: Entity, // The Entity ID of the Spline component this enemy follows
}

fn follow_spline_path(
    mut commands: Commands,
    mut enemies: Query<(Entity, &mut Enemy, &mut Transform)>,
    splines: Query<&Spline>, // Query for the Spline component data
    time: Res<Time>,
) {
    for (enemy_entity, mut enemy, mut transform) in enemies.iter_mut() {
        // Try to get the Spline component data using the Entity ID stored in the Enemy
        if let Ok(spline) = splines.get(enemy.spline_entity) {
            // Move along spline using delta_seconds
            let progress_delta = enemy.speed * time.delta_secs() * 0.01; 
            enemy.spline_progress += progress_delta;
            
            if enemy.spline_progress >= 1.0 {
                // Mark enemy for cleanup when it reaches the end by inserting the Cleanup component.
                commands.entity(enemy_entity).insert(Cleanup);
            } else {
                // Calculate new position along spline
                let new_pos = bezier_point(&spline.control_points, enemy.spline_progress);
                
                // Calculate a point slightly ahead for look_at direction
                let look_ahead_progress = (enemy.spline_progress + 0.01).min(1.0);
                let next_pos = bezier_point(&spline.control_points, look_ahead_progress);
                
                // Update transform
                transform.translation = new_pos;

                let direction = next_pos - new_pos;
                if direction.length_squared() > 0.0001 { 
                    transform.look_at(next_pos, Vec3::Y);
                }
            }
        } else {
            // Optional: Handle cases where the spline_entity is invalid.
            // commands.entity(enemy_entity).despawn(); 
        }
    }
}

#[derive(Component)]
struct Cleanup; // Marker component for enemies to be cleaned up

fn cleanup_enemies(
    mut commands: Commands,
    enemies_to_cleanup: Query<Entity, With<Cleanup>>, 
) {
    for enemy_entity in enemies_to_cleanup.iter() {
        commands.entity(enemy_entity).despawn_recursive(); 
    }
}

// Helper function to spawn an enemy.
pub fn spawn_enemy_with_spline(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>, 
    spline_entity: Entity,     
) -> Entity {
    // Load the enemy GLTF model
    let model_scene_handle: Handle<Scene> = asset_server.load("models/enemy/enemy_plane.gltf#Scene0");
    
    // Spawn the enemy using the (SceneRoot, Transform, Enemy) tuple pattern

    commands.spawn((
        SceneRoot::from(model_scene_handle),
        Transform::from_xyz(0.0, 2.0, -20.0) // Position in the middle of the scene
            .with_scale(Vec3::new(3.3, 3.3, 3.3))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            Enemy {
                speed: 15.0, // Adjust speed as needed
                spline_progress: 0.0,
                spline_entity,
            },
    )).id()
}
