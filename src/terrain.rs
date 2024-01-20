use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_xpbd_3d::prelude::*;
use noise::{
    utils::{NoiseMapBuilder, PlaneMapBuilder},
    Fbm, NoiseFn, Perlin,
};

use crate::render::grass::{GrassPlugin, Grassable};

#[derive(Component)]
pub struct Terrain;

#[derive(Component)]
struct GrassPoints(Vec<Vec3>);

// #[derive(Component)]
// struct GrassConfiguration {
//     density: f32,
//     grass_mesh: Handle<Mesh>,
//     grass_material: Handle<StandardMaterial>,
//     color_map_texture: Handle<Image>,
// }

pub struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            // MaterialPlugin::<StandardMaterial>::default(),
            // CustomMaterialPlugin,
            GrassPlugin,
        ))
        .add_systems(Startup, setup_terrain);
        // .add_systems(PostStartup, spawn_grass_points);
    }
}

fn setup_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let terrain_mesh = _create_mesh(128, 128, 1., 1);
    let terrain_mesh_handle = meshes.add(terrain_mesh.clone().into());

    let grass_mesh = asset_server.load::<Mesh>("models/grass.gltf#Mesh0/Primitive0");
    let grass_material = materials.add(StandardMaterial {
        base_color: Color::hsla(105., 0.53, 0.33, 1.0),
        reflectance: 0.05,
        diffuse_transmission: 0.5,
        ..default()
    });

    // let color_map_texture = asset_server.load("textures/color_map.png");

    commands.spawn((
        Terrain,
        MaterialMeshBundle {
            mesh: terrain_mesh_handle.clone(),
            material: materials.add(StandardMaterial {
                base_color: Color::hsla(105., 0.53, 0.33, 1.0),
                reflectance: 0.05,
                diffuse_transmission: 0.5,
                ..default()
            }),
            ..default()
        },
        RigidBody::Static,
        Collider::trimesh_from_mesh(&terrain_mesh.into()).unwrap(),
        Grassable {
            mesh: terrain_mesh_handle,
            density: 16.,
            grass_mesh,
            grass_material,
        },
    ));
}

// fn spawn_grass_points(
//     mut commands: Commands,
//     meshes: Res<Assets<Mesh>>,
//     terrain_q: Query<(&Transform, &GrassConfiguration, &Handle<Mesh>), With<Terrain>>,
// ) {
//     for (transform, grass_config, mesh_handle) in terrain_q.iter() {
//         let mesh = meshes.get(mesh_handle).unwrap();
//         let grass_points = UniformRandomSampler {
//             density: grass_config.density,
//             threshold: 0.75,
//         }
//         .sample(mesh);
//         println!("Grass points: {:?}", grass_points.len());
//         if grass_points.len() > 0 {
//             println!("{}", grass_points.len());
//             commands.spawn((
//                 grass_config.grass_mesh.clone(),
//                 grass_config.grass_material.clone(),
//                 ColorMap::from(grass_config.color_map_texture.clone()),
//                 SpatialBundle::INHERITED_IDENTITY,
//                 InstanceMaterialData(
//                     grass_points
//                         .iter()
//                         .map(|vec| InstanceData {
//                             position: transform.transform_point(*vec),
//                             scale: 1.0,
//                         })
//                         .collect(),
//                 ),
//                 NoFrustumCulling,
//             ));
//         }
//     }
// }

fn _terrain_height(fbm: &Fbm<Perlin>, x: f32, y: f32) -> f32 {
    return fbm.get([x as f64 * 0.05678, y as f64 * 0.05678]) as f32;
}

