use bevy::prelude::*;

use crate::rendering::spline::{bezier_point, Spline};
use crate::rendering::enemy::Enemy;
use crate::rendering::spline::spawn_spline;
use crate::rendering::enemy::spawn_enemy;

pub struct EnemySplineFollowerPlugin;

impl Plugin for EnemySplineFollowerPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Startup, spawn_enemy_with_spline)
        .add_systems(Update, (
            follow_spline_path,
            cleanup_enemies.after(follow_spline_path)
        ).chain());
    }
}

#[derive(Component)]
pub struct EnemySplineFollower {
    pub spline_entity: Entity,
    pub enemy_entity: Entity,
    pub spline_progress: f32
}

#[derive(Component)]
struct Cleanup; // Marker component for enemies to be cleaned up

fn spawn_enemy_with_spline(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>
) {

    let spline_entity = spawn_spline(&mut commands,meshes, materials);
    let enemy_entity = spawn_enemy(&mut commands, &asset_server);

    commands.spawn(
        EnemySplineFollower {
            spline_entity,
            enemy_entity,
            spline_progress: 0.0,
        }
    );
}

fn follow_spline_path(
    mut commands: Commands,
    splines: Query<&Spline>,
    enemies: Query<&Enemy>,
    mut followers: Query<(Entity, &mut EnemySplineFollower)>,
    mut enemy_transforms: Query<&mut Transform>,
    time: Res<Time>,
) {
    for (follower_entity, mut follower) in followers.iter_mut() {
        // Try to get the Spline component data using the Entity ID stored in the follower
        if let Ok(spline) = splines.get(follower.spline_entity) {
            // Get enemy transform
            if let Ok(mut transform) = enemy_transforms.get_mut(follower.enemy_entity) {

                // Get enemy speed
                let speed = enemies.get(follower.enemy_entity).unwrap().speed;

                // Move along spline using delta_seconds
                let progress_delta = speed * time.delta_secs() * 0.01; 
                follower.spline_progress += progress_delta;

                println!("Follower progress: {}", follower.spline_progress);
                
                if follower.spline_progress >= 1.0 {
                    // Mark enemy for cleanup when it reaches the end by inserting the Cleanup component.
                    commands.entity(follower_entity).insert(Cleanup);
                } else {
                    // Calculate new position along spline
                    let new_pos = bezier_point(&spline.control_points, follower.spline_progress);
                    
                    // Calculate a point slightly ahead for look_at direction
                    let look_ahead_progress = (follower.spline_progress + 0.01).min(1.0);
                    let next_pos = bezier_point(&spline.control_points, look_ahead_progress);
                    
                    // Update transform
                    transform.translation = new_pos;

                    let direction = next_pos - new_pos;
                    if direction.length_squared() > 0.0001 { 
                        transform.look_at(next_pos, Vec3::Y);
                    }
                }
            }
        } else {
            // Optional: Handle cases where the spline_entity is invalid
            commands.entity(follower.enemy_entity).despawn(); 
        }
    }
}

fn cleanup_enemies(
    mut commands: Commands,
    enemies_to_cleanup: Query<Entity, With<Cleanup>>,
    followers: Query<(Entity, &mut EnemySplineFollower)>,
) {
    for enemy_entity in enemies_to_cleanup.iter() {
        if let Ok(follower) = followers.get(enemy_entity) {
            println!("Found follower for enemy entity: {:?}", enemy_entity);
            // Despawn the enemy entity
            commands.entity(follower.1.enemy_entity).despawn();
            // Despawn the spline entity
            commands.entity(follower.1.spline_entity).despawn();
            // Despawn the follower entity
            commands.entity(follower.0).despawn();
        } else {
            println!("Failed to find follower for enemy entity: {:?}", enemy_entity);
        }
    }
}
