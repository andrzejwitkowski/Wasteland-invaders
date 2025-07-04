#import bevy_pbr::pbr_prelude
#import bevy_pbr::forward_io::{Vertex, VertexOutput, FragmentOutput}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::apply_pbr_lighting

// Caustic material parameters
struct CausticMaterial {
    caustic_params: vec4<f32>, // intensity, scale, speed, depth_fade
    water_params: vec4<f32>,   // wave_amplitude, wave_frequency, wave_speed, wave_steepness
    misc_params: vec4<f32>,    // water_surface_y, unused, unused, time
};

@group(2) @binding(100)
var<uniform> caustic_material: CausticMaterial;

// Simplex 2D noise implementation (reused from water shader)
fn mod289_vec2(x: vec2<f32>) -> vec2<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn mod289_vec3(x: vec3<f32>) -> vec3<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn permute3(x: vec3<f32>) -> vec3<f32> {
    return mod289_vec3(((x * 34.0) + 1.0) * x);
}

fn simplex2d(v: vec2<f32>) -> f32 {
    let C = vec4<f32>(0.211324865405187, 0.366025403784439, -0.577350269189626, 0.024390243902439);
    
    var i = floor(v + dot(v, vec2<f32>(C.y)));
    let x0 = v - i + dot(i, vec2<f32>(C.x));
    
    var i1: vec2<f32>;
    if (x0.x > x0.y) {
        i1 = vec2<f32>(1.0, 0.0);
    } else {
        i1 = vec2<f32>(0.0, 1.0);
    }
    
    let x1 = x0.xy + C.xx - i1;
    let x2 = x0.xy + C.zz;
    
    i = mod289_vec2(i);
    let p = permute3(permute3(i.y + vec3<f32>(0.0, i1.y, 1.0)) + i.x + vec3<f32>(0.0, i1.x, 1.0));
    
    var m = max(0.5 - vec3<f32>(dot(x0, x0), dot(x1, x1), dot(x2, x2)), vec3<f32>(0.0));
    m = m * m;
    m = m * m;
    
    let x = 2.0 * fract(p * C.w) - 1.0;
    let h = abs(x) - 0.5;
    let ox = floor(x + 0.5);
    let a0 = x - ox;
    
    m = m * (1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h));
    
    let g = vec3<f32>(
        a0.x * x0.x + h.x * x0.y,
        a0.y * x1.x + h.y * x1.y,
        a0.z * x2.x + h.z * x2.y
    );
    
    return 130.0 * dot(m, g);
}

// Simulate water surface height at given position
fn get_water_surface_height(pos: vec2<f32>, time: f32) -> f32 {
    let wave_amplitude = caustic_material.water_params.x;
    let wave_frequency = caustic_material.water_params.y;
    let wave_speed = caustic_material.water_params.z;
    
    let animated_pos = pos * wave_frequency;
    let time_offset1 = vec2<f32>(time * wave_speed * 0.3, time * wave_speed * 0.2);
    let time_offset2 = vec2<f32>(time * wave_speed * 0.1, time * wave_speed * 0.4);
    
    let noise1 = simplex2d(animated_pos + time_offset1);
    let noise2 = simplex2d(animated_pos * 0.7 + time_offset2);
    
    let combined_noise = noise1 * 0.7 + noise2 * 0.3;
    return combined_noise * wave_amplitude;
}

