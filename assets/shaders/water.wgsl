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
    data: vec4<f32>,
};

// Bind our custom material data to group 2.
@group(2) @binding(100)
var<uniform> water_material: WaterMaterial;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var displaced_position = vertex.position;

    // Displace using `time` from our `water_material` uniform.
    displaced_position.y += sin(vertex.position.x * 0.2 + water_material.data.w * 2.0) * 0.5;

    // --- The Corrected Approach ---
    // Use the `mesh_functions` helpers with the correct signatures as defined in the source you provided.
    
    // Get the world matrix using the helper function.
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    
    // Use the retrieved matrix to calculate the world position.
    let world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(displaced_position, 1.0));
    
    // Populate the VertexOutput struct according to the official definition.
    // The `position_world_to_clip` function uses the imported `view` uniform implicitly.
    out.position = position_world_to_clip(world_position.xyz);
    out.world_position = world_position;
    
    // Use the correct function signature for normal transformation.
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    
    // Use the correct function for tangent transformation.
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex.instance_index);

    out.uv = vertex.uv;
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

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool
    ) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    
    // Set color from our `water_material` uniform, using the .xyz components.
    pbr_input.material.base_color = vec4<f32>(water_material.data.xyz, pbr_input.material.base_color.a);
    
    let final_color = apply_pbr_lighting(pbr_input);
    
    var out: FragmentOutput;
    out.color = final_color;
    return out;
}