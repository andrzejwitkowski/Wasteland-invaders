// Correct imports for the helper functions and bindings
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::apply_pbr_lighting

// This struct must exactly match the `WaterMaterial` struct in your Rust code.
struct WaterMaterial {
    // .x = wave_amplitude, .y = wave_frequency, .z = wave_speed, .w = wave_steepness
    wave_params: vec4<f32>,
    
    // .x = foam_intensity, .y = foam_cutoff, .z = transparency, .w = time
    misc_params: vec4<f32>,
}

// Bind the material data to a high binding number to avoid conflicts.
// The bindings (100, 101) must match the #[uniform(...)] attributes in Rust.
@group(2) @binding(100) var<uniform> wave_params_uniform: vec4<f32>;
@group(2) @binding(101) var<uniform> misc_params_uniform: vec4<f32>;

// Create a single material struct for easier access in functions.
fn get_material() -> WaterMaterial {
    var material: WaterMaterial;
    material.wave_params = wave_params_uniform;
    material.misc_params = misc_params_uniform;
    return material;
}


// Wave displacement function
fn get_wave(pos: vec2<f32>, time: f32) -> vec4<f32> {
    let material = get_material();
    var p = vec3(pos.x, 0.0, pos.y);

    let wave_count = 2;
    let directions = array<vec2<f32>, 2>(vec2<f32>(1.0, 0.5), vec2<f32>(-0.5, 0.8));
    
    let wave_amplitude = material.wave_params.x;
    let wave_frequency = material.wave_params.y;
    let wave_speed = material.wave_params.z;
    let wave_steepness = material.wave_params.w;

    for (var i = 0; i < wave_count; i = i + 1) {
        let dir = normalize(directions[i]);
        let k = wave_frequency * (1.0 + f32(i) * 0.5);
        let c = wave_speed * (1.0 - f32(i) * 0.2);
        let a = wave_amplitude * (1.0 - f32(i) * 0.6);
        let s = wave_steepness;
        
        let f = k * (dot(dir, pos) - c * time);
        
        p.x += s * a * dir.x * cos(f);
        p.z += s * a * dir.y * cos(f);
        p.y += a * sin(f);
    }
    
    return vec4(p, 0.0);
}


@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let material = get_material();

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let initial_world_pos = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
    
    let time = material.misc_params.w;
    let wave = get_wave(initial_world_pos.xz, time);
    let displaced_world_pos = vec4(wave.xyz, 1.0);
    
    out.position = position_world_to_clip(displaced_world_pos.xyz);
    out.world_position = displaced_world_pos;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    out.instance_index = vertex.instance_index;

    // Pack the foam factor into the .w component of world_position
    let foam_cutoff = material.misc_params.y;
    let wave_amplitude = material.wave_params.x;
    let foam_factor = smoothstep(foam_cutoff, 1.0, wave.y / wave_amplitude);
    out.world_position.w = foam_factor;

    // Guard optional attributes
    #ifdef VERTEX_UVS_B
        out.uv_b = vertex.uv_b;
    #endif
    #ifdef VERTEX_TANGENTS
        out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex.instance_index);
    #endif
    #ifdef VERTEX_COLORS
        out.color = vertex.color;
    #endif

    return out;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let material = get_material();

    // Unpack foam data from the world_position.w component.
    let foam_factor = in.world_position.w;
    let foam_intensity = material.misc_params.x;
    let foam_color = vec4(1.0, 1.0, 0.95, 1.0);
    
    // Mix the foam color over the base water color that came from the StandardMaterial.
    pbr_input.material.base_color = mix(
        pbr_input.material.base_color,
        foam_color,
        foam_factor * foam_intensity
    );

    // Calculate fresnel for transparency using our custom transparency value
    let transparency = material.misc_params.z;
    let fresnel = pow(1.0 - max(dot(pbr_input.N, pbr_input.V), 0.0), 3.0);
    pbr_input.material.base_color.a = mix(transparency, 1.0, fresnel);
    
    // Let Bevy's PBR system apply lighting to our modified material
    let final_color = apply_pbr_lighting(pbr_input);
    
    var out: FragmentOutput;
    out.color = final_color;
    
    return out;
}