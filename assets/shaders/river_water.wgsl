// Optionally import PBR helpers if you use advanced lighting/etc
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings

struct RiverMaterial {
    color_and_time: vec4<f32>,
};
@group(2) @binding(0) var<uniform> material: RiverMaterial;

struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let color = material.color_and_time.rgb;
    let time = material.color_and_time.a;

    let wave = sin(in.uv.x * 10.0 + time * 2.0) * 0.5 + 0.5;
    return vec4<f32>(color, wave);
}
