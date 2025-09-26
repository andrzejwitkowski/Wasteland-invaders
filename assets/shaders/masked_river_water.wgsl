#import bevy_pbr::pbr_prelude
#import bevy_pbr::forward_io::{Vertex, VertexOutput, FragmentOutput}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::apply_pbr_lighting

struct WaterMaterial {
    wave_params: vec4<f32>,
    misc_params: vec4<f32>,
    river_params: vec4<f32>,
    river_position: vec4<f32>,
    terrain_params: vec4<f32>,
    debug_options: vec4<f32>, // x=show_mask, y=margin_step_world, z=bank_fill_ratio
};

@group(2) @binding(100)
var<uniform> water_material: WaterMaterial;

// Accessors - match terrain shader exactly
fn get_river_width() -> f32 { return water_material.river_params.x; }
fn get_bank_slope_distance() -> f32 { return water_material.river_params.y; }
fn get_meander_frequency() -> f32 { return water_material.river_params.z; }
fn get_meander_amplitude() -> f32 { return water_material.river_params.w; }

fn get_river_depth() -> f32 { return water_material.terrain_params.z; }
fn get_river_start() -> vec2<f32> { return water_material.river_position.xy; }
fn get_river_dir_raw() -> vec2<f32> { return water_material.river_position.zw; }

fn get_transparency() -> f32 { return water_material.misc_params.x; }
fn get_foam_intensity() -> f32 { return water_material.misc_params.y; }
fn get_foam_cutoff() -> f32 { return water_material.misc_params.z; }
fn get_time() -> f32 { return water_material.misc_params.w; }

fn mod289_vec2(x: vec2<f32>) -> vec2<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn mod289_vec3(x: vec3<f32>) -> vec3<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn permute3(x: vec3<f32>) -> vec3<f32> {
    return mod289_vec3(((x * 34.0) + 1.0) * x);
}

// Vector math helpers - same as terrain shader
fn vec2_length(v: vec2<f32>) -> f32 {
    return sqrt(v.x * v.x + v.y * v.y);
}

fn vec2_normalize(v: vec2<f32>) -> vec2<f32> {
    let len = vec2_length(v);
    if (len > 0.0) {
        return v / len;
    }
    return v;
}

fn vec2_dot(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.x + a.y * b.y;
}

fn vec2_distance(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return vec2_length(a - b);
}

// EXACT SAME meander calculation as terrain shader
fn calculate_realistic_meander(distance_along_river: f32) -> f32 {
    let meander_frequency = get_meander_frequency();
    let meander_phase = distance_along_river * meander_frequency;
    
    // Primary meandering - base sine wave
    let primary_meander = sin(meander_phase * 6.28318530718);
    
    // Secondary meandering
    let secondary_phase = distance_along_river * meander_frequency * 1.7;
    let secondary_meander = sin(secondary_phase * 6.28318530718) * 0.4;
    
    // Chaotic variations (simplified version of terrain's FBM)
    let chaos_variation = sin(meander_phase * 0.37) * 0.3;
    
    // Scale variation (simplified)
    let scale_variation = sin(meander_phase * 0.3) * 0.2;
    
    // Asymmetric variations
    let asymmetry = sin(meander_phase * 0.8 + 1.57) * 0.2;
    
    // Combine components - same weights as terrain shader
    let base_meander = primary_meander * 0.7 + secondary_meander * 0.3;
    let chaotic_component = chaos_variation * 0.6 * 0.5;
    let asymmetric_component = asymmetry * 0.2;
    
    let total_meander = (base_meander + chaotic_component + asymmetric_component) * (1.0 + scale_variation);
    
    return total_meander * get_meander_amplitude();
}

// EXACT SAME river calculation as terrain shader
fn calculate_river_center_and_distance(pos: vec2<f32>) -> vec2<f32> {
    let river_start = get_river_start();
    let river_dir = get_river_dir_raw();
    let base_river_dir = vec2_normalize(river_dir);
    
    let relative_pos = pos - river_start;
    let distance_along_river = vec2_dot(relative_pos, base_river_dir);
    
    // Generate meander offset using same function
    let meander_offset = calculate_realistic_meander(distance_along_river);
    
    // Calculate river center with meandering
    let perpendicular = vec2(-base_river_dir.y, base_river_dir.x);
    let river_center = river_start + base_river_dir * distance_along_river + perpendicular * meander_offset;
    
    // Distance from point to river centerline
    let distance_to_river = vec2_distance(pos, river_center);
    
    return vec2(distance_to_river, distance_along_river);
}

// EXACT SAME width variation as terrain shader
fn width_noise(along: f32) -> f32 {
    // Same as terrain: sample_noise(distance_along_river * 0.0005, 0) * 0.3
    // Using simplified noise for consistency
    return (sin(along * 0.0005) * 0.5 + 0.5) * 0.3;
}

