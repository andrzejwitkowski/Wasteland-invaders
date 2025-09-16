// src/flyby/fly_by.rs
// Updated implementation for flyby with terrain-based positioning, debug gizmo path, and scene integration.
// Use this to replace your entire file.

use bevy::prelude::*;
use bevy_blendy_cameras::{FlyCameraController, OrbitCameraController};
use bevy_egui::{egui, EguiContexts};
use crate::heightmapgenerator::height_map_renderer::{HeightmapTerrain, HeightmapRenderConfig};
use crate::heightmapgenerator::height_map_generator::{HeightmapNoise, HeightmapConfig};

// ===== TURBULENCE TRAIT =====
pub trait TurbulenceEffect {
    fn apply_turbulence(&self, position: Vec3, time: f32, intensity: f32) -> Vec3;
}

// ===== TURBULENCE IMPLEMENTATIONS =====
#[derive(Default)]
pub struct AtmosphericTurbulence;

impl TurbulenceEffect for AtmosphericTurbulence {
    fn apply_turbulence(&self, position: Vec3, time: f32, intensity: f32) -> Vec3 {
        let freq = 0.5;
        let x_offset = (time * freq + position.x * 0.01).sin() * intensity;
        let y_offset = (time * freq * 1.3 + position.y * 0.008).cos() * intensity * 0.5;
        let z_offset = (time * freq * 0.7 + position.z * 0.012).sin() * intensity;
        
        position + Vec3::new(x_offset, y_offset, z_offset)
    }
}

#[derive(Default)]
pub struct WindGustTurbulence;

impl TurbulenceEffect for WindGustTurbulence {
    fn apply_turbulence(&self, position: Vec3, time: f32, intensity: f32) -> Vec3 {
        // Stronger, less frequent gusts
        let gust_freq = 0.2;
        let micro_freq = 3.0;
        
        let gust_strength = (time * gust_freq).sin().abs(); // Varies from 0 to 1
        let micro_turbulence = (time * micro_freq + position.x * 0.05).sin() * 0.3;
        
        let total_intensity = intensity * (gust_strength + micro_turbulence);
        
        let x_offset = (time * 0.8 + position.length() * 0.01).cos() * total_intensity;
        let y_offset = (time * 1.2 + position.x * 0.01).sin() * total_intensity * 0.6;
        let z_offset = (time * 0.6 + position.z * 0.015).cos() * total_intensity;
        
        position + Vec3::new(x_offset, y_offset, z_offset)
    }
}

#[derive(Default)]
pub struct ThermalTurbulence;

impl TurbulenceEffect for ThermalTurbulence {
    fn apply_turbulence(&self, position: Vec3, time: f32, intensity: f32) -> Vec3 {
        // Rising thermal currents with updrafts
        let thermal_freq = 0.1;
        let updraft_strength = (position.x * 0.02 + position.z * 0.03 + time * thermal_freq).sin();
        
        // Stronger vertical movement, gentle horizontal drift
        let x_offset = (time * 0.3 + position.z * 0.01).sin() * intensity * 0.4;
        let y_offset = updraft_strength * intensity * 1.5; // Stronger vertical movement
        let z_offset = (time * 0.4 + position.x * 0.01).cos() * intensity * 0.4;
        
        position + Vec3::new(x_offset, y_offset, z_offset)
    }
}

// ===== TURBULENCE TYPES ENUM =====
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TurbulenceType {
    None,
    Atmospheric,
    WindGust,
    Thermal,
}

impl TurbulenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TurbulenceType::None => "None",
            TurbulenceType::Atmospheric => "Atmospheric",
            TurbulenceType::WindGust => "Wind Gust",
            TurbulenceType::Thermal => "Thermal",
        }
    }
}

// ===== EVENTS =====
#[derive(Event)]
pub struct StartRiverRaidFlyby;

#[derive(Event)]
pub struct StopRiverRaidFlyby;

#[derive(Event)]
pub struct RestoreCameraPosition;

// ===== COMPONENTS =====
#[derive(Component)]
pub struct RiverRaidCamera {
    pub waypoints: Vec<Vec3>,
    pub look_targets: Vec<Vec3>,
    pub start_time: f32,
    pub duration: f32,
    pub is_flying: bool,
}

// ===== RESOURCES =====
#[derive(Resource)]
pub struct OriginalCameraTransform {
    pub transform: Transform,
}

