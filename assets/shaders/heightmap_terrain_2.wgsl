// path: assets/shaders/heightmap_terrain.wgsl

#import bevy_pbr::pbr_prelude
#import bevy_pbr::forward_io::{Vertex, VertexOutput, FragmentOutput}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::apply_pbr_lighting

// Material parameters matching Rust struct
struct HeightmapMaterial {
    terrain_params: vec4<f32>,
    river_params: vec4<f32>,
    erosion_params: vec4<f32>,
    terrain_features: vec4<f32>,
    river_position: vec4<f32>,
    noise_config: vec4<f32>,
    debug_options: vec4<f32>,         
};

// Bind material data to group 2 like your water shader
@group(2) @binding(100)
var<uniform> heightmap_material: HeightmapMaterial;

// Extract parameters for easier access
fn get_terrain_scale() -> f32 { return heightmap_material.terrain_params.x; }
fn get_terrain_amplitude() -> f32 { return heightmap_material.terrain_params.y; }
fn get_river_depth() -> f32 { return heightmap_material.terrain_params.z; }
fn get_seed() -> f32 { return heightmap_material.terrain_params.w; }

fn get_river_width() -> f32 { return heightmap_material.river_params.x; }
fn get_bank_slope_distance() -> f32 { return heightmap_material.river_params.y; }
fn get_meander_frequency() -> f32 { return heightmap_material.river_params.z; }
fn get_meander_amplitude() -> f32 { return heightmap_material.river_params.w; }

fn get_erosion_strength() -> f32 { return heightmap_material.erosion_params.x; }
fn get_erosion_radius() -> f32 { return heightmap_material.erosion_params.y; }
fn get_valley_flattening() -> f32 { return heightmap_material.erosion_params.z; }
fn get_erosion_smoothing() -> f32 { return heightmap_material.erosion_params.w; }

fn get_flat_area_radius() -> f32 { return heightmap_material.terrain_features.x; }
fn get_flat_area_strength() -> f32 { return heightmap_material.terrain_features.y; }
fn get_hill_steepness() -> f32 { return heightmap_material.terrain_features.z; }
fn get_terrain_roughness() -> f32 { return heightmap_material.terrain_features.w; }

fn get_river_start() -> vec2<f32> { return heightmap_material.river_position.xy; }
fn get_river_dir() -> vec2<f32> { return heightmap_material.river_position.zw; }

fn get_noise_octaves() -> i32 { return i32(heightmap_material.noise_config.x); }
fn get_noise_lacunarity() -> f32 { return heightmap_material.noise_config.y; }
fn get_noise_persistence() -> f32 { return heightmap_material.noise_config.z; }
fn get_noise_seed() -> f32 { return heightmap_material.noise_config.w; }

fn get_show_mask() -> f32 { return heightmap_material.debug_options.x; }
fn get_mask_mode() -> f32 { return heightmap_material.debug_options.z; }

fn get_cell_step() -> f32 {
    let s = heightmap_material.debug_options.y;
    return select(2.0, s, s > 0.0001);
}

// Helper to test if a world XZ position is a river (same criterion used in fragment)
fn is_river_at(pos_xz: vec2<f32>) -> bool {
    let h = generate_height(pos_xz);
    return h < -get_river_depth() * 0.5;
}

// Add this rotation matrix function
fn rotate2d(angle: f32) -> mat2x2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return mat2x2<f32>(c, -s, s, c);
}

fn sample_fbm_rotated(coord: vec2<f32>, octaves: i32, lacunarity: f32, persistence: f32) -> f32 {
    var value = 0.0;
    var amplitude = 1.0;
    var frequency = 1.0;
    var max_value = 0.0; // For normalization
    
    let base_rotation = 0.52359877559;
    
    for (var i = 0; i < octaves; i = i + 1) {
        let rotation_angle = base_rotation * f32(i);
        let rotation_matrix = rotate2d(rotation_angle);
        let rotated_coord = rotation_matrix * (coord * frequency);
        
        value += amplitude * (sample_noise(rotated_coord) - 0.5); // Center around 0
        max_value += amplitude;
        
        amplitude *= persistence;
        frequency *= lacunarity;
    }
    
    return value / max_value; // Normalize to prevent extreme values
}