// Calculate caustic intensity based on water surface refraction
fn calculate_caustics(world_pos: vec3<f32>, time: f32) -> f32 {
    let caustic_scale = caustic_material.caustic_params.y;
    let caustic_speed = caustic_material.caustic_params.z;
    let water_surface_y = caustic_material.misc_params.x;
    
    // Calculate depth from water surface
    let depth = max(0.0, water_surface_y - world_pos.y);
    
    // Sample water surface at multiple points to simulate light refraction
    let sample_offset = 0.5 * caustic_scale;
    let base_pos = world_pos.xz;
    
    // Sample water surface heights in a cross pattern
    let h_center = get_water_surface_height(base_pos, time);
    let h_right = get_water_surface_height(base_pos + vec2<f32>(sample_offset, 0.0), time);
    let h_left = get_water_surface_height(base_pos + vec2<f32>(-sample_offset, 0.0), time);
    let h_forward = get_water_surface_height(base_pos + vec2<f32>(0.0, sample_offset), time);
    let h_back = get_water_surface_height(base_pos + vec2<f32>(0.0, -sample_offset), time);
    
    // Calculate surface normal approximation
    let dx = (h_right - h_left) / (2.0 * sample_offset);
    let dz = (h_forward - h_back) / (2.0 * sample_offset);
    
    // Calculate light ray bending (simplified refraction)
    let light_dir = vec3<f32>(0.0, -1.0, 0.0); // Downward light
    let surface_normal = normalize(vec3<f32>(-dx, 1.0, -dz));
    
    // Simple refraction approximation
    let n1 = 1.0; // Air
    let n2 = 1.33; // Water
    let cos_i = dot(-light_dir, surface_normal);
    let sin_i_squared = 1.0 - cos_i * cos_i;
    let sin_t_squared = (n1 / n2) * (n1 / n2) * sin_i_squared;
    
    if (sin_t_squared > 1.0) {
        return 0.0; // Total internal reflection
    }
    
    let cos_t = sqrt(1.0 - sin_t_squared);
    let refraction_strength = abs(cos_i - cos_t);
    
    // Add multiple layers of caustic patterns
    let caustic_time = time * caustic_speed;
    let caustic_pos = world_pos.xz * caustic_scale;
    
    // Primary caustic pattern
    let c1 = simplex2d(caustic_pos + vec2<f32>(caustic_time * 0.3, caustic_time * 0.2));
    let c2 = simplex2d(caustic_pos * 1.3 + vec2<f32>(caustic_time * -0.2, caustic_time * 0.4));
    let c3 = simplex2d(caustic_pos * 0.8 + vec2<f32>(caustic_time * 0.1, caustic_time * -0.3));
    
    // Secondary caustic pattern for more detail
    let c4 = simplex2d(caustic_pos * 2.1 + vec2<f32>(caustic_time * 0.15, caustic_time * 0.25));
    let c5 = simplex2d(caustic_pos * 1.7 + vec2<f32>(caustic_time * -0.1, caustic_time * 0.2));
    
    // Combine patterns
    let primary_caustic = (c1 + c2 * 0.7 + c3 * 0.5) * 0.4 + 0.5;
    let secondary_caustic = (c4 + c5 * 0.6) * 0.3 + 0.5;
    
    let combined_caustic = primary_caustic * 0.7 + secondary_caustic * 0.3;
    
    // Apply focusing effect based on refraction
    let focused_caustic = pow(combined_caustic, 2.0 - refraction_strength);
    
    // Add some sharp caustic lines
    let sharp_caustic = pow(focused_caustic, 3.0) * 2.0;
    let final_caustic = mix(focused_caustic, sharp_caustic, 0.3);
    
    // Apply depth fade
    let depth_fade = caustic_material.caustic_params.w;
    let fade_factor = exp(-depth * depth_fade);
    
    return final_caustic * fade_factor;
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    
    out.position = position_world_to_clip(world_position.xyz);
    out.world_position = world_position;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex.instance_index);
#endif

#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

    out.instance_index = vertex.instance_index;
    return out;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    
    let time = caustic_material.misc_params.w;
    let caustic_intensity = caustic_material.caustic_params.x;
    
    // Calculate caustic effect
    let caustic_effect = calculate_caustics(in.world_position.xyz, time);
    
    // Apply caustics as additional lighting
    let caustic_color = vec3<f32>(0.9, 1.0, 0.95); // Slightly blue-tinted white
    let caustic_contribution = caustic_color * caustic_effect * caustic_intensity;
    
    // Add caustics to emissive to make them glow
    pbr_input.material.emissive = vec4<f32>(caustic_contribution, 0.0);
    
    // Make the material slightly more reflective where caustics are strong
    pbr_input.material.perceptual_roughness = mix(
        pbr_input.material.perceptual_roughness,
        pbr_input.material.perceptual_roughness * 0.7,
        caustic_effect * 0.3
    );
    
    let final_color = apply_pbr_lighting(pbr_input);
    
    var out: FragmentOutput;
    out.color = final_color;
    return out;
}
