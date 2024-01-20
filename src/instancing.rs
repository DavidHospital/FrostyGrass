//! A shader that renders a mesh multiple times in one draw call.

use std::marker::PhantomData;

use bevy::{
    core_pipeline::core_3d::Transparent3d,
    ecs::{
        query::QueryItem,
        system::{lifetimeless::*, SystemParamItem},
    },
    pbr::{
        MeshPipeline, MeshPipelineKey, RenderMeshInstances, SetMaterialBindGroup, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            RenderPhase, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::*,
        renderer::RenderDevice,
        view::ExtractedView,
        Render, RenderApp, RenderSet,
    },
};
use bytemuck::{Pod, Zeroable};

#[derive(Component)]
struct BindGroupBuffer<T> {
    pub bind_group: BindGroup,
    _inner: PhantomData<T>,
}

impl<T> BindGroupBuffer<T> {
    fn new(bind_group: BindGroup) -> Self {
        BindGroupBuffer {
            bind_group,
            _inner: PhantomData,
        }
    }
}

#[derive(Component, Deref)]
pub struct InstanceMaterialData(pub Vec<InstanceData>);

impl ExtractComponent for InstanceMaterialData {
    type Query = &'static InstanceMaterialData;
    type Filter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, Self::Query>) -> Option<Self> {
        Some(InstanceMaterialData(item.0.clone()))
    }
}

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<InstanceMaterialData>::default(),
            ExtractComponentPlugin::<ColorMap>::default(),
        ));
        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawCustom<StandardMaterial>>()
            .init_resource::<SpecializedMeshPipelines<CustomPipeline<StandardMaterial>>>()
            .add_systems(
                Render,
                (
                    queue_custom::<StandardMaterial>.in_set(RenderSet::QueueMeshes),
                    (
                        prepare_instance_buffers,
                        prepare_color_map_buffers::<StandardMaterial>,
                    )
                        .in_set(RenderSet::PrepareResources),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<CustomPipeline<StandardMaterial>>();
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct InstanceData {
    pub position: Vec3,
    pub scale: f32,
}

#[allow(clippy::too_many_arguments)]
fn queue_custom<M: Material>(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<CustomPipeline<M>>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<Mesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    material_meshes: Query<Entity, With<InstanceMaterialData>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_custom = transparent_3d_draw_functions.read().id::<DrawCustom<M>>();

    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

    for (view, mut transparent_phase) in &mut views {
        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        let rangefinder = view.rangefinder3d();
        for entity in &material_meshes {
            let Some(mesh_instance) = render_mesh_instances.get(&entity) else {
                continue;
            };
            let Some(mesh) = meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
            let key = view_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
            let pipeline = pipelines
                .specialize(&pipeline_cache, &custom_pipeline, key, &mesh.layout)
                .unwrap();
            transparent_phase.add(Transparent3d {
                entity,
                pipeline,
                draw_function: draw_custom,
                distance: rangefinder
                    .distance_translation(&mesh_instance.transforms.transform.translation),
                batch_range: 0..1,
                dynamic_offset: None,
            });
        }
    }
}

#[derive(Component)]
pub struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &InstanceMaterialData)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, instance_data) in &query {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(instance_data.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            length: instance_data.len(),
        });
    }
}

/// The color-map defining the color of the grass blades.
///
/// The area covered by the color map is defined by the area of the [`Aabb`](bevy::render::primitives::Aabb) component.
/// The [`ColorMap`] texture will be scaled over the complete area
///
/// For a simple example, take a look at the [`load_grass`](https://github.com/emiongit/warbler_grass/latest/example/load_grass.rs) example
#[derive(Reflect, Clone, Component)]
pub struct ColorMap(Handle<Image>);

impl From<Handle<Image>> for ColorMap {
    fn from(value: Handle<Image>) -> Self {
        ColorMap(value)
    }
}
impl ExtractComponent for ColorMap {
    type Query = &'static Self;

    type Filter = ();