// Alternative: More dramatic rotation like in your screenshots
fn sample_fbm_matrix_rotated(coord: vec2<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 1.0;
    var p = coord;
    
    // Predefined rotation matrices for each octave (like M^k in your formula)
    for (var i = 0; i < octaves; i = i + 1) {
        value += amplitude * sample_noise(p);
        amplitude *= 0.5;
        
        // Apply different transformations each octave
        if (i == 0) {
            p = mat2x2<f32>(0.8, -0.6, 0.6, 0.8) * p * 2.0; // ~37° rotation + 2x scale
        } else if (i == 1) {
            p = mat2x2<f32>(0.6, -0.8, 0.8, 0.6) * p * 2.0; // ~53° rotation + 2x scale
        } else if (i == 2) {
            p = mat2x2<f32>(0.707, -0.707, 0.707, 0.707) * p * 2.0; // 45° rotation + 2x scale
        } else {
            // For remaining octaves, use a pattern
            let angle = 0.61803398875 * f32(i); // Golden angle for nice distribution
            let rot_mat = rotate2d(angle);
            p = rot_mat * p * 2.0;
        }
    }
    
    return value;
}

// Much better noise function - smooth interpolated noise
fn hash22(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    // Four corners of the grid cell
    let a = hash22(i);
    let b = hash22(i + vec2<f32>(1.0, 0.0));
    let c = hash22(i + vec2<f32>(0.0, 1.0));
    let d = hash22(i + vec2<f32>(1.0, 1.0));

    // Smooth interpolation
    let u = f * f * (3.0 - 2.0 * f);
    
    return mix(mix(dot(a, f - vec2<f32>(0.0, 0.0)),
                   dot(b, f - vec2<f32>(1.0, 0.0)), u.x),
               mix(dot(c, f - vec2<f32>(0.0, 1.0)),
                   dot(d, f - vec2<f32>(1.0, 1.0)), u.x), u.y);
}

// Use this instead of sample_noise
fn sample_noise(coord: vec2<f32>) -> f32 {
    return noise(coord) * 0.5 + 0.5; // Convert from [-1,1] to [0,1]
}


fn sample_fbm(coord: vec2<f32>, octaves: i32, lacunarity: f32, persistence: f32) -> f32 {
    var value = 0.0;
    var amplitude = 1.0;
    var frequency = 1.0;
    
    for (var i = 0; i < octaves; i = i + 1) {
        value += amplitude * sample_noise(coord * frequency);
        amplitude *= persistence;
        frequency *= lacunarity;
    }
    
    return value;
}

// Vector math helpers
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

// Main heightmap generation function
fn generate_height(position: vec2<f32>) -> f32 {
    // Generate base terrain
    let base_terrain = sample_enhanced_terrain_height(position);
    
    // Calculate river effects
    let river_effects = calculate_river_effects(position);
    
    // Apply erosion
    let final_height = apply_erosion_effects(base_terrain, position, river_effects.erosion_factor);
    
    return final_height + river_effects.river_modification;
}

struct RiverEffects {
    river_modification: f32,
    erosion_factor: f32,
}

fn calculate_river_effects(position: vec2<f32>) -> RiverEffects {
    let river_start = get_river_start();
    let river_dir = get_river_dir();
    let base_river_dir = vec2_normalize(river_dir);
    
    let relative_pos = position - river_start;
    let distance_along_river = vec2_dot(relative_pos, base_river_dir);
    
    // Generate meander offset
    let meander_offset = calculate_realistic_meander(distance_along_river);
    
    // Calculate river center with meandering
    let perpendicular = vec2(-base_river_dir.y, base_river_dir.x);
    let river_center = river_start + base_river_dir * distance_along_river + perpendicular * meander_offset;
    
    // Distance from point to river centerline
    let distance_to_river = vec2_distance(position, river_center);
    
    // Calculate variable river width
    let width_noise = sample_noise(vec2(distance_along_river * 0.0005, 0.0));
    let actual_river_width = get_river_width() * (1.0 + width_noise * 0.3);
    
    // Calculate river profile (carving)
    let river_carving = calculate_river_profile(distance_to_river, actual_river_width);
    
    // Calculate erosion factor
    let erosion_factor = calculate_erosion_factor(distance_to_river, actual_river_width);
    
    return RiverEffects(river_carving, erosion_factor);
}

