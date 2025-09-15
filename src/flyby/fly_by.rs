// src/flyby/fly_by.rs
// Updated implementation for flyby with terrain-based positioning, debug gizmo path, and scene integration.
// Use this to replace your entire file.

use bevy::prelude::*;
use bevy::gizmos::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::heightmapgenerator::height_map_renderer::{HeightmapTerrain, HeightmapRenderConfig};
use crate::heightmapgenerator::height_map_generator::{HeightmapNoise, HeightmapConfig};

// ===== EVENTS =====
#[derive(Event)]
pub struct MoveCameraToOverview;

#[derive(Event)]
pub struct RestoreCameraPosition;

// ===== RESOURCES =====
#[derive(Resource)]
pub struct OriginalCameraTransform {
    pub transform: Transform,
}

#[derive(Resource)]
pub struct FlybyState {
    pub is_flying: bool,
    pub start_time: f32,
    pub duration: f32,
    pub show_debug_path: bool,
    pub camera_height: f32,
    pub camera_distance_behind: f32,
    pub look_ahead_distance: f32,
}

impl Default for FlybyState {
    fn default() -> Self {
        Self {
            is_flying: false,
            start_time: 0.0,
            duration: 10.0,
            show_debug_path: true,
            camera_height: 150.0,
            camera_distance_behind: 100.0, // How far behind the river point
            look_ahead_distance: 150.0,    // How far ahead to look
        }
    }
}

// ===== PLUGIN =====
pub struct FlyByPlugin;

impl Plugin for FlyByPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MoveCameraToOverview>()
            .add_event::<RestoreCameraPosition>()
            .init_resource::<FlybyState>()
            .add_systems(Update, (
                flyby_ui_system,
                camera_event_handler_system,
                debug_path_system,
            ));
    }
}

// ===== UI SYSTEM =====
fn flyby_ui_system(
    mut contexts: EguiContexts,
    mut move_events: EventWriter<MoveCameraToOverview>,
    mut restore_events: EventWriter<RestoreCameraPosition>,
    terrain_query: Query<Entity, With<HeightmapTerrain>>,
    original_camera_resource: Option<Res<OriginalCameraTransform>>,
    mut flyby_state: ResMut<FlybyState>,
) {
    egui::Window::new("🎬 River Raid Camera")
        .default_width(320.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Camera Control");
            
            let has_terrain = !terrain_query.is_empty();
            let has_saved_position = original_camera_resource.is_some();
            
            ui.separator();
            
            ui.add_enabled_ui(has_terrain, |ui| {
                if ui.button("🚁 River Raid View").clicked() {
                    move_events.send(MoveCameraToOverview);
                }
            });
            
            if !has_terrain {
                ui.colored_label(egui::Color32::RED, "⚠️ Generate terrain first!");
            }
            
            ui.add_enabled_ui(has_saved_position, |ui| {
                if ui.button("🔙 Restore Camera").clicked() {
                    restore_events.send(RestoreCameraPosition);
                }
            });

            ui.separator();
            ui.heading("River Raid Settings");
            
            ui.add(egui::Slider::new(&mut flyby_state.camera_height, 80.0..=300.0)
                .text("Camera Height"));
            
            ui.add(egui::Slider::new(&mut flyby_state.camera_distance_behind, 50.0..=200.0)
                .text("Distance Behind"));
            
            ui.add(egui::Slider::new(&mut flyby_state.look_ahead_distance, 100.0..=300.0)
                .text("Look Ahead Distance"));
            
            ui.separator();
            ui.checkbox(&mut flyby_state.show_debug_path, "🐛 Show Flight Path");
            
            if has_saved_position {
                ui.colored_label(egui::Color32::GREEN, "💾 Camera position saved");
            }
        });
}

// ===== EVENT HANDLER =====
fn camera_event_handler_system(
    mut commands: Commands,
    mut move_events: EventReader<MoveCameraToOverview>,
    mut restore_events: EventReader<RestoreCameraPosition>,
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<HeightmapTerrain>)>,
    render_config: Res<HeightmapRenderConfig>,
    heightmap_config: Res<HeightmapConfig>,
    heightmap_noise: Res<HeightmapNoise>,
    original_camera_resource: Option<Res<OriginalCameraTransform>>,
    flyby_state: Res<FlybyState>,
) {
    // Handle move to River Raid view
    for _ in move_events.read() {
        info!("📥 Setting up River Raid camera view");
        
        if let Ok(mut camera_transform) = camera_query.get_single_mut() {
            // Save current position
            if original_camera_resource.is_none() {
                commands.insert_resource(OriginalCameraTransform {
                    transform: *camera_transform,
                });
                info!("💾 Saved original camera position");
            }
            
            // Get river start position
            let river_start = find_river_start(&heightmap_config, &heightmap_noise, &render_config);
            
            // Calculate river direction (normalized)
            let river_direction = heightmap_config.river_direction.normalize();
            let river_direction_3d = Vec3::new(river_direction.x, 0.0, river_direction.y);
            
            // Position camera BEHIND the river start, elevated
            let camera_position = Vec3::new(
                river_start.x - river_direction_3d.x * flyby_state.camera_distance_behind,
                river_start.y + flyby_state.camera_height,
                river_start.z - river_direction_3d.z * flyby_state.camera_distance_behind,
            );
            
            // Look ahead along the river path (River Raid style)
            let look_target = Vec3::new(
                river_start.x + river_direction_3d.x * flyby_state.look_ahead_distance,
                river_start.y,
                river_start.z + river_direction_3d.z * flyby_state.look_ahead_distance,
            );
            
            // Set camera transform (River Raid angle)
            *camera_transform = Transform::from_translation(camera_position)
                .looking_at(look_target, Vec3::Y);
            
            info!("🎮 River Raid camera positioned at: {:.2?}", camera_position);
            info!("👀 Looking at: {:.2?}", look_target);
            info!("🌊 River direction: {:.2?}", river_direction);
        }
    }
    
    // Handle restore
    for _ in restore_events.read() {
        if let Some(original) = &original_camera_resource {
            if let Ok(mut camera_transform) = camera_query.get_single_mut() {
                *camera_transform = original.transform;
                commands.remove_resource::<OriginalCameraTransform>();
                info!("🔙 Camera restored!");
            }
        }
    }
}

