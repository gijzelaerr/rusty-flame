use na::{Matrix3, Point2};

use crate::{
    flame::{BoundedState, State},
    geometry, get_state, BASE_LEVELS, INSTANCE_LEVELS,
};
use rendy::{
    command::{QueueId, RenderPassEncoder},
    core::types::Layout,
    factory::Factory,
    graph::{
        render::{PrepareResult, SimpleGraphicsPipeline, SimpleGraphicsPipelineDesc},
        GraphContext, NodeBuffer, NodeImage,
    },
    hal::{self},
    memory::Dynamic,
    mesh::{AsVertex, TexCoord},
    resource::{Buffer, BufferInfo, DescriptorSetLayout, Escape, Handle},
    shader::{PathBufShaderInfo, ShaderKind, SourceLanguage, SpirvReflection, SpirvShader},
};

lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = PathBufShaderInfo::new(
        std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/sprite.vert")),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref FRAGMENT: SpirvShader = PathBufShaderInfo::new(
        std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/solid.frag")),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref SHADERS: rendy::shader::ShaderSetBuilder = rendy::shader::ShaderSetBuilder::default()
        .with_vertex(&*VERTEX).unwrap()
        .with_fragment(&*FRAGMENT).unwrap();

    static ref SHADER_REFLECTION: SpirvReflection = SHADERS.reflect().unwrap();
}

#[derive(Debug, Default)]
pub struct PipelineDesc;

#[derive(Debug)]
pub struct Pipeline<B: gfx_hal::Backend> {
    vertex_buffer: Escape<Buffer<B>>,
    vertex_count: u32,
    instance_buffer: Escape<Buffer<B>>,
    instance_count: u32,
}

impl<B> SimpleGraphicsPipelineDesc<B, Point2<f64>> for PipelineDesc
where
    B: gfx_hal::Backend,
{
    type Pipeline = Pipeline<B>;

    fn vertices(
        &self,
    ) -> Vec<(
        Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>,
        gfx_hal::pso::ElemStride,
        gfx_hal::pso::VertexInputRate,
    )> {
        vec![
            SHADER_REFLECTION
                .attributes(&["vertex_position"])
                .unwrap()
                .gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex),
            SHADER_REFLECTION
                .attributes(&["sprite_position_1", "sprite_position_2"])
                .unwrap()
                .gfx_vertex_input_desc(hal::pso::VertexInputRate::Instance(1)),
        ]
    }

    fn layout(&self) -> Layout {
        SHADER_REFLECTION.layout().unwrap()
    }

    fn colors(&self) -> Vec<hal::pso::ColorBlendDesc> {
        vec![
            hal::pso::ColorBlendDesc {
                mask: hal::pso::ColorMask::ALL,
                blend: Some(hal::pso::BlendState::ADD)
            };
            1
        ]
    }

    fn depth_stencil(&self) -> Option<gfx_hal::pso::DepthStencilDesc> {
        None
    }

    fn load_shader_set(
        &self,
        factory: &mut Factory<B>,
        _aux: &Point2<f64>,
    ) -> rendy::shader::ShaderSet<B> {
        SHADERS.build(factory, Default::default()).unwrap()
    }

    fn build(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        _queue: QueueId,
        aux: &Point2<f64>,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<Pipeline<B>, gfx_hal::pso::CreationError> {
        assert!(buffers.is_empty());
        assert!(images.is_empty());
        assert!(set_layouts.is_empty());
        Ok(build_mesh(factory, aux))
    }
}

fn build_mesh<B: gfx_hal::Backend>(factory: &Factory<B>, aux: &Point2<f64>) -> Pipeline<B> {
    let root = get_state([aux.x + 1.0, aux.y + 1.0], [2.0, 2.0]);
    let state = root.get_state();
    let bounds = state.get_bounds();
    let root_mat = geometry::letter_box(
        geometry::Rect {
            min: na::Point2::new(-1.0, -1.0),
            max: na::Point2::new(1.0, 1.0),
        },
        bounds,
    );

    let corners = bounds.corners();
    let mut verts: Vec<TexCoord> = vec![];
    let tri_verts = [
        corners[0], corners[1], corners[2], corners[0], corners[2], corners[3],
    ];

    state.process_levels(BASE_LEVELS, &mut |state| {
        for t in &tri_verts {
            let t2 = state.mat * t;
            verts.push([t2.x as f32, t2.y as f32].into());
        }
    });
    let vertex_count: u32 = verts.len() as u32;

    let mut vertex_buffer = factory
        .create_buffer(
            BufferInfo {
                size: u64::from(TexCoord::vertex().stride) * u64::from(vertex_count),
                usage: gfx_hal::buffer::Usage::VERTEX,
            },
            Dynamic,
        )
        .unwrap();

    let mut instances: Vec<f32> = vec![];
    state.process_levels(INSTANCE_LEVELS, &mut |state| {
        let m: Matrix3<f64> = (root_mat * state.mat).to_homogeneous();
        let s = m.as_slice();
        instances.extend(
            [
                s[0] as f32,
                s[3] as f32,
                s[6] as f32,
                0f32,
                s[1] as f32,
                s[4] as f32,
                s[7] as f32,
                0f32,
            ]
            .iter(),
        );
    });
    let instance_count: u32 = instances.len() as u32 / 8;
    let mut instance_buffer = factory
        .create_buffer(
            BufferInfo {
                size: (instances.len() * std::mem::size_of::<f32>()) as u64,
                usage: gfx_hal::buffer::Usage::VERTEX,
            },
            Dynamic,
        )
        .unwrap();

    unsafe {
        factory
            .upload_visible_buffer(&mut vertex_buffer, 0, &verts)
            .unwrap();
        factory
            .upload_visible_buffer(&mut instance_buffer, 0, &instances)
            .unwrap();
    }

    Pipeline {
        vertex_buffer,
        vertex_count,
        instance_buffer,
        instance_count,
    }
}

impl<B> SimpleGraphicsPipeline<B, Point2<f64>> for Pipeline<B>
where
    B: gfx_hal::Backend,
{
    type Desc = PipelineDesc;

    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        _set_layouts: &[Handle<DescriptorSetLayout<B>>],
        _index: usize,
        aux: &Point2<f64>,
    ) -> PrepareResult {
        *self = build_mesh(factory, aux);
        PrepareResult::DrawRecord
    }

    fn draw(
        &mut self,
        _layout: &B::PipelineLayout,
        mut encoder: RenderPassEncoder<'_, B>,
        _index: usize,
        _aux: &Point2<f64>,
    ) {
        unsafe {
            encoder.bind_vertex_buffers(0, std::iter::once((self.vertex_buffer.raw(), 0)));
            encoder.bind_vertex_buffers(1, std::iter::once((self.instance_buffer.raw(), 0)));
            encoder.draw(0..self.vertex_count, 0..self.instance_count);
        }
    }

    fn dispose(self, _factory: &mut Factory<B>, _aux: &Point2<f64>) {}
}
