use bevy::{
    prelude::*,
    render::{mesh::VertexAttributeValues, render_resource::PrimitiveTopology},
};
use rand::prelude::*;
use rand_distr::{Distribution, WeightedAliasIndex};

type Triangle = [Vec3; 3];

pub trait MeshSampler {
    fn sample_tri_list(&self, mesh: &Mesh) -> Vec<Vec3>;
    fn sample_tri_strip(&self, mesh: &Mesh) -> Vec<Vec3>;
    fn sample(&self, mesh: &Mesh) -> Vec<Vec3> {
        match mesh.primitive_topology() {
            PrimitiveTopology::PointList
            | PrimitiveTopology::LineList
            | PrimitiveTopology::LineStrip => return vec![],
            PrimitiveTopology::TriangleList => return self.sample_tri_list(mesh),
            PrimitiveTopology::TriangleStrip => return self.sample_tri_strip(mesh),
        }
    }
}

pub struct UniformRandomSampler {
    pub density: f32,
    pub threshold: f32,
}

impl Default for UniformRandomSampler {
    fn default() -> Self {
        Self {
            density: 1.,
            threshold: 0.,
        }
    }
}

impl UniformRandomSampler {
    fn sample_triangle(&self, triangle: Triangle) -> Vec3 {
        let mut u = fastrand::f32();
        let mut v = fastrand::f32();
        if u + v > 1. {
            u = 1. - u;
            v = 1. - v;
        }
        triangle[0] + (triangle[1] - triangle[0]) * u + (triangle[2] - triangle[0]) * v
    }
}

impl MeshSampler for UniformRandomSampler {
    fn sample_tri_list(&self, mesh: &Mesh) -> Vec<Vec3> {
        let mesh_duped = mesh.clone().with_duplicated_vertices();
        let VertexAttributeValues::Float32x3(positions) =
            mesh_duped.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            return vec![];
        };
        let VertexAttributeValues::Float32x3(normals) =
            mesh_duped.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap()
        else {
            return vec![];
        };

        let mut mesh_sa = 0.;
        let triangles: Vec<(Triangle, f32)> = positions
            .into_iter()
            .zip(normals.into_iter())
            .map(|(v, n)| (Vec3::from_array(*v), Vec3::from_array(*n)))
            .collect::<Vec<(Vec3, Vec3)>>()
            .chunks(3)
            .filter_map(|triangle| {
                let [a, b, c] = triangle[..] else { return None };

                let dot = Vec3::Y.dot((a.1 + b.1 + c.1).normalize());
                if dot < self.threshold {
                    return None;
                }
                let area = (a.0 - b.0).cross(a.0 - c.0).length();
                if area > 0. {
                    mesh_sa += area;
                } else {
                    return None;
                }
                Some(([a.0, b.0, c.0], area))
            })
            .collect();

        let sample_count = (mesh_sa * self.density) as usize;

        let areas = &triangles
            .iter()
            .map(|(_, area)| *area)
            .collect::<Vec<f32>>()[..];
        let dist = WeightedAliasIndex::new(areas.to_vec()).unwrap();
        let mut rng = thread_rng();
        (0..sample_count)
            .map(|_| self.sample_triangle(triangles[dist.sample(&mut rng)].0))
            .collect()
    }

    fn sample_tri_strip(&self, mesh: &Mesh) -> Vec<Vec3> {
        vec![]
    }
}
