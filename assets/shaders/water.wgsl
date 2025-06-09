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

// Wave displacement function using the parameters from ComplexWaterMaterial
fn get_wave(pos: vec2<f32>, time: f32) -> vec4<f32> {
    var p = vec3(pos.x, 0.0, pos.y);

    let wave_count = 3;
    let directions = array<vec2<f32>, 3>(
        vec2<f32>(1.0, 0.5), 
        vec2<f32>(-0.5, 0.8),
        vec2<f32>(0.8, -0.3)
    );

    // // Option 2: Wind-driven (more realistic)
    // let directions = array<vec2<f32>, 3>(
    //     vec2<f32>(1.0, 0.0),           // Main wind direction
    //     vec2<f32>(0.707, 0.707),       // 45° off main
    //     vec2<f32>(0.707, -0.707)       // -45° off main  
    // );
    
    // Extract wave parameters from the uniform
    let wave_amplitude = water_material.wave_params.x;
    let wave_frequency = water_material.wave_params.y;
    let wave_speed = water_material.wave_params.z;
    let wave_steepness = water_material.wave_params.w;

    for (var i = 0; i < wave_count; i = i + 1) {
        let dir = normalize(directions[i]);
        let k = wave_frequency * (0.5 + f32(i) * 0.3);
        let c = wave_speed * (0.8 + f32(i) * 0.4);
        let a = wave_amplitude * (0.6 + f32(i) * 0.2);
        let s = wave_steepness;
        
        let f = k * (dot(dir, pos) - c * time);
        
        p.x += s * a * dir.x * cos(f);
        p.z += s * a * dir.y * cos(f);
        p.y += a * sin(f);
    }
    
    return vec4(p, 0.0);
}

// Calculate wave normal for better lighting
fn get_wave_normal(pos: vec2<f32>, time: f32) -> vec3<f32> {
    let eps = 0.1;
    let wave_center = get_wave(pos, time);
    let wave_right = get_wave(pos + vec2<f32>(eps, 0.0), time);
    let wave_forward = get_wave(pos + vec2<f32>(0.0, eps), time);
    
    let tangent_x = normalize(wave_right.xyz - wave_center.xyz);
    let tangent_z = normalize(wave_forward.xyz - wave_center.xyz);
    
    return normalize(cross(tangent_x, tangent_z));
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var displaced_position = vertex.position;

    // Get the world matrix using the helper function.
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    
    // Get initial world position
    let initial_world_pos = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));

    let time = water_material.misc_params.w;
    let wave = get_wave(initial_world_pos.xz, time);
    let displaced_world_pos = vec4(wave.xyz, 1.0);

    // Calculate wave normal for better lighting
    let wave_normal = get_wave_normal(initial_world_pos.xz, time);
    
    // Populate the VertexOutput struct according to the official definition.
    // The `position_world_to_clip` function uses the imported `view` uniform implicitly.
    out.position = position_world_to_clip(displaced_world_pos.xyz);
    out.world_position = displaced_world_pos;
    
    // Use the correct function signature for normal transformation.
    out.world_normal = wave_normal;
    //mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);

    out.uv = vertex.uv;
    
    // Use the correct function for tangent transformation.
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex.instance_index);
#endif

    
        // The 'color' field on VertexOutput is also guarded by an #ifdef.
    // We only assign to it if the mesh has vertex colors.
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

