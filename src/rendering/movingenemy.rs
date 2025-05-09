use bevy::prelude::*;
use rand::{Rng, thread_rng};
use bevy::math::primitives::Cuboid;

// --- Combined Plugin Definition ---
pub struct MovingEnemyPlugin; // Renamed from GameplayPlugin

impl Plugin for MovingEnemyPlugin { // Renamed
    fn build(&self, app: &mut App) {
        app
            // --- Startup Systems ---
            .add_systems(Startup, spawn_spline_and_enemy)
            .add_systems(Startup, draw_spline_visuals.after(spawn_spline_and_enemy))
            
            // --- Update Systems ---
            .add_systems(Update, enemy_follow_spline_path)
            .add_systems(Update, cleanup_finished_enemies.after(enemy_follow_spline_path));
    }
}

// --- Spline Related Components and Logic ---
#[derive(Component)]
pub struct Spline {
    pub control_points: Vec<Vec3>,
}

pub fn bezier_point(control_points: &[Vec3], t: f32) -> Vec3 {
    let n = control_points.len() - 1;
    if control_points.is_empty() { return Vec3::ZERO; }
    if n == 0 { return control_points[0]; } 
    
    let mut point = Vec3::ZERO;
    for i in 0..=n {
        let binomial = binomial_coefficient(n, i);
        let t_clamped = t.clamp(0.0, 1.0);
        let t_complement = 1.0 - t_clamped;

        if i < control_points.len() {
            let factor = binomial as f32 * t_complement.powi((n - i) as i32) * t_clamped.powi(i as i32);
            point += control_points[i] * factor;
        }
    }
    point
}

fn binomial_coefficient(n: usize, k: usize) -> usize {
    if k > n { return 0; }
    if k == 0 || k == n { return 1; }
    if k > n / 2 { return binomial_coefficient(n, n - k); }
    
    let mut result = 1;
    for i in 0..k {
        if i + 1 == 0 { return 0; } 
        result = result * (n - i) / (i + 1);
    }
    result
}

// --- Enemy Related Components and Logic ---
#[derive(Component)]
pub struct Enemy {
    pub speed: f32,
    pub spline_progress: f32,
    pub spline_entity: Entity,
}

#[derive(Component)]
struct Cleanup; // Marker for cleanup

// --- Systems ---

fn spawn_spline_and_enemy(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut rng = thread_rng(); 

    let z_top_screen = -5.0; 
    let z_bottom_screen = -45.0; 

    const VISIBLE_SCREEN_X_MIN: f32 = -15.0;
    const VISIBLE_SCREEN_X_MAX: f32 = 15.0;
    const SPAWN_AREA_X_MIN: f32 = -20.0; 
    const SPAWN_AREA_X_MAX: f32 = 20.0;

    let start_x = rng.gen_range(SPAWN_AREA_X_MIN..SPAWN_AREA_X_MAX); 
    let end_x = rng.gen_range(SPAWN_AREA_X_MIN..SPAWN_AREA_X_MAX);

    let mut points = vec![
        Vec3::new(start_x, 0.1, z_top_screen) // P0
    ];

    let pull_strength1 = rng.gen_range(20.0..30.0); 
    let pull_strength2 = rng.gen_range(20.0..30.0); 

    let raw_control1_x: f32;
    let raw_control2_x: f32;

    if rng.gen_bool(0.5) { 
        raw_control1_x = start_x - pull_strength1;
        raw_control2_x = end_x + pull_strength2;
    } else { 
        raw_control1_x = start_x + pull_strength1;
        raw_control2_x = end_x - pull_strength2;
    }
    
    let control1_x = raw_control1_x.clamp(VISIBLE_SCREEN_X_MIN, VISIBLE_SCREEN_X_MAX);
    let control2_x = raw_control2_x.clamp(VISIBLE_SCREEN_X_MIN, VISIBLE_SCREEN_X_MAX);
    
    let p1_z = z_top_screen + 0.33 * (z_bottom_screen - z_top_screen);
    points.push(Vec3::new(control1_x, 0.1, p1_z)); // P1

    let p2_z = z_top_screen + 0.66 * (z_bottom_screen - z_top_screen);
    points.push(Vec3::new(control2_x, 0.1, p2_z)); // P2

    points.push(Vec3::new(end_x, 0.1, z_bottom_screen)); // P3

    let spline_entity = commands.spawn(
        Spline { control_points: points }
    ).id();
    
    spawn_enemy_on_spline(&mut commands, &asset_server, spline_entity);
}

