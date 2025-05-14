use bevy::{asset::RenderAssetUsages, prelude::*, render::mesh::{Indices, PrimitiveTopology}};

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

#[derive(Component, Clone, Debug)]
pub struct Terrain {
    pub seed: u32,
    pub size: UVec2,
    pub plane_size: Vec2,
    pub height_scale: f32,
    pub frequency: f64,
    pub lacunarity: f32,
    pub octaves: usize,
    pub persistence: f32,
    pub material: Handle<StandardMaterial>,
}

impl Default for Terrain {
    fn default() -> Self {
        Self {
            seed: 0,
            size: UVec2::new(200, 1000),
            plane_size: Vec2::new(200.0, 1000.0),
            height_scale: 10.0,
            frequency: 0.05,
            lacunarity: 2.0,
            octaves: 5,
            persistence: 0.5,
            material: Handle::default(),
        }
    }
}

pub struct FbmTerrainPlugin;
impl Plugin for FbmTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_terrain_system);
    }
}

fn generate_terrain_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    terrain_query: Query<(Entity, &Terrain, Option<&Transform>), With<Terrain>>,
) {
    for (entity, terrain, transform) in terrain_query.iter() {
        info!("Generating terrain for entity {:?}", entity);

        let fbm = Fbm::<Perlin>::new(terrain.seed)
            .set_octaves(terrain.octaves)
            .set_frequency(terrain.frequency as f64)
            .set_lacunarity(terrain.lacunarity as f64)
            .set_persistence(terrain.persistence as f64);

        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        let num_vertices_x = terrain.size.x;
        let num_vertices_z = terrain.size.y;

        let step_x = terrain.plane_size.x / (num_vertices_x as f32);
        let step_z = terrain.plane_size.y / (num_vertices_z as f32);

        let offset_x = terrain.plane_size.x / 2.0;
        let offset_z = terrain.plane_size.y / 2.0;

        for z_idx in 0..num_vertices_z {
            for x_idx in 0..num_vertices_x {
                let x = x_idx as f32 * step_x + offset_x;
                let z = z_idx as f32 * step_z + offset_z;

                let noise_x = x as f64;
                let noise_z = z as f64;

                let noise_val = fbm.get([
                    noise_x * terrain.frequency, 
                    noise_z * terrain.frequency
                ]);

                let height = terrain.height_scale * noise_val as f32;

                positions.push([x, height, z]);
                uvs.push([
                    x_idx as f32 / num_vertices_x as f32, 
                    z_idx as f32 / num_vertices_z as f32
                    ]);
            }
        }

        // Triangles
        for z_idx in 0..num_vertices_z - 1 {
            for x_idx in 0..num_vertices_x - 1 {
                let first = z_idx * num_vertices_x + x_idx;
                let second = first + 1;
                let third = (z_idx + 1) * num_vertices_x + x_idx;
                let fourth = third + 1;

                indices.push(first);
                indices.push(second);
                indices.push(third);

                indices.push(second);
                indices.push(third);
                indices.push(fourth);
            }
        }

        // Calculate normals
        let mut normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; positions.len()];

        for i in (0..indices.len()).step_by(3) {
            let a = indices[i] as usize;
            let b = indices[i + 1] as usize;
            let c = indices[i + 2] as usize;

            let u = Vec3::from_array(positions[a]);
            let v = Vec3::from_array(positions[b]);
            let w = Vec3::from_array(positions[c]);

            let edge1 = v - u;
            let edge2 = w - u;
            let face_normal = edge1.cross(edge2).normalize_or_zero();

            normals[a][0] = face_normal.x;
            normals[a][1] = face_normal.y;
            normals[a][2] = face_normal.z;

            normals[b][0] = face_normal.x;
            normals[b][1] = face_normal.y;
            normals[b][2] = face_normal.z;

            normals[c][0] = face_normal.x;
            normals[c][1] = face_normal.y;
            normals[c][2] = face_normal.z;
        }

        // Face up if degenerate normals
        for normal_array in normals.iter_mut() {
            if *normal_array == [0.0, 0.0, 0.0] {
                *normal_array = [0.0, 1.0, 0.0];
            }
        }

        // Create mesh
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        mesh.insert_indices(Indices::U32(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

        let transform = transform.cloned().unwrap_or_default();

        commands.entity(entity).insert((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(terrain.material.clone()),
            transform,
        ));
    }
}

