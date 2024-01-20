use std::borrow::Cow;

use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        main_graph::node::CAMERA_DRIVER,
        render_graph::{Node, RenderGraph},
        render_resource::{
            BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, CachedComputePipelineId,
            CachedPipelineState, ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache,
            ShaderStages,
        },
        renderer::RenderDevice,
        Render, RenderApp, RenderSet,
    },
};

use crate::utils::create_storage_buffer_with_data;

pub struct GrassShaderPlugin;
impl Plugin for GrassShaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<GrassComputeBuffers>::default())
            .add_systems(Startup, setup);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<GrassComputePipeline>()
            .add_systems(Render, queue_bind_group.in_set(RenderSet::Queue));

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("grass_compute", GrassComputeNode::default());
        render_graph.add_node_edge("grass_compute", CAMERA_DRIVER);
    }
}

#[derive(Resource, Clone, ExtractResource)]
pub struct GrassComputeBuffers {
    in_buffer: Buffer,
    out_buffer: Buffer,
}

#[derive(Resource)]
struct GrassComputePipeline {
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for GrassComputePipeline {
    fn from_world(world: &mut World) -> Self {
        let bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("Grass Bind Group Layout"),
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_cache = world.resource::<PipelineCache>();
        let shader = world.resource::<AssetServer>().load("shaders/grass.wgsl");

        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            shader: shader.clone(),
            shader_defs: vec![],
            layout: vec![bind_group_layout.clone()],
            entry_point: Cow::from("init"),
            push_constant_ranges: Vec::new(),
            label: Some(Cow::Borrowed("Grass Init Pipeline")),
        });

        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            shader,
            shader_defs: vec![],
            layout: vec![bind_group_layout.clone()],
            entry_point: Cow::from("update"),
            push_constant_ranges: Vec::new(),
            label: Some(Cow::Borrowed("Grass Update Pipeline")),
        });

        GrassComputePipeline {
            bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

#[derive(Resource)]
struct GrassComputeBindGroup(pub BindGroup);

enum GrassComputeState {
    Loading,
    Init,
    Update,
}

struct GrassComputeNode {
    state: GrassComputeState,
}

impl Default for GrassComputeNode {
    fn default() -> Self {
        Self {
            state: GrassComputeState::Loading,
        }
    }
}

impl Node for GrassComputeNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<GrassComputePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            GrassComputeState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                {
                    self.state = GrassComputeState::Init;
                }
            }
            GrassComputeState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = GrassComputeState::Update;
                }
            }
            GrassComputeState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let bind_group = &world.resource::<GrassComputeBindGroup>().0;
        let pipeline = world.resource::<GrassComputePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, bind_group, &[]);

        match self.state {
            GrassComputeState::Update | GrassComputeState::Loading => {}
            GrassComputeState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(1, 1, 1)
            }
        }
        Ok(())
    }
}

fn setup(mut commands: Commands, render_device: Res<RenderDevice>) {
    commands.insert_resource(GrassComputeBuffers {
        in_buffer: create_storage_buffer_with_data::<Vec4>(
            &render_device,
            &vec![Vec4::ZERO],
            Some("Grass Compute Buffer 0"),
        ),
        out_buffer: create_storage_buffer_with_data::<Vec4>(
            &render_device,
            &vec![Vec4::ZERO],
            Some("Grass Compute Buffer 1"),
        ),
    });
}

fn queue_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<GrassComputePipeline>,
    buffers: Res<GrassComputeBuffers>,
) {
    let bind_group = render_device.create_bind_group(
        Some("Grass Bind Group"),
        &pipeline.bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: buffers.in_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: buffers.out_buffer.as_entire_binding(),
            },
        ],
    );
    commands.insert_resource(GrassComputeBindGroup(bind_group));
}
