#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings::{view, globals},
    forward_io::{VertexOutput, FragmentOutput},
    mesh_bindings::mesh,
}

struct WaterVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) water_data: vec4<f32>,  // Our custom field
}

@vertex
fn vertex(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32>) -> WaterVertexOutput {
    var out: WaterVertexOutput;
    out.position = vec4<f32>(position, 1.0);
    out.world_position = vec4<f32>(position, 1.0);
    out.world_normal = normal;
    out.uv = uv;
    out.water_data = vec4<f32>(0.5, 0.5, 0.0, 1.0); // Some test water data
    return out;
}

@fragment
fn fragment(in: WaterVertexOutput) -> @location(0) vec4<f32> {
    // Use the water data for some effect
    return vec4<f32>(in.uv.x, in.uv.y, in.water_data.x, 1.0);
}
