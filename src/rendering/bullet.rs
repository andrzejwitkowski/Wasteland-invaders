use bevy::prelude::*;

pub struct BulletPlugin;

#[derive(Component)]
pub struct Bullet {
    speed: f32,
}

impl Plugin for BulletPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (move_bullets, cleanup_bullets));
    }
}

fn move_bullets(mut bullets: Query<(&Bullet, &mut Transform)>, time: Res<Time>) {
    for (bullet, mut transform) in bullets.iter_mut() {
        // Move bullet forward (negative Z in our coordinate system)
        transform.translation.z -= bullet.speed * time.delta().as_secs_f32();
    }
}

fn cleanup_bullets(mut commands: Commands, bullets: Query<(Entity, &Transform), With<Bullet>>) {
    // Remove bullets that have gone too far
    for (entity, transform) in bullets.iter() {
        if transform.translation.z < -100.0 {
            commands.entity(entity).despawn();
        }
    }
}

// Function to spawn a bullet
pub fn spawn_bullet(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, position: Vec3) {
    commands.spawn((
        Bullet { speed: 50.0 }, // Bullet speed
        Mesh3d(meshes.add(Sphere::new(0.25))), // Bullet mesh (sphere)
        MeshMaterial3d(materials.add(Color::srgb(1.0, 1.0, 0.0))), // Yellow color using sRGB values
        Transform::from_translation(position),
    ));
}