#[derive(Resource)]
pub struct FlybyState {
    pub duration: f32,
    pub show_debug_path: bool,
    pub camera_height: f32,
    pub camera_distance_behind: f32,
    pub look_ahead_distance: f32,
    pub flight_speed: f32,
    pub path_smoothness: f32,
    // Turbulence settings
    pub turbulence_type: TurbulenceType,
    pub turbulence_intensity: f32,
    pub turbulence_enabled: bool,
}

impl Default for FlybyState {
    fn default() -> Self {
        Self {
            duration: 20.0,
            show_debug_path: true,
            camera_height: 120.0,
            camera_distance_behind: 80.0,
            look_ahead_distance: 200.0,
            flight_speed: 1.0,
            path_smoothness: 0.3,
            turbulence_type: TurbulenceType::Atmospheric,
            turbulence_intensity: 2.0,
            turbulence_enabled: true,
        }
    }
}

// ===== PLUGIN =====
pub struct FlyByPlugin;

impl Plugin for FlyByPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StartRiverRaidFlyby>()
            .add_event::<StopRiverRaidFlyby>()
            .add_event::<RestoreCameraPosition>()
            .init_resource::<FlybyState>()
            .add_systems(Update, (
                flyby_ui_system,
                camera_event_handler_system,
                animate_river_raid_camera,
                debug_path_system,
            ));
    }
}

// ===== UI SYSTEM =====
fn flyby_ui_system(
    mut contexts: EguiContexts,
    mut start_events: EventWriter<StartRiverRaidFlyby>,
    mut stop_events: EventWriter<StopRiverRaidFlyby>,
    mut restore_events: EventWriter<RestoreCameraPosition>,
    terrain_query: Query<Entity, With<HeightmapTerrain>>,
    original_camera_resource: Option<Res<OriginalCameraTransform>>,
    river_raid_camera: Query<&RiverRaidCamera>,
    mut flyby_state: ResMut<FlybyState>,
) {
    egui::Window::new("🎮 River Raid Flyby")
        .default_width(320.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Camera Control");
            
            let has_terrain = !terrain_query.is_empty();
            let has_saved_position = original_camera_resource.is_some();
            let is_flying = !river_raid_camera.is_empty();
            
            ui.separator();
            
            ui.add_enabled_ui(has_terrain && !is_flying, |ui| {
                if ui.button("🚁 Start River Raid Flyby").clicked() {
                    start_events.send(StartRiverRaidFlyby);
                }
            });
            
            ui.add_enabled_ui(is_flying, |ui| {
                if ui.button("⏹️ Stop Flyby").clicked() {
                    stop_events.send(StopRiverRaidFlyby);
                }
            });
            
            ui.add_enabled_ui(has_saved_position && !is_flying, |ui| {
                if ui.button("🔙 Restore Camera").clicked() {
                    restore_events.send(RestoreCameraPosition);
                }
            });

            if !has_terrain {
                ui.colored_label(egui::Color32::RED, "⚠️ Generate terrain first!");
            }

            ui.separator();
            ui.heading("Flight Settings");
            
            ui.add(egui::Slider::new(&mut flyby_state.duration, 10.0..=60.0)
                .text("Flight Duration (seconds)"));
            
            ui.add(egui::Slider::new(&mut flyby_state.flight_speed, 0.3..=2.0)
                .text("Flight Speed"));

            ui.add(egui::Slider::new(&mut flyby_state.path_smoothness, 0.0..=1.0)
                .text("Path Smoothness (0=curved, 1=straight)"));
            
            ui.add(egui::Slider::new(&mut flyby_state.camera_height, 80.0..=300.0)
                .text("Camera Height"));
            
            ui.separator();
            ui.heading("🌪️ Turbulence Effects");
            
            ui.checkbox(&mut flyby_state.turbulence_enabled, "Enable Turbulence");
            
            ui.add_enabled_ui(flyby_state.turbulence_enabled, |ui| {
                // Turbulence type selection
                egui::ComboBox::from_label("Turbulence Type")
                    .selected_text(flyby_state.turbulence_type.as_str())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut flyby_state.turbulence_type, TurbulenceType::None, "None");
                        ui.selectable_value(&mut flyby_state.turbulence_type, TurbulenceType::Atmospheric, "Atmospheric");
                        ui.selectable_value(&mut flyby_state.turbulence_type, TurbulenceType::WindGust, "Wind Gust");
                        ui.selectable_value(&mut flyby_state.turbulence_type, TurbulenceType::Thermal, "Thermal");
                    });
                
                ui.add(egui::Slider::new(&mut flyby_state.turbulence_intensity, 0.0..=10.0)
                    .text("Turbulence Intensity"));
                
                // Turbulence descriptions
                match flyby_state.turbulence_type {
                    TurbulenceType::None => {},
                    TurbulenceType::Atmospheric => {
                        ui.colored_label(egui::Color32::LIGHT_BLUE, "💨 Gentle atmospheric movements");
                    },
                    TurbulenceType::WindGust => {
                        ui.colored_label(egui::Color32::YELLOW, "🌬️ Strong wind gusts with micro-turbulence");
                    },
                    TurbulenceType::Thermal => {
                        ui.colored_label(egui::Color32::RED, "🔥 Rising thermal currents with updrafts");
                    },
                }
            });
            
            ui.separator();
            ui.checkbox(&mut flyby_state.show_debug_path, "🐛 Show Flight Path");
            
            // Status indicators
            ui.separator();
            if is_flying {
                ui.colored_label(egui::Color32::GREEN, "✈️ Flying along river!");
                if flyby_state.turbulence_enabled {
                    ui.colored_label(egui::Color32::YELLOW, format!("🌪️ {} turbulence active", flyby_state.turbulence_type.as_str()));
                }
            } else if has_saved_position {
                ui.colored_label(egui::Color32::YELLOW, "💾 Camera position saved");
            }
        });
}