    type Out = Self;

    fn extract_component(item: QueryItem<'_, Self::Query>) -> Option<Self::Out> {
        Some(ColorMap(item.0.clone_weak()))
    }
}

fn prepare_color_map_buffers<M: Material>(
    mut commands: Commands,
    query: Query<(Entity, &ColorMap)>,
    render_device: Res<RenderDevice>,
    pipeline: Res<CustomPipeline<M>>,
    images: Res<RenderAssets<Image>>,
) {
    let layout = pipeline.color_map_layout.clone();

    for (entity, color_map) in &query {
        let Some(texture) = images.get(&color_map.0) else {
            continue;
        };

        let bind_group = render_device.create_bind_group(
            Some("color map bind group"),
            &layout,
            &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture.texture_view),
            }],
        );
        commands
            .entity(entity)
            .insert(BindGroupBuffer::<ColorMap>::new(bind_group));
    }
}

#[derive(Resource)]
pub struct CustomPipeline<M: Material> {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    material_layout: BindGroupLayout,
    color_map_layout: BindGroupLayout,
    marker: PhantomData<M>,
}

impl<M: Material> FromWorld for CustomPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let shader = asset_server.load("shaders/instancing.wgsl");
        let render_device = world.resource::<RenderDevice>();

        let mesh_pipeline = world.resource::<MeshPipeline>();

        let color_map_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("warbler_grass color map layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });

        CustomPipeline {
            shader,
            mesh_pipeline: mesh_pipeline.clone(),
            material_layout: M::bind_group_layout(render_device),
            color_map_layout,
            marker: PhantomData,
        }
    }
}

impl<M: Material> SpecializedMeshPipeline for CustomPipeline<M> {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;

        descriptor.vertex.shader_defs.push("VERTEX_COLORS".into());
        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 3, // shader locations 0-2 are taken up by Position, Normal and UV attributes
                },
                // VertexAttribute {
                //     format: VertexFormat::Float32x4,
                //     offset: VertexFormat::Float32x4.size(),
                //     shader_location: 4,
                // },
            ],
        });
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.fragment.as_mut().unwrap().shader_defs = descriptor.vertex.shader_defs.clone();

        descriptor.layout.insert(1, self.material_layout.clone());
        descriptor.layout.insert(3, self.color_map_layout.clone());
        Ok(descriptor)
    }
}

struct SetColorMapBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetColorMapBindGroup<I> {
    type Param = ();
    type ViewWorldQuery = ();
    type ItemWorldQuery = Option<Read<BindGroupBuffer<ColorMap>>>;

    fn render<'w>(
        _item: &P,
        _view: (),
        bind_group: Option<&'w BindGroupBuffer<ColorMap>>,
        _cache: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(bind_group) = bind_group else {
            return RenderCommandResult::Failure;
        };
        pass.set_bind_group(I, &bind_group.bind_group, &[]);
        RenderCommandResult::Success
    }
}

type DrawCustom<M> = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<M, 1>,
    SetMeshBindGroup<2>,
    SetColorMapBindGroup<3>,
    DrawMeshInstanced,
);

pub struct DrawMeshInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawMeshInstanced {
    type Param = (SRes<RenderAssets<Mesh>>, SRes<RenderMeshInstances>);
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<InstanceBuffer>;

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        instance_buffer: &'w InstanceBuffer,
        (meshes, render_mesh_instances): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(mesh_instance) = render_mesh_instances.get(&item.entity()) else {
            return RenderCommandResult::Failure;
        };
        let gpu_mesh = match meshes.into_inner().get(mesh_instance.mesh_asset_id) {
            Some(gpu_mesh) => gpu_mesh,
            None => return RenderCommandResult::Failure,
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, 0..instance_buffer.length as u32);
            }
            GpuBufferInfo::NonIndexed => {
                pass.draw(0..gpu_mesh.vertex_count, 0..instance_buffer.length as u32);
            }
        }
        RenderCommandResult::Success
    }
}
