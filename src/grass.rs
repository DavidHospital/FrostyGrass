use bevy::{prelude::*, render::view::NoFrustumCulling};
use bytemuck::{Pod, Zeroable};

use crate::sampling::{MeshSampler, UniformRandomSampler};

use crate::render::instancing::{InstanceData, InstancedMaterial, InstancingPlugin};

#[derive(Component, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Grass {
    position: Vec3,
    scale: f32,
}

impl InstancedMaterial for Grass {
    type M = StandardMaterial;

    fn shader_path() -> &'static str {
        "shaders/grass.wgsl"
    }
}

#[derive(Component)]
pub struct Grassable {
    pub mesh: Handle<Mesh>,
    pub grass_mesh: Handle<Mesh>,
    pub grass_material: Handle<StandardMaterial>,
    pub density: f32,
}

pub struct GrassPlugin;

impl Plugin for GrassPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InstancingPlugin::<Grass>::default())
            .add_systems(PostStartup, spawn_grass_points);
    }
}

fn spawn_grass_points(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    grassables_q: Query<(&Grassable, &Transform)>,
) {
    for (grassable, transform) in grassables_q.iter() {
        let Some(mesh) = meshes.get(&grassable.mesh) else {
            continue;
        };
        let grass_points = UniformRandomSampler {
            density: grassable.density,
            threshold: 0.75,
        }
        .sample(mesh);
        if grass_points.len() == 0 {
            continue;
        }
        commands.spawn((
            grassable.grass_mesh.clone(),
            grassable.grass_material.clone(),
            SpatialBundle {
                transform: Transform::from_xyz(0., f32::MIN, 0.),
                ..SpatialBundle::INHERITED_IDENTITY
            },
            InstanceData {
                data: grass_points
                    .iter()
                    .map(|vec| Grass {
                        position: transform.transform_point(*vec),
                        scale: 1.,
                    })
                    .collect(),
                mesh: grassable.grass_mesh.clone(),
            },
            NoFrustumCulling,
        ));
    }
}