// Main river distance function - matches terrain shader logic
fn river_dist_and_along(pos: vec2<f32>) -> vec2<f32> {
    let result = calculate_river_center_and_distance(pos);
    let dist = result.x;
    let along = result.y;
    
    // Dynamic width variation - same as terrain shader
    let w_noise = width_noise(along);
    let actual_river_width = get_river_width() * (1.0 + w_noise);
    
    return vec2(dist, along);
}

// Simplex noise (2D) minimal (shortened gradient hash version)
fn hash(p: vec2<i32>) -> f32 {
    let h = f32(p.x * 374761393 + p.y * 668265263);
    return fract(sin(h) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash(i);
    let b = hash(i + vec2<i32>(1,0));
    let c = hash(i + vec2<i32>(0,1));
    let d = hash(i + vec2<i32>(1,1));
    return mix(mix(a,b,u.x), mix(c,d,u.x), u.y);
}

fn simplex2d(v: vec2<f32>) -> f32 {
    let C = vec4<f32>(0.211324865405187, 0.366025403784439, -0.577350269189626, 0.024390243902439);
    
    // First corner
    var i = floor(v + dot(v, vec2<f32>(C.y)));
    let x0 = v - i + dot(i, vec2<f32>(C.x));
    
    // Other corners
    var i1: vec2<f32>;
    if (x0.x > x0.y) {
        i1 = vec2<f32>(1.0, 0.0);
    } else {
        i1 = vec2<f32>(0.0, 1.0);
    }
    
    // Fixed: Create x12 properly without invalid assignment
    let x1 = x0.xy + C.xx - i1;
    let x2 = x0.xy + C.zz; // C.zz is equivalent to vec2(-1.0 + 2.0 * C.x)
    
    // Permutations
    i = mod289_vec2(i);
    let p = permute3(permute3(i.y + vec3<f32>(0.0, i1.y, 1.0)) + i.x + vec3<f32>(0.0, i1.x, 1.0));
    
    var m = max(0.5 - vec3<f32>(dot(x0, x0), dot(x1, x1), dot(x2, x2)), vec3<f32>(0.0));
    m = m * m;
    m = m * m;
    
    // Gradients
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

fn fbm(pos: vec2<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    
    for (var i = 0; i < octaves; i = i + 1) {
        value += amplitude * simplex2d(pos * frequency);
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    
    return value;
}

fn get_noise_wave(pos: vec2<f32>, time: f32) -> vec4<f32> {
    // Extract wave parameters from the uniform
    let wave_amplitude = water_material.wave_params.x;
    let wave_frequency = water_material.wave_params.y; // Controls noise scale
    let wave_speed = water_material.wave_params.z;
    let wave_steepness = water_material.wave_params.w; // Controls octaves

    // Scale down frequency for larger waves
    let scaled_frequency = wave_frequency * 0.1;
    
    // Create animated position for noise
    let animated_pos = pos * scaled_frequency;
    let time_offset1 = vec2<f32>(time * wave_speed * 0.3, time * wave_speed * 0.2);
    let time_offset2 = vec2<f32>(time * wave_speed * 0.1, time * wave_speed * 0.4);
    
    // Multiple octaves for more complex waves
    let octaves = i32(clamp(wave_steepness * 8.0, 2.0, 8.0));
    
    // Primary wave pattern
    let noise1 = fbm(animated_pos + time_offset1, octaves);
    
    // Secondary wave pattern for more complexity
    let noise2 = fbm(animated_pos * 0.7 + time_offset2, max(2, octaves - 2));
    
    // Combine noises
    let combined_noise = noise1 * 0.7 + noise2 * 0.3;
    
    // Calculate height
    let height = combined_noise * wave_amplitude * 3.0;
    
    // Calculate horizontal displacement for more realistic water motion
    let displacement_scale = wave_amplitude * 0.2;
    let dx = fbm(animated_pos + vec2<f32>(1000.0, 0.0) + time_offset1, 3) * displacement_scale;
    let dz = fbm(animated_pos + vec2<f32>(0.0, 1000.0) + time_offset1, 3) * displacement_scale;
    
    return vec4<f32>(pos.x + dx, height, pos.y + dz, 0.0);
}

fn get_noise_wave_normal(pos: vec2<f32>, time: f32) -> vec3<f32> {
    let eps = 0.01;
    
    // Sample neighboring points
    let center = get_noise_wave(pos, time);
    let right = get_noise_wave(pos + vec2<f32>(eps, 0.0), time);
    let forward = get_noise_wave(pos + vec2<f32>(0.0, eps), time);
    
    // Calculate tangent vectors
    let tangent_x = (right.xyz - center.xyz) / eps;
    let tangent_z = (forward.xyz - center.xyz) / eps;
    
    // Calculate normal via cross product
    return normalize(cross(tangent_x, tangent_z));
}

fn calculate_wave_height(pos: vec2<f32>, time: f32) -> f32 {
    let wave_params = water_material.wave_params;
    let amplitude = wave_params.x;
    let frequency = wave_params.y;
    let speed = wave_params.z;
    let steepness = wave_params.w;
    
    // Use simplex noise for more organic waves
    let noise_pos1 = pos * frequency * 0.1 + vec2<f32>(time * speed * 0.3, time * speed * 0.2);
    let noise_pos2 = pos * frequency * 0.05 + vec2<f32>(time * speed * 0.1, time * speed * 0.4);
    
    // Multiple layers of noise for more complex waves
    let wave1 = simplex2d(noise_pos1) * amplitude;
    let wave2 = simplex2d(noise_pos2) * amplitude * 0.5;
    
    // Add some directional waves for more realism
    let directional_wave = sin(pos.x * frequency * 0.02 + time * speed) * amplitude * 0.3;
    
    return (wave1 + wave2 + directional_wave) * 0.5;
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_pos4 = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    let t = get_time();

    let wave_displacement = get_noise_wave(world_pos4.xz, t);
    let wave_height = wave_displacement.y;

    var displaced = world_pos4;
    displaced.y = world_pos4.y + wave_height;
    displaced.x = wave_displacement.x;
    // displaced.z = world_pos4.z + wave_displacement.z;

    // var displaced_world_pos = initial_world_pos;
    // displaced_world_pos.y = initial_world_pos.y + wave_displacement.y; // Use the actual wave height
    // displaced_world_pos.x = wave_displacement.x;
    // displaced_world_pos.y += wave_displacement.y;

    out.position = position_world_to_clip(displaced.xyz);
    out.world_position = displaced;
    out.world_normal = get_noise_wave_normal(world_pos4.xz, t);
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

fn fresnel(cos_theta: f32, f0: f32) -> f32 {
    return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}

fn fresnel_water(view_dir: vec3<f32>, normal: vec3<f32>) -> f32 {
    let cos_theta = max(0.0, dot(view_dir, normal));
    let f0 = 0.02; // Water's reflectance at normal incidence (~2%)
    return fresnel(cos_theta, f0);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let da = river_dist_and_along(in.world_position.xz);
    let dist = da.x;
    let along = da.y;

    // Dynamic width variation - EXACTLY like terrain shader
    let w_noise = width_noise(along);
    let actual_river_width = get_river_width() * (1.0 + w_noise);

    // Use the SAME river profile calculation as terrain shader
    let water_edge = actual_river_width * 0.5;
    let bank_end = water_edge + get_bank_slope_distance();
    
    // Discard outside river area - match terrain's river carving area
    if (dist > bank_end) {
        discard;
    }

    // Edge alpha fade (last few world units into banks)
    let edge_fade = smoothstep(water_edge, bank_end, dist);

    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let t = get_time();
    
    // SOPHISTICATED WAVE HEIGHT CALCULATION from simplex_water.wgsl
    let wave_height = calculate_wave_height(in.world_position.xz, t);

    // Water color based on depth (distance from center)
    let depth_ratio = clamp(dist / water_edge, 0.0, 1.0);
    
    // Deep river in center, shallower near banks (matching terrain shader colors)
    let deep_color = vec3<f32>(0.1, 0.2, 0.4);    // Same as terrain's river color
    let shallow_color = vec3<f32>(0.3, 0.5, 0.7); // Same as terrain's water color
    
    let base_color = mix(deep_color, shallow_color, depth_ratio);

    // SOPHISTICATED FOAM CALCULATION from simplex_water.wgsl
    let foam_factor = smoothstep(get_foam_cutoff() - 0.1, get_foam_cutoff() + 0.1, abs(wave_height)) * get_foam_intensity();
    let foam_color = vec3<f32>(1.0, 1.0, 1.0);
    let color = mix(base_color, foam_color, foam_factor);
    
    // Alpha based on position in river with transparency control
    let center_alpha = get_transparency();
    let edge_alpha = center_alpha * 0.3;
    let alpha = mix(center_alpha, edge_alpha, edge_fade) * (1.0 - foam_factor * 0.3);

    pbr_input.material.base_color = vec4<f32>(color, alpha);
    pbr_input.material.perceptual_roughness = 0.02; // Very smooth/reflective like simplex_water
    pbr_input.material.metallic = 0.0;

    // SOPHISTICATED FRESNEL EFFECT from simplex_water.wgsl
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    let fresnel_factor = fresnel_water(view_dir, in.world_normal);
    
    // Apply Fresnel effect to make water more reflective
    let reflection_color = vec3<f32>(0.8, 0.9, 1.0);
    let final_col = mix(color, reflection_color, fresnel_factor * 0.7);
    
    pbr_input.material.base_color = vec4<f32>(final_col, alpha);

    let lit = apply_pbr_lighting(pbr_input);
    var out: FragmentOutput;
    out.color = lit;
    return out;
}