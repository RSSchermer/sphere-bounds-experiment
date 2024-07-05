use std::f32::consts::PI;
use std::ops::Rem;

use empa::buffer::{Buffer, BufferUsages, Storage};
use empa::command::{
    DrawIndexed, DrawIndexedCommandEncoder, RenderBundleEncoderDescriptor, RenderStateEncoder,
    ResourceBindingCommandEncoder,
};
use empa::device::Device;
use empa::render_pipeline::{
    BlendComponent, BlendFactor, BlendState, BlendedColorOutput, ColorOutput, ColorWrite,
    DepthStencilTest, FragmentStageBuilder, Index32, IndexAny, PrimitiveAssembly, RenderPipeline,
    RenderPipelineDescriptorBuilder, VertexStageBuilder,
};
use empa::resource_binding::BindGroupLayout;
use empa::shader_module::{shader_source, ShaderSource};
use empa::texture::format::{depth24plus, rgba8unorm};
use empa::{abi, buffer};

use crate::circle::Circle;
use crate::renderer::{MainPassBundle, MainPassLayout};

const SHADER: ShaderSource = shader_source!("shader.wgsl");

struct CircleData {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl CircleData {
    pub fn new(subdivisions: usize) -> Self {
        let vertex_count = subdivisions + 1;
        let index_count = subdivisions * 3;

        let mut vertices = Vec::with_capacity(vertex_count);
        let mut indices = Vec::with_capacity(index_count);

        vertices.push(Vertex {
            position: [0.0, 0.0],
        });

        let segment_angle = (2.0 * PI) / subdivisions as f32;

        for i in 0..subdivisions {
            let angle = i as f32 * segment_angle;

            vertices.push(Vertex {
                position: [f32::cos(angle), f32::sin(angle)],
            });

            indices.push(0);
            indices.push((i + 1) as u32);
            indices.push(((i + 1).rem(subdivisions) + 1) as u32);
        }

        CircleData { vertices, indices }
    }
}

#[derive(empa::render_pipeline::Vertex, Clone, Copy, Debug)]
#[repr(C)]
struct Vertex {
    #[vertex_attribute(location = 0, format = "float32x2")]
    position: [f32; 2],
}

#[derive(empa::resource_binding::Resources)]
struct Resources<'a> {
    #[resource(binding = 0, visibility = "VERTEX | FRAGMENT")]
    occluder_circles: Storage<'a, [Circle]>,
}

type ResourcesLayout = <Resources<'static> as empa::resource_binding::Resources>::Layout;

pub struct OccluderCirclesPass {
    device: Device,
    bind_group_layout: BindGroupLayout<ResourcesLayout>,
    pipeline: RenderPipeline<MainPassLayout, Vertex, IndexAny, (ResourcesLayout,)>,
    vertices: Buffer<[Vertex], BufferUsages!(Vertex)>,
    indices: Buffer<[u32], BufferUsages!(Index)>,
}

impl OccluderCirclesPass {
    pub async fn init(device: Device, subdivisions: usize) -> Self {
        let shader = device.create_shader_module(&SHADER);

        let bind_group_layout = device.create_bind_group_layout::<ResourcesLayout>();
        let pipeline_layout = device.create_pipeline_layout(&bind_group_layout);

        let pipeline = device
            .create_render_pipeline(
                &RenderPipelineDescriptorBuilder::begin()
                    .layout(&pipeline_layout)
                    .primitive_assembly(PrimitiveAssembly::triangle_list())
                    .vertex(
                        VertexStageBuilder::begin(&shader, "vert_main")
                            .vertex_layout::<Vertex>()
                            .finish(),
                    )
                    .fragment(
                        FragmentStageBuilder::begin(&shader, "frag_main")
                            .color_outputs(BlendedColorOutput {
                                format: rgba8unorm,
                                blend_state: BlendState {
                                    color: BlendComponent::Add {
                                        src_factor: BlendFactor::SrcAlpha,
                                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                                    },
                                    alpha: Default::default(),
                                },
                                write_mask: ColorWrite::All,
                            })
                            .finish(),
                    )
                    .depth_stencil_test(DepthStencilTest::read_write::<depth24plus>())
                    .finish(),
            )
            .await;

        let CircleData { vertices, indices } = CircleData::new(subdivisions);

        let vertices = device.create_buffer(vertices, buffer::Usages::vertex());
        let indices = device.create_buffer(indices, buffer::Usages::index());

        OccluderCirclesPass {
            device,
            bind_group_layout,
            pipeline,
            vertices,
            indices,
        }
    }

    pub fn render_bundle(
        &self,
        occluder_circles: buffer::View<[Circle], impl buffer::StorageBinding>,
    ) -> Option<MainPassBundle> {
        if occluder_circles.len() == 0 {
            return None;
        }

        let bind_group = self.device.create_bind_group(
            &self.bind_group_layout,
            Resources {
                occluder_circles: occluder_circles.storage(),
            },
        );

        let render_bundle_encoder = self.device.create_render_bundle_encoder(
            &RenderBundleEncoderDescriptor::new::<rgba8unorm>()
                .depth_stencil_format::<depth24plus>(),
        );

        let bundle = render_bundle_encoder
            .set_pipeline(&self.pipeline)
            .set_vertex_buffers(&self.vertices)
            .set_index_buffer(&self.indices)
            .set_bind_groups(&bind_group)
            .draw_indexed(DrawIndexed {
                index_count: self.indices.len() as u32,
                instance_count: occluder_circles.len() as u32,
                first_index: 0,
                base_vertex: 0,
                first_instance: 0,
            })
            .finish();

        Some(bundle)
    }
}