// ===== EVENT HANDLER =====
fn camera_event_handler_system(
    mut commands: Commands,
    mut start_events: EventReader<StartRiverRaidFlyby>,
    mut stop_events: EventReader<StopRiverRaidFlyby>,
    mut restore_events: EventReader<RestoreCameraPosition>,
    mut camera_query: Query<(Entity, &mut Transform, &mut OrbitCameraController, &mut FlyCameraController), With<Camera3d>>,
    render_config: Res<HeightmapRenderConfig>,
    heightmap_config: Res<HeightmapConfig>,
    heightmap_noise: Res<HeightmapNoise>,
    original_camera_resource: Option<Res<OriginalCameraTransform>>,
    flyby_state: Res<FlybyState>,
    time: Res<Time>,
) {
    // Handle start flyby
    for _ in start_events.read() {
        info!("🚁 Starting River Raid flyby!");
        
        if let Ok((camera_entity, mut camera_transform, mut orbit_controller, mut fly_controller)) = camera_query.get_single_mut() {
            // Disable camera controllers during flyby
            orbit_controller.is_enabled = false;
            fly_controller.is_enabled = false;
            
            // Save current position
            if original_camera_resource.is_none() {
                commands.insert_resource(OriginalCameraTransform {
                    transform: *camera_transform,
                });
            }
            
            // Generate flight path waypoints
            let (waypoints, look_targets) = generate_smooth_river_path(
                &heightmap_config, 
                &heightmap_noise, 
                &render_config, 
                &flyby_state
            );
            
            // Set initial camera position
            if let Some(first_waypoint) = waypoints.first() {
                *camera_transform = Transform::from_translation(*first_waypoint)
                    .looking_at(look_targets[0], Vec3::Y);
            }
            
            // Add RiverRaidCamera component for animation
            commands.entity(camera_entity).insert(RiverRaidCamera {
                waypoints,
                look_targets,
                start_time: time.elapsed_secs(),
                duration: flyby_state.duration,
                is_flying: true,
            });
            
            info!("✅ River Raid flyby started with {} turbulence!", flyby_state.turbulence_type.as_str());
        }
    }
    
    // Handle stop flyby
    for _ in stop_events.read() {
        info!("⏹️ Stopping River Raid flyby");
        
        if let Ok((camera_entity, _, mut orbit_controller, mut fly_controller)) = camera_query.get_single_mut() {
            commands.entity(camera_entity).remove::<RiverRaidCamera>();
            // Re-enable camera controllers
            orbit_controller.is_enabled = true;
            fly_controller.is_enabled = false;
        }
    }
    
    // Handle restore camera
    for _ in restore_events.read() {
        if let Some(original) = &original_camera_resource {
            if let Ok((camera_entity, mut camera_transform, mut orbit_controller, mut fly_controller)) = camera_query.get_single_mut() {
                *camera_transform = original.transform;
                commands.entity(camera_entity).remove::<RiverRaidCamera>();
                commands.remove_resource::<OriginalCameraTransform>();
                // Re-enable camera controllers
                orbit_controller.is_enabled = true;
                fly_controller.is_enabled = false;
                info!("🔙 Camera restored!");
            }
        }
    }
}