fn calculate_realistic_meander(distance_along_river: f32) -> f32 {
    let meander_frequency = get_meander_frequency();
    let meander_phase = distance_along_river * meander_frequency;
    
    // Primary meandering - base sine wave
    let primary_meander = sin(meander_phase * 6.28318530718);
    
    // Secondary meandering
    let secondary_phase = distance_along_river * meander_frequency * 1.7;
    let secondary_meander = sin(secondary_phase * 6.28318530718) * 0.4;
    
    // Chaotic variations
    let chaos_variation = sample_fbm(
        vec2(distance_along_river * 0.001, 0.0), 
        get_noise_octaves(), 
        get_noise_lacunarity(),
        get_noise_persistence());
    
    // Scale variation
    let scale_variation = sample_noise(vec2(distance_along_river * 0.0003, 0.0));
    let scale_factor = 1.0 + scale_variation * 0.4; // Hardcoded meander_scale_variation
    
    // Asymmetric variations
    let asymmetry = sample_noise(vec2(meander_phase * 0.8, 1000.0));
    
    // Combine components
    let base_meander = primary_meander * 0.7 + secondary_meander * 0.3;
    let chaotic_component = chaos_variation * 0.6 * 0.5; // Hardcoded meander_chaos
    let asymmetric_component = asymmetry * 0.2;
    
    let total_meander = (base_meander + chaotic_component + asymmetric_component) * scale_factor;
    
    return total_meander * get_meander_amplitude();
}

fn calculate_river_profile(distance_to_river: f32, river_width: f32) -> f32 {
    let water_edge = river_width * 0.5;
    let bank_end = water_edge + get_bank_slope_distance();
    
    if (distance_to_river <= water_edge) {
        // River bed - flat bottom
        return -get_river_depth();
    } else if (distance_to_river <= bank_end) {
        // River banks with smooth transition
        let bank_progress = (distance_to_river - water_edge) / get_bank_slope_distance();
        
        // Ultra-smooth transition using combined smoothing functions
        let smooth1 = 1.0 - pow(bank_progress, 3.0);
        let smooth2 = sin((1.0 - bank_progress) * 1.57079632679);
        let smooth3 = (1.0 + cos(bank_progress * 3.14159265359)) * 0.5;
        
        // Combine smoothing functions
        let combined_smooth = smooth1 * 0.5 + smooth2 * 0.3 + smooth3 * 0.2;
        return -get_river_depth() * combined_smooth;
    } else {
        // No river influence
        return 0.0;
    }
}

fn calculate_erosion_factor(distance_to_river: f32, river_width: f32) -> f32 {
    let water_edge = river_width * 0.5;
    let erosion_end = water_edge + get_erosion_radius();
    
    if (distance_to_river <= water_edge) {
        // Maximum erosion in river channel
        return get_erosion_strength();
    } else if (distance_to_river <= erosion_end) {
        // Gradual erosion falloff
        let erosion_progress = (distance_to_river - water_edge) / get_erosion_radius();
        let falloff = pow(1.0 - erosion_progress, 2.0);
        return get_erosion_strength() * falloff;
    } else {
        // No erosion
        return 0.0;
    }
}

fn apply_erosion_effects(base_height: f32, position: vec2<f32>, erosion_factor: f32) -> f32 {
    if (erosion_factor <= 0.0) {
        return base_height;
    }
    
    // Calculate target elevation for valley floor
    let valley_target_height = calculate_valley_floor_height(position);
    
    // Smooth the terrain towards valley floor
    let flattened_height = base_height * (1.0 - get_valley_flattening() * erosion_factor) + 
                          valley_target_height * get_valley_flattening() * erosion_factor;
    
    // Apply smoothing by reducing high-frequency terrain variations
    return apply_terrain_smoothing(flattened_height, position, erosion_factor);
}