// ===== DEBUG PATH SYSTEM =====
fn debug_path_system(
    flyby_state: Res<FlybyState>,
    mut gizmos: Gizmos,
    render_config: Res<HeightmapRenderConfig>,
    heightmap_config: Res<HeightmapConfig>,
    heightmap_noise: Res<HeightmapNoise>,
) {
    if !flyby_state.show_debug_path {
        return;
    }
    
    // Generate River Raid style camera path
    let mut camera_path_points = Vec::new();
    let mut look_target_points = Vec::new();
    let num_points = 15;
    
    for i in 0..=num_points {
        let progress = i as f32 / num_points as f32;
        let river_pos = interpolate_river_path(&heightmap_config, &heightmap_noise, &render_config, progress);
        
        // Calculate river direction at this point
        let river_direction = heightmap_config.river_direction.normalize();
        let river_direction_3d = Vec3::new(river_direction.x, 0.0, river_direction.y);
        
        // Camera position (behind and above the river point)
        let camera_pos = Vec3::new(
            river_pos.x - river_direction_3d.x * flyby_state.camera_distance_behind,
            river_pos.y + flyby_state.camera_height,
            river_pos.z - river_direction_3d.z * flyby_state.camera_distance_behind,
        );
        
        // Look target (ahead of the river point)
        let look_target = Vec3::new(
            river_pos.x + river_direction_3d.x * flyby_state.look_ahead_distance,
            river_pos.y,
            river_pos.z + river_direction_3d.z * flyby_state.look_ahead_distance,
        );
        
        camera_path_points.push(camera_pos);
        look_target_points.push(look_target);
    }
    
    // Draw camera flight path (red line)
    for i in 0..camera_path_points.len() - 1 {
        gizmos.line(
            camera_path_points[i],
            camera_path_points[i + 1],
            Color::srgb(1.0, 0.0, 0.0) // Red camera path
        );
    }
    
    // Draw look direction lines (yellow lines)
    for i in (0..camera_path_points.len()).step_by(3) {
        gizmos.line(
            camera_path_points[i],
            look_target_points[i],
            Color::srgb(1.0, 1.0, 0.0) // Yellow look direction
        );
    }
    
    // Draw start marker (green sphere)
    if let Some(start) = camera_path_points.first() {
        gizmos.sphere(*start, 8.0, Color::srgb(0.0, 1.0, 0.0));
    }
    
    // Draw end marker (magenta sphere)
    if let Some(end) = camera_path_points.last() {
        gizmos.sphere(*end, 8.0, Color::srgb(1.0, 0.0, 1.0));
    }
}

// ===== HELPER FUNCTIONS =====
fn find_river_start(
    heightmap_config: &HeightmapConfig,
    heightmap_noise: &HeightmapNoise,
    render_config: &HeightmapRenderConfig,
) -> Vec3 {
    let river_start_2d = heightmap_config.river_start;
    let height = heightmap_noise.sample_height_with_river(river_start_2d.x, river_start_2d.y, heightmap_config);
    Vec3::new(river_start_2d.x, height, river_start_2d.y)
}

fn interpolate_river_path(
    heightmap_config: &HeightmapConfig,
    heightmap_noise: &HeightmapNoise,
    render_config: &HeightmapRenderConfig,
    progress: f32,
) -> Vec3 {
    let river_length = render_config.chunk_size * 0.8;
    let distance_along_river = progress * river_length;
    
    let river_direction = heightmap_config.river_direction.normalize();
    let base_position = heightmap_config.river_start + river_direction * distance_along_river;
    
    // Add meandering
    let meander_offset = if heightmap_config.meander_frequency > 0.0 {
        (distance_along_river * heightmap_config.meander_frequency * std::f32::consts::TAU).sin() * heightmap_config.meander_amplitude
    } else {
        0.0
    };
    
    let perpendicular = Vec2::new(-river_direction.y, river_direction.x);
    let river_center = base_position + perpendicular * meander_offset;
    
    let height = heightmap_noise.sample_height_with_river(river_center.x, river_center.y, heightmap_config);
    Vec3::new(river_center.x, height, river_center.y)
}
