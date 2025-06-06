#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings::{view, globals},
    forward_io::{FragmentOutput},
    mesh_bindings::mesh,
}

// Single material struct that matches your Rust WaterMaterial
struct WaterMaterial {
    // Wave parameters
    wave_amplitude: f32,
    wave_frequency: f32,
    wave_speed: f32,
    wave_steepness: f32,
    
    // Surface parameters  
    roughness: f32,
    metallic: f32,
    reflectance: f32,
    transparency: f32,
    
    // Foam parameters
    foam_intensity: f32,
    foam_cutoff: f32,
    foam_scale: f32,
    foam_speed: f32,
    
    // Caustic parameters
    caustic_intensity: f32,
    caustic_scale: f32,
    caustic_speed: f32,
    caustic_depth_falloff: f32,
    
    // Water color
    water_color: vec4<f32>,
    
    // Time (updated each frame)
    time: f32,
    _padding1: f32,
    _padding2: f32,
    _padding3: f32,
}

@group(2) @binding(0) var<uniform> material: WaterMaterial;

// Textures (if you want them later)
// @group(2) @binding(1) var normal_map: texture_2d<f32>;
// @group(2) @binding(2) var normal_map_sampler: sampler;

// Custom vertex output with our water data
struct WaterVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) water_data: vec4<f32>,  // Our custom data
}

// Wave displacement function
fn get_wave_displacement(world_pos: vec2<f32>, time: f32) -> vec3<f32> {
    let amplitude = material.wave_amplitude;
    let frequency = material.wave_frequency;
    let speed = material.wave_speed;
    
    // Multiple wave directions
    let wave1 = vec2<f32>(1.0, 0.0);
    let wave2 = vec2<f32>(-0.7, 0.7);
    
    // Calculate waves
    let phase1 = dot(wave1, world_pos) * frequency + time * speed;
    let phase2 = dot(wave2, world_pos) * frequency * 0.8 + time * speed * 1.2;
    
    let height = sin(phase1) * amplitude + sin(phase2) * amplitude * 0.5;
    
    // Calculate normals (simplified)
    let dx = cos(phase1) * amplitude * frequency;
    let dz = cos(phase2) * amplitude * frequency * 0.5;
    
    return vec3<f32>(dx, height, dz);
}

@vertex
fn vertex(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32>) -> WaterVertexOutput {
    var out: WaterVertexOutput;
    
    // Get world position
    let world_position = mesh_functions::mesh_position_local_to_world(
        mesh_functions::get_world_from_local(0u), 
        vec4<f32>(position, 1.0)
    );
    
    // Apply wave displacement
    let wave_data = get_wave_displacement(world_position.xz, material.time);
    let displaced_world_pos = vec4<f32>(
        world_position.x + wave_data.x * material.wave_steepness,
        world_position.y + wave_data.y,
        world_position.z + wave_data.z * material.wave_steepness,
        1.0
    );
    
    out.position = view.clip_from_world * displaced_world_pos;
    out.world_position = displaced_world_pos;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(normal, 0u);
    out.uv = uv;
    
    // Pack data for fragment shader
    let foam_factor = clamp(wave_data.y / material.wave_amplitude, 0.0, 1.0);
    out.water_data = vec4<f32>(
        wave_data.y,     // wave height
        foam_factor,     // foam factor
        1.0,            // depth factor (placeholder)
        length(wave_data.xz) // wave velocity
    );
    
    return out;
}

@fragment
fn fragment(in: WaterVertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    
    // Unpack data
    let wave_height = in.water_data.r;
    let foam_factor = in.water_data.g;
    let depth_factor = in.water_data.b;
    let wave_velocity = in.water_data.a;
    
    // Calculate view direction and fresnel
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    let fresnel = pow(1.0 - max(dot(in.world_normal, view_dir), 0.0), 3.0);
    
    // Base water color
    var final_color = material.water_color.rgb;
    
    // Simple foam effect
    let foam_color = vec3<f32>(1.0, 1.0, 0.95);
    final_color = mix(final_color, foam_color, foam_factor * material.foam_intensity);
    
    // Apply transparency with fresnel
    let alpha = mix(material.transparency, 0.95, fresnel);
    
    out.color = vec4<f32>(final_color, alpha);
    
    return out;
}