fn apply_terrain_smoothing(height: f32, position: vec2<f32>, erosion_factor: f32) -> f32 {
    let smoothing_strength = get_erosion_smoothing() * erosion_factor;
    
    if (smoothing_strength <= 0.0) {
        return height;
    }
    
    // Sample nearby points for averaging
    let sample_radius = 2.0;
    var height_sum = height;
    var sample_count = 1.0;
    
    // Sample in a small circle around the point
    for (var i = 0; i < 4; i = i + 1) {
        let angle = (f32(i) / 4.0) * 6.28318530718;
        let sample_pos = position + vec2(cos(angle), sin(angle)) * sample_radius;
        
        let sample_height = sample_terrain_height(sample_pos);
        height_sum += sample_height;
        sample_count += 1.0;
    }
    
    let averaged_height = height_sum / sample_count;
    
    // Blend between original and smoothed height
    return height * (1.0 - smoothing_strength) + averaged_height * smoothing_strength;
}

fn sample_enhanced_terrain_height(position: vec2<f32>) -> f32 {
    // Base terrain with rotation - 6-8 octaves for detail
    var base = sample_fbm_rotated(position * get_terrain_scale(), 6, 2.0, 0.5);
    base = pow(abs(base), get_hill_steepness()) * sign(base);

    // Additional hill noise with different rotation pattern
    let hill_detail = sample_fbm_rotated(position * get_terrain_scale() * 2.0, 6, 2.2, 0.6) * 0.3 * get_terrain_roughness();

    // Detail layer - fewer octaves for performance
    let detail = sample_fbm_rotated(position * 0.05, 4, 2.0, 0.5) * 0.1 * get_terrain_roughness();

    // Apply flat area masking
    let flat_mask = calculate_flat_area_mask(position);
    let enhanced_terrain = (base + hill_detail + detail) * get_terrain_amplitude();

    return enhanced_terrain * (1.0 - flat_mask) + (enhanced_terrain * 0.3) * flat_mask;
}

fn sample_terrain_height(position: vec2<f32>) -> f32 {
    let base = sample_fbm_rotated(position * get_terrain_scale(), 6, 2.0, 0.5);
    let detail = sample_noise(position * 0.05) * 0.1;
    return (base + detail) * get_terrain_amplitude();
}

fn calculate_valley_floor_height(position: vec2<f32>) -> f32 {
    // Use rotated FBM for more natural valley floors
    let valley_base = sample_fbm_rotated(position * get_terrain_scale() * 0.3, 5, 2.0, 0.5);
    
    let river_start = get_river_start();
    let river_dir = vec2_normalize(get_river_dir());
    let relative_pos = position - river_start;
    let distance_along_river = vec2_dot(relative_pos, river_dir);
    let river_slope = distance_along_river * 0.001;

    return (valley_base * get_terrain_amplitude() * 0.3) + river_slope;
}

fn calculate_flat_area_mask(position: vec2<f32>) -> f32 {
    // Generate flat area centers using noise
    let flat_center_value = sample_noise(position * 0.002); // Hardcoded flat_area_frequency
    
    // Threshold to determine if this is a flat area center
    if (flat_center_value > 0.6) {
        // Sample nearby points to create smooth circular flat areas
        var total_flatness = 0.0;
        let sample_count = 8;
        
        for (var i = 0; i < sample_count; i = i + 1) {
            let angle = (f32(i) / f32(sample_count)) * 6.28318530718;
            let sample_pos = position + vec2(cos(angle), sin(angle)) * get_flat_area_radius() * 0.5;
            
            let sample_value = sample_noise(sample_pos * 0.002); // Hardcoded flat_area_frequency
            total_flatness += sample_value;
        }
        
        let avg_flatness = total_flatness / f32(sample_count);
    
        // Create smooth falloff from center to edge
        let distance_factor = 1.0 - (flat_center_value - 0.6) / 0.4;
        let flat_strength = avg_flatness * distance_factor * get_flat_area_strength();
        
        return clamp(flat_strength, 0.0, 1.0);
    } else {
        return 0.0;
    }
}

