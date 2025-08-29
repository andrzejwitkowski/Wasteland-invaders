use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::heightmapgenerator::height_map_renderer::{HeightmapTerrain, HeightmapRenderConfig};
use crate::heightmapgenerator::height_map_generator::{HeightmapConfig, HeightmapNoise};

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

// ===== PLUGIN =====
pub struct FlyByPlugin;

impl Plugin for FlyByPlugin {
    fn build(&self, app: &mut App) {
        app
            // Register events
            .add_event::<MoveCameraToOverview>()
            .add_event::<RestoreCameraPosition>()
            // Add systems
            .add_systems(Update, (
                flyby_ui_system,
                camera_event_handler_system,
            ));
    }
}

// ===== UI SYSTEM (Event Publisher) =====
fn flyby_ui_system(
    mut contexts: EguiContexts,
    mut move_events: EventWriter<MoveCameraToOverview>,
    mut restore_events: EventWriter<RestoreCameraPosition>,
    terrain_query: Query<&Transform, With<HeightmapTerrain>>,
    original_camera_resource: Option<Res<OriginalCameraTransform>>,
) {
    egui::Window::new("🎬 Fly By Camera")
        .default_width(300.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Camera Control");
            
            let has_terrain = !terrain_query.is_empty();
            let has_saved_position = original_camera_resource.is_some();
            
            ui.separator();
            
            // Move to overview button
            ui.add_enabled_ui(has_terrain, |ui| {
                if ui.button("📍 Move to River End Overview").clicked() {
                    move_events.send(MoveCameraToOverview);
                    info!("📤 Sent MoveCameraToOverview event");
                }
            });
            
            if !has_terrain {
                ui.colored_label(egui::Color32::RED, "⚠️ Generate terrain first!");
            }
            
            ui.separator();
            
            // Restore position button
            ui.add_enabled_ui(has_saved_position, |ui| {
                if ui.button("🔙 Restore Original Position").clicked() {
                    restore_events.send(RestoreCameraPosition);
                    info!("📤 Sent RestoreCameraPosition event");
                }
            });
            
            if !has_saved_position {
                ui.colored_label(egui::Color32::GRAY, "💾 No saved position");
            }
            
            ui.separator();
            ui.label("💡 Click overview to see the whole terrain from above the river end!");
        });
}

// ===== CAMERA EVENT HANDLER SYSTEM (Event Subscriber) =====
fn camera_event_handler_system(
    mut commands: Commands,
    mut move_events: EventReader<MoveCameraToOverview>,
    mut restore_events: EventReader<RestoreCameraPosition>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    heightmap_config: Res<HeightmapConfig>,
    heightmap_noise: Res<HeightmapNoise>,
    render_config: Res<HeightmapRenderConfig>,
    original_camera_resource: Option<Res<OriginalCameraTransform>>,
) {
    // Handle move camera events
    for _ in move_events.read() {
        info!("📥 Processing MoveCameraToOverview event");
        handle_move_to_overview(
            &mut commands,
            &heightmap_config,
            &heightmap_noise,
            &render_config,
            &mut camera_query,
        );
    }
    
    // Handle restore camera events
    for _ in restore_events.read() {
        info!("📥 Processing RestoreCameraPosition event");
        handle_restore_position(
            &mut commands,
            &mut camera_query,
            &original_camera_resource,
        );
    }
}

// ===== EVENT HANDLERS =====

fn handle_move_to_overview(
    commands: &mut Commands,
    heightmap_config: &HeightmapConfig,
    heightmap_noise: &HeightmapNoise,
    render_config: &HeightmapRenderConfig,
    camera_query: &mut Query<&mut Transform, With<Camera3d>>,
) {
    if let Ok(mut camera_transform) = camera_query.get_single_mut() {
        // Save current camera position
        commands.insert_resource(OriginalCameraTransform {
            transform: *camera_transform,
        });
        info!("💾 Saved original camera position");
        
        // Find the river end point (rightmost river point)
        let world_size = render_config.chunk_size;
        let step_size = 15.0;
        let num_samples = (world_size / step_size) as i32;
        
        let mut rightmost_river_point: Option<Vec3> = None;
        let mut max_x = f32::NEG_INFINITY;
        
        info!("🔍 Scanning for river end point...");
        
        // Scan the world for river points
        for z in -(num_samples/2)..=(num_samples/2) {
            for x in -(num_samples/2)..=(num_samples/2) {
                let world_x = x as f32 * step_size;
                let world_z = z as f32 * step_size;
                
                // Check if this point is part of a river
                let (river_mod, _) = heightmap_noise.calculate_river_effects(
                    Vec2::new(world_x, world_z), 
                    heightmap_config
                );
                
                // If it's a river point and further right than current max
                if river_mod < -0.7 && world_x > max_x {
                    max_x = world_x;
                    let terrain_height = heightmap_noise.sample_height_with_river(
                        world_x, world_z, heightmap_config
                    );
                    
                    rightmost_river_point = Some(Vec3::new(
                        world_x,
                        terrain_height,
                        world_z
                    ));
                }
            }
        }
        
        if let Some(river_end) = rightmost_river_point {
            // Calculate camera position high above the river end
            let camera_height = world_size * 0.8; // High enough to see everything
            let camera_position = Vec3::new(
                river_end.x,
                camera_height,
                river_end.z + world_size * 0.3, // Pull back for better viewing angle
            );
            
            // Look at the center of the terrain for best overview
            let look_target = Vec3::new(0.0, 0.0, 0.0);
            
            // Move the camera
            *camera_transform = Transform::from_translation(camera_position)
                .looking_at(look_target, Vec3::Y);
            
            info!("📹 Camera moved to overview position: {:.2?}", camera_position);
            info!("👀 Looking at terrain center: {:.2?}", look_target);
            info!("✅ Overview camera active!");
        } else {
            warn!("❌ No river end point found! Make sure you have generated terrain with rivers.");
        }
    } else {
        error!("❌ No camera found in the scene!");
    }
}

fn handle_restore_position(
    commands: &mut Commands,
    camera_query: &mut Query<&mut Transform, With<Camera3d>>,
    original_camera_resource: &Option<Res<OriginalCameraTransform>>,
) {
    if let Some(original) = original_camera_resource {
        if let Ok(mut camera_transform) = camera_query.get_single_mut() {
            // Restore the saved camera position
            *camera_transform = original.transform;
            
            // Clean up the saved position resource
            commands.remove_resource::<OriginalCameraTransform>();
            
            info!("🔙 Camera restored to original position!");
            info!("🗑️ Cleared saved camera position");
        } else {
            error!("❌ No camera found to restore!");
        }
    } else {
        warn!("❌ No original camera position saved! Move to overview first.");
    }
}