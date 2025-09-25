#import bevy_pbr::pbr_prelude
#import bevy_pbr::forward_io::{Vertex, VertexOutput, FragmentOutput}

// Import the view uniform binding to get the view-projection matrix.
#import bevy_pbr::mesh_view_bindings::view
// Import the entire mesh_functions module to access its helpers.
#import bevy_pbr::mesh_functions
// Import the specific helper for clip-space conversion.
#import bevy_pbr::view_transformations::position_world_to_clip

#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::apply_pbr_lighting

// This struct defines our custom material data.
struct WaterMaterial {
    wave_params: vec4<f32>,
    misc_params: vec4<f32>,
};

// Bind our custom material data to group 2.
@group(2) @binding(100)
var<uniform> water_material: WaterMaterial;

// Simplex 2D noise implementation
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

// Fractional Brownian Motion using simplex noise
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

// Fresnel effect calculation
fn fresnel(cos_theta: f32, f0: f32) -> f32 {
    // Schlick's approximation of Fresnel reflectance
    return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}

// Enhanced Fresnel with configurable parameters
fn fresnel_water(view_dir: vec3<f32>, normal: vec3<f32>) -> f32 {
    let cos_theta = max(0.0, dot(view_dir, normal));
    let f0 = 0.02; // Water's reflectance at normal incidence (~2%)
    return fresnel(cos_theta, f0);
}

// Noise-based wave displacement function
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

// Calculate wave normal using noise derivatives
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
    
    // Get the world matrix using the helper function.
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    
    // Get initial world position
    let initial_world_pos = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));

    let time = water_material.misc_params.w;
    
    // FIXED: Properly use the wave displacement function
    let wave_displacement = get_noise_wave(initial_world_pos.xz, time);
    
    // Apply wave displacement to create visible waves
    var displaced_world_pos = initial_world_pos;
    displaced_world_pos.y = initial_world_pos.y + wave_displacement.y; // Use the actual wave height
    displaced_world_pos.x = wave_displacement.x;
    displaced_world_pos.y = wave_displacement.y; // Apply horizontal displacement for realism
    
    // Calculate wave normal for better lighting
    let wave_normal = get_noise_wave_normal(initial_world_pos.xz, time);
    
    // Populate the VertexOutput struct
    out.position = position_world_to_clip(displaced_world_pos.xyz);
    out.world_position = displaced_world_pos;
    out.world_normal = wave_normal;
    out.uv = vertex.uv;
    
    // Use the correct function for tangent transformation.
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex.instance_index);
#endif

#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex.instance_index);
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
    out.instance_index = vertex.instance_index;

    return out;
}

// Noise-based caustics function
fn get_noise_caustics(world_pos: vec3<f32>, time: f32) -> f32 {
    let uv = world_pos.xz * 0.3;
    let caustic_time = time * 0.8;
    
    // Use noise for more organic caustics
    let c1 = simplex2d(uv + vec2<f32>(caustic_time * 0.3, caustic_time * 0.2));
    let c2 = simplex2d(uv * 1.5 + vec2<f32>(caustic_time * -0.2, caustic_time * 0.4));
    let c3 = simplex2d(uv * 0.7 + vec2<f32>(caustic_time * 0.1, caustic_time * -0.3));
    
    let combined = (c1 + c2 * 0.5 + c3 * 0.3) * 0.5 + 0.5;
    return pow(combined, 2.0) * 1.5;
}

fn get_water_depth_color(world_pos: vec3<f32>, surface_height: f32) -> vec3<f32> {
    // Assume water bottom is at y = -2.0
    let depth = max(0.0, surface_height - (-2.0));
    
    let shallow_color = vec3<f32>(0.4, 0.8, 0.9);  // Light blue-green
    let deep_color = vec3<f32>(0.1, 0.3, 0.6);     // Dark blue
    let very_deep_color = vec3<f32>(0.05, 0.1, 0.3); // Very dark blue
    
    let depth_factor = smoothstep(0.0, 3.0, depth);
    let very_deep_factor = smoothstep(2.0, 5.0, depth);
    
    return mix(
        mix(shallow_color, deep_color, depth_factor),
        very_deep_color,
        very_deep_factor
    );
}

fn get_water_alpha(depth: f32, fresnel_factor: f32) -> f32 {
    let base_alpha = smoothstep(0.0, 2.0, depth) * 0.8 + 0.2;
    return mix(base_alpha, 1.0, fresnel_factor * 0.5);
}