// Add caustics function
fn get_caustics(world_pos: vec3<f32>, time: f32) -> f32 {
    let uv = world_pos.xz * 0.1;
    let caustic_time = time * 0.5;
    
    // Multiple layers of caustics
    let c1 = sin((uv.x + uv.y) * 6.0 + caustic_time) * 0.5 + 0.5;
    let c2 = sin((uv.x - uv.y) * 8.0 + caustic_time * 1.3) * 0.5 + 0.5;
    let c3 = sin((uv.x * 1.5 + uv.y * 0.8) * 4.0 + caustic_time * 0.7) * 0.5 + 0.5;
    
    return (c1 * c2 * c3) * 2.0;
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

fn get_foam_factor(world_pos: vec3<f32>, wave_height: f32, time: f32) -> f32 {
    // Foam based on wave steepness
    let foam_from_waves = smoothstep(0.5, 1.2, wave_height);
    
    // Animated foam texture
    let foam_noise = sin(world_pos.x * 10.0 + time * 2.0) * sin(world_pos.z * 12.0 + time * 1.5);
    let foam_texture = smoothstep(-0.2, 0.8, foam_noise) * 0.3;
    
    return min(1.0, foam_from_waves + foam_texture);
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool
    ) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    
    let time = water_material.misc_params.w;
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    // Calculate Fresnel effect
    let fresnel_factor = fresnel_water(view_dir, in.world_normal);
    
    // Add some color variation based on wave height
    let wave_height = in.world_position.y;
    //let wave_color_variation = sin(wave_height * 5.0) * 0.1;
    let water_depth = max(0.0, wave_height - (-2.0)); // Assuming bottom at y=-2
    let depth_color = get_water_depth_color(in.world_position.xyz, wave_height);

    // Get caustics effect
    let caustics = get_caustics(in.world_position.xyz, time);
    
    // Get foam
    let foam = get_foam_factor(in.world_position.xyz, wave_height, time);
    let foam_color = vec3<f32>(0.9, 0.95, 1.0);
    
    // // Create different color zones
    // let base_water_color = vec3<f32>(0.1, 0.4, 0.7);
    // let foam_color = vec3<f32>(0.8, 0.9, 1.0);
    // Combine all effects
    let base_water_color = depth_color + caustics * 0.3;
    let final_color = mix(base_water_color, foam_color, foam);

    // let deep_color = vec3<f32>(0.05, 0.2, 0.4);
    // let reflection_color = vec3<f32>(0.6, 0.8, 1.0); // Sky-like reflection color
    // Apply Fresnel for reflections
    let reflection_color = vec3<f32>(0.6, 0.8, 1.0);
    let color_with_fresnel = mix(final_color, reflection_color, fresnel_factor * 0.6);
    
    // // Mix colors based on wave height
    // let color_mix = smoothstep(-1.0, 2.0, wave_height);
    // let depth_mix = smoothstep(-0.5, 0.5, wave_height);
    
    // let water_color = mix(
    //     mix(deep_color, base_water_color, depth_mix),
    //     foam_color, 
    //     color_mix * 0.4
    // );
    
    // // Apply Fresnel effect - more reflective at grazing angles
    // let final_color = mix(
    //     water_color + wave_color_variation,
    //     reflection_color,
    //     fresnel_factor * 0.8 // Control reflection strength
    // );
    
    pbr_input.material.base_color = vec4<f32>(color_with_fresnel, 1.0);

    // Dynamic transparency based on depth and Fresnel
    let water_alpha = get_water_alpha(water_depth, fresnel_factor);
    pbr_input.material.base_color.a = water_alpha;
    
    // Adjust material properties
    pbr_input.material.perceptual_roughness = mix(0.1, 0.05, fresnel_factor);
    pbr_input.material.metallic = 0.0;
    
    // // Adjust transparency based on Fresnel - more transparent when looking straight down
    // let base_transparency = water_material.misc_params.z;
    // let fresnel_transparency = mix(base_transparency, base_transparency * 0.3, fresnel_factor);
    // pbr_input.material.base_color.a = fresnel_transparency;
    
    // // Adjust roughness based on Fresnel for more realistic reflections
    // pbr_input.material.perceptual_roughness = mix(0.1, 0.05, fresnel_factor);
    
    let final_pbr_color = apply_pbr_lighting(pbr_input);
    
    var out: FragmentOutput;
    out.color = final_pbr_color;
    return out;
}

// @fragment
// fn fragment(
//     in: VertexOutput,
//     @builtin(front_facing) is_front: bool
//     ) -> FragmentOutput {
//     var pbr_input = pbr_input_from_standard_material(in, is_front);
    
//     // Set color from our `water_material` uniform, using the .xyz components.
//     // pbr_input.material.base_color = vec4<f32>(water_material.misc_params.xyz, pbr_input.material.base_color.a);
//         // Add some color variation based on wave height
//     let wave_height = in.world_position.y;
//     let wave_color_variation = sin(wave_height * 5.0) * 0.1;
    
//     // Create a more dynamic water color
//     let base_water_color = vec3<f32>(0.1, 0.4, 0.7);
//     let foam_color = vec3<f32>(0.8, 0.9, 1.0);
    
//     // Mix colors based on wave height
//     let color_mix = smoothstep(-0.5, 1.0, wave_height);
//     let final_water_color = mix(base_water_color, foam_color, color_mix * 0.3);
    
//     pbr_input.material.base_color = vec4<f32>(final_water_color + wave_color_variation, 0.8);
    
//     // Set transparency from misc_params.z
//     let transparency = water_material.misc_params.z;
//     pbr_input.material.base_color.a = transparency;
    
//     let final_color = apply_pbr_lighting(pbr_input);
    
//     var out: FragmentOutput;
//     out.color = final_color;
//     return out;
// }