#[rustfmt::skip]
fn _create_mesh(width: usize, height: usize, scale: f32, tex_scale: usize) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(4 * width * height);
    let mut uv_0s: Vec<[f32; 2]> = Vec::with_capacity(4 * width * height);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(4 * width * height);
    let mut indices: Vec<u32> = Vec::with_capacity(6 * width * height);

    let mut fbm = Fbm::<Perlin>::new(0);
    fbm.frequency = 0.032;
    fbm.persistence = 0.11;
    fbm.lacunarity = 11.;
    let noise_map = PlaneMapBuilder::<Fbm<Perlin>, 3>::new(fbm.clone())
        .set_size(width, height)
        .set_x_bounds(0., width as f64)
        .set_y_bounds(0., height as f64)
        .set_is_seamless(true)
        .build();

    let mut vertices = Vec::with_capacity((width + 1) * (height * 1));
    for i in 0..(width+1) {
        let x_pos = i as f32 * scale;
        for j in 0..(height+1) {
            let z_pos = j as f32 * scale;
            vertices.push(Vec3::new(x_pos, noise_map.get_value(i, j) as f32 * scale * 8., z_pos));
            // vertices.push(Vec3::new(x_pos, _terrain_height(&fbm, x_pos, z_pos), z_pos));
        }
    }

    let mut ind_offset = 0;
    for i in 0..width {
        for j in 0..height {
            unsafe {
                let a = *vertices.get_unchecked(i * (height + 1) + j);
                let b = *vertices.get_unchecked((i + 1) * (height + 1) + j);
                let c = *vertices.get_unchecked((i + 1) * (height + 1) + j + 1);
                let d = *vertices.get_unchecked(i * (height + 1) + j + 1);

                positions.push(a.into());
                positions.push(b.into());
                positions.push(c.into());
                positions.push(d.into());

                normals.push(_average_normal(&vertices, width + 1, height + 1, i, j).into());
                normals.push(_average_normal(&vertices, width + 1, height + 1, i + 1, j).into());
                normals.push(_average_normal(&vertices, width + 1, height + 1, i + 1, j + 1).into());
                normals.push(_average_normal(&vertices, width + 1, height + 1, i, j + 1).into());
            }
            let tex_scale_inv = 1. / tex_scale as f32;
            let u_idx = (i % tex_scale) as f32 * tex_scale_inv;
            let v_idx = (j % tex_scale) as f32 * tex_scale_inv;

            uv_0s.push([u_idx, v_idx + tex_scale_inv]);
            uv_0s.push([u_idx, v_idx]);
            uv_0s.push([u_idx + tex_scale_inv, v_idx]);
            uv_0s.push([u_idx + tex_scale_inv, v_idx + tex_scale_inv]);

            indices.push(ind_offset + 0);
            indices.push(ind_offset + 3);
            indices.push(ind_offset + 1);
            indices.push(ind_offset + 1);
            indices.push(ind_offset + 3);
            indices.push(ind_offset + 2);
            ind_offset += 4;
        }
    }
        
    Mesh::new(PrimitiveTopology::TriangleList)
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        positions
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        uv_0s
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        normals
    )
    .with_indices(Some(Indices::U32(indices)))
}

fn _average_normal(vertices: &Vec<Vec3>, width: usize, height: usize, x: usize, y: usize) -> Vec3 {
    if let Some(vertex) = vertices.get(x * height + y) {
        let mut normal = Vec3::ZERO;
        let left = if x > 0 {
            *(vertices.get((x - 1) * height + y).unwrap()) - *vertex
        } else {
            Vec3::ZERO
        };
        let right = if x < width - 1 {
            *(vertices.get((x + 1) * height + y).unwrap()) - *vertex
        } else {
            Vec3::ZERO
        };
        let up = if y > 0 {
            *(vertices.get(x * height + y - 1).unwrap()) - *vertex
        } else {
            Vec3::ZERO
        };
        let down = if y < width - 1 {
            *(vertices.get(x * height + y + 1).unwrap()) - *vertex
        } else {
            Vec3::ZERO
        };

        normal += left.cross(down) + down.cross(right) + right.cross(up) + up.cross(left);
        return normal.normalize();
    }
    Vec3::ZERO
}