// Noise-based foam generation
fn get_noise_foam_factor(world_pos: vec3<f32>, wave_height: f32, time: f32) -> f32 {
    // Foam based on wave steepness and height
    let foam_from_waves = smoothstep(0.3, 1.0, wave_height);
    
    // Animated foam texture using noise
    let foam_pos = world_pos.xz * 8.0;
    let foam_time = time * 1.5;
    let foam_noise1 = simplex2d(foam_pos + vec2<f32>(foam_time, foam_time * 0.7));
    let foam_noise2 = simplex2d(foam_pos * 1.3 + vec2<f32>(foam_time * -0.8, foam_time * 1.2));
    
    let foam_texture = (foam_noise1 + foam_noise2 * 0.5) * 0.5 + 0.5;
    let foam_mask = smoothstep(0.4, 0.9, foam_texture) * 0.4;
    
    return min(1.0, foam_from_waves + foam_mask);
}

// fn calculate_wave_height(pos: vec2<f32>, time: f32) -> f32 {
//     let wave_params = water_material.wave_params;
//     let amplitude = wave_params.x;
//     let frequency = wave_params.y;
//     let speed = wave_params.z;
//     let steepness = wave_params.w;
    
//     // Apply steepness to create sharper wave peaks
//     // Higher steepness = more pointed/sharp waves
//     // Lower steepness = more rounded/smooth waves
    
//     let base_freq = frequency + steepness * 0.5;
//     let wave_sharpness = 1.0 + steepness * 2.0;
    
//     // Multiple wave layers with steepness applied
//     let wave1 = pow(abs(sin((pos.x * base_freq + time * speed) * 2.0)), wave_sharpness) * 
//                 sign(sin((pos.x * base_freq + time * speed) * 2.0)) * amplitude;
    
//     let wave2 = pow(abs(sin((pos.y * base_freq * 0.8 + time * speed * 0.7) * 2.0)), wave_sharpness) * 
//                 sign(sin((pos.y * base_freq * 0.8 + time * speed * 0.7) * 2.0)) * amplitude * 0.7;
    
//     let wave3 = pow(abs(sin(((pos.x + pos.y) * base_freq * 1.2 + time * speed * 1.1) * 2.0)), wave_sharpness) * 
//                 sign(sin(((pos.x + pos.y) * base_freq * 1.2 + time * speed * 1.1) * 2.0)) * amplitude * 0.5;
    
//     // Add some noise for more organic movement (also affected by steepness)
//     let noise_pos = pos * (0.1 + steepness * 0.05) + time * 0.1;
//     let noise_wave = simplex2d(noise_pos) * amplitude * 0.3;
    
//     return wave1 + wave2 + wave3 + noise_wave;
// }


@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    
    let time = water_material.misc_params.w;
    let transparency = water_material.misc_params.x;
    let foam_intensity = water_material.misc_params.y;
    let foam_cutoff = water_material.misc_params.z;
    
    // Calculate wave height for foam and transparency effects
    let wave_height = calculate_wave_height(in.world_position.xz, time);
    
    // Crystal clear water properties
    let base_color = vec3<f32>(0.0, 0.4, 0.8); // Light blue tint
    let deep_color = vec3<f32>(0.0, 0.2, 0.6); // Deeper blue for depth
    
    // Make water more transparent and less colored
    let water_transparency = mix(0.95, 0.85, abs(wave_height) * 2.0); // Very transparent
    let water_color = mix(base_color, deep_color, 0.1); // Very subtle color
    
    // Calculate foam
    let foam_factor = smoothstep(foam_cutoff - 0.1, foam_cutoff + 0.1, abs(wave_height));
    let foam_color = vec3<f32>(1.0, 1.0, 1.0);
    
    // Mix water and foam
    let final_color = mix(water_color, foam_color, foam_factor * foam_intensity);
    let final_alpha = mix(water_transparency, 1.0, foam_factor * foam_intensity);
    
    // Crystal clear water material properties
    pbr_input.material.base_color = vec4<f32>(final_color, final_alpha);
    pbr_input.material.perceptual_roughness = 0.02; // Very smooth/reflective
    pbr_input.material.metallic = 0.0; // Water is not metallic
    
    // Remove the problematic reflectance line
    // pbr_input.material.reflectance = 0.9; // This line causes the error
    
    // Instead, we'll handle reflectance through Fresnel effect
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    let fresnel_factor = fresnel_water(view_dir, in.world_normal);
    
    // Apply Fresnel effect to make water more reflective
    let reflection_color = vec3<f32>(0.8, 0.9, 1.0);
    let color_with_fresnel = mix(final_color, reflection_color, fresnel_factor * 0.7);
    
    // Update the base color with Fresnel effect
    pbr_input.material.base_color = vec4<f32>(color_with_fresnel, final_alpha);
    
    // No emissive for crystal clear water
    pbr_input.material.emissive = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    
    let final_result = apply_pbr_lighting(pbr_input);
    
    var out: FragmentOutput;
    out.color = final_result;
    return out;
}