// ===== ANIMATION SYSTEM WITH TURBULENCE =====
fn animate_river_raid_camera(
    mut commands: Commands,
    mut camera_query: Query<(Entity, &mut Transform, &mut RiverRaidCamera)>,
    time: Res<Time>,
    flyby_state: Res<FlybyState>,
) {
    // Create turbulence effect instances
    let atmospheric = AtmosphericTurbulence;
    let wind_gust = WindGustTurbulence;
    let thermal = ThermalTurbulence;
    
    for (entity, mut transform, mut river_raid_camera) in camera_query.iter_mut() {
        if !river_raid_camera.is_flying {
            continue;
        }
        
        let elapsed = time.elapsed_secs() - river_raid_camera.start_time;
        let effective_duration = river_raid_camera.duration / flyby_state.flight_speed;
        let progress = (elapsed / effective_duration).clamp(0.0, 1.0);
        
        // Check if flight is complete
        if progress >= 1.0 {
            info!("✅ River Raid flyby completed!");
            commands.entity(entity).remove::<RiverRaidCamera>();
            continue;
        }
        
        let total_waypoints = river_raid_camera.waypoints.len();
        if total_waypoints < 2 {
            continue;
        }
        
        // Get smooth base position and look target
        let base_position = catmull_rom_interpolation(&river_raid_camera.waypoints, progress);
        let base_look_target = catmull_rom_interpolation(&river_raid_camera.look_targets, progress);
        
        // Apply turbulence if enabled
        let final_position = if flyby_state.turbulence_enabled && flyby_state.turbulence_type != TurbulenceType::None {
            let current_time = time.elapsed_secs();
            
            match flyby_state.turbulence_type {
                TurbulenceType::None => base_position,
                TurbulenceType::Atmospheric => {
                    atmospheric.apply_turbulence(base_position, current_time, flyby_state.turbulence_intensity)
                },
                TurbulenceType::WindGust => {
                    wind_gust.apply_turbulence(base_position, current_time, flyby_state.turbulence_intensity)
                },
                TurbulenceType::Thermal => {
                    thermal.apply_turbulence(base_position, current_time, flyby_state.turbulence_intensity)
                },
            }
        } else {
            base_position
        };
        
        // Also apply slight turbulence to look target for more realistic camera shake
        let final_look_target = if flyby_state.turbulence_enabled && flyby_state.turbulence_type != TurbulenceType::None {
            let current_time = time.elapsed_secs();
            let look_turbulence_intensity = flyby_state.turbulence_intensity * 0.3; // Reduced intensity for look target
            
            match flyby_state.turbulence_type {
                TurbulenceType::None => base_look_target,
                TurbulenceType::Atmospheric => {
                    atmospheric.apply_turbulence(base_look_target, current_time, look_turbulence_intensity)
                },
                TurbulenceType::WindGust => {
                    wind_gust.apply_turbulence(base_look_target, current_time, look_turbulence_intensity)
                },
                TurbulenceType::Thermal => {
                    thermal.apply_turbulence(base_look_target, current_time, look_turbulence_intensity)
                },
            }
        } else {
            base_look_target
        };
        
        // Update camera transform
        *transform = Transform::from_translation(final_position)
            .looking_at(final_look_target, Vec3::Y);
    }
}

