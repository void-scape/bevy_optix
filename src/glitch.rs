use bevy::{
    asset::load_internal_asset,
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    prelude::*,
    render::{
        globals::{GlobalsBuffer, GlobalsUniform},
        render_resource::ShaderType,
    },
};
use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    ecs::query::QueryItem,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
        RenderApp,
    },
};
use bevy_tween::{component_tween_system, prelude::Interpolator, BevyTweenRegisterSystems};

pub const GLITCH_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0x19A72E656);

pub struct GlitchPlugin;

impl Plugin for GlitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<GlitchSettings>::default(),
            UniformComponentPlugin::<GlitchSettings>::default(),
        ))
        .add_tween_systems(component_tween_system::<TweenGlitch>())
        .add_systems(Update, tween_glitch);

        load_internal_asset!(
            app,
            GLITCH_SHADER_HANDLE,
            "shaders/glitch.wgsl",
            Shader::from_wgsl
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<GlitchNode>>(Core2d, GlitchLabel)
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::Tonemapping,
                    GlitchLabel,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<GlitchPipeline>();
    }
}

#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct GlitchSettings {
    pub shake_power: f32,
    pub shake_rate: f32,
    pub shake_speed: f32,
    pub shake_block_size: f32,
    pub shake_color_rate: f32,
    pub intensity: f32,
}

impl Default for GlitchSettings {
    fn default() -> Self {
        Self {
            shake_power: 0.03,
            shake_rate: 0.5,
            shake_speed: 5.,
            shake_block_size: 30.5,
            shake_color_rate: 0.01,
            intensity: 0.5,
        }
    }
}

impl GlitchSettings {
    pub fn from_intensity(intensity: f32) -> Self {
        Self {
            intensity,
            ..Default::default()
        }
    }
}

/// Describes the `intensity` of the screen's [`GlitchUniform`].
///
/// Use [`Single`] to access.
#[derive(Default, Component)]
pub struct GlitchIntensity(pub f32);

pub fn glitch_intensity(start: f32, end: f32) -> TweenGlitch {
    TweenGlitch::new(start, end)
}

#[derive(Component)]
pub struct TweenGlitch {
    start: f32,
    end: f32,
}

impl TweenGlitch {
    pub fn new(start: f32, end: f32) -> Self {
        Self { start, end }
    }
}

impl Interpolator for TweenGlitch {
    type Item = GlitchIntensity;

    fn interpolate(&self, item: &mut Self::Item, value: f32) {
        item.0 = self.start.lerp(self.end, value);
    }
}

fn tween_glitch(mut glitch_query: Query<(&mut GlitchSettings, &GlitchIntensity)>) {
    for (mut settings, intensity) in glitch_query.iter_mut() {
        settings.intensity = intensity.0;
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct GlitchLabel;

#[derive(Default)]
struct GlitchNode;

impl ViewNode for GlitchNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static GlitchSettings,
        &'static DynamicUniformIndex<GlitchSettings>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, _post_process_settings, settings_index): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let post_process_pipeline = world.resource::<GlitchPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(post_process_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<GlitchSettings>>();
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
struct GlitchPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for GlitchPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "glitch_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<GlitchSettings>(true),
                    uniform_buffer::<GlobalsUniform>(false),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());
        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("glitch_pipeline".into()),
                    layout: vec![layout.clone()],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: GLITCH_SHADER_HANDLE,
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
        }
    }
}