fn calculate_terrain_normal(position: vec2<f32>) -> vec3<f32> {
    // Calculate normal by sampling nearby heights
    let eps = 0.1;
    
    let center_height = generate_height(position);
    let right_height = generate_height(position + vec2(eps, 0.0));
    let forward_height = generate_height(position + vec2(0.0, eps));
    
    let tangent_x = vec3(eps, right_height - center_height, 0.0);
    let tangent_z = vec3(0.0, forward_height - center_height, eps);
    
    return normalize(cross(tangent_z, tangent_x));
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    // Get world matrix
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    
    // Get initial world position
    let initial_world_pos = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    
    // Generate terrain height at this position
    let terrain_height = generate_height(initial_world_pos.xz);
    
    // Apply height displacement
    var displaced_world_pos = initial_world_pos;
    displaced_world_pos.y = terrain_height;
    
    // Calculate terrain normal
    let normal = calculate_terrain_normal(initial_world_pos.xz);
    
    // Populate VertexOutput
    out.position = position_world_to_clip(displaced_world_pos.xyz);
    out.world_position = displaced_world_pos;
    out.world_normal = normal;
    out.uv = vertex.uv;
    
    // Use the correct function for tangent transformation
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

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    
    // Calculate terrain-based material properties
    let height = in.world_position.y;
    let slope = 1.0 - dot(in.world_normal, vec3<f32>(0.0, 1.0, 0.0));
    
    // Determine terrain type based on height and slope
    let is_water = height < 0.0;
    let is_river = height < -get_river_depth() * 0.5;
    let is_mountain = height > get_terrain_amplitude() * 0.7;
    let is_flat = slope < 0.1;

    var any_neighbor_river = false;
    if (!is_river) {
        let step = get_cell_step();
        // Offsets for 8 neighbors
        for (var dx = -1; dx <= 1; dx = dx + 1) {
            for (var dz = -1; dz <= 1; dz = dz + 1) {
                if (dx == 0 && dz == 0) { continue; }
                let neighbor_pos = vec2(
                    in.world_position.x + f32(dx) * step,
                    in.world_position.z + f32(dz) * step
                );
                if (is_river_at(neighbor_pos)) {
                    any_neighbor_river = true;
                }
            }
        }
    }
    let is_river_margin = (!is_river) && any_neighbor_river;

    // Water mask (core river OR water)
    let water_mask = select(0.0, 1.0, (is_water || is_river));

    // After computing is_river, is_river_margin, etc.
    let mask_value = select(0.0, 1.0, is_river) +
                     select(0.0, 0.5, (is_river_margin && !is_river));

    // If explicit mask_mode (offscreen pass)
    if (get_mask_mode() > 0.5) {
        var out_mask: FragmentOutput;
        out_mask.color = vec4<f32>(mask_value, mask_value, mask_value, 1.0);
        return out_mask;
    }

    // Debug visualization:
    // Red   = river core
    // Yellow= river margin
    // Green = other terrain
    if (get_show_mask() > 0.5) {
        var out_debug: FragmentOutput;
        if (is_river) {
            out_debug.color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
        } else if (is_river_margin) {
            out_debug.color = vec4<f32>(1.0, 1.0, 0.0, 1.0);
        } else {
            out_debug.color = vec4<f32>(0.0, 1.0, 0.0, 1.0);
        }
        return out_debug;
    }
    
    // Terrain color blending
    var base_color: vec3<f32>;
    
    if (is_river) {
        // Deep river - dark blue
        base_color = vec3<f32>(0.1, 0.2, 0.4);
    } 
    else if (is_water) {
        // Shallow water - light blue
        base_color = vec3<f32>(0.3, 0.5, 0.7);
    } else if (is_mountain) {
        // Mountain - rocky gray
        base_color = vec3<f32>(0.5, 0.5, 0.5);
    } else if (is_flat) {
        // Flat land - green grass
        base_color = vec3<f32>(0.4, 0.6, 0.3);
    } 
    else {
        // Slope - brown dirt
        base_color = vec3<f32>(0.6, 0.5, 0.4);
    }
    
    // Adjust material properties based on terrain type
    pbr_input.material.base_color = vec4<f32>(base_color, 1.0);
    
    if (is_water || is_river) {
        pbr_input.material.perceptual_roughness = 0.1;
        pbr_input.material.metallic = 0.8;
    } else if (is_mountain) {
        pbr_input.material.perceptual_roughness = 0.9;
        pbr_input.material.metallic = 0.3;
    } else {
        pbr_input.material.perceptual_roughness = 0.7;
        pbr_input.material.metallic = 0.1;
    }
    
    let final_result = apply_pbr_lighting(pbr_input);
    
    var out: FragmentOutput;
    out.color = final_result;
    return out;
}