fn draw_spline_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    splines: Query<&Spline>, 
) {
    for spline in splines.iter() {
        if spline.control_points.len() < 4 { continue; } 

        for (idx, point_translation) in spline.control_points.iter().enumerate() {
            let cube_size = 2.0; 
            let cube_mesh_handle = meshes.add(Cuboid::new(cube_size, cube_size, cube_size));
            
            let point_color_enum = match idx {
                0 => Color::srgb(0.0, 1.0, 0.0), 
                1 => Color::srgb(1.0, 0.5, 0.0), 
                2 => Color::srgb(1.0, 0.0, 1.0), 
                3 => Color::srgb(0.0, 1.0, 1.0), 
                _ => Color::srgb(0.7, 0.7, 0.7),
            };

            let point_srgba = point_color_enum.to_srgba(); 
            let emissive_srgb = Color::srgb( 
                point_srgba.red * 0.5,
                point_srgba.green * 0.5,
                point_srgba.blue * 0.5
            );
            let emissive_linear_rgba = emissive_srgb.to_linear(); 

            let material_handle = materials.add(StandardMaterial {
                base_color: point_color_enum, 
                emissive: emissive_linear_rgba, 
                ..default()
            });

            // Reverted to using Mesh3d and MeshMaterial3d components as per user's setup
            commands.spawn((
                Mesh3d(cube_mesh_handle), 
                MeshMaterial3d(material_handle), 
                Transform::from_translation(*point_translation),
            ));
        }

        const SEGMENTS: usize = 50;
        let mut curve_points = Vec::with_capacity(SEGMENTS + 1);
        for i in 0..=SEGMENTS {
            let t = i as f32 / SEGMENTS as f32;
            curve_points.push(bezier_point(&spline.control_points, t));
        }
        
        for window in curve_points.windows(2) {
            let start_point = window[0];
            let end_point = window[1];
            
            let length = (end_point - start_point).length();
            if length < 0.001 { continue; } 

            let direction = (end_point - start_point).normalize_or_zero();
            
            let line_thickness = 0.3; 
            let line_mesh_handle = meshes.add(Cuboid::new(line_thickness, line_thickness, length));
            let rotation = Quat::from_rotation_arc(Vec3::Z, direction); 
            let midpoint = start_point + direction * length * 0.5;
            
            let near_control_point_threshold: f32 = 10.0; 
            let is_near_control_point = spline.control_points.iter().any(|cp| {
                (midpoint - *cp).length_squared() < near_control_point_threshold.powi(2)
            });
            
            let segment_color_enum = if is_near_control_point { 
                Color::srgb(1.0, 1.0, 0.0) 
            } else {
                Color::srgb(0.1, 0.1, 0.1) 
            };
            
            let segment_material_handle = materials.add(StandardMaterial {
                base_color: segment_color_enum, 
                ..default()
            });

            // Reverted to using Mesh3d and MeshMaterial3d components
            commands.spawn((
                Mesh3d(line_mesh_handle), 
                MeshMaterial3d(segment_material_handle), 
                Transform::from_translation(midpoint).with_rotation(rotation),
            ));
        }
    }
}

fn enemy_follow_spline_path(
    mut commands: Commands,
    mut enemies: Query<(Entity, &mut Enemy, &mut Transform)>,
    splines: Query<&Spline>, 
    time: Res<Time>,
) {
    for (enemy_entity, mut enemy, mut transform) in enemies.iter_mut() {
        if let Ok(spline) = splines.get(enemy.spline_entity) {
            if spline.control_points.len() < 2 { 
                continue;
            }
            let progress_delta = enemy.speed * time.delta_secs() * 0.01; 
            enemy.spline_progress += progress_delta;
            
            if enemy.spline_progress >= 1.0 {
                if let Some(mut entity_commands) = commands.get_entity(enemy_entity) {
                    entity_commands.insert(Cleanup);
                }
            } else {
                let new_pos = bezier_point(&spline.control_points, enemy.spline_progress);
                let look_ahead_progress = (enemy.spline_progress + 0.01).min(1.0);
                let next_pos = bezier_point(&spline.control_points, look_ahead_progress);
                
                transform.translation = new_pos;
                let direction = next_pos - new_pos;
                if direction.length_squared() > 0.0001 { 
                    transform.look_at(next_pos, Vec3::Y);
                }
            }
        } else {
            if let Some(mut entity_commands) = commands.get_entity(enemy_entity) {
                commands.entity(enemy_entity).insert(Cleanup);
            }
        }
    }
}

fn cleanup_finished_enemies(
    mut commands: Commands,
    enemies_to_cleanup: Query<Entity, With<Cleanup>>, 
) {
    for enemy_entity in enemies_to_cleanup.iter() {
        commands.entity(enemy_entity).despawn_recursive(); 
    }
}

fn spawn_enemy_on_spline(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>, 
    spline_entity: Entity,     
) -> Entity {
    let model_scene_handle: Handle<Scene> = asset_server.load("models/enemy/enemy_plane.gltf#Scene0");
    
    commands.spawn((
        SceneRoot::from(model_scene_handle),
        Transform::from_xyz(0.0, 2.0, -20.0) 
            .with_scale(Vec3::new(3.3, 3.3, 3.3))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            Enemy {
                speed: 15.0, 
                spline_progress: 0.0,
                spline_entity,
            },
    )).id()
}
