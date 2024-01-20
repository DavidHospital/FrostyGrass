use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::{Asset, StandardMaterial},
    reflect::Reflect,
    render::render_resource::AsBindGroup,
};

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct GrassExtension {
    density: f32,
}

impl Default for GrassExtension {
    fn default() -> Self {
        Self { density: 1.0 }
    }
}

impl MaterialExtension for GrassExtension {}

pub type GrassMaterial = ExtendedMaterial<StandardMaterial, GrassExtension>;
