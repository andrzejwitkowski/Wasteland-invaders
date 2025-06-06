#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings::{view, globals},
    forward_io::{VertexOutput, FragmentOutput},
    mesh_bindings::mesh,
    utils::PI,
}
#import bevy_render::instance_index::get_instance_index

// Water material uniform
@group(2) @binding(0) var<uniform> material: WaterMaterial;
@group(2) @binding(1) var water_normal_texture: texture_2d<f32>;
@group(2) @binding(2) var water_normal_sampler: sampler;
@group(2) @binding(3) var caustic_texture: texture_2d<f32>;
@group(2) @binding(4) var caustic_sampler: sampler;

struct WaterMaterial {
    wave_params: vec4<f32>,    // amplitude, frequency, speed, steepness
    surface_params: vec4<f32>, // roughness, metallic, specular, refraction_strength
    foam_params: vec4<f32>,    // scale, threshold, intensity, color_intensity
    caustic_params: vec4<f32>, // scale, speed, intensity, depth_fade
}

// Custom vertex output for water
struct WaterVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) wave_height: f32,
    @location(5) foam_factor: f32,
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

fn get_wave_height(pos: vec2<f32>, time: f32) -> f32 {
    let wave1 = sin(pos.x * material.wave_params.y + time * material.wave_params.z) * material.wave_params.x;
    let wave2 = sin(pos.y * material.wave_params.y * 0.8 + time * material.wave_params.z * 1.2) * material.wave_params.x * 0.7;
    let wave3 = sin((pos.x + pos.y) * material.wave_params.y * 1.3 + time * material.wave_params.z * 0.9) * material.wave_params.x * 0.5;
    return wave1 + wave2 + wave3;
}

fn get_wave_normal(pos: vec2<f32>, time: f32) -> vec3<f32> {
    let epsilon = 0.01;
    let height_center = get_wave_height(pos, time);
    let height_right = get_wave_height(pos + vec2<f32>(epsilon, 0.0), time);
    let height_up = get_wave_height(pos + vec2<f32>(0.0, epsilon), time);
    
    let dx = (height_right - height_center) / epsilon;
    let dy = (height_up - height_center) / epsilon;
    
    return normalize(vec3<f32>(-dx, 1.0, -dy));
}

@vertex
fn vertex(vertex: Vertex) -> WaterVertexOutput {
    var out: WaterVertexOutput;
    
    // Get model matrix
    let model = mesh_functions::get_world_from_local(vertex.instance_index);
    
    // Transform vertex position to world space
    var world_position = mesh_functions::mesh_position_local_to_world(
        model, 
        vec4<f32>(vertex.position, 1.0)
    );
    
    // Get time from globals
    let time = globals.time;
    let wave_height = get_wave_height(world_position.xz, time);
    world_position.y += wave_height;
    
    // Calculate wave normal
    let wave_normal = get_wave_normal(world_position.xz, time);
    let world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal, 
        vertex.instance_index
    );
    let final_normal = normalize(mix(world_normal, wave_normal, 0.8));
    
    // Calculate foam factor based on wave steepness
    let foam_factor = smoothstep(
        material.foam_params.y, 
        material.foam_params.y + 0.1, 
        abs(wave_height) / material.wave_params.x
    );
    
    // Transform to clip space using the correct matrix field
    out.clip_position = view.clip_from_world * world_position;
    out.world_position = world_position;
    out.world_normal = final_normal;
    out.world_tangent = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    out.uv = vertex.uv;
    out.wave_height = wave_height;
    out.foam_factor = foam_factor;
    
    return out;
}

@fragment  
fn fragment(in: WaterVertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    
    // Get time from globals
    let time = globals.time;
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    
    // Sample normal map with animated UVs
    let normal_uv1 = in.uv * 2.0 + time * 0.02;
    let normal_uv2 = in.uv * 3.0 - time * 0.03;
    let normal1 = textureSample(water_normal_texture, water_normal_sampler, normal_uv1).xyz * 2.0 - 1.0;
    let normal2 = textureSample(water_normal_texture, water_normal_sampler, normal_uv2).xyz * 2.0 - 1.0;
    let combined_normal = normalize(mix(normal1, normal2, 0.5));
    
    // Combine with wave normal
    let final_normal = normalize(in.world_normal + combined_normal * 0.3);
    
    // Calculate fresnel effect
    let fresnel = pow(1.0 - max(dot(view_dir, final_normal), 0.0), 3.0);
    
    // Sample caustics
    let caustic_uv1 = in.world_position.xz * material.caustic_params.x * 0.1 + time * material.caustic_params.y * 0.01;
    let caustic_uv2 = in.world_position.xz * material.caustic_params.x * 0.15 - time * material.caustic_params.y * 0.015;
    let caustic1 = textureSample(caustic_texture, caustic_sampler, caustic_uv1).r;
    let caustic2 = textureSample(caustic_texture, caustic_sampler, caustic_uv2).r;
    let caustics = (caustic1 * caustic2) * material.caustic_params.z;
    
    // Base water color
    let deep_color = vec3<f32>(0.0, 0.1, 0.3);
    let shallow_color = vec3<f32>(0.0, 0.4, 0.7);
    let depth_factor = smoothstep(-2.0, 0.0, in.world_position.y);
    let base_color = mix(deep_color, shallow_color, depth_factor);
    
    // Add caustics
    let final_color = base_color + caustics * vec3<f32>(0.8, 1.0, 1.0);
    
    // Add foam
    let foam_color = vec3<f32>(1.0, 1.0, 1.0) * in.foam_factor * material.foam_params.z;
    let color_with_foam = mix(final_color, foam_color, in.foam_factor * 0.8);
    
    // Set output
    out.color = vec4<f32>(color_with_foam, mix(0.8, 0.95, fresnel));
    
    return out;
}
