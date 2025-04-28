use bevy::{
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    prelude::*,
    render::{
        globals::{GlobalsBuffer, GlobalsUniform},
        render_resource::{
            ShaderType,
            binding_types::{sampler, texture_2d, uniform_buffer},
            encase::private::WriteInto,
        },
    },
};
use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    ecs::query::QueryItem,
    render::{
        RenderApp,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
    },
};
use std::{fmt::Debug, hash::Hash, marker::PhantomData};

pub trait PostProcessMaterial: ShaderType {
    /// Returns this material's fragment shader. If [`ShaderRef::Default`] is returned, the default mesh fragment shader
    /// will be used.
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }
}

pub struct PostProcessPlugin<S>(PhantomData<S>);

impl<S> Default for PostProcessPlugin<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<S> Plugin for PostProcessPlugin<S>
where
    S: Clone + Copy + Component + ExtractComponent + ShaderType + PostProcessMaterial + WriteInto,
    ViewNodeRunner<PostProcessNode<S>>: FromWorld,
{
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<S>::default(),
            UniformComponentPlugin::<S>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<PostProcessNode<S>>>(
                Core2d,
                PostProcessLabel::<S>::default(),
            )
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::Tonemapping,
                    PostProcessLabel::<S>::default(),
                    Node2d::EndMainPassPostProcessing,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<PostProcessPipeline<S>>();
    }
}

#[derive(Clone, RenderLabel)]
struct PostProcessLabel<S>(PhantomData<S>);

impl<S> PartialEq for PostProcessLabel<S> {
    fn eq(&self, other: &Self) -> bool {
        std::any::type_name_of_val(&self.0) == std::any::type_name_of_val(&other.0)
    }
}

impl<S> Eq for PostProcessLabel<S> {}

impl<S> Hash for PostProcessLabel<S> {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

impl<S> Debug for PostProcessLabel<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("PostProcessLabel({})", std::any::type_name::<S>()))
    }
}

impl<S> Default for PostProcessLabel<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(Default)]
struct PostProcessNode<S>(PhantomData<S>);

impl<S> ViewNode for PostProcessNode<S>
where
    S: Clone + Copy + Component + ShaderType + WriteInto,
{
    type ViewQuery = (
        &'static ViewTarget,
        &'static S,
        &'static DynamicUniformIndex<S>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, _post_process_settings, settings_index): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let post_process_pipeline = world.resource::<PostProcessPipeline<S>>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(post_process_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<S>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let Some(globals_binding) = world.resource::<GlobalsBuffer>().buffer.binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();
        let bind_group = render_context.render_device().create_bind_group(
            "post_process_bind_group",
            &post_process_pipeline.layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &post_process_pipeline.sampler,
                settings_binding,
                globals_binding,
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("post_process_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
struct PostProcessPipeline<S> {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
    _phantom: PhantomData<S>,
}

impl<S> FromWorld for PostProcessPipeline<S>
where
    S: PostProcessMaterial,
{
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "glitch_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<S>(true),
                    uniform_buffer::<GlobalsUniform>(false),
                ),
            ),
        );

        let shader = match S::fragment_shader() {
            ShaderRef::Handle(handle) => handle,
            ShaderRef::Path(path) => world.load_asset(path),
            ShaderRef::Default => todo!("default post_process shader"),
        };

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());
        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some(
                        format!("post_process_{}_pipeline", std::any::type_name::<S>()).into(),
                    ),
                    layout: vec![layout.clone()],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader,
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::Rgba16Float,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                    zero_initialize_workgroup_memory: false,
                });

        Self {
            layout,
            sampler,
            pipeline_id,
            _phantom: PhantomData,
        }
    }
}
