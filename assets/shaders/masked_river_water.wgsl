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

// Accessors
fn get_wave_amp() -> f32 { return water_material.wave_params.x; }
fn get_wave_freq() -> f32 { return water_material.wave_params.y; }
fn get_wave_speed() -> f32 { return water_material.wave_params.z; }
fn get_wave_steepness() -> f32 { return water_material.wave_params.w; }

fn get_transparency() -> f32 { return water_material.misc_params.x; }
fn get_foam_intensity() -> f32 { return water_material.misc_params.y; }
fn get_foam_cutoff() -> f32 { return water_material.misc_params.z; }
fn get_time() -> f32 { return water_material.misc_params.w; }

fn get_river_width() -> f32 { return water_material.river_params.x; }
fn get_meander_freq() -> f32 { return water_material.river_params.z; }
fn get_meander_amp() -> f32 { return water_material.river_params.w; }

fn get_river_start() -> vec2<f32> { return water_material.river_position.xy; }
fn get_river_dir_raw() -> vec2<f32> { return water_material.river_position.zw; }

fn bank_fill_ratio() -> f32 { return clamp(water_material.debug_options.z, 0.0, 1.0); }
fn bank_slope_distance() -> f32 { return water_material.debug_options.y; }

// Return (dist, along) so we can reuse along for width noise
fn river_dist_and_along(pos: vec2<f32>) -> vec2<f32> {
    let start = get_river_start();
    let dir = norm2(get_river_dir_raw());
    let rel = pos - start;
    let along = dot(rel, dir);
    let meander = meander_offset(along);
    let perp = vec2(-dir.y, dir.x);
    let center = start + dir * along + perp * meander;
    let dist = length(pos - center);
    return vec2(dist, along);
}

fn width_noise(along: f32) -> f32 {
    // replicate terrain: sample_noise(distance_along_river * 0.0005, 0)
    // We have only noise() in [0,1]; remap like terrain logic (terrain used sample_noise 0..1 then *0.3)
    return noise(vec2(along * 0.0005, 0.0));
}

fn norm2(v: vec2<f32>) -> vec2<f32> {
    let l = sqrt(dot(v,v));
    return select(vec2<f32>(0.0), v / l, l > 0.00001);
}

// Minimal meander (cheap)
fn meander_offset(dist_along: f32) -> f32 {
    let f = get_meander_freq();
    let phase = dist_along * f;
    let primary = sin(phase * 6.28318530718);
    let secondary = sin(phase * 1.7 * 6.28318530718) * 0.4;
    let chaos = sin(phase * 0.37) * 0.3;
    let asym = sin(phase * 0.8 + 1.57) * 0.2;
    let base = primary * 0.7 + secondary * 0.3;
    let total = (base + chaos * 0.6 * 0.5 + asym) * (1.0 + sin(phase * 0.3) * 0.2);
    return total * get_meander_amp();
}

fn river_distance(pos: vec2<f32>) -> f32 {
    let start = get_river_start();
    let dir = norm2(get_river_dir_raw());
    let rel = pos - start;
    let along = dot(rel, dir);
    let perp = vec2(-dir.y, dir.x);
    let center = start + dir * along + perp * meander_offset(along);
    return length(pos - center);
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

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var f = 1.0;
    for (var i = 0; i < 4; i = i + 1) {
        v += a * (noise(p * f) * 2.0 - 1.0);
        f *= 2.0;
        a *= 0.5;
    }
    return v;
}

fn wave_displacement(pos: vec2<f32>, t: f32) -> f32 {
    let freq = get_wave_freq() * 0.1;
    let base = fbm(pos * freq + vec2<f32>(t * get_wave_speed() * 0.1, t * 0.07));
    return base * get_wave_amp() * 3.0;
}

fn wave_normal(pos: vec2<f32>, t: f32) -> vec3<f32> {
    let e = 0.05;
    let h = wave_displacement(pos, t);
    let hx = wave_displacement(pos + vec2<f32>(e,0.0), t);
    let hz = wave_displacement(pos + vec2<f32>(0.0,e), t);
    let tangent_x = vec3<f32>(e, hx - h, 0.0);
    let tangent_z = vec3<f32>(0.0, hz - h, e);
    return normalize(cross(tangent_z, tangent_x));
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_pos4 = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    let t = get_time();

    var displaced = world_pos4;
    displaced.y += wave_displacement(world_pos4.xz, t);

    out.position = position_world_to_clip(displaced.xyz);
    out.world_position = displaced;
    out.world_normal = wave_normal(world_pos4.xz, t);
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

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let da = river_dist_and_along(in.world_position.xz);
    let dist = da.x;
    let along = da.y;

    // Dynamic width variation
    let w_noise = width_noise(along);
    let actual_river_width = get_river_width() * (1.0 + w_noise * 0.3);

    let core_half = actual_river_width * 0.5;

    // Extend into banks by ratio
    let bank_extra = bank_slope_distance() * bank_fill_ratio();
    let effective_half = core_half + bank_extra;

    // Discard outside extended region
    if (dist > effective_half) {
        discard;
    }

    // Edge alpha fade (last 3 world units)
    let edge_fade = smoothstep(effective_half - 3.0, effective_half, dist);

    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let t = get_time();
    let wave_h = wave_displacement(in.world_position.xz, t);

    // Base color varies slightly with how far into banks you are
    let edge_ratio = clamp(dist / effective_half, 0.0, 1.0);
    let depth_tint = mix(0.15, 0.35, edge_ratio);
    let base_color = vec3<f32>(0.0, depth_tint, 0.65 - 0.25 * (1.0 - edge_ratio));

    let foam_factor = smoothstep(get_foam_cutoff()-0.1, get_foam_cutoff()+0.1, abs(wave_h)) * get_foam_intensity();
    let foam_color = vec3<f32>(1.0,1.0,1.0);
    let color = mix(base_color, foam_color, foam_factor);
    let transparency = get_transparency();

    // Apply edge fade to alpha
    let alpha = mix(transparency, 1.0, foam_factor) * (1.0 - edge_fade);

    pbr_input.material.base_color = vec4<f32>(color, alpha);
    pbr_input.material.perceptual_roughness = 0.03;
    pbr_input.material.metallic = 0.0;

    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    let fres = fresnel(max(0.0, dot(view_dir, in.world_normal)), 0.02);
    let reflection_tint = vec3<f32>(0.9,0.95,1.0);
    let final_col = mix(color, reflection_tint, fres * 0.6);
    pbr_input.material.base_color = vec4<f32>(final_col, pbr_input.material.base_color.a);

    let lit = apply_pbr_lighting(pbr_input);
    var out: FragmentOutput;
    out.color = lit;
    return out;
}