use empa::buffer;
use empa::buffer::{Buffer, BufferUsages, Storage};
use empa::command::{
    DrawIndexed, DrawIndexedCommandEncoder, RenderBundleEncoderDescriptor, RenderStateEncoder,
    ResourceBindingCommandEncoder,
};
use empa::device::Device;
use empa::render_pipeline::{
    ColorOutput, ColorWrite, DepthStencilTest, FragmentStageBuilder, Index32, PrimitiveAssembly,
    RenderPipeline, RenderPipelineDescriptorBuilder, VertexStageBuilder,
};
use empa::resource_binding::BindGroupLayout;
use empa::shader_module::{shader_source, ShaderSource};
use empa::texture::format::{depth24plus, rgba8unorm};

use crate::line::Line;
use crate::renderer::{MainPassBundle, MainPassLayout};

const SHADER: ShaderSource = shader_source!("shader.wgsl");

#[derive(empa::resource_binding::Resources)]
struct Resources<'a> {
    #[resource(binding = 0, visibility = "VERTEX | FRAGMENT")]
    long_axes: Storage<'a, [Line]>,
}

type ResourcesLayout = <Resources<'static> as empa::resource_binding::Resources>::Layout;

pub struct LongAxesPass {
    device: Device,
    bind_group_layout: BindGroupLayout<ResourcesLayout>,
    pipeline: RenderPipeline<MainPassLayout, (), Index32, (ResourcesLayout,)>,
    indices: Buffer<[u32], BufferUsages!(Index)>,
}

impl LongAxesPass {
    pub async fn init(device: Device) -> Self {
        let shader = device.create_shader_module(&SHADER);

        let bind_group_layout = device.create_bind_group_layout::<ResourcesLayout>();
        let pipeline_layout = device.create_pipeline_layout(&bind_group_layout);

        let pipeline = device
            .create_render_pipeline(
                &RenderPipelineDescriptorBuilder::begin()
                    .layout(&pipeline_layout)
                    .primitive_assembly(PrimitiveAssembly::line_strip::<Index32>())
                    .vertex(
                        VertexStageBuilder::begin(&shader, "vert_main")
                            .vertex_layout::<()>()
                            .finish(),
                    )
                    .fragment(
                        FragmentStageBuilder::begin(&shader, "frag_main")
                            .color_outputs(ColorOutput {
                                format: rgba8unorm,
                                write_mask: ColorWrite::All,
                            })
                            .finish(),
                    )
                    .depth_stencil_test(DepthStencilTest::read_write::<depth24plus>())
                    .finish(),
            )
            .await;

        let indices = device.create_buffer([0, 1], buffer::Usages::index());

        LongAxesPass {
            device,
            bind_group_layout,
            pipeline,
            indices,
        }
    }

    pub fn render_bundle(
        &self,
        long_axes: buffer::View<[Line], impl buffer::StorageBinding>,
    ) -> Option<MainPassBundle> {
        if long_axes.len() == 0 {
            return None;
        }

        let bind_group = self.device.create_bind_group(
            &self.bind_group_layout,
            Resources {
                long_axes: long_axes.storage(),
            },
        );

        let render_bundle_encoder = self.device.create_render_bundle_encoder(
            &RenderBundleEncoderDescriptor::new::<rgba8unorm>()
                .depth_stencil_format::<depth24plus>(),
        );

        let bundle = render_bundle_encoder
            .set_pipeline(&self.pipeline)
            .set_index_buffer(&self.indices)
            .set_bind_groups(&bind_group)
            .draw_indexed(DrawIndexed {
                index_count: self.indices.len() as u32,
                instance_count: long_axes.len() as u32,
                first_index: 0,
                base_vertex: 0,
                first_instance: 0,
            })
            .finish();

        Some(bundle)
    }
}