// ===== HELPER FUNCTIONS =====
fn generate_smooth_river_path(
    heightmap_config: &HeightmapConfig,
    heightmap_noise: &HeightmapNoise,
    render_config: &HeightmapRenderConfig,
    flyby_state: &FlybyState,
) -> (Vec<Vec3>, Vec<Vec3>) {
    let mut camera_waypoints = Vec::new();
    let mut look_targets = Vec::new();
    let num_points = 30;
    
    // Calculate river start and end
    let river_start = heightmap_config.river_start;
    let river_direction = heightmap_config.river_direction.normalize();
    let river_length = render_config.chunk_size * 0.7;
    let river_end = river_start + river_direction * river_length;
    
    for i in 0..=num_points {
        let progress = i as f32 / num_points as f32;
        
        // Create a mostly straight path with occasional gentle curves
        let base_position = river_start.lerp(river_end, progress);
        
        // Add very gentle meandering (much reduced from original)
        let meander_intensity = 1.0 - flyby_state.path_smoothness;
        let gentle_meander = (progress * 2.0 * std::f32::consts::PI).sin() * 20.0 * meander_intensity;
        let perpendicular = Vec2::new(-river_direction.y, river_direction.x);
        let river_pos_2d = base_position + perpendicular * gentle_meander;
        
        // Get terrain height
        let height = heightmap_noise.sample_height_with_river(river_pos_2d.x, river_pos_2d.y, heightmap_config);
        let river_pos = Vec3::new(river_pos_2d.x, height, river_pos_2d.y);
        
        // River direction in 3D
        let river_direction_3d = Vec3::new(river_direction.x, 0.0, river_direction.y);
        
        // Camera position (behind and above)
        let camera_pos = Vec3::new(
            river_pos.x - river_direction_3d.x * flyby_state.camera_distance_behind,
            river_pos.y + flyby_state.camera_height,
            river_pos.z - river_direction_3d.z * flyby_state.camera_distance_behind,
        );
        
        // Look target (ahead along the path)
        let look_target = Vec3::new(
            river_pos.x + river_direction_3d.x * flyby_state.look_ahead_distance,
            river_pos.y + 10.0,
            river_pos.z + river_direction_3d.z * flyby_state.look_ahead_distance,
        );
        
        camera_waypoints.push(camera_pos);
        look_targets.push(look_target);
    }
    
    (camera_waypoints, look_targets)
}

// Catmull-Rom spline interpolation for ultra-smooth movement
fn catmull_rom_interpolation(points: &[Vec3], t: f32) -> Vec3 {
    let n = points.len();
    if n < 2 {
        return points[0];
    }
    
    let segment_length = 1.0 / (n - 1) as f32;
    let segment_index = (t / segment_length).floor() as usize;
    let segment_index = segment_index.min(n - 2);
    
    let local_t = (t - segment_index as f32 * segment_length) / segment_length;
    
    // Get control points for Catmull-Rom
    let p0 = if segment_index == 0 { points[0] } else { points[segment_index - 1] };
    let p1 = points[segment_index];
    let p2 = points[segment_index + 1];
    let p3 = if segment_index + 2 >= n { points[n - 1] } else { points[segment_index + 2] };
    
    // Catmull-Rom formula
    let t2 = local_t * local_t;
    let t3 = t2 * local_t;
    
    0.5 * (
        2.0 * p1 +
        (-p0 + p2) * local_t +
        (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 +
        (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
    )
}

// ===== DEBUG PATH SYSTEM =====
fn debug_path_system(
    flyby_state: Res<FlybyState>,
    mut gizmos: Gizmos,
    render_config: Res<HeightmapRenderConfig>,
    heightmap_config: Res<HeightmapConfig>,
    heightmap_noise: Res<HeightmapNoise>,
    river_raid_camera: Query<&RiverRaidCamera>,
    time: Res<Time>,
) {
    if !flyby_state.show_debug_path {
        return;
    }
    
    let (camera_path_points, look_target_points) = generate_smooth_river_path(
        &heightmap_config, 
        &heightmap_noise, 
        &render_config, 
        &flyby_state
    );
    
    // Draw camera flight path (red line)
    for i in 0..camera_path_points.len() - 1 {
        gizmos.line(
            camera_path_points[i],
            camera_path_points[i + 1],
            Color::srgb(1.0, 0.0, 0.0)
        );
    }
    
    // Draw look direction lines (yellow lines) - every 5th point
    for i in (0..camera_path_points.len()).step_by(5) {
        if i < look_target_points.len() {
            gizmos.line(
                camera_path_points[i],
                look_target_points[i],
                Color::srgb(1.0, 1.0, 0.0)
            );
        }
    }
    
    // Draw current camera position if flying
    if let Ok(river_raid_camera) = river_raid_camera.get_single() {
        let elapsed = time.elapsed_secs() - river_raid_camera.start_time;
        let effective_duration = river_raid_camera.duration / flyby_state.flight_speed;
        let progress = (elapsed / effective_duration).clamp(0.0, 1.0);
        
        let current_pos = catmull_rom_interpolation(&camera_path_points, progress);
        gizmos.sphere(current_pos, 15.0, Color::srgb(0.0, 1.0, 1.0));
    }
    
    // Draw start and end markers
    if let Some(start) = camera_path_points.first() {
        gizmos.sphere(*start, 10.0, Color::srgb(0.0, 1.0, 0.0));
    }
    if let Some(end) = camera_path_points.last() {
        gizmos.sphere(*end, 10.0, Color::srgb(1.0, 0.0, 1.0));
    }
}